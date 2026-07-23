//! Adaptive probe controller.
//!
//! Enables exact instrumentation only for selected symbols, for a bounded window.
//! See `docs/ARCHITECTURE.md`.

#![allow(dead_code)]

use codepulse_protocol::{ProbeCommand, SymbolId};

pub struct ProbeController;

impl ProbeController {
    pub fn new() -> Self {
        Self
    }

    pub fn enable_targeted(
        &self,
        _targets: Vec<SymbolId>,
        _duration_s: u64,
    ) -> Result<ProbeCommand, ControllerError> {
        Err(ControllerError::NotImplemented)
    }
}

impl Default for ProbeController {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    #[error("controller not implemented yet (design-phase stub)")]
    NotImplemented,
}
