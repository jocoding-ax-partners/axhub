#!/usr/bin/env bun
/**
 * Phase 9 sub-task 9.3 — routing:tune.
 *
 * Pipeline:
 *   1. corpus.{tier}.jsonl + baseline-results.{docs-only,claude-native}.{tier}.json 읽기
 *   2. failing case 추출 (corpus.expected_skill ≠ baseline.fired_skill 또는 docs-only ↔ claude-native drift)
 *   3. 각 SKILL 의 failing utterance 그룹화 → deterministic dry-run 또는 명시적 LLM call → suggestion
 *   4. --dry-run: stdout JSON. --apply: git branch + SKILL.md edit + commit + push + gh PR draft
 *
 * LlmClient + 기타 entry point DI design (test 시 mock-able).
 *
 * Usage:
 *   bun run scripts/routing-tune.ts                    # dry-run, full corpus.100
 *   bun run scripts/routing-tune.ts --skill deploy     # 1 SKILL 만
 *   bun run scripts/routing-tune.ts --online --dry-run # ANTHROPIC_API_KEY 로 LLM suggestion
 *   bun run scripts/routing-tune.ts --apply            # PR draft 생성
 *   bun run scripts/routing-tune.ts --confused         # Phase 10 clarify feedback hash input
 */

import { execFileSync, execSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { join } from "node:path";

const REPO_ROOT = join(import.meta.dir, "..");

export interface CorpusRow {
  id: string;
  utterance: string;
  expected_skill: string | null;
}

export interface BaselineEntry {
  utterance_id: string;
  fired_skill: string | null;
}

export interface FailingCase {
  utterance_id: string;
  utterance: string;
  expected_skill: string;
  actual_skill: string | null;
  source: "docs-only" | "claude-native" | "drift";
}

export interface TuneSuggestion {
  skill: string;
  case: FailingCase;
  description_additions: string[];
  example_additions: { utterance: string; intent: string }[];
}

export interface ConfusedPrompt {
  hash: string;
  count: number;
  chosen_skill: string | null;
  latest_ts?: string;
}

export interface ConfusedStats {
  total_prompts: number;
  confused_prompts: ConfusedPrompt[];
}

export interface LlmClient {
  suggest(failing: FailingCase, currentDescription: string): Promise<{
    description_additions: string[];
    example_additions: { utterance: string; intent: string }[];
  }>;
}

export function loadCorpus(path: string): CorpusRow[] {
  return readFileSync(path, "utf8")
    .split("\n")
    .filter((l) => l.trim().length > 0 && !l.startsWith("//"))
    .map((l) => JSON.parse(l) as CorpusRow);
}

export function loadBaseline(path: string): BaselineEntry[] {
  const arr = JSON.parse(readFileSync(path, "utf8")) as Array<BaselineEntry | Record<string, unknown>>;
  return arr.filter((r): r is BaselineEntry => typeof (r as BaselineEntry).utterance_id === "string");
}

export function parseConfusedStats(raw: string): ConfusedStats {
  type RawConfusedPrompt = {
    hash?: string;
    prompt_hash?: string;
    count?: number;
    chosen_skill?: string | null;
    latest_ts?: string;
  };
  const parsed = JSON.parse(raw || "{}") as {
    total_prompts?: number;
    confused_prompts?: RawConfusedPrompt[];
    top_axhub_hashes?: Array<{ hash?: string; count?: number }>;
  };
  const direct: RawConfusedPrompt[] = Array.isArray(parsed.confused_prompts) ? parsed.confused_prompts : [];
  const fallback: RawConfusedPrompt[] = direct.length === 0 && Array.isArray(parsed.top_axhub_hashes)
    ? parsed.top_axhub_hashes.map((h) => ({ ...h, chosen_skill: null }))
    : direct;
  const confused_prompts = fallback
    .map((p): ConfusedPrompt | null => {
      const hash = p.hash ?? p.prompt_hash;
      if (!hash) return null;
      return {
        hash,
        count: typeof p.count === "number" && Number.isFinite(p.count) ? p.count : 1,
        chosen_skill: p.chosen_skill ?? null,
        latest_ts: p.latest_ts,
      };
    })
    .filter((p): p is ConfusedPrompt => p !== null);
  return {
    total_prompts: parsed.total_prompts ?? confused_prompts.length,
    confused_prompts,
  };
}

export function findFailingCases(opts: {
  corpus: CorpusRow[];
  docsOnly: BaselineEntry[];
  claudeNative: BaselineEntry[];
}): FailingCase[] {
  const docsMap = new Map(opts.docsOnly.map((e) => [e.utterance_id, e.fired_skill]));
  const cnMap = new Map(opts.claudeNative.map((e) => [e.utterance_id, e.fired_skill]));
  // Phase 11 fix: 1 case per utterance_id (priority: drift > docs-only > claude-native).
  const cases: FailingCase[] = [];
  for (const row of opts.corpus) {
    if (!row.expected_skill) continue;
    const docs = docsMap.get(row.id);
    const cn = cnMap.get(row.id);
    const base = {
      utterance_id: row.id,
      utterance: row.utterance,
      expected_skill: row.expected_skill,
    } as const;
    if (docs !== undefined && cn !== undefined && docs !== cn) {
      cases.push({ ...base, actual_skill: cn, source: "drift" });
    } else if (docs !== undefined && docs !== row.expected_skill) {
      cases.push({ ...base, actual_skill: docs, source: "docs-only" });
    } else if (cn !== undefined && cn !== row.expected_skill) {
      cases.push({ ...base, actual_skill: cn, source: "claude-native" });
    }
  }
  return cases;
}

export function loadSkillDescription(slug: string, skillsDir: string): string {
  const path = join(skillsDir, slug, "SKILL.md");
  const raw = readFileSync(path, "utf8");
  const match = raw.match(/^description:\s*(['"])([\s\S]*?)\1$/m);
  return match?.[2] ?? "";
}

export async function runTune(opts: {
  failingCases: FailingCase[];
  llm: LlmClient;
  skillFilter?: string;
  skillsDir: string;
}): Promise<TuneSuggestion[]> {
  const filtered = opts.skillFilter
    ? opts.failingCases.filter((c) => c.expected_skill === opts.skillFilter)
    : opts.failingCases;
  const suggestions: TuneSuggestion[] = [];
  for (const fcase of filtered) {
    const description = loadSkillDescription(fcase.expected_skill, opts.skillsDir);
    const result = await opts.llm.suggest(fcase, description);
    suggestions.push({
      skill: fcase.expected_skill,
      case: fcase,
      description_additions: result.description_additions,
      example_additions: result.example_additions,
    });
  }
  return suggestions;
}

class AnthropicTuneClient implements LlmClient {
  constructor(private readonly apiKey: string, private readonly model: string) {}
  async suggest(failing: FailingCase, currentDescription: string) {
    // Phase 11 — sanitize prompt input (matches measure-docs-only-baseline:sanitizeForPrompt contract).
    const safe = (s: string, n = 200): string =>
      s
        .replace(/\\/g, "\\\\")
        .replace(/"/g, '\\"')
        .replace(/\n/g, "\\n")
        .replace(/\r/g, "\\r")
        .slice(0, n);
    const prompt = [
      "당신은 axhub plugin 의 SKILL 작성자예요. 다음 routing failure 를 수정해야 해요:",
      "",
      `발화: "${safe(failing.utterance)}"`,
      `기대된 skill: ${safe(failing.expected_skill, 64)}`,
      `실제 fired skill: ${safe(failing.actual_skill ?? "null", 64)}`,
      `failure source: ${failing.source}`,
      "",
      `현재 description:`,
      currentDescription.slice(0, 500),
      "",
      "이 발화 가 정확히 매칭되도록 description 의 trigger 어구 또는 examples 어느 쪽을 어떻게 보강하면 좋을지 1-3 suggestion JSON 으로:",
      `{"description_additions": ["어구1"], "example_additions": [{"utterance": "...", "intent": "..."}]}`,
    ].join("\n");
    const response = await fetch("https://api.anthropic.com/v1/messages", {
      method: "POST",
      headers: {
        "x-api-key": this.apiKey,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json",
      },
      body: JSON.stringify({
        model: this.model,
        max_tokens: 300,
        temperature: 0,
        messages: [{ role: "user", content: prompt }],
      }),
    });
    if (!response.ok) {
      throw new Error(`Anthropic API ${response.status}`);
    }
    const data = (await response.json()) as { content?: { text?: string }[] };
    const text = data.content?.[0]?.text ?? "{}";
    const match = text.match(/\{[\s\S]*\}/);
    const parsed = match ? JSON.parse(match[0]) : { description_additions: [], example_additions: [] };
    return {
      description_additions: Array.isArray(parsed.description_additions) ? parsed.description_additions : [],
      example_additions: Array.isArray(parsed.example_additions) ? parsed.example_additions : [],
    };
  }
}

class DeterministicDryRunClient implements LlmClient {
  async suggest(failing: FailingCase) {
    const normalized = failing.utterance.slice(0, 80);
    return {
      description_additions: [
        `${failing.expected_skill} intent phrase for ${failing.source} failure ${failing.utterance_id}`,
      ],
      example_additions: [
        {
          utterance: normalized,
          intent: `route utterance to ${failing.expected_skill}`,
        },
      ],
    };
  }
}

function gitBranchAndPr(suggestions: TuneSuggestion[]): void {
  if (suggestions.length === 0) {
    process.stderr.write("[routing-tune] no suggestions to apply\n");
    return;
  }
  const date = new Date().toISOString().slice(0, 10);
  const branch = `routing-tune/${date}-suggestions`;
  process.stderr.write(`[routing-tune] creating branch ${branch}\n`);
  execSync(`git checkout -b ${branch}`, { stdio: "inherit" });
  process.stderr.write(
    `[routing-tune] manual review required — apply suggestions to skills/*/SKILL.md, then:\n  git commit -m 'routing-tune suggestions ${date}'\n  git push -u origin ${branch}\n  gh pr create\n`,
  );
}

function summarizeStdout(suggestions: TuneSuggestion[]): void {
  process.stdout.write(JSON.stringify(suggestions, null, 2) + "\n");
}

export function shouldUseOnlineTuneClient(apply: boolean, argv: string[]): boolean {
  return apply || argv.includes("--online") || argv.includes("--llm");
}

function formatExecFailure(label: string, error: unknown): string {
  const err = error as { message?: string; stderr?: Buffer | string; stdout?: Buffer | string; status?: number };
  const stderr = typeof err.stderr === "string" ? err.stderr : err.stderr?.toString("utf8");
  const stdout = typeof err.stdout === "string" ? err.stdout : err.stdout?.toString("utf8");
  return [
    `${label}: ${err.message ?? String(error)}`,
    typeof err.status === "number" ? `exit_status: ${err.status}` : undefined,
    stderr?.trim() ? `stderr: ${stderr.trim()}` : undefined,
    stdout?.trim() ? `stdout: ${stdout.trim()}` : undefined,
  ]
    .filter((line): line is string => Boolean(line))
    .join("\n");
}

function execAxhubHelper(candidate: string, args: string[]): string {
  return execFileSync(candidate, args, {
    encoding: "utf8",
    cwd: REPO_ROOT,
    env: process.env,
    stdio: ["ignore", "pipe", "pipe"],
  });
}

function runAxhubHelpers(args: string[]): string {
  const configured = process.env["AXHUB_HELPERS_BIN"];
  if (configured) {
    try {
      return execAxhubHelper(configured, args);
    } catch (e) {
      throw new Error(`configured AXHUB_HELPERS_BIN failed (${configured})\n${formatExecFailure(configured, e)}`);
    }
  }

  const failures: string[] = [];
  const packaged = join(REPO_ROOT, "bin/axhub-helpers");
  try {
    return execAxhubHelper(packaged, args);
  } catch (e) {
    failures.push(formatExecFailure(packaged, e));
  }
  try {
    return execFileSync("cargo", ["run", "--quiet", "-p", "axhub-helpers", "--", ...args], {
      encoding: "utf8",
      cwd: REPO_ROOT,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (e) {
    failures.push(formatExecFailure("cargo run -p axhub-helpers", e));
  }
  throw new Error(failures.join("\n"));
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const apply = argv.includes("--apply");
  const online = shouldUseOnlineTuneClient(apply, argv);
  const confused = argv.includes("--confused");
  let skillFilter: string | undefined;
  let corpus = "tests/corpus.100.jsonl";
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--skill" && i + 1 < argv.length) {
      skillFilter = argv[i + 1];
      i += 1;
    } else if (argv[i] === "--corpus" && i + 1 < argv.length) {
      corpus = argv[i + 1] ?? corpus;
      i += 1;
    }
  }

  if (confused) {
    let result: string;
    try {
      result = runAxhubHelpers(["routing-stats", "--confused", "--json"]);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      process.stderr.write(
        `[routing-tune] axhub-helpers 호출 실패: ${msg}\n` +
          "  bin/axhub-helpers 가 stale 이면 cargo run -p axhub-helpers fallback 도 함께 실패했는지 확인해요.\n",
      );
      process.exit(1);
    }
    const parsed = parseConfusedStats(result);
    const total = parsed.total_prompts;
    process.stderr.write(
      [
        "",
        `[routing-tune --confused] clarify 발동 prompt ${total} 개`,
        "",
        "audit log 는 prompt 원문 저장 X (sha256 hash 만). chosen_skill 정보로 routing:tune 자동화 어려워요.",
        "manual review 절차:",
        "  1. routing-stats --confused 출력의 hash + chosen_skill 확인",
        "  2. corpus 또는 사용자 기억으로 원본 발화 lookup",
        "  3. bun run routing:tune --skill <chosen_skill> 으로 fresh suggestion",
        "",
      ].join("\n"),
    );
    process.stdout.write(
      JSON.stringify(
        {
          ...parsed,
          manual_review_required: true,
          next_command:
            parsed.confused_prompts[0]?.chosen_skill
              ? `bun run routing:tune --skill ${parsed.confused_prompts[0].chosen_skill} --dry-run`
              : "bun run routing:tune --confused",
        },
        null,
        2,
      ) + "\n",
    );
    return;
  }

  const apiKey = process.env["ANTHROPIC_API_KEY"];
  if (online && !apiKey) {
    process.stderr.write("ANTHROPIC_API_KEY env var 필요해요. offline dry-run 은 --online 없이 실행해요.\n");
    process.exit(1);
  }
  const model = process.env["CLAUDE_MODEL"] ?? "claude-sonnet-4-6";

  const tier = corpus.includes("corpus.20") ? "20" : "100";
  const corpusRows = loadCorpus(join(REPO_ROOT, corpus));
  const docsOnly = loadBaseline(join(REPO_ROOT, `tests/baseline-results.docs-only.${tier}.json`));
  const claudeNative = loadBaseline(join(REPO_ROOT, `tests/baseline-results.claude-native.${tier}.json`));
  const failing = findFailingCases({ corpus: corpusRows, docsOnly, claudeNative });

  const llm = online ? new AnthropicTuneClient(apiKey as string, model) : new DeterministicDryRunClient();
  const suggestions = await runTune({
    failingCases: failing,
    llm,
    skillFilter,
    skillsDir: join(REPO_ROOT, "skills"),
  });

  summarizeStdout(suggestions);
  if (apply) gitBranchAndPr(suggestions);
}

if (import.meta.main) {
  main().catch((e) => {
    process.stderr.write(`error: ${e instanceof Error ? e.message : String(e)}\n`);
    process.exit(1);
  });
}
