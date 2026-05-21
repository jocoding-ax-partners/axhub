#!/usr/bin/env bun
/**
 * Phase 9 sub-task 9.1.2 — codegen 18 SKILL.md frontmatter examples backfill.
 *
 * Each SKILL gets 5 examples. utterance source priority:
 *   1. corpus.100 의 expected_skill = X 인 row 의 utterance
 *   2. SKILL.md description 의 trigger 어구 (Phase 1 의 codegen 결과)
 * intent source: SKILL_INTENTS hardcoded mapping (per-skill English verb phrase).
 *
 * lang balance: ko ≥ 2 + en ≥ 2 enforce.
 *
 * Idempotent — 다시 실행해도 file 변경 0 (기존 examples 가 valid 면 skip).
 *
 * Modes:
 *   --dry-run (default): suggestion 만 stdout
 *   --apply: SKILL.md frontmatter 갱신
 */

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { readSkillDescription } from "./codegen-skill-keywords-from-rust";
import { isKorean, parseExamples, type SkillExample } from "./skill-doctor-quality";

const REPO_ROOT = join(import.meta.dir, "..");
const SKILLS_DIR = join(REPO_ROOT, "skills");
const CORPUS_PATH = join(REPO_ROOT, "tests/corpus.100.jsonl");

export const SKILL_INTENTS: Record<string, string> = {
  deploy: "deploy current branch to axhub live",
  status: "check axhub deployment status",
  logs: "view axhub deployment logs",
  recover: "rollback axhub deployment",
  apps: "list axhub apps",
  auth: "authenticate to axhub",
  update: "update axhub CLI",
  upgrade: "upgrade axhub plugin",
  doctor: "diagnose axhub setup",
  init: "scaffold new axhub app",
  env: "manage axhub environment variables",
  github: "connect github repo to axhub",
  open: "open axhub deployment in browser",
  whatsnew: "show axhub release notes",
  profile: "manage axhub profile",
  "install-cli": "install axhub CLI binary",
  clarify: "disambiguate axhub intent",
  "routing-stats": "show axhub routing statistics summary",
};

interface CorpusRow {
  utterance: string;
  expected_skill: string | null;
}

export function loadCorpusUtterances(path: string): Map<string, string[]> {
  const text = readFileSync(path, "utf8");
  const rows = text
    .split("\n")
    .filter((l) => l.trim().length > 0 && !l.startsWith("//"))
    .map((l) => JSON.parse(l) as CorpusRow);
  const grouped = new Map<string, string[]>();
  for (const row of rows) {
    if (!row.expected_skill) continue;
    let arr = grouped.get(row.expected_skill);
    if (!arr) {
      arr = [];
      grouped.set(row.expected_skill, arr);
    }
    arr.push(row.utterance);
  }
  return grouped;
}

export function selectExamples(
  slug: string,
  descriptionPhrases: string[],
  corpusUtter: string[],
): SkillExample[] {
  const intent = SKILL_INTENTS[slug] ?? `axhub ${slug} intent`;
  const koPool = [
    ...corpusUtter.filter(isKorean),
    ...descriptionPhrases.filter(isKorean),
  ];
  const enPool = [
    ...corpusUtter.filter((u) => !isKorean(u)),
    ...descriptionPhrases.filter((p) => !isKorean(p)),
  ];
  const seen = new Set<string>();
  const dedup = (pool: string[], cap: number): string[] => {
    const out: string[] = [];
    for (const u of pool) {
      if (out.length >= cap) break;
      if (!seen.has(u)) {
        out.push(u);
        seen.add(u);
      }
    }
    return out;
  };
  const ko = dedup(koPool, 3);
  const en = dedup(enPool, 3);
  const result: string[] = [...ko.slice(0, 2), ...en.slice(0, 2)];
  if (ko.length > 2) result.push(ko[2]!);
  else if (en.length > 2) result.push(en[2]!);
  return result.slice(0, 5).map((utterance) => ({ utterance, intent }));
}

