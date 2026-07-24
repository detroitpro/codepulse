# Rust dependency maintenance

You are executing an unattended gojo scheduled task for **codepulse** (Rust workspace / daemon).

## Goals

1. Identify outdated Cargo dependencies that are safe to upgrade.
2. Apply patch/minor upgrades (lockfile and, when needed, floors in `Cargo.toml` / `[workspace.dependencies]`).
3. Fix compile or test failures caused by those upgrades.
4. Leave the workspace ready for gojo’s validation (`cargo test --workspace`).

## Scope

- Root [`Cargo.toml`](Cargo.toml), member crates under `crates/`, and [`Cargo.lock`](Cargo.lock).
- Do **not** change Python, .NET, or `packages/mcp` in this run.

## Hard rules

- Do **not** push, open PRs, or merge. gojo owns Git integration (`pull-request` mode).
- Do **not** commit secrets or credential material.
- Do **not** perform major framework migrations; record them as deferred in the handoff.
- Prefer the smallest change set that keeps tests green.
- Stay inside this worktree.
- Do **not** weaken or delete CI workflows to force a pass.

## Process

1. Inspect workspace dependency versions (`cargo tree -d`, outdated crates, or equivalent).
2. Run `cargo update` for safe lockfile refresh within existing semver ranges.
3. When a direct dependency floor must rise for a safe minor/patch, edit `[workspace.dependencies]` (or the member crate) carefully, then `cargo update`.
4. Fix compile/test fallout locally as needed; gojo re-runs full validation after you exit.
5. If nothing useful is outdated, leave the tree clean (or no meaningful diff) and say so in the handoff.

## Required handoff

Write `.gojo/handoff.json` before you finish (schemaVersion 1), including:

- `summary` of upgrades applied (or “no changes”)
- `filesChanged`
- `decisions` (especially skipped majors)
- `unresolvedIssues`
- `recommendedNextActions`
- `agentAssessment.successful` and `confidence`
- `status`: `"completed"`

Use a placeholder ULID for `runId` if unknown.
