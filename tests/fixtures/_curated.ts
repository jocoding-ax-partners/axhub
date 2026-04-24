// Phase 2 US-102: 38 hand-curated fixtures source of truth.
//
// Run `bun tests/fixtures/_curated.ts` to regenerate the .json files in this
// directory. The .json files are the test contract; this script is the
// curation index. Adding a fixture: append below + rerun the script + commit
// both the new .json AND the updated _curated.ts.
//
// Categories (target: 38 total):
//   10 destructive (deploy_create variants, update_apply, auth_login)
//    8 read-only  (apps list, apis list, deploy status, deploy logs, auth status)
//    8 adversarial (env-prefix, sub-shell, eval, &&, ;, parens, backticks, quote)
//    4 unicode    (Cyrillic homoglyph, ZWJ, Bidi override, full-width digit)
//    4 profile/headless (AXHUB_PROFILE override, --profile flag, $CODESPACES, $SSH_TTY)
//    4 negative   (false-positive checks: comments, strings containing axhub, unrelated)

import { writeFile } from "node:fs/promises";
import { join, dirname } from "node:path";

interface Fixture {
  id: string;
  category: "destructive" | "read-only" | "adversarial" | "unicode" | "profile-headless" | "negative";
  description: string;
  input: { command: string };
  expected: {
    is_destructive: boolean;
    action?: "deploy_create" | "update_apply" | "deploy_logs_kill" | "auth_login";
    app_id?: string;
    branch?: string;
    commit_sha?: string;
    profile?: string;
  };
}

