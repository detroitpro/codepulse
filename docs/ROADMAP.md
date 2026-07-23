# Roadmap

## Phase 0 — Design

- [x] Product thesis and principles
- [x] Architecture (Rust core + TypeScript MCP + plugin agents)
- [x] Data model
- [x] MCP tool contracts (including `structural_search`)
- [x] Agent protocol (structural search explicitly out of agent wire)
- [x] Repo skeleton

## Phase 1 — Core + Python baseline agent

- [x] SQLite store + migrations in `crates/store`
- [x] tree-sitter Python indexer writing `symbols`
- [x] Indexer pattern engine + MCP `structural_search`
- [x] Daemon HTTP ingest (`POST /v1/batches`) + query API
- [x] Python agent: `sys.monitoring` baseline aggregation + flush
- [x] MCP tools via `@modelcontextprotocol/sdk`
- [x] Demo app (FastAPI) + `scripts/e2e-python.sh`

## Phase 2 — Adaptive probes

- [x] Controller + `probe_windows`
- [x] Agent poll/ack for `ProbeCommand`
- [x] MCP `enable_targeted_instrumentation`
- [x] Budgets and auto-disable
- [x] `compare_static_vs_runtime`, `list_uncovered_hot_symbols`

## Phase 3 — Multi-runtime agents

- [x] .NET agent (`agents/dotnet`, Lib.Harmony) + C# grammar in indexer
- [x] `examples/dotnet-demo` + `scripts/e2e-dotnet.sh`
- [ ] Additional agents (e.g. Node) as pure protocol implementers
- [ ] Optional: file watch reindex, git SHA builds, compare across commits

## Explicitly deferred

- SaaS / hosted multi-tenant
- Arg/return capture
- Hot attach without restart
- Replacing OpenTelemetry
- eBPF-based agents (possible later as another plugin)
- Structural rewrite/codemod / YAML rule packs
- External `ast-grep` dependency
