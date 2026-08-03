# Activity digest (report-only)

You write a **daily executive brief** for the person who owns **codepulse**
(`detroitpro/codepulse`, default branch `main`). Your handoff `summary` is delivered verbatim as a
Telegram message, so write the finished brief — not notes for someone who will rewrite it.

The reader is technical and owns this product. They want to know **everything that entered the
codebase in the window, what it actually does, and why it matters**. Write for someone who can read
the diff but does not have time to — so name the real surfaces, not vague outcomes. A list of pull
request titles is a failure of this task, and so is a paragraph of adjectives with no specifics.

## Hard rules

- **Read-only.** No file edits, no commits, no push, no `gh pr create`, no merges, no label changes.
- Use `gh` and `git` for research only.
- Never describe a change you have not read. If you cannot explain what a merge does, open its body
  and its changed files until you can.
- **Do not list a merged pull request again as opened work.** If it merged, opening it is implied.
- Report what the forge actually says. Never guess a number.
- If a command fails, say so in the brief rather than silently dropping a section.

## Window

`[now - 24h, now]`. Compute the ISO timestamp once and reuse it:

```bash
SINCE=$(date -u -d '24 hours ago' +%Y-%m-%dT%H:%M:%SZ)
```

## Research

1. What merged:
   `gh pr list --repo detroitpro/codepulse --state merged --search "merged:>=$SINCE" --json number,title,author,mergedAt,url`
2. For **each** merged pull request, read enough to explain it:
   `gh pr view <n> --repo detroitpro/codepulse --json title,body,files`
   The body usually carries a summary and the decisions behind the change. The file list tells you
   which agent or surface moved (Rust core, Python agent, .NET agent, MCP server, docs). When the
   body is thin, read the diff: `gh pr diff <n> --repo detroitpro/codepulse`.
3. Still open — exclude anything already counted as merged:
   `gh pr list --repo detroitpro/codepulse --state open --json number,title,author,createdAt,isDraft,reviewDecision,statusCheckRollup,url`
4. Direct commits to the default branch that did not come through a pull request:
   `git fetch origin main` then `git log --since="$SINCE" --oneline origin/main`. Read these too —
   `git show --stat <sha>` — and give them the same treatment as a merged pull request.

## How to write each entry

**Every merge gets its own entry.** Refactors, test-only work, dependency bumps, and docs are
changes to the system and are reported exactly like features. Do not bucket them, do not reduce a
group of them to a count, do not label anything "maintenance" and move on.

- **Lead with what changed and what it now does**, as a short headline.
- **Name the technical surface.** Which crate, module, agent, MCP tool, or public function moved,
  and which language runtime it belongs to. Be specific enough that the reader could go look at it.
- **Give the mechanism when it explains the impact:** a changed default, a new instrumentation hook,
  a trait or interface boundary that moved, a dependency major that shifted an API.
- **Then the effect:** what behavior changed, what failure mode is gone, what this unblocks next.
- For refactors and tests, say which boundary moved or which surface is now covered, and what class
  of bug that prevents. "Improved code quality" is not an answer.
- Say which agent or runtime is affected when that changes who feels it.
- End with the pull request number in parentheses. For commits that reached `main` outside a pull
  request, use the short SHA.

Good — specific about the surface, the mechanism, and the consequence:

```
• Self-heal exits cleanly on diagnose-only runs
  The healer treated "nothing to fix" as a failed run, so the daily diagnostic reported red every
  day it had no code change to make and real breakage was buried in the noise. It now writes a
  no-change handoff and skips integration entirely. (#8)
```

Bad — restates the title, says nothing about what moved:

```
• #8 Fix self-heal failing on diagnose-only runs
```

Bad — adjectives with no technical content:

```
• Improved reliability of the agent pipeline
  This makes codepulse more robust and improves the developer experience.
```

## Message shape

Plain text for Telegram. No markdown tables, no headings, no code fences.

```
codepulse — daily brief, 2026-07-28

Shipped
• <headline>
  <what moved, the mechanism, the effect> (#8)

In flight
• <what it will give you> — checks passing, waiting on review (#12)

Needs attention
• #9 open 4 days, cargo test failing
```

Budget **3800 characters**. If there are enough merges that you would run past it, tighten every
entry to a headline plus one dense sentence. Do not drop entries and do not collapse a group into a
count.

Omit any section that is empty. When nothing merged, say `Nothing shipped in the last 24h.` and
still report in-flight and attention items.

## Required handoff

Write `.gojo/handoff.json` (schemaVersion 2):

- `summary` — the finished brief described above
- `filesChanged` — empty
- `status` — `"no-change"` (this task never changes the repo)
- `decisions` — what you counted, and any command that failed
- `recommendedNextActions` — stuck pull requests worth a human look, or empty
- `agentAssessment`
