# Headless Auth Flow — Codespaces / SSH / no-DISPLAY

Detection rules and token-paste fallback for environments where `axhub auth login` cannot open a browser. Implements PLAN E12 fix (§16.16 multi-tenant credential isolation companion). Sibling-aware: this file expands the `headless-auth` flow originally summarized in `recovery-flows.md` §2; that section remains the cross-skill recovery entrypoint and is left intact.

All user-facing copy is Korean. All commands assume `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` is on PATH.

---

## 1. Headless 환경 감지 (탐지 우선순위)

The SessionStart hook resolves headlessness once per session and writes the verdict to `~/.cache/axhub-plugin/env.json`. Skills MUST NOT re-detect ad-hoc — read the cache and trust it.

```json
{
  "headless": true,
  "reason": "CODESPACES env var detected",
  "browser_available": false,
  "open_command": null,
  "detected_at": "2026-04-23T10:14:32Z"
}
```

### Detection ladder (first match wins)

The helper evaluates these in strict order. Stop at the first hit; record the matching `reason` verbatim into the cache.

| Order | Condition | `reason` value | Notes |
|---|---|---|---|
| 1 | `$CODESPACES` is set (any non-empty value) | `"CODESPACES env var detected"` | GitHub Codespaces sets this; trust it absolutely. Even if a `$DISPLAY` exists (rare), it's a forwarded one and the OAuth callback won't reach Codespaces. |
| 2 | `$SSH_TTY` is set AND `$DISPLAY` is empty | `"SSH session without X11 forwarding"` | classic remote shell. X11 forwarding (`ssh -Y`) sets `$DISPLAY`, so this catches the no-forwarding case. |
| 3 | macOS (`uname -s` = `Darwin`) AND `command -v open` fails | `"macOS without 'open' command"` | extremely rare (broken PATH); included for completeness. |
| 4 | Linux (`uname -s` = `Linux`) AND `command -v xdg-open` fails AND `$DISPLAY` empty | `"Linux without xdg-open and no DISPLAY"` | typical container or minimal install. |
| 5 | `$AXHUB_FORCE_HEADLESS=1` | `"forced via AXHUB_FORCE_HEADLESS"` | escape hatch for users on weird setups (e.g., GUI exists but they prefer paste flow). |
| — | none of the above | `headless: false` | proceed with normal browser OAuth. |

**Why this order:** `$CODESPACES` first because it's the loudest signal and ships broken `$DISPLAY` values; SSH second because remote sessions are the next most common; OS-specific `open`/`xdg-open` checks last because they're slowest (fork+exec of `command -v`).

The cache is invalidated on every SessionStart. Skills MUST NOT extend its TTL — environment can change between sessions (user opened the same project from a Codespace and then from their laptop). One session = one detection.

---

## 2. Token-paste flow (사용자 가이드 KR)

When `headless: true` and the user's intent is auth, the skill skips the browser path entirely. **Never call `axhub auth login` in this mode** — the CLI will block trying to open a browser, the user will see no signal, and the session hangs. 표준 흐름은 별도 브라우저 노트북에서 로그인 후 keychain 추출 → secure 채널로 토큰 전달, 또는 `AXHUB_TOKEN` 환경변수 직접 export 입니다.

### Korean instructions block

Render this verbatim when entering the token-paste flow. The numbered steps are intentional: vibe coders need explicit "now do this" framing.

```
잠깐만요. 지금 환경 (Codespaces 또는 SSH) 에서는 브라우저를 못 열어요.
대신 별도 노트북에서 토큰을 받아 여기에 붙여넣어 주세요.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
1단계 — 브라우저가 있는 노트북에서:
  터미널을 열고 다음을 순서대로 실행하세요 →

    axhub auth login              # 브라우저 OAuth 로그인
    security find-generic-password -s axhub -w   # macOS keychain 에서 token blob 출력
    # Linux:  secret-tool lookup service axhub

  출력된 'go-keyring-base64:eyJ...' 한 줄을 통째로 복사하세요.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
2단계 — secure 채널로 본인에게 전달:
  Slack DM (자기 자신), secure email, 1Password Send 같은
  암호화 전송 수단을 사용하세요. 평문 채팅 X.

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
3단계 — 지금 이 환경에서:
  아래 입력창에 그 token blob 을 붙여넣어 주세요.
  helper 가 base64 decode → access_token 추출 → 0600 으로 안전 저장합니다.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

(더 간단한 방법) 1단계 노트북에서 본 access_token 평문 (axhub_pat_…) 을
이미 알고 있다면 헤드리스 환경에서 바로:

    export AXHUB_TOKEN=axhub_pat_...

후 작업하면 token-init 이 즉시 그 환경변수를 사용합니다.
```

