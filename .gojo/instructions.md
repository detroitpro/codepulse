# Codepulse project instructions

Shared for every unattended AI gojo task in this repository. Task prompts add role-specific goals, limits, and process.

## Code qualities

Every decision should converge on these:

- **Minimal** — fewest files, smallest coherent diff, no drive-by refactors.
- **Stack-scoped** — stay inside the task’s language/agent surface (Rust vs Python vs .NET vs MCP).
- **Behavior-preserving** — dependency bumps keep tests/CI green; skip majors unless the task allows them.
- **Reviewable** — stop at the task’s numeric limit; put leftovers in `recommendedNextActions`.

## Operating defaults

- Unattended scheduled run in a gojo worktree — stay inside that worktree.
- Do **not** invent product features.
- Do **not** weaken CI, delete tests to pass, or commit secrets / `.env` / `.venv/` / `.codepulse/` artifacts.
- Prefer the smallest change set that meets the task goals.
- If nothing useful needs doing, leave a clean tree and say so in the handoff (`summary` may be “no changes”).

## Git and handoff

Unless the task explicitly owns merge/push (e.g. deps-pr-babysit):

- Do **not** push, open PRs, or merge. gojo `pull-request` / `commit-only` integration owns Git.
- Write `.gojo/handoff.json` before you finish. Prefer **schemaVersion 3** (or 2 when reporting `impact` only). **gojo opens the PR from this handoff.** Do **not** run `gh pr create` yourself.
- Use a placeholder ULID for `runId` if unknown.

## How you report (handoff judgment)

The PR title/body come from the handoff. Reviewers should not need the raw agent transcript.

- `summary` — first line is the PR title (or diagnosis title). Body must cover **what**, **why**, and **value** (or explicitly “no changes”).
- `decisions` — rationale for notable choices (skipped majors, deferred packages, diagnosis), not only a list of actions.
- `filesChanged` — accurate when you edited the tree; empty when diagnose-only / no PR.
- `unresolvedIssues` / `recommendedNextActions` — deferred work after hitting the numeric limit, or operator follow-ups.
- `agentAssessment.successful` + `confidence`, and `status`: `"completed"`.

## Validation

Validation steps are your definition of done — run them and fix failures before writing the handoff.

## Impact claims (`impact.items`)

Prefer one item per concrete subject (package, issue, module). Allowed `category` values **exactly**: `dependency-update`, `bug-fix`, `bug-prevention`, `documentation`, `test-coverage`, `security`, `feature`, `performance`, `maintenance`. **Omit `impact` if unsure** — never invent categories (`code-quality`, `refactor`, etc.).

## Repair rounds (CI / reviewer feedback)

When `subject.kind` is `pull-request` and `subject.feedback` is present (checks or review summary):

- Treat the run as **repair-only** on the existing PR branch — do not open a new scope or invent unrelated work.
- Read the PR body, `subject.feedback`, and `git log` / diff vs the target base.
- Fix the cited CI or reviewer findings; keep the diff small.
- Still write `.gojo/handoff.json`.
