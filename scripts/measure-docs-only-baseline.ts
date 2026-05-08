#!/usr/bin/env bun
/**
 * Phase 8 — fresh docs-only baseline measurement script.
 *
 * Pipeline (docs/baseline-measurement.md):
 *   1. corpus.100 row 별 LLM call (Claude Sonnet, description 만 매칭 추천)
 *   2. stdin review (accept / override / null / skip)
 *   3. JSON 정규화 → tests/baseline-results.docs-only.{tier}.json
 *
 * LlmClient + ReviewerPrompt 둘 다 dependency injection 가능 (test 에서 mock).
 *
 * Usage:
 *   bun run scripts/measure-docs-only-baseline.ts                          # corpus.100
 *   bun run scripts/measure-docs-only-baseline.ts --corpus tests/corpus.20.jsonl
 *   bun run scripts/measure-docs-only-baseline.ts --skip-prompt            # auto-accept Claude
 *   bun run scripts/measure-docs-only-baseline.ts --output /tmp/out.json   # custom output
 */

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import * as readline from "node:readline";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");

export interface CorpusRow {
  id: string;
  utterance: string;
  intent: string;
  expected_skill: string | null;
}

export interface SkillDescription {
  name: string;
  description: string;
}

export interface LlmRecommendation {
  skill: string | null;
  confidence: "high" | "medium" | "low";
}

export interface ReviewerDecision {
  kind: "accept" | "override" | "null" | "skip";
  overrideSkill?: string;
}

export interface BaselineEntry {
  utterance_id: string;
  fired_skill: string | null;
  actual_tool_calls: never[];
  required_consent_seen: boolean;
  notes: string;
}

export interface BaselineMetadata {
  _metadata: {
    measured_at: string;
    reviewer: string;
    corpus_version: string;
    skills_version: string;
    claude_model: string;
    rows_measured: number;
    rows_skipped: number;
    decisions: { accept: number; override: number; null: number };
  };
}

export interface LlmClient {
  recommend(utterance: string, skills: SkillDescription[]): Promise<LlmRecommendation>;
}

export interface ReviewerPrompt {
  ask(row: CorpusRow, recommendation: LlmRecommendation): Promise<ReviewerDecision>;
}

export function loadCorpus(path: string): CorpusRow[] {
  const text = readFileSync(path, "utf8");
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith("//"))
    .map((l) => JSON.parse(l) as CorpusRow);
}

