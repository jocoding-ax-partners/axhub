import { describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

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
});
