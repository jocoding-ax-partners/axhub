// Tests for hooks/session-start.ps1 — Windows PowerShell SessionStart hook mirror.
// Pure file-text assertions (no PS spawn — pwsh not on macOS dev host).

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const SESSION_PS1 = join(REPO_ROOT, "hooks/session-start.ps1");
const TELEMETRY_TS = join(REPO_ROOT, "src/axhub-helpers/telemetry.ts");
const INDEX_TS = join(REPO_ROOT, "src/axhub-helpers/index.ts");

describe("hooks/session-start.ps1 — Windows SessionStart mirror", () => {
  test("first line: $ErrorActionPreference = 'Stop' (US-1000 outcome A — silent assumed)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toContain("$ErrorActionPreference = 'Stop'");
  });

  test("$Helper path uses CLAUDE_PLUGIN_ROOT + bin/axhub-helpers.exe", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toContain("CLAUDE_PLUGIN_ROOT");
    expect(ps1).toContain("axhub-helpers.exe");
  });

  test("install.ps1 spawn captures $LASTEXITCODE and surfaces non-zero (D3)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toContain("$installExit = $LASTEXITCODE");
    expect(ps1).toMatch(/\$installExit\s+-ne\s+0/);
  });

  test("`& $Helper session-start` — direct binary call (not PS-spawned)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toContain("& $Helper session-start");
  });

  test("`& $Helper token-init` — auto-trigger after auth status check", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toContain("& $Helper token-init");
    expect(ps1).toContain("axhub auth status");
  });

  test("MAX_PATH catch via [System.IO.PathTooLongException] (pre-mortem #5)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toContain("PathTooLongException");
    expect(ps1).toMatch(/MAX_PATH|260/);
    expect(ps1).toContain("LongPathsEnabled");
  });

  test("AMSI/EDR pattern detection in catch-all (pre-mortem #2)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    expect(ps1).toMatch(/AntiMalwareProvider|AMSI|quarantine|virus|threat/);
    expect(ps1).toContain("Authenticode");
    expect(ps1).toContain("보안 솔루션");
  });

  test("state dir uses XDG_STATE_HOME (NOT %LOCALAPPDATA%) — mirrors telemetry.ts:40-44 (F2)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    const tel = readFileSync(TELEMETRY_TS, "utf8");
    expect(ps1).toContain("XDG_STATE_HOME");
    expect(ps1).toContain(".local\\state\\axhub-plugin");
    // F2 regression guard — must NOT use %LOCALAPPDATA%
    expect(ps1).not.toContain("LOCALAPPDATA");
    expect(ps1).not.toContain("$LocalAppData");
    // Telemetry.ts must also reference XDG_STATE_HOME for the mirror to be valid
    expect(tel).toContain("XDG_STATE_HOME");
  });

  test("token dir uses XDG_CONFIG_HOME — DISTINCT from state dir (mirrors index.ts cmdTokenInit)", () => {
    const ps1 = readFileSync(SESSION_PS1, "utf8");
    const idx = readFileSync(INDEX_TS, "utf8");
    expect(ps1).toContain("XDG_CONFIG_HOME");
    expect(ps1).toContain(".config\\axhub-plugin");
    // index.ts cmdTokenInit must use XDG_CONFIG_HOME (not XDG_STATE_HOME)
    expect(idx).toContain("XDG_CONFIG_HOME");
    // ps1 must have BOTH state dir AND token dir as separate variables
    expect(ps1).toContain("$StateDir");
    expect(ps1).toContain("$TokenDir");
  });
});
