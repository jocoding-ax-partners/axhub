/**
 * resolve.ts — live resolution of {profile, endpoint, app_id, branch, commit}
 * from `axhub auth status` + `axhub apps list` + git (US-001 deploy step 1).
 *
 * Implements PLAN row 13 (live profile/app resolve in mutation paths — never
 * use cached app_id) and row 44 (echo all 5 identity fields in preview card).
 *
 * Design notes
 * ------------
 * - axhub v0.1.0 `apps list` does NOT support `--slug-prefix` (verified
 *   against the GA binary 2026-04-23). We fetch the full list and filter
 *   client-side. M2+ may switch to server-side filter once landed.
 * - Slug extraction is deliberately conservative — the AskUserQuestion
 *   preview card is the disambiguation surface, not this helper.
 * - All shell calls go through an injectable CommandRunner so tests never
 *   need a real axhub binary or live git repo.
 */

import {
  axhubBin,
  defaultRunner,
  EXIT_AUTH,
  EXIT_OK,
  EXIT_USAGE,
  parseAuthStatus,
  type AuthStatus,
  type CommandRunner,
  type SpawnResult,
} from "./preflight.ts";

// PLAN §3.2: 67 = resource not found (app slug doesn't match anything).
export const EXIT_NOT_FOUND = 67;

// Default ETA for deploy in seconds (PLAN row 20: ETA in preview card).
// M2+ may refine via app historical p50 from `axhub deploy list`.
export const DEFAULT_DEPLOY_ETA_SEC = 60;

export interface ResolveArgs {
  intent: string | null;
  userUtterance: string;
}

/**
 * Parse the resolve subcommand's CLI args. Recognized:
 *   --intent <name>           the skill that called us (deploy, status, ...)
 *   --user-utterance "<text>" raw user input to mine for a slug candidate
 *   --json                    accepted for forward-compat (output is always JSON)
 */
export function parseResolveArgs(args: string[]): ResolveArgs {
  let intent: string | null = null;
  let userUtterance = "";
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === "--intent" && i + 1 < args.length) {
      intent = args[++i] ?? null;
    } else if (a === "--user-utterance" && i + 1 < args.length) {
      userUtterance = args[++i] ?? "";
    }
    // --json is accepted for forward-compat but has no behavioural effect;
    // resolve always emits JSON.
  }
  return { intent, userUtterance };
}

/**
 * Stop-words stripped when extracting candidate app slugs from utterance.
 * Mixes Korean deploy/manage verbs + English imperatives + filler particles.
 * Conservative — over-filtering is recoverable (caller handles "no candidate"),
 * under-filtering surfaces noise as the matched slug.
 */
const STOP_WORDS: ReadonlySet<string> = new Set<string>([
  // Korean verbs / particles
  "배포", "배포해", "배포해줘", "올려", "올리자", "쏘자", "내보내자",
  "푸시한", "프로덕션에", "박아", "터트려", "공개해", "그거", "거", "좀",
  "해줘", "해", "하자", "해봐", "보여줘", "주세요", "을", "를", "에", "이",
  "가", "은", "는", "의", "도", "만", "지금", "방금", "어떻게", "됐어",
  // English verbs / fillers
  "deploy", "ship", "release", "rollout", "launch", "push", "the", "to",
  "now", "please", "my", "app", "for", "of", "a", "an",
]);

/**
 * Extract a likely slug candidate from natural-language utterance. Heuristic:
 *   1. NFKC normalize + lowercase
 *   2. split on whitespace + common punctuation
 *   3. drop stop-words, flag-shaped tokens, sub-2-char tokens
 *   4. return the first token that matches slug shape `[a-z0-9][a-z0-9-]*`
 *
 * Returns null when no plausible slug survives — the caller then asks the
 * user to disambiguate via the preview card.
 */
