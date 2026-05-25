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

axhub 앱과 GitHub repo 연결 상태를 안전하게 확인하고 connect/disconnect 를 consent 로 보호해요. CLI 는 `axhub apps git` 서브커맨드로 이동했어요 (구 `axhub github` 는 exit 7 GITHUB_CMD_DEPRECATED 로 거절돼요).

## Workflow

To connect GitHub:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

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

   **preflight 의 `cli_state` 를 보고 분기해요** (v0.9.6 부터 명시적 필드 emit). `cli_present:false` 만으로 "CLI 미설치 (PATH 에 없음)" 으로 해석하지 마세요 — cli_state 4 값에 따라 안내가 달라요:

   - `"ok"` → 정상, SKILL 흐름 그대로 진행
   - `"not_found"` → "axhub CLI 가 PATH 에서 안 보여요." `/axhub:install-cli` 또는 macOS Apple Silicon Homebrew 사용 중이면 `/opt/homebrew/bin` inherit 안 됐을 가능성. `/axhub:doctor` 로 진단.
   - `"config_corrupted"` → "axhub CLI 는 설치돼 있지만 `~/.config/axhub/config.yaml` 이 새 schema 와 안 맞아요 (예: user_id UUID vs int64 mismatch)." `/axhub:auth` 로 재로그인하면 fresh config 가 작성되면서 자동 fix 돼요. (CLI 미설치 아니라 config drift 임을 명확히 구분)
   - `"runtime_error"` → "axhub CLI 가 실행은 됐지만 비정상 exit 했어요." `/axhub:doctor` 로 진단.

   진단 카드 / status 표시할 때 cli_state 별 메시지를 그대로 써요. `cli_present:false` 를 임의로 "PATH 에 없음" 으로 매핑하지 마세요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `list_only` 예요. repo create / remote add / first push / connect 는 모두 `abort` 예요.

2. **작업을 고르게 해요.**

   ```json
   {
     "question": "GitHub 연동 작업을 고를까요?",
     "header": "GitHub",
     "options": [
       {"label": "현재 상태", "value": "list_only", "description": "앱의 현재 GitHub 연결 상태를 봐요"},
       {"label": "연결", "value": "connect", "description": "앱에 GitHub 저장소를 연결해요"},
       {"label": "연결 해제", "value": "disconnect", "description": "exact confirm 과 consent 가 필요해요"}
     ]
   }
   ```

3. **현재 연결 상태를 read-only 로 확인해요.**

   ```bash
   axhub apps git status --app "$APP_ID" --json
   ```

   출력에 `install_url` 이 들어 있으면 즉시 `GitHub 연결 링크: <install_url>` 로 안내해요. 다른 슬래시 커맨드 호출을 요구하지 마세요. 연결이 아직 없으면 status 가 404 / `git_connection` not_found 를 반환해요 — 이 경우 install_url 안내 후 Step 4 consent-connect 로 진행해요. 연결이 이미 있으면 `repo_full_name` / `branch` / `installation_id` 를 사용자에게 보여줘요.

3.5. **Strict guided capability ladder for missing repo/remote/push.** Stay inside this ladder and stop on every unsupported gap. Do not skip ahead to connect while GitHub cannot see the repo.

   1. **read-only git inspect** — gather local facts only:

      ```bash
      git rev-parse --is-inside-work-tree
      git remote -v
      git branch --show-current
      git status --short
      ```

      parse existing remote from `origin`/first GitHub remote when present. If no remote exists, derive only a suggested `OWNER_REPO` from the confirmed account and folder name; never treat it as confirmed.

   2. **Verify remote visibility in axhub before mutating.** Run dry-run connect — `axhub apps git connect` 는 `--execute` 없이 호출하면 dry-run 이라 OAuth/installation 검증만 수행하고 mutate 하지 않아요.

      ```bash
      axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json
      ```

      dry-run 결과의 `installation_id` + `repo_full_name` 이 채워지면 visibility 확인됨 — Step 4 consent-connect 로 진행해요. dry-run 이 install_url 을 emit 하거나 `not_in_installation` 류 에러를 내면 Step 3.5 의 다음 단계 (repo 생성 / remote 추가 / push) 로 내려가요.

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

      Only after `create`, run a concrete `gh repo create "$OWNER_REPO" --private|--public` command that matches the confirmed visibility. Then re-list after create/push with `axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json` (dry-run). If the repo still does not appear, stop with the install/access gap.

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

      If the user consents, run `git push -u origin "$BRANCH"`. Then re-list after create/push with `axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --json` (dry-run) before connect.

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

