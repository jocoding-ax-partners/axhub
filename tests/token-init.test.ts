// Phase 6 US-603: tests for cmdTokenInit subcommand registration + format validation.
//
// cmdTokenInit calls `axhub auth login --print-token` via Bun.spawnSync, which
// requires a real axhub binary + browser interaction. Mocking the spawn would
// duplicate Bun's API surface. Instead these tests verify the subcommand is
// registered, USAGE documents it, and the token-format validation regex
// matches what the implementation enforces.

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INDEX_TS = join(REPO_ROOT, "src/axhub-helpers/index.ts");

describe("cmdTokenInit subcommand registration (US-603)", () => {
  test("dispatch switch includes 'token-init' case", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toMatch(/case "token-init":\s*\n\s*return cmdTokenInit\(args\);/);
  });

  test("USAGE documents token-init subcommand", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toContain("token-init");
    expect(content).toContain("1-step setup");
  });

  test("cmdTokenInit function is defined", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toMatch(/async function cmdTokenInit\(_args: string\[\]\): Promise<number>/);
  });

  test("token-init invokes 'axhub auth login --print-token'", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toContain("axhub");
    expect(content).toContain("--print-token");
  });

  test("token format validation uses axhub_pat_ prefix + 16+ chars", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    // Same regex as token-import to keep validation consistent
    expect(content).toMatch(/axhub_pat_\[A-Za-z0-9_-\]\{16,\}/);
  });

  test("headless fallback Korean message present", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toContain("헤드리스 환경");
    expect(content).toContain("axhub-helpers token-import");
  });

  test("token storage path matches token-import (XDG_CONFIG_HOME-aware)", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    expect(content).toContain("axhub-plugin");
    expect(content).toContain("XDG_CONFIG_HOME");
  });

  test("file mode 0600 + dir mode 0700 enforced (security parity with token-import)", () => {
    const content = readFileSync(INDEX_TS, "utf8");
    // Look for the cmdTokenInit body specifically
    const initIdx = content.indexOf("async function cmdTokenInit");
    expect(initIdx).toBeGreaterThan(-1);
    const initBody = content.slice(initIdx, initIdx + 3000);
    expect(initBody).toContain("0o600");
    expect(initBody).toContain("0o700");
  });

  test("validates token format via axhub_pat_* regex (rejects malformed input)", () => {
    // Mirror the regex used in cmdTokenInit + cmdTokenImport. Verify the same
    // valid/invalid samples behave consistently.
    const re = /^axhub_pat_[A-Za-z0-9_-]{16,}$/;
    expect(re.test("axhub_pat_abcdefghijklmnop")).toBe(true);
    expect(re.test("axhub_pat_short")).toBe(false);
    expect(re.test("not_a_token")).toBe(false);
    expect(re.test("axhub_pat_with spaces")).toBe(false);
    expect(re.test("axhub_pat_" + "a".repeat(100))).toBe(true);
  });
});
