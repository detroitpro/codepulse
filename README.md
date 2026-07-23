# codepulse

Adaptive **runtime intelligence** for AI coding agents: live behavioral model + static structure, queried through a compact MCP surface.

> **Status: design phase.** Docs and stubs only — no live instrumentation yet.

**Site:** [detroitpro.github.io/codepulse](https://detroitpro.github.io/codepulse/) (static landing in [`site/`](site/), deployed via GitHub Actions)

## Why

Agents today mostly see source text. codepulse adds *what actually runs*: call counts, hot paths, observed edges, joined with AST-derived complexity — with an **adaptive probe controller** so exact instrumentation is time-boxed and budgeted.

## Stack

| Piece | Choice |
|---|---|
| Core (ingest, store, controller, indexer) | Rust |
| Static parsing | tree-sitter (Python grammar first) |
| MCP server | TypeScript |
| First runtime agent | Python ≥3.12 (`agents/python`) |

The platform language is **not** dictated by the target runtime. Runtime agents are plugins that speak a shared [agent protocol](docs/AGENT_PROTOCOL.md).

## Repo layout

```
codepulse/
  docs/                 Product & architecture design
  site/                 Landing page (GitHub Pages)
  crates/               Rust workspace (daemon + libraries)
  packages/mcp/         TypeScript MCP server stub
  agents/python/        CPython agent stub
```

## Docs

| Doc | Contents |
|---|---|
| [docs/PRODUCT.md](docs/PRODUCT.md) | Thesis, personas, wedge, non-goals |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Components, flows, overhead/privacy |
| [docs/DATA_MODEL.md](docs/DATA_MODEL.md) | SQLite schema, identity rules |
| [docs/MCP_API.md](docs/MCP_API.md) | Tool contracts |
| [docs/AGENT_PROTOCOL.md](docs/AGENT_PROTOCOL.md) | Agent ↔ daemon wire contract |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Phase 0–3 |

## Quick stubs

```bash
# Rust daemon stub
cargo run -p codepulse-daemon

# MCP stub
cd packages/mcp && npm install && npm run dev

# Python agent stub
cd agents/python && python -m codepulse_agent
```

## Principles

1. Answers over streams  
2. Adaptive cost  
3. Static + dynamic together  
4. Local-first  
5. Fail closed on privacy  
6. Runtime agents are plugins  

## License

[MIT](LICENSE)
