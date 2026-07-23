//! Shared wire types for agent ↔ core and MCP ↔ core.

use serde::{Deserialize, Serialize};

/// Protocol version spoken by agents and the daemon.
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId {
    pub language: String,
    pub path: String,
    pub qualname: String,
}

impl SymbolId {
    pub fn new(language: impl Into<String>, path: impl Into<String>, qualname: impl Into<String>) -> Self {
        Self {
            language: language.into(),
            path: path.into(),
            qualname: qualname.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatBatch {
    pub protocol_version: u32,
    pub session_id: String,
    pub process_id: u32,
    pub window_start_ms: u64,
    pub window_end_ms: u64,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub stats: Vec<FunctionRuntimeStat>,
    #[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProbeWindowRequest {
    pub session_id: Option<String>,
    pub targets: Vec<SymbolId>,
    pub duration_s: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProbeWindowResponse {
    pub window_id: String,
    pub status: String,
    pub expires_at_ms: u64,
    pub budget: ProbeBudget,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeBudget {
    pub max_events_per_sec: u64,
    pub max_targets: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeAck {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotPathEntry {
    pub symbol: SymbolId,
    pub value: f64,
    pub metric: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionRuntimeSummary {
    pub symbol: SymbolId,
    pub invocations: u64,
    pub exceptions: u64,
    pub duration_ms_p50: f64,
    pub duration_ms_p95: f64,
    pub distinct_callers: u64,
    pub top_callers: Vec<CallerCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallerCount {
    pub qualname: String,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<SymbolId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticSummary {
    pub symbol: SymbolId,
    pub kind: String,
    pub complexity: i64,
    pub param_count: i64,
    pub lines: i64,
    pub syntactic_callee_count: i64,
    pub start_line: i64,
    pub end_line: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareStaticRuntime {
    pub static_callees: u64,
    pub observed_callees: u64,
    pub never_observed_static_callees: Vec<String>,
    pub runtime_only_callees: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralMatch {
    pub path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub matched_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralSearchResponse {
    pub language: String,
    pub match_count: usize,
    pub truncated: bool,
    pub matches: Vec<StructuralMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UncoveredSymbol {
    pub symbol: SymbolId,
    pub complexity: i64,
    pub path: String,
}
