---
name: status
description: '이 스킬은 사용자가 배포 진행 상황 또는 상태를 묻거나 추적하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "끝났", "끝났어", "다 됐", "다 됐어", "됐어", "떴어", "라이브 됐", "라이브 됐어", "반영 됐", "반영 됐어", "방금 거 됐어", "배포 끝났나요", "배포 상태", "빌드 됐", "빌드 됐어", "상태 봐", "어디까지", "어디까지 됐나요", "어디쯤", "어디쯤이야", "어떻게 됐", "어떻게 됐어", "올라갔", "올라갔어", "지금 어디까지", "진행 상황", "진행 상황 알려주세요", "진행 중", "진행 중이야", "build status", "deploy state", "follow", "is it done", "progress", "status", "watch", 또는 진행 중 axhub 배포를 추적하는 모든 요청. NDJSON tick 스트림을 한국어 진행 안내로 humanize 하고 terminal exit code 를 empathy catalog 로 라우팅합니다.'
examples:
  - utterance: "지금 진행 중인 배포 어떻게 됐어"
    intent: "check axhub deployment status"
  - utterance: "방금 배포한 거 status"
    intent: "check axhub deployment status"
  - utterance: "status"
    intent: "check axhub deployment status"
  - utterance: "is it done"
    intent: "check axhub deployment status"
  - utterance: "paydrop status 봐줘"
    intent: "check axhub deployment status"
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Deploy Status (watch + narrate)

Track an axhub deploy without dumping raw JSON ticks. Use the adapter `axhub-helpers` for deixis-resolved deployment lookup and stream the watch output through the humanized narration table.

## Workflow

To check status:

1. **최근 배포 목록 조회.** 앱을 확인하고 배포 목록을 가져와요:

   ```bash
   APP=$(${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent status --user-utterance "$ARGS" --json | jq -r '.resolve.app_id // empty')
   DEPLOY_LIST_JSON=$(axhub deploy list --app "$APP" --json)
   ```

   배포 이력이 없으면 안내하고 종료해요:

   ```bash
   if [ "$(echo "$DEPLOY_LIST_JSON" | jq '(.items // .) | length')" -eq 0 ]; then
     echo '{"systemMessage":"배포 이력이 없어요. 먼저 /axhub:deploy 로 배포 후 다시 호출하세요."}'
     exit 0
   fi
   ```

   배포가 있으면 AskUserQuestion 으로 선택해요. 비대화형 환경에서는 `cold_cache_default: most_recent` (registry) 에 따라 `items[0]` 를 자동 선택해요:

   ```json
   {
     "question": "어떤 배포 상태를 볼까요?",
     "header": "배포 선택",
     "options": "<list[0..N] — id + app_slug + branch + created_at 으로 구성한 옵션>"
   }
   ```

   선택한 항목의 `id` 를 `$DEPLOYMENT_ID` 에 저장해요.

2. **Pre-flight version check** (only if mutation chain is implied — pure read can skip):

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

3. **상태 확인.** NDJSON 스트림을 `--watch` 로 받아요:

   ```bash
   axhub deploy status "${DEPLOYMENT_ID}" --app "${APP}" --watch --json
   ```

   **Non-interactive guard:** If running in non-interactive context (`$CI` or `$CLAUDE_NON_INTERACTIVE` env var set, OR no TTY, OR `claude -p` invocation), DROP `--watch` flag and render single snapshot instead — `--watch` blocks indefinitely in headless/subprocess mode and `/axhub:status` hangs forever. Detection: prefix command with `if [ -t 1 ] && [ -z "$CI" ] && [ -z "$CLAUDE_NON_INTERACTIVE" ]; then WATCH=--watch; else WATCH=; fi` then use `$WATCH`.

4. **Render Korean narration.** Apply the throttle + phase table from `../deploy/references/recovery-flows.md` ("watch-narration"): one line per ~25s, terminal-state lines are unthrottled. Examples:

   - 0s + `queued` → "배포 요청 받았어요. 잠시 후 빌드 시작해요 (정상)"
   - ~30s + `building` → "30초 경과, 빌드 시작했어요 (정상)"
   - ~1m + `building` → "1분 경과, 빌드 중이에요 (정상). 보통 2~3분 정도 걸려요"
   - ~2m + `pushing_image` → "2분 경과, 이미지 푸시 중이에요 (정상). 거의 다 왔어요"
   - ~3m + `health_check` → "헬스체크 중. 마지막 단계예요"
   - terminal `succeeded` → trigger exit 0 success template
   - terminal `failed` → trigger exit 1/65/66/67/68 template per emitted error

5. **Silent stream guard.** If 60s pass with no NDJSON event, emit "조용하네요. 서버 응답 기다리는 중이에요 (정상). 30초 후 다시 알려줄게요."

6. **User interrupt.** If the user says "그만 봐", "그만", "충분해", "stop watching", terminate the watch process and report the last observed phase. The deploy continues server-side regardless.

7. **On any non-zero exit**, route to `../deploy/references/error-empathy-catalog.md` by exit code:
   - exit 65 → token expired template + AskUserQuestion to run auth login
   - exit 67 → resource not found + did-you-mean from `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers list-deployments`
   - exit 68 → rate limit + Retry-After backoff
   - exit 1 → transport error; retry the watch once for read paths

## NEVER

- NEVER drop `--json` from `axhub deploy status` (parsing depends on it).
- NEVER echo the raw NDJSON tick stream verbatim — vibe coders cannot read it.
- NEVER auto-trigger `axhub deploy create` from the status path (read-only intent).
- NEVER invent a `deployment_id` when the cache is cold; ask via AskUserQuestion.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — cold cache deploy pick → `most_recent` (가장 최근 succeeded), exit-65 re-login → `abort` (subprocess 자동 로그인 안 해요).
- NEVER throttle the terminal-state narration — success/failure must surface immediately.

## Additional Resources

For Korean trigger lexicon (반말 / 존댓말 / demo / 경어 status variants): `../deploy/references/nl-lexicon.md` (intent: status).
For 4-part Korean exit-code templates: `../deploy/references/error-empathy-catalog.md`.
For multi-machine cold cache + watch-narration phase table: `../deploy/references/recovery-flows.md`.
