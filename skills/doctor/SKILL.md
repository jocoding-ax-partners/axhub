---
name: doctor
description: '이 스킬은 사용자가 자신의 axhub 설치 또는 환경을 진단하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "axhub 설치돼 있어", "doctor", "닥터", "진단해", "환경 점검", "환경 점검해", "axhub 점검", "헬스체크 해", "잘 깔렸어", "셋업 다 됐어", "설정 봐", "환경 변수 확인해주세요", "설치 상태 알려주세요", "진단 부탁드려요", "환경 점검해주세요", "시스템 상태 확인해주세요", "셋업이 다 끝났나요", "doctor", "check", "diagnose", "health check", "sanity check", "setup check", "env check", 또는 axhub 진단 요청. CLI 버전, 인증 상태, profile, endpoint, scopes 를 보고하고 실패 항목마다 다음에 할 수 있는 자연어 안내를 제공합니다.'
multi-step: true
needs-preflight: false
---

# Doctor (env + install diagnostic)

Run a full axhub plugin health check. Report what's working, what's not, and the next natural-language phrase the user can say to fix each gap.

## Workflow

To run diagnostics:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start:

   ```typescript
   TodoWrite({ todos: [
     { content: "helper binary 점검",            status: "in_progress", activeForm: "helper 보는 중" },
     { content: "axhub CLI 버전 점검",           status: "pending",     activeForm: "CLI 버전 보는 중" },
     { content: "인증 상태 점검",                status: "pending",     activeForm: "인증 보는 중" },
     { content: "profile / endpoint 점검",      status: "pending",     activeForm: "환경 보는 중" },
     { content: "결과 표 출력",                  status: "pending",     activeForm: "표 만드는 중" }
   ]})
   ```

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Detect helper binary with OS-aware install-state rows** (Phase 5 US-503 + Windows helper bootstrap hotfix — `CLAUDE_PLUGIN_ROOT` or PATH may differ per shell):

   Unix / Git Bash:

   ```bash
   command -v axhub-helpers || echo "missing"
   test -x "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" || echo "plugin-local-missing"
   ```

   Windows PowerShell:

   ```powershell
   Get-Command axhub-helpers -ErrorAction SilentlyContinue
   Test-Path "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe"
   Test-Path "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers-windows-amd64.exe"
   Test-Path "$env:CLAUDE_PLUGIN_ROOT\bin\install.ps1"
   ```

   Render helper state as separate rows:

   - `helper PATH`: `command -v axhub-helpers` or `Get-Command axhub-helpers`
   - `helper plugin-local`: `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers` or `$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe`
   - `helper downloaded artifact` (Windows only): `$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers-windows-amd64.exe`
   - `helper installer`: `${CLAUDE_PLUGIN_ROOT}/bin/install.sh` or `$env:CLAUDE_PLUGIN_ROOT\bin\install.ps1`

   **CLAUDE_PLUGIN_ROOT empty fallback** — Claude Code 가 env var 를 propagate 안 했거나 PowerShell session 에서 unset 된 경우, 알려진 cache path 패턴으로 scan:

   - Windows: `$env:USERPROFILE\.claude\plugins\cache\axhub\axhub\*\bin\axhub-helpers.exe`
   - Unix: `$HOME/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers`

   가장 최신 버전 (semver descending) 의 binary 사용. 발견 시 row 는 ✓ 로 표시하되 본문에 사용한 절대경로 명시.

   **Status mapping** (PATH missing 은 plugin-local 가 작동하면 정상 fallback):

   | PATH | plugin-local (또는 cache scan) | 결과 |
   |---|---|---|
   | ✓ found | ✓ found | ✓ helper PATH (정상) |
   | ✗ missing | ✓ found | ✓ helper plugin-local (정상 fallback — PATH 미등록은 plugin design 상 의도) |
   | ✗ missing | ✗ missing | ✗ helper missing — install 안내 + 후속 row skip |

   PATH 미등록 자체는 ⚠ 가 아닌 정상 동작이에요. plugin-local path 는 plugin 설치 위치 (`$CLAUDE_PLUGIN_ROOT/bin/`) 한정이라 user PATH 오염 방지가 의도된 design. plugin-local 또는 cache scan 으로 binary 를 찾을 수 있으면 ✓ 표시하고, 어떤 경로를 사용했는지 본문에 한 줄 명시 ("plugin-local 사용 중: \<path\>").

   Windows missing-helper next action:

   ```powershell
   powershell -NoProfile -ExecutionPolicy Bypass -File "$env:CLAUDE_PLUGIN_ROOT\bin\install.ps1"
   ```

   Unix missing-helper next action:

   ```bash
   bash "${CLAUDE_PLUGIN_ROOT}/bin/install.sh"
   ```

