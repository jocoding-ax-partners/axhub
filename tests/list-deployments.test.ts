// Phase 5 US-501: tests for list-deployments helper subcommand.

import { describe, expect, test, beforeEach, afterEach } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync, mkdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  runListDeployments,
  resolveToken,
  EXIT_LIST_OK,
  EXIT_LIST_AUTH,
  EXIT_LIST_NOT_FOUND,
  EXIT_LIST_TRANSPORT,
} from "../src/axhub-helpers/list-deployments";

let scratchDir: string;
let originalEnv: Record<string, string | undefined>;

beforeEach(() => {
  scratchDir = mkdtempSync(join(tmpdir(), "axhub-list-deployments-"));
  originalEnv = {
    AXHUB_TOKEN: process.env["AXHUB_TOKEN"],
    AXHUB_ENDPOINT: process.env["AXHUB_ENDPOINT"],
    XDG_CONFIG_HOME: process.env["XDG_CONFIG_HOME"],
  };
  process.env["XDG_CONFIG_HOME"] = scratchDir;
  delete process.env["AXHUB_TOKEN"];
  delete process.env["AXHUB_ENDPOINT"];
});

afterEach(() => {
  for (const [k, v] of Object.entries(originalEnv)) {
    if (v === undefined) delete process.env[k];
    else process.env[k] = v;
  }
  rmSync(scratchDir, { recursive: true, force: true });
});

describe("token discovery (US-501)", () => {
  test("returns null when no token source available", () => {
    expect(resolveToken()).toBeNull();
  });

  test("AXHUB_TOKEN env var takes precedence", () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_envvar_token_value";
    expect(resolveToken()).toBe("axhub_pat_envvar_token_value");
  });

  test("falls back to ${XDG_CONFIG_HOME}/axhub-plugin/token file", () => {
    const dir = join(scratchDir, "axhub-plugin");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "token"), "axhub_pat_file_token_value\n");
    expect(resolveToken()).toBe("axhub_pat_file_token_value");
  });
});

describe("runListDeployments — auth gate (US-501)", () => {
  test("missing token returns exit 65 + Korean message", async () => {
    const result = await runListDeployments({ appId: "42" });
    expect(result.exit_code).toBe(EXIT_LIST_AUTH);
    expect(result.error_code).toBe("auth.token_missing");
    expect(result.error_message_kr).toContain("토큰을 찾을 수 없어요");
  });

  test("invalid app id returns exit 67", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    const result = await runListDeployments({ appId: "not-a-number" });
    expect(result.exit_code).toBe(EXIT_LIST_NOT_FOUND);
    expect(result.error_code).toBe("validation.app_id_invalid");
  });
});

describe("runListDeployments — REST API (US-501)", () => {
  test("successful list returns deployments with status name mapping", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    const fakeFetch = async (_url: string | URL | Request, _init?: RequestInit): Promise<Response> => {
      return new Response(
        JSON.stringify({
          success: true,
          data: [
            { id: 384, app_id: 6, status: 3, commit_sha: "abc123", commit_message: "fix", branch: "main", created_at: "2026-04-23T10:00:00Z" },
            { id: 383, app_id: 6, status: 4, commit_sha: "def456", commit_message: "broken", branch: "feature", created_at: "2026-04-22T10:00:00Z" },
          ],
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    };
    const result = await runListDeployments({ appId: "6", limit: 5 }, fakeFetch as unknown as typeof fetch);
    expect(result.exit_code).toBe(EXIT_LIST_OK);
    expect(result.deployments.length).toBe(2);
    expect(result.deployments[0].status).toBe("active");
    expect(result.deployments[1].status).toBe("failed");
    expect(result.deployments[0].id).toBe(384);
  });

  test("401 returns auth error with re-login Korean message", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_expired";
    const fakeFetch = async (): Promise<Response> => new Response("Unauthorized", { status: 401 });
    const result = await runListDeployments({ appId: "6" }, fakeFetch as unknown as typeof fetch);
    expect(result.exit_code).toBe(EXIT_LIST_AUTH);
    expect(result.error_code).toBe("auth.token_invalid");
    expect(result.error_message_kr).toContain("axhub auth login");
  });

  test("404 returns app-not-found error", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    const fakeFetch = async (): Promise<Response> => new Response("Not Found", { status: 404 });
    const result = await runListDeployments({ appId: "999999" }, fakeFetch as unknown as typeof fetch);
    expect(result.exit_code).toBe(EXIT_LIST_NOT_FOUND);
    expect(result.error_code).toBe("resource.app_not_found");
  });

  test("network error returns transport error", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    const fakeFetch = async (): Promise<Response> => {
      throw new Error("ECONNREFUSED");
    };
    const result = await runListDeployments({ appId: "6" }, fakeFetch as unknown as typeof fetch);
    expect(result.exit_code).toBe(EXIT_LIST_TRANSPORT);
    expect(result.error_code).toBe("transport.network_error");
    expect(result.error_message_kr).toContain("ECONNREFUSED");
  });

  test("AXHUB_ENDPOINT env var override is used", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    process.env["AXHUB_ENDPOINT"] = "https://staging-api.jocodingax.ai";
    let capturedUrl = "";
    const fakeFetch = async (url: string | URL | Request): Promise<Response> => {
      capturedUrl = url.toString();
      return new Response(JSON.stringify({ success: true, data: [] }), { status: 200 });
    };
    const result = await runListDeployments({ appId: "6" }, fakeFetch as unknown as typeof fetch);
    expect(result.exit_code).toBe(EXIT_LIST_OK);
    expect(capturedUrl).toContain("staging-api.jocodingax.ai");
    expect(result.endpoint_used).toBe("https://staging-api.jocodingax.ai");
  });

  test("default endpoint when AXHUB_ENDPOINT unset", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    let capturedUrl = "";
    const fakeFetch = async (url: string | URL | Request): Promise<Response> => {
      capturedUrl = url.toString();
      return new Response(JSON.stringify({ success: true, data: [] }), { status: 200 });
    };
    await runListDeployments({ appId: "6" }, fakeFetch as unknown as typeof fetch);
    expect(capturedUrl).toContain("hub-api.jocodingax.ai");
  });

  test("limit parameter propagates to per_page query", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_test";
    let capturedUrl = "";
    const fakeFetch = async (url: string | URL | Request): Promise<Response> => {
      capturedUrl = url.toString();
      return new Response(JSON.stringify({ success: true, data: [] }), { status: 200 });
    };
    await runListDeployments({ appId: "6", limit: 3 }, fakeFetch as unknown as typeof fetch);
    expect(capturedUrl).toContain("per_page=3");
  });

  test("Bearer token sent in Authorization header", async () => {
    process.env["AXHUB_TOKEN"] = "axhub_pat_my_token_42";
    let capturedAuth = "";
    const fakeFetch = async (_url: string | URL | Request, init?: RequestInit): Promise<Response> => {
      capturedAuth = (init?.headers as Record<string, string>)?.["Authorization"] ?? "";
      return new Response(JSON.stringify({ success: true, data: [] }), { status: 200 });
    };
    await runListDeployments({ appId: "6" }, fakeFetch as unknown as typeof fetch);
    expect(capturedAuth).toBe("Bearer axhub_pat_my_token_42");
  });
});
