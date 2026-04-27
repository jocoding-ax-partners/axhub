// Tests for bin/install.ps1 — Windows PowerShell installer mirror of bin/install.sh.
// Pure file-text assertions (no PS spawn — pwsh not on macOS dev host).

import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const INSTALL_PS1 = join(REPO_ROOT, "bin/install.ps1");
const INSTALL_SH = join(REPO_ROOT, "bin/install.sh");

describe("bin/install.ps1 — Windows installer mirror", () => {
  test("RELEASE_VERSION literal matches install.sh:48 (parity assertion)", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    const sh = readFileSync(INSTALL_SH, "utf8");
    const ps1Version = ps1.match(/\$ReleaseVersion\s*=\s*if\s*\([^)]*\)[^']*'([^']+)'/);
    const shVersion = sh.match(/RELEASE_VERSION="?\$\{[^:]*:-(v[0-9.]+)/);
    expect(ps1Version).not.toBeNull();
    expect(shVersion).not.toBeNull();
    expect(ps1Version![1]).toBe(shVersion![1]);
  });

  test("AMD64 arch detection via PROCESSOR_ARCHITECTURE", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    expect(ps1).toContain("PROCESSOR_ARCHITECTURE");
    expect(ps1).toContain("AMD64");
    expect(ps1).toContain("amd64");
  });

  test("Invoke-WebRequest with -TimeoutSec 600 (slow corp network pre-mortem #3)", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    expect(ps1).toContain("Invoke-WebRequest");
    expect(ps1).toContain("-TimeoutSec 600");
  });

  test("Move-Item + Start-Sleep + Test-Path re-check (Defender post-Move pre-mortem #6)", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    expect(ps1).toContain("Move-Item");
    expect(ps1).toContain("Start-Sleep -Seconds 2");
    expect(ps1).toMatch(/Move-Item[\s\S]{0,300}Start-Sleep[\s\S]{0,300}Test-Path/);
  });

  test("NO Add-Type / NO Reflection.Assembly (EDR-clean — Phase 9 PInvoke not used in installer)", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    expect(ps1).not.toContain("Add-Type");
    expect(ps1).not.toContain("Reflection.Assembly");
    expect(ps1).not.toContain("DllImport");
  });

  test("Explicit Test-Path / Remove-Item NOT install.sh:80 || operator (D4)", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    // Must use explicit pattern
    expect(ps1).toContain("Test-Path -Path $LinkPath -PathType Any");
    expect(ps1).toContain("Remove-Item -Path $LinkPath -Force");
    // Must reference install.sh:80 bug in NOTE comment
    expect(ps1).toContain("install.sh:80");
    expect(ps1).toContain("operator precedence bug");
  });

  test("Every catch emits ConvertTo-Json @{ systemMessage = ... } envelope + exit 0 (F3)", () => {
    const ps1 = readFileSync(INSTALL_PS1, "utf8");
    // Count catch blocks (try/catch with optional typed exception)
    const catchKeywords = ps1.match(/^\s*}\s*catch\s*(\[[^\]]+\])?\s*\{/gm);
    expect(catchKeywords).not.toBeNull();
    expect(catchKeywords!.length).toBeGreaterThanOrEqual(3); // PathTooLongException + WebException + catch-all
    // Count systemMessage envelope emits
    const envelopes = ps1.match(/ConvertTo-Json @\{ systemMessage =/g);
    expect(envelopes).not.toBeNull();
    // Must have at least one envelope per catch block
    expect(envelopes!.length).toBeGreaterThanOrEqual(catchKeywords!.length);
    // Count exit 0 statements (must be at least one per catch path)
    const exitZeros = ps1.match(/exit\s+0\s*$/gm);
    expect(exitZeros).not.toBeNull();
    expect(exitZeros!.length).toBeGreaterThanOrEqual(catchKeywords!.length);
  });
});
