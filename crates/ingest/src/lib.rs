//! Accepts aggregated runtime batches from language agents.
//!
//! Implementation deferred — see `docs/ARCHITECTURE.md`.

#![allow(dead_code)]

use codepulse_protocol::RuntimeStatBatch;

/// Placeholder ingest handle. Will validate protocol version and forward to store.
pub struct IngestService;

impl IngestService {
    pub fn new() -> Self {
        Self
    }

    pub fn accept_batch(&self, _batch: RuntimeStatBatch) -> Result<(), IngestError> {
        Err(IngestError::NotImplemented)
    }
}

impl Default for IngestService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    #[error("ingest not implemented yet (design-phase stub)")]
    NotImplemented,
}
