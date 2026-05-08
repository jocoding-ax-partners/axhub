// Phase 9 sub-task 9.3 — routing-tune.ts unit tests (mock LlmClient).

import { describe, expect, test } from "bun:test";
import { chmodSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import {
  findFailingCases,
  parseConfusedStats,
  runTune,
  shouldUseOnlineTuneClient,
  type BaselineEntry,
  type CorpusRow,
  type FailingCase,
  type LlmClient,
} from "../scripts/routing-tune";

const SAMPLE_CORPUS: CorpusRow[] = [
  { id: "T1", utterance: "내 앱 목록 보여줘", expected_skill: "apps" },
  { id: "T2", utterance: "이거 배포해줘", expected_skill: "deploy" },
  { id: "T3", utterance: "로그 보여줘", expected_skill: "logs" },
  { id: "M1", utterance: "이 코드 어떻게 동작해?", expected_skill: null },
];

const DOCS_ONLY: BaselineEntry[] = [
  { utterance_id: "T1", fired_skill: "apps" },
  { utterance_id: "T2", fired_skill: "deploy" },
  { utterance_id: "T3", fired_skill: "deploy" },
  { utterance_id: "M1", fired_skill: null },
];

const CLAUDE_NATIVE: BaselineEntry[] = [
  { utterance_id: "T1", fired_skill: "apps" },
  { utterance_id: "T2", fired_skill: "deploy" },
  { utterance_id: "T3", fired_skill: "logs" },
  { utterance_id: "M1", fired_skill: null },
];

class MockLlmClient implements LlmClient {
  public callCount = 0;
  async suggest(failing: FailingCase) {
    this.callCount += 1;
    return {
      description_additions: [`mock-add-for-${failing.utterance_id}`],
      example_additions: [{ utterance: failing.utterance, intent: `axhub ${failing.expected_skill} intent` }],
    };
  }
}

describe("findFailingCases", () => {
  test("filters drift > 0 rows only — T3 docs-only ≠ claude-native → 1 'drift' case (priority)", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    const t3 = cases.filter((c) => c.utterance_id === "T3");
    // Phase 11 fix — dedup: drift > docs-only > claude-native priority. 1 case per utterance_id.
    expect(t3.length).toBe(1);
    expect(t3[0]?.source).toBe("drift");
  });

  test("no_duplicate_failing_case_per_utterance_id (Phase 11 dedup)", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    const ids = cases.map((c) => c.utterance_id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  test("expected_skill = null rows skipped", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    expect(cases.every((c) => c.utterance_id !== "M1")).toBe(true);
  });

  test("matching baselines produce no failing cases for that row", () => {
    const cases = findFailingCases({
      corpus: SAMPLE_CORPUS,
      docsOnly: DOCS_ONLY,
      claudeNative: CLAUDE_NATIVE,
    });
    expect(cases.every((c) => c.utterance_id !== "T1")).toBe(true);
    expect(cases.every((c) => c.utterance_id !== "T2")).toBe(true);
  });
});

describe("parseConfusedStats", () => {
  test("confused_mode_consumes_audit_feedback: hash + chosen_skill records become tune input", () => {
    const parsed = parseConfusedStats(
      JSON.stringify({
        total_prompts: 2,
        confused_prompts: [
          { hash: "sha256:abc", count: 1, chosen_skill: "status", latest_ts: "2026-05-08T00:00:00Z" },
          { hash: "sha256:def", count: 2, chosen_skill: null },
        ],
      }),
    );

    expect(parsed.total_prompts).toBe(2);
    expect(parsed.confused_prompts).toEqual([
      { hash: "sha256:abc", count: 1, chosen_skill: "status", latest_ts: "2026-05-08T00:00:00Z" },
      { hash: "sha256:def", count: 2, chosen_skill: null },
    ]);
  });

  test("supports legacy top_axhub_hashes fallback without chosen skill", () => {
    const parsed = parseConfusedStats(
      JSON.stringify({ total_prompts: 1, top_axhub_hashes: [{ hash: "sha256:legacy", count: 3 }] }),
    );
    expect(parsed.confused_prompts).toEqual([
      { hash: "sha256:legacy", count: 3, chosen_skill: null, latest_ts: undefined },
    ]);
  });
});

describe("client mode selection", () => {
  test("dry-run stays deterministic/offline even when credentials exist", () => {
    expect(shouldUseOnlineTuneClient(false, [])).toBe(false);
    expect(shouldUseOnlineTuneClient(false, ["--dry-run"])).toBe(false);
  });

  test("online LLM mode requires explicit online flag or apply", () => {
    expect(shouldUseOnlineTuneClient(false, ["--online", "--dry-run"])).toBe(true);
    expect(shouldUseOnlineTuneClient(false, ["--llm", "--dry-run"])).toBe(true);
    expect(shouldUseOnlineTuneClient(true, ["--apply"])).toBe(true);
  });
});

describe("routing:tune CLI", () => {

  test("explicit AXHUB_HELPERS_BIN failure is fatal and does not fall back", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-routing-tune-bad-helper-"));
    const helper = join(dir, "axhub-helpers");
    writeFileSync(
      helper,
      ["#!/usr/bin/env bash", "echo configured helper broken >&2", "exit 42"].join("\n"),
    );
    chmodSync(helper, 0o755);

    const result = spawnSync("bun", ["run", "scripts/routing-tune.ts", "--confused"], {
      cwd: join(import.meta.dir, ".."),
      encoding: "utf8",
      env: { ...process.env, AXHUB_HELPERS_BIN: helper },
    });

    expect(result.status).not.toBe(0);
    expect(result.stderr).toContain("AXHUB_HELPERS_BIN");
    expect(result.stderr).toContain(helper);
    expect(result.stderr).toContain("configured helper broken");
    expect(result.stderr).not.toContain("fallback");
  });

  test("confused package entrypoint consumes helper JSON", () => {
    const dir = mkdtempSync(join(tmpdir(), "axhub-routing-tune-"));
    const helper = join(dir, "axhub-helpers");
    writeFileSync(
      helper,
      [
        "#!/usr/bin/env bash",
        "if [ \"$1\" = \"routing-stats\" ] && [ \"$2\" = \"--confused\" ] && [ \"$3\" = \"--json\" ]; then",
        "  printf '%s\\n' '{\"total_prompts\":1,\"confused_prompts\":[{\"hash\":\"sha256:test\",\"count\":1,\"chosen_skill\":\"logs\",\"latest_ts\":\"2026-05-08T00:00:00Z\"}]}'",
        "  exit 0",
        "fi",
        "exit 64",
      ].join("\n"),
    );
    chmodSync(helper, 0o755);

    const result = spawnSync("bun", ["run", "scripts/routing-tune.ts", "--confused"], {
      cwd: join(import.meta.dir, ".."),
      encoding: "utf8",
      env: { ...process.env, AXHUB_HELPERS_BIN: helper },
    });

    expect(result.status).toBe(0);
    expect(result.stdout).toContain('"hash": "sha256:test"');
    expect(result.stdout).toContain('"next_command": "bun run routing:tune --skill logs --dry-run"');
  });
});

