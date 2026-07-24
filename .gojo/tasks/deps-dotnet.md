# .NET agent dependency maintenance

You are executing an unattended gojo scheduled task for **codepulse** (.NET Harmony agent).

## Goals

1. Identify outdated NuGet packages for the .NET agent and tests.
2. Apply safe patch/minor upgrades in `PackageReference` versions.
3. Fix compile/test failures caused by those upgrades.
4. Leave the solution ready for gojo validation (`dotnet test -c Release agents/dotnet/CodePulse.Agent.Tests/CodePulse.Agent.Tests.csproj`).

## Scope

- [`agents/dotnet/CodePulse.Agent/CodePulse.Agent.csproj`](agents/dotnet/CodePulse.Agent/CodePulse.Agent.csproj)
- [`agents/dotnet/CodePulse.Agent.Tests/CodePulse.Agent.Tests.csproj`](agents/dotnet/CodePulse.Agent.Tests/CodePulse.Agent.Tests.csproj)
- Related sources under `agents/dotnet/`
- Do **not** change Rust, Python, or `packages/mcp` in this run.

## Hard rules

- Do **not** push, open PRs, or merge. gojo owns Git integration (`pull-request` mode).
- Do **not** commit secrets or `bin/`/`obj/` output.
- Skip major migrations; record deferred upgrades in the handoff.
- Prefer the smallest change set.
- Stay inside this worktree.
- Do **not** weaken CI to force a pass.

## Process

1. From `agents/dotnet`, run `dotnet list package --outdated` (or equivalent).
2. Bump safe package versions in the csproj files; `dotnet restore` as needed.
3. Fix compile/test fallout; gojo re-runs `dotnet test -c Release agents/dotnet/CodePulse.Agent.Tests/CodePulse.Agent.Tests.csproj` after you exit.
4. If nothing useful is outdated, leave a clean tree and say so in the handoff.

## Required handoff

Write `.gojo/handoff.json` before you finish (schemaVersion 1), including:

- `summary` of upgrades applied (or “no changes”)
- `filesChanged`
- `decisions` / `unresolvedIssues` / `recommendedNextActions`
- `agentAssessment.successful` and `confidence`
- `status`: `"completed"`

Use a placeholder ULID for `runId` if unknown.
