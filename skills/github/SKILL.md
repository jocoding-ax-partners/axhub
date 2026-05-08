---
name: github
description: '이 스킬은 사용자가 axhub 앱과 GitHub repo 를 연결하거나 끊고 싶어할 때 사용해요. 다음 표현에서 활성화: "깃허브 연결", "내 repo 붙", "내 repo 붙여", "git 연결", "github 연결", "GitHub 연결", "GitHub repo 연결해", "repo 끊", "repo 끊어", "repo 연결", "github connect", "github disconnect", "github repo", 또는 GitHub 연동 의도. GitHub App 설치가 없으면 install URL 을 안내해요.'
examples:
  - utterance: "깃허브 연결"
    intent: "connect github repo to axhub"
  - utterance: "내 repo 붙"
    intent: "connect github repo to axhub"
  - utterance: "github connect"
    intent: "connect github repo to axhub"
  - utterance: "github disconnect"
    intent: "connect github repo to axhub"
  - utterance: "내 repo 붙여"
    intent: "connect github repo to axhub"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
---

# GitHub

axhub 앱과 GitHub repo 연결 상태를 안전하게 확인하고 connect/disconnect 를 consent 로 보호해요.

## Workflow

To connect GitHub:

!`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "앱과 auth 컨텍스트 확인", status: "in_progress", activeForm: "컨텍스트 확인 중" },
     { content: "GitHub 작업 분기", status: "pending", activeForm: "작업 고르는 중" },
     { content: "repo 연결 상태 처리", status: "pending", activeForm: "GitHub 처리 중" },
     { content: "다음 deploy 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

1. **preflight 와 current app 을 확인해요.** 앱이 없으면 `apps` skill 흐름으로 먼저 고르게 해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `목록만` 예요.

2. **작업을 고르게 해요.**

   ```json
   {
     "question": "GitHub 연동 작업을 고를까요?",
     "header": "GitHub",
     "options": [
       {"label": "목록만", "value": "repos", "description": "연결 가능한 repo 목록을 봐요"},
       {"label": "연결", "value": "connect", "description": "앱에 repo 와 branch 를 연결해요"},
       {"label": "연결 해제", "value": "disconnect", "description": "exact confirm 과 consent 가 필요해요"}
     ]
   }
   ```

3. **repo 목록은 read-only 로 실행해요.**

   ```bash
   axhub github repos list --account "$ACCOUNT" --json
   ```

   If the user has not picked an account yet, first run:

   ```bash
   axhub github repos list --json
   ```

   If the output contains `install_url`, show it immediately as `GitHub 연결 링크: <install_url>`. Do not ask the user to invoke another slash command just to see the link. If `installed:false` or no repo list is available for the desired account, stop after showing the link and the account name. If the account is installed, continue to Step 4 with the selected `OWNER_REPO`, `BRANCH`, and `ACCOUNT`.

4. **connect 는 consent 후 실행해요.**

   ```bash
   APP_ID="${APP_ID:-$APP}"
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   if [ -z "$HELPER" ] || [ ! -x "$HELPER" ]; then
     HELPER="axhub-helpers"
   fi
   cat <<JSON | "$HELPER" consent-mint
   {"tool_call_id":"pending","action":"github_connect","app_id":"${APP_ID}","profile":"","branch":"${BRANCH}","commit_sha":"","context":{"repo":"${OWNER_REPO}","branch":"${BRANCH}","account":"${ACCOUNT}"}}
   JSON

   axhub github connect "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --account "$ACCOUNT" --json
   ```

   `consent-mint` 에 `action=github_connect`, top-level `app_id`, `context={repo,branch,account}` 를 넣어요. GitHub App 설치가 없다는 응답이면 install URL 만 안내해요.
   `CLAUDE_PLUGIN_ROOT` 가 훅 환경에 없더라도 사용자에게 수동 실행이나 bang-prefixed connect 우회를 요청하지 말고, PATH 의 `axhub-helpers` 로 pending token 을 민 뒤 같은 흐름에서 top-level Bash 로 connect 를 실행해요.

5. **disconnect 는 exact confirm 후 실행해요.**

   ```bash
   APP_ID="${APP_ID:-$APP}"
   APP_ID_OR_SLUG="${APP_ID_OR_SLUG:-$APP_ID}"
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   if [ -z "$HELPER" ] || [ ! -x "$HELPER" ]; then
     HELPER="axhub-helpers"
   fi
   cat <<JSON | "$HELPER" consent-mint
   {"tool_call_id":"pending","action":"github_disconnect","app_id":"${APP_ID}","profile":"","branch":"","commit_sha":"","context":{"slug":"${APP_ID_OR_SLUG}"}}
   JSON

   axhub github disconnect "$APP_ID" --force --confirm "$APP_ID" --json
   ```

   `consent-mint` 에 `action=github_disconnect`, top-level `app_id`, `context={slug}` 를 넣어요.

## NEVER

- NEVER GitHub App install URL 을 자동으로 열거나 권한을 부여하지 않아요.
- NEVER owner/repo 를 추측해서 connect 하지 않아요.
- NEVER disconnect 를 subprocess 에서 자동 실행하지 않아요.
- NEVER `CLAUDE_PLUGIN_ROOT` 누락을 이유로 사용자에게 bang-prefixed connect 수동 우회를 요청하지 않아요.
- NEVER `--json` 을 빼지 않아요.
