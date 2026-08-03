#!/usr/bin/env bash
# Canonical Python agent check — used by CI and gojo validationProfiles.
set -euo pipefail
cd "$(dirname "$0")/.."
VENV_DIR="${VENV_DIR:-.venv}"
python3 -m venv "$VENV_DIR"
# shellcheck disable=SC1091
source "$VENV_DIR/bin/activate"
pip install -U pip
pip install -e "agents/python[dev]"
python -c "from codepulse_agent import install; print('ok')"
