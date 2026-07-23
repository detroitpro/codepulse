//! Local-first intelligence store.
//!
//! Schema described in `docs/DATA_MODEL.md`. Implementation deferred.

#![allow(dead_code)]

pub struct Store;

impl Store {
    pub fn open(_path: &str) -> Result<Self, StoreError> {
        Err(StoreError::NotImplemented)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store not implemented yet (design-phase stub)")]
    NotImplemented,
}
