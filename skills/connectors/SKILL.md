---
name: connectors
description: '이 스킬은 사용자가 axhub 외부 데이터베이스 커넥터를 등록·수정·삭제하거나 자격증명을 갱신하고 싶어할 때 사용해요. "Postgres 데이터베이스 연결하고 싶어", "DB 연결", "데이터베이스 연결", "커넥터 추가", "커넥터 만들", "postgres 연결", "mysql 연결", "외부 DB 붙여", "DB 자격증명", "커넥터 삭제", "connector", "connect database", "add connector", "db credentials", 또는 axhub 데이터 커넥터 관리 의도에서 활성화해요. 로컬 앱 코드에 pg 패키지를 붙이거나 DATABASE_URL 코드를 수정하는 흐름이 아니라 AXHub 외부 데이터 커넥터 관리 흐름이에요.'
examples:
  - utterance: "Postgres 데이터베이스 연결하고 싶어"
    intent: "manage database connector"
  - utterance: "DB 연결해"
    intent: "manage database connector"
  - utterance: "커넥터 추가"
    intent: "manage database connector"
  - utterance: "postgres 연결"
    intent: "manage database connector"
  - utterance: "add connector"
    intent: "manage database connector"
  - utterance: "db credentials update"
    intent: "manage database connector credentials"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# External data connectors

외부 DB 커넥터를 등록·수정·삭제하고 자격증명을 갱신해요. credentials 는 로컬 파일 digest 로 고정하고, mutation 은 preview 와 명시 확인 뒤에만 `--execute` 해요.

## Steps

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop natural-language contract.** 일반 사용자가 `Postgres 데이터베이스 연결하고 싶어`처럼 말하면 로컬 앱 코드 수정으로 우회하지 않고 AXHub 외부 데이터베이스 연결 설정으로 시작해요. 첫 visible chat 문장은 정확히 `데이터베이스 연결을 준비할게요.` 로 시작해요. 그 뒤에도 `workflow`, `워크플로`, skill 이름, slash command, route label 을 말하지 않아요. `List dir`, `server.js`, `package.json`, `pg 설치`, `DATABASE_URL env var 로 연결`, `catalog`, `A/B`, `connector 조회` 같은 구현 분기나 로컬 파일 탐색을 사용자에게 보이지 않아요. 부족한 정보는 사람 말투로 짧게 물어요: 커넥터 이름, workspace, 엔진, host, port, database, username, SSL 모드, credentials 파일/안전한 입력 방식. 비밀값은 채팅 평문으로 받지 않아요. 로그인 확인 결과에는 계정 이메일, raw user id 를 쓰지 말고 `로그인되어 있어요`처럼 상태만 말해요. Bash title 은 `커넥터 상태 확인`, `커넥터 목록 확인`, `커넥터 변경 준비`, `커넥터 변경 실행` 같은 한국어만 써요. 생성·수정·삭제는 preview 와 명시적 승인 전에는 실행하지 않아요.

**인증/컨텍스트 확인.** 작업을 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s
' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d	%s
",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
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

**Tenant grounding.** 커넥터는 tenant-scoped 예요. 사용자가 tenant 를 명시하지 않았으면 preflight 의 active team 만 사용하고, active team 이 없으면 실행을 멈춰요. `tenants[]` 첫 항목을 추측해서 쓰지 않아요.

```bash
# tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
if [ -z "$TENANT" ]; then
  echo "현재 workspace 를 특정할 수 없어요. 먼저 workspace 목록을 확인하거나 AXHUB_TENANT 를 명시해요." >&2
  exit 64
fi
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "작업 확인", status: "in_progress", activeForm: "작업 고르는 중" },
     { content: "엔진·설정 준비", status: "pending", activeForm: "설정 확인 중" },
     { content: "자격증명 파일 digest 준비", status: "pending", activeForm: "secret 파일 확인 중" },
     { content: "preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "실행 중" },
     { content: "결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   작업을 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.

1. **엔진과 현재 커넥터를 확인해요.**

   ```bash
   axhub engines list --json
   axhub connectors list --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --enabled-only --json
   axhub connectors discover "$CONNECTOR_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --json
   ```

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

2. **작업 선택.** 비대화형 기본은 `list`예요.

   ```json
   {"questions":[{"question":"커넥터 작업을 골라요","header":"커넥터","options":[{"label":"목록 보기","description":"read-only 로 확인해요"},{"label":"생성/수정","description":"설정과 자격증명을 준비해요"},{"label":"삭제","description":"삭제 preview 를 준비해요"}]}]}
   ```

3. **config 와 credentials 를 분리해요.** config 는 로컬 파일이나 inline JSON digest 로 고정하고, credentials 는 로컬 파일 digest 로만 고정해요. stdin 은 PreToolUse 가 payload 를 볼 수 없어 destructive 실행에 쓰지 않아요.

4. **mutation 명령.**

   Approval context 은 helper parser 와 같은 action/context 로 맞춰요: create 는 `{name,tenant,engine,source,config_file,config_digest,credentials_file,credentials_digest}`, update 는 `{connector_id,tenant,fields,source?,config_file?,config_digest?,description_digest?,enabled?,disabled?}`, credentials 는 `{connector_id,tenant,source,credentials_file,credentials_digest}`, delete 는 `{connector_id,tenant}` 예요. 파일 payload 는 preview 직후 `shasum -a 256` 으로 digest 를 계산하고 같은 digest 로 확인해요.

   ```bash
   CFG_DIGEST="sha256:$(shasum -a 256 cfg.json | awk '{print $1}')"
   CREDS_DIGEST="sha256:$(shasum -a 256 creds.json | awk '{print $1}')"
   axhub connectors create --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --name "$NAME" --engine postgres --config-file cfg.json --credentials-file creds.json --execute --json
   axhub connectors update "$CONNECTOR_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --config-file cfg.json --enabled --execute --json
   axhub connectors credentials-set "$CONNECTOR_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --credentials-file creds.json --execute --json
   axhub connectors delete "$CONNECTOR_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --execute --json
   ```


   Registry-backed confirmation questions used by mutation previews:

   - `이 커넥터를 변경할까요?`

## NEVER

- NEVER credentials 를 argv, telemetry, chat 에 평문으로 넣지 않아요.
- NEVER `--credentials-stdin` 을 destructive approval 경로에 쓰지 않아요. PreToolUse 가 stdin payload digest 를 검증할 수 없어요.
- NEVER 비대화형에서 커넥터 생성/수정/삭제를 자동 실행하지 않아요.
