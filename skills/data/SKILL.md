---
name: data
description: '이 스킬은 AXHub 데이터 리소스 조회, 테이블 설명, 안전한 읽기, 집계 인사이트, 코드 스니펫 생성에 사용해요. 다음 표현에서 활성화: "orders 데이터 조회해줘", "데이터 조회해줘", "테이블 설명", "SQL로 읽어줘", "snippet 만들어줘", "describe table", "generate snippet", "인사이트 뽑아줘", "분석해줘", "집계해줘", "통계 내줘".'
examples:
  - utterance: "orders 데이터 조회해줘"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "이 테이블 읽는 python snippet 만들어줘"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "describe snowflake analytics orders table"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "generate a TypeScript snippet for this catalog resource"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "SQL로 읽어줘"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "이 데이터로 인사이트 뽑아줘"
    intent: "run aggregate SQL via catalog invoke and narrate insights"
  - utterance: "부서별 인원 집계해줘"
    intent: "run aggregate SQL via catalog invoke and narrate insights"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Data

AXHub 데이터 리소스를 CLI-only 방식으로 탐색하고, first live read approval 뒤에 read-only invoke (단순 조회 + 집계 인사이트) 또는 snippet 을 만들어줘요. 인사이트 요청은 `axhub data list` 로 헤매지 말고 catalog 로 connector/path 를 해석한 뒤 `catalog invoke --action read` 로 집계 SQL 을 돌려서 한국어로 요약해줘요.

## Routing boundary — dynamic app tables are not catalog data

- `orders 동적 테이블 만들고 title:text 컬럼 추가해`, `앱 테이블 스키마 변경`, `컬럼 추가`, `행 넣어`, `grant/revoke` 처럼 앱 동적 테이블 DDL/DML 의도가 보이면 이 skill 을 즉시 멈추고 `skills/tables/SKILL.md` 를 로드해요.
- 이 skill 에서 `axhub tables` 명령을 실행하지 않아요. 테이블 스키마·행·권한 작업은 tables skill 의 preview/approval/`--execute` 규칙으로 처리해요.
- catalog connector/path 조회, safe SQL read, aggregate insight, snippet 생성만 이 skill 의 범위예요.