2. **Run preflight** (CLI version range + auth status combined):

   ```bash
   axhub-helpers preflight --json
   ```

   If PATH is missing but plugin-local helper exists, run the plugin-local helper instead:

   ```bash
   "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" preflight --json
   ```

   ```powershell
   & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" preflight --json
   ```

   This returns: `cli_version, in_range, cli_too_old, cli_too_new, cli_present, auth_ok, auth_error_code, scopes, profile, endpoint, user_email, expires_at`.

3. **Fetch raw version + path** for the report:

   ```bash
   # POSIX: macOS / Linux / Git Bash / WSL
   axhub --version
   which axhub
   ```

   ```powershell
   # Windows PowerShell
   axhub --version
   Get-Command axhub
   where.exe axhub
   ```

4. **Render the diagnostic card in Korean.** Use checkmarks (✓ / ✗ / ⚠) per row. Profile/endpoint NULL is ✓ "기본값 사용 중" not ✗ (default state, not a failure):

   ```
   axhub 진단 결과:
     ✓ helper 바이너리: 정상 (axhub-helpers v<HELPER_VERSION>)
     ✓ CLI 설치:        v<CLI_VERSION> (<WHICH_PATH>)
     ✓ 버전 범위:       호환 (필요: v<MIN> ~ v<MAX> 미만)
     ✓ 로그인:          <USER_EMAIL>
     ✓ 만료:            <EXPIRES_AT> (남은 시간: <DELTA>)
     ✓ 권한 (scope):    <SCOPES joined>
     ✓ 환경 (profile):  <PROFILE 또는 "default (기본값 사용 중)">
     ✓ endpoint:        <ENDPOINT 또는 "https://hub-api.jocodingax.ai (기본값)">

   모두 정상이에요. 배포하실 준비 완료!
   ```

5. **On any failure row**, replace ✓ with ✗ and append a one-line fix suggestion as a literal next phrase. Order: failures FIRST (so user sees them), then warnings, then ✓ rows:

   | Failure | Suggested phrase |
   |---|---|
   | helper missing on Unix (PATH + plugin-local both missing) | "axhub-helpers 바이너리가 없어요. 'bash \"${CLAUDE_PLUGIN_ROOT}/bin/install.sh\"' 수동 실행 또는 CC 재시작으로 자동 다운로드 트리거." |
   | helper missing on Windows (PATH + plugin-local both missing) | "axhub-helpers.exe 바이너리가 없어요. Windows native 는 자동 SessionStart 다운로드가 아직 deferred 예요. 'powershell -NoProfile -ExecutionPolicy Bypass -File \"$env:CLAUDE_PLUGIN_ROOT\\bin\\install.ps1\"' 수동 실행으로 복구해요." |
   | Windows artifact exists but `axhub-helpers.exe` missing | "다운로드 artifact 는 있지만 실행 파일 복사가 안 됐어요. 'powershell -NoProfile -ExecutionPolicy Bypass -File \"$env:CLAUDE_PLUGIN_ROOT\\bin\\install.ps1\"' 로 다시 연결해요." |
   | Windows `install.ps1` missing | "install.ps1 이 없어서 플러그인 install 이 손상된 상태예요. '/plugin install axhub@axhub' 로 재설치해요." |
   | `cli_present: false` | "axhub CLI 가 설치되어 있지 않아요." → 즉시 Step 5.5 의 AskUserQuestion 으로 설치 의향 확인 (사용자가 phrase 다시 발화 안 해도 됨) |
   | `cli_too_old: true` | "axhub가 너무 오래된 버전이에요 (v<CUR>). 'axhub 업그레이드해줘' 라고 말씀해주세요." |
   | `cli_too_new: true` | "axhub가 플러그인보다 최신이에요. 'axhub 플러그인 업데이트' 라고 말씀해주세요." |
   | `auth_ok: false` (token_expired) | "로그인이 만료됐어요. '다시 로그인해줘' 라고 말씀해주세요." |
   | `auth_ok: false` (not_logged_in) | "아직 로그인 안 했어요. '로그인해줘' 라고 말씀해주세요." |

   **Note**: `profile: null` 또는 `endpoint: null` 은 default 사용 중인 정상 상태. ✓ 로 표시하고 default 값을 괄호로 부연 설명. AXHUB_PROFILE 또는 AXHUB_ENDPOINT 설정은 회사 IT 정책에 따라 선택사항.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — single CLI-missing pick → `나중에`, multi-failure pick → `later` (subprocess 에서 자동 fix 안 해요, 진단만 보여줘요).

