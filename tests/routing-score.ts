#!/usr/bin/env bun
/**
 * Phase 0 — Sub-task 0.1: Routing-specific scorer (Approach E).
 *
 * Compares two baseline result JSONs by routing decision (expected_skill vs fired_skill).
 * Distinct from tests/score.ts which targets command completion / recovery / consent.
 *
 * Inputs:
 *   --baseline <file>    docs-only baseline JSONL/JSON (Claude native matching simulator)
 *   --against <file>     claude-native baseline JSONL/JSON (Approach E plugin arm)
 *   --corpus <file>      expected_skill source of truth (default: tests/corpus.jsonl auto-derived)
 *   --threshold <0-1>    accuracy gate (default: 0.95)
 *   --json               machine-readable output
 *   --help               this message
 *
 * Schema:
 *   - corpus row:    { id, utterance, intent, expected_skill, expected_cmd_pattern, destructive, lang }
 *   - result row:    { utterance_id, fired_skill, actual_tool_calls, required_consent_seen, ts }
 *
 * Metrics:
 *   - per-skill precision / recall
 *   - null-rejection rate (negative + meta_question rows)
 *   - overall accuracy = (matched expected_skill) / total
 *   - drift between baseline and against (row-level fired_skill mismatch)
 *
 * Exit:
 *   - 0 if against.accuracy >= threshold AND drift <= 5%
 *   - 1 otherwise
 */

import { z } from "zod";
import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const CorpusRowSchema = z.object({
  id: z.string(),
  utterance: z.string(),
  intent: z.string(),
  expected_skill: z.string().nullable(),
  expected_cmd_pattern: z.string().nullable(),
  destructive: z.boolean(),
  lang: z.string(),
});

type CorpusRow = z.infer<typeof CorpusRowSchema>;

// ResultRow only depends on utterance_id + fired_skill for routing-score purposes;
// pass through other fields (actual_tool_calls, required_consent_seen, ts) without
// requiring callers (e.g. unit tests) to populate them.
const ResultRowSchema = z
  .object({
    utterance_id: z.string(),
    fired_skill: z.string().nullable(),
  })
  .passthrough();

type ResultRow = z.infer<typeof ResultRowSchema>;

interface ScoreSummary {
  total: number;
  matched: number;
  null_rejected: number;     // expected null, fired null
  null_missed: number;       // expected null, fired non-null (false positive route)
  positive_matched: number;  // expected non-null, fired matches
  positive_wrong: number;    // expected non-null, fired non-null but wrong skill
  positive_missed: number;   // expected non-null, fired null
  missing_results: number;   // corpus rows with no corresponding result row
  extra_results: number;     // result rows whose utterance_id is not in the corpus
  per_skill: Record<string, { precision: number; recall: number; tp: number; fp: number; fn: number }>;
  overall_accuracy: number;
  null_rejection_rate: number;
}

interface DriftSummary {
  total_rows: number;
  matching_rows: number;
  mismatching_rows: number;
  mismatch_rate: number;
  mismatches: Array<{ id: string; baseline: string | null; against: string | null }>;
}

const HELP = `routing-score.ts — Approach E routing accuracy scorer

USAGE:
  bun tests/routing-score.ts --baseline <file> --against <file> [OPTIONS]

OPTIONS:
  --baseline <file>    docs-only baseline (Claude native simulator)
  --against <file>     claude-native baseline (Approach E plugin arm)
  --corpus <file>      corpus row source (default: tests/corpus.jsonl auto-derived from baseline filename)
  --threshold <0-1>    overall accuracy gate (default: 0.95)
  --json               JSON output to stdout
  --help               this message

EXIT:
  0 if against.accuracy >= threshold AND drift <= 5%
  1 otherwise

EXAMPLE:
  bun tests/routing-score.ts \\
    --baseline tests/baseline-results.docs-only.100.json \\
    --against tests/baseline-results.claude-native.100.json
`;

function parseArgs(argv: string[]): {
  baseline: string;
  against: string;
  corpus: string | null;
  threshold: number;
  json: boolean;
} {
  let baseline = "";
  let against = "";
  let corpus: string | null = null;
  let threshold = 0.95;
  let json = false;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--help" || a === "-h") {
      process.stdout.write(HELP);
      process.exit(0);
    } else if (a === "--baseline") {
      baseline = argv[++i] ?? "";
    } else if (a === "--against") {
      against = argv[++i] ?? "";
    } else if (a === "--corpus") {
      corpus = argv[++i] ?? "";
    } else if (a === "--threshold") {
      threshold = Number(argv[++i] ?? "0.95");
    } else if (a === "--json") {
      json = true;
    } else {
      process.stderr.write(`unknown flag: ${a}\n`);
      process.exit(2);
    }
  }
  if (!baseline || !against) {
    process.stderr.write(`--baseline and --against are required\n${HELP}`);
    process.exit(2);
  }
  if (!Number.isFinite(threshold) || threshold < 0 || threshold > 1) {
    process.stderr.write(`--threshold must be between 0 and 1\n`);
    process.exit(2);
  }
  return { baseline, against, corpus, threshold, json };
}

