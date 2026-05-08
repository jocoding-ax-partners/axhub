#!/usr/bin/env bun
/**
 * Phase 1 — Component 5: Keyword Phrase → SKILL.md Description Codegen.
 *
 * Parses crates/axhub-helpers/src/main.rs:detect_prompt_route() to extract
 * the keyword chain (~300 phrases across 19 if-else blocks) and merges any
 * missing phrases into each skills/<skill>/SKILL.md frontmatter description's
 * trigger phrase region (between "다음 표현에서 활성화: " and ", 또는").
 *
 * Idempotent: safe to run multiple times. Re-running produces zero diff once
 * descriptions are converged with main.rs.
 *
 * Modes:
 *   --dry-run     Print diff per skill, no file writes (default for CI)
 *   --apply       Write merged phrases back to SKILL.md files
 *   --json        Emit machine-readable diff to stdout
 *
 * Why: Phase 2 (Component 1) deletes detect_prompt_route() entirely. Before
 * deletion, the keyword chain phrase set must be reflected in SKILL.md so
 * Claude's native skill matching has the same trigger surface.
 */

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

interface KeywordBlock {
  skill: string;
  phrases: string[];
}

interface SkillDescriptionRegion {
  path: string;
  description: string;
  rawDescription: string;
  quote: "'" | '"';
  triggerStart: number;
  triggerEnd: number;
  existingPhrases: string[];
  marker: typeof TRIGGER_MARKERS[number];
}

interface SkillDiff {
  skill: string;
  path: string;
  existing: string[];
  fromMainRs: string[];
  missing: string[];
  willAdd: number;
}

const REPO_ROOT = join(import.meta.dir, "..");
const MAIN_RS = join(REPO_ROOT, "crates/axhub-helpers/src/main.rs");
const SKILLS_DIR = join(REPO_ROOT, "skills");

const TRIGGER_MARKERS = [
  "다음 표현에서 활성화:",
  "다음과 같은 불확실 컨텍스트에서 활성화:",
  "Triggers on:",
] as const;

const UPGRADE_COMPOUND_TRIGGER_PHRASES = [
  "플러그인 새 버전",
  "플러그인 업그레이드",
  "플러그인 업데이트",
  "플러그인 버전",
  "플러그인 호환",
  "지금 플러그인 버전이 뭐야",
  "플러그인이랑 호환되는 버전이야",
  "axhub plugin update",
  "axhub plugin upgrade",
  "axhub plugin version",
  "plugin self-upgrade",
  "plugin update",
  "plugin upgrade",
  "plugin version",
] as const;

function normalizeKeywordBlock(skill: string, phrases: string[]): string[] {
  if (skill !== "upgrade") return phrases;

  const unsafeStandalone = new Set([
    "plugin",
    "플러그인",
    "upgrade",
    "update",
    "version",
    "업데이트",
    "업그레이드",
    "새 버전",
    "버전",
    "호환",
  ]);
  const allFromCompoundGuard = phrases.length > 0 && phrases.every((phrase) => unsafeStandalone.has(phrase));
  if (!allFromCompoundGuard) return phrases;

  // main.rs routes upgrade only when a plugin guard AND an update/version term
  // both match. Flat SKILL descriptions cannot express that conjunction, so emit
  // only plugin-qualified phrases and never the guard terms as standalone triggers.
  return [...UPGRADE_COMPOUND_TRIGGER_PHRASES];
}