export function loadSkills(skillsDir: string): SkillDescription[] {
  const skills: SkillDescription[] = [];
  for (const name of readdirSync(skillsDir)) {
    const skillPath = join(skillsDir, name, "SKILL.md");
    let raw: string;
    try {
      raw = readFileSync(skillPath, "utf8");
    } catch {
      continue;
    }
    const match = raw.match(/^description:\s*(['"])([\s\S]*?)\1$/m);
    if (!match || !match[2]) continue;
    skills.push({ name, description: match[2] });
  }
  return skills;
}

export function buildLlmPrompt(utterance: string, skills: SkillDescription[]): string {
  const skillBlock = skills
    .map((s) => `- ${s.name}: ${s.description.slice(0, 200)}`)
    .join("\n");
  return [
    "당신은 axhub plugin 의 routing 분류기예요.",
    "사용자 발화를 보고 가장 적합한 SKILL name 1 개 또는 null (axhub 도구 호출 의도 X) 결정해요.",
    "",
    `발화: "${utterance}"`,
    "",
    "후보 SKILL:",
    skillBlock,
    "",
    "JSON 형식으로 응답:",
    `{"skill": "skill_name_or_null", "confidence": "high" | "medium" | "low"}`,
  ].join("\n");
}

export function entryFromDecision(
  row: CorpusRow,
  recommendation: LlmRecommendation,
  decision: ReviewerDecision,
  measuredAt: string,
): BaselineEntry | null {
  if (decision.kind === "skip") return null;
  let firedSkill: string | null;
  switch (decision.kind) {
    case "accept":
      firedSkill = recommendation.skill;
      break;
    case "override":
      firedSkill = decision.overrideSkill ?? null;
      break;
    case "null":
      firedSkill = null;
      break;
  }
  const decisionLabel = decision.kind === "accept" ? `accept (claude ${recommendation.confidence})` : decision.kind;
  return {
    utterance_id: row.id,
    fired_skill: firedSkill,
    actual_tool_calls: [],
    required_consent_seen: false,
    notes: `docs-only fresh baseline ${measuredAt.slice(0, 10)}. reviewer: ${decisionLabel}.`,
  };
}

export async function runMeasurement(opts: {
  corpus: CorpusRow[];
  skills: SkillDescription[];
  llm: LlmClient;
  prompt: ReviewerPrompt;
  measuredAt?: string;
  reviewer?: string;
  corpusVersion?: string;
  skillsVersion?: string;
  claudeModel?: string;
}): Promise<{ metadata: BaselineMetadata; entries: BaselineEntry[] }> {
  const measuredAt = opts.measuredAt ?? new Date().toISOString();
  const decisions = { accept: 0, override: 0, null: 0 };
  const entries: BaselineEntry[] = [];
  let skipped = 0;

  for (const row of opts.corpus) {
    const recommendation = await opts.llm.recommend(row.utterance, opts.skills);
    const decision = await opts.prompt.ask(row, recommendation);
    if (decision.kind === "skip") {
      skipped += 1;
      continue;
    }
    decisions[decision.kind] += 1;
    const entry = entryFromDecision(row, recommendation, decision, measuredAt);
    if (entry) entries.push(entry);
  }

  const metadata: BaselineMetadata = {
    _metadata: {
      measured_at: measuredAt,
      reviewer: opts.reviewer ?? "unknown",
      corpus_version: opts.corpusVersion ?? "unknown",
      skills_version: opts.skillsVersion ?? "unknown",
      claude_model: opts.claudeModel ?? "claude-sonnet-4-6",
      rows_measured: entries.length,
      rows_skipped: skipped,
      decisions,
    },
  };
  return { metadata, entries };
}

class AnthropicClient implements LlmClient {
  constructor(private readonly apiKey: string, private readonly model: string) {}
  async recommend(utterance: string, skills: SkillDescription[]): Promise<LlmRecommendation> {
    const body = {
      model: this.model,
      max_tokens: 100,
      temperature: 0,
      messages: [{ role: "user", content: buildLlmPrompt(utterance, skills) }],
    };
    const response = await fetch("https://api.anthropic.com/v1/messages", {
      method: "POST",
      headers: {
        "x-api-key": this.apiKey,
        "anthropic-version": "2023-06-01",
        "content-type": "application/json",
      },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(`Anthropic API ${response.status}: ${await response.text()}`);
    }
    const data = (await response.json()) as { content?: { text?: string }[] };
    const text = data.content?.[0]?.text ?? "{}";
    const match = text.match(/\{[\s\S]*\}/);
    const parsed = match ? JSON.parse(match[0]) : { skill: null, confidence: "low" };
    return {
      skill: typeof parsed.skill === "string" && parsed.skill !== "null" ? parsed.skill : null,
      confidence: ["high", "medium", "low"].includes(parsed.confidence) ? parsed.confidence : "low",
    };
  }
}

class StdinReviewerPrompt implements ReviewerPrompt {
  private readonly rl: readline.Interface;
  constructor() {
    this.rl = readline.createInterface({ input: process.stdin, output: process.stdout });
  }
  async ask(row: CorpusRow, recommendation: LlmRecommendation): Promise<ReviewerDecision> {
    process.stdout.write(
      [
        ``,
        `utterance: "${row.utterance}"`,
        `expected_skill (corpus): ${row.expected_skill ?? "null"}`,
        `Claude 추천: ${recommendation.skill ?? "null"} (confidence: ${recommendation.confidence})`,
        `[a] accept Claude / [o] override / [n] null / [s] skip`,
        `> `,
      ].join("\n"),
    );
    const answer = await new Promise<string>((resolve) => this.rl.question("", resolve));
    const trimmed = answer.trim().toLowerCase();
    if (trimmed === "a") return { kind: "accept" };
    if (trimmed === "n") return { kind: "null" };
    if (trimmed === "s") return { kind: "skip" };
    if (trimmed === "o") {
      const override = await new Promise<string>((resolve) =>
        this.rl.question("override skill name (e.g. deploy)> ", resolve),
      );
      return { kind: "override", overrideSkill: override.trim() || undefined };
    }
    return { kind: "skip" };
  }
  close(): void {
    this.rl.close();
  }
}

class AutoAcceptPrompt implements ReviewerPrompt {
  async ask(_row: CorpusRow, _rec: LlmRecommendation): Promise<ReviewerDecision> {
    return { kind: "accept" };
  }
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  let corpusPath = join(REPO_ROOT, "tests/corpus.100.jsonl");
  let outputPath: string | null = null;
  let skipPrompt = false;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--corpus" && i + 1 < argv.length) {
      corpusPath = argv[i + 1] ?? corpusPath;
      i += 1;
    } else if (arg === "--output" && i + 1 < argv.length) {
      outputPath = argv[i + 1] ?? null;
      i += 1;
    } else if (arg === "--skip-prompt") {
      skipPrompt = true;
    }
  }

  if (outputPath === null) {
    const tier = corpusPath.includes("corpus.20") ? "20" : "100";
    outputPath = join(REPO_ROOT, `tests/baseline-results.docs-only.${tier}.json`);
  }

  const apiKey = process.env["ANTHROPIC_API_KEY"];
  if (!apiKey) {
    process.stderr.write("ANTHROPIC_API_KEY env var 필요해요. docs/baseline-measurement.md 참고.\n");
    process.exit(1);
  }
  const model = process.env["CLAUDE_MODEL"] ?? "claude-sonnet-4-6";
  const reviewer = process.env["BASELINE_REVIEWER"] ?? process.env["USER"] ?? "unknown";

  const corpus = loadCorpus(corpusPath);
  const skills = loadSkills(SKILLS_DIR);
  const llm = new AnthropicClient(apiKey, model);
  const prompt: ReviewerPrompt = skipPrompt ? new AutoAcceptPrompt() : new StdinReviewerPrompt();

  const { metadata, entries } = await runMeasurement({
    corpus,
    skills,
    llm,
    prompt,
    reviewer,
    corpusVersion: corpusPath,
    skillsVersion: "skills/",
    claudeModel: model,
  });

  const output: (BaselineMetadata | BaselineEntry)[] = [metadata, ...entries];
  writeFileSync(outputPath, JSON.stringify(output, null, 2) + "\n");
  if (prompt instanceof StdinReviewerPrompt) prompt.close();
  process.stdout.write(
    `\n[measure-baseline] wrote ${entries.length} entries (${metadata._metadata.rows_skipped} skipped) to ${outputPath}\n`,
  );
}

if (import.meta.main) {
  main().catch((e) => {
    process.stderr.write(`error: ${e instanceof Error ? e.message : String(e)}\n`);
    process.exit(1);
  });
}
