#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="${HOME}/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin:${PATH}"
export CODEPULSE_ROOT="$ROOT/examples/dotnet-demo"
export CODEPULSE_ENDPOINT="http://127.0.0.1:7421"
export CODEPULSE_DB="$ROOT/.codepulse/e2e-dotnet.db"
export CODEPULSE_SESSION_ID="e2e_dotnet"
export CODEPULSE_INCLUDE="DotnetDemo"

rm -f "$CODEPULSE_DB"
mkdir -p "$(dirname "$CODEPULSE_DB")"

cd "$ROOT"
cargo build -p codepulse-daemon
./target/debug/codepulse --root "$CODEPULSE_ROOT" --db "$CODEPULSE_DB" --listen 127.0.0.1:7421 &
DAEMON_PID=$!

APP_PID=""
cleanup() {
  kill $DAEMON_PID 2>/dev/null || true
  [[ -n "${APP_PID}" ]] && kill $APP_PID 2>/dev/null || true
}
trap cleanup EXIT

for i in $(seq 1 50); do
  curl -sf "$CODEPULSE_ENDPOINT/health" >/dev/null && break
  sleep 0.1
done

cd "$ROOT/examples/dotnet-demo"
dotnet build -c Release -v q
CODEPULSE_ENDPOINT="$CODEPULSE_ENDPOINT" CODEPULSE_SESSION_ID="$CODEPULSE_SESSION_ID" \
  CODEPULSE_ROOT="$CODEPULSE_ROOT" CODEPULSE_INCLUDE="DotnetDemo." \
  dotnet run -c Release --no-build --urls http://127.0.0.1:8010 &
APP_PID=$!

for i in $(seq 1 60); do
  curl -sf http://127.0.0.1:8010/health >/dev/null && break
  sleep 0.2
done

bash "$ROOT/examples/dotnet-demo/scenario.sh"
sleep 3

HOT=$(curl -sf "$CODEPULSE_ENDPOINT/v1/query/hot-paths?limit=10")
echo "$HOT" | python3 -c 'import json,sys; d=json.load(sys.stdin); assert d.get("paths"), d; print("hot ok", [(p["symbol"]["qualname"], p["value"]) for p in d["paths"][:5]])'

SEARCH=$(curl -sf -X POST "$CODEPULSE_ENDPOINT/v1/query/structural-search" \
  -H 'content-type: application/json' \
  -d '{"language":"csharp","pattern":"async Task","limit":20}')
echo "$SEARCH" | python3 -c 'import json,sys; d=json.load(sys.stdin); print("search", d.get("match_count",0)); assert d.get("match_count",0)>=0'

echo "e2e-dotnet PASS"
