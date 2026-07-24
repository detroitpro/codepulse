# Python agent dependency maintenance

You are executing an unattended gojo scheduled task for **codepulse** (Python runtime agent).

## Goals

1. Identify outdated dependencies for `agents/python`.
2. Apply safe patch/minor upgrades (update floors in `agents/python/pyproject.toml` when appropriate).
3. Fix import/runtime breaks caused by those upgrades.
4. Leave the agent ready for gojo validation (venv install + import smoke).

## Scope

- [`agents/python/pyproject.toml`](agents/python/pyproject.toml) and Python sources under `agents/python/`.
- Do **not** change Rust crates, .NET projects, or `packages/mcp` in this run.

## Hard rules

- Do **not** push, open PRs, or merge. gojo owns Git integration (`pull-request` mode).
- Do **not** commit secrets, `.venv/`, or `.codepulse/` artifacts.
- Skip major migrations; record deferred upgrades in the handoff.
- Prefer the smallest change set.
- Stay inside this worktree.
- Do **not** weaken CI to force a pass.

## Process

1. Read `agents/python/pyproject.toml` (runtime + `[dev]` extras: pytest, fastapi, uvicorn, httpx, etc.).
2. Bump safe lower bounds / versions for outdated direct deps.
3. Optionally verify in a local venv under `.codepulse/` (gitignored); gojo validation will reinstall.
4. If nothing useful is outdated, leave a clean tree and say so in the handoff.

## Required handoff

Write `.gojo/handoff.json` before you finish (schemaVersion 1), including:

- `summary` of upgrades applied (or “no changes”)
- `filesChanged`
- `decisions` / `unresolvedIssues` / `recommendedNextActions`
- `agentAssessment.successful` and `confidence`
- `status`: `"completed"`

Use a placeholder ULID for `runId` if unknown.
