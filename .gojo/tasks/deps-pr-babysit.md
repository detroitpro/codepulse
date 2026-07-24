# Dependency PR babysitter

You are executing an unattended gojo scheduled task for **codepulse**. Your job is to get open **dependency-update PRs** merge-ready and merge them when green.

## Goals

1. Find open GitHub PRs for this repo that look like gojo dependency runs.
2. Babysit each until mergeable (or clearly blocked).
3. Merge when CI is green and comments are triaged.

## Which PRs

Target open PRs against `main` that match any of:

- Branch name containing `gojo/deps-` or `gojo/deps-rust` / `deps-python` / `deps-dotnet`
- Title containing `gojo: deps` or `dependency` / `deps-rust` / `deps-python` / `deps-dotnet`
- Head branch created by a recent gojo dep maintenance run

Skip unrelated PRs. Prefer newest first. Cap at **3 PRs** per run.

## Babysit loop (Cursor babysit skill)

For each PR:

1. **Conflicts:** resolve intelligently, preserving intent of both sides; if intents conflict, stop that PR and record why.
2. **Comments:** triage unresolved review/Bugbot comments; fix valid issues; explain disagreement in the handoff when you skip.
3. **CI:** fix failures caused by the PR’s dependency changes. Do **not** weaken or delete CI workflows to force a pass. If the branch is behind `main` and failures look unrelated, merge/rebase latest `main` and re-check.
4. Push scoped fixes to the PR branch.
5. When the PR is mergeable, CI green, and comments triaged: merge with `gh pr merge <n> --squash` (or `--merge` if squash is unavailable). Do not force-merge failing checks.

## Hard rules

- You **may** use `gh` and `git push` on the PR branches — this task owns GitHub merge, unlike the bump tasks.
- Do **not** push unrelated commits to `main` outside of the merge.
- Do **not** invent secrets or change gojo host config.
- Stay focused on dependency PRs for **detroitpro/codepulse**.
- If no matching open PRs: exit successfully with an empty-action handoff.

## Required handoff

Write `.gojo/handoff.json` before you finish (schemaVersion 1), including:

- `summary` (PRs found, fixed, merged, skipped)
- `filesChanged` (if you pushed fixes)
- `decisions` / `unresolvedIssues` / `recommendedNextActions`
- `agentAssessment.successful` and `confidence`
- `status`: `"completed"`

Use a placeholder ULID for `runId` if unknown.
