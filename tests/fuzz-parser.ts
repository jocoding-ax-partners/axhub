#!/usr/bin/env bun
/**
 * tests/fuzz-parser.ts — parser-drift fuzzer for parseAxhubCommand.
 *
 * Phase 2 US-103: stress-test the destructive-command parser with 1000+
 * randomized variants of known-destructive base commands. Goal is regression
 * protection: if a future refactor narrows detection, the fuzzer trips.
 *
 * Each variant wraps a base destructive command with one wrapper from each of
 * three buckets (env-prefix, structural, whitespace). The first 1000 variants
 * stay within the parser-detectable space documented in Phase 2; the final 100
 * (Phase 3 US-201) probe the three gotcha classes that Phase 2 deliberately
 * avoided and Phase 3 closed:
 *   - trailing close-delimiter contamination on action token
 *   - nested sub-shell inside eval/bash -c (recursive shell tokenization)
 *   - quoted subcommand tokens (`axhub "deploy" "create"`)
 * Total target: 1100/1100 caught.
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
// Phase 3 US-201: gotcha variant generators (trailing-delimiter, nested-shell,
// quoted-subcommand). 100 variants total, evenly split. These are SEPARATE
// from the 1000-variant standard set above so the gotcha classes are an
// explicit add-on, not a silent change to the original fuzzer's coverage.
// ---------------------------------------------------------------------------

// Bases that pad less aggressively so close-delimiter contamination on the
// ACTION token is actually possible (the standard BASE_COMMANDS pad with a
// trailing flag specifically to avoid this).
const TIGHT_BASES: ReadonlyArray<string> = [
  "axhub auth login",
  "axhub update apply --yes",
  "axhub deploy create --app paydrop --branch main --commit abc",
];

// Gotcha #1: trailing close-delimiter — wrap with parens/backticks/sub-shell
// such that the closing char glues onto the LAST token (which may be the
// action subcommand or a flag value).
const gotcha1 = (rng: Rand): Variant => {
  const base = pick(rng, TIGHT_BASES);
  const wrappers: Array<[string, string, string]> = [
    ["paren", "(", ")"],
    ["dollar-paren", "$(", ")"],
    ["backtick", "`", "`"],
  ];
  const w = pick(rng, wrappers);
  const cmd = `${w[1]}${base}${w[2]}`;
  return { command: cmd, base, layers: ["gotcha1-trailing-delim", w[0]] };
};

// Gotcha #2: nested sub-shell inside eval/bash -c — `bash -c "(axhub ...)"`
// or `eval "bash -c \"axhub ...\""`. Tests recursive collectCommandPositions.
const gotcha2 = (rng: Rand): Variant => {
  const base = pick(rng, TIGHT_BASES);
  const inner = pick(rng, [
    `(${base})`,
    `$(${base})`,
    `\`${base}\``,
  ]);
  const outer = pick(rng, [
    (s: string) => `bash -c "${s}"`,
    (s: string) => `sh -c "${s}"`,
    (s: string) => `zsh -c '${s}'`,
    (s: string) => `eval "${s.replace(/"/g, '\\"')}"`,
    (s: string) => `eval '${s.replace(/'/g, "'\\''")}'`,
  ]);
  const cmd = outer(inner);
  return { command: cmd, base, layers: ["gotcha2-nested-shell"] };
};

// Gotcha #3: quoted subcommand tokens — `axhub "deploy" "create" ...`.
// Parser must strip surrounding quotes from each token.
const gotcha3 = (rng: Rand): Variant => {
  const base = pick(rng, TIGHT_BASES);
  const tokens = base.split(/\s+/);
  // Quote tokens 1 and 2 (the subcommand pair). Use random quote style.
  const q = pick(rng, ['"', "'"]);
  if (tokens.length >= 3) {
    tokens[1] = `${q}${tokens[1]}${q}`;
    tokens[2] = `${q}${tokens[2]}${q}`;
  }
  const cmd = tokens.join(" ");
  return { command: cmd, base, layers: ["gotcha3-quoted-sub", q === '"' ? "dq" : "sq"] };
};

const generateGotchaVariant = (rng: Rand, idx: number): Variant => {
  // Round-robin: 0,3,6... → gotcha1; 1,4,7... → gotcha2; 2,5,8... → gotcha3.
  const which = idx % 3;
  if (which === 0) return gotcha1(rng);
  if (which === 1) return gotcha2(rng);
  return gotcha3(rng);
};

// ---------------------------------------------------------------------------
// Main: generate VARIANT_COUNT standard + GOTCHA_COUNT gotcha variants.
// ---------------------------------------------------------------------------
const VARIANT_COUNT = 1000;
const GOTCHA_COUNT = 100;
const TOTAL_COUNT = VARIANT_COUNT + GOTCHA_COUNT;

const main = (): number => {
  const seedArg = process.argv[2];
  const seed = parseSeed(seedArg);
  const rng = mulberry32(seed);

  const seedDisplay = "0x" + seed.toString(16).padStart(8, "0");
  process.stdout.write(
    `fuzz-parser: seed=${seedDisplay} variants=${TOTAL_COUNT} (${VARIANT_COUNT} standard + ${GOTCHA_COUNT} gotcha)\n`,
  );

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

  for (let g = 0; g < GOTCHA_COUNT; g++) {
    const v = generateGotchaVariant(rng, g);
    const r = parseAxhubCommand(v.command);
    const isDestructive = r.is_destructive === true || r.action !== undefined;
    if (isDestructive) {
      caught++;
    } else {
      bypasses.push({ idx: VARIANT_COUNT + g, v });
    }
  }

  if (bypasses.length === 0) {
    process.stdout.write(`${caught}/${TOTAL_COUNT} caught\n`);
    return 0;
  }

  process.stdout.write(
    `${caught}/${TOTAL_COUNT} caught — ${bypasses.length} BYPASS(ES) FOUND:\n`,
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
