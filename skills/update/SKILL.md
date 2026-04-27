---
name: update
description: 이 스킬은 사용자가 axhub CLI 업데이트를 확인하거나 적용하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "axhub 새 버전 있어", "업데이트 있어", "새 버전 나왔어", "최신이야", "버전 확인", "업데이트 확인해", "업데이트해", "업그레이드해", "최신으로 올려", "새 버전 받아", "axhub 업그레이드 부탁드려요", "업데이트 적용해주세요", "최신 버전으로 올려주세요", "CLI 업데이트 부탁드려요", "brew upgrade 해줘", "update", "upgrade", "version", "new release", "check version", "update available", "latest", 또는 axhub CLI 버전 관리 요청. PLAN §16.10 에 따라 cosign 서명 검증을 기본 enforce 합니다.
multi-step: true
needs-preflight: false
---

# Update (CLI version check + apply)

Check and apply axhub CLI updates with cosign signature verification mandatory by default (PLAN §16.10 / row 59 / row 47).

## Workflow

To handle updates:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start:

   ```typescript
   TodoWrite({ todos: [
     { content: "현재 / 최신 버전 비교",        status: "in_progress", activeForm: "버전 확인하는 중" },
     { content: "릴리즈 노트 요약",             status: "pending",     activeForm: "변경사항 정리하는 중" },
     { content: "동의 받고 cosign 검증",         status: "pending",     activeForm: "서명 검증하는 중" },
     { content: "binary 교체",                  status: "pending",     activeForm: "교체 진행하는 중" },
     { content: "결과 안내",                    status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Check for update.** Run `axhub update check --json` directly — do NOT force `AXHUB_DISABLE_AUTOUPDATE=1` (Phase 5 US-505: 회사 정책으로 disable 한 환경만 자연스럽게 disable 처리되도록 둠):

   ```bash
   axhub update check --json
   ```

   Parse the response: `{"current": "0.1.0", "latest": "0.1.1", "has_update": true}` or `{"has_update": false}`. Exit 2 means the user's environment has `AXHUB_DISABLE_AUTOUPDATE=1` set (회사 보안 정책). Surface the Korean message: "회사 환경에서 autoupdate 정책이 disable 되어 있어요. 강제 진행하려면 IT/보안팀에 먼저 확인해주세요." and stop. Exit 0 with `has_update:false` means already latest.

2. **On `has_update: false`.** Tell the user "이미 최신 버전이에요 (v<CURRENT>). 업데이트 안 받아도 돼요." and stop.

3. **On `has_update: true`.** Render Korean upgrade card:

   ```
   새 버전이 나왔어요.
     · 현재 버전: v<CURRENT>
     · 새 버전:   v<LATEST>
     · 변경사항:  <RELEASE_NOTES_FIRST_LINE>

   업데이트하시겠어요?
   ```

   **Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — apply 확인 → `skip` (binary 교체는 subprocess 에서 자동 안 해요).

   AskUserQuestion:

   ```json
   {
     "question": "axhub CLI 업그레이드해요?",
     "header": "업그레이드",
     "options": [
       {"label": "네, 업그레이드", "value": "apply", "description": "cosign 서명 검증 후 업데이트"},
       {"label": "릴리즈 노트 보기", "value": "notes", "description": "변경사항 자세히 보기"},
       {"label": "지금은 그대로", "value": "skip", "description": "현재 버전 유지"}
     ]
   }
   ```

4. **On `apply`.** Run apply with cosign required (default-on per Phase 6 §16.10 / row 59):

   ```bash
   AXHUB_REQUIRE_COSIGN=1 axhub update apply --yes --json
   ```

   The CLI verifies the binary signature before swapping. On exit 0, render: "업그레이드 완료! v<LATEST> 깔렸어요. 새 터미널을 열거나 다시 실행하시면 됩니다."

5. **On exit 66 + `update.cosign_verification_failed`.** HARD STOP. Route to `../deploy/references/error-empathy-catalog.md` ("exit 66 + update.cosign_verification_failed"). Tell user:

   > "보안 검증 실패. 절대 강제 진행하지 마세요. 회사 IT 보안팀에 즉시 알려주세요. 그동안 axhub는 현재 버전으로 계속 사용 가능해요."

   Do NOT offer `AXHUB_ALLOW_UNSIGNED=1` override (only IT/admins should ever set that).

6. **On Homebrew/Scoop installs.** If the CLI detects it was installed via package manager, it returns exit 1 with `package_manager: "brew"` and a hint. Surface to user:

   > "axhub가 brew로 설치돼 있어서 직접 업데이트는 못 해요. 'brew upgrade axhub' 라고 터미널에 입력하시면 됩니다. 도와드릴까요?"

7. **On non-zero exit (other)**, route to `error-empathy-catalog.md` by code (65 / 68 / 1).

## NEVER

- NEVER drop `AXHUB_REQUIRE_COSIGN=1` from the apply call (Phase 6 §16.10 default-on supply chain protection).
- NEVER offer to set `AXHUB_ALLOW_UNSIGNED=1` (IT-only override per PLAN row 59).
- NEVER continue past `update.cosign_verification_failed` under any circumstance.
- NEVER call `axhub update apply` without explicit AskUserQuestion confirmation.
- NEVER auto-retry `update apply` on transport failure (binary may be partially swapped).

## Additional Resources

For Korean trigger lexicon (update intent): `../deploy/references/nl-lexicon.md` (sections 7a/7b).
For exit 66 + cosign template: `../deploy/references/error-empathy-catalog.md`.
For PLAN reference: §16.10 (cosign default-on), row 59 (default `AXHUB_REQUIRE_COSIGN=1`), row 47 (cosign-signed manifest).
