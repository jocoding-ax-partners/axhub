#!/usr/bin/env bun

import { spawnSync } from "node:child_process";
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

export function analyzeRoutingFixtureSync(changedFiles: string[]): FixtureSyncReport {
  const normalized = changedFiles.map((f) => f.trim()).filter(Boolean);
  const routingAffectingFiles = normalized.filter((f) => routingAffectingPatterns.some((p) => p.test(f)));
  const baselineFiles = normalized.filter((f) => baselinePatterns.some((p) => p.test(f)));
  return {
    changedFiles: normalized,
    routingAffectingFiles,
    baselineFiles,
    ok: routingAffectingFiles.length === 0 || baselineFiles.length > 0,
  };
}

function changedFilesFromGit(baseRef: string, headRef: string): string[] {
  const result = spawnSync("git", ["diff", "--name-only", `${baseRef}...${headRef}`], {
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(result.stderr.trim() || `git diff failed with status ${result.status}`);
  }
  return result.stdout.split("\n");
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
  const report = analyzeRoutingFixtureSync(changedFiles);

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
