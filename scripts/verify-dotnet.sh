#!/usr/bin/env bash
# Canonical .NET agent check — used by CI and gojo validationProfiles.
set -euo pipefail
cd "$(dirname "$0")/../agents/dotnet"
dotnet test -c Release
