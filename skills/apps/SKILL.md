---
name: apps
description: '이 스킬은 사용자가 팀에 등록된 axhub 앱 목록을 보거나 명시적인 앱 관리 작업을 요청할 때 사용해요. 다음 표현에서 활성화: "내 앱", "내 앱 보여줘", "내 앱 봐", "등록된 앱", "등록된 앱 봐", "앱 등록", "앱 리스트", "앱 목록", "앱 목록 보여주세요", "앱 목록 봐", "앱 뭐", "앱 뭐 있어", "앱 보여", "앱 봐", "앱 삭제", "앱 생성", "앱 슬러그", "앱 슬러그 봐", "앱 제거", "앱 지워", "앱 id", "앱 ID 봐", "어떤 앱", "어떤 앱 있어", "어떤 앱이 있나요", "우리 앱", "우리 앱 봐", "운영 중인 앱", "운영 중인 앱 뭐 있어", "운영 중인 앱 보여주세요", "제 앱", "제 앱들 보여주세요", "회사 앱", "회사 앱 뭐 있어", "app catalog", "app list", "apps", "apps create", "apps delete", "apps rm", "available apps", "list apps", "my apps", "which apps", 또는 앱 카탈로그/관리 흐름. 현재 팀 scope 으로 출력 필터링하고 생성/수정/delete 작업은 미리보기와 명시 확인을 요구해요.'
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

# Apps Management (team-scoped; mutations preview-gated)

Show registered axhub apps for the current team. Listing/details are read-only; create, update, and delete paths require an AskUserQuestion preview plus explicit confirmation before any mutation command.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

**Tenant 선택 (axhub-tenant-picker:L1).** 모든 fence 에서 `.axhub/state/tenant.json` 을 다시 읽어요 (cross-block source of truth). 명시 override → 캐시 re-read → tenants list → preflight fallback 순으로 tenant 를 결정해요.

