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

1. **Detect helper binary via PATH first** (Phase 5 US-503 — env var `CLAUDE_PLUGIN_ROOT` may not propagate to skill bash subshells):

   ```bash
   command -v axhub-helpers || echo "missing"
   ```

   If output is `missing`, surface: "axhub-helpers 바이너리가 PATH에 없어요. 첫 CC 세션에서 자동 다운로드돼야 정상이에요. 'bash ${CLAUDE_PLUGIN_ROOT}/bin/install.sh' 수동 실행 또는 자동 다운로드 비활성화 (export AXHUB_SKIP_AUTODOWNLOAD=1) 확인해주세요." Skip remaining steps; report ✗ helper missing only.

2. **Run preflight** (CLI version range + auth status combined):

   ```bash
   axhub-helpers preflight --json
   ```

   This returns: `cli_version, in_range, cli_too_old, cli_too_new, cli_present, auth_ok, auth_error_code, scopes, profile, endpoint, user_email, expires_at`.

3. **Fetch raw version + path** for the report:

   ```bash
   axhub --version
   which axhub
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
   | helper missing (step 1 returned `missing`) | "axhub-helpers 바이너리가 PATH에 없어요. 'bash ${CLAUDE_PLUGIN_ROOT}/bin/install.sh' 수동 실행 또는 CC 재시작으로 자동 다운로드 트리거." |
   | `cli_present: false` | "axhub 설치되어 있지 않아요. 'brew install axhub' 또는 회사 IT 안내대로 설치해주세요." |
   | `cli_too_old: true` | "axhub가 너무 오래된 버전이에요 (v<CUR>). 'axhub 업그레이드해줘' 라고 말씀해주세요." |
   | `cli_too_new: true` | "axhub가 플러그인보다 최신이에요. 'axhub 플러그인 업데이트' 라고 말씀해주세요." |
   | `auth_ok: false` (token_expired) | "로그인이 만료됐어요. '다시 로그인해줘' 라고 말씀해주세요." |
   | `auth_ok: false` (not_logged_in) | "아직 로그인 안 했어요. '로그인해줘' 라고 말씀해주세요." |

   **Note**: `profile: null` 또는 `endpoint: null` 은 default 사용 중인 정상 상태. ✓ 로 표시하고 default 값을 괄호로 부연 설명. AXHUB_PROFILE 또는 AXHUB_ENDPOINT 설정은 회사 IT 정책에 따라 선택사항.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — multi-failure pick → `later` (subprocess 에서 자동 fix 안 해요, 진단만 보여줘요).

5. **Multi-failure summary.** If multiple rows fail, list all of them and surface AskUserQuestion to pick the first one to fix:

   ```json
   {
     "question": "여러 항목 점검 필요해요. 어디부터 고쳐요?",
     "header": "고칠 항목",
     "options": [
       {"label": "1. CLI 업그레이드", "value": "upgrade", "description": "skills/update 호출"},
       {"label": "2. 로그인 다시", "value": "login", "description": "skills/auth 호출"},
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
