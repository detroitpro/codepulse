#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="${HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:${PATH}"
export CODEPULSE_ROOT="$ROOT/examples/python-demo"
export CODEPULSE_ENDPOINT="http://127.0.0.1:7420"
export CODEPULSE_DB="$ROOT/.codepulse/e2e-python.db"
export CODEPULSE_SESSION_ID="e2e_python"

rm -f "$CODEPULSE_DB"
mkdir -p "$(dirname "$CODEPULSE_DB")"

cd "$ROOT"
cargo build -p codepulse-daemon

./target/debug/codepulse --root "$CODEPULSE_ROOT" --db "$CODEPULSE_DB" --listen 127.0.0.1:7420 &
DAEMON_PID=$!
cleanup() {
  kill $DAEMON_PID 2>/dev/null || true
  kill $APP_PID 2>/dev/null || true
}
trap cleanup EXIT

for i in $(seq 1 50); do
  if curl -sf "$CODEPULSE_ENDPOINT/health" >/dev/null; then
    break
  fi
  sleep 0.1
done
curl -sf "$CODEPULSE_ENDPOINT/health" >/dev/null

fuser -k 8000/tcp 2>/dev/null || true
sleep 0.5

VENV="$ROOT/.codepulse/venv-python"
python3 -m venv "$VENV"
# shellcheck disable=SC1091
source "$VENV/bin/activate"
pip install -q --upgrade pip
pip install -q -e "$ROOT/agents/python" "fastapi>=0.115" "uvicorn>=0.32" "httpx>=0.27"
python -c "import fastapi, uvicorn, codepulse_agent; print('deps ok')"

cd "$CODEPULSE_ROOT"
PYTHONPATH="$ROOT/agents/python/src:${PYTHONPATH:-}" \
  python - <<'PY' &
from codepulse_agent import install
import sys
sid = install()
print("started", sid, flush=True)
import uvicorn
from app import app
uvicorn.run(app, host="127.0.0.1", port=8000, log_level="warning")
PY
APP_PID=$!

for i in $(seq 1 80); do
  if curl -sf http://127.0.0.1:8000/health >/dev/null; then
    echo "app up"
    break
  fi
  # fail fast if app process died
  if ! kill -0 "$APP_PID" 2>/dev/null; then
    echo "app process exited early" >&2
    exit 1
  fi
  sleep 0.1
done
curl -sf http://127.0.0.1:8000/health >/dev/null

python "$CODEPULSE_ROOT/scenario.py"
sleep 4

HOT=$(curl -sf "$CODEPULSE_ENDPOINT/v1/query/hot-paths?limit=10")
echo "$HOT" | python -c 'import json,sys; d=json.load(sys.stdin); assert d.get("paths"), d; print("hot ok", [(p["symbol"]["qualname"], p["value"]) for p in d["paths"][:5]])'

SEARCH=$(curl -sf -X POST "$CODEPULSE_ENDPOINT/v1/query/structural-search" \
  -H 'content-type: application/json' \
  -d '{"language":"python","pattern":"async def $NAME($$$ARGS): $$$BODY","limit":20}')
echo "$SEARCH" | python -c 'import json,sys; d=json.load(sys.stdin); assert d["match_count"]>=1, d; print("search ok", d["match_count"])'

# probe window
PROBE=$(curl -sf -X POST "$CODEPULSE_ENDPOINT/v1/probe-windows" \
  -H 'content-type: application/json' \
  -d "{\"session_id\":\"$CODEPULSE_SESSION_ID\",\"duration_s\":5,\"targets\":[{\"language\":\"python\",\"path\":\"app.py\",\"qualname\":\"PricingWorkflow.execute\"}]}")
echo "$PROBE" | python -c 'import json,sys; d=json.load(sys.stdin); assert d.get("window_id"); print("probe ok", d["window_id"])'
sleep 3
python "$CODEPULSE_ROOT/scenario.py"
sleep 3

CALLERS=$(curl -sf "$CODEPULSE_ENDPOINT/v1/query/callers?language=python&path=app.py&qualname=PricingWorkflow.execute&session_id=$CODEPULSE_SESSION_ID")
echo "$CALLERS" | python -c 'import json,sys; d=json.load(sys.stdin); print("callers", d)'

echo "e2e-python PASS"
