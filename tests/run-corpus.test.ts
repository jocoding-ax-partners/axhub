// Phase 2 PLAN execution — tests/run-corpus.sh must be a real deterministic
// fixture replay runner, not an empty placeholder writer.

import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const REPO_ROOT = join(import.meta.dir, "..");
const RUNNER = join(REPO_ROOT, "tests/run-corpus.sh");

const run = (args: string[]) =>
  spawnSync("bash", [RUNNER, ...args], {
    cwd: REPO_ROOT,
    encoding: "utf8",
    timeout: 30000,
  });

const readJson = (path: string) => JSON.parse(readFileSync(path, "utf8"));

describe("tests/run-corpus.sh fixture replay runner", () => {
  test("plugin mode writes committed 20-row plugin results instead of an empty placeholder", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-corpus-"));
    try {
      const out = join(dir, "plugin.json");
      const result = run(["--mode", "plugin", "--corpus", "tests/corpus.20.jsonl", "--out", out]);
      expect(result.status).toBe(0);
      const rows = readJson(out);
      expect(rows).toHaveLength(20);
      expect(rows[0].actual_tool_calls.length).toBeGreaterThan(0);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("plugin mode can score the 100-row committed arm against the matching baseline", () => {
    const result = run(["--mode", "plugin", "--corpus", "tests/corpus.100.jsonl", "--score"]);
    if (result.status !== 0) {
      process.stderr.write(`${result.stdout}\n${result.stderr}\n`);
    }
    expect(result.status).toBe(0);
    expect(result.stdout).toContain("M1.5 GO/KILL gate: GO");
  });


  test("docs-only score is informational and does not fail the runner", () => {
    const result = run(["--mode", "docs-only", "--corpus", "tests/corpus.20.jsonl", "--score"]);
    expect(result.status).toBe(0);
    expect(result.stderr).toContain("docs-only baseline");
    expect(result.stdout).toContain("M1.5 GO/KILL gate: KILL");
  });

  test("full corpus without explicit fixture fails closed instead of fabricating results", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-corpus-"));
    try {
      const out = join(dir, "plugin.json");
      const result = run(["--mode", "plugin", "--corpus", "tests/corpus.jsonl", "--out", out]);
      expect(result.status).toBe(2);
      expect(result.stderr).toContain("no committed plugin fixture for 331-row corpus");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
