// Phase 22.2 — Bun mock-hub for axhub plugin E2E.
// Localhost HTTP only (AXHUB_ALLOW_PROXY=1 우회로 helper bin 이 그대로 호출 가능).
// Fixture-driven response, append-only log of every request.

import { existsSync, appendFileSync, readFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const FIXTURES = join(__dirname, "..", "fixtures");

const args = Bun.argv.slice(2);
let port = 18080;
let logPath: string | null = null;
for (let i = 0; i < args.length; i++) {
  if (args[i] === "--port" && args[i + 1]) port = Number(args[++i]);
  if (args[i] === "--log" && args[i + 1]) logPath = args[++i] ?? null;
}

const log = (line: string): void => {
  if (logPath) appendFileSync(logPath, `${line}\n`);
};

const readFixture = (name: string): unknown => {
  const path = join(FIXTURES, name);
  if (!existsSync(path)) return null;
  return JSON.parse(readFileSync(path, "utf8"));
};

const json = (status: number, body: unknown): Response =>
  new Response(JSON.stringify(body), {
    status,
    headers: { "content-type": "application/json" },
  });

Bun.serve({
  port,
  hostname: "127.0.0.1",
  fetch(req) {
    const url = new URL(req.url);
    const stamp = new Date().toISOString();
    log(`${stamp} ${req.method} ${url.pathname}${url.search}`);

    if (url.pathname === "/_ping") return new Response("ok");

    if (req.method === "GET" && url.pathname === "/v1/apps") {
      const fix = readFixture("apps-list.json");
      if (fix) return json(200, fix);
      return json(200, { apps: [] });
    }

    if (req.method === "GET" && url.pathname === "/v1/apis") {
      const fix = readFixture("apis-list.json");
      if (fix) return json(200, fix);
      return json(200, { apis: [] });
    }

    if (req.method === "GET" && url.pathname === "/v1/auth/whoami") {
      const fix = readFixture("auth-whoami.json");
      if (fix) return json(200, fix);
      return json(401, { error: "not_authenticated" });
    }

    if (req.method === "GET" && url.pathname.startsWith("/v1/deployments/")) {
      const fix = readFixture("deploy-status.json");
      if (fix) return json(200, fix);
      return json(404, { error: "deployment_not_found" });
    }

    if (req.method === "POST" && url.pathname === "/v1/deployments") {
      const fix = readFixture("deploy-create-success.json");
      if (fix) return json(201, fix);
      return json(201, { deployment_id: "mock-dep-1" });
    }

    return json(404, { error: "mock_path_not_implemented", path: url.pathname });
  },
});

console.error(`[mock-hub] listening on http://127.0.0.1:${port}`);
