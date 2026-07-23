/**
 * codepulse MCP server stub.
 *
 * Tool contracts: docs/MCP_API.md
 * Talks to Rust daemon over local HTTP / Unix socket (not wired yet).
 */

const TOOLS = [
  "get_function_runtime_summary",
  "get_actual_callers",
  "get_actual_callees",
  "get_hot_paths",
  "get_static_summary",
  "compare_static_vs_runtime",
  "enable_targeted_instrumentation",
  "list_uncovered_hot_symbols",
] as const;

function main(): void {
  console.log("codepulse MCP — design-phase stub");
  console.log("Planned tools:");
  for (const name of TOOLS) {
    console.log(`  - ${name}`);
  }
  console.log("See docs/MCP_API.md");
}

main();
