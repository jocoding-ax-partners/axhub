---
name: app-lifecycle
description: '이 스킬은 사용자가 axhub 앱을 복제하거나 일시정지·재개하고 싶어할 때 사용해요. 다음 표현에서 활성화: "앱 복제", "앱 포크", "앱 복사해", "앱 일시정지", "앱 멈춰", "앱 중지", "앱 재개", "앱 다시 켜", "fork app", "suspend app", "resume app", 또는 axhub 앱 생명주기 관리 의도. GitHub 저장소 연결은 github 스킬이 담당해요.'
examples:
  - utterance: "앱 복제해"
    intent: "manage app lifecycle"
  - utterance: "앱 일시정지"
    intent: "manage app lifecycle"
  - utterance: "앱 다시 켜"
    intent: "manage app lifecycle"
  - utterance: "fork app"
    intent: "manage app lifecycle"
  - utterance: "resume app"
    intent: "manage app lifecycle"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# App lifecycle

앱 fork, suspend, resume 을 담당해요. runtime 영향이 있는 작업이라 preview 와 consent 뒤에만 mutation 을 실행해요.

## Workflow

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s
' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d	%s
",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

**Tenant grounding.** `fork` 는 destination tenant 를 mutation context 에 묶어요. 사용자가 tenant 를 명시하지 않았으면 preflight 의 active team 만 사용하고, active team 이 없으면 실행을 멈춰요. `tenants[]` 첫 항목을 추측해서 쓰지 않아요.

```bash
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
if [ -z "$TENANT" ]; then
  echo "현재 workspace 를 특정할 수 없어요. workspace skill 로 tenant 를 확인하거나 AXHUB_TENANT 를 명시해요." >&2
  exit 64
fi
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "작업 확인", status: "in_progress", activeForm: "작업 고르는 중" },
     { content: "앱 resolve", status: "pending", activeForm: "앱 확인 중" },
     { content: "caveat 안내", status: "pending", activeForm: "영향 안내 중" },
     { content: "preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "실행 중" },
     { content: "후속 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **작업을 분기해요.** fork/suspend/resume 중 하나예요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

2. **AskUserQuestion 으로 작업을 확인해요.** 비대화형 기본은 `abort`예요.

   ```json
   {"questions":[{"question":"앱 생명주기 작업을 실행할까요?","header":"앱 작업","options":[{"label":"중단","description":"아무것도 바꾸지 않아요"},{"label":"실행","description":"선택한 앱 작업을 실행해요"}]}]}
   ```

3. **CLI 명령.**

   Consent binding 은 helper parser 와 같은 action/context 로 맞춰요: fork 는 `app_id=$SOURCE_APP`, `context={source,slug,subdomain,tenant,name,template,repo_public}` 이고, suspend/resume 은 `{app_id}` 예요. `--template` 을 안 쓰면 `template="<source>"`, `--repo-public` 을 안 쓰면 `repo_public="false"` 로 맞춰요.

   ```bash
   axhub apps fork "$SOURCE_APP" --slug "$NEW_SLUG" --subdomain "$NEW_SUBDOMAIN" --name "$NAME" --tenant "$TENANT" --execute --json
   axhub apps suspend "$APP_ID" --execute --json
   axhub apps resume "$APP_ID" --execute --json
   ```

4. **후속 안내.** resume 은 자동 redeploy 를 보장하지 않으니 필요하면 `deploy` skill 로 이어가요.

## NEVER

- NEVER GitHub repo connect/disconnect 를 여기서 처리하지 않아요. `github` skill 로 넘겨요.
- NEVER suspend/resume 를 read-only 처럼 표현하지 않아요.
- NEVER 비대화형에서 앱 runtime 변경을 자동 실행하지 않아요.
