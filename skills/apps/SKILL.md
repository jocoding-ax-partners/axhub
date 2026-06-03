---
name: apps
description: '이 스킬은 사용자가 팀에 등록된 axhub 앱 목록을 보거나 명시적인 앱 관리 작업을 요청할 때 사용해요. 다음 표현에서 활성화: "내 앱", "내 앱 보여줘", "내 앱 봐", "등록된 앱", "등록된 앱 봐", "앱 등록", "앱 리스트", "앱 목록", "앱 목록 보여주세요", "앱 목록 봐", "앱 뭐", "앱 뭐 있어", "앱 보여", "앱 봐", "앱 삭제", "앱 생성", "앱 슬러그", "앱 슬러그 봐", "앱 제거", "앱 지워", "앱 id", "앱 ID 봐", "어떤 앱", "어떤 앱 있어", "어떤 앱이 있나요", "우리 앱", "우리 앱 봐", "운영 중인 앱", "운영 중인 앱 뭐 있어", "운영 중인 앱 보여주세요", "제 앱", "제 앱들 보여주세요", "회사 앱", "회사 앱 뭐 있어", "app catalog", "app list", "apps", "apps create", "apps delete", "apps rm", "available apps", "list apps", "my apps", "which apps", 또는 앱 카탈로그/관리 흐름. 현재 팀 scope 으로 출력 필터링하고 생성/수정/delete 작업은 승인 토큰을 요구해요.'
examples:
  - utterance: "내 앱 목록 보여줘"
    intent: "list axhub apps"
  - utterance: "이 앱 삭제해"
    intent: "list axhub apps"
  - utterance: "list my apps"
    intent: "list axhub apps"
  - utterance: "list apps"
    intent: "list axhub apps"
  - utterance: "내 앱 봐"
    intent: "list axhub apps"
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Apps Management (team-scoped; mutations consent-gated)

Show registered axhub apps for the current team. Listing/details are read-only; create, update, and delete paths require an AskUserQuestion preview plus HMAC consent token before any mutation command.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

## Workflow

To list apps:

1. **Pre-flight (lightweight).** Confirm auth before the list call:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" preflight --json
   ```

   On `auth_ok: false`, halt and route to `../deploy/references/error-empathy-catalog.md` (auth 템플릿 — catalog "exit 4" 섹션; helper preflight 가 내는 65 는 거기로 정규화돼요). Suggest the auth skill via "다시 로그인해줘".

2. **Fetch apps:**

   ```bash
   TEAM_ID="$(printf '%s\n' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')"
   if [ -n "$TEAM_ID" ]; then
     axhub apps list --tenant "$TEAM_ID" --json
   else
     axhub apps list --json
   fi
   ```

3. **Keep scope server-side.** Prefer the preflight `current_team_id` and pass it as `--tenant` so the CLI/server owns tenant filtering. If preflight has no team id, use the current profile scoped `axhub apps list --json` result as-is. Do not invent a client-side `team_id` filter, because v0.17.3 app rows do not expose that field.

4. **Render top 10 in Korean.** Format as a numbered list with `slug (id=N) — <status>` per row:

   ```
   현재 팀 앱 10개 (전체 N개):
     ① paydrop (id=42) — production: succeeded (12분 전)
     ② paydrop-staging (id=43) — staging: succeeded (1시간 전)
     ③ checkout-svc (id=44) — production: failed (어제)
     ...
   ```

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — expansion → `skip` (top 10 으로 충분), delete confirmation → `abort` (삭제 안 함).

5. **Offer expansion.** If the filtered list exceeds 10, surface AskUserQuestion:

   ```json
   {
     "question": "앱이 더 있어요. 전체 목록 볼래요?",
     "header": "전체 보기",
     "options": [
       {"label": "네, 전체 보기", "value": "show_all", "description": "현재 팀의 모든 앱"},
       {"label": "지금은 그대로", "value": "skip", "description": "상위 10개로 충분"},
       {"label": "검색 (slug 입력)", "value": "search", "description": "특정 slug 검색"}
     ]
   }
   ```

6. **On `validation.app_list_truncated`** (>100 apps server-side): route to `../deploy/references/error-empathy-catalog.md` ("exit 64 + validation.app_list_truncated"); ask user to provide a numeric `--app <id>` directly.

7. **On non-zero exit**, route via `axhub-helpers classify-exit "$EXIT" "$STDOUT"` (spec 004 Fork-A — canonical router) or `../deploy/references/error-empathy-catalog.md` by current CLI exit code: 4 (auth, 옛 sysexits 65 아님) / 5 (not-found, 옛 67) / 6 (rate-limit, 옛 68) / 1 (transport). `axhub apps list` 는 CLI-direct 라 CLI-native 4/5/6 을 내요. Read paths may auto-retry once on exit 1.

## v0.2.0 command coverage polish

Use these paths only when the user intent is explicit. Listing remains the default.

### apps owned / workspace / members

Read-only inventory variants stay in this skill. Use them when the user asks for ownership, workspace-shared apps, or app membership/access details:

```bash
axhub apps owned --json
axhub apps workspace --json
axhub apps members "$APP" --page "$PAGE" --per-page "$PER_PAGE" --json
```

`apps owned` and `apps workspace` have no pagination flags in v0.17.3; do not invent filters. For `apps members`, keep `--page`/`--per-page` optional and render a small Korean summary instead of dumping the raw member payload.

### apps create

1. Preview the source file or interactive intent first.
2. If this is an interactive Claude Code session, render a short approval card before minting consent:

   ```json
   {
     "question": "앱을 만들까요?",
     "header": "앱 생성",
     "options": [
       {"label": "생성", "value": "create", "description": "표시한 앱을 실제로 만들어요."},
       {"label": "취소", "value": "abort", "description": "앱을 만들지 않아요."}
     ]
   }
   ```

   In non-interactive mode, use the registry safe default `abort` and stop before `consent-mint`.
3. Mint consent with stdin JSON:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   printf '%s\n' "$CONSENT_BINDING_JSON" | "$HELPER" consent-mint
   ```

   Required binding fields must match the exact command shape:

   ```bash
   # axhub apps create --from-file axhub.yaml --json
   CONSENT_BINDING_JSON=$(jq -nc \
     '{tool_call_id:"pending",action:"apps_create",app_id:"",profile:"",branch:"",commit_sha:"",context:{source:"axhub.yaml"}}')

   # axhub apps create --interactive --json
   CONSENT_BINDING_JSON=$(jq -nc \
     '{tool_call_id:"pending",action:"apps_create",app_id:"",profile:"",branch:"",commit_sha:"",context:{source:"interactive"}}')

   # axhub apps create --name "$NAME" --slug "$SLUG" --json
   CONSENT_BINDING_JSON=$(jq -nc \
     --arg slug "$SLUG" \
     '{tool_call_id:"pending",action:"apps_create",app_id:$slug,profile:"",branch:"",commit_sha:"",context:{slug:$slug,source:"inline"}}')
   ```

   `apps create` has no dry-run/`--execute` in v0.17.3, so the preview must come from local file parsing or explicit user intent before the consent token is minted. For the `--name` + `--slug` path, `app_id` and `context.slug` must both equal the exact `$SLUG`; otherwise the PreToolUse HMAC gate rejects the command.
