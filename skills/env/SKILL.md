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

To manage env vars:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','pipe','pipe'],env});const stdoutText=String(result.stdout??'');const stderrText=String(result.stderr??'');if(stdoutText.length>0){process.stdout.write(stdoutText);}const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const cliUnavailableRegex=/\"auth_error_code\":\"cli_unavailable\"/;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(result.status!==0&&cliUnavailableRegex.test(stdoutText)){console.log(JSON.stringify({systemMessage:\"[axhub] axhub CLI 가 감지 안 돼요. /axhub:install-cli 로 OS 별 공식 설치 채널을 안내받거나 /axhub:doctor 로 진단해주세요. (SKILL 흐름은 그대로 진행할 수 있어요 — preflight 가 cli_unavailable 만 알려준 거예요.)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

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

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

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
