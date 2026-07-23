//! Shared wire types for agent ↔ core and MCP ↔ core.
//!
//! See `docs/AGENT_PROTOCOL.md` and `docs/DATA_MODEL.md`.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Protocol version spoken by agents and the daemon.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolId {
    pub language: String,
    pub path: String,
    pub qualname: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatBatch {
    pub protocol_version: u32,
    pub session_id: String,
    pub process_id: u32,
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    pub stats: Vec<FunctionRuntimeStat>,
    pub edges: Vec<CallEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRuntimeStat {
    pub symbol: SymbolId,
    pub invocations: u64,
    pub exceptions: u64,
    pub duration_ns_p50: u64,
    pub duration_ns_p95: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub caller: SymbolId,
    pub callee: SymbolId,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeCommand {
    pub protocol_version: u32,
    pub window_id: String,
    pub action: ProbeAction,
    pub targets: Vec<SymbolId>,
    pub duration_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeAction {
    Enable,
    Disable,
}
