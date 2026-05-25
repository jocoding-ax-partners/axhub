/**
 * Canonical in-body preflight block — single source for the scaffold (skill-new),
 * the one-shot migration, and any future tooling.
 *
 * Replaces the retired generated `!command` injection (codegen-preflight-injection.ts).
 * A needs-preflight:true SKILL runs `axhub-helpers preflight --json` as this in-body
 * bash step instead of a load-time `!command` injection. Rationale: the injection
 * hard-failed on first run with a raw English "requires approval" — Claude Code
 * permission-gates the outer `node -e` wrapper itself, and the inner denialRegex fallback
 * could never catch its own denial (dead path). An in-body bash call goes through the
 * standard interactive Bash permission flow instead. See docs/adr/0013-skill-preflight-in-body.md
 * (supersedes ADR-0011).
 *
 * Block semantics:
 *  - Pick the helper ONCE: plugin-root binary if executable, else PATH `axhub-helpers`.
 *  - Capture stdout regardless of exit code — preflight exits non-zero WITH error JSON
 *    (e.g. auth_error_code=cli_config_corrupted), so an `||` fallback would wrongly
 *    discard the useful error payload. Only fall back to `{}` when output is empty
 *    (binary truly missing) so jq stays parseable.
 *  - `echo "$PREFLIGHT_JSON"` surfaces auth/team/app/env (and any auth_error_code) to the
 *    model, which the prose then routes to the right /axhub:* skill. Consumers
 *    (deploy, my-resources) additionally read `$PREFLIGHT_JSON` in later steps.
 */
export const CANONICAL_PREFLIGHT_BLOCK = [
  "**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.",
  "",
  "```bash",
  'HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"',
  'PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)',
  `[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'`,
  'echo "$PREFLIGHT_JSON"',
  "```",
  "",
  "`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.",
].join("\n");
