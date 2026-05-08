#!/usr/bin/env bun
/**
 * Phase 8 AC wrapper — routing drift gate entry point.
 *
 * The GitHub workflow calls tests/run-corpus.sh directly for speed. This script
 * preserves the named plan artifact and gives maintainers the same gate through
 * a Bun entry point.
 */

import { spawnSync } from "node:child_process";
import { join } from "node:path";

const root = join(import.meta.dir, "..");
const args = process.argv.slice(2);
const corpus = args.includes("--corpus") ? args[args.indexOf("--corpus") + 1] : "tests/corpus.100.jsonl";
const result = spawnSync(
  "bash",
  ["tests/run-corpus.sh", "--mode", "plugin", "--corpus", corpus ?? "tests/corpus.100.jsonl", "--vs", "claude-native", "--score"],
  { cwd: root, stdio: "inherit" },
);

process.exit(result.status ?? 1);
