// Phase 25 PR 25.3 — tests for the PostToolUse deploy artifact verifier.
//
// Coverage:
//   1. `verifyUserAppArtifact` — pure verifier on stdout strings
//   2. `_helpers.ts` — kill switch precedence + axhub deploy command detection
//   3. End-to-end hook subprocess — verifies systemMessage emission paths
//      including kill switch + fail-open contract

import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

import { verifyUserAppArtifact } from "../scripts/verify-user-app-artifact";
import {
  __resetLegacyWarningForTest,
  isAxhubDeployCommand,
  isHookDisabled,
} from "../hooks/_helpers";

const root = join(import.meta.dir, "..");
const hookPath = join(root, "hooks/post-tool-verify-deploy-artifacts.ts");

const KILL_SWITCH_ENVS = [
  "AXHUB_DISABLE_HOOKS",
  "AXHUB_DISABLE_HOOK",
  "DISABLE_AXHUB",
] as const;

function clearKillSwitchEnvs() {
  for (const key of KILL_SWITCH_ENVS) {
    delete process.env[key];
  }
  __resetLegacyWarningForTest();
}

function runHook(stdin: string, env: Record<string, string> = {}) {
  const finalEnv = { ...process.env } as NodeJS.ProcessEnv;
  for (const key of KILL_SWITCH_ENVS) {
    delete finalEnv[key];
  }
  Object.assign(finalEnv, env);
  return spawnSync("bun", [hookPath], {
    cwd: root,
    input: stdin,
    encoding: "utf8",
    env: finalEnv,
    timeout: 7_000,
  });
}

describe("verifyUserAppArtifact", () => {
  test("non-JSON stdout → pass (fail-open, no signal)", () => {
    const r = verifyUserAppArtifact("Deployment started\nLive at https://app.example.com\n");
    expect(r.passed).toBe(true);
    expect(r.violations).toHaveLength(0);
  });

  test("valid manifest_hash + live state → pass", () => {
    const stdout = JSON.stringify({
      manifest_hash: "a".repeat(64),
      state: "live",
      url: "https://app.example.com",
      deployment_id: "dep_abc123",
    });
    const r = verifyUserAppArtifact(stdout);
    expect(r.passed).toBe(true);
    expect(r.violations).toHaveLength(0);
  });

  test("invalid manifest_hash → fail with hex violation", () => {
    const stdout = JSON.stringify({
      manifest_hash: "not-a-sha256",
      state: "live",
    });
    const r = verifyUserAppArtifact(stdout);
    expect(r.passed).toBe(false);
    expect(r.violations.join(",")).toContain("sha256 hex");
  });

  test("non-live state → fail with state violation", () => {
    const stdout = JSON.stringify({ state: "rolled_back" });
    const r = verifyUserAppArtifact(stdout);
    expect(r.passed).toBe(false);
    expect(r.violations.join(",")).toContain('state="rolled_back"');
  });

  test("non-https url → fail with url violation", () => {
    const stdout = JSON.stringify({ state: "running", url: "ftp://wrong.example.com" });
    const r = verifyUserAppArtifact(stdout);
    expect(r.passed).toBe(false);
    expect(r.violations.join(",")).toContain("http(s)");
  });

  test("empty deployment_id → fail with id violation", () => {
    const stdout = JSON.stringify({ state: "ok", deployment_id: "   " });
    const r = verifyUserAppArtifact(stdout);
    expect(r.passed).toBe(false);
    expect(r.violations.join(",")).toContain("deployment_id");
  });

  test("array payload → treated as non-JSON (fail-open)", () => {
    const r = verifyUserAppArtifact(JSON.stringify([1, 2, 3]));
    expect(r.passed).toBe(true);
  });
});

