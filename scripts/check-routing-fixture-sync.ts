#!/usr/bin/env bun

import { spawnSync } from "node:child_process";
import type { SpawnSyncReturns } from "node:child_process";
import { readFileSync } from "node:fs";

const routingAffectingPatterns = [
  /^skills\/[^/]+\/SKILL\.md$/,
  /^commands\/.*\.md$/,
  /^tests\/corpus(?:\.20|\.100)?\.jsonl$/,
  /^scripts\/codegen-skill-examples\.ts$/,
];

const baselinePatterns = [
  /^tests\/baseline-results\.(?:docs-only|claude-native)\.(?:20|100)\.json$/,
];

export interface FixtureSyncReport {
  changedFiles: string[];
  routingAffectingFiles: string[];
  baselineFiles: string[];
  ok: boolean;
}

export interface AnalyzeRoutingFixtureSyncOptions {
  isSkillRoutingMetadataChanged?: (file: string) => boolean;
}

function isSkillFile(file: string): boolean {
  return /^skills\/[^/]+\/SKILL\.md$/.test(file);
}

function isRoutingAffectingFile(
  file: string,
  options: AnalyzeRoutingFixtureSyncOptions = {},
): boolean {
  if (isSkillFile(file) && options.isSkillRoutingMetadataChanged) {
    return options.isSkillRoutingMetadataChanged(file);
  }
  return routingAffectingPatterns.some((p) => p.test(file));
}

export function analyzeRoutingFixtureSync(
  changedFiles: string[],
  options: AnalyzeRoutingFixtureSyncOptions = {},
): FixtureSyncReport {
  const normalized = changedFiles.map((f) => f.trim()).filter(Boolean);
  const routingAffectingFiles = normalized.filter((f) => isRoutingAffectingFile(f, options));
  const baselineFiles = normalized.filter((f) => baselinePatterns.some((p) => p.test(f)));
  return {
    changedFiles: normalized,
    routingAffectingFiles,
    baselineFiles,
    ok: routingAffectingFiles.length === 0 || baselineFiles.length > 0,
  };
}

function git(args: string[]): SpawnSyncReturns<string> {
  return spawnSync("git", args, { encoding: "utf8" });
}

function changedFilesFromGit(baseRef: string, headRef: string): string[] {
  let result = git(["diff", "--name-only", `${baseRef}...${headRef}`]);
  if (result.status !== 0 && (result.stderr ?? "").includes("no merge base")) {
    result = git(["diff", "--name-only", `${baseRef}..${headRef}`]);
  }
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `git diff failed with status ${result.status}`);
  }
  return result.stdout.split("\n");
}

function showFile(ref: string, file: string): string | null {
  const result = git(["show", `${ref}:${file}`]);
  if (result.status !== 0) return null;
  return result.stdout;
}

function frontmatterDescription(content: string): string | null {
  const match = content.match(/^---\n([\s\S]*?)\n---/);
  if (!match) return null;
  const description = match[1]?.match(/^description:\s*(.*)$/m)?.[1];
  return description?.trim() ?? null;
}

function skillRoutingMetadataChanged(file: string, baseRef: string, headRef: string): boolean {
  const before = showFile(baseRef, file);
  const after = showFile(headRef, file);
  if (before === null || after === null) return true;
  return frontmatterDescription(before) !== frontmatterDescription(after);
}

function parseArgs(argv: string[]): { base?: string; head: string } {
  const out: { base?: string; head: string } = { head: "HEAD" };
  for (let i = 0; i < argv.length; i++) {
    if (argv[i] === "--base" && i + 1 < argv.length) {
      out.base = argv[++i];
    } else if (argv[i] === "--head" && i + 1 < argv.length) {
      out.head = argv[++i] ?? out.head;
    } else if (argv[i] === "-h" || argv[i] === "--help") {
      process.stdout.write(`Usage:\n  git diff --name-only <base>...HEAD | bun scripts/check-routing-fixture-sync.ts\n  bun scripts/check-routing-fixture-sync.ts --base origin/main [--head HEAD]\n`);
      process.exit(0);
    } else {
      throw new Error(`Unknown argument: ${argv[i]}`);
    }
  }
  return out;
}

function main(): void {
  const opts = parseArgs(process.argv.slice(2));
  const stdin = readFileSync(0, "utf8");
  const changedFiles = stdin.trim().length > 0 ? stdin.split("\n") : opts.base ? changedFilesFromGit(opts.base, opts.head) : [];
  const report = analyzeRoutingFixtureSync(
    changedFiles,
    opts.base
      ? {
          isSkillRoutingMetadataChanged: (file) =>
            skillRoutingMetadataChanged(file, opts.base!, opts.head),
        }
      : {},
  );

  if (report.ok) {
    process.stderr.write(
      `[routing-fixture-sync] ok: routing-affecting=${report.routingAffectingFiles.length} baseline=${report.baselineFiles.length}\n`,
    );
    return;
  }

  process.stderr.write(
    [
      "[routing-fixture-sync] ERROR: routing-affecting files changed without baseline fixture updates.",
      "",
      "routing-affecting files:",
      ...report.routingAffectingFiles.map((f) => `  - ${f}`),
      "",
      "Required: update at least one committed baseline fixture:",
      "  - tests/baseline-results.docs-only.20.json",
      "  - tests/baseline-results.docs-only.100.json",
      "  - tests/baseline-results.claude-native.20.json",
      "  - tests/baseline-results.claude-native.100.json",
      "",
      "If the drift is intentionally not measured in this PR, use the explicit [skip-routing-gate] PR title override.",
      "",
    ].join("\n"),
  );
  process.exit(1);
}

if (import.meta.main) {
  main();
}