```bash
# axhub-tenant-picker:L1 — canonical tenant resolver (매 fence .axhub/state/tenant.json re-read)
TENANT_CACHE=".axhub/state/tenant.json"
TENANT_CACHE_TTL="${AXHUB_TENANT_CACHE_TTL_SECS:-28800}"
AXHUB_TENANT="${AXHUB_TENANT:-}"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"

# Precedence 1: 명시 AXHUB_TENANT env/flag override → 즉시 사용, picker skip
if [ -z "$AXHUB_TENANT" ]; then
  # Precedence 2: .axhub/state/tenant.json re-read — cross-block source of truth
  if [ -f "$TENANT_CACHE" ]; then
    _T=$(jq -r '.tenant // empty' "$TENANT_CACHE" 2>/dev/null || true)
    _TS=$(jq -r '.ts // 0' "$TENANT_CACHE" 2>/dev/null || echo '0')
    # ts 는 신뢰할 수 없는 캐시 값 — 산술 $(( )) injection 방지로 숫자만 남겨요
    case "$_TS" in *[!0-9]*|"") _TS=0;; esac
    _NOW=$(date +%s 2>/dev/null || echo '0')
    _AGE=$(( _NOW - _TS ))
    if [ -n "$_T" ] && [ "$_AGE" -ge 0 ] && [ "$_AGE" -lt "$TENANT_CACHE_TTL" ]; then
      AXHUB_TENANT="$_T"
    else
      rm -f "$TENANT_CACHE"
    fi
  fi

  if [ -z "$AXHUB_TENANT" ]; then
    # Precedence 3: axhub tenants list → needs_pick(≥2) / auto(1) / fallback(0·fail)
    _TENANTS_JSON=$(axhub tenants list --json 2>/dev/null || echo '[]')
    _COUNT=$(printf '%s' "$_TENANTS_JSON" | jq 'if type=="array" then length else 0 end' 2>/dev/null || echo '0')
    if [ "$_COUNT" -eq 1 ]; then
      AXHUB_TENANT=$(printf '%s' "$_TENANTS_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
      mkdir -p "$(dirname "$TENANT_CACHE")"
      _TS_NOW=$(date +%s 2>/dev/null || echo '0')
      printf '{"tenant":"%s","source":"auto","ts":%s}\n' "$AXHUB_TENANT" "$_TS_NOW" > "$TENANT_CACHE"
    elif [ "$_COUNT" -ge 2 ]; then
      CANDIDATES_JSON="$_TENANTS_JSON"
      if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
        # non-TTY: active fallback + 경고 (R4 fail-wrong guard — L1 bash 위치 필수)
        AXHUB_TENANT=$(printf '%s' "$_TENANTS_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
        echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant(\`$AXHUB_TENANT\`)로 진행해요"
      else
        NEEDS_PICK="true"
      fi
    else
      # Precedence 4: preflight current_team_id fallback
      AXHUB_TENANT=$(printf '%s' "${PREFLIGHT_JSON:-{}}" | jq -r '.current_team_id // empty' 2>/dev/null || true)
    fi
  fi
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요.

**Tenant picker (axhub-tenant-picker:L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 실행해요. `CANDIDATES_JSON` 에서 후보 목록을 읽어 AskUserQuestion 으로 사용자에게 선택을 요청해요. 선택 결과를 `.axhub/state/tenant.json` 에 `{tenant, source:"picker", ts}` 형태로 기록해요 (이후 fence 가 re-read 해서 상속).

```typescript
if (NEEDS_PICK === "true") {
  const candidates = JSON.parse(CANDIDATES_JSON);
  AskUserQuestion({
    questions: [{
      question: "어떤 tenant 로 진행할까요?",
      header: "Tenant",
      multiSelect: false,
      options: candidates.map((t: { id?: string; slug?: string; name?: string }) => ({
        label: t.name ?? t.slug ?? t.id ?? "unknown",
        description: `ID: ${t.id ?? t.slug}`,
      })),
    }],
  });
  // 선택된 tenant ID 를 .axhub/state/tenant.json 에 write-back
  // mkdir -p .axhub/state && echo '{"tenant":"<선택값>","source":"picker","ts":<epoch>}' > .axhub/state/tenant.json
}
```

AskUserQuestion 답변을 받은 뒤 선택된 tenant ID 를 `AXHUB_TENANT` 로 확정하고 `.axhub/state/tenant.json` 에 `{"tenant": "<id>", "source": "picker", "ts": <epoch>}` 를 기록해요. 이후 fence 가 이 파일을 re-read 해서 같은 tenant 를 재사용해요.

**Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경에서는 L2 AskUserQuestion 을 건너뛰어요 — L1 블록이 이미 active fallback + 경고를 처리했어요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 `picker` 채널 참조.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To list apps:

1. **Pre-flight (lightweight).** Confirm auth before the list call:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
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
2. If this is an interactive Claude Code session, render a short approval card before confirming approval:

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

   In non-interactive mode, use the registry safe default `abort` and stop before any mutation command.
3. After approval, run one of the current CLI contracts. Use one mutation command per Bash tool call; do not batch another destructive axhub command into the same Bash input:

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

Preview each changed field, get explicit confirmation, then use the real v0.17.3 typed flags:

```bash
axhub apps update "$APP" --name "$NAME" --description "$DESCRIPTION" --visibility private --json
axhub apps update "$APP" --resource-tier M --subdomain "$SUBDOMAIN" --json
axhub apps update "$APP" --clear-subdomain --json
```

`--field` is not a v0.17.3 CLI flag; do not generate it.

### apps delete

Deletion is preview-gated. Do **not** run `axhub apps delete ... --dry-run --json` before approval; build the preview from read-only data instead.

1. Build the preview only from read-only data:

   ```bash
   axhub apps list --json
   axhub apps get "$COMMAND_TARGET" --json
   ```

2. Define one target and keep it unchanged through the whole flow:

   ```bash
   COMMAND_TARGET="$APP"
   ```

   Prefer the exact slug the user typed or selected. If the user selected a numeric id instead, use that exact numeric id. The preview may show both slug and numeric id, but the later command must use only `COMMAND_TARGET`.

3. Ask for exact confirmation before deleting:

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

4. Run exactly one delete command using the same target string after approval:

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
- NEVER switch apps delete target after the preview. Keep `COMMAND_TARGET` identical.
- NEVER echo internal endpoint URLs of cross-team apps even if visible in stdout.

## Additional Resources

For Korean trigger lexicon (apps intent): `../deploy/references/nl-lexicon.md`.
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For privacy filter rules (cross-team scope, NFKC normalize): see the redact subcommand in `axhub-helpers` and PLAN §16.17.
