//! Static symbol indexer and structural pattern search (tree-sitter).
//!
//! Responsibilities:
//! - Walk workspace roots and extract symbols / syntactic edges
//! - On-demand `structural_search` with a codepulse-owned pattern language
//!   (metavariables `$NAME` / `$$$ARGS`; ast-grep-like ergonomics, not
//!   ast-grep wire-compatible; no external `ast-grep` CLI)
//!
//! Python grammar first; additional languages are grammar plugins.
//! Structural search is **not** part of the runtime agent protocol — MCP
//! reaches this crate via the daemon. See `docs/ARCHITECTURE.md` and
//! `docs/MCP_API.md`.

#![allow(dead_code)]

pub struct Indexer;

/// One compact match returned to MCP (payloads are capped upstream).
#[derive(Debug, Clone)]
pub struct StructuralMatch {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub matched_text: String,
}

#[derive(Debug, Clone)]
pub struct StructuralSearchRequest {
    pub language: String,
    pub pattern: String,
    pub path_prefix: Option<String>,
    pub limit: u32,
}

#[derive(Debug, Clone)]
pub struct StructuralSearchResponse {
    pub language: String,
    pub match_count: usize,
    pub truncated: bool,
    pub matches: Vec<StructuralMatch>,
}

impl Indexer {
    pub fn new() -> Self {
        Self
    }

    pub fn index_root(&self, _root: &str) -> Result<usize, IndexerError> {
        Err(IndexerError::NotImplemented)
    }

    /// On-demand AST pattern match over the workspace (or `path_prefix`).
    pub fn structural_search(
        &self,
        _req: &StructuralSearchRequest,
    ) -> Result<StructuralSearchResponse, IndexerError> {
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
