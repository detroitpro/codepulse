# codepulse Python agent

In-process instrumentation for CPython. This is the **first target runtime agent**.

It is *not* the platform: store, indexer, controller, and MCP live in Rust / TypeScript.
This package only implements the [agent protocol](../../docs/AGENT_PROTOCOL.md) for Python
(`sys.monitoring`, aggregated batches, adaptive probe commands).

**Status:** design-phase stub. No live instrumentation yet.
