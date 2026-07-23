# Agent integration

Configure AI hosts (Cursor, Claude Code, etc.) to use codepulse via MCP. Includes a copy-paste bootstrap prompt.

Site mirror: [Agents](https://detroitpro.github.io/codepulse/docs/agents.html)

**Agent Skill (repo):** [`skills/codepulse/SKILL.md`](../../skills/codepulse/SKILL.md) — teach agents when and how to use runtime tools.

## One-liner (short)

> Set up codepulse in this repo: clone detroitpro/codepulse, run the daemon on this workspace root, wire the MCP server, install the Python or .NET agent, then use `get_hot_paths` / `structural_search`.

## Full bootstrap prompt

Copy into a new agent chat for the *current* project:

```
You are setting up codepulse (https://github.com/detroitpro/codepulse) for THIS repository so I can ask runtime questions via MCP.

Do the following:

1. Detect the primary language (Python ≥3.12 preferred for first setup; .NET if this is a C# app).
2. If codepulse is not already available locally, clone https://github.com/detroitpro/codepulse.git to a sensible path (or use an existing clone). Build: `cargo build -p codepulse-daemon`.
3. Start the daemon against THIS workspace root:
   `./target/debug/codepulse --root <THIS_REPO_ROOT> --db <clone>/.codepulse/<name>.db --listen 127.0.0.1:7420`
   Verify: `curl -s http://127.0.0.1:7420/health`
4. Install MCP: in `packages/mcp`, `npm install && npm run build`. Add Cursor MCP config:
   {
     "mcpServers": {
       "codepulse": {
         "command": "node",
         "args": ["<ABS>/packages/mcp/dist/index.js"],
         "env": { "CODEPULSE_ENDPOINT": "http://127.0.0.1:7420" }
       }
     }
   }
5. Install the matching runtime agent:
   - Python: `pip install -e <clone>/agents/python`, call `codepulse_agent.install()` at process start,
     set CODEPULSE_ENDPOINT, CODEPULSE_ROOT=<THIS_REPO_ROOT>, CODEPULSE_SESSION_ID.
   - .NET: reference CodePulse.Agent, call `CodePulseAgent.Install("Your.Namespace")`,
     set CODEPULSE_INCLUDE to that namespace prefix.
6. Tell me how to exercise the app so events appear. Then verify with get_session_overview / get_hot_paths
   (or curl /v1/query/hot-paths). Prefer answers over raw dumps.

Constraints: local-first; never request args/returns/locals; use structural_search for AST patterns
(indexer, not ast-grep CLI). Skill: skills/codepulse/SKILL.md in the codepulse repo.
Docs: docs/guide/GETTING_STARTED.md, docs/MCP_API.md.
```

## Install script snippet

Agents/users can adapt this (set `APP_ROOT` to the project you care about):

```bash
CODEPULSE_HOME="${CODEPULSE_HOME:-$HOME/src/codepulse}"
APP_ROOT="${APP_ROOT:-$PWD}"

if [ ! -d "$CODEPULSE_HOME/.git" ]; then
  git clone https://github.com/detroitpro/codepulse.git "$CODEPULSE_HOME"
fi
cd "$CODEPULSE_HOME"
cargo build -p codepulse-daemon
(cd packages/mcp && npm install && npm run build)

# start daemon (foreground) — use a second terminal for the app
./target/debug/codepulse \
  --root "$APP_ROOT" \
  --db "$CODEPULSE_HOME/.codepulse/app.db" \
  --listen 127.0.0.1:7420
```

Python agent (separate terminal, after daemon is up):

```bash
export CODEPULSE_ENDPOINT=http://127.0.0.1:7420
export CODEPULSE_ROOT="$APP_ROOT"
export CODEPULSE_SESSION_ID=dev
# pip install -e "$CODEPULSE_HOME/agents/python"
# then install() at app start
```

## Cursor MCP config

Same JSON as in [Getting started](GETTING_STARTED.md#4-wire-the-mcp-server-cursor). Restart Cursor / reload MCP after changes.

## Tool catalog — when to use which

| Tool | Use when… |
|---|---|
| `get_session_overview` | Orient: sessions, coverage, top hot paths |
| `get_hot_paths` | “What ran the most?” |
| `get_actual_callers` | Real callers of a symbol (not static graph) |
| `get_function_runtime_summary` | Stats for one function |
| `structural_search` | Find code by AST pattern (Python/C#) |
| `compare_static_vs_runtime` | Declared vs observed for a symbol |
| `enable_targeted_instrumentation` | Need deeper edges for a short window |
| `list_uncovered_hot_symbols` | Complex static symbols never hit |

Full schemas: [MCP_API.md](../MCP_API.md).

## Suggested workflow

1. `structural_search` or name guess → candidate symbols  
2. `get_hot_paths` / `get_function_runtime_summary` → did they run?  
3. `get_actual_callers` → who called them?  
4. If thin: `enable_targeted_instrumentation` → reproduce → re-query  
5. Answer with concrete functions and relative evidence; do not paste huge event dumps

## Related

- [Getting started](GETTING_STARTED.md)
- [Advanced](ADVANCED.md)
- [Architecture](../ARCHITECTURE.md)
