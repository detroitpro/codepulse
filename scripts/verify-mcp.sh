#!/usr/bin/env bash
# Canonical MCP package check — used by CI and gojo validationProfiles.
set -euo pipefail
cd "$(dirname "$0")/../packages/mcp"
npm install
npm run build