4. **connect 는 consent 후 실행해요.** interactive 에서 device flow 가 승인까지 block 하니까, `/axhub:auth` Step 5b 와 동일하게 consent-mint 를 별도 호출로 분리한 뒤 connect 를 detach + tail 로 호출해서 verification URL 을 먼저 surface 해요. sync 호출하면 Claude Code 가 종료까지 출력을 buffer 해서 URL 이 안 보이고 OAuth 가 timeout 으로 멈춰요.

   **4a. consent token 을 먼저 민어요** (별도 bash call — PreToolUse gate 가 다음 connect call 의 pending claim 을 검증해요):

   ```bash
   APP_ID="${APP_ID:-$APP}"
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   if [ -z "$HELPER" ] || [ ! -x "$HELPER" ]; then
     HELPER="axhub-helpers"
   fi
   cat <<JSON | "$HELPER" consent-mint
   {"tool_call_id":"pending","action":"github_connect","app_id":"${APP_ID}","profile":"","branch":"${BRANCH}","commit_sha":"","context":{"repo":"${OWNER_REPO}","branch":"${BRANCH}","account":"${ACCOUNT}"}}
   JSON
   ```

   `consent-mint` 에 `action=github_connect`, top-level `app_id`, `context={repo,branch,account}` 를 넣어요. `CLAUDE_PLUGIN_ROOT` 가 훅 환경에 없어도 PATH 의 `axhub-helpers` 로 민어요 — 사용자에게 수동 실행이나 bang-prefixed 우회를 요청하지 마세요.

   **4b. connect 를 detach 후 device challenge 먼저 surface.** challenge 의 verification URL + user_code 는 stderr(unbuffered) 로 나오니까 nohup detach + tail 로 즉시 잡혀요. `axhub apps git connect` 는 `--execute` 가 있어야 실제 연결돼요 (없으면 dry-run):

   ```bash
   APP_ID="${APP_ID:-$APP}"
   GIT_LOG=$(mktemp -t axhub-gitconnect-XXXXXX)
   nohup axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json >"$GIT_LOG" 2>&1 </dev/null &
   GIT_PID=$!
   disown 2>/dev/null || true

   URL=""; CODE=""; INSTALL_URL=""; DONE=""
   for _ in $(seq 1 40); do
     if [ -s "$GIT_LOG" ]; then
       # verification_uri_complete (코드 자동입력 ?user_code=...) 를 우선, 없으면 basic verification_uri
       URL=$(grep -oE 'https?://github\.com/login/device\?[A-Za-z0-9._~+%/?=#&-]+' "$GIT_LOG" | head -1)
       [ -n "$URL" ] || URL=$(grep -oE 'https?://github\.com/login/device[A-Za-z0-9._~+%/?=#&-]*' "$GIT_LOG" | head -1)
       CODE=$(grep -oE '[A-Z0-9]{4}-[A-Z0-9]{4}' "$GIT_LOG" | head -1)
       INSTALL_URL=$(grep -oE '"install_url":"[^"]+"' "$GIT_LOG" | head -1 | sed 's/.*:"//; s/"$//')
       DONE=$(grep -oE '"repo_full_name":"[^"]+"' "$GIT_LOG" | head -1 | sed 's/.*:"//; s/"$//')
       { [ -n "$URL" ] && [ -n "$CODE" ]; } && break
       [ -n "$INSTALL_URL" ] && break
       [ -n "$DONE" ] && break
     fi
     kill -0 "$GIT_PID" 2>/dev/null || break
     sleep 0.5
   done

   # 프로세스가 빨리 끝나며(App 이미 설치 등) exit 시 flush 한 stdout 값을 마지막으로 한 번 더 긁어요
   [ -z "$URL" ] && URL=$(grep -oE 'https?://github\.com/login/device[A-Za-z0-9._~+%/?=#&-]*' "$GIT_LOG" 2>/dev/null | head -1)
   [ -z "$CODE" ] && CODE=$(grep -oE '[A-Z0-9]{4}-[A-Z0-9]{4}' "$GIT_LOG" 2>/dev/null | head -1)
   [ -z "$INSTALL_URL" ] && INSTALL_URL=$(grep -oE '"install_url":"[^"]+"' "$GIT_LOG" 2>/dev/null | head -1 | sed 's/.*:"//; s/"$//')
   [ -z "$DONE" ] && DONE=$(grep -oE '"repo_full_name":"[^"]+"' "$GIT_LOG" 2>/dev/null | head -1 | sed 's/.*:"//; s/"$//')

   if [ -n "$URL" ] && [ -n "$CODE" ]; then
     printf '\nGitHub OAuth 인증이 필요해요. 다음 두 단계를 진행해 주세요:\n\n  1. 브라우저에서 열기: %s\n  2. 코드 입력: %s\n\n완료되면 자동으로 연결돼요.\n[axhub] GIT_LOG=%s GIT_PID=%s APP_ID=%s (다음 step 확인에서 사용)\n' "$URL" "$CODE" "$GIT_LOG" "$GIT_PID" "$APP_ID"
     exit 0
   elif [ -n "$INSTALL_URL" ]; then
     printf '\naxhub GitHub App 이 아직 설치 안 됐어요. 먼저 설치해 주세요:\n\n  %s\n\n설치 후 다시 연결을 시도해요.\n' "$INSTALL_URL"
     exit 0
   elif [ -n "$DONE" ]; then
     printf 'GitHub 연결 완료: %s (device flow 불필요 — GitHub App 이 이미 설치돼 있었어요)\n' "$DONE"
     exit 0
   else
     echo "device URL/code 를 20초 안에 추출 못 했어요. CLI 출력 형식이 바뀌었을 가능성. /axhub:doctor 로 진단해주세요."
     tail -5 "$GIT_LOG" 2>/dev/null
     kill "$GIT_PID" 2>/dev/null
     exit 1
   fi
   ```

   `verification_uri_complete` 가 stderr 에 "Or open directly:" 로 같이 나오면 그 URL 을 1번에 우선 표시해요 (코드 자동 입력이라 2번 생략 가능). `installation_id` 가 여러 개로 모호하면 connect 에 `--installation-id <id>` 를 더해 재시도해요.

   **4c. 승인 후 연결 결과를 확인해요.** device flow 를 surface 한 경우만 필요해요 (4b 가 이미 "연결 완료" 면 생략). 직전 step 의 `APP_ID` 로 `git status` 를 poll 해요 — 한 번에 최대 ~4.5분, 미완이면 이 step 을 다시 호출해요:

   ```bash
   APP_ID="<직전 step 의 APP_ID>"
   DEADLINE=$(( $(date +%s) + 270 ))
   REPO=""
   while [ "$(date +%s)" -lt "$DEADLINE" ]; do
     REPO=$(axhub apps git status --app "$APP_ID" --json 2>/dev/null | jq -r '.data.repo_full_name // .repo_full_name // empty')
     [ -n "$REPO" ] && break
     sleep 4
   done
   if [ -n "$REPO" ]; then
     printf 'GitHub 연결 완료: %s\n' "$REPO"
   else
     echo "아직 연결 확인이 안 돼요. 브라우저 승인을 마쳤는지 확인하고, 잠시 후 이 step 을 다시 호출해요."
   fi
   ```

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

   axhub apps git disconnect --app "$APP_ID" --execute --json
   ```

   `consent-mint` 에 `action=github_disconnect`, top-level `app_id`, `context={slug}` 를 넣어요. `--execute` 없이는 dry-run 이라 mutate 하지 않아요. 구 `--force` / `--confirm` 플래그는 제거됐어요.

## NEVER

- NEVER GitHub App install URL 을 자동으로 열거나 권한을 부여하지 않아요.
- NEVER owner/repo 를 추측해서 connect 하지 않아요.
- NEVER disconnect 를 subprocess 에서 자동 실행하지 않아요.
- NEVER `CLAUDE_PLUGIN_ROOT` 누락을 이유로 사용자에게 bang-prefixed connect 수동 우회를 요청하지 않아요.
- NEVER `--json` 을 빼지 않아요.
- NEVER 구 `axhub github connect|disconnect|repos list` 명령어를 호출하지 않아요. exit 7 GITHUB_CMD_DEPRECATED 로 거절돼요. 항상 `axhub apps git connect|disconnect|status` 를 써요.
- NEVER `axhub apps git connect|disconnect` 를 `--execute` 없이 mutate 한다고 가정하지 않아요. dry-run 이 기본이라 `--execute` 가 빠지면 backend mutation 은 발생 안 해요.
- NEVER OAuth device flow 의 `verification_uri` + `user_code` 를 사용자에게 안 보여주지 않아요. CLI 가 emit 한 `device_code_issued` JSON event 또는 stderr "To connect GitHub, visit: …" 줄을 그대로 흘려보내면 OAuth 가 timeout 으로 멈춰요. 두 값을 한 번에 묶어서 위 형식으로 안내해요.
- NEVER add a 4th option (e.g. "지금은 스킵") to Step 2 의 AskUserQuestion. backend 가 git_connection_required (HTTP 422) 로 거절해요. options 는 정확히 3개: list_only / connect / disconnect.
