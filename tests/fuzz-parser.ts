#!/usr/bin/env bun
/**
 * tests/fuzz-parser.ts — parser-drift fuzzer for parseAxhubCommand.
 *
 * Phase 2 US-103: stress-test the destructive-command parser with 1000+
 * randomized variants of known-destructive base commands. Goal is regression
 * protection: if a future refactor narrows detection, the fuzzer trips.
 *
 * Each variant wraps a base destructive command with one wrapper from each of
 * three buckets (env-prefix, structural, whitespace). Wrappers are designed to
 * stay within parser-detectable space — sub-shell wrappers leave a trailing
 * argument so close-delimiters never contaminate the action token, and
 * eval/bash -c bodies are NOT also wrapped in parens/sub-shells (which would
 * require recursive shell tokenization the parser intentionally does not do).
 * Adversarial gotchas (Unicode whitespace as token separators, multi-level
 * eval-of-eval) are explicitly out-of-scope per the task spec.
 *
 * Reproducibility: deterministic PRNG (mulberry32) seeded from CLI arg.
 *   bun tests/fuzz-parser.ts            # default seed (FNV-1a of "0xAxHub42")
 *   bun tests/fuzz-parser.ts 0xdeadbeef # custom seed
 *   bun tests/fuzz-parser.ts 12345      # decimal seed
 *
 * Exit code 0 if zero bypasses, 1 if any bypass discovered (with details).
 */
import { parseAxhubCommand } from "../src/axhub-helpers/consent.ts";

// ---------------------------------------------------------------------------
// Deterministic PRNG — mulberry32 (32-bit, 2^32 period, no deps).
// ---------------------------------------------------------------------------
const mulberry32 = (seed: number): (() => number) => {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
};

// FNV-1a 32-bit string hash. Used to fold non-hex seed args (like the default
// "0xAxHub42", which isn't valid hex because x/H/u aren't hex digits) into a
// reproducible 32-bit value.
const fnv1a = (s: string): number => {
  let h = 2166136261 >>> 0;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 16777619) >>> 0;
  }
  return h >>> 0;
};

const DEFAULT_SEED = fnv1a("0xAxHub42");

const parseSeed = (raw: string | undefined): number => {
  if (raw === undefined || raw.length === 0) return DEFAULT_SEED;
  if (/^0x[0-9a-f]+$/i.test(raw)) return parseInt(raw, 16) >>> 0;
  if (/^\d+$/.test(raw)) return parseInt(raw, 10) >>> 0;
  return fnv1a(raw);
};

// ---------------------------------------------------------------------------
// Base destructive commands the fuzzer wraps.
//
// Each base ends with a trailing argument so that wrappers which append a
// close-delimiter (`)`, backtick) contaminate only the tail token, never the
// subcommand tokens the parser matches against (`deploy`/`create`,
// `update`/`apply`, `auth`/`login`).
// ---------------------------------------------------------------------------
const BASE_COMMANDS: ReadonlyArray<string> = [
  "axhub deploy create --app paydrop --branch main --commit abc123",
  "axhub update apply --yes",
  // `auth login` has no flags in v0.1.0; pad with --json (a benign flag the CLI
  // accepts everywhere) so the action token isn't the last token.
  "axhub auth login --json",
];

// ---------------------------------------------------------------------------
// Random helpers driven by the seeded PRNG.
// ---------------------------------------------------------------------------
type Rand = () => number;
const pick = <T>(rng: Rand, arr: ReadonlyArray<T>): T => {
  const idx = Math.min(Math.floor(rng() * arr.length), arr.length - 1);
  return arr[idx] as T;
};
const intRange = (rng: Rand, min: number, maxExclusive: number): number =>
  min + Math.floor(rng() * (maxExclusive - min));

// ---------------------------------------------------------------------------
// Bucket 1: env-prefix wrappers (1-3 assignments, then space, then inner).
// ---------------------------------------------------------------------------
const ENV_NAMES: ReadonlyArray<string> = [
  "AXHUB_TOKEN",
  "AXHUB_PROFILE",
  "FOO",
  "BAR",
  "DEBUG",
  "NODE_ENV",
  "_LEAD",
  "X1",
];
const ENV_VALUES: ReadonlyArray<string> = [
  "1",
  "0",
  "true",
  "abc",
  "v_1",
  "deadbeef",
];

const envPrefix = (rng: Rand, inner: string): string => {
  if (rng() < 0.4) return inner; // ~40% of variants have no env prefix
  const n = intRange(rng, 1, 4);
  const parts: string[] = [];
  for (let i = 0; i < n; i++) {
    parts.push(`${pick(rng, ENV_NAMES)}=${pick(rng, ENV_VALUES)}`);
  }
  return parts.join(" ") + " " + inner;
};

