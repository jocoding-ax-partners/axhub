// Phase 3 PLAN execution — session-start should run real preflight diagnostics,
// not emit the old M0 placeholder.

import { describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const REPO_ROOT = join(import.meta.dir, "..");
const HELPER_SRC = join(REPO_ROOT, "src/axhub-helpers/index.ts");

const runSessionStart = (env: Record<string, string>) =>
  spawnSync(process.execPath, [HELPER_SRC, "session-start"], {
    cwd: REPO_ROOT,
    env: { ...process.env, AXHUB_TELEMETRY: "0", ...env },
    encoding: "utf8",
    timeout: 10000,
  });

const parseOutput = (stdout: string): { systemMessage: string } => JSON.parse(stdout.trim());

describe("session-start preflight diagnostics", () => {
  test("reports CLI/auth/profile diagnostics and removes placeholder copy", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-session-"));
    try {
      const axhub = join(dir, "axhub");
      writeFileSync(
        axhub,
        [
          "#!/usr/bin/env bash",
          "set -euo pipefail",
          "if [ \"${1:-}\" = \"--version\" ]; then echo 'axhub 0.1.20 (test)'; exit 0; fi",
          "if [ \"${1:-}\" = \"auth\" ] && [ \"${2:-}\" = \"status\" ]; then",
          "  echo '{\"user_email\":\"giri@jocodingax.ai\",\"user_id\":1,\"expires_at\":\"2026-12-31T00:00:00Z\",\"scopes\":[\"deploy\"]}'",
          "  exit 0",
          "fi",
          "exit 64",
          "",
        ].join("\n"),
      );
      chmodSync(axhub, 0o755);

      const result = runSessionStart({ PATH: `${dir}:${process.env.PATH ?? ""}`, AXHUB_PROFILE: "production", AXHUB_ENDPOINT: "https://hub-api.jocodingax.ai" });
      expect(result.status).toBe(0);
      const payload = parseOutput(result.stdout);
      expect(payload.systemMessage).toContain("Plugin v");
      expect(payload.systemMessage).toContain("axhub CLI 0.1.20 OK");
      expect(payload.systemMessage).toContain("giri@jocodingax.ai");
      expect(payload.systemMessage).toContain("profile=production");
      expect(payload.systemMessage).not.toContain("M0 scaffold");
      expect(payload.systemMessage).not.toContain("placeholder");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("missing CLI still exits zero with actionable guidance", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-session-"));
    try {
      const result = runSessionStart({ PATH: dir });
      expect(result.status).toBe(0);
      const payload = parseOutput(result.stdout);
      expect(payload.systemMessage).toContain("axhub CLI를 찾지 못했어요");
      expect(payload.systemMessage).toContain("axhub auth login");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