function readJsonOrJsonl(path: string): unknown[] {
  if (!existsSync(path)) {
    process.stderr.write(`file not found: ${path}\n`);
    process.exit(2);
  }
  const text = readFileSync(path, "utf8").trim();
  if (text.startsWith("[")) {
    return JSON.parse(text);
  }
  return text
    .split("\n")
    .filter((line) => line.trim().length > 0)
    .map((line) => JSON.parse(line));
}

function parseCorpus(path: string): CorpusRow[] {
  const raw = readJsonOrJsonl(path);
  const out: CorpusRow[] = [];
  const errors: string[] = [];
  for (const [index, row] of raw.entries()) {
    const parsed = CorpusRowSchema.safeParse(row);
    if (parsed.success) {
      out.push(parsed.data);
    } else {
      errors.push(`entry ${index + 1}: ${parsed.error.issues.map((issue) => issue.path.join(".") || "<root>").join(", ")}`);
    }
  }
  if (errors.length > 0) {
    process.stderr.write(`invalid corpus rows in ${path}:\n${errors.join("\n")}\n`);
    process.exit(2);
  }
  return out;
}

function parseResults(path: string): ResultRow[] {
  const raw = readJsonOrJsonl(path);
  const out: ResultRow[] = [];
  const errors: string[] = [];
  for (const [index, row] of raw.entries()) {
    if (
      row !== null &&
      typeof row === "object" &&
      "_metadata" in row &&
      !("utterance_id" in row)
    ) {
      continue;
    }
    const parsed = ResultRowSchema.safeParse(row);
    if (parsed.success) {
      out.push(parsed.data);
    } else {
      errors.push(`entry ${index + 1}: ${parsed.error.issues.map((issue) => issue.path.join(".") || "<root>").join(", ")}`);
    }
  }
  if (errors.length > 0) {
    process.stderr.write(`invalid result rows in ${path}:\n${errors.join("\n")}\n`);
    process.exit(2);
  }
  return out;
}

function deriveCorpusPath(baselinePath: string): string {
  const base = baselinePath.replace(/\.json$/, "").replace(/baseline-results\.[^.]+/, "corpus");
  if (base.endsWith(".20")) return base + ".jsonl";
  if (base.endsWith(".100")) return base + ".jsonl";
  return base + ".jsonl";
}

function score(corpus: CorpusRow[], results: ResultRow[]): ScoreSummary {
  const corpusIds = new Set(corpus.map((r) => r.id));
  const byResultId: Map<string, ResultRow> = new Map(results.map((r) => [r.utterance_id, r]));
  const extraResults = results.filter((r) => !corpusIds.has(r.utterance_id)).length;
  const perSkill: Record<string, { tp: number; fp: number; fn: number }> = {};
  let matched = 0;
  let nullRejected = 0;
  let nullMissed = 0;
  let positiveMatched = 0;
  let positiveWrong = 0;
  let positiveMissed = 0;
  let missingResults = 0;

  for (const c of corpus) {
    const r = byResultId.get(c.id);
    if (!r) {
      missingResults++;
      if (c.expected_skill !== null) {
        positiveMissed++;
        const slot = (perSkill[c.expected_skill] ??= { tp: 0, fp: 0, fn: 0 });
        slot.fn++;
      }
      continue;
    }
    const expected = c.expected_skill;
    const fired = r.fired_skill;
    if (expected === null) {
      if (fired === null) {
        nullRejected++;
        matched++;
      } else {
        nullMissed++;
        const slot = (perSkill[fired] ??= { tp: 0, fp: 0, fn: 0 });
        slot.fp++;
      }
    } else {
      if (fired === expected) {
        positiveMatched++;
        matched++;
        const slot = (perSkill[expected] ??= { tp: 0, fp: 0, fn: 0 });
        slot.tp++;
      } else if (fired === null) {
        positiveMissed++;
        const slot = (perSkill[expected] ??= { tp: 0, fp: 0, fn: 0 });
        slot.fn++;
      } else {
        positiveWrong++;
        const expSlot = (perSkill[expected] ??= { tp: 0, fp: 0, fn: 0 });
        expSlot.fn++;
        const firedSlot = (perSkill[fired] ??= { tp: 0, fp: 0, fn: 0 });
        firedSlot.fp++;
      }
    }
  }

  const total = corpus.length;
  const perSkillFinal: ScoreSummary["per_skill"] = {};
  for (const [skill, s] of Object.entries(perSkill)) {
    const precision = s.tp + s.fp === 0 ? 0 : s.tp / (s.tp + s.fp);
    const recall = s.tp + s.fn === 0 ? 0 : s.tp / (s.tp + s.fn);
    perSkillFinal[skill] = { precision, recall, tp: s.tp, fp: s.fp, fn: s.fn };
  }

  const negativeTotal = nullRejected + nullMissed;
  return {
    total,
    matched,
    null_rejected: nullRejected,
    null_missed: nullMissed,
    positive_matched: positiveMatched,
    positive_wrong: positiveWrong,
    positive_missed: positiveMissed,
    missing_results: missingResults,
    extra_results: extraResults,
    per_skill: perSkillFinal,
    overall_accuracy: total === 0 ? 0 : matched / total,
    null_rejection_rate: negativeTotal === 0 ? 1 : nullRejected / negativeTotal,
  };
}