export function formatExamplesYaml(examples: SkillExample[]): string {
  const lines = ["examples:"];
  for (const ex of examples) {
    const escapedUtter = ex.utterance.replace(/"/g, '\\"');
    const escapedIntent = ex.intent.replace(/"/g, '\\"');
    lines.push(`  - utterance: "${escapedUtter}"`);
    lines.push(`    intent: "${escapedIntent}"`);
  }
  return lines.join("\n");
}

export function mergeExamplesIntoContent(content: string, yaml: string): string {
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch || fmMatch[1] === undefined) return content;
  let fm = fmMatch[1];

  const existingBlock = fm.match(/(^|\n)examples:\s*\n((?:\s+- utterance: .*\n\s+intent: .*\n)+)/);
  if (existingBlock) {
    fm = fm.replace(existingBlock[0], (existingBlock[1] ?? "") + yaml + "\n");
  } else if (/^multi-step:/m.test(fm)) {
    fm = fm.replace(/^(multi-step:)/m, `${yaml}\n$1`);
  } else {
    fm = fm.trimEnd() + "\n" + yaml;
  }

  return content.replace(/^---\n[\s\S]*?\n---/, `---\n${fm}\n---`);
}

function isAlreadyValid(content: string, expected: SkillExample[]): boolean {
  const fmMatch = content.match(/^---\n([\s\S]*?)\n---/);
  if (!fmMatch || fmMatch[1] === undefined) return false;
  const existing = parseExamples(fmMatch[1]);
  if (existing.length < 5) return false;
  const koCount = existing.filter((e) => isKorean(e.utterance)).length;
  const enCount = existing.length - koCount;
  if (koCount < 2 || enCount < 2) return false;
  const allEnIntents = existing.every((e) => !isKorean(e.intent));
  if (!allEnIntents) return false;
  // utterance set 가 expected 와 겹치면 idempotent skip OK
  const expectedSet = new Set(expected.map((e) => e.utterance));
  const overlap = existing.filter((e) => expectedSet.has(e.utterance)).length;
  return overlap >= 3;
}

interface CodegenResult {
  slug: string;
  status: "applied" | "skipped" | "insufficient";
  reason?: string;
}

export function runCodegen(opts: {
  apply: boolean;
  skillsDir?: string;
  corpus?: Map<string, string[]>;
}): CodegenResult[] {
  const skillsDir = opts.skillsDir ?? SKILLS_DIR;
  const corpus = opts.corpus ?? loadCorpusUtterances(CORPUS_PATH);
  const skillSlugs = readdirSync(skillsDir).filter((d) => {
    try {
      readFileSync(join(skillsDir, d, "SKILL.md"), "utf8");
      return true;
    } catch {
      return false;
    }
  });

  const results: CodegenResult[] = [];
  for (const slug of skillSlugs) {
    const path = join(skillsDir, slug, "SKILL.md");
    const region = readSkillDescription(path);
    if (!region) {
      results.push({ slug, status: "skipped", reason: "no description region" });
      continue;
    }
    const corpusUtter = corpus.get(slug) ?? [];
    const examples = selectExamples(slug, region.existingPhrases, corpusUtter);
    if (examples.length < 5) {
      results.push({ slug, status: "insufficient", reason: `only ${examples.length} examples` });
      continue;
    }
    const content = readFileSync(path, "utf8");
    if (isAlreadyValid(content, examples)) {
      results.push({ slug, status: "skipped", reason: "already valid" });
      continue;
    }
    const yaml = formatExamplesYaml(examples);
    const updated = mergeExamplesIntoContent(content, yaml);
    if (updated === content) {
      results.push({ slug, status: "skipped", reason: "no change" });
      continue;
    }
    if (opts.apply) {
      writeFileSync(path, updated);
      results.push({ slug, status: "applied" });
    } else {
      results.push({ slug, status: "skipped", reason: "dry-run" });
    }
  }
  return results;
}

if (import.meta.main) {
  const apply = process.argv.includes("--apply");
  const results = runCodegen({ apply });
  for (const r of results) {
    const tag = r.status === "applied" ? "[apply]" : r.status === "insufficient" ? "[fail]" : "[skip]";
    process.stdout.write(`${tag} ${r.slug}: ${r.reason ?? r.status}\n`);
  }
  const insufficient = results.filter((r) => r.status === "insufficient");
  if (insufficient.length > 0) {
    process.stderr.write(`${insufficient.length} SKILL 의 examples < 5 — corpus 또는 description 보강 필요\n`);
    process.exit(1);
  }
  if (!apply) {
    process.stderr.write(`[dry-run] use --apply to write changes\n`);
  }
}
