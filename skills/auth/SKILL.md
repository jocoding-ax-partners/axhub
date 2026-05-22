---
name: auth
description: '이 스킬은 사용자가 로그인, 로그아웃, 토큰 상태, 또는 현재 계정 identity 를 묻거나 변경할 때 사용합니다. 다음 표현에서 활성화: "계정", "권한", "권한이 없대", "누구로", "누구로 접속하는 거야", "누구야", "다시 로그인", "다시 로그인해주세요", "로그아웃", "로그아웃해", "로그인", "로그인 됐어", "로그인 상태", "로그인 상태 알려주세요", "로그인해", "로그인해주세요", "로그인해줘", "어떤 계정으로 접속 중인가요", "어떤 계정이야", "인증", "인증 다시", "토큰", "토큰 갱신해줘", "토큰 만료됐어", "토큰 살아있어", "토큰 지워줘", "scope 없대", "auth", "authenticate", "log in", "log out", "login", "logout", "refresh token", "scope", "sign in", "sign out", "token expired", "who am I", "who am i", "whoami", 또는 axhub identity 관리 의도. 헤드리스 환경 (Codespaces, SSH) 을 자동 감지하여 브라우저 사용 불가 시 토큰 붙여넣기 흐름으로 전환합니다.'
examples:
  - utterance: "로그인 만료 같아"
    intent: "authenticate to axhub"
  - utterance: "로그인"
    intent: "authenticate to axhub"
  - utterance: "login"
    intent: "authenticate to axhub"
  - utterance: "auth"
    intent: "authenticate to axhub"
  - utterance: "다시 로그인"
    intent: "authenticate to axhub"
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Auth (login / logout / status / refresh / pat)

Manage axhub identity. Always check current state first via `axhub auth status` to avoid prompting the user for a login they already have. `axhub auth whoami` (alias `axhub auth me`) 는 identity-only 쿼리로 status 와 동일 출력 — "누구야" / "who am I" 발화에는 whoami 를 써요.

## Workflow

To handle auth:

1. **Check current state first:**

   ```bash
   axhub auth status --json
   # 또는 identity-only:
   axhub auth whoami --json    # alias: axhub auth me --json
   ```

   Parse the result to discriminate four cases:
   - `user_email` present → currently logged in; show identity + scopes + expiry
   - `code: token_expired` → token expired; flow to refresh (Step 3a) 우선
   - `code: not_logged_in` → never logged in; flow to login (Step 3b)
   - `code: ...` other → surface the helper's classify-exit template
   - `auth_mode: "pat"` → PAT context; flow to PAT identity card (Step 2b)

2. **On "logged in" (status query intent).** Render Korean identity card:

   ```
   현재 로그인:
     · 계정: <user_email>  (user_id: <user_id>)
     · 이름: <name>                          # name 이 있으면 표시, 없으면 줄 생략
     · 만료: <EXPIRES_HUMAN>
     · 권한: <scopes joined by ", ">
     · 환경: <profile> (<endpoint>)
     · Platform admin: 네                    # platform_admin=true 일 때만 표시
   ```

   tenants 가 있으면 아래에 이어서:

   ```
   소속 tenants:
     - <tenant_slug or tenant_name>
     - ...
   ```

   tenants 가 비어 있으면 `소속 tenants: 없음` 한 줄 표시. Stop here unless the user also asked for a re-login.

2b. **PAT context identity card** (when `auth_mode=pat` in status output):

   ```
   현재 인증: PAT (X-Api-Key)
     · 계정: <user_email>  (user_id: <user_id>)
     · 출처: <env:AXHUB_API_KEY | env:AXHUB_PAT_ID | profile:current_pat | keychain:current_pat>
     · Platform admin: 네                    # platform_admin=true 일 때만 표시
   ```

   PAT mode 에서는 expires / scopes 정보가 OAuth status 와 다르게 표시 안 돼요. PAT 관리 (issue / list / revoke / rotate) 가 필요하면 Step 8 PAT 섹션 참고.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — login confirm → `abort`, headless → `token_file`, logout confirm → `abort` (subprocess 자동 logout 안 해요), PAT revoke confirm → `abort`.

3. **On expired or login-intent.** Token 이 expired 면 먼저 refresh 를 시도해요 (Step 3a) — full device flow 보다 friction 0. refresh 불가능 / invalid_grant 만 full login 으로 (Step 3b).

