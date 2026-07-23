//! Adaptive probe controller — budgeted, time-boxed instrumentation.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use codepulse_protocol::{
    CreateProbeWindowResponse, ProbeAction, ProbeBudget, ProbeCommand, SymbolId, PROTOCOL_VERSION,
};
use codepulse_store::{now_ms, Store};
use parking_lot::Mutex;
use thiserror::Error;
use uuid::Uuid;

pub const MAX_TARGETS: u32 = 32;
pub const MAX_EVENTS_PER_SEC: u64 = 50_000;
pub const DEFAULT_DURATION_S: u64 = 30;

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("too many targets: {0} (max {MAX_TARGETS})")]
    TooManyTargets(usize),
    #[error("empty targets")]
    EmptyTargets,
    #[error("unknown probe window {0}")]
    UnknownWindow(String),
    #[error(transparent)]
    Store(#[from] codepulse_store::StoreError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ControllerError>;

#[derive(Clone)]
pub struct Controller {
    store: Store,
    /// Pending enable commands not yet acked, keyed by session_id (or "").
    pending: Arc<Mutex<HashMap<String, Vec<ProbeCommand>>>>,
    delivered: Arc<Mutex<HashSet<String>>>,
}

impl Controller {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            pending: Arc::new(Mutex::new(HashMap::new())),
            delivered: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn create_window(
        &self,
        session_id: Option<&str>,
        targets: Vec<SymbolId>,
        duration_s: u64,
    ) -> Result<CreateProbeWindowResponse> {
        if targets.is_empty() {
            return Err(ControllerError::EmptyTargets);
        }
        if targets.len() > MAX_TARGETS as usize {
            return Err(ControllerError::TooManyTargets(targets.len()));
        }
        let duration_s = if duration_s == 0 {
            DEFAULT_DURATION_S
        } else {
            duration_s
        };
        let window_id = format!("pw_{}", Uuid::new_v4());
        let started = now_ms();
        let expires_at_ms = started + duration_s * 1000;

        self.store.create_probe_window(
            &window_id,
            session_id,
            started,
            duration_s,
            &targets,
        )?;

        let cmd = ProbeCommand {
            protocol_version: PROTOCOL_VERSION,
            window_id: window_id.clone(),
            action: ProbeAction::Enable,
            targets,
            duration_s,
        };

        let key = session_id.unwrap_or("").to_string();
        self.pending.lock().entry(key).or_default().push(cmd);

        Ok(CreateProbeWindowResponse {
            window_id,
            status: "active".into(),
            expires_at_ms,
            budget: ProbeBudget {
                max_events_per_sec: MAX_EVENTS_PER_SEC,
                max_targets: MAX_TARGETS,
            },
        })
    }

    pub fn poll_commands(&self, session_id: &str) -> Result<Vec<ProbeCommand>> {
        self.expire_windows()?;

        let mut pending = self.pending.lock();
        let mut out = Vec::new();

        if let Some(cmds) = pending.get_mut(session_id) {
            out.append(cmds);
        }
        if session_id != "" {
            if let Some(cmds) = pending.get_mut("") {
                out.append(cmds);
            }
        }

        // Also emit disable for expired windows once
        let active = self.store.active_probe_windows(Some(session_id))?;
        let now = now_ms();
        let mut delivered = self.delivered.lock();
        for w in active {
            let expires = w.started_at_ms + w.duration_s * 1000;
            if now >= expires {
                let disable_key = format!("disable:{}", w.id);
                if !delivered.contains(&disable_key) {
                    delivered.insert(disable_key);
                    let targets: Vec<SymbolId> = serde_json::from_str(&w.targets_json)?;
                    out.push(ProbeCommand {
                        protocol_version: PROTOCOL_VERSION,
                        window_id: w.id.clone(),
                        action: ProbeAction::Disable,
                        targets,
                        duration_s: 0,
                    });
                    self.store
                        .update_probe_status(&w.id, "completed", Some(now))?;
                }
            }
        }

        Ok(out)
    }

    pub fn ack(&self, window_id: &str, status: &str) -> Result<()> {
        let now = now_ms();
        let row = self
            .store
            .get_probe_window(window_id)?
            .ok_or_else(|| ControllerError::UnknownWindow(window_id.to_string()))?;

        let mapped = match status {
            "budget_exceeded" => "budget_exceeded",
            "disabled" | "completed" => "completed",
            "error" => "cancelled",
            "active" => "active",
            other => other,
        };

        let ended = if mapped == "active" {
            None
        } else {
            Some(row.ended_at_ms.unwrap_or(now))
        };
        self.store
            .update_probe_status(window_id, mapped, ended)?;
        Ok(())
    }

    pub fn expire_windows(&self) -> Result<()> {
        let now = now_ms();
        let active = self.store.active_probe_windows(None)?;
        for w in active {
            let expires = w.started_at_ms + w.duration_s * 1000;
            if now >= expires {
                self.store
                    .update_probe_status(&w.id, "completed", Some(now))?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_poll() {
        let store = Store::open_in_memory().unwrap();
        let ctrl = Controller::new(store);
        let resp = ctrl
            .create_window(
                Some("sess"),
                vec![SymbolId::new("python", "a.py", "f")],
                30,
            )
            .unwrap();
        assert!(resp.window_id.starts_with("pw_"));
        let cmds = ctrl.poll_commands("sess").unwrap();
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0].action, ProbeAction::Enable));
    }
}