function parseMainRs(): KeywordBlock[] {
  const text = readFileSync(MAIN_RS, "utf8");

  // detect_prompt_route() 함수 본문만 (line 510~) 추출
  const startIdx = text.indexOf("fn detect_prompt_route(");
  if (startIdx === -1) {
    throw new Error("detect_prompt_route() not found in main.rs");
  }
  // 함수 끝 (다음 fn 선언) 찾기
  const afterStart = text.slice(startIdx);
  const endRel = afterStart.indexOf("\nfn cmd_prompt_route(");
  if (endRel === -1) {
    throw new Error("cmd_prompt_route() boundary not found");
  }
  const fnBody = afterStart.slice(0, endRel);

  const blocks: KeywordBlock[] = [];

  // Pattern A: contains_any(&p, &[...]) { ... return Some(PromptRoute { skill: "X" })
  const containsAnyRe = /contains_any\(\s*&p,\s*&\[([\s\S]*?)\]\s*,?\s*\)\s*\{[\s\S]*?skill:\s*"([^"]+)"/g;
  let match: RegExpExecArray | null;
  while ((match = containsAnyRe.exec(fnBody)) !== null) {
    const arrayBody = match[1];
    const skill = match[2];
    if (!arrayBody || !skill) continue;
    const phrases: string[] = [];
    const phraseRe = /"((?:[^"\\]|\\.)*)"/g;
    let pm: RegExpExecArray | null;
    while ((pm = phraseRe.exec(arrayBody)) !== null) {
      const raw = pm[1];
      if (!raw) continue;
      // 이스케이프 해제
      const decoded = raw.replace(/\\"/g, '"').replace(/\\\\/g, "\\");
      phrases.push(decoded);
    }
    blocks.push({ skill, phrases: normalizeKeywordBlock(skill, phrases) });
  }

  // Pattern B: if p == "X" { return Some(PromptRoute { skill: "Y" })
  const eqRe = /if p == "([^"]+)"\s*\{[\s\S]*?skill:\s*"([^"]+)"/g;
  while ((match = eqRe.exec(fnBody)) !== null) {
    const phrase = match[1];
    const skill = match[2];
    if (!phrase || !skill) continue;
    blocks.push({ skill, phrases: normalizeKeywordBlock(skill, [phrase]) });
  }

  return blocks;
}

function aggregatePerSkill(blocks: KeywordBlock[]): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  for (const block of blocks) {
    let set = map.get(block.skill);
    if (!set) {
      set = new Set();
      map.set(block.skill, set);
    }
    for (const phrase of block.phrases) {
      set.add(phrase);
    }
  }
  return map;
}

function readSkillDescription(path: string): SkillDescriptionRegion | null {
  let raw: string;
  try {
    raw = readFileSync(path, "utf8");
  } catch {
    return null;
  }

  // frontmatter 안 description 라인 추출 (single-line description, quoted prose).
  // YAML single-quoted scalars escape apostrophes as doubled single quotes.
  const single = raw.match(/^description:\s*'((?:[^']|'')*)'$/m);
  const double = single ? null : raw.match(/^description:\s*"((?:[^"\\]|\\.)*)"$/m);
  if (!single && !double) return null;

  const quote = single ? "'" : '"';
  const rawDescription = (single?.[1] ?? double?.[1]) ?? "";
  const description = single
    ? rawDescription.replace(/''/g, "'")
    : JSON.parse(`"${rawDescription}"`);
  if (!description) return null;

  let marker: typeof TRIGGER_MARKERS[number] | null = null;
  let markerIdx = -1;
  for (const m of TRIGGER_MARKERS) {
    const idx = description.indexOf(m);
    if (idx !== -1) {
      marker = m;
      markerIdx = idx;
      break;
    }
  }
  if (!marker) return null;

  // trigger 영역 끝: "또는" 첫 번째 등장 (marker 이후)
  const after = description.slice(markerIdx + marker.length);
  const orIdx = after.indexOf(", 또는");
  const altOrIdx = after.indexOf("또는");
  let endRel = orIdx >= 0 ? orIdx : altOrIdx;
  if (endRel < 0) {
    // marker 후 첫 마침표 또는 줄 끝
    const dot = after.indexOf(". ");
    endRel = dot >= 0 ? dot : after.length;
  }

  const triggerStart = markerIdx + marker.length;
  const triggerEnd = triggerStart + endRel;
  const triggerRegion = description.slice(triggerStart, triggerEnd);

  // 기존 phrase 추출 (quoted)
  const existingPhrases: string[] = [];
  const phraseRe = /"([^"]+)"/g;
  let m: RegExpExecArray | null;
  while ((m = phraseRe.exec(triggerRegion)) !== null) {
    if (m[1]) existingPhrases.push(m[1]);
  }

  return {
    path,
    description,
    rawDescription,
    quote,
    triggerStart,
    triggerEnd,
    existingPhrases,
    marker,
  };
}

function computeDiff(skill: string, region: SkillDescriptionRegion, mainRsPhrases: Set<string>): SkillDiff {
  const existing = new Set(region.existingPhrases);
  const fromMainRs = new Set(mainRsPhrases);
  const missing: string[] = [];
  for (const phrase of fromMainRs) {
    if (!existing.has(phrase)) missing.push(phrase);
  }
  return {
    skill,
    path: region.path,
    existing: [...existing].sort(),
    fromMainRs: [...fromMainRs].sort(),
    missing: missing.sort(),
    willAdd: missing.length,
  };
}

function formatPhraseList(phrases: string[]): string {
  // 한국어 → 영어 + alphabetical within each group
  const collator = new Intl.Collator("ko");
  const isKorean = (s: string): boolean => /[가-힯]/.test(s);
  const korean = phrases.filter(isKorean).sort(collator.compare);
  const english = phrases.filter((p) => !isKorean(p)).sort();
  const all = [...korean, ...english];
  return all.map((p) => `"${p}"`).join(", ");
}

