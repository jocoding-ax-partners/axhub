---
name: workspace
description: '이 스킬은 사용자가 "내가 속한 팀", "내 팀", "내 소속", "내 워크스페이스"처럼 자신의 axhub 팀/워크스페이스/테넌트 membership 목록을 조회하고 싶어할 때 사용해요. 다음 표현에서 활성화: "내가 속한 팀이랑 워크스페이스 보여줘", "내가 속한 팀 보여줘", "내가 속한 워크스페이스", "내 팀이랑 워크스페이스", "내 워크스페이스", "내 워크스페이스 보여줘", "내 워크스페이스 목록", "내 소속", "내 소속 봐", "어느 워크스페이스", "워크스페이스", "워크스페이스 목록", "테넌트", "테넌트 목록", "workspace", "tenant", "my teams and workspaces", "my workspaces". 팀 멤버 초대/권한 변경 같은 관리는 team 스킬, 백엔드 endpoint/profile 전환은 profile 스킬이 담당해요.'
examples:
  - utterance: "내 워크스페이스 보여줘"
    intent: "list axhub workspaces"
  - utterance: "테넌트 목록"
    intent: "list axhub workspaces"
  - utterance: "내 소속 봐"
    intent: "list axhub workspaces"
  - utterance: "내가 속한 팀이랑 워크스페이스 보여줘"
    intent: "list axhub workspaces"
  - utterance: "workspace list"
    intent: "list axhub workspaces"
  - utterance: "my workspaces"
    intent: "list axhub workspaces"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: haiku
---

# Workspace and tenants

현재 사용자의 워크스페이스/테넌트 소속을 read-only 로 보여줘요. endpoint/profile 전환은 profile skill 로 넘겨요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

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

**Tenant grounding.** 조회 대상 tenant 는 사용자가 고른 `AXHUB_TENANT` 또는 preflight 의 active team 이에요. active team 이 없으면 `tenants list --all` 로 후보만 보여주고 특정 tenant 의 비공개 상세는 추측하지 않아요.

```bash
# tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "소속 확인", status: "in_progress", activeForm: "소속 확인 중" },
     { content: "목록 조회", status: "pending", activeForm: "목록 조회 중" },
     { content: "상세 요약", status: "pending", activeForm: "상세 요약 중" },
     { content: "다음 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **테넌트 소속을 조회해요.**

   ```bash
   # tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
   TENANT="${TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
   if [ -n "$TENANT" ]; then
     axhub tenants whoami --tenant "$TENANT" --json
     axhub tenants get "$TENANT" --json
   else
     axhub tenants whoami --json
   fi
   axhub tenants list --all --json
   ```

2. **현재/후보 workspace 를 요약해요.** slug/id/name/role 만 보여줘요.

3. **전환 요청은 profile skill 로 넘겨요.** 이 skill 은 조회만 담당해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 이 read-only skill 은 질문 없이 안전하게 조회만 해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 빈 metadata entry 를 참조해요.

## NEVER

- NEVER `tenants create/update/delete/icon` 을 vibe skill 로 실행하지 않아요.
- NEVER endpoint/profile 을 여기서 바꾸지 않아요.
- NEVER 다른 tenant 의 비공개 정보를 추측해서 보여주지 않아요.
