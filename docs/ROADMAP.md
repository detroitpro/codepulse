# Roadmap

## Phase 0 — Design (current)

- [x] Product thesis and principles
- [x] Architecture (Rust core + TypeScript MCP + plugin agents)
- [x] Data model
- [x] MCP tool contracts
- [x] Agent protocol
- [x] Repo skeleton (stubs only)

## Phase 1 — Core + Python baseline agent

1. SQLite store + migrations in `crates/store`
2. tree-sitter Python indexer writing `symbols`
3. Daemon HTTP ingest (`POST /v1/batches`)
4. Python agent: `sys.monitoring` baseline aggregation + flush
5. MCP read tools: `get_function_runtime_summary`, `get_hot_paths`, `get_static_summary`, callers/callees
6. Demo app (FastAPI or Flask) + scripted scenario

**Exit criteria:** An AI agent can answer “what was hottest?” and “who calls X?” from live data.

## Phase 2 — Adaptive probes

1. Controller + `probe_windows`
2. Agent poll/ack for `ProbeCommand`
3. MCP `enable_targeted_instrumentation` + window summary
4. Budgets and auto-disable
5. `compare_static_vs_runtime`, `list_uncovered_hot_symbols`

**Exit criteria:** Agent can deepen instrumentation for 30s and get a compact evidence summary.

## Phase 3 — Multi-runtime agents

1. Harden protocol based on Python learnings
2. Additional agents (e.g. Node, then others) as pure protocol implementers
3. Additional tree-sitter grammars for static index
4. Optional: file watch reindex, git SHA builds, compare across commits

## Explicitly deferred

- SaaS / hosted multi-tenant
- Arg/return capture
- Hot attach without restart
- Replacing OpenTelemetry
- eBPF-based agents (possible later as another plugin)
