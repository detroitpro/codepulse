#!/usr/bin/env bash
# Canonical Rust check — used by CI and gojo validationProfiles.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo test --workspace
