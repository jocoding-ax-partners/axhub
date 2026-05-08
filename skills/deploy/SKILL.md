---
name: deploy
description: '이 스킬은 사용자가 현재 브랜치를 axhub 라이브로 배포하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "공개해", "내보내자", "띄워", "배포", "배포해", "배포해줘", "쏘자", "올려", "올리자", "터트려", "푸시한 거 띄워", "프로덕션", "프로덕션에 박아", "demo가 필요", "demo가 필요해", "deploy", "launch", "release", "rollout", "ship", 또는 현재 브랜치를 axhub 라이브로 push 하고 싶다는 모든 의도. axhub deploy create 를 안전 가드와 함께 트리거: 라이브 profile/app 해석, AskUserQuestion preview card 를 통한 HMAC consent gate, exit-code 기반 복구 라우팅.'
examples:
  - utterance: "paydrop 배포해"
    intent: "deploy current branch to axhub live"
  - utterance: "어쨌든 그냥 배포해, 묻지 말고 prod에 박아"
    intent: "deploy current branch to axhub live"
  - utterance: "ship paydrop"
    intent: "deploy current branch to axhub live"
  - utterance: "/axhub:deploy paydrop --branch main"
    intent: "deploy current branch to axhub live"
  - utterance: "이전에 승인한 거 그대로 다시 배포해"
    intent: "deploy current branch to axhub live"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
---

# Deploy via axhub

Deploy a vibe coder's app to axhub with safety primitives. Use the adapter `axhub-helpers` (auto on PATH while plugin is enabled) for live resolution and consent management. Do not call `axhub deploy create` directly without going through the helper flow.

## Workflow

**CLAUDE_PLUGIN_ROOT 자동 확인.** 모든 helper 호출 전에 `CLAUDE_PLUGIN_ROOT` 를 자동으로 채워요. Claude Code 가 env 를 전달하지 않은 세션에서는 `CLAUDE_SKILL_DIR` 로 plugin root 를 계산하고, 그래도 없으면 PATH 의 `axhub-helpers` / `axhub-helpers.exe` 위치에서 root 를 역산해요. 성공하면 조용히 `PATH` 에 plugin `bin/` 을 앞에 붙이고, 사용자에게 절대경로 우회 안내를 시키지 않아요.

```bash
if [ -z "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  if [ -n "${CLAUDE_SKILL_DIR:-}" ] && [ -d "${CLAUDE_SKILL_DIR}/../.." ]; then
    export CLAUDE_PLUGIN_ROOT="$(cd "${CLAUDE_SKILL_DIR}/../.." && pwd)"
  elif HELPER_FROM_PATH="$(command -v axhub-helpers 2>/dev/null)"; then
    export CLAUDE_PLUGIN_ROOT="$(cd "$(dirname "$HELPER_FROM_PATH")/.." && pwd)"
  elif [ -x "./bin/axhub-helpers" ]; then
    export CLAUDE_PLUGIN_ROOT="$(pwd)"
  fi
fi
if [ -n "${CLAUDE_PLUGIN_ROOT:-}" ]; then
  export PATH="${CLAUDE_PLUGIN_ROOT}/bin:${PATH}"
fi
```

Windows PowerShell 에서는 같은 규칙을 아래처럼 적용해요. native Windows 는 `.exe` helper 를 명시해요.

```powershell
if (-not $env:CLAUDE_PLUGIN_ROOT) {
  if ($env:CLAUDE_SKILL_DIR -and (Test-Path (Join-Path $env:CLAUDE_SKILL_DIR "..\.."))) {
    $env:CLAUDE_PLUGIN_ROOT = (Resolve-Path (Join-Path $env:CLAUDE_SKILL_DIR "..\..")).Path
  } elseif ($cmd = Get-Command axhub-helpers.exe -ErrorAction SilentlyContinue) {
    $env:CLAUDE_PLUGIN_ROOT = (Resolve-Path (Join-Path (Split-Path $cmd.Source -Parent) "..")).Path
  } elseif (Test-Path ".\bin\axhub-helpers.exe") {
    $env:CLAUDE_PLUGIN_ROOT = (Get-Location).Path
  }
}
if ($env:CLAUDE_PLUGIN_ROOT) {
  $env:PATH = (Join-Path $env:CLAUDE_PLUGIN_ROOT "bin") + [IO.Path]::PathSeparator + $env:PATH
}
```

