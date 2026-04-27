---
name: recover
description: 이 스킬은 사용자가 직전 배포를 되돌리거나 이전 버전으로 복원하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "방금 거 되돌려", "되돌려", "롤백해", "롤백", "이전 버전으로", "직전 버전으로 돌려줘", "방금 배포 취소해", "망했어 되돌려", "어제 거로 돌려줘", "잘 되던 버전으로 돌려", "마지막 정상 빌드로", "복구해", "안정 버전으로", "롤백 부탁드립니다", "직전 안정 버전으로 복구 부탁", "마지막 정상 빌드로 돌려주세요", "이전 버전으로 되돌려주세요", "rollback", "roll back", "revert", "undo", "undo deploy", "restore previous", "redeploy previous", "hot fix", "hotfix", "forward fix", 또는 이전 상태 복원 의도. 참고: v0.1.0 CLI 가 실제 rollback 미지원이므로 직전 안정 commit 재배포 (forward-fix) 방식으로 구현합니다.
---

# Recover (forward-fix-as-rollback)

Restore the previous known-good deploy by **redeploying the prior commit**, not by reversing the current one. The axhub v0.1.0 CLI has no `axhub deploy rollback` command — this skill is transparent about that.

> "이건 진짜 rollback이 아니라 forward-fix예요 — 직전 안정 커밋을 다시 배포하는 방식입니다. 결과는 같지만 새 배포가 한 건 더 생겨요."

## Workflow

To recover:

1. **Read last-known-good from local cache.** The deployments cache holds `(deployment_id → app_id → commit_sha → status)` per machine:

   ```bash
   cat ~/.config/axhub/deployments.json
   ```

   Find the most recent entry where `status == "succeeded"` for the current app. If the cache is cold, fall back to:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers list-deployments --app <APP_ID> --limit 10
   ```

   ax-hub-cli has no `axhub deploy list` — helper hits REST API directly. On exit 65 (token missing — Phase 7 US-701 이후엔 SessionStart 자동 setup):
   > "토큰을 찾을 수 없어요. 'axhub auth login' 또는 CC 세션 재시작."

   Filter to `status == "succeeded"` and pick the second-most-recent (the most recent succeeded ≠ current state because the user wants to back out).

2. **Resolve the prior commit context.** For the chosen deploy, surface:
   - `commit_sha` (short)
   - `commit_message` (first line)
   - `created_at` (humanized: "어제 14:30", "30분 전")
   - `deployment_id` of the original deploy

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — rollback 확인 → `abort` (destructive 작업이라 subprocess 자동 진행 안 해요), pick_other → `abort`.

3. **Render transparent confirmation card.** AskUserQuestion:

   ```
   직전 안정 배포로 되돌릴게요 (forward-fix 방식):
     · 앱:    <APP_SLUG> (id=<APP_ID>)
     · 커밋:  <PREV_SHA> — "<COMMIT_MSG>" (<RELATIVE_TIME>)
     · 원래 배포: <DEPLOY_ID>

   ⚠️ 진짜 rollback이 아니에요. 이전 커밋을 새로 배포하는 거예요.
      이전 배포는 그대로 기록에 남고, 새 배포 한 건이 추가됩니다.

   진행할까요?
   ```

   Options:

   ```json
   {
     "question": "직전 안정 커밋으로 다시 배포할까요?",
     "options": [
       {"label": "네, 직전 커밋으로", "value": "confirm", "description": "<PREV_SHA>를 새로 배포"},
       {"label": "다른 커밋 고르기", "value": "pick_other", "description": "최근 succeeded 배포 목록에서 선택"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

4. **On `confirm`.** Mint consent token and run deploy create with the prior commit:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint --tool-call-id "$NEXT_BASH_TOOL_CALL_ID" --action deploy_create \
     --app-id "$APP_ID" --profile "$PROFILE" --branch "$BRANCH" --commit "$PREV_SHA"

   axhub deploy create --app "$APP_ID" --branch "$BRANCH" --commit "$PREV_SHA" --json
   ```

   The PreToolUse hook verifies the consent token before allowing the bash call.

5. **Auto-watch.** Capture the new `dep_<id>` and route to `skills/status` for narrated tracking. Do not block on completion — the user wanted the recovery action; status is a courtesy follow-up.

6. **On `pick_other`.** Surface the last 5 succeeded deploys from `axhub deploy list` via AskUserQuestion. Repeat from step 3 with the chosen commit.

7. **On non-zero exit from create**, route to `../deploy/references/error-empathy-catalog.md`. The `validation.deployment_in_progress` case is especially relevant here (user might recover during another deploy) — follow `../deploy/references/recovery-flows.md` ("deployment_in_progress") and offer to watch the in-flight deploy first.

8. **No prior succeeded deploy found.** Surface: "되돌릴 수 있는 직전 안정 배포를 못 찾았어요. 이 앱의 첫 배포이거나, 모든 이전 배포가 실패한 상태일 수 있어요. 'logs'로 현재 배포 원인 먼저 보시겠어요?"

## NEVER

- NEVER claim this is a real rollback — always say "forward-fix" / "직전 커밋 재배포" in the confirmation card.
- NEVER skip the consent token mint (PreToolUse hook will deny).
- NEVER skip the AskUserQuestion confirmation — destructive op needs explicit yes.
- NEVER auto-pick the most-recent succeeded deploy without showing the candidate to the user (commit_sha + commit_message in the card).
- NEVER drop `--json` from `axhub deploy create`.

## Additional Resources

For Korean trigger lexicon (recover/rollback intent): `../deploy/references/nl-lexicon.md` (section 10).
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For deployment_in_progress flow during recovery: `../deploy/references/recovery-flows.md` ("deployment_in_progress").
