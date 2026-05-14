import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import {
  getStatuslineSnippet,
  getStatuslineSnippetUnix,
  getStatuslineSnippetWindows,
} from "../scripts/codegen-statusline-snippet";

const REPO = join(import.meta.dir, "..");
const SCRIPT = join(REPO, "scripts/codegen-statusline-snippet.ts");
const SKILL = join(REPO, "skills/enable-statusline/SKILL.md");

describe("codegen-statusline-snippet drift lock", () => {
  test("--check exits 0 when SKILL has matching snippet", () => {
    const res = spawnSync("bun", [SCRIPT, "--check"], { encoding: "utf8", cwd: REPO });
    expect(res.status).toBe(0);
  });
  test("--check exits non-zero on drift", () => {
    const backup = readFileSync(SKILL, "utf8");
    try {
      writeFileSync(SKILL, backup.replace(/"statusLine"/g, '"TAMPERED_STATUSLINE"'));
      const res = spawnSync("bun", [SCRIPT, "--check"], { encoding: "utf8", cwd: REPO });
      expect(res.status).not.toBe(0);
    } finally {
      writeFileSync(SKILL, backup);
    }
  });
  test("--write idempotent across two runs", () => {
    const r1 = spawnSync("bun", [SCRIPT, "--write"], { encoding: "utf8", cwd: REPO });
    expect(r1.status).toBe(0);
    const after1 = readFileSync(SKILL);
    const r2 = spawnSync("bun", [SCRIPT, "--write"], { encoding: "utf8", cwd: REPO });
    expect(r2.status).toBe(0);
    const after2 = readFileSync(SKILL);
    expect(after2.equals(after1)).toBe(true);
  });

  // Windows SSOT extension (v0.5.12)
  test("getStatuslineSnippetWindows export contains explicit powershell.exe -ExecutionPolicy Bypass form", () => {
    const snippet = getStatuslineSnippetWindows();
    expect(snippet).toContain("powershell.exe");
    expect(snippet).toContain("-NoProfile");
    expect(snippet).toContain("-ExecutionPolicy Bypass");
    expect(snippet).toContain("-File");
    expect(snippet).toContain("statusline.ps1");
    // Must have JSON-escaped quotes around the path
    expect(snippet).toContain('\\"${CLAUDE_PLUGIN_ROOT}/bin/statusline.ps1\\"');
  });
  test("getStatuslineSnippet alias equals getStatuslineSnippetUnix", () => {
    expect(getStatuslineSnippet()).toBe(getStatuslineSnippetUnix());
  });
  test("--check exits 0 with both Unix + Windows blocks in sync", () => {
    // Both _UNIX and _WINDOWS markers exist after Task #5; --check must pass both.
    const res = spawnSync("bun", [SCRIPT, "--check"], { encoding: "utf8", cwd: REPO });
    expect(res.status).toBe(0);
    expect(res.stdout).toContain("Unix + Windows snippets in sync");
  });
  test("--check detects Windows block drift", () => {
    const backup = readFileSync(SKILL, "utf8");
    try {
      // Tamper only the Windows block
      writeFileSync(
        SKILL,
        backup.replace(
          '"powershell.exe -NoProfile -ExecutionPolicy Bypass -File',
          '"TAMPERED_WIN_CMD'
        )
      );
      const res = spawnSync("bun", [SCRIPT, "--check"], { encoding: "utf8", cwd: REPO });
      expect(res.status).not.toBe(0);
      expect(res.stderr).toMatch(/Windows statusline snippet drift|TAMPERED/);
    } finally {
      writeFileSync(SKILL, backup);
    }
  });
});
