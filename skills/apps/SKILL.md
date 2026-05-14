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

**Pre-execute preflight context (Phase 17 US-1706 — `!command` injection)**:

```
!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`
```

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요. 출력 (auth_status, current_app, current_env) 이 컨텍스트에 자동 주입돼서 별도 auth/profile 호출이 줄어요.

## Workflow

To list apps:

1. **Pre-flight (lightweight).** Confirm auth before the list call:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

   On `auth_ok: false`, halt and route to `../deploy/references/error-empathy-catalog.md` ("exit 65"). Suggest the auth skill via "다시 로그인해줘".

2. **Fetch apps:**

   ```bash
   axhub apps list --json
   ```

3. **Filter to current team scope.** Drop entries whose `team_id` does not match `$AXHUB_TEAM_ID` (or the team derived from `axhub auth status --json`). Do NOT dump cross-team apps to the user — they are surface noise that breaks the F4 privacy guarantee.

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

7. **On non-zero exit**, route to `../deploy/references/error-empathy-catalog.md` by exit code (65 / 67 / 68 / 1). Read paths may auto-retry once on exit 1.

## v0.2.0 command coverage polish

Use these paths only when the user intent is explicit. Listing remains the default.

### apps create

1. Preview the source file or interactive intent first.
2. Mint consent with stdin JSON:

   ```bash
   printf '%s\n' "$CONSENT_BINDING_JSON" | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   ```

   Required binding fields: `action=apps_create`, `context={source}`.
3. Run one of the current CLI contracts:

   ```bash
   axhub apps create --from-file apphub.yaml --yes --json
   axhub apps create --interactive --json
   ```

### apps get

Read-only details use:

```bash
axhub apps get "$APP" --json
```

### apps update

Preview each `key=value` field, then mint `action=apps_update` with top-level `app_id` and `context={slug,field}` before:

```bash
axhub apps update "$APP" --field "$FIELD" --json
```

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
   # Binding shape: {"action":"apps_delete","app_id":"$COMMAND_TARGET","context":{"slug":"$COMMAND_TARGET"}}
   CONSENT_BINDING_JSON=$(jq -nc \
     --arg target "$COMMAND_TARGET" \
     '{tool_call_id:"pending",action:"apps_delete",app_id:$target,profile:"",branch:"",commit_sha:"",context:{slug:$target}}')
   printf '%s\n' "$CONSENT_BINDING_JSON" | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   ```

5. Run exactly one delete command using the same target string:

   ```bash
   axhub apps delete "$COMMAND_TARGET" --yes --json
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
