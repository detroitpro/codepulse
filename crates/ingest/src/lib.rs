//! Validate and accept aggregated agent batches.

use codepulse_protocol::{RuntimeStatBatch, PROTOCOL_VERSION};
use codepulse_store::Store;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IngestError {
    #[error("unsupported protocol version {0}")]
    ProtocolVersion(u32),
    #[error("invalid window: start {0} >= end {1}")]
    InvalidWindow(u64, u64),
    #[error("empty session_id")]
    EmptySession,
    #[error(transparent)]
    Store(#[from] codepulse_store::StoreError),
}

pub type Result<T> = std::result::Result<T, IngestError>;

pub struct Ingestor {
    store: Store,
}

impl Ingestor {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    pub fn ingest(&self, batch: &RuntimeStatBatch) -> Result<()> {
        if batch.protocol_version != PROTOCOL_VERSION {
            return Err(IngestError::ProtocolVersion(batch.protocol_version));
        }
        if batch.session_id.is_empty() {
            return Err(IngestError::EmptySession);
        }
        if batch.window_start_ms >= batch.window_end_ms {
            return Err(IngestError::InvalidWindow(
                batch.window_start_ms,
                batch.window_end_ms,
            ));
        }

        let language = batch
            .language
            .as_deref()
            .or_else(|| batch.stats.first().map(|s| s.symbol.language.as_str()));

        self.store.ensure_session(
            &batch.session_id,
            language,
            batch.process_id,
            batch.window_start_ms,
        )?;

        for stat in &batch.stats {
            let symbol_id = self.store.ensure_symbol(&stat.symbol)?;
            self.store.upsert_runtime_stat(
                &batch.session_id,
                &symbol_id,
                batch.window_start_ms,
                batch.window_end_ms,
                stat.invocations,
                stat.exceptions,
                stat.duration_ns_p50,
                stat.duration_ns_p95,
            )?;
        }

        for edge in &batch.edges {
            let caller_id = self.store.ensure_symbol(&edge.caller)?;
            let callee_id = self.store.ensure_symbol(&edge.callee)?;
            self.store.upsert_edge(
                &batch.session_id,
                &caller_id,
                &callee_id,
                batch.window_start_ms,
                edge.count,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codepulse_protocol::{FunctionRuntimeStat, SymbolId};

    #[test]
    fn round_trip_hot_paths() {
        let store = Store::open_in_memory().unwrap();
        let ingest = Ingestor::new(store.clone());
        let batch = RuntimeStatBatch {
            protocol_version: PROTOCOL_VERSION,
            session_id: "s1".into(),
            process_id: 7,
            window_start_ms: 0,
            window_end_ms: 1000,
            language: Some("python".into()),
            stats: vec![FunctionRuntimeStat {
                symbol: SymbolId::new("python", "demo.py", "hot"),
                invocations: 100,
                exceptions: 0,
                duration_ns_p50: 1,
                duration_ns_p95: 2,
            }],
            edges: vec![],
        };
        ingest.ingest(&batch).unwrap();
        let hot = store.hot_paths(Some("s1"), 5, "invocations").unwrap();
        assert_eq!(hot[0].symbol.qualname, "hot");
        assert_eq!(hot[0].value, 100.0);
    }
}
