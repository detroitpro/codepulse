/**
 * Smoke-test daemon query API used by MCP tools (no MCP stdio handshake).
 */

const ENDPOINT = (process.env.CODEPULSE_ENDPOINT ?? "http://127.0.0.1:7420").replace(
  /\/$/,
  "",
);

async function main() {
  const health = await fetch(`${ENDPOINT}/health`);
  if (!health.ok) throw new Error(`daemon unhealthy: ${health.status}`);
  const hot = await fetch(`${ENDPOINT}/v1/query/hot-paths?limit=5`);
  if (!hot.ok) throw new Error(`hot-paths failed: ${hot.status}`);
  const search = await fetch(`${ENDPOINT}/v1/query/structural-search`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      language: "python",
      pattern: "async def $NAME($$$ARGS): $$$BODY",
      limit: 5,
    }),
  });
  if (!search.ok) throw new Error(`structural-search failed: ${search.status}`);
  console.log("mcp smoke ok");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