function applyMerge(
  region: SkillDescriptionRegion,
  mainRsPhrases: Set<string>,
): { newDescription: string; changed: boolean; finalPhrases: string[] } {
  const merged = new Set<string>([...region.existingPhrases, ...mainRsPhrases]);
  const sorted = [...merged].sort();
  const existingSorted = [...new Set(region.existingPhrases)].sort();
  if (
    sorted.length === existingSorted.length &&
    sorted.every((p, i) => p === existingSorted[i])
  ) {
    return { newDescription: region.description, changed: false, finalPhrases: existingSorted };
  }

  const formattedList = formatPhraseList([...merged]);
  const before = region.description.slice(0, region.triggerStart);
  const after = region.description.slice(region.triggerEnd);
  const newDescription = `${before} ${formattedList}${after}`;

  return {
    newDescription,
    changed: true,
    finalPhrases: [...merged],
  };
}

function serializeDescription(description: string, quote: "'" | '"'): string {
  if (quote === "'") return description.replace(/'/g, "''");
  return description.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

function writeSkillFile(region: SkillDescriptionRegion, newDescription: string): void {
  const raw = readFileSync(region.path, "utf8");
  const oldLine = `description: ${region.quote}${region.rawDescription}${region.quote}`;
  const newLine = `description: ${region.quote}${serializeDescription(newDescription, region.quote)}${region.quote}`;
  const updated = raw.replace(oldLine, newLine);
  if (updated === raw) {
    throw new Error(`description replace failed for ${region.path}`);
  }
  writeFileSync(region.path, updated);
}

function main(): void {
  const argv = process.argv.slice(2);
  const mode = argv.includes("--apply") ? "apply" : "dry-run";
  const json = argv.includes("--json");
  const verbose = argv.includes("--verbose");

  const blocks = parseMainRs();
  const perSkill = aggregatePerSkill(blocks);

  if (verbose) {
    process.stderr.write(`[codegen-skill-keywords] parsed ${blocks.length} keyword blocks across ${perSkill.size} skills from main.rs\n`);
  }

  const diffs: SkillDiff[] = [];
  let appliedCount = 0;

  for (const [skill, mainRsPhrases] of perSkill) {
    const path = join(SKILLS_DIR, skill, "SKILL.md");
    const region = readSkillDescription(path);
    if (!region) {
      diffs.push({
        skill,
        path,
        existing: [],
        fromMainRs: [...mainRsPhrases].sort(),
        missing: [...mainRsPhrases].sort(),
        willAdd: -1, // sentinel: SKILL.md or marker missing
      });
      continue;
    }
    const diff = computeDiff(skill, region, mainRsPhrases);
    diffs.push(diff);

    if (mode === "apply" && diff.missing.length > 0) {
      const merge = applyMerge(region, mainRsPhrases);
      if (merge.changed) {
        writeSkillFile(region, merge.newDescription);
        appliedCount += 1;
      }
    }
  }

  if (json) {
    process.stdout.write(JSON.stringify({ mode, diffs, appliedCount }, null, 2) + "\n");
    return;
  }

  // Korean human-readable output
  const lines: string[] = [];
  lines.push(`[codegen-skill-keywords] mode=${mode}`);
  lines.push(`  parsed ${blocks.length} keyword blocks → ${perSkill.size} unique skills`);
  lines.push(`  ${diffs.filter((d) => d.willAdd > 0).length} skills with missing phrases (will add ${diffs.reduce((s, d) => s + Math.max(0, d.willAdd), 0)} total)`);
  lines.push(`  ${diffs.filter((d) => d.willAdd === -1).length} skills with missing SKILL.md or trigger marker`);
  lines.push("");
  for (const d of diffs) {
    if (d.willAdd === -1) {
      lines.push(`  ${d.skill}: SKILL.md or marker missing (${d.path})`);
      continue;
    }
    if (d.willAdd === 0) {
      lines.push(`  ${d.skill}: already converged (${d.existing.length} phrases)`);
      continue;
    }
    lines.push(`  ${d.skill}: +${d.willAdd} missing phrase(s):`);
    for (const m of d.missing) {
      lines.push(`    + "${m}"`);
    }
  }
  if (mode === "apply") {
    lines.push("");
    lines.push(`[codegen-skill-keywords] applied to ${appliedCount} SKILL.md file(s)`);
  } else {
    lines.push("");
    lines.push(`[codegen-skill-keywords] dry-run only. Use --apply to write changes.`);
  }
  process.stdout.write(lines.join("\n") + "\n");
}

if (import.meta.main) {
  main();
}

export { parseMainRs, aggregatePerSkill, readSkillDescription, computeDiff, applyMerge, formatPhraseList };