### AskUserQuestion (text input)

```json
{
  "question": "토큰을 붙여넣어 주세요 (axhub_pat_ 로 시작):",
  "input_type": "text",
  "secret": true
}
```

The `secret: true` flag tells the harness to treat the input as a credential — no echo on screen, no inclusion in transcript replay. If the host doesn't honor `secret`, the redact hook (PLAN E7 / step 3 below) catches the leak as a second line of defense.

### Validation before storage

The helper validates the pasted string before writing:

```bash
${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers token-install --from-stdin
```

Rejection rules (Korean error messages):

| Pattern | Rejection message |
|---|---|
| Doesn't start with `axhub_pat_` | "토큰 형식이 아니에요. 다시 1단계부터 해주세요. 출력 마지막 줄만 복사하면 됩니다." |
| Contains whitespace mid-string | "토큰 가운데에 공백이 들어갔어요. 줄바꿈 없이 한 줄로 붙여넣어 주세요." |
| Length < 32 chars | "토큰이 너무 짧아요. 잘려서 복사된 것 같아요. 다시 복사해주세요." |
| Already exists at `~/.config/axhub/token` | (after AskUserQuestion confirm) "기존 토큰을 덮어쓸까요?" — yes/cancel |

On success: `axhub auth status --json --token-file ~/.config/axhub/token` runs immediately to confirm the token works. If `auth status` returns `code: token_invalid`, the file is deleted and the user is told the token failed validation (likely expired between issuance and paste). Re-enter the flow from step 1.

---

## 3. Token storage 보안 (`~/.config/axhub/token` 0600, redact when echoing)

### File mode

The token file MUST be created with mode `0600` (owner read+write only). The helper uses `umask 077` before the write to defend against race conditions where the file briefly exists at default mode:

```ts
// Inside ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers token-install
const oldMask = process.umask(0o077);
try {
  await Bun.write(tokenPath, tokenString, { mode: 0o600 });
} finally {
  process.umask(oldMask);
}
// Verify post-write
const stat = await Bun.file(tokenPath).stat();
if ((stat.mode & 0o077) !== 0) {
  throw new Error("token file mode check failed: world/group bits set");
}
```

The post-write `mode` check is mandatory. On some filesystems (NFS, certain Docker overlay setups) the requested mode is silently downgraded. Failing post-check means the helper deletes the file and reports a Korean error: "토큰 파일 권한 설정에 실패했어요. 이 환경에서는 토큰 파일을 안전하게 저장할 수 없어요. (NFS / Docker overlay?)"

### Storage location rules

- Default: `~/.config/axhub/token` (XDG-compliant, per-user).
- Codespaces / SSH: same path; Codespaces persists `~/.config/` across rebuilds.
- Shared machine policy (PLAN §16.16): if the helper detects shared-machine markers (multiple `$HOME` users in `/Users` or `/home` AND `$AXHUB_PROFILE != "personal"`), it warns: "이 노트북은 여러 사람이 쓰는 것 같아요. 작업 끝나면 'axhub auth logout' 으로 토큰 지우는 걸 잊지 마세요."
- NEVER write to `/tmp`, `/var/tmp`, or any path outside `$HOME/.config/axhub/`.
- NEVER duplicate the token to another path "for backup" — single source of truth.

### Redaction on echo (PLAN E7)

Any `axhub_pat_*` string MUST be redacted before reaching the transcript. The hook layer does this in two places:

1. **PostToolUse `tool_response` redaction** — applied to every Bash output before classification:
   ```regex
   /axhub_pat_[A-Za-z0-9_-]{16,}/g  →  "axhub_pat_[redacted]"
   ```