// ---------------------------------------------------------------------------
// Bucket 2: structural wrappers (exactly ONE applied per variant).
//
// Each wrapper produces a string the parser is documented to detect via
// `collectCommandPositions` + `tokensIfAxhubCommand`. Wrappers are NOT
// composed with each other — composing eval/bash -c with parens/sub-shells
// would require recursive shell tokenization the parser deliberately omits
// (gotcha-class, out of scope per the task spec).
// ---------------------------------------------------------------------------

const STRUCT_NONE = (_rng: Rand, inner: string): string => inner;
const STRUCT_DOLLAR_PAREN = (_rng: Rand, inner: string): string => `$(${inner})`;
const STRUCT_BACKTICKS = (_rng: Rand, inner: string): string => `\`${inner}\``;
const STRUCT_PARENS = (_rng: Rand, inner: string): string => `(${inner})`;

const PRE_COMMANDS: ReadonlyArray<string> = [
  "cd /tmp",
  "ls -la",
  "true",
  "echo ready",
  "pwd",
  "date +%s",
  "umask 022",
];
const STRUCT_AMP_CHAIN = (rng: Rand, inner: string): string =>
  `${pick(rng, PRE_COMMANDS)} && ${inner}`;
const STRUCT_OR_CHAIN = (rng: Rand, inner: string): string =>
  `${pick(rng, PRE_COMMANDS)} || ${inner}`;
const STRUCT_SEMI_LEAD = (_rng: Rand, inner: string): string => `; ${inner}`;
const STRUCT_SEMI_CHAIN = (rng: Rand, inner: string): string =>
  `${pick(rng, PRE_COMMANDS)} ; ${inner}`;
const STRUCT_PIPE_CHAIN = (rng: Rand, inner: string): string =>
  `${pick(rng, PRE_COMMANDS)} | ${inner}`;

const SHELLS: ReadonlyArray<string> = ["bash", "sh", "zsh", "dash", "ksh"];
const STRUCT_SHELL_C_DQ = (rng: Rand, inner: string): string =>
  // bash -c "axhub ..." — outer double-quoted, inner has none.
  `${pick(rng, SHELLS)} -c "${inner}"`;
const STRUCT_SHELL_C_SQ = (rng: Rand, inner: string): string =>
  // sh -c 'axhub ...' — outer single-quoted, no escaping needed.
  `${pick(rng, SHELLS)} -c '${inner}'`;
const STRUCT_EVAL_DQ = (_rng: Rand, inner: string): string =>
  `eval "${inner}"`;
const STRUCT_EVAL_SQ = (_rng: Rand, inner: string): string =>
  `eval '${inner}'`;

type Wrapper = (rng: Rand, inner: string) => string;
const STRUCTURAL_WRAPPERS: ReadonlyArray<{ name: string; fn: Wrapper }> = [
  { name: "none", fn: STRUCT_NONE },
  { name: "$()", fn: STRUCT_DOLLAR_PAREN },
  { name: "backticks", fn: STRUCT_BACKTICKS },
  { name: "parens", fn: STRUCT_PARENS },
  { name: "&&", fn: STRUCT_AMP_CHAIN },
  { name: "||", fn: STRUCT_OR_CHAIN },
  { name: "leading;", fn: STRUCT_SEMI_LEAD },
  { name: ";chain", fn: STRUCT_SEMI_CHAIN },
  { name: "|chain", fn: STRUCT_PIPE_CHAIN },
  { name: "shell-c-dq", fn: STRUCT_SHELL_C_DQ },
  { name: "shell-c-sq", fn: STRUCT_SHELL_C_SQ },
  { name: "eval-dq", fn: STRUCT_EVAL_DQ },
  { name: "eval-sq", fn: STRUCT_EVAL_SQ },
];

// ---------------------------------------------------------------------------
// Bucket 3: whitespace wrappers — replace inter-token spaces with tabs or
// multi-space runs. Stays within ASCII whitespace (`\s` regex class), which
// the parser tokenizer (`split(/\s+/)`) handles.
//
// NOTE: Unicode whitespace (U+00A0, U+2000-200B, U+3000) is NOT a real token
// separator under JS `\s`, so injecting it as a separator would be a known
// gotcha the parser deliberately doesn't handle. We inject Unicode whitespace
// only INSIDE arg values (after a `=` in a `--flag=value` form) where it
// stays part of one token and doesn't change tokenization.
// ---------------------------------------------------------------------------

const tabsAndSpaces = (rng: Rand, inner: string): string => {
  if (rng() < 0.5) return inner; // half the variants keep stock spacing
  return inner.replace(/ /g, () => {
    const r = rng();
    if (r < 0.20) return "\t";
    if (r < 0.30) return "  ";
    if (r < 0.35) return " \t ";
    return " ";
  });
};

