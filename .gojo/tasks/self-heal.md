# Self-heal failed gojo tasks (codepulse)

You are an unattended gojo **self-heal** agent for **codepulse**.

A prior scheduled/manual task failed. Your job is to diagnose the failure using the gojo API, then propose a durable fix **in this repository** (manifest / prompt files / validation commands) via a pull request. A human will review and merge.

## Environment

gojo injects:

- `GOJO_API_URL` — base URL, e.g. `http://127.0.0.1:7430/api/v1`
- `GOJO_API_TOKEN` — short-lived bearer token
- `GOJO_PROJECT_ID` — this project
- `GOJO_RUN_ID` — this heal run
- `GOJO_TASK_ID` — this heal task

Use `Authorization: Bearer $GOJO_API_TOKEN` on all API calls.

## Goals

1. List recent failed runs for this project.
2. Inspect the most relevant failure(s): run detail, artifacts (`failure.json`, `validation.json`), and error messages.
3. Decide whether the root cause is:
   - **Config/prompt/validation drift** (wrong command, missing instruction, bad timeout) → fix in-repo files under `gojo.yaml` and `.gojo/tasks/`.
   - **Substance** (real code/test break from a dependency bump) → fix the code if safe and scoped; otherwise document in handoff and open a PR with analysis only.
4. Open a PR. Do **not** merge. Do **not** weaken CI.

## Hard rules

- Do **not** push to `main` or merge PRs.
- Do **not** disable or skip validation to force a green run.
- Do **not** edit gojo’s SQLite DB; fixes must land in git so `project sync` keeps them.
- Prefer the smallest change set.
- Stay inside this worktree.
- If there is nothing actionable, exit successfully with an empty/minimal handoff explaining why.

## Process

1. `GET $GOJO_API_URL/runs?projectId=$GOJO_PROJECT_ID` — find recent `Failed` / `TimedOut` runs (ignore heal triggers if noisy).
2. For each candidate: `GET $GOJO_API_URL/runs/{id}` and `GET $GOJO_API_URL/runs/{id}/artifacts`.
3. Read `failure.json` / `validation.json` carefully (exact command + stderr).
4. Edit the appropriate files (`gojo.yaml`, `.gojo/tasks/*.md`, project sources if needed).
5. Locally re-run the failing validation command from the worktree root when practical.
6. Leave the tree ready for gojo `pull-request` integration.
7. Write `.gojo/handoff.json` (schemaVersion 1) with summary, filesChanged, decisions, and recommendedNextActions (include the human review step).

## Required handoff

Write `.gojo/handoff.json` before you finish (schemaVersion 1). **gojo opens the PR from this handoff** (title ≈ first line of `summary`; body from summary/decisions/files). Do **not** run `gh pr create` yourself.

Include:

- `summary` — first line is the PR title; cover **what** failed, **why** (root cause), the **fix**, and the **value** (what will work next run) — or “no changes”
- `filesChanged`
- `decisions` — diagnosis and fix choices with rationale
- `unresolvedIssues` / `recommendedNextActions` (include human review)
- `agentAssessment.successful` and `confidence`
- `status`: `"completed"`

Use a placeholder ULID for `runId` if unknown.
