---
name: status
description: This skill should be used when the user asks about deploy progress or status. Activates on "배포 상태", "어떻게 됐어", "됐어", "다 됐어", "끝났어", "지금 어디까지", "어디쯤이야", "진행 중이야", "방금 거 됐어", "올라갔어", "떴어", "라이브 됐어", "반영 됐어", "빌드 됐어", "상태 봐", "진행 상황 알려주세요", "배포 끝났나요", "어디까지 됐나요", "status", "watch", "follow", "progress", "is it done", "deploy state", "build status", or any request to track an in-flight axhub deploy. Streams humanized Korean progress narration off the NDJSON tick stream and routes terminal exit codes through the empathy catalog.
---

# Deploy Status (watch + narrate)

Track an axhub deploy without dumping raw JSON ticks. Use the adapter `axhub-helpers` for deixis-resolved deployment lookup and stream the watch output through the humanized narration table.

## Workflow

To check status:

1. **Resolve the deployment.** Call the helper to look up the deixis-resolved deployment id from the local cache, or fall back to user prompt:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent status --user-utterance "$ARGS" --json
   ```

   On `cache_hit: false`, follow `references/recovery-flows.md` ("cold-cache"): ask the user which app and surface the last 3 deploys via `axhub deploy list --app <APP_SLUG> --json --limit 3` for AskUserQuestion disambiguation. Persist the chosen mapping to `~/.config/axhub/deployments.json` for next time.

2. **Pre-flight version check** (only if mutation chain is implied — pure read can skip):

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json
   ```

3. **Start watching.** Stream NDJSON with `--watch`:

   ```bash
   axhub deploy status dep_<DEPLOY_ID> --app <APP_ID> --watch --json
   ```

4. **Render Korean narration.** Apply the throttle + phase table from `references/recovery-flows.md` ("watch-narration"): one line per ~25s, terminal-state lines are unthrottled. Examples:

   - 0s + `queued` → "배포 요청 받았어요. 잠시 후 빌드 시작합니다 (정상)"
   - ~30s + `building` → "30초 경과, 빌드 시작했어요 (정상)"
   - ~1m + `building` → "1분 경과, 빌드 중이에요 (정상). 보통 2~3분 정도 걸려요"
   - ~2m + `pushing_image` → "2분 경과, 이미지 푸시 중이에요 (정상). 거의 다 왔어요"
   - ~3m + `health_check` → "헬스체크 중. 마지막 단계예요"
   - terminal `succeeded` → trigger exit 0 success template
   - terminal `failed` → trigger exit 1/65/66/67/68 template per emitted error

5. **Silent stream guard.** If 60s pass with no NDJSON event, emit "조용하네요. 서버 응답 기다리는 중입니다 (정상). 30초 후 다시 알려드릴게요."

6. **User interrupt.** If the user says "그만 봐", "그만", "충분해", "stop watching", terminate the watch process and report the last observed phase. The deploy continues server-side regardless.

7. **On any non-zero exit**, route to `references/error-empathy-catalog.md` by exit code:
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
