# codepulse

Adaptive **runtime intelligence** for AI coding agents: live behavioral model + static structure (including indexer-backed structural search), queried through a compact MCP surface.

> **Status:** implementable MVP — Rust daemon, MCP server, Python + .NET agents, demos and E2E scripts.

**Site:** [detroitpro.github.io/codepulse](https://detroitpro.github.io/codepulse/) · [alpine-realm-ney2.here.now](https://alpine-realm-ney2.here.now/) (static landing in [`site/`](site/))

## Why

Agents today mostly see source text. codepulse adds *what actually runs*: call counts, hot paths, observed edges, joined with AST-derived complexity — plus **structural pattern search** via the tree-sitter indexer (ast-grep-like questions over MCP, no separate tool) — with an **adaptive probe controller** so exact instrumentation is time-boxed and budgeted.

Example structural asks: “find all async defs”, “where is `requests.get` called?”, “show bare `except:`”, “find `open(...)` sites”, “locate `execute` methods” — then join to runtime tools. Pattern table: [docs/MCP_API.md](docs/MCP_API.md#structural_search).

## Stack

| Piece | Choice |
|---|---|
| Core (ingest, store, controller, indexer) | Rust (axum + SQLite) |
| Static parsing + structural search | tree-sitter (Python + C#) |
| MCP server | TypeScript (`@modelcontextprotocol/sdk`) |
| Runtime agents | Python ≥3.12 (`sys.monitoring`) · .NET (`Lib.Harmony`) |

The platform language is **not** dictated by the target runtime. Runtime agents are plugins that speak a shared [agent protocol](docs/AGENT_PROTOCOL.md).

## Quickstart

```bash
# 1) Daemon (indexes --root, listens on :7420)
cargo run -p codepulse-daemon -- --root examples/python-demo --db .codepulse/dev.db

# 2) MCP server (another terminal)
cd packages/mcp && npm install && npm run dev
# point your AI host at this stdio server; CODEPULSE_ENDPOINT=http://127.0.0.1:7420

# 3) Python demo under the agent
pip install -e "agents/python[dev]"
CODEPULSE_ROOT=$PWD/examples/python-demo CODEPULSE_ENDPOINT=http://127.0.0.1:7420 \
  python -c "from codepulse_agent import install; install(); import uvicorn; from app import app" \
  # or use scripts/e2e-python.sh
```

E2E:

```bash
chmod +x scripts/*.sh examples/dotnet-demo/scenario.sh
./scripts/e2e-python.sh
./scripts/e2e-dotnet.sh
```

## Repo layout

```
codepulse/
  docs/                 Design docs + user guide (docs/guide/)
  site/                 Landing + static user docs (GitHub Pages)
  skills/codepulse/     Agent Skill (SKILL.md)
  crates/               Rust workspace (daemon + libraries)
  packages/mcp/         TypeScript MCP server
  agents/python/        CPython agent
  agents/dotnet/        .NET agent (Harmony)
  examples/             python-demo + dotnet-demo
  scripts/              e2e-python.sh / e2e-dotnet.sh
```

**Agent Skill:** [`skills/codepulse`](skills/codepulse) — teach agents when to use runtime tools and how to set up the stack.

## Docs

### User guide

| Doc | Contents |
|---|---|
| [docs/guide/GETTING_STARTED.md](docs/guide/GETTING_STARTED.md) | Clone, daemon, Python demo, MCP, first hot paths |
| [docs/guide/ADVANCED.md](docs/guide/ADVANCED.md) | Structural search, probes, .NET, env, troubleshooting |
| [docs/guide/AGENTS.md](docs/guide/AGENTS.md) | MCP config, tool map, copy-paste bootstrap prompt |

Site: [Getting started](https://detroitpro.github.io/codepulse/docs/getting-started.html) · [Advanced](https://detroitpro.github.io/codepulse/docs/advanced.html) · [Agents](https://detroitpro.github.io/codepulse/docs/agents.html)

### Design

| Doc | Contents |
|---|---|
| [docs/PRODUCT.md](docs/PRODUCT.md) | Thesis, personas, wedge, non-goals |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Components, flows, overhead/privacy |
| [docs/DATA_MODEL.md](docs/DATA_MODEL.md) | SQLite schema, identity rules |
| [docs/MCP_API.md](docs/MCP_API.md) | Tool contracts |
| [docs/AGENT_PROTOCOL.md](docs/AGENT_PROTOCOL.md) | Agent ↔ daemon wire contract |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phase 0–3 |

## Principles

1. Answers over streams  
2. Adaptive cost  
3. Static + dynamic together (catalog + structural search in the indexer)  
4. Local-first  
5. Fail closed on privacy  
6. Runtime agents are plugins (pattern search is not on the agent wire)  

## License

[MIT](LICENSE)