2. **Skill output redaction** — when a skill renders status, error, or success cards, the `axhub-helpers redact` filter is the LAST step in the pipeline, after all formatting:
   ```bash
   ... | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers redact
   ```

Skills MUST NOT interpolate the token value into their own templates even when masked — once the literal value is in skill markdown's variable scope, a future edit can leak it. The token never leaves the helper's memory; the helper hands back `auth status` output (which contains `user_email`, `scopes`, `expires_at` but never the raw token).

---

## 4. Recovery from auth-then-stuck (브라우저 시도 timeout → 자동 fallback)

There's one failure mode the SessionStart detection can miss: **a Codespace where `$DISPLAY` is set by an X11-forwarded VS Code remote tunnel, but the OAuth callback URL `http://localhost:<port>/callback` cannot reach the Codespace's bound port.** Detection passes ("looks browser-capable"), the user runs `axhub auth login`, the browser opens on their laptop, login succeeds — and then the CLI hangs forever waiting for the callback.

### Detection (in skill, not SessionStart)

After invoking `axhub auth login` in a non-headless flow, the skill watches for these signals to trigger fallback:

| Signal | Threshold | Action |
|---|---|---|
| No CLI exit within 90 seconds | `timeout 90 axhub auth login` | force-kill, fall back to token-paste |
| User types "안 돼", "멈췄어", "stuck" while waiting | UserPromptSubmit during in-flight auth | force-kill, fall back to token-paste |
| Process emits "waiting for callback" message > 2 times | stderr line count | force-kill, fall back to token-paste |

### Korean fallback message

When the timeout fires, render this Korean message and route into section 2's token-paste flow:

```
어... 브라우저는 열렸는데 로그인 응답이 여기까지 안 오네요.
이런 경우는 보통 Codespaces 같은 원격 환경에서 콜백 포트가 막혀서 그래요.
당신 잘못 아니에요.

대신 토큰을 직접 붙여넣는 방법으로 바꿀게요. 30초면 끝나요.

(아래 1단계부터 따라와 주세요)
```

Then proceed with section 2 verbatim. The helper records the auto-fallback to `~/.cache/axhub-plugin/env.json`:

```json
{
  "headless": true,
  "reason": "auto-fallback after browser callback timeout",
  "browser_available": false,
  "auto_fallback": true,
  "original_detection": { "headless": false, "reason": "no headless markers" }
}
```

The `auto_fallback: true` field is read by the corpus scorer (`tests/score.ts`) as a signal that the original detection ladder needs strengthening for that environment. After 3 auto-fallbacks for the same `$CODESPACE_NAME` or `$SSH_CLIENT`, the helper auto-adds that environment to the headless detection cache permanently for the user (one less retry next session).

### What NOT to do on stuck-auth

- **NEVER** retry the same browser flow silently — the user will believe their first click "didn't take" and re-authorize, doubling the OAuth token churn.
- **NEVER** delete a partially-written token file from a previous successful login when the new one times out — preserve the old working token until the new one validates.
- **NEVER** offer the user "wait longer?" as an option — vibe coders read that as "I might be doing it wrong, let me try again" and re-click. Cut over to paste flow definitively.

---

## Cross-flow rules

- **Detection is one-shot per session.** SessionStart writes; skills read. No live re-detection unless the auto-fallback in section 4 fires.
- **Token storage is single-path.** Always `~/.config/axhub/token`, mode 0600, with post-write verification.
- **Redaction is double-layered.** PostToolUse hook + skill output filter, both mandatory.
- **Fail-closed on storage errors.** If the token file can't be written safely, the skill refuses the session — never falls back to env-var or memory-only credentials.
- **Sibling reference:** `recovery-flows.md` §2 (`headless-auth`) holds the cross-skill state-machine summary. This file holds the implementation detail. Both must stay in sync — when this file changes substantively, update `recovery-flows.md` §2's cross-link only (not the body).

For the consent state machine that decides when to prompt at all: PLAN E9 / `error-empathy-catalog.md`.
For the single-binary helper this file calls into: PLAN §16.13.
For PLAN reference: E12 (headless-auth fix), §16.16 (multi-tenant credential isolation), E7 (token redaction enforcement), §16.11 (Unicode hardening — applies to user_email display in identity card).
