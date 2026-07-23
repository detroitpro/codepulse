# Data model

Local-first SQLite store. Conceptual schema for Phase 1+.

## Identity

### `SymbolId` (logical)

```json
{
  "language": "python",
  "path": "src/app/pricing.py",
  "qualname": "PricingWorkflow.execute"
}
```

Rules:

- `path` is repo-relative, POSIX separators, normalized.
- `qualname` is language-specific but stable (Python: dotted qualname from module/class/function nesting).
- Nested / lambdas: include disambiguator when needed (`fn.<locals>.helper`).
- Never use raw pointer / code object id as durable key.

## Tables

### `builds`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | UUID |
| `git_sha` | TEXT NULL | Optional |
| `indexed_at_ms` | INTEGER | |
| `root_path` | TEXT | Absolute or workspace root |

### `symbols`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | Hash of language+path+qualname |
| `build_id` | TEXT FK | |
| `language` | TEXT | |
| `path` | TEXT | |
| `qualname` | TEXT | |
| `kind` | TEXT | `function` / `method` / `class` / … |
| `start_line` | INTEGER | |
| `end_line` | INTEGER | |
| `param_count` | INTEGER | |
| `complexity` | INTEGER | Cyclomatic (static) |
| `syntactic_callee_count` | INTEGER | Optional rollup |

Unique: `(build_id, language, path, qualname)`.

### `runtime_sessions`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | Agent session id |
| `started_at_ms` | INTEGER | |
| `ended_at_ms` | INTEGER NULL | |
| `language` | TEXT | |
| `process_id` | INTEGER | |
| `command` | TEXT NULL | How the process was started |

### `runtime_stats`

Rolling / windowed aggregates keyed by symbol.

| Column | Type | Notes |
|---|---|---|
| `session_id` | TEXT FK | |
| `symbol_id` | TEXT FK | |
| `window_start_ms` | INTEGER | |
| `window_end_ms` | INTEGER | |
| `invocations` | INTEGER | |
| `exceptions` | INTEGER | |
| `duration_ns_p50` | INTEGER | |
| `duration_ns_p95` | INTEGER | |

Primary key: `(session_id, symbol_id, window_start_ms)`.

### `edges`

Observed caller → callee.

| Column | Type | Notes |
|---|---|---|
| `session_id` | TEXT FK | |
| `caller_symbol_id` | TEXT FK | |
| `callee_symbol_id` | TEXT FK | |
| `window_start_ms` | INTEGER | |
| `count` | INTEGER | |

### `probe_windows`

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | |
| `session_id` | TEXT FK | |
| `started_at_ms` | INTEGER | |
| `duration_s` | INTEGER | |
| `ended_at_ms` | INTEGER NULL | |
| `status` | TEXT | `active` / `completed` / `budget_exceeded` / `cancelled` |
| `targets_json` | TEXT | JSON array of SymbolId |

### `static_edges` (optional Phase 1)

Syntactic call edges from the indexer (may be incomplete for dynamic calls).

| Column | Type | Notes |
|---|---|---|
| `build_id` | TEXT FK | |
| `caller_symbol_id` | TEXT FK | |
| `callee_symbol_id` | TEXT FK | |
| `uncertain` | INTEGER | 1 if resolved heuristically |

## Aggregation windows

- Agent default flush: **1–5 seconds**.
- Store may roll up short windows into **1 minute** retention tiers later.
- MCP queries default to **current session** + last N minutes.

## Join semantics

`compare_static_vs_runtime` joins on `symbols.id` derived from the same identity rules as agent-emitted `SymbolId`. Mismatches (e.g. generated code) surface as unmatched runtime-only or static-only rows.
