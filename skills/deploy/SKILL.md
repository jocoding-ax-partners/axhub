---
name: deploy
description: This skill should be used when the user asks to "deploy", "ship", "release", "rollout", "launch", "배포해", "배포해줘", "올려", "올리자", "쏘자", "내보내자", "푸시한 거 띄워", "프로덕션에 박아", "터트려", "공개해", "demo가 필요해", or any request to push the current branch live to axhub. Triggers axhub deploy create with safety primitives: live profile/app resolution, HMAC consent gate via AskUserQuestion preview card, exit-code recovery routing.
---

# Deploy via axhub

Deploy a vibe coder's app to axhub with safety primitives. Use the adapter `axhub-helpers` (auto on PATH while plugin is enabled) for live resolution and consent management. Do not call `axhub deploy create` directly without going through the helper flow.

## Workflow

To deploy:

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
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers consent-mint --tool-call-id "$NEXT_BASH_TOOL_CALL_ID" --action deploy_create \
     --app-id "$APP_ID" --profile "$PROFILE" --branch "$BRANCH" --commit "$COMMIT_SHA"

   axhub deploy create --app "$APP_ID" --branch "$BRANCH" --commit "$COMMIT_SHA" --json
   ```

   The PreToolUse hook verifies the consent token before the bash call; if absent or non-matching, the command is blocked.

5. **Post-deploy chain** — capture `.id` from the deploy create JSON, then auto-follow:

   ```bash
   axhub deploy status dep_$DEPLOY_ID --watch --json
   ```

   Render humanized Korean progress every ~30s ("1분 경과, 빌드 중이에요 (정상)") per `references/recovery-flows.md` ("watch-narration").

6. **On any non-zero exit**, route to `references/error-empathy-catalog.md` by exit code:
   - exit 64 + `validation.deployment_in_progress` → 4-part Korean copy: "다른 배포가 진행 중이에요. 당신 앱은 안전합니다. 5분만 기다리면 자동으로 다음 배포가 가능해요." Never retry. Offer to watch the in-flight deploy instead.
   - exit 65 → token expired template + AskUserQuestion to run auth login
   - exit 67 → resource not found + did-you-mean suggestion from apps list
   - exit 68 → rate limit + Retry-After backoff
   - exit 1 → transport error; retry at most once for read paths, never for create

7. **Dry-run NL trigger** — if the user said "한번 해보기만", "리허설", "테스트로", "진짜 안 올리고", add `--dry-run` to step 4 and skip step 5.

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
For working transcripts: `examples/golden-deploy-transcript.md`, `examples/concurrent-deploy-rejection.md`.
For privacy filter on apis list: `../apis/references/privacy-filter.md` (used by sibling apis skill).
