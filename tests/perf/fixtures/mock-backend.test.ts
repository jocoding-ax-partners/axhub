import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { MockBackendServer } from "./mock-backend";

describe("MockBackendServer", () => {
  let server: MockBackendServer;
  let baseUrl: string;

  beforeAll(async () => {
    server = new MockBackendServer(0);
    const started = await server.start();
    baseUrl = started.baseUrl;
    expect(started.port).toBeGreaterThan(0);
  });

  afterAll(async () => {
    await server.stop();
  });

  test("POST /api/v1/resolve returns app payload", async () => {
    const res = await fetch(`${baseUrl}/api/v1/resolve`, { method: "POST" });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { app_slug: string; commit_sha: string };
    expect(body.app_slug).toBe("paydrop");
    expect(body.commit_sha).toMatch(/^[a-f0-9]+$/);
  });

  test("POST /api/v1/apps returns 201 with subdomain", async () => {
    const res = await fetch(`${baseUrl}/api/v1/apps`, { method: "POST" });
    expect(res.status).toBe(201);
    const body = (await res.json()) as { subdomain: string };
    expect(body.subdomain).toContain("axhub.dev");
  });

  test("POST /api/v1/deploys returns 201 with deploy_id", async () => {
    const res = await fetch(`${baseUrl}/api/v1/deploys`, { method: "POST" });
    expect(res.status).toBe(201);
    const body = (await res.json()) as { deploy_id: string; status: string };
    expect(body.deploy_id).toBe("deploy-xyz789");
    expect(body.status).toBe("queued");
  });

  test("GET status SSE streams 4 phases ending in complete", async () => {
    const res = await fetch(`${baseUrl}/api/v1/deploys/deploy-xyz789/status`);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-type")).toContain("text/event-stream");
    const text = await res.text();
    expect(text).toContain('"phase":"building"');
    expect(text).toContain('"phase":"built"');
    expect(text).toContain('"phase":"deploying"');
    expect(text).toContain('"phase":"complete"');
  });

  test("POST /api/v1/auth/refresh returns 200 with new token", async () => {
    const res = await fetch(`${baseUrl}/api/v1/auth/refresh`, { method: "POST" });
    expect(res.status).toBe(200);
    const body = (await res.json()) as { token: string; expires_at: string };
    expect(body.token).toBe("refreshed-token");
    expect(body.expires_at).toMatch(/^\d{4}-\d{2}-\d{2}T/);
  });

  test("unknown route returns 404", async () => {
    const res = await fetch(`${baseUrl}/api/v1/unknown`);
    expect(res.status).toBe(404);
  });
});