**Pre-execute preflight context (Phase 17 US-1706 — `!command` injection)**:

```
!`node -e "const fs=require('fs'),path=require('path'),cp=require('child_process'),isWin=process.platform==='win32';let root=process.env.CLAUDE_PLUGIN_ROOT||'';const env=Object.assign({},process.env);let pathKey='PATH';for(const key of Object.keys(env)){if(key.toLowerCase()==='path'){pathKey=key;break;}}if(root.length===0&&process.env.CLAUDE_SKILL_DIR){const candidate=path.resolve(process.env.CLAUDE_SKILL_DIR,'..','..');if(fs.existsSync(candidate))root=candidate;}if(root.length===0){const helperName=isWin?'axhub-helpers.exe':'axhub-helpers';for(const dir of (env[pathKey]||'').split(path.delimiter)){const helperPath=path.join(dir,helperName);if(fs.existsSync(helperPath)){root=path.resolve(dir,'..');break;}}}if(root.length===0&&fs.existsSync(path.resolve('bin',isWin?'axhub-helpers.exe':'axhub-helpers')))root=process.cwd();if(root.length>0){env.CLAUDE_PLUGIN_ROOT=root;env[pathKey]=path.join(root,'bin')+path.delimiter+(env[pathKey]||'');}const helper=root.length>0?path.join(root,'bin',isWin?'axhub-helpers.exe':'axhub-helpers'):(isWin?'axhub-helpers.exe':'axhub-helpers');const result=cp.spawnSync(helper,['preflight','--json'],{stdio:'inherit',env});if(result.error){console.log(JSON.stringify({systemMessage:'[axhub] plugin root 를 자동 확인하지 못했어요. axhub-helpers 를 PATH 에 넣거나 Claude Code plugin 을 다시 로드해요.'}));process.exit(0);}process.exit(typeof result.status==='number'?result.status:0);"`
```

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요. 출력 (auth_status, current_app, current_env, last_deploy_id, last_deploy_status, plugin_version) 이 모델 컨텍스트에 자동 주입돼서 Step 1 의 별도 shell 호출이 줄어요. PreToolUse hook 은 preprocessing 단계에서 trigger 안 해요 (Claude Code SKILL primitive 동작). 실질 명령은 `axhub-helpers preflight --json` 이고, `CLAUDE_PLUGIN_ROOT` 가 비어 있어도 cross-shell Node runner 가 POSIX/Git Bash/WSL 과 Windows PowerShell 양쪽에서 먼저 root 를 확인해요.

**Command lane.** POSIX/Git Bash/WSL 은 `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` 를 쓰고, Windows PowerShell 은 `& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe"` 를 써요. JSON stdin 이 필요한 helper 호출은 PowerShell 에서 `ConvertTo-Json -Compress | & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" <subcommand>` 형태로 실행해요. Bash 배열 예시는 Windows 에서 그대로 붙여넣지 말고 PowerShell 배열 (`$ProfileArgs = @("--profile", $env:PROFILE)`) 로 바꿔요.

