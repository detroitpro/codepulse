# Product

## Problem

AI coding agents today reason almost entirely from **source text** (and occasionally LSP symbols). That misses what matters when changing real systems:

- Which functions actually run in this workflow?
- Who calls this method *in practice*?
- Where is time and failure concentrated?
- What is complex in the AST but never hit at runtime (or the reverse)?

Profilers and APM tools answer parts of this for humans staring at dashboards. They are not shaped as a **queryable, adaptive knowledge surface for agents**.

## Vision

**codepulse** is an adaptive runtime-intelligence platform for AI agents:

1. Build a **live behavioral model** of a running application (calls, hot paths, errors, fan-in/out).
2. Join it with **static structure** (symbols, complexity, syntactic callees).
3. Answer **structural pattern questions** over the workspace via the tree-sitter indexer (ast-grep-like patterns, codepulse-owned API — no separate search tool).
4. Expose a **compact MCP query surface** — answers, not firehoses.
5. Use an **adaptive probe controller**: cheap always-on sampling by default; exact instrumentation only when the agent asks, for a bounded window.

## Personas

| Persona | Job |
|---|---|
| **AI coding agent** (primary) | Ask semantic questions: “what actually calls X?”, “what got hotter after this change?”, “find async defs matching this shape”, “where should I look next?” |
| **Developer** (secondary) | Run codepulse locally while developing/testing; keep overhead low; trust privacy defaults |

## Jobs to be done

1. As an agent, I can get a **runtime summary** for a function without reading megabytes of traces.
2. As an agent, I can deepen instrumentation on a **small target set** when baseline data is insufficient.
3. As an agent, I can compare **static vs observed** structure (declared callees vs real edges).
4. As an agent (or human via MCP), I can run **structural searches** — e.g. find async functions or call shapes — without installing a separate AST tool.
5. As a developer, I can enable codepulse on a Python app with **bounded overhead** and no SaaS requirement.

## Competitive wedge

| Approach | Gap |
|---|---|
| Source-only agents | No behavioral truth |
| AppMap-class recorders | Powerful, but often “record scenarios” / commercial; not adaptive probe-budget first |
| Digma / OTel-centric | Great for spans/services; not every-method adaptive exact counts |
| Continuous profilers | Hot paths via sampling; not exact invocation graphs for agents |
| Frida / DBI | Hot attach foundations; no agent knowledge product |

**codepulse differentiates by:** open, local-first, agent-first MCP, adaptive cost model, static+dynamic join, indexer-backed structural search, runtime agents as plugins.

## Product principles

1. **Answers over streams** — agents get summaries, not firehoses.
2. **Adaptive cost** — overhead is a budget the controller owns.
3. **Static + dynamic together** — neither alone is enough; static includes catalog *and* pattern search.
4. **Local-first** — works offline in a repo; cloud optional later.
5. **Fail closed on privacy** — no args/returns by default.
6. **Runtime agents are plugins** — platform stack never follows the target language; structural search stays in the indexer.

## Non-goals (near term)

- Multi-language *runtime* agents beyond Python (static multi-lang + structural search per grammar is designed in)
- Hosted SaaS / multi-tenant UI
- Capturing argument and return values
- Replacing OpenTelemetry APM
- Hot-attaching into arbitrary processes without restart (v1 starts with the agent loaded)
- Structural rewrite/codemod (search-only in v1)
- Depending on an external `ast-grep` CLI

## Success criteria (MVP)

An AI agent connected via MCP can, against a demo Python app with codepulse running:

1. Name the hottest functions in a request/test session.
2. List actual callers of a chosen function.
3. Enable targeted instrumentation, wait, and receive a compact deepened summary.
4. Surface symbols that are statically complex but never observed at runtime.
5. Run a `structural_search` pattern (e.g. async function shape) and get a capped match list from the indexer.
