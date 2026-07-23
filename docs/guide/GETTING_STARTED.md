# Getting started

First-time path: run the **Python demo** under codepulse, wire MCP, ask for hot paths.

Site mirror: [Getting started](https://detroitpro.github.io/codepulse/docs/getting-started.html)

## Prerequisites

- Rust toolchain (`cargo`)
- Node ≥20
- Python ≥3.12
- Git

## 1. Clone and build the daemon

```bash
git clone https://github.com/detroitpro/codepulse.git
cd codepulse
cargo build -p codepulse-daemon
```

## 2. Start the daemon on the demo root

```bash
./target/debug/codepulse \
  --root examples/python-demo \
  --db .codepulse/dev.db \
  --listen 127.0.0.1:7420
```

Check: `curl -s http://127.0.0.1:7420/health` → `{"ok":true,...}`

## 3. Run the Python demo with the agent

In another terminal:

```bash
python3 -m venv .codepulse/venv
source .codepulse/venv/bin/activate
pip install -e agents/python "fastapi>=0.115" "uvicorn>=0.32"

export CODEPULSE_ENDPOINT=http://127.0.0.1:7420
export CODEPULSE_ROOT="$PWD/examples/python-demo"
export CODEPULSE_SESSION_ID=demo

cd examples/python-demo
PYTHONPATH="$PWD/../../agents/python/src" python - <<'PY'
from codepulse_agent import install
install()
import uvicorn
from app import app
uvicorn.run(app, host="127.0.0.1", port=8000, log_level="info")
PY
```

Drive load:

```bash
python examples/python-demo/scenario.py
# or from examples/python-demo: python scenario.py
```

## 4. Wire the MCP server (Cursor)

```bash
cd packages/mcp && npm install && npm run build
```

Add to Cursor MCP settings (adjust path):

```json
{
  "mcpServers": {
    "codepulse": {
      "command": "node",
      "args": ["/ABS/PATH/to/codepulse/packages/mcp/dist/index.js"],
      "env": {
        "CODEPULSE_ENDPOINT": "http://127.0.0.1:7420"
      }
    }
  }
}
```

## 5. First successful ask

Via MCP (or curl):

```bash
curl -s "http://127.0.0.1:7420/v1/query/hot-paths?limit=5"
```

You should see `PricingWorkflow.execute` / `inventory_reserve` after the scenario.

Or ask your agent: “What were the hottest functions in this session?”

## One-command check

```bash
./scripts/e2e-python.sh
```

Prints `e2e-python PASS` when the stack is healthy.

## Next

- [Advanced](ADVANCED.md) — structural search, probes, .NET
- [Agents](AGENTS.md) — MCP tools, bootstrap prompt, skill
