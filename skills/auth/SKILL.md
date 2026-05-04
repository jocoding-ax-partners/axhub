---
name: auth
description: '이 스킬은 사용자가 로그인, 로그아웃, 토큰 상태, 또는 현재 계정 identity 를 묻거나 변경할 때 사용합니다. 다음 표현에서 활성화: "로그인해", "로그인해줘", "다시 로그인", "토큰 만료됐어", "토큰 갱신해줘", "인증 다시", "권한이 없대", "scope 없대", "누구로 접속하는 거야", "누구야", "어떤 계정이야", "토큰 살아있어", "로그인 됐어", "로그인 상태", "로그아웃해", "토큰 지워줘", "로그인해주세요", "다시 로그인해주세요", "로그인 상태 알려주세요", "어떤 계정으로 접속 중인가요", "login", "log in", "sign in", "logout", "log out", "sign out", "who am I", "whoami", "auth", "authenticate", "scope", "token expired", "refresh token", 또는 axhub identity 관리 의도. 헤드리스 환경 (Codespaces, SSH) 을 자동 감지하여 브라우저 사용 불가 시 토큰 붙여넣기 흐름으로 전환합니다.'
multi-step: false
needs-preflight: false
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

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — login confirm → `abort`, headless → `token_file`, logout confirm → `abort` (subprocess 자동 logout 안 해요).

3. **On expired or login-intent.** Confirm via AskUserQuestion:

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
   - 1순위: `export AXHUB_TOKEN=axhub_pat_...` 환경변수 직접 설정 (가장 간단)
   - 2순위: 별도 노트북에서 `axhub auth login` 실행 후, 그 노트북의 keychain에서 토큰 추출 → secure 채널 (Slack DM, secure email) 로 헤드리스 환경에 전달 → `export AXHUB_TOKEN=...`
   - 3순위: pasted token을 `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers token-import` 로 `~/.config/axhub-plugin/token` 에 mode 0600 저장

   **Token 자동 추출 메커니즘:** 브라우저 환경에서는 `axhub auth login` 한 번 실행 후, 헬퍼의 `token-init` 서브커맨드가 macOS keychain / Linux secret-service / Windows Credential Manager (PowerShell + Add-Type 단일 호출) 에서 `axhub` CLI 가 저장한 토큰을 자동 추출해요. vibe coder 가 별도 토큰 setup 단계를 볼 일 없어요.

5. **On `browser`** (browser available):

   **5a. Mint consent token** (PreToolUse gate requires it before `axhub auth login`):

   ```bash
   echo '{"tool_call_id":"pending","action":"auth_login","app_id":"_","profile":"'"${AXHUB_PROFILE:-default}"'","branch":"_","commit_sha":"_","context":{}}' \
     | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   ```

   `auth_login` binding은 실제 app/branch/commit이 필요 없지만 `asConsentBinding`이 모든 필드에서 비어있지 않은 문자열을 요구하므로 `"_"`를 플레이스홀더로 사용해요. 다음 Bash tool id는 consent-mint 이후에 생기므로 `pending` token을 한 번만 쓰게 해요.
   macOS/Linux/Windows 모두에서 `CLAUDE_SESSION_ID`를 지우지 마세요. `tool_call_id:"pending"` 자체가 helper에게 "다음 실제 tool call에서 한 번만 claim"하라는 명시 신호예요.

   **5b. Run auth login**:

   ```bash
   axhub auth login
   ```

   PreToolUse hook이 step 5a에서 발급한 consent token을 검증해요. 유효하면 브라우저가 열려 OAuth Device Flow가 시작돼요. 만료되거나 없으면 hook이 한국어 메시지와 함께 deny 해요.

   After completion, re-run `axhub auth status --json` and render the identity card from step 2.

6. **Logout intent.** When user says "로그아웃", "토큰 지워줘", "세션 끊어":

   Confirm via AskUserQuestion before deleting the active local token:

   ```json
   {
     "question": "로그아웃할래요?",
     "header": "로그아웃 확인",
     "options": [
       {"label": "네, 로그아웃", "value": "confirm", "description": "이 노트북의 axhub 토큰을 제거"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

   Only when the answer is `confirm`, run:

   ```bash
   axhub auth logout
   ```

   Confirm to user: "로그아웃 완료. 이 노트북에서 토큰이 제거됐어요. 다른 노트북은 영향 없어요."

7. **Show scopes after success.** Always echo `scopes` from the post-login `auth status` so the user sees what they can/cannot do (prevents downstream exit 66 surprises).

## NEVER

- NEVER echo the raw token value (`axhub_pat_*`) — the redact helper masks it but skill output must not interpolate it back.
- NEVER auto-launch browser in headless environments — the CLI will block and confuse the user.
- NEVER call `axhub auth login` without first checking `auth status` (avoids re-login when already valid).
- NEVER persist tokens outside `~/.config/axhub-plugin/token` (0600).
- NEVER call `axhub auth logout` without confirming via AskUserQuestion (destructive — kills active session).
- NEVER call `axhub auth login` without running the stdin JSON `consent-mint` step (step 5a) first — PreToolUse hook이 consent token 없이 deny 해요.

## Additional Resources

For Korean trigger lexicon (auth intent): `../deploy/references/nl-lexicon.md` (sections 6a/6b/6c).
For 4-part Korean exit templates (exit 65, exit 66): `../deploy/references/error-empathy-catalog.md`.
For headless-auth + token-paste flow: `../deploy/references/recovery-flows.md` ("headless-auth").
For headless / Codespaces / SSH auth fallback (token-paste flow), see `../deploy/references/headless-flow.md`.
