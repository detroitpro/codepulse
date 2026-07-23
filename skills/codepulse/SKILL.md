---
name: codepulse
description: >-
  Use codepulse for runtime intelligence: hot paths, actual callers, structural
  AST search, and adaptive probes via MCP. Apply when the user asks what code
  actually ran, who called a function at runtime, or how to set up the daemon /
  MCP / Python or .NET agent.
---

# codepulse

Local-first adaptive runtime intelligence for AI coding agents. Daemon (Rust) + MCP (TypeScript) + language runtime agents. Answers over streams; no args/returns/locals by default.

## When to use

- “What actually ran?” / hottest functions / session overview
- Actual callers (not static call graph)
- Structural / AST pattern search in the workspace
- Compare static declaration vs runtime observation
- Short targeted probe windows when edges are thin
- Setting up codepulse in a project (daemon, MCP, agent)

## Setup (current project)

1. Ensure a codepulse clone is built: `cargo build -p codepulse-daemon`
2. Start daemon on **this** workspace root:
   `codepulse --root <WORKSPACE> --db .codepulse/<name>.db --listen 127.0.0.1:7420`
3. Verify `GET /health`
4. MCP: `packages/mcp` built; host env `CODEPULSE_ENDPOINT=http://127.0.0.1:7420`
5. Runtime agent matching the app language:
   - **Python ≥3.12:** `codepulse_agent.install()` at process start; `CODEPULSE_ROOT`, `CODEPULSE_SESSION_ID`
   - **.NET:** `CodePulseAgent.Install("Namespace")`; `CODEPULSE_INCLUDE`
6. Exercise the app, then query tools

User guides: `docs/guide/GETTING_STARTED.md`, `docs/guide/ADVANCED.md`, `docs/guide/AGENTS.md`.  
API: `docs/MCP_API.md`.

## Tool map

| Tool | Use when… |
|---|---|
| `get_session_overview` | Orient before deep questions |
| `get_hot_paths` | Rank executed functions |
| `get_actual_callers` | Runtime callers of a symbol |
| `get_function_runtime_summary` | Per-function stats |
| `structural_search` | AST pattern match (Python/C# indexer; not ast-grep CLI) |
| `compare_static_vs_runtime` | Declared vs observed |
| `enable_targeted_instrumentation` | Temporary deeper collection |
| `list_uncovered_hot_symbols` | Complex static, zero hits |

## Example workflow

1. `structural_search` (or name) → candidates  
2. `get_hot_paths` / `get_function_runtime_summary` → evidence they ran  
3. `get_actual_callers` → who called them  
4. Thin edges → `enable_targeted_instrumentation` → reproduce → re-query  
5. Answer with concrete symbols and relative evidence; prefer summaries over dumps

## Constraints

- Local-first; do not assume cloud SaaS
- Never request or invent args, returns, or locals
- Prefer MCP answers over raw HTTP event streams
- Structural search is daemon/indexer-backed, not the runtime agent wire
- Point `--root` / `CODEPULSE_ROOT` at the same app root for path alignment