3a. **Try `axhub auth refresh` first (token expired path).** Stored refresh_token 이 있으면 device flow 없이 새 access_token 발급 가능해요. CLI 가 자동으로 invalid_grant 일 때만 device-flow fallback 해요.

   ```bash
   axhub auth refresh --json
   ```

   성공 시: 새 token 으로 자동 진행, Step 2 identity card 로 마무리. 사용자에게 추가 prompt 필요 없어요.

   실패 시:
   - `invalid_grant` → refresh_token 이 revoked/expired. CLI 가 자동으로 device flow 로 fallback 하지만, `--no-input` / headless 컨텍스트면 SKILL 이 Step 3b token_file 흐름으로 이동
   - 5xx / 429 / timeout (transient) → exit 6 (rate-limited) / exit 7 (server error) / exit 10 (timeout) 으로 끝나요. CI / agent 는 retry, 사용자에게 한 줄 안내
   - 그 외 → Step 3b 로 이동

3b. **Confirm full login** (refresh 불가 또는 사용자가 명시적으로 "다시 로그인" 요청 시):

   ```json
   {
     "question": "다시 로그인할래요?",
     "header": "로그인 방법",
     "options": [
       {"label": "네, 브라우저로 로그인", "value": "browser", "description": "axhub auth login (OAuth Device Flow)"},
       {"label": "토큰 파일로 로그인 (헤드리스)", "value": "token_file", "description": "Codespaces나 SSH 환경"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

4. **Detect headless environment.** If `$CODESPACES`, `$SSH_TTY` without `$DISPLAY`, or no `open`/`xdg-open` on PATH, default the AskUserQuestion to `token_file` and skip the browser option entirely. Follow `../deploy/references/recovery-flows.md` ("headless-auth").

   Token 소스 우선순위 (헤드리스):
   - 1순위: 환경변수 직접 설정. POSIX/Git Bash/WSL 은 `export AXHUB_TOKEN=axhub_pat_...`, Windows PowerShell 은 `$env:AXHUB_TOKEN='axhub_pat_...'` 를 사용해요.
   - 2순위: 별도 노트북에서 `axhub auth login` 실행 후, 그 노트북의 keychain에서 토큰 추출 → secure 채널 (Slack DM, secure email) 로 헤드리스 환경에 전달 → 현재 shell 에 맞게 `AXHUB_TOKEN` 설정
   - 3순위: pasted token을 helper 로 import. POSIX/Git Bash/WSL 은 `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers token-import`, Windows PowerShell 은 `& "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" token-import` 를 사용해요.

   **Token 자동 추출 메커니즘:** 브라우저 환경에서는 `axhub auth login` 한 번 실행 후, 헬퍼의 `token-init` 서브커맨드가 macOS keychain / Linux secret-service / Windows Credential Manager (PowerShell + Add-Type 단일 호출) 에서 `axhub` CLI 가 저장한 토큰을 자동 추출해요. vibe coder 가 별도 토큰 setup 단계를 볼 일 없어요.

5. **On `browser`** (browser available):

   **5a. Mint consent token with the current shell lane** (PreToolUse gate requires it before any `axhub auth login` command). Do not use one POSIX-only pipe for every environment.

   POSIX/Git Bash/WSL lane:

   ```bash
   echo '{"tool_call_id":"pending","action":"auth_login","app_id":"_","profile":"'"${AXHUB_PROFILE:-default}"'","branch":"_","commit_sha":"_","context":{}}' \
     | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   ```

   PowerShell lane:

   ```powershell
   $AxhubHelper = $null
   if ($env:CLAUDE_PLUGIN_ROOT) {
     $PluginHelper = Join-Path $env:CLAUDE_PLUGIN_ROOT "bin\axhub-helpers.exe"
     if (Test-Path $PluginHelper) { $AxhubHelper = $PluginHelper }
   }
   if (-not $AxhubHelper) {
     $HelperCommand = Get-Command axhub-helpers.exe -ErrorAction SilentlyContinue
     if ($HelperCommand) { $AxhubHelper = $HelperCommand.Source }
   }
   if (-not $AxhubHelper) {
     throw "axhub-helpers.exe 를 찾지 못했어요. axhub doctor 로 plugin helper 설치 상태를 확인해요."
   }
   @{
     tool_call_id = "pending"
     action = "auth_login"
     app_id = "_"
     profile = if ($env:AXHUB_PROFILE) { $env:AXHUB_PROFILE } else { "default" }
     branch = "_"
     commit_sha = "_"
     context = @{}
   } | ConvertTo-Json -Compress | & $AxhubHelper consent-mint
   ```

   PowerShell 에서도 `CLAUDE_PLUGIN_ROOT` 가 비어 있으면 PATH 의 `axhub-helpers.exe` 를 자동으로 찾아요. temp-file fallback 은 위 두 stdin lane 을 쓸 수 없을 때만 secondary 로 써요. JSON 파일을 만들더라도 raw token 값을 쓰지 말고, consent JSON 만 0600/사용자 전용 ACL 임시 파일에 저장한 뒤 helper stdin 으로 다시 넣어요.

   `auth_login` binding은 실제 app/branch/commit이 필요 없지만 `asConsentBinding`이 모든 필드에서 비어있지 않은 문자열을 요구하므로 `"_"`를 플레이스홀더로 사용해요. 다음 Bash/PowerShell tool id는 consent-mint 이후에 생기므로 `pending` token을 한 번만 쓰게 해요.
   macOS/Linux/Windows 모두에서 `CLAUDE_SESSION_ID`를 지우지 마세요. `tool_call_id:"pending"` 자체가 helper에게 "다음 실제 tool call에서 한 번만 claim"하라는 명시 신호예요.

   **5b. Surface OAuth challenge before waiting.** `axhub auth login` 은 device flow URL + code 를 stderr 로 emit 한 뒤 사용자 승인을 기다리며 block 해요. SKILL 이 sync 로 호출하면 Claude Code 가 명령 종료까지 stdout buffer 를 surface 안 해서 사용자 화면에 URL 이 안 보여요. 반드시 다음 wrapper 패턴으로 호출해요 (URL/code 를 먼저 추출해서 사용자에게 보여준 뒤 wait):

   ```bash
   # 기본 (default scopes / 단일 tenant)
   AUTH_EXTRA=""
   # 다중 tenant 소속이면:
   #   AUTH_EXTRA="--tenant <tenant-slug>"
   # scope 변경하면 (default: read,write):
   #   AUTH_EXTRA="--scopes read,write,deploy"

   AUTH_LOG=$(mktemp -t axhub-auth-XXXXXX)
   # nohup + disown 으로 login 을 detach. bash 종료 후에도 OAuth polling 계속해요.
   nohup axhub auth login --force --no-browser $AUTH_EXTRA >"$AUTH_LOG" 2>&1 </dev/null &
   AUTH_PID=$!
   disown 2>/dev/null || true

   URL=""; CODE=""
   for _ in $(seq 1 30); do
     if [ -s "$AUTH_LOG" ]; then
       URL=$(grep -oE 'https?://[A-Za-z0-9._~+%/?=#-]+' "$AUTH_LOG" | head -1)
       CODE=$(grep -oE '[A-Z0-9]{4}-[A-Z0-9]{4}' "$AUTH_LOG" | head -1)
       [ -n "$URL" ] && [ -n "$CODE" ] && break
     fi
     kill -0 "$AUTH_PID" 2>/dev/null || break
     sleep 0.5
   done

   if [ -n "$URL" ] && [ -n "$CODE" ]; then
     printf '\naxhub OAuth 인증이 필요해요. 다음 두 단계 진행해주세요:\n\n  1. 브라우저에서 열기: %s\n  2. 코드 입력: %s\n\n승인 후 잠시 기다리면 자동으로 토큰을 받아요.\n\n[axhub] AUTH_PID=%s AUTH_LOG=%s (다음 step 의 poll 에서 사용)\n' "$URL" "$CODE" "$AUTH_PID" "$AUTH_LOG"
     exit 0
   else
     echo "OAuth device URL/code 를 15초 안에 추출 못 했어요. CLI 출력 형식이 바뀌었을 가능성. /axhub:doctor 로 진단해주세요."
     kill "$AUTH_PID" 2>/dev/null
     rm -f "$AUTH_LOG"
     exit 1
   fi
   ```

   **5c. Poll auth status until login completes.** 위 step 5b 는 URL surface 직후 exit 0 으로 빠져나와요 (Claude Code 가 stdout 을 즉시 사용자에게 보여줌). login process 는 detach 돼서 백그라운드 OAuth polling 계속해요. 사용자가 브라우저에서 승인하면 토큰 저장돼요. 다음 별도 bash call 로 완료 확인:

   ```bash
   # 최대 5분 (60회 × 5s) auth status 폴링
   for _ in $(seq 1 60); do
     EMAIL=$(axhub auth status --json 2>/dev/null | jq -r '.user_email // empty')
     if [ -n "$EMAIL" ]; then
       printf '인증 완료: %s\n' "$EMAIL"
       exit 0
     fi
     sleep 5
   done
   echo "OAuth 승인 5분 안 들어왔어요. 다시 시도하려면 /axhub:auth 호출."
   exit 64
   ```

   poll loop 이 너무 길게 안 block 되도록 5분 cap. 사용자가 5분 내 브라우저 승인 안 하면 안내 후 종료. (background login process 는 자체 15분 timeout 으로 곧 정리됨.)

   **왜 wrapper 가 필요해요?** Claude Code shell tool 은 명령 종료 후 한 번에 output 을 surface 해요. `axhub auth login --no-browser` 는 OAuth device flow polling 으로 최대 15분 block 되니까 sync 호출 + wait 패턴은 URL 을 사용자가 못 봐요. 새 wrapper 는 (1) nohup + disown 으로 login 을 detach 해서 bash 가 빠르게 exit, (2) exit 직전 URL/code 추출해서 stdout 으로 surface, (3) 후속 step 에서 auth status polling 으로 완료 확인 — 3 단계 분리로 사용자 화면에 URL 이 즉시 보여요.

   `--json 은 challenge fields` (`device_url`, `user_code`, verification URL, or equivalent) 를 polling 전에 emit 할 때만 사용해요. 현재 CLI v1.0.0-rc.1 의 `--json` 은 polling 완료 후 한 번에 결과 envelope 만 emit 하니까 interactive wait 에는 사용 금지예요 (challenge surface 못 함).

   Never run a blocking login unless the device URL/code is visible first: do not run blocking `axhub auth login --force` and then wait silently. If the current CLI cannot expose the challenge pre-wait, stop with a CLI follow-up gap instead of improvising a hidden/blocking auth flow.

   PreToolUse hook이 step 5a에서 발급한 consent token을 검증해요. 유효하면 OAuth Device Flow가 시작돼요. 만료되거나 없으면 hook이 한국어 메시지와 함께 deny 해요.

   After completion, re-run `axhub auth status --json` and render the identity card from step 2.

6. **Logout intent.** When user says "로그아웃", "토큰 지워줘", "세션 끊어":

   confirm AskUserQuestion 으로 사용자 의도를 먼저 확인해요:

   ```json
   {
     "question": "로그아웃할래요?",
     "header": "로그아웃 확인",
     "options": [
       {"label": "네, 로그아웃", "value": "confirm", "description": "이 노트북의 axhub 토큰을 제거 (실행 전 dry-run preview 표시)"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

   사용자가 `confirm` 을 선택하면 destructive 실행 직전에 dry-run 으로 미리 보여줘요:

   ```bash
   axhub auth logout --dry-run --json
   ```

   `would_delete: true` / `profile: <name>` 출력을 사용자에게 한국어 한 줄로 요약 후 (예: "프로필 `default` 의 토큰이 제거돼요") 실제 실행:

   ```bash
   axhub auth logout
   ```

   Confirm to user: "로그아웃 완료. 이 노트북에서 토큰이 제거됐어요. 다른 노트북은 영향 없어요."

7. **Show scopes after success.** Always echo `scopes` from the post-login `auth status` so the user sees what they can/cannot do (prevents downstream exit 66 surprises).

8. **PAT (Personal Access Token) management** — 사용자가 "PAT 발급", "토큰 발급", "agent token", "automation token", "CI 토큰" 등을 요청하거나, PAT context 에서 관리 작업 (list/revoke/rotate) 을 요청할 때 사용해요. PAT 는 X-Api-Key 인증 헤더로 동작하고 OAuth session 과 별도 storage 에 보관돼요.

   8a. **List PATs:**

   ```bash
   axhub auth pat list --json
   ```

   출력의 `id` / `name` / `revoked_at` 을 한국어 한 줄씩 요약. revoked 는 "(폐기됨)" 로 표시.

   8b. **Issue a new PAT** (consent + raw-token 1회 표시):

   ```bash
   axhub auth pat issue --name "<descriptive-name>" --expires-in-days 90 --json
   # 즉시 활성 PAT 으로 사용하려면:
   axhub auth pat issue --name "<n>" --expires-in-days 90 --use --json
   # raw token 저장 없이 1회만 보여주려면:
   axhub auth pat issue --name "<n>" --no-save --show-token
   ```

   **raw_token 은 응답 출력에 1회만 나타나요.** SKILL output / chat / log 에 echo 금지 (NEVER 섹션 참고). 사용자에게는 `id` / `fingerprint` / `expires_at` 만 표시. raw 는 keychain 에 저장되거나 `--show-token` 인 경우 stdout 으로 1회 표시되고 그 다음부터 다시 못 봐요.

   8c. **Revoke a PAT** (dry-run 기본, `--execute` 로 mutate):

   ```bash
   # preview:
   axhub auth pat revoke <pat-id> --json
   # 실제 폐기:
   axhub auth pat revoke <pat-id> --execute --json
   ```

   `--execute` 없이 호출하면 dry-run 이라 mutate 하지 않아요. confirm AskUserQuestion 후 `--execute` 붙여 실행:

   ```json
   {
     "question": "PAT <id> 를 폐기할까요?",
     "header": "PAT 폐기 확인",
     "options": [
       {"label": "네, 폐기", "value": "confirm", "description": "백엔드에서 PAT 를 revoke 하고 keychain 에서도 제거해요"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

   8d. **Rotate active PAT** (replace + revoke old):

   ```bash
   axhub auth pat rotate --name "<new-name>" --expires-in-days 90 --json
   ```

   활성 PAT 을 새 PAT 으로 교체하고 old 는 자동 revoke. raw token 은 issue 때와 동일하게 1회만 표시.

   8e. **Switch active PAT** (saved PATs 중 선택):

   ```bash
   axhub auth pat use <pat-id>
   axhub auth pat unset            # 활성 PAT 해제
   ```

   8f. **PAT whoami** (출처 표시):

   ```bash
   axhub auth pat whoami --json
   ```

   `source` 필드 (`env:AXHUB_API_KEY` / `env:AXHUB_PAT_ID` / `profile:current_pat` / `keychain:current_pat`) 와 `fingerprint` 를 사용자에게 표시. raw token 노출 X.

## NEVER

- NEVER echo the raw token value (`axhub_pat_*`) — the redact helper masks it but skill output must not interpolate it back.
- NEVER raw PAT token (`pat issue` / `pat rotate` 의 `raw_token` 응답 필드) 을 SKILL output / chat / log 에 echo 안 해요. 1회 표시는 CLI 가 처리하고 사용자가 직접 복사해요.
- NEVER auto-launch browser in headless environments — the CLI will block and confuse the user.
- NEVER call `axhub auth login` without first checking `auth status` (avoids re-login when already valid).
- NEVER token 이 expired 일 때 곧바로 full device-flow login 강제 안 해요. `axhub auth refresh` 가 먼저 시도되고, invalid_grant 일 때만 full login fallback 해요.
- NEVER persist tokens outside `~/.config/axhub-plugin/token` (0600).
- NEVER call `axhub auth logout` without confirming via AskUserQuestion (destructive — kills active session). dry-run preview 를 먼저 보여주세요.
- NEVER call `axhub auth pat revoke` without `--execute` 를 mutate intent 로 가정 안 해요. dry-run 이 기본이라 `--execute` 가 빠지면 backend revoke 발생 안 해요.
- NEVER call `axhub auth login` without running the stdin JSON `consent-mint` step (step 5a) first — PreToolUse hook이 consent token 없이 deny 해요.
- NEVER OAuth device flow 의 `verification_uri` + `user_code` 를 사용자에게 안 보여주지 않아요. CLI 가 stderr "To continue, visit: …" / "Enter code: …" 줄을 emit 한 직후 한국어로 묶어서 표시.

## Additional Resources

For Korean trigger lexicon (auth intent): `../deploy/references/nl-lexicon.md` (sections 6a/6b/6c).
For 4-part Korean exit templates (exit 65, exit 66): `../deploy/references/error-empathy-catalog.md`.
For headless-auth + token-paste flow: `../deploy/references/recovery-flows.md` ("headless-auth").
For headless / Codespaces / SSH auth fallback (token-paste flow), see `../deploy/references/headless-flow.md`.
For expires_at humanization rule: `../deploy/references/time-render.md`.
