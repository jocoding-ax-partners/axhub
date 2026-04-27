#!/usr/bin/env bun
/**
 * Skill keyword preservation check.
 *
 * nl-lexicon.md user-utterance phrases AND SKILL.md frontmatter description
 * quoted Korean phrases are activation triggers — Claude's skill router uses
 * them to route natural language requests to the right skill. Phase 13 Toss
 * tone migration MUST NOT touch these phrases (would break activation).
 *
 * This script captures a baseline snapshot of all quoted Korean phrases in
 * SKILL.md descriptions + extracts user-utterance bullet items from
 * nl-lexicon.md. PR2 must show diff = 0 vs baseline.
 *
 * Modes:
 *   --baseline    write baseline file `.omc/lint-baselines/skill-keywords.json`
 *   --check       compare current vs baseline, exit 1 on diff
 *   default       print current snapshot
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { glob } from "node:fs/promises";

const REPO_ROOT = join(import.meta.dir, "..");
const BASELINE_PATH = join(REPO_ROOT, ".omc/lint-baselines/skill-keywords.json");

const extractDescriptionPhrases = async (): Promise<Record<string, string[]>> => {
  const out: Record<string, string[]> = {};
  for await (const f of glob("skills/*/SKILL.md", { cwd: REPO_ROOT })) {
    const content = readFileSync(join(REPO_ROOT, f), "utf8");
    const fmMatch = content.match(/^---\s*\n([\s\S]*?)\n---/);
    if (!fmMatch) continue;
    // description may be on one long line or broken across lines until next frontmatter key
    const desc = fmMatch[1].match(/description:\s*(.+?)(?=\n[a-z_]+:\s|$)/s);
    if (!desc) continue;
    const phrases = desc[1].match(/"([^"]*[가-힣][^"]*)"/g) ?? [];
    if (phrases.length > 0) out[f] = phrases.map((p) => p.replace(/^"|"$/g, ""));
  }
  return out;
};

const extractLexiconPhrases = (): string[] => {
  const lex = join(REPO_ROOT, "skills/deploy/references/nl-lexicon.md");
  if (!existsSync(lex)) return [];
  const content = readFileSync(lex, "utf8");
  const phrases: string[] = [];
  for (const line of content.split("\n")) {
    // Plain bullet item with Korean text — `- 배포해` format
    const m = line.match(/^\s*[-*]\s+(.+?)\s*$/);
    if (m && /[가-힣]/.test(m[1]) && !m[1].startsWith("**") && !m[1].includes(":")) {
      phrases.push(m[1]);
    }
  }
  return phrases;
};

const snapshot = async () => ({
  description_phrases: await extractDescriptionPhrases(),
  lexicon_phrases: extractLexiconPhrases(),
  generated_at: new Date().toISOString(),
});

const main = async (): Promise<number> => {
  const args = process.argv.slice(2);
  const current = await snapshot();
  if (args.includes("--baseline")) {
    mkdirSync(dirname(BASELINE_PATH), { recursive: true });
    writeFileSync(BASELINE_PATH, JSON.stringify(current, null, 2));
    process.stdout.write(`baseline written to ${BASELINE_PATH}\n`);
    process.stdout.write(`description_phrases: ${Object.keys(current.description_phrases).length} files, lexicon_phrases: ${current.lexicon_phrases.length} entries\n`);
    return 0;
  }
  if (args.includes("--check")) {
    if (!existsSync(BASELINE_PATH)) {
      process.stderr.write(`baseline not found at ${BASELINE_PATH} — run with --baseline first\n`);
      return 1;
    }
    const baseline = JSON.parse(readFileSync(BASELINE_PATH, "utf8"));
    const baseFp = JSON.stringify(baseline.description_phrases) + JSON.stringify(baseline.lexicon_phrases);
    const curFp = JSON.stringify(current.description_phrases) + JSON.stringify(current.lexicon_phrases);
    if (baseFp === curFp) {
      process.stdout.write("OK — keywords preserved (no diff vs baseline)\n");
      return 0;
    }
    process.stderr.write("DRIFT DETECTED — skill keywords changed since baseline\n");
    process.stderr.write(`baseline lexicon: ${baseline.lexicon_phrases.length} phrases\n`);
    process.stderr.write(`current lexicon: ${current.lexicon_phrases.length} phrases\n`);
    return 1;
  }
  process.stdout.write(JSON.stringify(current, null, 2) + "\n");
  return 0;
};

if (import.meta.main) {
  main().then((code) => process.exit(code));
}

export { snapshot, extractDescriptionPhrases, extractLexiconPhrases, BASELINE_PATH };
