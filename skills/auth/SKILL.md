---
name: auth
description: 이 스킬은 사용자가 로그인, 로그아웃, 토큰 상태, 또는 현재 계정 identity 를 묻거나 변경할 때 사용합니다. 다음 표현에서 활성화: "로그인해", "로그인해줘", "다시 로그인", "토큰 만료됐어", "토큰 갱신해줘", "인증 다시", "권한이 없대", "scope 없대", "누구로 접속하는 거야", "누구야", "어떤 계정이야", "토큰 살아있어", "로그인 됐어", "로그인 상태", "로그아웃해", "토큰 지워줘", "로그인해주세요", "다시 로그인해주세요", "로그인 상태 알려주세요", "어떤 계정으로 접속 중인가요", "login", "log in", "sign in", "logout", "log out", "sign out", "who am I", "whoami", "auth", "authenticate", "scope", "token expired", "refresh token", 또는 axhub identity 관리 의도. 헤드리스 환경 (Codespaces, SSH) 을 자동 감지하여 브라우저 사용 불가 시 토큰 붙여넣기 흐름으로 전환합니다.
---

# Auth (login / logout / status)

Manage axhub identity. Always check current state first via `axhub auth status` to avoid prompting the user for a login they already have.

## Workflow

To handle auth:

1. **Check current state first:**

   ```bash
   axhub auth status --json
   ```

   Parse the result to discriminate four cases:
   - `user_email` present → currently logged in; show identity + scopes + expiry
   - `code: token_expired` → token expired; flow to login
   - `code: not_logged_in` → never logged in; flow to login
   - `code: ...` other → surface the helper's classify-exit template

2. **On "logged in" (status query intent).** Render Korean identity card:

   ```
   현재 로그인:
     · 계정: <user_email>
     · 만료: <expires_at> (남은 시간: <DELTA>)
     · 권한: <scopes joined by ", ">
     · 환경: <profile> (<endpoint>)
   ```

   Stop here unless the user also asked for a re-login.

3. **On expired or login-intent.** Confirm via AskUserQuestion:

   ```json
   {
     "question": "다시 로그인할까요?",
     "options": [
       {"label": "네, 브라우저로 로그인", "value": "browser", "description": "axhub auth login (OAuth Device Flow)"},
       {"label": "토큰 파일로 로그인 (헤드리스)", "value": "token_file", "description": "Codespaces나 SSH 환경"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

4. **Detect headless environment.** If `$CODESPACES`, `$SSH_TTY` without `$DISPLAY`, or no `open`/`xdg-open` on PATH, default the AskUserQuestion to `token_file` and skip the browser option entirely. Follow `../deploy/references/recovery-flows.md` ("headless-auth"):

   ```
   1단계 (브라우저 있는 노트북): axhub auth login --print-token
   2단계 (출력된 axhub_pat_... 복사)
   3단계 (여기 환경에 붙여넣기)
   ```

   Save with `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers token-install --from-stdin` (creates `~/.config/axhub/token` with mode 0600).

   **Headless consent-mint:** token-paste flow도 consent-mint가 필요하지 않지만, 향후 `axhub auth login --print-token` Bash 호출이 포함되는 경우 step 5a와 동일하게 `--action auth_login`으로 먼저 mint해야 합니다.

5. **On `browser`** (browser available):

   **5a. Mint consent token** (PreToolUse gate requires it before `axhub auth login`):

   ```bash
   echo '{"tool_call_id":"'"$CLAUDE_SESSION_ID"':'"$NEXT_BASH_TOOL_CALL_ID"'","action":"auth_login","app_id":"_","profile":"'"${AXHUB_PROFILE:-default}"'","branch":"_","commit_sha":"_"}' \
     | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   ```

   `auth_login` binding은 실제 app/branch/commit이 필요 없지만 `asConsentBinding`이 모든 필드에서 비어있지 않은 문자열을 요구하므로 `"_"`를 플레이스홀더로 사용합니다. `tool_call_id`는 반드시 `<session_id>:<tool_call_id>` 형식으로 조합해야 hook의 검증과 일치합니다.

   **5b. Run auth login**:

   ```bash
   axhub auth login
   ```

   PreToolUse hook이 step 5a에서 발급한 consent token을 검증합니다. 유효하면 브라우저가 열려 OAuth Device Flow가 시작됩니다. 만료되거나 없으면 hook이 한국어 메시지와 함께 deny합니다.

   After completion, re-run `axhub auth status --json` and render the identity card from step 2.

6. **Logout intent.** When user says "로그아웃", "토큰 지워줘", "세션 끊어":

   ```bash
   axhub auth logout
   ```

   Confirm to user: "로그아웃 완료. 이 노트북에서 토큰이 제거됐어요. 다른 노트북은 영향 없어요."

7. **Show scopes after success.** Always echo `scopes` from the post-login `auth status` so the user sees what they can/cannot do (prevents downstream exit 66 surprises).

## NEVER

- NEVER echo the raw token value (`axhub_pat_*`) — the redact helper masks it but skill output must not interpolate it back.
- NEVER auto-launch browser in headless environments — the CLI will block and confuse the user.
- NEVER call `axhub auth login` without first checking `auth status` (avoids re-login when already valid).
- NEVER persist tokens outside `~/.config/axhub/token` (0600).
- NEVER call `axhub auth logout` without confirming via AskUserQuestion (destructive — kills active session).
- NEVER call `axhub auth login` without running `consent-mint --action auth_login` (step 5a) first — PreToolUse hook이 consent token 없이 deny합니다.

## Additional Resources

For Korean trigger lexicon (auth intent): `../deploy/references/nl-lexicon.md` (sections 6a/6b/6c).
For 4-part Korean exit templates (exit 65, exit 66): `../deploy/references/error-empathy-catalog.md`.
For headless-auth + token-paste flow: `../deploy/references/recovery-flows.md` ("headless-auth").
For headless / Codespaces / SSH auth fallback (token-paste flow), see `../deploy/references/headless-flow.md`.
