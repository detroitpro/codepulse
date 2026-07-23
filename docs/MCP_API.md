# MCP API

TypeScript MCP server in `packages/mcp`. Talks to the Rust daemon over local RPC.
Default responses are **compact summaries** — never raw per-call traces.

## Tools (v1)

### `get_function_runtime_summary`

Return observed runtime stats for a symbol.

**Input**

```json
{
  "language": "python",
  "path": "src/app/pricing.py",
  "qualname": "PricingWorkflow.execute",
  "session_id": "optional",
  "window_minutes": 15
}
```

**Output**

```json
{
  "symbol": { "language": "python", "path": "src/app/pricing.py", "qualname": "PricingWorkflow.execute" },
  "invocations": 148221,
  "exceptions": 117,
  "duration_ms_p50": 12.4,
  "duration_ms_p95": 42.0,
  "distinct_callers": 6,
  "top_callers": [
    { "qualname": "api.handle_checkout", "count": 90000 }
  ]
}
```

### `get_actual_callers` / `get_actual_callees`

**Input:** symbol identity + optional limit (default 20).

**Output:** ranked list of `{ symbol, count }`.

### `get_hot_paths`

**Input**

```json
{
  "session_id": "optional",
  "limit": 20,
  "metric": "invocations" 
}
```

`metric`: `invocations` | `duration_p95` | `exceptions`.

**Output:** ranked symbols with metric values.

### `get_static_summary`

**Input:** symbol identity.

**Output:** complexity, param_count, lines, syntactic_callee_count, kind.

### `compare_static_vs_runtime`

**Input:** symbol identity (or path prefix).

**Output**

```json
{
  "static_callees": 23,
  "observed_callees": 6,
  "never_observed_static_callees": ["…"],
  "runtime_only_callees": ["…"]
}
```

### `enable_targeted_instrumentation`

**Input**

```json
{
  "targets": [
    { "language": "python", "path": "src/app/pricing.py", "qualname": "PricingWorkflow.execute" }
  ],
  "duration_s": 30
}
```

**Output**

```json
{
  "window_id": "…",
  "status": "active",
  "expires_at_ms": 0,
  "budget": { "max_events_per_sec": 50000, "max_targets": 32 }
}
```

When the window completes, a follow-up read via `get_function_runtime_summary` (or a dedicated `get_probe_window_summary` in Phase 2) returns deepened stats.

### `list_uncovered_hot_symbols`

Symbols that are statically notable (e.g. complexity ≥ N) but have **zero** runtime invocations in the session.

**Input:** `{ "min_complexity": 10, "limit": 20 }`

**Output:** ranked static symbols with complexity and path.

## Example agent workflow

1. `get_hot_paths` → find suspects.
2. `get_actual_callers` on a suspect → understand fan-in.
3. `compare_static_vs_runtime` → see dynamic vs declared.
4. If edges are thin: `enable_targeted_instrumentation` for 30s while reproducing.
5. Re-query summaries; propose a code change with evidence.

## Non-goals for MCP responses

- Streaming individual call events to the model
- Dumping full SQL/HTTP payloads
- Returning argument or return values
