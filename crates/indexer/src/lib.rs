//! Static symbol indexer (tree-sitter).
//!
//! Python grammar first; additional languages are grammar plugins.
//! See `docs/ARCHITECTURE.md`.

#![allow(dead_code)]

pub struct Indexer;

impl Indexer {
    pub fn new() -> Self {
        Self
    }

    pub fn index_root(&self, _root: &str) -> Result<usize, IndexerError> {
        Err(IndexerError::NotImplemented)
    }
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IndexerError {
    #[error("indexer not implemented yet (design-phase stub)")]
    NotImplemented,
}
