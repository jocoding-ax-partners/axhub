// Phase 0.5.12 US-1707 — bin/statusline.ps1 Windows native contract.
// Last 2 tests gated by SKIP_NON_WIN (only run on Windows CI runner).

import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

const REPO = join(import.meta.dir, "..");
const PS1 = join(REPO, "bin/statusline.ps1");
const SKIP_NON_WIN = process.platform !== "win32";

describe("Phase 0.5.12 — bin/statusline.ps1 Windows native contract", () => {
  test("file exists", () => {
    expect(existsSync(PS1)).toBe(true);
  });

  test("UTF-8 BOM encoded", () => {
    const buf = readFileSync(PS1);
    expect(buf[0]).toBe(0xef);
    expect(buf[1]).toBe(0xbb);
    expect(buf[2]).toBe(0xbf);
  });

  test("header references mirror parity", () => {
    const body = readFileSync(PS1, "utf8");
    expect(body).toMatch(/statusline\.sh/);
    expect(body).toMatch(/US-1707|Phase 17|v0\.5\.12/);
  });

  test("body uses 해요체 (no 합니다/입니다/시겠어요/드립니다/당신/아이고)", () => {
    const body = readFileSync(PS1, "utf8");
    expect(body).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });

  test("references axhub-helpers.exe fast path", () => {
    const body = readFileSync(PS1, "utf8");
    expect(body).toMatch(/axhub-helpers(\.exe)?/);
  });

  test.skipIf(SKIP_NON_WIN)("PS1 syntax parses (pwsh)", () => {
    const r = spawnSync(
      "pwsh",
      [
        "-NoProfile",
        "-Command",
        `[scriptblock]::Create((Get-Content -Raw '${PS1}')) | Out-Null`,
      ],
      { encoding: "utf8" },
    );
    expect(r.status).toBe(0);
  });

  test.skipIf(SKIP_NON_WIN)("PS1 invocation prints axhub: prefix in <500ms", () => {
    const start = Date.now();
    const r = spawnSync(
      "powershell.exe",
      ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", PS1],
      { encoding: "utf8", timeout: 1500 },
    );
    const elapsed = Date.now() - start;
    expect(r.status).toBe(0);
    expect(r.stdout).toMatch(/^axhub:/);
    expect(elapsed).toBeLessThan(500);
  });
});