export function extractSlugCandidate(utterance: string): string | null {
  const normalized = utterance.normalize("NFKC").toLowerCase();
  const tokens = normalized.split(/[\s,./!?;:()"'`\[\]{}]+/).filter(Boolean);
  for (const tok of tokens) {
    if (STOP_WORDS.has(tok)) continue;
    if (tok.startsWith("-")) continue;
    if (tok.length < 2) continue;
    if (/^[a-z0-9][a-z0-9-]*$/.test(tok)) return tok;
  }
  return null;
}

export interface AppRecord {
  id: number;
  slug: string;
  name?: string;
}

/**
 * Parse `axhub apps list --json` output. Returns null when the output isn't
 * a JSON array (e.g. error envelope). Drops items missing required fields.
 */
export function parseAppsList(stdout: string): AppRecord[] | null {
  let parsed: unknown;
  try {
    parsed = JSON.parse(stdout);
  } catch {
    return null;
  }
  if (!Array.isArray(parsed)) return null;
  const apps: AppRecord[] = [];
  for (const item of parsed) {
    if (item && typeof item === "object") {
      const obj = item as Record<string, unknown>;
      if (typeof obj["id"] === "number" && typeof obj["slug"] === "string") {
        apps.push({
          id: obj["id"],
          slug: obj["slug"],
          name: typeof obj["name"] === "string" ? obj["name"] : undefined,
        });
      }
    }
  }
  return apps;
}

/**
 * Filter apps by slug. Prefix match is preferred; falls back to substring
 * match if no prefix hit, so utterances like "show paydrop status" still
 * resolve when the slug appears mid-sentence.
 */
export function filterAppsBySlug(apps: AppRecord[], candidate: string): AppRecord[] {
  const needle = candidate.normalize("NFKC").toLowerCase();
  const prefixHits = apps.filter((a) => a.slug.toLowerCase().startsWith(needle));
  if (prefixHits.length > 0) return prefixHits;
  return apps.filter((a) => a.slug.toLowerCase().includes(needle));
}

export interface GitContext {
  branch: string | null;
  commit_sha: string | null;
  commit_message: string | null;
}

/**
 * Read the current branch / HEAD SHA / commit subject. Each command failure
 * surfaces as a null field rather than an error — useful when resolve runs
 * outside a git repo (e.g. headless smoke test).
 */
export function readGitContext(runner: CommandRunner): GitContext {
  const safe = (cmd: string[]): SpawnResult => {
    try {
      return runner(cmd);
    } catch {
      return { exitCode: 1, stdout: "", stderr: "" };
    }
  };
  const branch = safe(["git", "branch", "--show-current"]);
  const sha = safe(["git", "rev-parse", "HEAD"]);
  const msg = safe(["git", "log", "-1", "--pretty=%s"]);
  return {
    branch: branch.exitCode === EXIT_OK ? branch.stdout.trim() || null : null,
    commit_sha: sha.exitCode === EXIT_OK ? sha.stdout.trim() || null : null,
    commit_message: msg.exitCode === EXIT_OK ? msg.stdout.trim() || null : null,
  };
}

export interface ResolveOutput {
  profile: string | null;
  endpoint: string | null;
  app_id: number | null;
  app_slug: string | null;
  candidate_slug: string | null;
  matched_apps: Array<{ id: number; slug: string }>;
  branch: string | null;
  commit_sha: string | null;
  commit_message: string | null;
  eta_sec: number;
  error: string | null;
}

/**
 * Live resolve {profile, endpoint, app_id, branch, commit_sha, ...}. Pure-ish:
 * no console output, no process.exit. Returns the structured output + the
 * exit code the CLI subcommand should propagate.
 *
 * Exit codes:
 *   - 0  exactly one app matched the candidate slug
 *   - 64 ambiguous (≥2 matches) — caller asks user to pick numeric id
 *   - 65 auth missing/expired/misconfigured
 *   - 67 candidate produced no matches (or no candidate extractable)
 */
export function runResolve(
  args: string[],
  runner: CommandRunner = defaultRunner,
): { output: ResolveOutput; exitCode: number } {
  const { userUtterance } = parseResolveArgs(args);
  const candidate = extractSlugCandidate(userUtterance);
  const bin = axhubBin();

  const baseOutput: ResolveOutput = {
    profile: process.env["AXHUB_PROFILE"] || null,
    endpoint: process.env["AXHUB_ENDPOINT"] || null,
    app_id: null,
    app_slug: null,
    candidate_slug: candidate,
    matched_apps: [],
    branch: null,
    commit_sha: null,
    commit_message: null,
    eta_sec: DEFAULT_DEPLOY_ETA_SEC,
    error: null,
  };

  // 1. Auth — establishes that the CLI is configured for this profile.
  let authResult: SpawnResult;
  try {
    authResult = runner([bin, "auth", "status", "--json"]);
  } catch {
    authResult = { exitCode: 1, stdout: "", stderr: "auth status spawn failed" };
  }
  const auth: AuthStatus = parseAuthStatus(authResult.stdout);
  if (!auth.ok) {
    return {
      output: { ...baseOutput, error: `auth_${auth.code}` },
      exitCode: EXIT_AUTH,
    };
  }

  // 2. Apps list — full fetch since v0.1.0 has no --slug-prefix.
  let appsResult: SpawnResult;
  try {
    appsResult = runner([bin, "apps", "list", "--json"]);
  } catch {
    appsResult = { exitCode: 1, stdout: "", stderr: "apps list spawn failed" };
  }
  const apps = parseAppsList(appsResult.stdout);
  if (apps === null) {
    return {
      output: { ...baseOutput, error: "apps_list_parse_error" },
      exitCode: EXIT_NOT_FOUND,
    };
  }

  // 3. Filter + git context. Git lookup happens regardless of match outcome
  // so the preview card has branch/commit even when disambiguation is needed.
  const matches = candidate ? filterAppsBySlug(apps, candidate) : [];
  const git = readGitContext(runner);

  if (matches.length === 0) {
    return {
      output: {
        ...baseOutput,
        ...git,
        error: candidate ? "app_not_found" : "no_candidate_slug",
      },
      exitCode: EXIT_NOT_FOUND,
    };
  }

  if (matches.length > 1) {
    return {
      output: {
        ...baseOutput,
        ...git,
        matched_apps: matches.map((a) => ({ id: a.id, slug: a.slug })),
        error: "app_ambiguous",
      },
      exitCode: EXIT_USAGE,
    };
  }

  // Exactly one match — resolved. Non-null asserted because we just checked length.
  const sole = matches[0]!;
  return {
    output: {
      ...baseOutput,
      ...git,
      app_id: sole.id,
      app_slug: sole.slug,
      matched_apps: [{ id: sole.id, slug: sole.slug }],
    },
    exitCode: EXIT_OK,
  };
}