function drift(baseline: ResultRow[], against: ResultRow[]): DriftSummary {
  const byIdBase: Map<string, ResultRow> = new Map(baseline.map((r) => [r.utterance_id, r]));
  let matching = 0;
  let mismatching = 0;
  const mismatches: DriftSummary["mismatches"] = [];
  for (const r of against) {
    const b = byIdBase.get(r.utterance_id);
    if (!b) continue;
    if (b.fired_skill === r.fired_skill) {
      matching++;
    } else {
      mismatching++;
      if (mismatches.length < 50) {
        mismatches.push({ id: r.utterance_id, baseline: b.fired_skill, against: r.fired_skill });
      }
    }
  }
  const total = matching + mismatching;
  return {
    total_rows: total,
    matching_rows: matching,
    mismatching_rows: mismatching,
    mismatch_rate: total === 0 ? 0 : mismatching / total,
    mismatches,
  };
}

function formatHuman(args: ReturnType<typeof parseArgs>, baselineScore: ScoreSummary, againstScore: ScoreSummary, driftSummary: DriftSummary, ok: boolean): string {
  const lines: string[] = [];
  lines.push("[routing-score] Approach E routing accuracy");
  lines.push(`  baseline (docs-only):  ${args.baseline}`);
  lines.push(`  against  (claude-native): ${args.against}`);
  lines.push("");
  lines.push("== baseline ==");
  lines.push(`  total: ${baselineScore.total}`);
  lines.push(`  accuracy: ${(baselineScore.overall_accuracy * 100).toFixed(2)}%`);
  lines.push(`  null-rejection: ${(baselineScore.null_rejection_rate * 100).toFixed(2)}%`);
  lines.push(`  missing/extra rows: ${baselineScore.missing_results}/${baselineScore.extra_results}`);
  lines.push("");
  lines.push("== against ==");
  lines.push(`  total: ${againstScore.total}`);
  lines.push(`  accuracy: ${(againstScore.overall_accuracy * 100).toFixed(2)}%`);
  lines.push(`  null-rejection: ${(againstScore.null_rejection_rate * 100).toFixed(2)}%`);
  lines.push(`  missing/extra rows: ${againstScore.missing_results}/${againstScore.extra_results}`);
  lines.push("");
  lines.push("== drift (baseline vs against) ==");
  lines.push(`  rows: ${driftSummary.total_rows}`);
  lines.push(`  mismatch: ${driftSummary.mismatching_rows} (${(driftSummary.mismatch_rate * 100).toFixed(2)}%)`);
  if (driftSummary.mismatches.length > 0 && driftSummary.mismatches.length <= 10) {
    lines.push("  examples:");
    for (const m of driftSummary.mismatches.slice(0, 10)) {
      lines.push(`    ${m.id}: ${m.baseline} → ${m.against}`);
    }
  }
  lines.push("");
  lines.push(`threshold: accuracy >= ${(args.threshold * 100).toFixed(2)}% AND drift <= 5%`);
  lines.push(`verdict: ${ok ? "PASS" : "FAIL"}`);
  return lines.join("\n");
}

function main(): void {
  const args = parseArgs(process.argv.slice(2));
  const corpusPath = resolve(args.corpus ?? deriveCorpusPath(args.baseline));
  if (!existsSync(corpusPath)) {
    process.stderr.write(`corpus not found: ${corpusPath} (use --corpus to override)\n`);
    process.exit(2);
  }
  const corpus = parseCorpus(corpusPath);
  const baseline = parseResults(args.baseline);
  const against = parseResults(args.against);
  const baselineScore = score(corpus, baseline);
  const againstScore = score(corpus, against);
  const driftSummary = drift(baseline, against);
  const completeFixtures =
    baselineScore.missing_results === 0 &&
    baselineScore.extra_results === 0 &&
    againstScore.missing_results === 0 &&
    againstScore.extra_results === 0;
  const ok =
    againstScore.overall_accuracy >= args.threshold &&
    driftSummary.mismatch_rate <= 0.05 &&
    completeFixtures;

  if (args.json) {
    const payload = {
      threshold: args.threshold,
      baseline: baselineScore,
      against: againstScore,
      drift: driftSummary,
      verdict: ok ? "PASS" : "FAIL",
    };
    process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
  } else {
    process.stdout.write(formatHuman(args, baselineScore, againstScore, driftSummary, ok) + "\n");
  }

  process.exit(ok ? 0 : 1);
}

if (import.meta.main) {
  main();
}

export { parseArgs, score, drift, parseCorpus, parseResults };
export type { CorpusRow, ResultRow, ScoreSummary, DriftSummary };
