# .NET agent dependency maintenance

Unattended gojo task for **codepulse** (.NET Harmony agent). Keep NuGet refs current within safe ranges; no product features.

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

## How you think

- Prefer patch/minor `PackageReference` bumps; skip majors and record them as deferred.
- Keep agent and test projects aligned when they share a package.
- Do not commit `bin/` / `obj/` output.

## Hard rules

- Branch will look like `gojo/deps-dotnet/...`.
- **Limit:** bump at most **8** `PackageReference` versions across the agent + tests projects per run.
- If more packages need upgrades, stop at the limit once tests are green and list the rest in `recommendedNextActions`.

## Process

1. From `agents/dotnet`, run `dotnet list package --outdated` (or equivalent).
2. Bump safe package versions in the csproj files; `dotnet restore` as needed.
3. Fix compile/test fallout; gojo re-runs `dotnet test -c Release agents/dotnet/CodePulse.Agent.Tests/CodePulse.Agent.Tests.csproj` after you exit.
4. If nothing useful is outdated, leave a clean tree and say so in the handoff.

## Required handoff

Write `.gojo/handoff.json` (see project instructions for report judgment). Include `summary` (packages/versions, why, value — or “no changes”), `filesChanged`, `decisions` (skipped majors), follow-ups, `agentAssessment`, `status`: `"completed"`.
