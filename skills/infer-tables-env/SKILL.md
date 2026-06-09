---
name: infer-tables-env
description: '이 스킬은 사용자가 자기 앱 소스코드를 분석해서 필요한 axhub 동적 테이블과 환경변수를 추론·추천받고 싶어할 때 사용해요. 다음 표현에서 활성화: "내 코드 분석해서 테이블 추천", "필요한 테이블 뭐야", "필요한 환경변수 추론해줘", "코드 분석해서 env 추천", "스키마 분석해줘", "프로젝트 스캔해서 추천", "어떤 테이블 필요해", "필요한 env 뭐 있어", "소스 분석 추천", "scan my project", "infer tables", "recommend env", 또는 소스 기반 테이블·env 추론 의도. 추천은 read-only 이고, 실제 생성/설정은 승인 후 tables·env 절차로 위임해요. 동적 테이블 CRUD 자체는 tables, env 변경 자체는 env 절차가 담당해요.'
examples:
  - utterance: "내 코드 분석해서 테이블 추천"
    intent: "infer tables and env from source code"
  - utterance: "필요한 환경변수 추론해줘"
    intent: "infer tables and env from source code"
  - utterance: "스키마 분석해줘"
    intent: "infer tables and env from source code"
  - utterance: "프로젝트 스캔해서 추천"
    intent: "infer tables and env from source code"
  - utterance: "infer tables"
    intent: "infer tables and env from source code"
  - utterance: "scan my project for tables and env"
    intent: "infer tables and env from source code"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Infer Tables Env

개발한 소스코드를 분석해서 이 앱에 필요한 axhub 동적 테이블(컬럼·타입·제약)과 환경변수 키를 근거와 함께 추천하고, 승인하면 tables·env 절차로 위임해서 만들어줘요. 추천 단계는 read-only 이고 시크릿 값은 절대 노출하지 않아요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**로컬 소스 검사 계약.** 이 스킬은 로컬 앱 소스(데이터 모델 파일, 마이그레이션, `.env.example`, config)를 read-only 로 직접 읽어 추론해요. 데이터 조회 절차와 달리 로컬 파일 검사가 본 목적이에요. 단, 어떤 시크릿 값이나 하드코딩된 시크릿 리터럴도 출력·복사하지 않아요.

