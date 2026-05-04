// Phase 17 US-1707 — statusline binary contract.
// bin/statusline.sh exists, executable, runs <50ms, output ≤80 chars in 해요체.

import { describe, expect, test } from "bun:test";
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");
const STATUSLINE = join(REPO_ROOT, "bin/statusline.sh");

describe("Phase 17 C7/US-1707 — statusline binary contract", () => {
  test("bin/statusline.sh exists", () => {
    expect(existsSync(STATUSLINE)).toBe(true);
  });

  test("bin/statusline.sh is executable (mode +x)", () => {
    const mode = statSync(STATUSLINE).mode;
    // owner exec bit set (S_IXUSR = 0o100)
    expect((mode & 0o100) !== 0).toBe(true);
  });

  test("bin/statusline.sh delegates to the Rust helper when available", () => {
    const content = readFileSync(STATUSLINE, "utf8");
    expect(content).toContain("axhub-helpers");
    expect(content).toContain("statusline");
  });

  test("bin/statusline.sh runs and exits 0 in <500ms cold", () => {
    const start = Date.now();
    const result = spawnSync(STATUSLINE, [], { encoding: "utf8", timeout: 1000 });
    const elapsed = Date.now() - start;
    expect(result.status).toBe(0);
    // Latency budget: 500ms cold (target 50ms but file-system + bash startup
    // varies; 500ms is the upper bound for a "non-blocking" UX guarantee)
    expect(elapsed).toBeLessThan(500);
  });

  test("bin/statusline.sh uses the helper output on the primary path", () => {
    const temp = mkdtempSync(join(tmpdir(), "axhub-statusline-helper-"));
    try {
      const helper = join(temp, "axhub-helpers");
      writeFileSync(
        helper,
        "#!/bin/sh\n[ \"$1\" = statusline ] || exit 64\nprintf 'axhub: helper path\\n'\n",
      );
      chmodSync(helper, 0o755);
      const result = spawnSync(STATUSLINE, [], {
        encoding: "utf8",
        env: { ...process.env, AXHUB_HELPER_BIN: helper },
        timeout: 1000,
      });
      expect(result.status).toBe(0);
      expect(result.stdout).toBe("axhub: helper path\n");
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  test("bin/statusline.sh keeps the portable shell fallback when helper is unavailable", () => {
    const temp = mkdtempSync(join(tmpdir(), "axhub-statusline-fallback-"));
    try {
      const result = spawnSync(STATUSLINE, [], {
        encoding: "utf8",
        env: {
          ...process.env,
          AXHUB_HELPER_BIN: join(temp, "missing-helper"),
          AXHUB_TOKEN: "",
          HOME: temp,
          XDG_CONFIG_HOME: join(temp, ".config"),
          XDG_CACHE_HOME: join(temp, ".cache"),
        },
        timeout: 1000,
      });
      expect(result.status).toBe(0);
      expect(result.stdout).toBe("axhub: 로그인 안 됐어요\n");
    } finally {
      rmSync(temp, { recursive: true, force: true });
    }
  });

  test("bin/statusline.sh output is ≤80 characters", () => {
    const result = spawnSync(STATUSLINE, [], { encoding: "utf8", timeout: 1000 });
    const lines = result.stdout.trim().split("\n");
    for (const line of lines) {
      expect(Array.from(line).length).toBeLessThanOrEqual(80);
    }
  });

  test("bin/statusline.sh output uses 해요체 (no forbidden Toss tokens)", () => {
    const result = spawnSync(STATUSLINE, [], { encoding: "utf8", timeout: 1000 });
    expect(result.stdout).not.toMatch(/합니다|입니다|시겠어요|드립니다|당신|아이고/);
  });

  test("bin/statusline.sh output starts with 'axhub:' prefix (identity marker)", () => {
    const result = spawnSync(STATUSLINE, [], { encoding: "utf8", timeout: 1000 });
    expect(result.stdout.trim().startsWith("axhub:")).toBe(true);
  });
});
