#!/usr/bin/env bash
set -euo pipefail
BASE="${1:-http://127.0.0.1:8010}"
for i in $(seq 1 40); do
  curl -sf -X POST "$BASE/checkout" \
    -H 'content-type: application/json' \
    -d "{\"total\": $((10+i)), \"sku\": \"SKU-1\"}" >/dev/null
done
echo "dotnet scenario complete"
