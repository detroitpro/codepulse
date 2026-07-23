#!/usr/bin/env node
/**
 * codepulse MCP server — proxies tools to the local daemon query API.
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const ENDPOINT = (process.env.CODEPULSE_ENDPOINT ?? "http://127.0.0.1:7420").replace(
  /\/$/,
  "",
);

async function daemonGet(path: string, query: Record<string, string | undefined> = {}) {
  const url = new URL(ENDPOINT + path);
  for (const [k, v] of Object.entries(query)) {
    if (v !== undefined && v !== "") url.searchParams.set(k, v);
  }
  const res = await fetch(url);
  const text = await res.text();
  let body: unknown;
  try {
    body = text ? JSON.parse(text) : null;
  } catch {
    body = { error: text };
  }
  if (!res.ok) {
    const err =
      typeof body === "object" && body && "error" in body
        ? String((body as { error: unknown }).error)
        : `HTTP ${res.status}`;
    throw new Error(err);
  }
  return body;
}

async function daemonPost(path: string, body: unknown) {
  const res = await fetch(ENDPOINT + path, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  const text = await res.text();
  let parsed: unknown;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = { error: text };
  }
  if (!res.ok) {
    const err =
      typeof parsed === "object" && parsed && "error" in parsed
        ? String((parsed as { error: unknown }).error)
        : `HTTP ${res.status}`;
    throw new Error(err);
  }
  return parsed;
}

function ok(data: unknown) {
  return {
    content: [{ type: "text" as const, text: JSON.stringify(data, null, 2) }],
  };
}

function fail(err: unknown) {
  const msg = err instanceof Error ? err.message : String(err);
  return {
    content: [{ type: "text" as const, text: JSON.stringify({ error: msg }) }],
    isError: true,
  };
}

const symbolShape = {
  language: z.string(),
  path: z.string(),
  qualname: z.string(),
  session_id: z.string().optional(),
};

async function main() {
  const server = new McpServer({
    name: "codepulse",
    version: "0.1.0",
  });

  server.tool(
    "get_hot_paths",
    "Rank hottest symbols by invocations, duration_p95, or exceptions",
    {
      session_id: z.string().optional(),
      limit: z.number().int().positive().optional(),
      metric: z.enum(["invocations", "duration_p95", "exceptions"]).optional(),
    },
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/hot-paths", {
            session_id: args.session_id,
            limit: args.limit?.toString(),
            metric: args.metric,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "get_function_runtime_summary",
    "Return observed runtime stats for a symbol",
    { ...symbolShape, window_minutes: z.number().optional() },
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/function-summary", {
            language: args.language,
            path: args.path,
            qualname: args.qualname,
            session_id: args.session_id,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "get_actual_callers",
    "Ranked observed callers of a symbol",
    { ...symbolShape, limit: z.number().int().positive().optional() },
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/callers", {
            language: args.language,
            path: args.path,
            qualname: args.qualname,
            session_id: args.session_id,
            limit: args.limit?.toString(),
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "get_actual_callees",
    "Ranked observed callees of a symbol",
    { ...symbolShape, limit: z.number().int().positive().optional() },
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/callees", {
            language: args.language,
            path: args.path,
            qualname: args.qualname,
            session_id: args.session_id,
            limit: args.limit?.toString(),
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "get_static_summary",
    "Static complexity/size summary for a symbol",
    symbolShape,
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/static-summary", {
            language: args.language,
            path: args.path,
            qualname: args.qualname,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "compare_static_vs_runtime",
    "Compare declared vs observed callees",
    symbolShape,
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/compare", {
            language: args.language,
            path: args.path,
            qualname: args.qualname,
            session_id: args.session_id,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "list_uncovered_hot_symbols",
    "Complex static symbols with zero runtime hits",
    {
      min_complexity: z.number().int().optional(),
      limit: z.number().int().positive().optional(),
      session_id: z.string().optional(),
    },
    async (args) => {
      try {
        return ok(
          await daemonGet("/v1/query/uncovered", {
            min_complexity: args.min_complexity?.toString(),
            limit: args.limit?.toString(),
            session_id: args.session_id,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "enable_targeted_instrumentation",
    "Open a budgeted probe window on selected symbols",
    {
      targets: z.array(
        z.object({
          language: z.string(),
          path: z.string(),
          qualname: z.string(),
        }),
      ),
      duration_s: z.number().int().positive().optional(),
      session_id: z.string().optional(),
    },
    async (args) => {
      try {
        return ok(
          await daemonPost("/v1/probe-windows", {
            targets: args.targets,
            duration_s: args.duration_s ?? 30,
            session_id: args.session_id ?? null,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  server.tool(
    "structural_search",
    "AST pattern search over the workspace (tree-sitter indexer)",
    {
      language: z.string(),
      pattern: z.string(),
      path_prefix: z.string().optional(),
      limit: z.number().int().positive().optional(),
    },
    async (args) => {
      try {
        return ok(
          await daemonPost("/v1/query/structural-search", {
            language: args.language,
            pattern: args.pattern,
            path_prefix: args.path_prefix,
            limit: args.limit ?? 50,
          }),
        );
      } catch (e) {
        return fail(e);
      }
    },
  );

  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
