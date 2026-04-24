---
name: doctor
description: This skill should be used when the user wants to diagnose their axhub install or environment. Activates on "axhub 설치돼 있어", "doctor", "닥터", "진단해", "환경 점검해", "axhub 점검", "헬스체크 해", "잘 깔렸어", "셋업 다 됐어", "설정 봐", "환경 변수 확인해주세요", "설치 상태 알려주세요", "진단 부탁드려요", "환경 점검해주세요", "시스템 상태 확인해주세요", "셋업이 다 끝났나요", "doctor", "check", "diagnose", "health check", "sanity check", "setup check", "env check", or any axhub diagnostic request. Reports CLI version, auth state, profile, endpoint, and scopes with fix suggestions per failure.
---

# Doctor (env + install diagnostic)

Run a full axhub plugin health check. Report what's working, what's not, and the next natural-language phrase the user can say to fix each gap.

## Workflow

To run diagnostics:

1. **Run preflight** (CLI version range + auth status combined):

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

   This returns: `cli_version, in_range, cli_too_old, cli_too_new, cli_present, auth_ok, auth_error_code, scopes, profile, endpoint, user_email, expires_at`.

2. **Fetch raw version + path** for the report:

   ```bash
   axhub --version
   which axhub
   ```

3. **Render the diagnostic card in Korean.** Use checkmarks (✓ / ✗ / ⚠) per row:

   ```
   axhub 진단 결과:
     ✓ CLI 설치:     v<CLI_VERSION> (<WHICH_PATH>)
     ✓ 버전 범위:    호환 (필요: v<MIN> ~ v<MAX> 미만)
     ✓ 로그인:       <USER_EMAIL>
     ✓ 만료:         <EXPIRES_AT> (남은 시간: <DELTA>)
     ✓ 권한 (scope): <SCOPES joined>
     ✓ 환경 (profile): <PROFILE> (<ENDPOINT>)

   모두 정상이에요. 배포하실 준비 완료!
   ```

4. **On any failure row**, replace ✓ with ✗ and append a one-line fix suggestion as a literal next phrase:

   | Failure | Suggested phrase |
   |---|---|
   | `cli_present: false` | "axhub 설치되어 있지 않아요. 'brew install axhub' 또는 회사 IT 안내대로 설치해주세요." |
   | `cli_too_old: true` | "axhub가 너무 오래된 버전이에요 (v<CUR>). 'axhub 업그레이드해줘' 라고 말씀해주세요." |
   | `cli_too_new: true` | "axhub가 플러그인보다 최신이에요. 'axhub 플러그인 업데이트' 라고 말씀해주세요." |
   | `auth_ok: false` (token_expired) | "로그인이 만료됐어요. '다시 로그인해줘' 라고 말씀해주세요." |
   | `auth_ok: false` (not_logged_in) | "아직 로그인 안 했어요. '로그인해줘' 라고 말씀해주세요." |
   | `profile: null` | "환경 (profile)이 설정 안 됐어요. AXHUB_PROFILE을 회사 IT가 안내한 값으로 설정해주세요." |
   | `endpoint: null` | "endpoint가 설정 안 됐어요. AXHUB_ENDPOINT 또는 ~/.config/axhub/config.yaml 확인해주세요." |

5. **Multi-failure summary.** If multiple rows fail, list all of them and surface AskUserQuestion to pick the first one to fix:

   ```json
   {
     "question": "여러 항목 점검 필요해요. 어디부터 고칠까요?",
     "options": [
       {"label": "1. CLI 업그레이드", "value": "upgrade", "description": "skills/update 호출"},
       {"label": "2. 로그인 다시", "value": "login", "description": "skills/auth 호출"},
       {"label": "전부 나중에", "value": "later", "description": "지금은 그대로"}
     ]
   }
   ```

6. **Report exit code** in the summary block: green (all 0), yellow (warnings only), red (preflight returned 64 or 65). The skill itself always returns to the user — never `exit 1` from the doctor flow.

## NEVER

- NEVER attempt auto-fix from doctor — only report + suggest the next natural-language phrase. The user routes to the relevant sibling skill.
- NEVER echo the raw token contents even if `~/.config/axhub/token` is readable.
- NEVER skip preflight — that is the single source of truth for version + auth state.
- NEVER mark the system "정상" when any required field is null.

## Additional Resources

For Korean trigger lexicon (doctor intent): `../deploy/references/nl-lexicon.md` (section 8).
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For version-skew flows (too old / too new): `../deploy/references/recovery-flows.md` ("version-skew").
