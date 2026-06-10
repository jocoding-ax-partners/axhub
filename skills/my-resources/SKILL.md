---
name: my-resources
description: '이 스킬은 사용자가 본인이 접근 가능한 axhub 리소스 전체 인벤토리를 한눈에 보고 싶을 때 사용해요. 다음 표현에서 활성화: "내 리소스", "내 리소스 봐", "내 리소스 보여", "내 리소스 보여줘", "내 리소스 목록", "리소스 봐", "리소스 보여", "리소스 보여줘", "리소스 목록", "리소스 뭐 있어", "리소스 조회", "뭐 접근 가능", "뭐 접근 가능해", "내가 뭐 봐", "내가 뭐 봐", "내 자산", "내 자산 봐", "내 스코프", "스코프 봐", "권한 봐", "권한 뭐 있어", "접근 가능한 거", "접근 권한", "접근 권한 봐", "쓸 수 있는 거", "available", "inventory", "list resources", "my resources", "my access", "my scope", "what can I access", "what do I have", "what i have access to", "access", "resources", "show my resources", 또는 사용자 scope 의 통합 리소스 카탈로그 조회. team scope 필터로 cross-tenant 데이터 노출 차단해요. 7 family (tenants / apps / members / engines / connectors / resources / catalog kinds) 를 병렬 호출해서 한 응답에 한국어 GFM 표(테이블) + drill-down hint 로 자세히 렌더해요.'
examples:
  - utterance: "내 리소스 보여줘"
    intent: "list accessible axhub resources"
  - utterance: "뭐 접근 가능해"
    intent: "list accessible axhub resources"
  - utterance: "what can I access"
    intent: "list accessible axhub resources"
  - utterance: "내 스코프 봐"
    intent: "list accessible axhub resources"
  - utterance: "inventory"
    intent: "list accessible axhub resources"
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# My Resources

사용자가 접근 가능한 axhub 리소스를 7개 family (tenants / apps / members / engines / connectors / resources / catalog kinds) 로 한 번에 조회해서 한국어 GFM 표(테이블)로 자세히 렌더해요 (bullet 요약 아님 — Step 5 표 1·2·3). 읽기 전용, mutation 경로 없음, F4 privacy 로 cross-tenant 데이터 차단해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To list resources:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `axhub_bin_invalid` 는 `AXHUB_BIN` 환경변수가 잘못된 경로 (`cli_resolved_path` 값) 를 가리키는 상태라 재설치 대신 `unset AXHUB_BIN` 후 새 세션 재시도 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

