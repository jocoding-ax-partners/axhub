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
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
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

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

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

4. **connect 는 consent 후 실행해요.**

   ```bash
   APP_ID="${APP_ID:-$APP}"
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   cat <<JSON | "$HELPER" consent-mint
   {"tool_call_id":"pending","action":"github_connect","app_id":"${APP_ID}","profile":"","branch":"${BRANCH}","commit_sha":"","context":{"repo":"${OWNER_REPO}","branch":"${BRANCH}","account":"${ACCOUNT}"}}
   JSON

   axhub apps git connect --app "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --execute --json
   ```

   `consent-mint` 에 `action=github_connect`, top-level `app_id`, `context={repo,branch,account}` 를 넣어요. `axhub apps git connect` 가 `--execute` 없이 호출되면 dry-run 이라 mutate 하지 않아요 — 실제 연결은 `--execute` 가 필요해요. GitHub App 설치 / installation 다중 후보가 있으면 CLI 가 자동 OAuth device flow 로 처리하고, 안 되면 install_url 을 emit 해요. `installation_id` 가 여러 개로 모호하면 `--installation-id <id>` 로 disambiguate 해요.

   **OAuth device flow 가 시작되면 사용자에게 verification URL + user_code 를 즉시 보여주세요.** CLI 는 `device_code_issued` JSON event (`{user_code, verification_uri, verification_uri_complete}`) + stderr plain (`To connect GitHub, visit: <verification_uri>` / `Enter code: <user_code>` / `Or open directly: <verification_uri_complete>`) 를 emit 해요. JSON 이벤트나 stderr 한 줄도 사용자에게 안 보여주면 OAuth 가 timeout 으로 멈춰요. 다음 형식으로 사용자에게 한 번에 안내해요:

   ```
   GitHub OAuth 인증이 필요해요. 다음 두 단계를 진행해주세요:

   1. 브라우저에서 열기: <verification_uri_complete or verification_uri>
   2. 코드 입력: <user_code>

   ```

   `verification_uri_complete` 가 있으면 우선 표시 (코드 입력 자동). 없으면 `verification_uri` + 별도 `user_code` 표시.

   **컨텍스트별 완료 (axhub-cli 0.15.3+).** 대화형 TTY 면 connect 가 승인까지 polling 해서 자동으로 다음 단계로 진행돼요. 에이전트 / 비-TTY 컨텍스트면 connect 가 `device_code_issued` emit 직후 fast-exit 하므로 자동 진행이 안 돼요. challenge 를 보여준 뒤 멈추고, 사용자에게 "이 호출은 승인 완료를 polling 하지 않아요. 계속하려면 대화형 터미널에서 같은 `axhub apps git connect ... --execute` 흐름을 다시 시작해 새 device flow 를 완료해 주세요" 라고 안내해요. 이전 `user_code` 를 승인한 뒤 같은 에이전트 명령을 재호출해도 이어지지 않아요. CLI 가 internal `device_code` 를 노출하지 않기 때문이에요. 완전 autonomous 완료는 CLI 의 device_code persist resume 기능을 기다려요 (`docs/superpowers/specs/2026-05-25-github-device-flow-surface-design.md`).

   `CLAUDE_PLUGIN_ROOT` 가 훅 환경에 없더라도 사용자에게 수동 실행이나 bang-prefixed connect 우회를 요청하지 말고, PATH 의 `axhub-helpers` 로 pending token 을 민 뒤 같은 흐름에서 top-level Bash 로 connect 를 실행해요.

5. **disconnect 는 exact confirm 후 실행해요.**

   ```bash
   APP_ID="${APP_ID:-$APP}"
   APP_ID_OR_SLUG="${APP_ID_OR_SLUG:-$APP_ID}"
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
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