To deploy:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start so the user can glance and see how far we've come:

   ```typescript
   TodoWrite({ todos: [
     { content: "토큰 확인 (preflight)",         status: "in_progress", activeForm: "토큰 확인하는 중" },
     { content: "앱 / 환경 / 브랜치 확정",         status: "pending",     activeForm: "앱 정보 정리하는 중" },
     { content: "git 저장 지점 확인",             status: "pending",     activeForm: "배포용 저장 지점 보는 중" },
     { content: "미리보기 카드 보여드리기",         status: "pending",     activeForm: "미리보기 준비하는 중" },
     { content: "동의 받고 배포 시작",            status: "pending",     activeForm: "배포 시작하는 중" },
     { content: "빌드 모니터 (~3분)",             status: "pending",     activeForm: "빌드 진행 보는 중" },
     { content: "결과 안내",                     status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.
   TodoWrite 상태는 Claude Code 세션 안에서 이어질 수 있어요. 그래서 이 스킬을 시작할 때는 기존 todo 에 항목을 하나씩 더하거나 일부만 고치지 말고, 위 배열 전체로 교체해요. 이전 스킬 todo 가 화면에 남아 있으면 Step 1 전에 deploy 목록만 보이도록 다시 호출해요.


1. **Live resolve first.** Fetch authoritative `{profile, endpoint, app_id, app_slug, branch, commit_sha, commit_message, eta_sec}` before any bootstrap create flow:

   ```bash
   echo '[deploy:Step 1 resolve] entered' >&2
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent deploy --user-utterance "$ARGS" --json
   ```

   Never use cached `app_id` for mutation. If live resolve returns an `app_id`, this is an existing app deploy: do **not** run `bootstrap apps_create`, and continue with git readiness, preflight, preview, and the normal consent-deploy path. If resolve returns ambiguity, ask the user to disambiguate (slug list with numeric IDs). If resolve cannot identify a registered app and the project has an `apphub.yaml`/`axhub.yaml`, enter the first-run bootstrap bridge below. The resolve JSON also includes `git_repo`, `git_has_commit`, and `git_init_needed`; deploy MUST NOT continue to the preview card while `branch` or `commit_sha` is empty.

1.1. **First-run bootstrap plan/record bridge (Sprint 3).** Use this only when Step 1 did not resolve an existing `app_id`. Before any first-run remote mutation, ask the Rust FSM for the next safe step:

   ```bash
   echo '[deploy:Step 1 bootstrap-plan] entered' >&2
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers bootstrap --auto-chain --json
   ```

   Treat this output as the source of truth for Sprint 3 bootstrap state. If it returns `template_required`, `git_init_required`, `first_commit_required`, `subdomain_collision`, `backend_contract_missing_defaults`, or `idempotency_unavailable`, stop at that user-decision state and show the helper reason plus the safest next command. If it returns `next_action: apps_create` or `next_action: deploy_create`, show the exact `command`, `binding_hash`, `pending_action_id`, `pending_action_hash`, `retry_policy`, and consent preview before running anything. The helper is only a planner/recorder here; it must not be treated as approval to mutate. If `deploy_create` is executed and recorded here, do not mint or run a second `deploy_create` in Step 4; jump to Step 5 status-chain with the recorded deployment id.

   Execute returned destructive `axhub ... --json` commands only as top-level Bash after the preview/consent path. Then record the observed result back into the FSM with the same pending metadata:

   ```bash
   echo '[deploy:Step 1 bootstrap-record] entered' >&2
   cat > /tmp/axhub-bootstrap-record.json <<JSON
   {
     "schema_version": "bootstrap-record/v1",
     "pending_action_id": "$PENDING_ACTION_ID",
     "pending_action_hash": "$PENDING_ACTION_HASH",
     "command_argv": $COMMAND_ARGV_JSON,
     "exit_code": $EXIT_CODE,
     "stdout_json": $STDOUT_JSON,
     "stderr": "$STDERR_JSON_ESCAPED"
   }
   JSON
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers bootstrap --record "$NEXT_ACTION" --json < /tmp/axhub-bootstrap-record.json
   ```

   S3B retry ownership lives in this skill because this skill runs the top-level command. Retry a create only when helper output explicitly provides an idempotency key and a retry policy that allows it. If the helper says `no_retry_without_confirmed_idempotency` or returns `idempotency_unavailable`, do not retry; show the typed stop.

1.2. **Fresh resolve after local/bootstrap state changes** — call the helper again if git/bootstrap work changed app or commit identity:

   ```bash
   echo '[deploy:Step 1 resolve] entered' >&2
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent deploy --user-utterance "$ARGS" --json
   ```

   Never use cached `app_id` for mutation. If resolve still returns ambiguity, ask the user to disambiguate (slug list with numeric IDs). Deploy MUST NOT continue to the preview card while `branch` or `commit_sha` is empty.

1.5. **Git 저장 지점 준비** — if resolve returns `git_init_needed: true` OR `git_has_commit: false` OR either `branch`/`commit_sha` is empty, do not show the deploy preview yet. Before showing any explanatory copy or AskUserQuestion, replace the full TodoWrite list with the local git readiness checklist. Do not render this plan as a markdown checklist; Claude Code TodoWrite is the progress UI for every 3+ step branch.

   ```typescript
   TodoWrite({ todos: [
     { content: "git 저장소 만들기",        status: "in_progress", activeForm: "git 저장소 만드는 중" },
     { content: "파일을 첫 저장 지점에 담기", status: "pending",     activeForm: "파일 담는 중" },
     { content: "첫 커밋 만들기",          status: "pending",     activeForm: "첫 커밋 만드는 중" },
     { content: "배포 정보 다시 확인하기",   status: "pending",     activeForm: "배포 정보 다시 보는 중" },
     { content: "미리보기 카드 보여드리기",  status: "pending",     activeForm: "미리보기 준비하는 중" }
   ]})
   ```

   Then explain in non-developer Korean:

   ```
   배포 전에 저장 지점이 필요해요.
   axhub 배포는 "어떤 버전의 파일을 올릴지"를 정확히 알아야 해서 branch 와 commit SHA 를 써요.
   지금 폴더에는 아직 그 저장 지점이 없어서, 제가 git 초기화와 첫 커밋을 만들어드릴 수 있어요.
   ```

   Then ask:

   ```json
   {
     "question": "배포 전 저장 지점을 만들까요?",
     "header": "저장 지점",
     "options": [
       {
         "label": "초기화하고 계속",
         "value": "init_and_continue",
         "description": "현재 폴더에 git 저장소와 첫 커밋을 만들고 배포를 이어가요."
       },
       {
         "label": "명령어만 보기",
         "value": "show_commands",
         "description": "아무것도 바꾸지 않고 직접 실행할 명령어만 보여줘요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   If the user chooses "초기화하고 계속", run only local git commands, then re-run resolve and continue from Step 2. Keep the git readiness TodoWrite list on screen and update statuses as each command finishes. 이 TodoWrite 호출도 기존 목록을 기준으로 patch 하지 말고 전체 교체로 실행해요. If another skill or stale todo list appears, replace the whole list again instead of patching individual items. 이전 스킬 todo 를 섞으면 사용자가 지금 흐름을 잘못 이해해요.

   ```bash
   echo '[deploy:Step 1.5 git-init] entered' >&2
   if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
     git init
   fi
   git add -A
   git commit -m "init: axhub deploy baseline"
   git branch -M main
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent deploy --user-utterance "$ARGS" --json
   ```

   If `git commit` fails because there are no staged files or git identity is missing, stop before deploy and show the exact git error plus the smallest next command. Do not mint deploy consent until a fresh resolve returns both `branch` and `commit_sha`.
   If the user chooses "명령어만 보기", show the command block above and stop. In non-interactive mode, use the registry safe default "명령어만 보기" and never run `git init` automatically.

2. **Pre-flight version check**:

   ```bash
   echo '[deploy:Step 2 preflight] entered' >&2
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

   On `cli_too_old: true` or `cli_too_new: true`, halt and surface the corresponding entry from `references/error-empathy-catalog.md` ("version-skew"). Do not proceed.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — Step 3 preview → `--dry-run` (가장 안전해요), Step 6 exit-65 → `abort` (subprocess 자동 로그인 안 해요).

3. **Render preview card via AskUserQuestion**. The card MUST echo all five identity fields verbatim in Korean:

   ```
   다음을 실행할게요:
   ① 앱:    paydrop (id=42)
   ② 환경:  production (https://hub-api.jocodingax.ai)
   ③ 브랜치: main
   ④ 커밋:  a3f9c1b — "결제 페이지 버그 수정" (12분 전 푸시, you)
   ⑤ 예상:  약 3분 소요

   진행할까요? [네 / 아니요 / 미리보기만 (--dry-run)]
   ```

   Use the template in `references/error-empathy-catalog.md` ("deploy-preview"). Apply NFKC normalize to displayed slug; if NFKC altered the string, surface a warning.

   Then ask with structured AskUserQuestion JSON:

   ```json
   {
     "question": "진행할까요?",
     "header": "배포 확인",
     "options": [
       {
         "label": "네, 배포",
         "value": "approve",
         "description": "consent token 을 만들고 실제 배포를 시작해요."
       },
       {
         "label": "미리보기만",
         "value": "dry_run",
         "description": "--dry-run 으로 실제 배포 없이 확인해요."
       },
       {
         "label": "취소",
         "value": "abort",
         "description": "배포를 멈춰요."
       }
     ]
   }
   ```

   If the user chooses `dry_run`, add `--dry-run` to Step 4 and skip Step 5. If the user chooses `abort`, stop without minting consent.

4. **On user approval**, mint a consent token and run deploy. Run this step only when Step 1.1 did not already execute and record `deploy_create`; never double-submit a deploy for the same pending bootstrap action.

   ```bash
   echo '[deploy:Step 4 consent-deploy] entered' >&2
   CONSENT_PROFILE=""
   PROFILE_FLAG=()
   if [ -n "${PROFILE:-}" ] && [ "${PROFILE:-}" != "default" ]; then
     CONSENT_PROFILE="$PROFILE"
     PROFILE_FLAG=(--profile "$PROFILE")
   fi
   cat <<JSON | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   {"tool_call_id":"pending","action":"deploy_create","app_id":"${APP_ID}","profile":"${CONSENT_PROFILE}","branch":"${BRANCH}","commit_sha":"${COMMIT_SHA}","context":{}}
   JSON

   axhub deploy create --app "$APP_ID" "${PROFILE_FLAG[@]}" --branch "$BRANCH" --commit "$COMMIT_SHA" --json
   ```

   The next Bash tool call id is created by Claude after consent-mint runs, so never invent `${NEXT_BASH_TOOL_CALL_ID}`, never set a fake `CLAUDE_SESSION_ID`, and never clear the real session env just to mint consent. `tool_call_id:"pending"` explicitly mints a short-lived pending token; the PreToolUse hook claims it once only when action/app/profile/branch/commit/context all match. If the token is absent, already used, expired, or non-matching, the command is blocked. This avoids POSIX-only session-unset commands and keeps the flow portable across macOS/Linux/Windows Claude Code environments.

5. **Post-deploy chain** — capture `.id` from the deploy create JSON, then auto-follow:

   ```bash
   echo '[deploy:Step 5 status-chain] entered' >&2
   axhub deploy status dep_$DEPLOY_ID $WATCH --json
   ```

   **Non-interactive guard:** If running in non-interactive context (`$CI` or `$CLAUDE_NON_INTERACTIVE` env var set, OR no TTY, OR `claude -p` invocation), DROP `--watch` flag and render single snapshot — `--watch` blocks indefinitely in headless/subprocess mode and `/axhub:deploy` post-chain hangs forever (same root cause as v0.1.12 status/logs hotfix). Detection: `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi` then use `$WATCH`.

   Render humanized Korean progress every ~30s ("1분 경과, 빌드 중이에요 (정상)") per `references/recovery-flows.md` ("watch-narration").

6. **On any non-zero exit**, route to `references/error-empathy-catalog.md` by exit code:
   - exit 64 + `validation.deployment_in_progress` → 4-part Korean copy: "다른 배포가 진행 중이에요. 앱은 안전해요. 5분만 기다리면 자동으로 다음 배포가 가능해요." Never retry. Offer to watch the in-flight deploy instead.
   - exit 64/67 + `github.git_connection_required`, `github.git_connection_not_found`, `git_connection_required`, or CLI stderr containing "GitHub 저장소 연결" → do not ask "지금 GitHub repo 연결 진행할까요?" and do not ask the user to invoke `/axhub:github`. Immediately show a direct GitHub connection block:

     ```bash
     echo '[deploy:Step 6 github-link] entered' >&2
     axhub github repos list --json
     ```

     Render the first `install_url` from that output as `GitHub 연결 링크: <install_url>` so the user can grant repo access directly. If the repo itself does not exist yet, also show `GitHub repo 만들기: https://github.com/new?name=$APP_SLUG` as context only. Then route into `skills/github/SKILL.md` guided setup/connect; do not end with a manual connect command as the next step. GitHub guided setup/connect owns repo create, remote add, first push, and connect consent.

     ```bash
     axhub github connect "$APP_ID" --repo "$OWNER_REPO" --branch "$BRANCH" --account "$ACCOUNT" --json
     ```

     Do not present the command above as the user's next manual command. It is the final command that the GitHub skill may run only after its guided ladder verifies repo visibility and mints consent. If the account is already installed and the desired repo appears in `axhub github repos list --account "$ACCOUNT" --json`, tell the user the repo is ready and route directly to `skills/github/SKILL.md` Step 4 consent-connect without another yes/no handoff.
   - exit 65 → token expired template + AskUserQuestion to run auth login
   - exit 67 → resource not found + did-you-mean suggestion from apps list
   - exit 68 → rate limit + Retry-After backoff
   - exit 1 → transport error; retry at most once for read paths, never for create

7. **Dry-run NL trigger** — if the user said "한번 해보기만", "리허설", "테스트로", "진짜 안 올리고", add `--dry-run` to step 4 and skip step 5.

8. **Cache last-deploy for statusline (Phase 17 US-1707).** After Step 5 terminal status, write the deploy summary so statusline readers can show it across sessions. The Bash block below is for POSIX/Git Bash/WSL tool execution; native Windows statusLine wiring must use the documented helper/PowerShell path only after the Windows packaging spike promotes it:

   ```bash
   echo '[deploy:Step 8 statusline-cache] entered' >&2
   mkdir -p ~/.cache/axhub-plugin
   cat > ~/.cache/axhub-plugin/last-deploy.json <<JSON
   {"deployment_id":"$DEPLOY_ID","status":"$TERMINAL_STATUS","commit_sha":"$COMMIT_SHA","app_slug":"$APP_SLUG","timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
   JSON
   ```

   Skip on `--dry-run` (statusline 은 실제 deploy 만 추적).

## v0.2.0 command coverage polish

### deploy list

Read-only deployment browsing uses the current CLI command:

```bash
axhub deploy list --app "$APP_ID" --json
```

If pagination appears in JSON, show the first page and offer a follow-up instead of dumping a long list.

### deploy cancel

Cancel is a mutation. Preview the in-progress deployment first:

- app id / slug
- deployment id
- branch and commit if present
- current status
- expected effect

Mint consent with stdin JSON using `action=deploy_cancel`, top-level `app_id`, and `context={deployment_id}` and then run:

```bash
axhub deploy cancel "$DEPLOYMENT_ID" --app "$APP_ID" --yes --json
```

After cancellation, run a read-only status check and summarize the terminal state.

## NEVER

- NEVER treat `axhub-helpers bootstrap --auto-chain --json` as approval; it is only a plan/record FSM.
- NEVER retry `apps_create` or `deploy_create` unless bootstrap returns a confirmed idempotency key and retry policy that allows retry.
- NEVER skip `bootstrap --record` after a returned top-level destructive command finishes; pending action correlation is the audit trail.

- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop `--json` (parsing relies on it).
- NEVER call `axhub deploy create` without going through `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint` first; the PreToolUse hook will deny.
- NEVER call `axhub deploy cancel` without a matching `deploy_cancel` consent token.
- NEVER infer `app_id` from `pwd` or git remote alone in the mutation path; always live resolve through the helper.
- NEVER bypass the AskUserQuestion preview card on slash invocation; slash is explicit consent for the SKILL invocation, not for the destructive operation.

## Additional Resources

For Korean trigger lexicon (informal, honorific, demo-context variants): `references/nl-lexicon.md`.
For exit-code → 4-part Korean error template (emotion + cause + action + button): `references/error-empathy-catalog.md`.
For multi-machine cold cache, headless/Codespaces, version skew, watch narration: `references/recovery-flows.md`.
For working transcripts, use captured `.omc/evidence/` pilot logs; no standalone example transcript files ship in this plugin.
For privacy filter on apis list: `../apis/references/privacy-filter.md` (used by sibling apis skill).
