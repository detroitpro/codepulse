# Rust dependency maintenance

Unattended gojo task for **codepulse** (Rust workspace / daemon). Keep Cargo deps current within safe semver; no product features.

## Goals

1. Identify outdated Cargo dependencies that are safe to upgrade.
2. Apply patch/minor upgrades (lockfile and, when needed, floors in `Cargo.toml` / `[workspace.dependencies]`).
3. Fix compile or test failures caused by those upgrades.
4. Leave the workspace ready for gojo’s validation (`cargo test --workspace`).

## Scope

- Root [`Cargo.toml`](Cargo.toml), member crates under `crates/`, and [`Cargo.lock`](Cargo.lock).
- Do **not** change Python, .NET, or `packages/mcp` in this run.

## How you think

- Prefer `cargo update` within existing ranges before raising floors.
- Prefer patch/minor; record majors as deferred — do not perform major framework migrations.
- One coherent lockfile refresh that stays green beats a scatter of unrelated floor bumps.

## Hard rules

- Branch will look like `gojo/deps-rust/...`.
- **Limit:** raise at most **8** direct dependency floors (workspace or member) per run; prefer lockfile-only updates when enough.
- If more work remains, stop once tests are green and list deferred upgrades in `recommendedNextActions`.

## Process

1. Inspect workspace dependency versions (`cargo tree -d`, outdated crates, or equivalent).
2. Run `cargo update` for safe lockfile refresh within existing semver ranges.
3. When a direct dependency floor must rise for a safe minor/patch, edit `[workspace.dependencies]` (or the member crate) carefully, then `cargo update`.
4. Fix compile/test fallout locally as needed; gojo re-runs full validation after you exit.
5. If nothing useful is outdated, leave the tree clean and say so in the handoff.

## Required handoff

Write `.gojo/handoff.json` (see project instructions for report judgment). Include `summary` (crates/versions bumped, why, value — or “no changes”), `filesChanged`, `decisions` (skipped majors), follow-ups, `agentAssessment`, `status`: `"completed"`.
