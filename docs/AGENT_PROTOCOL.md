# Agent protocol

Language-agnostic contract between **runtime agents** and the **Rust daemon**.
The Python agent (`agents/python`) is the first implementer. Future `agents/node`,
`agents/dotnet`, etc. must speak the same protocol.

Protocol version: **1** (`PROTOCOL_VERSION` in `crates/protocol`).

## Transport (MVP)

- Default: HTTP on localhost (e.g. `http://127.0.0.1:7420`)
- Optional later: Unix domain socket
- Content-Type: `application/json`

Environment variables (proposed):

| Variable | Meaning |
|---|---|
| `CODEPULSE_ENDPOINT` | Base URL of daemon |
| `CODEPULSE_SESSION_ID` | Stable id for this process session |
| `CODEPULSE_MODE` | `off` / `baseline` / `targeted` (controller may override) |

## Endpoints

### `POST /v1/batches`

Agent → daemon. Aggregated runtime stats for a time window.

Body: `RuntimeStatBatch`

```json
{
  "protocol_version": 1,
  "session_id": "sess_…",
  "process_id": 12345,
  "window_start_ms": 0,
  "window_end_ms": 1000,
  "stats": [
    {
      "symbol": {
        "language": "python",
        "path": "src/app/pricing.py",
        "qualname": "PricingWorkflow.execute"
      },
      "invocations": 42,
      "exceptions": 1,
      "duration_ns_p50": 12000000,
      "duration_ns_p95": 40000000
    }
  ],
  "edges": [
    {
      "caller": {
        "language": "python",
        "path": "src/app/api.py",
        "qualname": "handle_checkout"
      },
      "callee": {
        "language": "python",
        "path": "src/app/pricing.py",
        "qualname": "PricingWorkflow.execute"
      },
      "count": 42
    }
  ]
}
```

**Responses**

- `204` success
- `400` invalid payload
- `409` protocol version unsupported
- `503` store unavailable — agent should shed / retry with backoff

### `GET /v1/probe-commands?session_id=…`

Agent polls (or later: websocket/SSE) for probe commands.

Response:

```json
{
  "commands": [
    {
      "protocol_version": 1,
      "window_id": "pw_…",
      "action": "enable",
      "targets": [
        {
          "language": "python",
          "path": "src/app/pricing.py",
          "qualname": "PricingWorkflow.execute"
        }
      ],
      "duration_s": 30
    }
  ]
}
```

`action`: `enable` | `disable`.

### `POST /v1/probe-windows/{window_id}/ack`

Agent acknowledges enable/disable or budget breach.

```json
{
  "status": "active" 
}
```

`status`: `active` | `disabled` | `budget_exceeded` | `error`.

## Agent responsibilities

1. **Never crash the host app** due to codepulse errors.
2. Emit **aggregates only** (no per-call streams to the daemon by default).
3. Respect **budgets** (max events/sec, max targets); auto-disable and ack `budget_exceeded`.
4. Map frames to `SymbolId` using the same path/qualname rules as the indexer when possible.
5. **Do not** capture args/returns/locals unless a future explicit opt-in protocol exists.

## Modes

| Mode | Behavior |
|---|---|
| `baseline` | Low-overhead sampling or coarse counts; edges may be partial |
| `targeted` | Exact counts + edges for listed targets only, time-boxed |
| `off` | No instrumentation; agent idle |

## Privacy

Default payload fields are counts, durations, and symbol identities only.
Forbidden without explicit future protocol + user opt-in: arguments, return values, locals, secrets, raw SQL with literals.
