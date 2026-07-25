# Dependency PR babysitter

Unattended gojo task for **codepulse**. Get open **dependency-update PRs** merge-ready and merge them when green.

This task **owns merge/push** on allowlisted PR branches (exception to the shared “do not push/merge” default).

## Goals

1. Find open GitHub PRs for this repo that look like gojo dependency runs.
2. Babysit each until mergeable (or clearly blocked).
3. Merge when CI is green and comments are triaged.
4. Cap at **3 PRs** per run (newest first).

## Which PRs

Target open PRs against `main` that match any of:

- Branch name containing `gojo/deps-` or `gojo/deps-rust` / `deps-python` / `deps-dotnet`
- Title containing `gojo: deps` or `dependency` / `deps-rust` / `deps-python` / `deps-dotnet`
- Head branch created by a recent gojo dep maintenance run

Skip unrelated PRs. Prefer newest first.

## Babysit loop

For each PR:

1. **Conflicts:** resolve intelligently, preserving intent of both sides; if intents conflict, stop that PR and record why.
2. **Comments:** triage unresolved review/Bugbot comments; fix valid issues; explain disagreement in the handoff when you skip.
3. **CI:** fix failures caused by the PR’s dependency changes. Do **not** weaken or delete CI workflows to force a pass. If the branch is behind `main` and failures look unrelated, merge/rebase latest `main` and re-check.
4. Push scoped fixes to the PR branch.
5. When the PR is mergeable, CI green, and comments triaged: merge with `gh pr merge <n> --squash` (or `--merge` if squash is unavailable). Do not force-merge failing checks.

## Hard rules

- You **may** use `gh` and `git push` on the PR branches — this task owns GitHub merge, unlike the bump tasks.
- If an allowlisted PR still has a stub/empty body (`Automated run…` / no what-why-value), improve the description with `gh pr edit` when you touch that PR (use the handoff or commit messages — do not invent features).
- **Limit:** babysit/merge at most **3** allowlisted PRs per run.
- Do **not** push unrelated commits to `main` outside of the merge.
- Do **not** invent secrets or change gojo host config.
- Stay focused on dependency PRs for **detroitpro/codepulse**.
- If no matching open PRs: exit successfully with an empty-action handoff.
- If more allowlisted PRs remain open, stop at three and list them in `recommendedNextActions`.

## Required handoff

Write `.gojo/handoff.json` (schemaVersion 1). Include `summary` (PRs found, fixed, merged, skipped — with why for skips), `filesChanged` (if you pushed fixes), `decisions` / `unresolvedIssues` / `recommendedNextActions`, `agentAssessment`, `status`: `"completed"`. Use a placeholder ULID for `runId` if unknown.
