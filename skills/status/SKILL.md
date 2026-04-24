---
name: status
description: 이 스킬은 사용자가 배포 진행 상황 또는 상태를 묻거나 추적하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "배포 상태", "어떻게 됐어", "됐어", "다 됐어", "끝났어", "지금 어디까지", "어디쯤이야", "진행 중이야", "방금 거 됐어", "올라갔어", "떴어", "라이브 됐어", "반영 됐어", "빌드 됐어", "상태 봐", "진행 상황 알려주세요", "배포 끝났나요", "어디까지 됐나요", "status", "watch", "follow", "progress", "is it done", "deploy state", "build status", 또는 진행 중 axhub 배포를 추적하는 모든 요청. NDJSON tick 스트림을 한국어 진행 안내로 humanize 하고 terminal exit code 를 empathy catalog 로 라우팅합니다.
---

# Deploy Status (watch + narrate)

Track an axhub deploy without dumping raw JSON ticks. Use the adapter `axhub-helpers` for deixis-resolved deployment lookup and stream the watch output through the humanized narration table.

## Workflow

To check status:

1. **Resolve the deployment.** Call the helper to look up the deixis-resolved deployment id from the local cache, or fall back to user prompt:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent status --user-utterance "$ARGS" --json
   ```

   On `cache_hit: false`, follow `../deploy/references/recovery-flows.md` ("cold-cache"): ask the user which app first (use `axhub apps list --json` for choices), then surface the last 3 deploys via the helper:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers list-deployments --app <APP_ID> --limit 3
   ```

   On exit 65 (token missing — Phase 7 US-701 이후엔 SessionStart hook 가 자동 token-init 하므로 거의 발생 안 함):
   > "토큰을 찾을 수 없어요. 'axhub auth login' 으로 한 번 로그인하시거나 CC 세션을 재시작해주세요."

   사용자가 deployment 선택 후 mapping 을 `~/.config/axhub/deployments.json` 에 저장.

   Note: ax-hub-cli v0.1.x has no `axhub deploy list` — the helper hits `GET /api/v1/apps/{id}/deployments` directly with the user's token (env `AXHUB_TOKEN` or `~/.config/axhub-plugin/token`).

2. **Pre-flight version check** (only if mutation chain is implied — pure read can skip):

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

3. **Start watching.** Stream NDJSON with `--watch`:

   ```bash
   axhub deploy status dep_<DEPLOY_ID> --app <APP_ID> --watch --json
   ```

4. **Render Korean narration.** Apply the throttle + phase table from `../deploy/references/recovery-flows.md` ("watch-narration"): one line per ~25s, terminal-state lines are unthrottled. Examples:

   - 0s + `queued` → "배포 요청 받았어요. 잠시 후 빌드 시작합니다 (정상)"
   - ~30s + `building` → "30초 경과, 빌드 시작했어요 (정상)"
   - ~1m + `building` → "1분 경과, 빌드 중이에요 (정상). 보통 2~3분 정도 걸려요"
   - ~2m + `pushing_image` → "2분 경과, 이미지 푸시 중이에요 (정상). 거의 다 왔어요"
   - ~3m + `health_check` → "헬스체크 중. 마지막 단계예요"
   - terminal `succeeded` → trigger exit 0 success template
   - terminal `failed` → trigger exit 1/65/66/67/68 template per emitted error

5. **Silent stream guard.** If 60s pass with no NDJSON event, emit "조용하네요. 서버 응답 기다리는 중입니다 (정상). 30초 후 다시 알려드릴게요."

6. **User interrupt.** If the user says "그만 봐", "그만", "충분해", "stop watching", terminate the watch process and report the last observed phase. The deploy continues server-side regardless.

7. **On any non-zero exit**, route to `../deploy/references/error-empathy-catalog.md` by exit code:
   - exit 65 → token expired template + AskUserQuestion to run auth login
   - exit 67 → resource not found + did-you-mean from `axhub deploy list`
   - exit 68 → rate limit + Retry-After backoff
   - exit 1 → transport error; retry the watch once for read paths

## NEVER

- NEVER drop `--json` from `axhub deploy status` (parsing depends on it).
- NEVER echo the raw NDJSON tick stream verbatim — vibe coders cannot read it.
- NEVER auto-trigger `axhub deploy create` from the status path (read-only intent).
- NEVER invent a `deployment_id` when the cache is cold; ask via AskUserQuestion.
- NEVER throttle the terminal-state narration — success/failure must surface immediately.

## Additional Resources

For Korean trigger lexicon (반말 / 존댓말 / demo / 경어 status variants): `../deploy/references/nl-lexicon.md` (intent: status).
For 4-part Korean exit-code templates: `../deploy/references/error-empathy-catalog.md`.
For multi-machine cold cache + watch-narration phase table: `../deploy/references/recovery-flows.md`.
