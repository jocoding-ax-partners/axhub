---
name: logs
description: This skill should be used when the user asks for build or runtime logs of an axhub deploy. Activates on "로그 봐", "로그 보여줘", "로그 까봐", "빌드 로그 봐", "런타임 로그 봐", "왜 실패했어", "왜 안돼", "왜 깨졌어", "왜 죽었어", "에러 봐", "에러 메시지 봐", "콘솔 봐", "출력 보여줘", "방금 거 로그", "로그 보여주세요", "빌드 로그 확인해주세요", "실패 원인 알려주세요", "에러 로그 보여주세요", "logs", "log", "tail", "build output", "console", "console log", "error log", "runtime log", "why did it fail", "why is it broken", or any axhub log retrieval request. Defaults to build logs and offers pod logs on explicit runtime intent.
---

# Deploy Logs (follow + classify source)

Stream axhub deploy logs in either build or runtime mode. Default `--source=build` because the most common ask is "왜 빌드 실패했어"; switch to `--source=pod` only when the user explicitly says "런타임 로그", "running logs", "컨테이너 로그".

## Workflow

To fetch logs:

1. **Resolve the deployment.** Look up `dep_<id>` from cache or ask the user:

   ```bash
   ${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers resolve --intent logs --user-utterance "$ARGS" --json
   ```

   On `cache_hit: false`, follow `../deploy/references/recovery-flows.md` ("cold-cache") to surface the last 3 deploys via AskUserQuestion.

2. **Pick source.** Default `--source=build`. Switch to `--source=pod` only when the utterance contains "런타임 로그", "running logs", "컨테이너 로그", "pod logs", or when the deploy is already in a `health_check`/terminal `succeeded` phase. When uncertain, ask once via AskUserQuestion ("빌드 로그 / 런타임 로그 / 둘 다").

3. **Stream logs with SSE follow:**

   ```bash
   axhub deploy logs dep_<DEPLOY_ID> --app <APP_ID> --follow --source build --json
   ```

   For pod logs, swap `--source build` with `--source pod`.

4. **Handle SSE eof + resume.** Watch for the `eof:true` sentinel — that is the natural terminator, not a transport error. If the stream drops mid-flight, resume once via `Last-Event-ID` (CLI handles this automatically when re-invoked with `--follow`); never attempt a second resume from the agent side (avoids re-spam to the user).

5. **Render trimmed output.** For non-failure logs, show the last 50 lines plus a "전체 보기" AskUserQuestion option. For failure logs, show the last 200 lines and surface the first error-level line at the top with "이 줄에서 멈춘 것 같아요:".

6. **On non-zero exit**, route to `../deploy/references/error-empathy-catalog.md`:
   - exit 65 → token expired
   - exit 67 → deploy id not found + did-you-mean
   - exit 68 → rate limit (logs is the most rate-limited surface)
   - exit 1 → transport; allow one retry on read path

7. **No source available.** If both build and pod logs return empty, emit: "아직 로그가 없어요. 배포가 시작되기 전이거나, 빌드 단계가 너무 빨라서 출력이 캡처 안 됐을 수 있어요. 'status'로 단계 먼저 확인해보시겠어요?"

## NEVER

- NEVER drop `--json` (NDJSON parsing depends on it).
- NEVER attempt more than one `Last-Event-ID` resume per stream (PLAN §3.1 contract).
- NEVER default to `--source=pod` (build logs are the failure-mode default).
- NEVER echo `axhub_pat_*` tokens that may appear in logs — the redact filter handles this but skill output stays in the helper-redacted lane.
- NEVER continue streaming after the user types "그만" / "stop" / "충분해" — kill the process.

## Additional Resources

For Korean trigger lexicon (logs intent): `../deploy/references/nl-lexicon.md`.
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For SSE resume + cold-cache flows: `../deploy/references/recovery-flows.md`.