const UNICODE_WS = [" ", " ", "​", "　"] as const;
const unicodeInArg = (rng: Rand, inner: string): string => {
  if (rng() < 0.7) return inner; // sparse — only ~30% of variants get Unicode
  // Find a `--flag=value` token and inject Unicode whitespace AFTER the `=`,
  // so it stays inside the value (single token) — does not affect tokenization.
  const tokens = inner.split(/ /);
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    if (t === undefined) continue;
    if (t.startsWith("--") && t.includes("=")) {
      const eq = t.indexOf("=");
      tokens[i] = t.slice(0, eq + 1) + pick(rng, UNICODE_WS) + t.slice(eq + 1);
      return tokens.join(" ");
    }
  }
  return inner;
};

// Some variants flip `--flag value` to `--flag=value` so the unicode-in-arg
// wrapper has something to bite on. Conservative: only the first eligible flag.
const FLAG_EQ_CANDIDATES: ReadonlyArray<string> = ["--app", "--branch", "--commit", "--profile"];
const maybeFlagEqForm = (rng: Rand, inner: string): string => {
  if (rng() < 0.5) return inner;
  const tokens = inner.split(/ /);
  for (let i = 0; i < tokens.length - 1; i++) {
    if (FLAG_EQ_CANDIDATES.includes(tokens[i] as string)) {
      const flag = tokens[i] as string;
      const val = tokens[i + 1] as string;
      tokens.splice(i, 2, `${flag}=${val}`);
      return tokens.join(" ");
    }
  }
  return inner;
};

// ---------------------------------------------------------------------------
// Variant generator: pick base, optionally flip flag form, apply env prefix,
// apply ONE structural wrapper, finally apply whitespace mutations.
// ---------------------------------------------------------------------------
interface Variant {
  command: string;
  base: string;
  layers: string[];
}

const generateVariant = (rng: Rand): Variant => {
  const base = pick(rng, BASE_COMMANDS);
  const layers: string[] = [];

  let cur = base;

  // Step 1: optionally rewrite a flag to --flag=value form (so unicodeInArg
  // has a target). No-op for `auth login` and `update apply` (no value flags).
  const beforeEq = cur;
  cur = maybeFlagEqForm(rng, cur);
  if (cur !== beforeEq) layers.push("flag=val");

  // Step 2: env-prefix (often skipped — see envPrefix's internal coin flip).
  const beforeEnv = cur;
  cur = envPrefix(rng, cur);
  if (cur !== beforeEnv) layers.push("env-prefix");

  // Step 3: exactly one structural wrapper (one of 13, including "none").
  const struct = pick(rng, STRUCTURAL_WRAPPERS);
  cur = struct.fn(rng, cur);
  layers.push(struct.name);

  // Step 4: whitespace mutations (tabs / multi-space, sometimes Unicode-in-arg).
  const beforeWs = cur;
  cur = tabsAndSpaces(rng, cur);
  if (cur !== beforeWs) layers.push("ws");
  const beforeUni = cur;
  cur = unicodeInArg(rng, cur);
  if (cur !== beforeUni) layers.push("uni-in-arg");

  return { command: cur, base, layers };
};

// ---------------------------------------------------------------------------
// Main: generate VARIANT_COUNT variants, classify, report bypass cases.
// ---------------------------------------------------------------------------
const VARIANT_COUNT = 1000;

const main = (): number => {
  const seedArg = process.argv[2];
  const seed = parseSeed(seedArg);
  const rng = mulberry32(seed);

  const seedDisplay = "0x" + seed.toString(16).padStart(8, "0");
  process.stdout.write(`fuzz-parser: seed=${seedDisplay} variants=${VARIANT_COUNT}\n`);

  let caught = 0;
  const bypasses: Array<{ idx: number; v: Variant }> = [];

  for (let i = 0; i < VARIANT_COUNT; i++) {
    const v = generateVariant(rng);
    const r = parseAxhubCommand(v.command);
    const isDestructive = r.is_destructive === true || r.action !== undefined;
    if (isDestructive) {
      caught++;
    } else {
      bypasses.push({ idx: i, v });
    }
  }

  if (bypasses.length === 0) {
    process.stdout.write(`${caught}/${VARIANT_COUNT} caught\n`);
    return 0;
  }

  process.stdout.write(
    `${caught}/${VARIANT_COUNT} caught — ${bypasses.length} BYPASS(ES) FOUND:\n`,
  );
  // Cap detail output so a wide bypass doesn't flood CI logs.
  const showMax = Math.min(bypasses.length, 25);
  for (let i = 0; i < showMax; i++) {
    const b = bypasses[i] as { idx: number; v: Variant };
    process.stdout.write(
      `  #${b.idx} layers=[${b.v.layers.join(",")}] base="${b.v.base}"\n` +
        `       cmd=${JSON.stringify(b.v.command)}\n`,
    );
  }
  if (bypasses.length > showMax) {
    process.stdout.write(`  ...and ${bypasses.length - showMax} more\n`);
  }
  return 1;
};

process.exit(main());
