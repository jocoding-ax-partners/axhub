#!/usr/bin/env bun
/**
 * Phase 9 sub-task 9.3 — routing:tune.
 *
 * Pipeline:
 *   1. corpus.{tier}.jsonl + baseline-results.{docs-only,claude-native}.{tier}.json 읽기
 *   2. failing case 추출 (corpus.expected_skill ≠ baseline.fired_skill 또는 docs-only ↔ claude-native drift)
 *   3. 각 SKILL 의 failing utterance 그룹화 → LLM call (Claude Sonnet) → description / examples 보강 suggestion
 *   4. --dry-run: stdout JSON. --apply: git branch + SKILL.md edit + commit + push + gh PR draft
 *
 * LlmClient + 기타 entry point DI design (test 시 mock-able).
 *
 * Usage:
 *   bun run scripts/routing-tune.ts                    # dry-run, full corpus.100
 *   bun run scripts/routing-tune.ts --skill deploy     # 1 SKILL 만
 *   bun run scripts/routing-tune.ts --apply            # PR draft 생성
 *   bun run scripts/routing-tune.ts --confused         # Phase 10 의 clarify confusion log input (stub)
 */

import { execSync } from "node:child_process";
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

export function findFailingCases(opts: {
  corpus: CorpusRow[];
  docsOnly: BaselineEntry[];
  claudeNative: BaselineEntry[];
}): FailingCase[] {
  const docsMap = new Map(opts.docsOnly.map((e) => [e.utterance_id, e.fired_skill]));
  const cnMap = new Map(opts.claudeNative.map((e) => [e.utterance_id, e.fired_skill]));
  const cases: FailingCase[] = [];
  for (const row of opts.corpus) {
    if (!row.expected_skill) continue;
    const docs = docsMap.get(row.id);
    const cn = cnMap.get(row.id);
    if (docs !== undefined && docs !== row.expected_skill) {
      cases.push({
        utterance_id: row.id,
        utterance: row.utterance,
        expected_skill: row.expected_skill,
        actual_skill: docs,
        source: "docs-only",
      });
    }
    if (cn !== undefined && cn !== row.expected_skill) {
      cases.push({
        utterance_id: row.id,
        utterance: row.utterance,
        expected_skill: row.expected_skill,
        actual_skill: cn,
        source: "claude-native",
      });
    }
    if (docs !== undefined && cn !== undefined && docs !== cn) {
      cases.push({
        utterance_id: row.id,
        utterance: row.utterance,
        expected_skill: row.expected_skill,
        actual_skill: cn,
        source: "drift",
      });
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
    const prompt = [
      "당신은 axhub plugin 의 SKILL 작성자예요. 다음 routing failure 를 수정해야 해요:",
      "",
      `발화: "${failing.utterance}"`,
      `기대된 skill: ${failing.expected_skill}`,
      `실제 fired skill: ${failing.actual_skill ?? "null"}`,
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

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const apply = argv.includes("--apply");
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
    const result = execSync("bin/axhub-helpers routing-stats --confused --json", {
      encoding: "utf8",
      cwd: REPO_ROOT,
    });
    const parsed = JSON.parse(result || "{}") as { total_prompts?: number; top_axhub_hashes?: { hash: string; count: number }[] };
    const total = parsed.total_prompts ?? 0;
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
    process.stdout.write(JSON.stringify(parsed, null, 2) + "\n");
    return;
  }

  const apiKey = process.env["ANTHROPIC_API_KEY"];
  if (!apiKey) {
    process.stderr.write("ANTHROPIC_API_KEY env var 필요해요.\n");
    process.exit(1);
  }
  const model = process.env["CLAUDE_MODEL"] ?? "claude-sonnet-4-6";

  const tier = corpus.includes("corpus.20") ? "20" : "100";
  const corpusRows = loadCorpus(join(REPO_ROOT, corpus));
  const docsOnly = loadBaseline(join(REPO_ROOT, `tests/baseline-results.docs-only.${tier}.json`));
  const claudeNative = loadBaseline(join(REPO_ROOT, `tests/baseline-results.claude-native.${tier}.json`));
  const failing = findFailingCases({ corpus: corpusRows, docsOnly, claudeNative });

  const llm = new AnthropicTuneClient(apiKey, model);
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
