---
name: env
description: '이 스킬은 사용자가 axhub 앱의 환경변수를 보거나 추가하거나 삭제하고 싶어할 때 사용해요. 다음 표현에서 활성화: "환경변수 뭐 있어", "환경변수 추가", "환경 변수 확인", "DB URL 추가", "API 키 등록", "secret 추가", "env 봐", "env 추가", "env 삭제", 또는 axhub 앱의 env var 조회/변경 의도. set 은 --from-stdin 으로만 받아 argv 노출을 막아요.'
multi-step: true
needs-preflight: true
---

# Env

axhub 앱 환경변수 조회와 변경을 안전하게 처리해요. secret value 는 argv, shell history, telemetry, debug log 에 남기지 않아요.

## Workflow

To manage env vars:

!`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "앱과 auth 컨텍스트 확인", status: "in_progress", activeForm: "컨텍스트 확인 중" },
     { content: "env 작업 분기", status: "pending", activeForm: "작업 고르는 중" },
     { content: "안전한 CLI 호출", status: "pending", activeForm: "env 처리 중" },
     { content: "마스킹된 결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

1. **preflight 와 current app 을 확인해요.** `auth_ok` 가 false 면 auth skill 로 넘겨요. 앱이 없으면 `axhub apps list --json` 또는 resolve helper 로 후보를 좁혀요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `조회만` 예요.

2. **작업을 분기해요.**

   ```json
   {
     "question": "어떤 환경변수 작업을 할까요?",
     "header": "env",
     "options": [
       {"label": "조회만", "value": "list", "description": "값은 마스킹하고 key 목록을 봐요"},
       {"label": "추가 또는 수정", "value": "set", "description": "값은 stdin 으로만 전달해요"},
       {"label": "삭제", "value": "delete", "description": "key 확인과 consent 가 필요해요"}
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

   실행 전 `consent-mint` 에 `action=env_set`, top-level `app_id`, `context={key}` 를 사용해요. 값은 출력하지 말고 즉시 마스킹해요.

5. **delete 는 exact confirm 을 요구해요.**

   ```bash
   axhub env delete "$KEY" --app "$APP" --force --confirm "$KEY" --json
   ```

   실행 전 `consent-mint` 에 `action=env_delete`, top-level `app_id`, `context={key}` 를 사용해요.

## NEVER

- NEVER secret value 를 argv 로 넘기지 않아요.
- NEVER `set` 에서 `--from-stdin` 을 빼지 않아요.
- NEVER secret value 를 로그, 응답, telemetry 에 평문으로 남기지 않아요.
- NEVER subprocess 에서 set/delete 를 자동 선택하지 않아요.
