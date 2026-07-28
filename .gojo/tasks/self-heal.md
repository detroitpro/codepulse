# Self-heal failed gojo tasks (codepulse)

Unattended gojo **self-heal** agent for **codepulse**. Diagnose a prior failed run via the gojo API, then propose a durable in-repo fix via PR — or leave a clear diagnosis when unsafe to fix from the worktree.

## Environment

- `GOJO_API_URL` — e.g. `http://127.0.0.1:7430/api/v1`
- `GOJO_API_TOKEN` — bearer token
- `GOJO_PROJECT_ID`, `GOJO_RUN_ID`, `GOJO_TASK_ID`

Use `Authorization: Bearer $GOJO_API_TOKEN` on all API calls.

## Goals

1. List recent failed runs for this project.
2. Inspect the most relevant failure(s): run detail, artifacts (`failure.json`, `validation.json`), and error messages.
3. Decide whether the root cause is:
   - **Config/prompt/validation drift** → fix `gojo.yaml` / `.gojo/tasks/` / `.gojo/instructions.md`.
   - **Substance** (real code/test break from a dependency bump) → fix if safe and scoped; otherwise analysis-only PR.
4. Land durable fixes in git when safe (see **Integration** below). Do **not** push, merge, or run `gh pr create` yourself.

## How you think

- One root cause only — do not chase every failure in the project.
- Prefer config/prompt/validation fixes over broad code rewrites.
- Dedupe: if an open PR already targets the same failure signature, stop and point at it.
- Workspace / dirty primary-checkout failures are diagnose-only — never “clean” the operator’s checkout.

## Hard rules

- Do **not** push to `main` or merge PRs.
- Do **not** edit gojo’s SQLite DB; fixes must land in git so `project sync` keeps them.
- **Limit:** fix **one** root cause per run; touch at most **5** files. Do not expand into unrelated maintenance.
- If nothing actionable, complete with a clear handoff.
- If more failures remain after one focused fix, list them in `recommendedNextActions`.
- **Never** mutate the operator's **primary checkout** outside your worktree.

## Process

1. `GET $GOJO_API_URL/runs?projectId=$GOJO_PROJECT_ID` — find recent `Failed` / `TimedOut` runs (ignore heal triggers if noisy).
2. For each candidate: `GET $GOJO_API_URL/runs/{id}` and `GET $GOJO_API_URL/runs/{id}/artifacts`.
3. **Dedupe open PRs:** `gh pr list --state open` — if one already covers the same failure signature, handoff pointing at it and stop.
4. **Workspace / base-checkout failures:** diagnose only; recommend operator action; do not clean the primary tree.
5. Otherwise edit the appropriate files; re-run the failing validation from the worktree root when practical.
6. Write `.gojo/handoff.json` (schemaVersion 1).

## Integration

This task omits `integration` in `gojo.yaml` (report-only). gojo records the handoff artifact and succeeds without opening a PR — including diagnose-only runs with no tree diff.

- **Diagnose-only** (most runs): `status`: `"no-change"`, empty `filesChanged`, clean tree.
- **Substantive manifest/code fix**: edit files in the worktree, validate locally, list paths in `filesChanged`, and spell out the exact diff in `summary`/`decisions`. The worktree is discarded after the run; the persisted handoff artifact is the source of truth for a follow-up PR (operator or a one-off task with `integration.mode: pull-request`).
- **Dedupe**: if an open PR already covers the failure, point at it and use `status`: `"no-change"`.

## Required handoff

Write `.gojo/handoff.json` (see project instructions for report judgment).

Include `summary` (what failed, why, fix or why no code fix, value for next run), `filesChanged`, `decisions`, follow-ups (include human review), `agentAssessment`, and `status`: `"no-change"` when idle or `"completed"` when you validated a fix in the worktree.
