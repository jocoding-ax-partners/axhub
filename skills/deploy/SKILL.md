---
name: deploy
description: 이 스킬은 사용자가 현재 브랜치를 axhub 라이브로 배포하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "deploy", "ship", "release", "rollout", "launch", "배포해", "배포해줘", "올려", "올리자", "쏘자", "내보내자", "푸시한 거 띄워", "프로덕션에 박아", "터트려", "공개해", "demo가 필요해", 또는 현재 브랜치를 axhub 라이브로 push 하고 싶다는 모든 의도. axhub deploy create 를 안전 가드와 함께 트리거: 라이브 profile/app 해석, AskUserQuestion preview card 를 통한 HMAC consent gate, exit-code 기반 복구 라우팅.
multi-step: true
needs-preflight: true
---

# Deploy via axhub

Deploy a vibe coder's app to axhub with safety primitives. Use the adapter `axhub-helpers` (auto on PATH while plugin is enabled) for live resolution and consent management. Do not call `axhub deploy create` directly without going through the helper flow.

## Workflow

**Pre-execute preflight context (Phase 17 US-1706 — `!command` injection)**:

```
!`${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json`
```

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요. 출력 (auth_status, current_app, current_env, last_deploy_id, last_deploy_status, plugin_version) 이 모델 컨텍스트에 자동 주입돼서 Step 1 의 별도 bash 호출이 줄어요. PreToolUse Bash hook 은 preprocessing 단계에서 trigger 안 해요 (Claude Code SKILL primitive 동작).

To deploy:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start so the user can glance and see how far we've come:

   ```typescript
   TodoWrite({ todos: [
     { content: "토큰 확인 (preflight)",         status: "in_progress", activeForm: "토큰 확인하는 중" },
     { content: "앱 / 환경 / 브랜치 확정",         status: "pending",     activeForm: "앱 정보 정리하는 중" },
     { content: "미리보기 카드 보여드리기",         status: "pending",     activeForm: "미리보기 준비하는 중" },
     { content: "동의 받고 배포 시작",            status: "pending",     activeForm: "배포 시작하는 중" },
     { content: "빌드 모니터 (~3분)",             status: "pending",     activeForm: "빌드 진행 보는 중" },
     { content: "결과 안내",                     status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Live resolve** — call the helper to fetch authoritative `{profile, endpoint, app_id, app_slug, branch, commit_sha, commit_message, eta_sec}`:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent deploy --user-utterance "$ARGS" --json
   ```

   Never use cached `app_id` for mutation. If resolve returns ambiguity, ask the user to disambiguate (slug list with numeric IDs).

2. **Pre-flight version check**:

   ```bash
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

4. **On user approval**, mint a consent token and run deploy:

   ```bash
   cat <<JSON | ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint
   {"tool_call_id":"${CLAUDE_SESSION_ID}:${NEXT_BASH_TOOL_CALL_ID}","action":"deploy_create","app_id":"${APP_ID}","profile":"${PROFILE}","branch":"${BRANCH}","commit_sha":"${COMMIT_SHA}"}
   JSON

   axhub deploy create --app "$APP_ID" --branch "$BRANCH" --commit "$COMMIT_SHA" --json
   ```

   The PreToolUse hook verifies the consent token before the bash call; if absent or non-matching, the command is blocked.

5. **Post-deploy chain** — capture `.id` from the deploy create JSON, then auto-follow:

   ```bash
   axhub deploy status dep_$DEPLOY_ID $WATCH --json
   ```

   **Non-interactive guard:** If running in non-interactive context (`$CI` or `$CLAUDE_NON_INTERACTIVE` env var set, OR no TTY, OR `claude -p` invocation), DROP `--watch` flag and render single snapshot — `--watch` blocks indefinitely in headless/subprocess mode and `/axhub:deploy` post-chain hangs forever (same root cause as v0.1.12 status/logs hotfix). Detection: `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi` then use `$WATCH`.

   Render humanized Korean progress every ~30s ("1분 경과, 빌드 중이에요 (정상)") per `references/recovery-flows.md` ("watch-narration").

6. **On any non-zero exit**, route to `references/error-empathy-catalog.md` by exit code:
   - exit 64 + `validation.deployment_in_progress` → 4-part Korean copy: "다른 배포가 진행 중이에요. 앱은 안전해요. 5분만 기다리면 자동으로 다음 배포가 가능해요." Never retry. Offer to watch the in-flight deploy instead.
   - exit 65 → token expired template + AskUserQuestion to run auth login
   - exit 67 → resource not found + did-you-mean suggestion from apps list
   - exit 68 → rate limit + Retry-After backoff
   - exit 1 → transport error; retry at most once for read paths, never for create

7. **Dry-run NL trigger** — if the user said "한번 해보기만", "리허설", "테스트로", "진짜 안 올리고", add `--dry-run` to step 4 and skip step 5.

8. **Cache last-deploy for statusline (Phase 17 US-1707).** After Step 5 terminal status, write the deploy summary so `bin/statusline.sh` can show it across sessions:

   ```bash
   mkdir -p ~/.cache/axhub-plugin
   cat > ~/.cache/axhub-plugin/last-deploy.json <<JSON
   {"deployment_id":"$DEPLOY_ID","status":"$TERMINAL_STATUS","commit_sha":"$COMMIT_SHA","app_slug":"$APP_SLUG","timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)"}
   JSON
   ```

   Skip on `--dry-run` (statusline 은 실제 deploy 만 추적).

## NEVER

- NEVER retry `axhub deploy create` on exit 64.
- NEVER drop `--json` (parsing relies on it).
- NEVER call `axhub deploy create` without going through `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint` first; the PreToolUse hook will deny.
- NEVER infer `app_id` from `pwd` or git remote alone in the mutation path; always live resolve through the helper.
- NEVER bypass the AskUserQuestion preview card on slash invocation; slash is explicit consent for the SKILL invocation, not for the destructive operation.

## Additional Resources

For Korean trigger lexicon (informal, honorific, demo-context variants): `references/nl-lexicon.md`.
For exit-code → 4-part Korean error template (emotion + cause + action + button): `references/error-empathy-catalog.md`.
For multi-machine cold cache, headless/Codespaces, version skew, watch narration: `references/recovery-flows.md`.
For working transcripts, use captured `.omc/evidence/` pilot logs; no standalone example transcript files ship in this plugin.
For privacy filter on apis list: `../apis/references/privacy-filter.md` (used by sibling apis skill).