**Tenant 선택 (axhub-tenant-picker:L1).** axhub-helpers `tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요. fence 간 env 는 휘발하므로 결정된 tenant 를 캐시에 영속화해서 다음 fence 가 re-read 해요. 명시 `AXHUB_TENANT` override 가 있으면 helper 를 건너뛰어요.

```bash
# axhub-tenant-picker:L1 — thin resolver (위험 로직은 Rust axhub-helpers tenant-resolve 가 소유)
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
# Precedence 1: 명시 AXHUB_TENANT env override → helper 호출 skip
if [ -z "${AXHUB_TENANT:-}" ]; then
  HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
  if [ -n "$HELPER" ] && [ ! -x "$HELPER" ] && [ -x "${HELPER}.exe" ]; then HELPER="${HELPER}.exe"; fi
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null || command -v axhub-helpers.exe 2>/dev/null)"
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers* "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers*; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
  TENANT_JSON=$([ -n "$HELPER" ] && "$HELPER" tenant-resolve --json 2>/dev/null)
  [ -n "$TENANT_JSON" ] || TENANT_JSON='{}'
  AXHUB_TENANT=$(printf '%s' "$TENANT_JSON" | jq -r '.tenant // empty' 2>/dev/null || true)
  _NEEDS_PICK_RAW=$(printf '%s' "$TENANT_JSON" | jq -r '.needs_pick // false' 2>/dev/null || echo false)
  # no-loop: needs_pick 는 비어있지 않은 resolve 에서만 true; 빈/부재 helper → false (재프롬프트 안 함)
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    CANDIDATES_JSON=$(printf '%s' "$TENANT_JSON" | jq -c '.candidates // []' 2>/dev/null || echo '[]')
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      # non-TTY: active fallback + 경고 (R4 fail-wrong guard — bash 위치 필수)
      AXHUB_TENANT=$(printf '%s' "$CANDIDATES_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
      echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant($AXHUB_TENANT)로 진행해요"
    else
      NEEDS_PICK="true"
    fi
  fi
fi
# 결정된 tenant 영속화 (fence 간 source of truth) — needs_pick 대기 중엔 미기록(L2 가 기록)
if [ -n "${AXHUB_TENANT:-}" ] && [ "$NEEDS_PICK" = "false" ]; then
  mkdir -p "$(dirname "$TENANT_CACHE")"
  printf '{"tenant":"%s","source":"resolved","ts":%s}\n' "$AXHUB_TENANT" "$(date +%s 2>/dev/null || echo '0')" > "$TENANT_CACHE"
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요. 구버전·부재 helper 면 빈 값 → active tenant 로 진행하고, picker 는 helper 업데이트 후 돌아와요.

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

1. **인증/Scope 확정.** preflight 결과에서 `auth_ok` 와 `current_team_id` 를 추출해요. 미인증이면 즉시 안내 후 종료:

   ```bash
   AUTH_OK=$(echo "$PREFLIGHT_JSON" | jq -r '.auth_ok // false')
   TEAM_ID=$(echo "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')
   if [ "$AUTH_OK" != "true" ]; then
     echo '{"systemMessage":"로그인이 필요해요. 다시 로그인해줘라고 말하면 이어서 도와줄게요."}'
     exit 0
   fi
   ```

2. **한 번에 요약을 만들어요.** Desktop 에서 JSON shape 를 추리하느라 여러 번 우회하지 마세요. 아래 Bash 블록 하나만 실행해요. 이 블록은 known response shapes (`array`, `data[]`, `data.data[]`, `data.items[]`, `items[]`, `resources[]`) 를 직접 normalize 하고, 바로 렌더 가능한 Markdown 을 출력해요. 사용자가 보게 되는 진행 문구는 "리소스 인벤토리 요약 생성" 한 단계면 충분해요.

   ```bash
   INV_TMP=$(mktemp -d -t axhub-inv-XXXX)
   trap 'rm -rf "$INV_TMP"' EXIT

   ( axhub tenants list --json    >"$INV_TMP/tenants.json"    2>"$INV_TMP/tenants.err"    ; echo $? >"$INV_TMP/tenants.code"    ) &
   ( axhub apps mine --json       >"$INV_TMP/apps.json"       2>"$INV_TMP/apps.err"       ; echo $? >"$INV_TMP/apps.code"       ) &
   ( axhub members list --json    >"$INV_TMP/members.json"    2>"$INV_TMP/members.err"    ; echo $? >"$INV_TMP/members.code"    ) &
   ( axhub engines list --json    >"$INV_TMP/engines.json"    2>"$INV_TMP/engines.err"    ; echo $? >"$INV_TMP/engines.code"    ) &
   ( axhub connectors list --json >"$INV_TMP/connectors.json" 2>"$INV_TMP/connectors.err" ; echo $? >"$INV_TMP/connectors.code" ) &
   ( axhub resources list --json  >"$INV_TMP/resources.json"  2>"$INV_TMP/resources.err"  ; echo $? >"$INV_TMP/resources.code"  ) &
   ( axhub catalog kinds --json   >"$INV_TMP/catalog.json"    2>"$INV_TMP/catalog.err"    ; echo $? >"$INV_TMP/catalog.code"    ) &
   wait

   for name in tenants apps members engines connectors resources catalog; do
     [ -s "$INV_TMP/$name.json" ] || printf '{}' >"$INV_TMP/$name.json"
     [ -s "$INV_TMP/$name.code" ] || printf '1' >"$INV_TMP/$name.code"
   done

   jq -nr \
     --arg team_id "$TEAM_ID" \
     --arg tenants_code "$(cat "$INV_TMP/tenants.code")" \
     --arg apps_code "$(cat "$INV_TMP/apps.code")" \
     --arg members_code "$(cat "$INV_TMP/members.code")" \
     --arg engines_code "$(cat "$INV_TMP/engines.code")" \
     --arg connectors_code "$(cat "$INV_TMP/connectors.code")" \
     --arg resources_code "$(cat "$INV_TMP/resources.code")" \
     --arg catalog_code "$(cat "$INV_TMP/catalog.code")" \
     --slurpfile tenants "$INV_TMP/tenants.json" \
     --slurpfile apps "$INV_TMP/apps.json" \
     --slurpfile members "$INV_TMP/members.json" \
     --slurpfile engines "$INV_TMP/engines.json" \
     --slurpfile connectors "$INV_TMP/connectors.json" \
     --slurpfile resources "$INV_TMP/resources.json" \
     --slurpfile catalog "$INV_TMP/catalog.json" '
     def root($v): $v[0] // {};
     def arr($v):
       if ($v|type) == "array" then $v
       elif ($v.data|type) == "array" then $v.data
       elif ($v.data.data|type) == "array" then $v.data.data
       elif ($v.data.items|type) == "array" then $v.data.items
       elif ($v.items|type) == "array" then $v.items
       elif ($v.resources|type) == "array" then $v.resources
       else [] end;
     def clip: tostring | gsub("[\r\n|]"; " ") | if length > 50 then .[0:47] + "..." else . end;
     def top($xs): ($xs[0:4] | map(clip) | join(", "));
     def count($xs; $code): if $code == "0" then "\($xs|length)개" else "조회 불가" end;
     def detail($xs; $code):
       if $code == "0" then top($xs)
       elif $code == "65" then "(미인증 - 다시 로그인해줘)"
       elif $code == "67" then "(관리자 인증 필요 - 다시 로그인해줘)"
       elif $code == "68" then "(scope 외)"
       else "(조회 불가 - exit \($code))" end;
     def scoped($xs):
       if $team_id == "" then $xs
       else [$xs[] | select((.tenant_id // .team_id // .app.tenant_id // "") as $tid | $tid == "" or $tid == $team_id)] end;
     (arr(root($tenants))) as $tenant_rows |
     (scoped([arr(root($apps))[] | (.app // .) | select((.deleted_at // null) == null)])) as $app_rows |
     (scoped(arr(root($members)))) as $member_rows |
     (arr(root($engines))) as $engine_rows |
     (scoped(arr(root($connectors)))) as $connector_rows |
     (scoped(arr(root($resources)))) as $resource_rows |
     (arr(root($catalog))) as $catalog_rows |
     ($tenant_rows | map((.tenant_slug // .slug // .name // .tenant_id // "unknown") + " (" + (.role // .member_role // "member") + ")")) as $tenant_top |
     ($app_rows | map((.slug // .name // .app.slug // .app.name // "unknown") + " [" + (.operating_status // .status // .last_deployment_status // "unknown") + "]")) as $app_top |
     ($member_rows | group_by(.role // "member") | map((.[0].role // "member") + " " + (length|tostring))) as $member_top |
     ($engine_rows | map(.kind // .engine // .name // "unknown")) as $engine_top |
     ($connector_rows | map((.name // .slug // .id // "unknown") + " (" + (.engine // .kind // "connector") + ")")) as $connector_top |
     ($resource_rows | map(.name // .path // .id // "resource")) as $resource_top |
     ($catalog_rows | map((.kind // .name // "kind") + (if (.invokable // false) then "(invokable)" else "" end))) as $catalog_top |
     "## 접근 가능 리소스 - scope=내 계정" ,
     "",
     "| 리소스 | 개수 | 상세 |",
     "|---|---:|---|",
     "| 팀 (tenants) | \(count($tenant_rows; $tenants_code)) | \(detail($tenant_top; $tenants_code)) |",
     "| 앱 (apps) | \(count($app_rows; $apps_code)) | \(detail($app_top; $apps_code)) |",
     "| 멤버 (members) | \(count($member_rows; $members_code)) | \(detail($member_top; $members_code)) |",
     "",
     "| 리소스 | 개수 | 상세 |",
     "|---|---:|---|",
     "| Engines | \(count($engine_rows; $engines_code)) | \(detail($engine_top; $engines_code)) |",
     "| Connectors | \(count($connector_rows; $connectors_code)) | \(detail($connector_top; $connectors_code)) |",
     "| Resources | \(count($resource_rows; $resources_code)) | \(detail($resource_top; $resources_code)) |",
     "| Catalog kinds | \(count($catalog_rows; $catalog_code)) | \(detail($catalog_top; $catalog_code)) |",
     "",
     "앱별 자원(env, tables, apis)은 앱을 고른 뒤 이어서 물어보면 돼요.",
     "자세히 보고 싶으면 `내 앱 보여줘`, `환경변수 봐`, `깃허브 연결`, `배포 상태 봐`처럼 말해요."
   '
   ```

3. **결과를 그대로 전달해요.** 위 Bash 출력은 이미 최종 Markdown 이에요. JSON path 재확인, 추가 jq probing, 전체 raw JSON 표시, family별 재시도 loop 를 하지 마세요. 일부 family 가 실패해도 표의 해당 행에만 `조회 불가`를 표시하고 종료해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 은 대화형 질문 prompt 를 호출하지 않아요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 와 대화형 환경 모두에서 동일하게 동작해요. `tests/fixtures/ask-defaults/registry.json` 의 my-resources 항목은 no-op stub (질문 없음).

## NEVER

- NEVER 한 family 실패로 전체 인벤토리 abort. 항상 7 family 모두 한 응답에 렌더 (성공/실패 혼합).
- NEVER 표를 bullet 요약("- 앱: 10개 …" 같은)으로 축약. Step 5 의 GFM 표(표 1·2·3)를 그대로 그려요 — 사용자가 테이블 형식으로 자세히 보길 원해요.
- NEVER cross-tenant 데이터 (team_id != $TEAM_ID) 화면에 표시. F4 privacy 위반.
- NEVER mutation 호출 (`create` / `update` / `delete` / `set-role` / `bulk-register`) 진입. 이 SKILL 은 read-only.
- NEVER per-app drill (`env list --app X` / `tables list --app X`) 자동 진행. 사용자가 명시적으로 앱 선택 후 다른 SKILL 로 위임.
- NEVER tmp dir cleanup 누락. `trap "rm -rf '$INV_TMP'" EXIT` 강제.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — exit-code 별 4-part 한국어 fallback.
- `../apps/SKILL.md` — apps drill-down (앱 단일 family 상세 + 생성/삭제).
- `../status/SKILL.md` — 배포 상태 추적 (haiku read-only 동일 패턴 참고).
