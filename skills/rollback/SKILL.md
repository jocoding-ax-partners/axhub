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

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

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

   Consent binding 은 `action=deploy_rollback`, `app_id=$APP_ID`, `context={from_deployment}` 로 맞춰요.

   ```bash
   axhub deploy rollback --app "$APP_ID" --from-deployment "$DEPLOYMENT_ID" --execute --json
   ```

5. **결과를 status/logs 로 이어가요.** 완료 여부는 `status` 또는 `verify` skill 로 확인해요.

## NEVER

- NEVER `recover` 의 forward-fix 와 이 rollback 을 같은 것으로 설명하지 않아요.
- NEVER `deploy cancel` 을 이 skill 로 처리하지 않아요.
- NEVER 비대화형에서 rollback 을 자동 실행하지 않아요.