**자동 제안(경량 넛지).** `init`/`deploy` 흐름에서 이 분석을 먼저 제안할 때는 무거운 전체 스캔을 돌리지 않고 "필요한 테이블·환경변수 추천해드릴까요?" 한 줄만 비차단으로 띄워요. 사용자가 수락하면 그때 아래 전체 분석을 실행하고, 거절하면 아무 부작용 없이 원래 흐름을 이어가요.

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
     { content: "앱과 인증 컨텍스트 확인",        status: "in_progress", activeForm: "컨텍스트 확인 중" },
     { content: "소스 분석해서 테이블·env 추론",  status: "pending",     activeForm: "소스 분석 중" },
     { content: "추천 표 보여주기",               status: "pending",     activeForm: "추천 만드는 중" },
     { content: "승인 시 적용 위임",              status: "pending",     activeForm: "적용하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **preflight 와 대상 앱을 확인해요.** `auth_ok` 가 false 면 인증 상태를 설명하고 `다시 로그인해줘`라고 말하면 된다고 안내해요. 분석(추천) 자체는 인증 없이도 read-only 로 할 수 있지만, 나중에 적용하려면 인증과 대상 앱이 필요해요. 대상 앱은 preflight 컨텍스트나 `axhub apps list --json` 으로 좁혀요.

2. **소스를 분석해서 추천을 만들어요(read-only).** 선언적 아티팩트를 우선 봐요: `schema.prisma`, Prisma/Alembic 마이그레이션 파일, `.env.example`/`.env.sample` 는 고신뢰예요. 흩어진 ORM 클래스·런타임 `getenv`/`process.env` 는 베스트에포트로 "검토 필요"로 표시해요. 각 추론마다 근거(파일·위치)를 함께 잡아요.

   - 테이블: 엔티티 이름 + 컬럼(이름·타입·제약 필수/고유/PK 추정). 소스 타입 → axhub 타입 매핑: 문자열→text, 정수·소수→number, 불리언→boolean, 날짜·시간→datetime, JSON·object→json. 모호하면 best-guess + "검토 필요".
   - 환경변수: 필요한 **키**와 시크릿 여부만 도출해요(값은 추론하지 않아요). 비시크릿 기본값이 소스에 드러나면(예: `getenv("PORT", "8000")`) 그 값을 미리 채울 후보로만 적어요.
   - 외부 연결 데이터 소스를 가리키는 모델은 앱 소유 동적 테이블이 아니므로 새 테이블로 추천하지 않아요.

3. **이미 설정됐는지 cross-check 해요.** 대상 앱이 있으면 아래로 비교해서 각 항목 상태를 정해요: 신규 / 이미 있음 / 검토 필요. 대상 앱이 없으면 상태는 "미확인"으로 두고 추천만 보여줘요.

   ```bash
   axhub tables list --app "$APP" --json
   axhub env list --app "$APP" --json
   ```

4. **추천을 표로 보여줘요(read-only).** 테이블 표(테이블·컬럼(타입·제약)·근거·상태)와 환경변수 표(키·시크릿?·기본값·근거·상태)를 한국어 GFM 표로 내고, 마지막에 커버리지 한 줄(분석한 것 / 미스캔 영역 / "검토 필요" N건)을 붙여요. 시크릿 키는 `기본값` 칸을 항상 `—` 로 둬요. 추론된 게 하나도 없으면 표 대신 "추론된 게 없어요"와 다음에 할 수 있는 일을 안내해요. 이 단계까지는 아무것도 바꾸지 않아요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `추천만` 이에요.

5. **무엇을 할지 분기해요.**

   ```json
   {
     "questions": [{
       "question": "추천 결과를 어떻게 할까요?",
       "header": "추천적용",
       "multiSelect": false,
       "options": [
         {"label": "추천만", "description": "추천만 보고 끝내요. 아무것도 만들지 않아요"},
         {"label": "적용", "description": "승인하면 tables·env 절차로 위임해서 만들어요"}
       ]
     }]
   }
   ```

6. **적용을 고르면 tables·env 절차로 인계해요.** 이 스킬은 무엇을 만들지 추천만 하고, 실제 생성·설정은 tables·env 절차가 각자 자기 안전 흐름(미리보기·동의·stdin 값 입력)으로 처리해요. 이 스킬 안에서 직접 테이블을 만들거나 env 를 설정하지 않아요. 인계 전에 `auth_ok` 와 대상 앱만 확인하고, 빠진 게 있으면 먼저 해결하도록 안내해요(추천은 read-only 로 계속 유효).

7. **`신규` 항목만 tables·env 절차에 넘기고 무엇을 넘겼는지 요약해요.** 멱등이라 `이미 있음` 은 빼고 `신규` 만 넘겨요.

   - 테이블: 추론한 테이블 이름과 컬럼(타입·제약)을 tables 절차에 넘겨서 만들어요. 미리보기·동의·생성·컬럼 추가는 tables 절차가 자기 흐름으로 처리해요.
   - 환경변수: 추론한 키 목록을 env 절차에 넘겨서 등록해요. 값은 추론하지 않으니, env 절차의 안전 입력(stdin)으로 사용자가 각 값을 넣거나 건너뛰어요. 비시크릿 기본값이 있으면 미리 채울 후보로만 알려줘요.

   인계가 끝나면 무엇을 tables·env 절차로 넘겼고 무엇이 만들어졌는지 항목별로 요약해요(만듦 / 건너뜀).

## NEVER

- NEVER 시크릿 값이나 하드코딩된 시크릿 리터럴을 추천·미리보기·로그에 평문으로 노출하지 않아요. 발견하면 "환경변수로 옮기세요"로만 플래그해요.
- NEVER 환경변수 값을 추론하거나 만들어내지 않아요 — 키와 시크릿 여부만 도출하고 값은 사용자 입력으로만 받아요.
- NEVER 사용자의 명시적 확인 없이 테이블이나 환경변수를 만들거나 바꾸지 않아요. 추천 단계는 read-only 예요.
- NEVER 새 mutation 경로를 만들지 않아요 — 적용은 항상 기존 tables·env 절차로 위임해요.
- NEVER 외부 연결 데이터 소스를 앱 소유 동적 테이블로 추천하지 않아요.
- NEVER 추천 결과를 파일로 저장하지 않아요 — 휘발성이라 다시 필요하면 소스를 재분석해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../tables/SKILL.md` · `../env/SKILL.md` — 승인 후 적용을 위임하는 대상 절차.
