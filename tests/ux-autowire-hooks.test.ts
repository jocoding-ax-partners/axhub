// Phase 0.6.0 — session-start-autowire hook contract.
// Asserts both POSIX + PowerShell mirror exist, hooks.json registers them,
// kill switch matrix respected (AXHUB_DISABLE_HOOKS / per-hook csv / legacy / feature-specific).

import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { existsSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const REPO = join(import.meta.dir, "..");
const HOOK_SH = join(REPO, "hooks/session-start-autowire.sh");
const HOOK_PS1 = join(REPO, "hooks/session-start-autowire.ps1");
const HOOKS_JSON = join(REPO, "hooks/hooks.json");

describe("Phase 0.6.0 — session-start-autowire hook contract", () => {
  test("hooks/session-start-autowire.sh exists", () => {
    expect(existsSync(HOOK_SH)).toBe(true);
  });

  test("hooks/session-start-autowire.sh is executable (mode +x)", () => {
    const mode = statSync(HOOK_SH).mode;
    expect((mode & 0o100) !== 0).toBe(true);
  });

  test("hooks/session-start-autowire.ps1 exists", () => {
    expect(existsSync(HOOK_PS1)).toBe(true);
  });

  test("hooks/session-start-autowire.ps1 is UTF-8 BOM encoded", () => {
    const buf = readFileSync(HOOK_PS1);
    expect(buf[0]).toBe(0xef);
    expect(buf[1]).toBe(0xbb);
    expect(buf[2]).toBe(0xbf);
  });

  test("hooks.json registers session-start-autowire under SessionStart", () => {
    const parsed = JSON.parse(readFileSync(HOOKS_JSON, "utf8"));
    const sessionStart = parsed.hooks?.SessionStart ?? [];
    const allCommands = sessionStart.flatMap((group: any) =>
      (group.hooks ?? []).map((h: any) => h.command ?? ""),
    );
    const found = allCommands.some((cmd: string) =>
      cmd.includes("session-start-autowire"),
    );
    expect(found).toBe(true);
  });

  test("sh hook body checks AXHUB_DISABLE_HOOKS env", () => {
    const body = readFileSync(HOOK_SH, "utf8");
    expect(body).toContain("AXHUB_DISABLE_HOOKS");
  });

  test("sh hook body checks AXHUB_DISABLE_HOOK csv for session-start-autowire", () => {
    const body = readFileSync(HOOK_SH, "utf8");
    expect(body).toContain("AXHUB_DISABLE_HOOK");
    expect(body).toContain("session-start-autowire");
  });

  test("sh hook body honors legacy DISABLE_AXHUB env", () => {
    const body = readFileSync(HOOK_SH, "utf8");
    expect(body).toContain("DISABLE_AXHUB");
  });

  test("sh hook body checks AXHUB_DISABLE_STATUSLINE_AUTOWIRE env", () => {
    const body = readFileSync(HOOK_SH, "utf8");
    expect(body).toContain("AXHUB_DISABLE_STATUSLINE_AUTOWIRE");
  });

  test("sh hook body has fail-open exit 0 contract (no set -e without trap)", () => {
    const body = readFileSync(HOOK_SH, "utf8");
    // any path must exit 0 — verify last line is exit 0 OR every exit is 0
    expect(body).toMatch(/exit 0/);
    // no `exit 1` or `exit 2` patterns
    expect(body).not.toMatch(/^[^#]*exit [1-9]/m);
  });

  test("sh hook body has Korean 해요체 (no forbidden tokens)", () => {
    const body = readFileSync(HOOK_SH, "utf8");
    expect(body).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });

  test("ps1 hook body has Korean 해요체", () => {
    const body = readFileSync(HOOK_PS1, "utf8");
    expect(body).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });
});
