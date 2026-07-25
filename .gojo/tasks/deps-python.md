# Python agent dependency maintenance

Unattended gojo task for **codepulse** (Python runtime agent). Keep `agents/python` deps current within safe ranges; no product features.

## Goals

1. Identify outdated dependencies for `agents/python`.
2. Apply safe patch/minor upgrades (update floors in `agents/python/pyproject.toml` when appropriate).
3. Fix import/runtime breaks caused by those upgrades.
4. Leave the agent ready for gojo validation (venv install + import smoke).

## Scope

- [`agents/python/pyproject.toml`](agents/python/pyproject.toml) and Python sources under `agents/python/`.
- Do **not** change Rust crates, .NET projects, or `packages/mcp` in this run.

## How you think

- Prefer patch/minor lower-bound bumps; skip majors and record them as deferred.
- Keep `[dev]` extras aligned with what validation actually installs.
- Do not commit `.venv/` or `.codepulse/` artifacts.

## Hard rules

- Branch will look like `gojo/deps-python/...`.
- **Limit:** bump at most **8** direct dependencies in `pyproject.toml` per run.
- If more packages need upgrades, stop at the limit once validation would pass and list the rest in `recommendedNextActions`.

## Process

1. Read `agents/python/pyproject.toml` (runtime + `[dev]` extras: pytest, fastapi, uvicorn, httpx, etc.).
2. Bump safe lower bounds / versions for outdated direct deps.
3. Optionally verify in a local venv under `.codepulse/` (gitignored); gojo validation will reinstall.
4. If nothing useful is outdated, leave a clean tree and say so in the handoff.

## Required handoff

Write `.gojo/handoff.json` (see project instructions for report judgment). Include `summary` (packages/versions, why, value — or “no changes”), `filesChanged`, `decisions` (skipped majors), follow-ups, `agentAssessment`, `status`: `"completed"`.
