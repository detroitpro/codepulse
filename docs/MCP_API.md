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

### `structural_search`

On-demand AST pattern match over the workspace via the Rust tree-sitter indexer.
No `ast-grep` dependency. Pattern language is **codepulse-owned** (metavariable style
inspired by ast-grep; not wire-compatible with ast-grep).

**Input**

```json
{
  "language": "python",
  "pattern": "async def $NAME($$$ARGS):\n    $$$BODY",
  "path_prefix": "optional/repo/relative",
  "limit": 50
}
```

| Field | Notes |
|---|---|
| `language` | Selects the tree-sitter grammar (Python first; expands with indexer grammars) |
| `pattern` | Code snippet with metavariables: `$NAME` = one node, `$$$ARGS` = node list |
| `path_prefix` | Optional repo-relative scope; default = workspace root |
| `limit` | Max matches (hard-capped server-side; default 50) |

**Output**

```json
{
  "language": "python",
  "match_count": 2,
  "truncated": false,
  "matches": [
    {
      "path": "src/app/api.py",
      "start_line": 40,
      "end_line": 52,
      "matched_text": "async def handle_checkout(...):\n    ..."
    }
  ]
}
```

`matched_text` is truncated. Payload stays compact (answers over streams).

**Errors (structured):** unsupported language, invalid pattern, indexer not ready.

**Non-goals (v1):** rewrite/codemod, YAML multi-rule packs, lint autofix.

#### Example questions → patterns (Python)

Natural-language asks agents (or humans) can make; the host maps them to `structural_search`:

| Question | Example pattern |
|---|---|
| Find all async function definitions | `async def $NAME($$$ARGS): $$$BODY` |
| Find bare `except:` clauses | `except: $$$BODY` |
| Find `print(...)` calls (any arity) | `print($$$ARGS)` |
| Find calls to `requests.get(...)` | `requests.get($$$ARGS)` |
| Find `open(...)` without a context manager nearby (shape hunt) | `open($$$ARGS)` |
| Find class methods named `execute` | `def execute($$$ARGS): $$$BODY` |
| Find `@app.route` / decorator + function pairs | `@$DECORATOR\ndef $NAME($$$ARGS): $$$BODY` |
| Find `raise` of a specific exception type | `raise HTTPException($$$ARGS)` |
| Find `await` of a named call | `await $FUNC($$$ARGS)` |
| Find assignments to a settings-like name | `$NAME = os.environ[$KEY]` |

Typical follow-up with runtime tools: take a match’s `path` / inferred qualname → `get_function_runtime_summary` or `get_hot_paths` to see whether that shape actually ran.

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

1. `structural_search` → find candidate shapes in source (e.g. async handlers without a try).
2. `get_hot_paths` → see which of those (or nearby) actually ran.
3. `get_actual_callers` on a suspect → understand fan-in.
4. `compare_static_vs_runtime` → see dynamic vs declared.
5. If edges are thin: `enable_targeted_instrumentation` for 30s while reproducing.
6. Re-query summaries; propose a code change with evidence.

## Non-goals for MCP responses

- Streaming individual call events to the model
- Dumping full SQL/HTTP payloads
- Returning argument or return values
- Full-file dumps from `structural_search` (matches are capped and truncated)