## Steps

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop natural-language contract.** 일반 사용자가 `orders 데이터 조회해줘`처럼 말하면 첫 visible chat 문장은 정확히 `데이터 리소스를 확인할게요.` 로 시작해요. 그 뒤에도 `workflow`, `워크플로`, skill 이름, slash command, route label 을 말하지 않아요. `preflight`, `catalog 조회`, `catalog 비어있음`, `connector 목록`, `catalog kinds`, `governance`, `path 추측`, raw JSON field, raw ID, raw email, account scope, raw app slug, English tool title, A/B 구현 라벨을 사용자에게 보이지 않아요. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요`처럼 상태만 말해요. Bash title 은 `로그인 상태 확인`, `데이터 리소스 확인`, `데이터 설명`, `실데이터 확인`, `스니펫 준비` 같은 한국어만 써요. 연결된 데이터 리소스가 없으면 `현재 연결된 데이터 리소스를 찾지 못했어요. 먼저 데이터베이스 연결을 만들어야 해요.`처럼 말하고, 내부 목록·종류·빈 결과를 덧붙이지 않아요. live read 는 대상, 컬럼, row limit, query shape 를 보여준 뒤 명시적 승인 전에는 실행하지 않아요.

**인증/컨텍스트 확인.** 작업을 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
SAFE_PREFLIGHT_JSON=$(printf '%s' "$PREFLIGHT_JSON" | jq 'del(.user_email, .user_id, .email, .account_email)' 2>/dev/null)
[ -n "$SAFE_PREFLIGHT_JSON" ] || SAFE_PREFLIGHT_JSON='{"auth_ok":false,"auth_error_code":"preflight_summary_unavailable"}'
echo "$SAFE_PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 계정 이메일이나 raw user id 는 사용자에게 쓰지 않아요. 치명적이지 않으면 계속 진행해요.

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

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "catalog context 동기화", status: "in_progress", activeForm: "catalog context 동기화 중" },
     { content: "resource 검색과 describe", status: "pending", activeForm: "resource 확인 중" },
     { content: "first live read approval 확인", status: "pending", activeForm: "live read 안전 확인 중" },
     { content: "read invoke 또는 snippet 생성", status: "pending", activeForm: "결과 생성 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **Sync local catalog snapshot.** Use the helper, not MCP server config. Default output is the git toplevel `.axhub/`; use `--out` only when the user gives a separate workspace.

   Before the first sync, check whether `.axhub/` already exists. If it does not exist, explain that sync will create `.axhub/AXHUB.md`, `.axhub/AXHUB_TARGET`, private `.axhub/catalog.json`, and append `.axhub/catalog.json` to `.gitignore`; then ask once before mutating files.

   ```json
   {
     "questions": [{
       "question": "catalog context 를 처음 만들까요?",
       "header": "Catalog",
       "multiSelect": false,
       "options": [
         {"label": "Skip sync", "description": "파일을 만들지 않고 catalog search/get 또는 snippet dry-run 만 진행해요."},
         {"label": "Create context", "description": ".axhub 규칙 파일과 private catalog snapshot 을 만들어요."}
       ]
     }]
   }
   ```

   In non-interactive mode, use `Skip sync`. If the user skips, do not run `axhub-helpers sync`; continue with live `axhub catalog search/get` only when the request can be answered without local snapshot writes.

   ```bash
   axhub-helpers sync --target auto --json
   axhub-helpers sync --target local-python --json
   ```

   If sync returns `ambiguous_target`, choose the closest runtime from project evidence. If it returns `identity_changed`, stop before overwrite unless the user explicitly confirms the new principal; non-interactive safe default is skip overwrite.

2. **Search and describe resources before any live read.** Prefer a broad catalog search, then a precise describe. Keep connector/path from catalog output only.

   ```bash
   axhub catalog search --json --limit 200
   axhub catalog get --connector <connector> --path <path> --json
   ```

   Summarize only what is needed: connector, path, kind, allowed_columns, masked columns, row policy, and deny_reason when present. Do not print full `.axhub/catalog.json`. In Claude Desktop visible text, do not say `catalog search`, `catalog 비어있음`, `connector 목록`, or `catalog kinds`; translate them to user-facing wording such as `연결된 데이터 리소스`, `조회 가능한 컬럼`, and `현재 찾은 리소스`.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 각 질문 별 safe_default.

3. **Confirm first live read.** Before the first live read for a resource in this session, show connector, path, SQL, row limit, allowed_columns, masked fields, and why the read is needed. Ask once.

   ```json
   {
     "questions": [{
       "question": "실데이터 read 를 실행할까요?",
       "header": "Live read",
       "multiSelect": false,
       "options": [
         {"label": "Dry-run only", "description": "실데이터를 읽지 않고 SQL, allowed_columns, snippet 만 보여줘요."},
         {"label": "Run read", "description": "표시한 connector/path/SQL/row limit 그대로 한 번만 실행해요."}
       ]
     }]
   }
   ```

   In non-interactive mode, use `Dry-run only`. If the server denies with `allowed:false` or `deny_reason`, show that reason and NEVER retry denied.

4. **Invoke read safely after approval.** Live reads must include `--execute --json`, an explicit row limit, and read-only SQL. Keep row limit small unless the user explicitly asks for less-restricted output. 인사이트/집계 요청이면 같은 read 경로로 집계 SQL (GROUP BY / COUNT / AVG / SUM) 을 돌려요 — `allowed_columns` 안의, 마스킹 안 된 컬럼만 집계해요.

   ```bash
   # 단순 조회
   axhub catalog invoke --connector <connector> --path <path> --action read --sql '<SELECT ...>' --row-limit 100 --execute --json
   # 집계 인사이트 (예: 부서별 인원)
   axhub catalog invoke --connector <connector> --path <path> --action read --sql 'SELECT department, COUNT(*) AS headcount FROM employees GROUP BY department ORDER BY headcount DESC' --row-limit 100 --execute --json
   ```

   Parse error output through the generated catalog empathy copy. For catalog internal errors, do not retry automatically; re-check `allowed_columns` with `catalog get` first.

5. **Generate snippets from described catalog context.** Use helper templates so auth posture stays target-aware.

   ```bash
   axhub-helpers snippet --mode A --language typescript --target web-axhub --connector <connector> --path <path> --sql '<SELECT ...>' --allowed-columns <csv> --masked <csv>
   axhub-helpers snippet --mode B --language python --target local-python --connector <connector> --path <path> --sql '<SELECT ...>' --allowed-columns <csv> --masked <csv>
   axhub-helpers snippet --mode B --language shell --target local-bash --connector <connector> --path <path> --sql '<SELECT ...>' --allowed-columns <csv>
   ```

   Mode A uses browser cookie auth with `credentials: 'include'`. Mode B uses `AXHUB_PAT` as `X-Api-Key`. Local bash uses `axhub catalog invoke --execute --json` via CLI/keychain and does not print PATs.

6. **Final response.** Return the selected connector/path, row limit, allowed_columns, masked handling, whether live read ran, and exact commands or snippet produced. If no live read ran, say dry-run only. In Claude Desktop, keep the final response human-readable: no raw account identity, no raw JSON, no `preflight`, no `catalog 조회`, no `connector 목록`, no `catalog kinds`, no internal governance/path-guessing jargon, no route labels, no skill names.

   **인사이트/집계 요청이면** raw JSON 을 그대로 쏟지 말고 humanize 해요: 핵심 수치를 GFM 마크다운 표로 (예: 부서 / 인원, 월 / 합계 — 셀이 길면 ~50자에서 잘라요), 표 아래 1~3문장으로 가장 큰 값·편중·추세·빈 그룹 같은 발견을 짚어줘요. 마스킹 컬럼 (`mask_hint` 있는 값) 은 집계가 null/●●● 일 수 있다고 안내해요.

## Identity-change question

Use this only after `axhub-helpers sync` returns `identity_changed`.

```json
{
  "questions": [{
    "question": "인증 주체가 바뀌었어요. catalog 를 새로 쓸까요?",
    "header": "Identity",
    "multiSelect": false,
    "options": [
      {"label": "Skip overwrite", "description": "기존 catalog 를 보존하고 새 주체 확인을 요청해요."},
      {"label": "Overwrite", "description": "명시 동의가 있을 때만 --allow-identity-change 로 새로 써요."}
    ]
  }]
}
```

## NEVER

- NEVER governance bypass: do not invent policies, scopes, row access, or masked output.
- NEVER path guessing: use connector/path from `catalog search` or `catalog get` only.
- NEVER retry denied: if `allowed:false`, `deny_reason`, 66, or catalog internal error appears, stop and show the reason.
- NEVER run a live read without first live read approval in the current session/resource.
- NEVER omit `--execute --json` for a live `catalog invoke`.
- NEVER exceed the stated row limit or `allowed_columns`.
- NEVER print `.axhub/catalog.json` or hardcode PATs in snippets.
- NEVER 인사이트를 `axhub data list` 로 뽑으려고 헤매기 — connector/path 는 catalog search/get 으로 해석하고 집계는 `axhub catalog invoke --action read` 로 해요.
- NEVER raw JSON 집계 결과를 그대로 출력 — 한국어 인사이트 표 + 발견으로 humanize 해요.

## Additional Resources

- `../deploy/references/error-empathy-catalog.generated.md` — generated exit-code copy from `crates/axhub-helpers/data/catalog.json`.
