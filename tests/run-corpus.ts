#!/usr/bin/env bun
/**
 * Cross-platform corpus runner for axhub plugin evaluation.
 *
 * This is the canonical implementation for fixture replay and scoring. The
 * legacy tests/run-corpus.sh file remains as a POSIX compatibility wrapper, but
 * package scripts and CI use this Bun entrypoint so Windows does not require
 * Git Bash.
 */
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";

interface Options {
  mode: "docs-only" | "plugin";
  outFile: string;
  corpus: string;
  fixture: string;
  vsFile: string;
  score: boolean;
  help: boolean;
}

const PLUGIN_ROOT = process.env.PLUGIN_ROOT ?? join(import.meta.dir, "..");
const DEFAULT_MODEL = "claude-sonnet-4-6";

const usage = (): string => [
  "tests/run-corpus.ts — corpus runner for axhub plugin evaluation.",
  "",
  "Usage:",
  "  bun tests/run-corpus.ts [--mode docs-only|plugin] [--corpus <file>] [--out <file>] [--fixture <file>] [--vs <baseline-file-or-name>] [--score]",
  "",
  "The runner replays committed fixtures for corpus.20/corpus.100 and optionally scores them.",
  "The full corpus is advisory-only unless an explicit --fixture is provided.",
].join("\n");

const stderr = (line: string): void => {
  process.stderr.write(`${line}\n`);
};

const parseArgs = (argv: string[]): Options => {
  const opts: Options = {
    mode: "docs-only",
    outFile: "",
    corpus: "",
    fixture: "",
    vsFile: "",
    score: false,
    help: false,
  };

  for (let i = 0; i < argv.length;) {
    const arg = argv[i];
    switch (arg) {
      case "--mode": {
        const value = argv[i + 1] ?? "";
        if (value !== "docs-only" && value !== "plugin") {
          throw new Error(`unknown mode '${value}'. Use docs-only or plugin.`);
        }
        opts.mode = value;
        i += 2;
        break;
      }
      case "--corpus":
        opts.corpus = argv[i + 1] ?? "";
        i += 2;
        break;
      case "--out":
        opts.outFile = argv[i + 1] ?? "";
        i += 2;
        break;
      case "--fixture":
        opts.fixture = argv[i + 1] ?? "";
        i += 2;
        break;
      case "--vs":
        opts.vsFile = argv[i + 1] ?? "";
        i += 2;
        break;
      case "--score":
        opts.score = true;
        i += 1;
        break;
      case "--help":
      case "-h":
        opts.help = true;
        i += 1;
        break;
      default:
        throw new Error(`Unknown argument: ${arg}`);
    }
  }

  opts.corpus ||= join(PLUGIN_ROOT, "tests/corpus.jsonl");
  return opts;
};

const lineCount = (path: string): number => {
  const content = readFileSync(path, "utf8");
  if (content.length === 0) return 0;
  return content.endsWith("\n") ? content.split("\n").length - 1 : content.split("\n").length;
};

const tierSuffixFor = (corpusPath: string): "" | ".20" | ".100" => {
  if (corpusPath.endsWith("corpus.20.jsonl")) return ".20";
  if (corpusPath.endsWith("corpus.100.jsonl")) return ".100";
  return "";
};

const fixtureFor = (mode: Options["mode"], tier: string): string | null => {
  if ((tier === ".20" || tier === ".100") && mode === "docs-only") {
    return join(PLUGIN_ROOT, `tests/baseline-results.docs-only${tier}.json`);
  }
  if ((tier === ".20" || tier === ".100") && mode === "plugin") {
    return join(PLUGIN_ROOT, `tests/baseline-results.claude-native${tier}.json`);
  }
  return null;
};

const baselineFor = (tier: string): string | null => {
  if (tier === ".20" || tier === ".100") return join(PLUGIN_ROOT, `tests/baseline-results.docs-only${tier}.json`);
  return null;
};

