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
model: sonnet
---

# GitHub

axhub 앱과 GitHub repo 연결 상태를 안전하게 확인하고 connect/disconnect 를 consent 로 보호해요.

## Workflow

To connect GitHub:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText)}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "앱과 auth 컨텍스트 확인", status: "in_progress", activeForm: "컨텍스트 확인 중" },
     { content: "GitHub 작업 분기", status: "pending", activeForm: "작업 고르는 중" },
     { content: "GitHub 저장소 연결 상태 점검", status: "pending", activeForm: "GitHub 처리 중" },
     { content: "다음 deploy 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **preflight 와 current app 을 확인해요.** 앱이 없으면 `apps` skill 흐름으로 먼저 고르게 해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `list_only` 예요. repo create / remote add / first push / connect 는 모두 `abort` 예요.

2. **작업을 고르게 해요.**

   ```json
   {
     "question": "GitHub 연동 작업을 고를까요?",
     "header": "GitHub",
     "options": [
       {"label": "목록만", "value": "list_only", "description": "연결할 수 있는 GitHub 저장소 목록을 봐요"},
       {"label": "연결", "value": "connect", "description": "앱에 GitHub 저장소를 연결해요"},
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

3.5. **Strict guided capability ladder for missing repo/remote/push.** Stay inside this ladder and stop on every unsupported gap. Do not skip ahead to connect while GitHub cannot see the repo.

   1. **read-only git inspect** — gather local facts only:

      ```bash
      git rev-parse --is-inside-work-tree
      git remote -v
      git branch --show-current
      git status --short
      ```

      parse existing remote from `origin`/first GitHub remote when present. If no remote exists, derive only a suggested `OWNER_REPO` from the confirmed account and folder name; never treat it as confirmed.

   2. **Verify remote visibility in axhub before mutating.**

      ```bash
      axhub github repos list --account "$ACCOUNT" --json
      ```

      If `OWNER_REPO` appears, continue toward connect. If it does not appear, do not connect yet.

   3. **Create repo only when every gate is true:** gh exists/authenticated (`gh auth status` succeeds for the target host/account), owner-repo-visibility confirmed by the user, visibility is explicit (`private`/`public`), and the user consents. If any gate is missing, stop with an unsupported gap and show the smallest next manual action.

      ```json
      {
        "question": "GitHub repo 를 만들까요?",
        "header": "저장소 만들기",
        "options": [
          {"label": "취소", "value": "abort", "description": "GitHub 저장소를 만들지 않고 멈춰요"},
          {"label": "생성", "value": "create", "description": "확인한 이름과 공개 범위로 GitHub 저장소를 새로 만들어요"}
        ]
      }
      ```

      Only after `create`, run a concrete `gh repo create "$OWNER_REPO" --private|--public` command that matches the confirmed visibility. Then re-list after create/push with `axhub github repos list --account "$ACCOUNT" --json`. If the repo still does not appear, stop with the install/access gap.

   4. **Add remote only with separate consent.**

      ```json
      {
        "question": "git remote 를 추가할까요?",
        "header": "GitHub 주소 추가",
        "options": [
          {"label": "취소", "value": "abort", "description": "GitHub 연결 설정을 바꾸지 않고 멈춰요"},
          {"label": "추가", "value": "add_remote", "description": "확인한 GitHub 주소를 현재 폴더에 연결해요"}
        ]
      }
      ```

      If the user consents, run `git remote add origin "$GITHUB_URL"` only when no `origin` exists. If a different `origin` exists, stop and ask the user to resolve it outside this skill.

   5. **First push only with separate consent.**

      ```json
      {
        "question": "첫 push 를 실행할까요?",
        "header": "첫 올리기",
        "options": [
          {"label": "취소", "value": "abort", "description": "push 하지 않고 멈춰요"},
          {"label": "올리기", "value": "push", "description": "현재 branch 를 origin 에 처음 올려요"}
        ]
      }
      ```

      If the user consents, run `git push -u origin "$BRANCH"`. Then re-list after create/push with `axhub github repos list --account "$ACCOUNT" --json` before connect.

   6. **Connect only after the repo is visible and with separate consent.**

      ```json
      {
        "question": "axhub 앱에 repo 를 연결할까요?",
        "header": "저장소 연결",
        "options": [
          {"label": "취소", "value": "abort", "description": "앱 연결 없이 멈춰요"},
          {"label": "연결", "value": "connect", "description": "동의를 받고 axhub 앱과 GitHub 저장소를 연결해요"}
        ]
      }
      ```

      Re-list before connect if any create/push happened. If the repo is not listed for the account, stop on the unsupported gap and show the GitHub App install/access URL when available.

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
- NEVER add a 4th option (e.g. "지금은 스킵") to Step 2 의 AskUserQuestion. backend 가 git_connection_required (HTTP 422) 로 거절해요. options 는 정확히 3개: list_only / connect / disconnect.
