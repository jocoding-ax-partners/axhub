---
name: env
description: '이 스킬은 사용자가 axhub 앱의 환경변수를 보거나 추가하거나 삭제하고 싶어할 때 사용해요. 다음 표현에서 활성화: "환경 변수", "환경 변수 확인", "환경변수", "환경변수 뭐 있어", "환경변수 추가", "api 키", "API 키 등록", "DB URL 추가", "env 봐", "env 삭제", "env 추가", "secret 추가", "api key", "database url", "db url", "env list", "env var", "secret", 또는 axhub 앱의 env var 조회/변경 의도. set 은 --from-stdin 으로만 받아 argv 노출을 막아요.'
examples:
  - utterance: "환경 변수"
    intent: "manage axhub environment variables"
  - utterance: "환경 변수 확인"
    intent: "manage axhub environment variables"
  - utterance: "api key"
    intent: "manage axhub environment variables"
  - utterance: "database url"
    intent: "manage axhub environment variables"
  - utterance: "환경변수"
    intent: "manage axhub environment variables"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Env

axhub 앱 환경변수 조회와 변경을 안전하게 처리해요. secret value 는 argv, shell history, telemetry, debug log 에 남기지 않아요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To manage env vars:

## Claude Desktop 계약

사용자가 `환경변수 뭐 있어?`, `환경 변수 확인해줘`, `env 봐`처럼 조회만 묻는 경우에는 첫 visible chat 문장을 정확히 `환경변수를 확인할게요.`로 말한 뒤, Bash tool title을 정확히 `환경변수 확인`으로 설정하고 아래 명령을 한 번만 실행해요.

```bash
axhub-helpers env-summary --user-utterance "<방금 사용자 문장>"
```

이 조회 흐름에서는 셸 환경변수, `.env` 파일, repo 파일, plugin source, git history, `.omc`, `.claude`, QA 결과 파일을 읽지 않아요. raw value, secret value, JSON field name, preflight, 내부 라벨, 원시 명령명을 사용자에게 쓰지 말고 helper stdout의 한국어 문장을 그대로 답해요. 값은 secret 플래그와 무관하게 기본적으로 표시하지 않아요.

For ordinary Claude Desktop env questions, stop after this step. Do not show raw values or secret values.

추가/수정/삭제 의도(`환경변수 추가`, `API 키 등록`, `env delete` 등)는 아래 전체 workflow를 사용하고, 값은 반드시 stdin/동의 흐름으로 처리해요.

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

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "앱과 auth 컨텍스트 확인", status: "in_progress", activeForm: "컨텍스트 확인 중" },
     { content: "env 작업 분기", status: "pending", activeForm: "작업 고르는 중" },
     { content: "안전한 CLI 호출", status: "pending", activeForm: "env 처리 중" },
     { content: "마스킹된 결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **preflight 와 current app 을 확인해요.** `auth_ok` 가 false 면 auth skill 로 넘겨요. 앱이 없으면 `axhub apps list --json` 또는 resolve helper 로 후보를 좁혀요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `조회만` 예요.

2. **작업을 분기해요.**

   ```json
   {
     "question": "어떤 환경변수 작업을 할까요?",
     "header": "환경변수",
     "options": [
       {"label": "조회만", "value": "list", "description": "값은 가리고 환경변수 이름 목록만 봐요"},
       {"label": "추가 또는 수정", "value": "set", "description": "값은 안전하게 입력으로만 전달해요"},
       {"label": "삭제", "value": "delete", "description": "이름 확인과 동의를 받고 지워요"}
     ]
   }
   ```

3. **조회는 read-only 로 실행해요.**

   ```bash
   axhub env list --app "$APP" --json
   axhub env get "$KEY" --app "$APP" --json
   ```

4. **set 은 stdin 만 써요.**

   ```bash
   printf %s "$VALUE" | axhub env set "$KEY" --app "$APP" --from-stdin --json
   ```

   실행 전 `preview confirmation` 에 `action=env_set`, top-level `app_id`, `context={key}` 를 사용해요. 값은 출력하지 말고 즉시 마스킹해요.

5. **delete 는 exact confirm 을 요구해요.**

   ```bash
   axhub env delete "$KEY" --app "$APP" --execute --json
   ```

   실행 전 `preview confirmation` 에 `action=env_delete`, top-level `app_id`, `context={key}` 를 사용해요.

## NEVER

- NEVER secret value 를 argv 로 넘기지 않아요.
- NEVER `set` 에서 `--from-stdin` 을 빼지 않아요.
- NEVER secret value 를 로그, 응답, telemetry 에 평문으로 남기지 않아요.
- NEVER subprocess 에서 set/delete 를 자동 선택하지 않아요.