const runBun = (script: string, args: string[]): number => {
  const result = spawnSync(process.execPath, [script, ...args], {
    cwd: PLUGIN_ROOT,
    encoding: "utf8",
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  return result.status ?? 1;
};

const main = (): number => {
  let opts: Options;
  try {
    opts = parseArgs(process.argv.slice(2));
  } catch (err) {
    stderr(`ERROR: ${err instanceof Error ? err.message : String(err)}`);
    return 1;
  }

  if (opts.help) {
    process.stdout.write(`${usage()}\n`);
    return 0;
  }

  if (!existsSync(opts.corpus)) {
    stderr(`ERROR: corpus not found at ${opts.corpus}`);
    return 1;
  }

  const corpusRows = lineCount(opts.corpus);
  const tierSuffix = tierSuffixFor(opts.corpus);
  let advisory = false;

  if (!tierSuffix) {
    stderr(`[corpus-runner] ADVISORY ONLY: ${basename(opts.corpus)} (${corpusRows} rows) has no committed reliable baseline.`);
    stderr("[corpus-runner] ADVISORY ONLY: results below are manual/advisory, NOT a CI gate.");
    stderr("[corpus-runner] ADVISORY ONLY: use tests/corpus.20.jsonl or tests/corpus.100.jsonl for CI.");
    advisory = true;
  }

  if (opts.score && advisory) {
    if (!opts.fixture || (opts.vsFile && !opts.vsFile.endsWith(".json"))) {
      stderr(`[ADVISORY] ${corpusRows}-row corpus 의 routing-score 결과는 manual/advisory 입니다 (CI gate X). 호출 안 해요.`);
      return 0;
    }
  }

  let fixture = opts.fixture;
  if (!fixture) {
    fixture = fixtureFor(opts.mode, tierSuffix) ?? "";
    if (!fixture) {
      stderr(`ERROR: no committed ${opts.mode} fixture for ${corpusRows}-row corpus.`);
      stderr("ERROR: pass --corpus tests/corpus.20.jsonl, --corpus tests/corpus.100.jsonl, or --fixture <results.json>.");
      return 2;
    }
  }

  if (!existsSync(fixture)) {
    stderr(`ERROR: fixture not found at ${fixture}`);
    return 1;
  }

  let resultFile = fixture;
  const model = process.env.MODEL ?? DEFAULT_MODEL;
  stderr(`[corpus-runner] mode=${opts.mode} corpus=${opts.corpus} rows=${corpusRows} model=${model}`);
  stderr(`[corpus-runner] fixture=${fixture}`);

  if (opts.outFile) {
    mkdirSync(dirname(opts.outFile), { recursive: true });
    copyFileSync(fixture, opts.outFile);
    resultFile = opts.outFile;
    stderr(`[corpus-runner] wrote replay results to ${opts.outFile}`);
  }

  if (opts.score) {
    if (opts.vsFile && !opts.vsFile.endsWith(".json")) {
      const routingBaseline = join(PLUGIN_ROOT, `tests/baseline-results.docs-only${tierSuffix}.json`);
      const routingAgainst = join(PLUGIN_ROOT, `tests/baseline-results.${opts.vsFile}${tierSuffix}.json`);
      if (!existsSync(routingBaseline)) {
        stderr(`ERROR: routing baseline missing: ${routingBaseline}`);
        return 1;
      }
      if (!existsSync(routingAgainst)) {
        stderr(`ERROR: routing 'against' baseline missing: ${routingAgainst}`);
        return 1;
      }
      stderr(`[corpus-runner] routing-score: ${routingBaseline}  vs  ${routingAgainst}  (threshold 0.95)`);
      return runBun(join(PLUGIN_ROOT, "tests/routing-score.ts"), [
        "--baseline", routingBaseline,
        "--against", routingAgainst,
        "--threshold", "0.95",
      ]);
    }

    let vsFile = opts.vsFile;
    if (!vsFile && opts.mode === "plugin") {
      vsFile = baselineFor(tierSuffix) ?? "";
      if (!vsFile) {
        stderr(`ERROR: no committed docs-only baseline for ${corpusRows}-row corpus; pass --vs <baseline.json>.`);
        return 2;
      }
    }

    if (opts.mode === "plugin") {
      stderr(`[corpus-runner] scoring plugin arm against baseline ${vsFile}`);
      const scoreExit = runBun(join(PLUGIN_ROOT, "tests/score.ts"), [resultFile, "--corpus", opts.corpus, "--vs", vsFile]);
      if (advisory && scoreExit !== 0) {
        stderr(`[corpus-runner] ADVISORY ONLY: score exit ${scoreExit} suppressed (not a CI gate).`);
        return 0;
      }
      return scoreExit;
    }

    stderr("[corpus-runner] scoring docs-only baseline (informational; GO/KILL applies to plugin arm)");
    const scoreExit = runBun(join(PLUGIN_ROOT, "tests/score.ts"), [resultFile, "--corpus", opts.corpus]);
    if (scoreExit !== 0) {
      stderr(`[corpus-runner] docs-only scorer exited ${scoreExit}; treating as baseline signal, not runner failure`);
    }
    return 0;
  }

  stderr("[corpus-runner] replay complete. To score:");
  if (opts.mode === "plugin") {
    const vsDefault = baselineFor(tierSuffix);
    if (vsDefault) {
      stderr(`[corpus-runner]   bun tests/score.ts ${resultFile} --corpus ${opts.corpus} --vs ${vsDefault}`);
    } else {
      stderr(`[corpus-runner]   bun tests/score.ts ${resultFile} --corpus ${opts.corpus} --vs <baseline.json>`);
    }
  } else {
    stderr(`[corpus-runner]   bun tests/score.ts ${resultFile} --corpus ${opts.corpus}`);
  }

  return 0;
};

if (import.meta.main) {
  process.exit(main());
}