4. Run one of the current CLI contracts. Use one mutation command per Bash tool call; do not batch another destructive axhub command into the same Bash input:

   ```bash
   axhub apps create --from-file axhub.yaml --json
   axhub apps create --interactive --json
   axhub apps create --name "$NAME" --slug "$SLUG" --json
   ```

### apps get

Read-only details use:

```bash
axhub apps get "$APP" --json
```

### apps update

Preview each changed field, then mint `action=apps_update` with top-level `app_id` and `context={slug,fields,<changed typed values>}` before using the real v0.17.3 typed flags:

```bash
axhub apps update "$APP" --name "$NAME" --description "$DESCRIPTION" --visibility private --json
axhub apps update "$APP" --resource-tier M --subdomain "$SUBDOMAIN" --json
axhub apps update "$APP" --clear-subdomain --json
```

`--field` is not a v0.17.3 CLI flag; do not generate it.

### apps delete

Deletion is consent-gated. Do **not** run `axhub apps delete ... --dry-run --json` before approval; the hook parser currently treats every `apps delete` shape as destructive, including dry-run.

1. Build the preview only from read-only data:

   ```bash
   axhub apps list --json
   axhub apps get "$COMMAND_TARGET" --json
   ```

2. Define one target and keep it unchanged through the whole flow:

   ```bash
   COMMAND_TARGET="$APP"
   ```

   Prefer the exact slug the user typed or selected. If the user selected a numeric id instead, use that exact numeric id. The preview may show both slug and numeric id, but consent-bound fields use only `COMMAND_TARGET`.

3. Ask for exact confirmation before minting a token:

   ```json
   {
     "question": "앱을 삭제할까요?",
     "header": "앱 삭제",
     "options": [
       {"label": "삭제", "value": "delete", "description": "표시한 COMMAND_TARGET 앱을 삭제해요."},
       {"label": "취소", "value": "abort", "description": "삭제하지 않아요."}
     ]
   }
   ```

   In non-interactive mode, use the registry safe default `abort` and stop.

4. Mint consent with the literal command-target invariant. For `apps_delete`, `context.slug` is the parser field name and may contain a numeric id when `COMMAND_TARGET` is numeric.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   # Binding shape: {"action":"apps_delete","app_id":"$COMMAND_TARGET","context":{"slug":"$COMMAND_TARGET"}}
   CONSENT_BINDING_JSON=$(jq -nc \
     --arg target "$COMMAND_TARGET" \
     '{tool_call_id:"pending",action:"apps_delete",app_id:$target,profile:"",branch:"",commit_sha:"",context:{slug:$target}}')
   printf '%s\n' "$CONSENT_BINDING_JSON" | "$HELPER" consent-mint
   ```

5. Run exactly one delete command using the same target string:

   ```bash
   axhub apps delete "$COMMAND_TARGET" --execute --json
   ```

### apps open delegation

If the user wants to open a live app or dashboard, route to `../open/SKILL.md` instead of using `axhub apps open`.

## NEVER

- NEVER list cross-team apps without explicit user opt-in (F4 privacy guarantee).
- NEVER dump >10 rows in the first response (overwhelms vibe coders).
- NEVER drop `--json` (parsing depends on it).
- NEVER cache app_id locally for deploy mutation paths — the deploy skill must live-resolve.
- NEVER remint apps delete consent with numeric id when the command will use a slug, or with slug when the command will use a numeric id. Keep `COMMAND_TARGET` identical.
- NEVER echo internal endpoint URLs of cross-team apps even if visible in stdout.

## Additional Resources

For Korean trigger lexicon (apps intent): `../deploy/references/nl-lexicon.md`.
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For privacy filter rules (cross-team scope, NFKC normalize): see the redact subcommand in `axhub-helpers` and PLAN §16.17.
