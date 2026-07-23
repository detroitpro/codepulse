# Advanced usage

Power-user patterns beyond the Python demo.

Site mirror: [Advanced](https://detroitpro.github.io/codepulse/docs/advanced.html)

## Structural search

Indexer-backed AST patterns (no `ast-grep` CLI). Via MCP `structural_search` or:

```bash
curl -s -X POST http://127.0.0.1:7420/v1/query/structural-search \
  -H 'content-type: application/json' \
  -d '{"language":"python","pattern":"async def $NAME($$$ARGS): $$$BODY","limit":20}'
```

Useful asks → patterns: see [MCP_API.md](../MCP_API.md#structural_search).

After matches, join to runtime: `get_hot_paths`, `get_actual_callers`, `get_function_runtime_summary`.

## Adaptive probe windows

When baseline edges are thin:

1. MCP `enable_targeted_instrumentation` with symbol targets + `duration_s` (e.g. 30)
2. Reproduce the workload while the window is active
3. Re-query callers / summaries

Agents poll `GET /v1/probe-commands` and ack; the controller owns budgets (max 32 targets, 50k events/s).

## .NET agent

```bash
# Terminal 1 — daemon on the .NET demo
./target/debug/codepulse \
  --root examples/dotnet-demo \
  --db .codepulse/dotnet.db \
  --listen 127.0.0.1:7421

# Terminal 2 — demo (Harmony patches DotnetDemo.*)
export CODEPULSE_ENDPOINT=http://127.0.0.1:7421
export CODEPULSE_SESSION_ID=dotnet_demo
export CODEPULSE_INCLUDE=DotnetDemo
export CODEPULSE_ROOT="$PWD/examples/dotnet-demo"
cd examples/dotnet-demo && dotnet run --urls http://127.0.0.1:8010

# Terminal 3 — load
./examples/dotnet-demo/scenario.sh
```

Or: `./scripts/e2e-dotnet.sh`

In your app: call `CodePulseAgent.Install("Your.Namespace")` early, set `CODEPULSE_INCLUDE` to match.

## Point the daemon at *your* project

```bash
./target/debug/codepulse \
  --root /path/to/your/app \
  --db .codepulse/myapp.db \
  --listen 127.0.0.1:7420
```

Python agent: set `CODEPULSE_ROOT` to that same root so symbol paths stay repo-relative.

Reindex after big changes: `curl -X POST http://127.0.0.1:7420/v1/reindex`

## Environment variables

| Variable | Used by | Meaning |
|---|---|---|
| `CODEPULSE_ENDPOINT` | agents, MCP | Daemon base URL (default `http://127.0.0.1:7420`) |
| `CODEPULSE_ROOT` | agents, daemon `--root` | Workspace root for paths / indexing |
| `CODEPULSE_SESSION_ID` | agents | Stable session id for queries |
| `CODEPULSE_INCLUDE` | .NET agent | Namespace prefix to patch |
| `CODEPULSE_MODE` | agents | `baseline` / `targeted` / `off` |
| `CODEPULSE_DB` | daemon | SQLite path |
| `CODEPULSE_LISTEN` | daemon | Bind address |

## Multi-session

Each process can use its own `CODEPULSE_SESSION_ID`. Query tools accept `session_id` to filter; omit it to aggregate across sessions in the DB.

## Troubleshooting

| Symptom | Check |
|---|---|
| Empty `hot-paths` | Agent installed *before* app code runs; workload actually hit instrumented functions; `CODEPULSE_ROOT` matches daemon `--root` |
| MCP tools error | Daemon up (`/health`); `CODEPULSE_ENDPOINT` in MCP env |
| Structural search “unsupported language” | File extension / grammar (`.py` or `.cs`); daemon indexed that root |
| .NET patched 0 methods | `CODEPULSE_INCLUDE` matches namespace (e.g. `DotnetDemo` not `DotnetDemo.` only); call `Install` after types are loadable |
| Python monitoring silent | Python ≥3.12; free `sys.monitoring` tool id; paths under `CODEPULSE_ROOT` |

## Related

- [Getting started](GETTING_STARTED.md)
- [Agents](AGENTS.md)
- [MCP API](../MCP_API.md)