5.5. **Single failure: cli_present:false 즉시 AskUserQuestion.** CLI 만 부재이고 다른 row 가 모두 ✓ 일 때, 사용자가 "CLI 설치해줘" 라고 다시 발화 안 해도 되도록 즉시 AskUserQuestion 으로 설치 의향 확인:

   ```json
   {
     "question": "axhub CLI 가 설치되어 있지 않아요. 지금 설치할까요?",
     "header": "CLI 설치",
     "multiSelect": false,
     "options": [
       {"label": "1. 자동 설치 (Recommended)", "value": "install", "description": "skills/install-cli 즉시 호출 — OS 별 공식 채널로 설치"},
       {"label": "2. 명령어만 보고 직접", "value": "manual", "description": "설치 명령어 출력 후 종료 — 사용자가 직접 실행"},
       {"label": "3. 나중에", "value": "later", "description": "지금은 그대로 두고 진단만 끝내기"}
     ]
   }
   ```

   "1. 자동 설치" 선택 시 → `Skill("axhub:install-cli")` 즉시 호출. doctor SKILL 의 `NEVER auto-fix` 규칙은 보존 — direct install 안 하고 sibling skill 로 consent route 만 함. multi-failure 가 아닌 단일 cli-missing 시나리오에서만 fire (다른 row 도 fail 이면 Step 5 (multi-failure summary) 가 우선).

5. **Multi-failure summary.** If multiple rows fail, list all of them and surface AskUserQuestion to pick the first one to fix:

   ```json
   {
     "question": "여러 항목 점검 필요해요. 어디부터 고쳐요?",
     "header": "고칠 항목",
     "options": [
       {"label": "1. CLI 설치", "value": "install", "description": "skills/install-cli 호출"},
       {"label": "2. CLI 업그레이드", "value": "upgrade", "description": "skills/update 호출"},
       {"label": "3. 로그인 다시", "value": "login", "description": "skills/auth 호출"},
       {"label": "전부 나중에", "value": "later", "description": "지금은 그대로"}
     ]
   }
   ```

6. **Report exit code** in the summary block: green (all 0), yellow (warnings only), red (preflight returned 64 or 65). The skill itself always returns to the user — never `exit 1` from the doctor flow.

## v0.2.0 command coverage polish

### doctor audit

After the normal readiness summary, offer the agent observability check when the user mentions audit, agent logs, or observability.

```bash
axhub doctor audit --json
```

Render these rows when present:

- `migration_applied`
- `endpoint_reachable`
- `role`
- `export_permission`

Keep this read-only. If audit export requires extra permission, explain the missing role and point to the admin owner instead of attempting a fix.

## NEVER

- NEVER attempt auto-fix from doctor — only report + suggest the next natural-language phrase. The user routes to the relevant sibling skill.
- NEVER echo the raw token contents even if `~/.config/axhub-plugin/token` is readable.
- NEVER skip preflight — that is the single source of truth for version + auth state.
- NEVER mark the system "정상" when any required field is null.

## Additional Resources

For Korean trigger lexicon (doctor intent): `../deploy/references/nl-lexicon.md` (section 8).
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For version-skew flows (too old / too new): `../deploy/references/recovery-flows.md` ("version-skew").