describe("runTune", () => {
  test("llm_call_mock_returns_valid_json: mock LLM → suggestion JSON parse", async () => {
    const failing: FailingCase[] = [
      {
        utterance_id: "T3",
        utterance: "로그 보여줘",
        expected_skill: "logs",
        actual_skill: "deploy",
        source: "docs-only",
      },
    ];
    const llm = new MockLlmClient();
    const suggestions = await runTune({
      failingCases: failing,
      llm,
      skillsDir: "skills",
    });
    expect(suggestions.length).toBe(1);
    expect(suggestions[0]?.skill).toBe("logs");
    expect(suggestions[0]?.description_additions).toContain("mock-add-for-T3");
    expect(suggestions[0]?.example_additions[0]?.utterance).toBe("로그 보여줘");
    expect(llm.callCount).toBe(1);
  });

  test("skill_filter_works: --skill flag → only that SKILL processed", async () => {
    const failing: FailingCase[] = [
      {
        utterance_id: "T3",
        utterance: "로그 보여줘",
        expected_skill: "logs",
        actual_skill: "deploy",
        source: "docs-only",
      },
      {
        utterance_id: "T2",
        utterance: "배포해",
        expected_skill: "deploy",
        actual_skill: "logs",
        source: "drift",
      },
    ];
    const llm = new MockLlmClient();
    const suggestions = await runTune({
      failingCases: failing,
      llm,
      skillFilter: "logs",
      skillsDir: "skills",
    });
    expect(suggestions.length).toBe(1);
    expect(suggestions[0]?.skill).toBe("logs");
    expect(llm.callCount).toBe(1);
  });

  test("dry_run_outputs_suggestions_no_files_changed: runTune does not touch filesystem", async () => {
    const failing: FailingCase[] = [
      {
        utterance_id: "T3",
        utterance: "로그",
        expected_skill: "logs",
        actual_skill: "deploy",
        source: "docs-only",
      },
    ];
    const llm = new MockLlmClient();
    const before = await Bun.file("skills/logs/SKILL.md").text();
    await runTune({ failingCases: failing, llm, skillsDir: "skills" });
    const after = await Bun.file("skills/logs/SKILL.md").text();
    expect(after).toBe(before);
  });
});
