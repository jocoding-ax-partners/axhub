---
name: rollback
description: '이 스킬은 사용자가 특정 이전 배포 상태로 실제 rollback 하고 싶어할 때 사용해요. 다음 표현에서 활성화: "이전 배포로 롤백", "특정 배포로 되돌려", "그 배포로 롤백", "배포 롤백", "rollback deployment", "rollback to deployment", 또는 axhub deploy rollback 의도. 진행 중 배포 취소는 deploy 스킬, 직전 커밋 재배포 forward-fix는 recover 스킬이 담당해요.'
examples:
  - utterance: "이전 배포로 롤백"
    intent: "rollback deployment"
  - utterance: "특정 배포로 되돌려"
    intent: "rollback deployment"
  - utterance: "그 배포로 롤백"
    intent: "rollback deployment"
  - utterance: "rollback deployment"
    intent: "rollback deployment"
  - utterance: "rollback to deployment"
    intent: "rollback deployment"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Deployment rollback

특정 이전 deployment id 로 `axhub deploy rollback` 을 실행해요. 직전 commit 재배포 방식은 recover skill 이 담당하고, 진행 중 배포 취소는 deploy skill 이 담당해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

## Claude Desktop natural-language path

For ordinary Claude Desktop prompts such as `방금 배포 되돌려줘`, `방금 거 되돌려줘`, `직전 안정 버전으로 복구해줘`, or `잘 되던 버전으로 돌려`, start with exactly:

`되돌릴 수 있는 배포를 확인할게요.`

Then make exactly one Bash call with the tool description/title exactly:

`배포 되돌리기 확인`

Run:

```bash
axhub-helpers rollback-summary --user-utterance "<latest user sentence>"
```

Copy the Korean stdout as the answer, then stop. Do not read the rest of this workflow, do not call TodoWrite, do not run preflight/list/rollback/recover/create directly, and do not mention `rollback`, `recover`, slash commands, skill names, route labels, preflight, raw deploy IDs, raw commit hashes, raw status names, `commit_not_found`, `no-op`, app IDs/slugs, or English tool-title fragments in visible text. Any actual rollback or redeploy is destructive/external and must wait until the user sees a Korean preview and explicitly approves in a later turn.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s
' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d	%s
",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
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
     { content: "대상 배포 찾기", status: "in_progress", activeForm: "배포 찾는 중" },
     { content: "rollback 대상 확인", status: "pending", activeForm: "대상 확인 중" },
     { content: "preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "rollback 실행 중" },
     { content: "상태 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **후보를 조회해요.**

   ```bash
   axhub deploy list --app "$APP_ID" --json
   ```

2. **대상 deployment 를 고르고 preview 해요.** app, from-deployment, 현재 live 상태, 예상 영향을 표시해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

3. **AskUserQuestion 으로 rollback 을 확인해요.** 비대화형 기본은 `abort`예요.

   ```json
   {"questions":[{"question":"이 배포로 롤백할까요?","header":"롤백","options":[{"label":"중단","description":"rollback 하지 않아요"},{"label":"롤백","description":"선택한 배포로 되돌려요"}]}]}
   ```

4. **동의 후 실행해요.**

   Approval context 은 `action=deploy_rollback`, `app_id=$APP_ID`, `context={from_deployment}` 로 맞춰요.

   ```bash
   axhub deploy rollback --app "$APP_ID" --from-deployment "$DEPLOYMENT_ID" --execute --json
   ```

5. **결과를 status/logs 로 이어가요.** 완료 여부는 `status` 또는 `verify` skill 로 확인해요.

## NEVER

- NEVER `recover` 의 forward-fix 와 이 rollback 을 같은 것으로 설명하지 않아요.
- NEVER `deploy cancel` 을 이 skill 로 처리하지 않아요.
- NEVER 비대화형에서 rollback 을 자동 실행하지 않아요.