describe("_helpers", () => {
  beforeEach(clearKillSwitchEnvs);
  afterEach(clearKillSwitchEnvs);

  test("default state does not disable", () => {
    expect(isHookDisabled("post-tool-verify-deploy-artifacts")).toBe(false);
  });

  test("AXHUB_DISABLE_HOOKS=1 disables every hook", () => {
    process.env.AXHUB_DISABLE_HOOKS = "1";
    expect(isHookDisabled("post-tool-verify-deploy-artifacts")).toBe(true);
    expect(isHookDisabled("classify-exit")).toBe(true);
  });

  test("AXHUB_DISABLE_HOOK csv disables only listed names", () => {
    process.env.AXHUB_DISABLE_HOOK = "post-tool-verify-deploy-artifacts , other";
    expect(isHookDisabled("post-tool-verify-deploy-artifacts")).toBe(true);
    expect(isHookDisabled("classify-exit")).toBe(false);
  });

  test("legacy DISABLE_AXHUB still honored (deprecation pending)", () => {
    process.env.DISABLE_AXHUB = "1";
    expect(isHookDisabled("post-tool-verify-deploy-artifacts")).toBe(true);
  });

  test("truthy variants recognized for canonical env", () => {
    for (const value of ["1", "true", "yes", "on"]) {
      clearKillSwitchEnvs();
      process.env.AXHUB_DISABLE_HOOKS = value;
      expect(isHookDisabled("any")).toBe(true);
    }
  });

  test("isAxhubDeployCommand matches axhub deploy create only", () => {
    expect(isAxhubDeployCommand({ command: "axhub deploy create" })).toBe(true);
    expect(isAxhubDeployCommand({ command: "  axhub  deploy   create --json" })).toBe(true);
    expect(isAxhubDeployCommand({ command: "axhub deploy list" })).toBe(false);
    expect(isAxhubDeployCommand({ command: "ls" })).toBe(false);
    expect(isAxhubDeployCommand(null)).toBe(false);
    expect(isAxhubDeployCommand({})).toBe(false);
  });
});

describe("post-tool-verify-deploy-artifacts hook (subprocess)", () => {
  const goodPayload = JSON.stringify({
    tool_input: { command: "axhub deploy create --json" },
    tool_response: {
      exit_code: 0,
      stdout: JSON.stringify({
        manifest_hash: "a".repeat(64),
        state: "live",
        url: "https://app.example.com",
      }),
    },
  });

  const badPayload = JSON.stringify({
    tool_input: { command: "axhub deploy create --json" },
    tool_response: {
      exit_code: 0,
      stdout: JSON.stringify({ state: "rolled_back" }),
    },
  });

  test("happy path → exit 0, no systemMessage", () => {
    const out = runHook(goodPayload);
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });

  test("violation path → exit 0 with systemMessage warning (fail-open)", () => {
    const out = runHook(badPayload);
    expect(out.status).toBe(0);
    const stdout = out.stdout.toString();
    expect(stdout).toContain("systemMessage");
    expect(stdout).toContain("rolled_back");
  });

  test("non-deploy command → silent skip", () => {
    const payload = JSON.stringify({
      tool_input: { command: "ls -la" },
      tool_response: { exit_code: 0, stdout: "" },
    });
    const out = runHook(payload);
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });

  test("deploy failure (non-zero exit) → silent skip (artifact untrusted)", () => {
    const payload = JSON.stringify({
      tool_input: { command: "axhub deploy create" },
      tool_response: {
        exit_code: 64,
        stdout: JSON.stringify({ state: "rolled_back" }),
      },
    });
    const out = runHook(payload);
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });

  test("AXHUB_DISABLE_HOOKS=1 → silent skip", () => {
    const out = runHook(badPayload, { AXHUB_DISABLE_HOOKS: "1" });
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });

  test("AXHUB_DISABLE_HOOK=post-tool-verify-deploy-artifacts → silent skip", () => {
    const out = runHook(badPayload, {
      AXHUB_DISABLE_HOOK: "post-tool-verify-deploy-artifacts",
    });
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });

  test("malformed JSON stdin → fail-open exit 0 without crash", () => {
    const out = runHook("not json at all");
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });

  test("missing tool_response → fail-open silent skip", () => {
    const payload = JSON.stringify({
      tool_input: { command: "axhub deploy create" },
    });
    const out = runHook(payload);
    expect(out.status).toBe(0);
    expect(out.stdout.toString().trim()).toBe("");
  });
});

describe("verify-user-app-artifact CLI (pipe contract)", () => {
  test("happy path JSON returns exit 0 + passed:true", () => {
    const stdin = JSON.stringify({ state: "live", manifest_hash: "b".repeat(64) });
    const out = spawnSync("bun", ["scripts/verify-user-app-artifact.ts"], {
      cwd: root,
      input: stdin,
      encoding: "utf8",
      timeout: 5_000,
    });
    expect(out.status).toBe(0);
    const parsed = JSON.parse(out.stdout.toString().trim());
    expect(parsed.passed).toBe(true);
  });

  test("violation JSON returns exit 1 + violations populated", () => {
    const stdin = JSON.stringify({ state: "rolled_back" });
    const out = spawnSync("bun", ["scripts/verify-user-app-artifact.ts"], {
      cwd: root,
      input: stdin,
      encoding: "utf8",
      timeout: 5_000,
    });
    expect(out.status).toBe(1);
    const parsed = JSON.parse(out.stdout.toString().trim());
    expect(parsed.passed).toBe(false);
    expect(parsed.violations.length).toBeGreaterThan(0);
  });
});
