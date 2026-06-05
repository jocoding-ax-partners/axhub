#!/usr/bin/env bun
/**
 * Phase 8 AC wrapper — routing drift gate entry point.
 *
 * Uses the canonical Bun corpus runner so the gate works on Windows without
 * Git Bash while preserving the named plan artifact for maintainers.
 */

import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const args = process.argv.slice(2);
const corpus = args.includes("--corpus") ? args[args.indexOf("--corpus") + 1] : "tests/corpus.100.jsonl";
const result = spawnSync(
  process.execPath,
  ["tests/run-corpus.ts", "--mode", "plugin", "--corpus", corpus ?? "tests/corpus.100.jsonl", "--vs", "claude-native", "--score"],
  { cwd: root, stdio: "inherit" },
);

process.exit(result.status ?? 1);