const FIXTURES: Fixture[] = [
  // ---- 10 destructive ---------------------------------------------------
  {
    id: "destructive-001-deploy-create-basic",
    category: "destructive",
    description: "Bare deploy create with all required flags",
    input: { command: "axhub deploy create --app paydrop --branch main --commit abc123" },
    expected: { is_destructive: true, action: "deploy_create", app_id: "paydrop", branch: "main", commit_sha: "abc123" },
  },
  {
    id: "destructive-002-deploy-create-json",
    category: "destructive",
    description: "Deploy create with --json flag",
    input: { command: "axhub deploy create --app paydrop --branch main --commit abc123 --json" },
    expected: { is_destructive: true, action: "deploy_create", app_id: "paydrop", branch: "main", commit_sha: "abc123" },
  },
  {
    id: "destructive-003-deploy-create-equals",
    category: "destructive",
    description: "Deploy create using --flag=value syntax",
    input: { command: "axhub deploy create --app=paydrop --branch=main --commit=abc123" },
    expected: { is_destructive: true, action: "deploy_create", app_id: "paydrop", branch: "main", commit_sha: "abc123" },
  },
  {
    id: "destructive-004-deploy-create-numeric-id",
    category: "destructive",
    description: "Deploy create with numeric app id",
    input: { command: "axhub deploy create --app 42 --branch main --commit abc123 --json" },
    expected: { is_destructive: true, action: "deploy_create", app_id: "42", branch: "main", commit_sha: "abc123" },
  },
  {
    id: "destructive-005-deploy-create-feature-branch",
    category: "destructive",
    description: "Deploy create with feature branch (slash in branch name)",
    input: { command: "axhub deploy create --app paydrop --branch feat/auth --commit deadbeef" },
    expected: { is_destructive: true, action: "deploy_create", app_id: "paydrop", branch: "feat/auth", commit_sha: "deadbeef" },
  },
  {
    id: "destructive-006-update-apply-basic",
    category: "destructive",
    description: "Update apply (CLI version upgrade)",
    input: { command: "axhub update apply --yes" },
    expected: { is_destructive: true, action: "update_apply" },
  },
  {
    id: "destructive-007-update-apply-with-cosign",
    category: "destructive",
    description: "Update apply with cosign required env (default-on per Phase 6 §16.10)",
    input: { command: "AXHUB_REQUIRE_COSIGN=1 axhub update apply --yes --json" },
    expected: { is_destructive: true, action: "update_apply" },
  },
  {
    id: "destructive-008-auth-login-basic",
    category: "destructive",
    description: "Auth login (mutates token storage at ~/.config/axhub/token)",
    input: { command: "axhub auth login" },
    expected: { is_destructive: true, action: "auth_login" },
  },
  {
    id: "destructive-009-auth-login-print-token",
    category: "destructive",
    description: "Auth login with print-token (still mutates token state)",
    input: { command: "axhub auth login --print-token" },
    expected: { is_destructive: true, action: "auth_login" },
  },
  {
    id: "destructive-010-deploy-create-tab-separated",
    category: "destructive",
    description: "Deploy create with tab-separated flags (whitespace robustness)",
    input: { command: "axhub\tdeploy\tcreate\t--app\tpaydrop\t--branch\tmain\t--commit\tabc" },
    expected: { is_destructive: true, action: "deploy_create", app_id: "paydrop", branch: "main", commit_sha: "abc" },
  },

  // ---- 8 read-only ------------------------------------------------------
  {
    id: "ro-001-apps-list",
    category: "read-only",
    description: "Apps list — pure read, no consent gate",
    input: { command: "axhub apps list --json" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-002-apps-list-paginated",
    category: "read-only",
    description: "Apps list with pagination",
    input: { command: "axhub apps list --json --per-page=10" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-003-apis-list-scoped",
    category: "read-only",
    description: "APIs list scoped to current app (default-deny pattern)",
    input: { command: "axhub apis list --app-id 42 --json" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-004-deploy-status",
    category: "read-only",
    description: "Deploy status without watch",
    input: { command: "axhub deploy status dep_42 --json" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-005-deploy-status-watch",
    category: "read-only",
    description: "Deploy status with --watch (still read-only)",
    input: { command: "axhub deploy status dep_42 --watch --json" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-006-deploy-logs-build",
    category: "read-only",
    description: "Deploy build logs",
    input: { command: "axhub deploy logs dep_42 --source build --json" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-007-deploy-logs-follow",
    category: "read-only",
    description: "Deploy logs with --follow (SSE stream, still read-only)",
    input: { command: "axhub deploy logs dep_42 --follow --source pod --json" },
    expected: { is_destructive: false },
  },
  {
    id: "ro-008-auth-status",
    category: "read-only",
    description: "Auth status query (no token mutation)",
    input: { command: "axhub auth status --json" },
    expected: { is_destructive: false },
  },

  // ---- 8 adversarial ----------------------------------------------------
  {
    id: "adv-001-env-prefix",
    category: "adversarial",
    description: "Env-prefix bypass attempt — must still detect destructive",
    input: { command: "FOO=bar axhub deploy create --app paydrop --branch main --commit abc" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-002-multi-env-prefix",
    category: "adversarial",
    description: "Multiple env-prefix assignments",
    input: { command: "FOO=bar BAZ=qux axhub deploy create --app paydrop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-003-sub-shell",
    category: "adversarial",
    description: "Sub-shell wrapper $(...) bypass attempt",
    input: { command: "$(axhub deploy create --app paydrop --branch main --commit abc --json)" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-004-eval-prefix",
    category: "adversarial",
    description: "Eval prefix bypass attempt",
    input: { command: "eval \"axhub deploy create --app paydrop --branch main --commit abc --json\"" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-005-and-chain",
    category: "adversarial",
    description: "&&-chain — destructive command in second position",
    input: { command: "true && axhub deploy create --app paydrop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-006-semi-chain",
    category: "adversarial",
    description: ";-chain — destructive command after semicolon",
    input: { command: "echo hi; axhub deploy create --app paydrop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-007-pipe-chain",
    category: "adversarial",
    description: "Pipe chain — destructive in second position",
    input: { command: "echo hi | axhub deploy create --app paydrop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "adv-008-bash-c",
    category: "adversarial",
    description: "bash -c wrapper — destructive in shell-string",
    input: { command: "bash -c \"axhub deploy create --app paydrop --branch main --commit abc --json\"" },
    expected: { is_destructive: true, action: "deploy_create" },
  },

  // ---- 4 unicode --------------------------------------------------------
  {
    id: "uni-001-cyrillic-homoglyph",
    category: "unicode",
    description: "Cyrillic 'а' in app slug (homoglyph for ASCII 'a') — parser still flags destructive",
    input: { command: "axhub deploy create --app pаydrop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "uni-002-zwj",
    category: "unicode",
    description: "Zero-width joiner inside app slug — parser still flags destructive",
    input: { command: "axhub deploy create --app pay‍drop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "uni-003-fullwidth-digit",
    category: "unicode",
    description: "Full-width digit '１' in commit sha — parser still flags destructive",
    input: { command: "axhub deploy create --app paydrop --branch main --commit abc１23 --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "uni-004-nbsp-in-flag",
    category: "unicode",
    description: "Non-breaking space (U+00A0) inside flag value",
    input: { command: "axhub deploy create --app=pay drop --branch=main --commit=abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },

  // ---- 4 profile/headless ----------------------------------------------
  {
    id: "prf-001-axhub-profile-env",
    category: "profile-headless",
    description: "AXHUB_PROFILE env override (staging profile)",
    input: { command: "AXHUB_PROFILE=staging axhub deploy create --app paydrop --branch main --commit abc --json" },
    expected: { is_destructive: true, action: "deploy_create" },
  },
  {
    id: "prf-002-bare-axhub-version",
    category: "profile-headless",
    description: "Bare axhub --version (read-only diagnostic, not destructive)",
    input: { command: "axhub --version" },
    expected: { is_destructive: false },
  },
  {
    id: "prf-003-headless-token-paste",
    category: "profile-headless",
    description: "Headless token-paste flow (auth login without browser)",
    input: { command: "axhub auth login --print-token" },
    expected: { is_destructive: true, action: "auth_login" },
  },
  {
    id: "prf-004-axhub-help",
    category: "profile-headless",
    description: "axhub --help (info-only, not destructive)",
    input: { command: "axhub --help" },
    expected: { is_destructive: false },
  },

  // ---- 4 negative (false-positive checks) ------------------------------
  {
    id: "neg-001-not-axhub-command",
    category: "negative",
    description: "Random non-axhub bash command — must NOT be destructive",
    input: { command: "ls -la" },
    expected: { is_destructive: false },
  },
  {
    id: "neg-002-comment-with-axhub",
    category: "negative",
    description: "Comment containing 'axhub deploy' — must NOT be destructive",
    input: { command: "# This will run axhub deploy create later" },
    expected: { is_destructive: false },
  },
  {
    id: "neg-003-string-with-axhub",
    category: "negative",
    description: "String containing 'axhub deploy create' — echo only, NOT executable",
    input: { command: "echo 'axhub deploy create --app paydrop'" },
    expected: { is_destructive: false },
  },
  {
    id: "neg-004-other-tool-named-axhub",
    category: "negative",
    description: "Different tool whose name starts with 'axhub' (no subcommand match) — NOT destructive",
    input: { command: "axhubctl status" },
    expected: { is_destructive: false },
  },
];

const TOTAL = FIXTURES.length;
const COUNTS = FIXTURES.reduce<Record<string, number>>((acc, f) => {
  acc[f.category] = (acc[f.category] ?? 0) + 1;
  return acc;
}, {});

const main = async () => {
  if (TOTAL !== 38) {
    console.error(`FAIL: fixture count is ${TOTAL}, expected 38`);
    process.exit(1);
  }
  const dir = dirname(import.meta.path);
  for (const fixture of FIXTURES) {
    const path = join(dir, `${fixture.id}.json`);
    await writeFile(path, JSON.stringify({ input: fixture.input, expected: fixture.expected, description: fixture.description }, null, 2) + "\n");
  }
  console.log(`fixtures: wrote ${TOTAL} files`);
  console.log("category breakdown:", JSON.stringify(COUNTS));
};

if (import.meta.main) {
  await main();
}

export { FIXTURES };
export type { Fixture };
