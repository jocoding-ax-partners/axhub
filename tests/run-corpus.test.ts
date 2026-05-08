// Phase 2 PLAN execution — tests/run-corpus.sh must be a real deterministic
// fixture replay runner, not an empty placeholder writer.

import { describe, expect, test } from "bun:test";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
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
  test("plugin mode writes committed 20-row plugin results (Phase 5: 20 base + 3 meta_question = 23)", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-corpus-"));
    try {
      const out = join(dir, "plugin.json");
      const result = run(["--mode", "plugin", "--corpus", "tests/corpus.20.jsonl", "--out", out]);
      expect(result.status).toBe(0);
      const rows = readJson(out);
      expect(rows).toHaveLength(23);
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
      // Phase 5 — corpus.jsonl row count is 346 (331 base + 15 meta_question).
      expect(result.stderr).toContain("no committed plugin fixture for 346-row corpus");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("vs_claude_native_diff_5pct: mock baselines (100% match) → routing-score exit 0", () => {
    // Synthetic convergence: copy claude-native.20.json over docs-only.20.json so
    // routing-score sees drift 0% / accuracy match → verdict PASS.
    const dir = mkdtempSync(join(tmpdir(), "axhub-corpus-vs-"));
    try {
      const fakeRoot = dir;
      mkdirSync(join(fakeRoot, "tests"), { recursive: true });

      const copy = (src: string, dst: string) =>
        copyFileSync(join(REPO_ROOT, src), join(fakeRoot, dst));
      copy("tests/corpus.20.jsonl", "tests/corpus.20.jsonl");
      copy("tests/baseline-results.claude-native.20.json", "tests/baseline-results.claude-native.20.json");
      copy("tests/baseline-results.claude-native.20.json", "tests/baseline-results.docs-only.20.json");
      copy("tests/routing-score.ts", "tests/routing-score.ts");
      copy("tests/run-corpus.sh", "tests/run-corpus.sh");

      const result = spawnSync(
        "bash",
        [join(fakeRoot, "tests/run-corpus.sh"), "--mode", "plugin", "--corpus", "tests/corpus.20.jsonl", "--vs", "claude-native", "--score"],
        {
          cwd: fakeRoot,
          encoding: "utf8",
          timeout: 30000,
          env: { ...process.env, PLUGIN_ROOT: fakeRoot },
        },
      );
      if (result.status !== 0) {
        process.stderr.write(`stdout=${result.stdout}\nstderr=${result.stderr}\n`);
      }
      expect(result.status).toBe(0);
      expect(result.stdout).toContain("verdict: PASS");
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test("331_row_advisory_mode: corpus.jsonl + --vs claude-native + --score → advisory + exit 0", () => {
    const result = run(["--mode", "plugin", "--corpus", "tests/corpus.jsonl", "--vs", "claude-native", "--score"]);
    expect(result.status).toBe(0);
    expect(result.stderr).toContain("ADVISORY");
    expect(result.stderr).toContain("manual/advisory");
  });

  test("meta_question rows null match: fixture has fired_skill=null + null-rejection metric reaches 100% on identical baselines", () => {
    // Fixture validity: corpus.20 의 3 meta rows (M1, M3, M11) 가 baseline 의 fired_skill = null.
    const claudeNative = readJson("tests/baseline-results.claude-native.20.json");
    const docsOnly = readJson("tests/baseline-results.docs-only.20.json");
    for (const id of ["M1", "M3", "M11"]) {
      const cn = claudeNative.find((r: { utterance_id: string }) => r.utterance_id === id);
      const dl = docsOnly.find((r: { utterance_id: string }) => r.utterance_id === id);
      expect(cn?.fired_skill).toBeNull();
      expect(dl?.fired_skill).toBeNull();
    }

    // routing-score metric: identical baselines → null-rejection-rate 100% on full set.
    const dir = mkdtempSync(join(tmpdir(), "axhub-corpus-meta-"));
    try {
      const fakeRoot = dir;
      mkdirSync(join(fakeRoot, "tests"), { recursive: true });
      const copy = (src: string, dst: string) =>
        copyFileSync(join(REPO_ROOT, src), join(fakeRoot, dst));
      copy("tests/corpus.20.jsonl", "tests/corpus.20.jsonl");
      copy("tests/baseline-results.claude-native.20.json", "tests/baseline-results.claude-native.20.json");
      copy("tests/baseline-results.claude-native.20.json", "tests/baseline-results.docs-only.20.json");
      copy("tests/routing-score.ts", "tests/routing-score.ts");
      copy("tests/run-corpus.sh", "tests/run-corpus.sh");

      const result = spawnSync(
        "bash",
        [join(fakeRoot, "tests/run-corpus.sh"), "--mode", "plugin", "--corpus", "tests/corpus.20.jsonl", "--vs", "claude-native", "--score"],
        { cwd: fakeRoot, encoding: "utf8", timeout: 30000, env: { ...process.env, PLUGIN_ROOT: fakeRoot } },
      );
      expect(result.status).toBe(0);
      expect(result.stdout).toMatch(/null-rejection:\s+100\.00%/);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});
