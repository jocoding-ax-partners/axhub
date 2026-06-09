---
name: status
description: '이 스킬은 사용자가 배포 진행 상황 또는 배포 상태를 묻거나 추적하고 싶어할 때 사용해요. 다음 표현에서 활성화: "끝났", "끝났어", "다 됐", "다 됐어", "됐어", "떴어", "라이브 됐", "라이브 됐어", "반영 됐", "반영 됐어", "방금 거 됐어", "배포 끝났나요", "배포 상태", "빌드 됐", "빌드 됐어", "상태 봐", "어디까지", "어디까지 됐나요", "어디쯤", "어디쯤이야", "어떻게 됐", "어떻게 됐어", "올라갔", "올라갔어", "지금 어디까지", "진행 상황", "진행 상황 알려주세요", "진행 중", "진행 중이야", "build status", "deploy state", "deploy status", "follow", "is it done", "progress", "watch", 또는 진행 중 axhub 배포를 추적하는 모든 요청. 일반 CLI 상태 조회는 inspect skill 의 `axhub status --json` 으로 처리해요.'
examples:
  - utterance: "어디까지 됐어"
    intent: "check axhub deployment status"
  - utterance: "지금 진행 중인 배포 어떻게 됐어"
    intent: "check axhub deployment status"
  - utterance: "방금 배포한 거 deploy status"
    intent: "check axhub deployment status"
  - utterance: "is it done"
    intent: "check axhub deployment status"
  - utterance: "deploy status"
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

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To check status:

**Claude Desktop visible contract:** start with `배포 상태를 확인할게요.` when the host permits visible text before tools. Use one Bash tool with the Korean title `배포 상태 확인`. Do not show intermediate resolver failures, JSON field names, raw selector names, environment-mode labels, English tool titles, or command names to the user.

**Tenant 선택 (axhub-tenant-picker:L1).** axhub-helpers `tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요. fence 간 env 는 휘발하므로 결정된 tenant 를 캐시에 영속화해서 다음 fence 가 re-read 해요. 명시 `AXHUB_TENANT` override 가 있으면 helper 를 건너뛰어요.

```bash
# axhub-tenant-picker:L1 — thin resolver (위험 로직은 Rust axhub-helpers tenant-resolve 가 소유)
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
# Precedence 1: 명시 AXHUB_TENANT env override → helper 호출 skip
if [ -z "${AXHUB_TENANT:-}" ]; then
  HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
  if [ -n "$HELPER" ] && [ ! -x "$HELPER" ] && [ -x "${HELPER}.exe" ]; then HELPER="${HELPER}.exe"; fi
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null || command -v axhub-helpers.exe 2>/dev/null)"
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers* "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers*; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
  TENANT_JSON=$([ -n "$HELPER" ] && "$HELPER" tenant-resolve --json 2>/dev/null)
  [ -n "$TENANT_JSON" ] || TENANT_JSON='{}'
  AXHUB_TENANT=$(printf '%s' "$TENANT_JSON" | jq -r '.tenant // empty' 2>/dev/null || true)
  _NEEDS_PICK_RAW=$(printf '%s' "$TENANT_JSON" | jq -r '.needs_pick // false' 2>/dev/null || echo false)
  # no-loop: needs_pick 는 비어있지 않은 resolve 에서만 true; 빈/부재 helper → false (재프롬프트 안 함)
  if [ "$_NEEDS_PICK_RAW" = "true" ]; then
    CANDIDATES_JSON=$(printf '%s' "$TENANT_JSON" | jq -c '.candidates // []' 2>/dev/null || echo '[]')
    if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]; then
      # non-TTY: active fallback + 경고 (R4 fail-wrong guard — bash 위치 필수)
      AXHUB_TENANT=$(printf '%s' "$CANDIDATES_JSON" | jq -r '.[0].id // .[0].slug // empty' 2>/dev/null || true)
      echo "여러 tenant 에 속해 있는데 picker 를 건너뛰고 기본 tenant($AXHUB_TENANT)로 진행해요"
    else
      NEEDS_PICK="true"
    fi
  fi
fi
# 결정된 tenant 영속화 (fence 간 source of truth) — needs_pick 대기 중엔 미기록(L2 가 기록)
if [ -n "${AXHUB_TENANT:-}" ] && [ "$NEEDS_PICK" = "false" ]; then
  mkdir -p "$(dirname "$TENANT_CACHE")"
  printf '{"tenant":"%s","source":"resolved","ts":%s}\n' "$AXHUB_TENANT" "$(date +%s 2>/dev/null || echo '0')" > "$TENANT_CACHE"
fi
export AXHUB_TENANT
export NEEDS_PICK
export CANDIDATES_JSON
```

`AXHUB_TENANT` 가 비어 있으면 tenant 를 확정할 수 없어요 — preflight `auth_ok` 와 `current_team_id` 를 먼저 확인하고 `다시 로그인해줘` 라고 안내해요. 구버전·부재 helper 면 빈 값 → active tenant 로 진행하고, picker 는 helper 업데이트 후 돌아와요.

**Tenant picker (axhub-tenant-picker:L2).** `NEEDS_PICK=true` 이고 대화형 TTY 일 때만 실행해요. `CANDIDATES_JSON` 에서 후보 목록을 읽어 AskUserQuestion 으로 사용자에게 선택을 요청해요. 선택 결과를 `.axhub/state/tenant.json` 에 `{tenant, source:"picker", ts}` 형태로 기록해요 (이후 fence 가 re-read 해서 상속).

```typescript
if (NEEDS_PICK === "true") {
  const candidates = JSON.parse(CANDIDATES_JSON);
  AskUserQuestion({
    questions: [{
      question: "어떤 tenant 로 진행할까요?",
      header: "Tenant",
      multiSelect: false,
      options: candidates.map((t: { id?: string; slug?: string; name?: string }) => ({
        label: t.name ?? t.slug ?? t.id ?? "unknown",
        description: `ID: ${t.id ?? t.slug}`,
      })),
    }],
  });
  // 선택된 tenant ID 를 .axhub/state/tenant.json 에 write-back
  // mkdir -p .axhub/state && echo '{"tenant":"<선택값>","source":"picker","ts":<epoch>}' > .axhub/state/tenant.json
}
```

AskUserQuestion 답변을 받은 뒤 선택된 tenant ID 를 `AXHUB_TENANT` 로 확정하고 `.axhub/state/tenant.json` 에 `{"tenant": "<id>", "source": "picker", "ts": <epoch>}` 를 기록해요. 이후 fence 가 이 파일을 re-read 해서 같은 tenant 를 재사용해요.

**Non-interactive AskUserQuestion guard (D1):** `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 환경에서는 L2 AskUserQuestion 을 건너뛰어요 — L1 블록이 이미 active fallback + 경고를 처리했어요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 `picker` 채널 참조.

1. **상태 요약 한 번만 실행해요.** The helper resolves the app, picks the most recent deployment, checks status, and prints a Korean user-facing summary. Do not read `axhub.yaml`, run raw `axhub deploy list`, or run raw `axhub deploy status` unless the user explicitly asks for raw details.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   USER_UTTERANCE="<the user's exact latest sentence>"
   "$HELPER" status-summary --user-utterance "$USER_UTTERANCE"
   ```

   Show the Korean stdout as-is. If it says the app or deployment is missing, stop there and ask a natural follow-up. Cold-cache deploy selection uses the registered AskUserQuestion text `어떤 배포 상태를 볼까요?` and defaults to the most recent deployment in non-interactive hosts. Do not recover by reading files or showing raw command output. For ordinary Claude Desktop status questions, stop after this step.

2. **Raw watch fallback for explicit advanced watch requests only** (ordinary Desktop status questions must skip this):

3. **Pre-flight version check** (only if mutation chain is implied — pure read can skip):

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" preflight --json
   ```

4. **상태 확인.** NDJSON 스트림을 `--watch --watch-timeout` 으로 받아요:

   ```bash
   axhub deploy status "${DEPLOYMENT_ID}" --app "${APP}" --watch --watch-timeout 9m --json
   ```

   **에이전트도 terminal 까지 폴링해요 (axhub-cli 0.15.3+).** bare `--watch` 는 비-TTY/에이전트 컨텍스트에서 단일 스냅샷으로 degrade 하지만, `--watch-timeout` (또는 `--watch-interval`) 을 붙이면 explicit streaming override 라 CLI 가 degrade 하지 않고 terminal status(`succeeded` / `failed` / `cancelled` / `rolled_back`) 까지 직접 폴링하면서 NDJSON `stage_transition` 을 emit 해요. 그래서 사용자가 "아직도 진행 중이야?" 하고 다시 안 물어도 돼요. 이 bash 는 Bash tool `timeout: 570000` (9.5분, `--watch-timeout 9m` 보다 약간 큼) 으로 호출해요. 9분 초과 시 CLI 가 Timeout error + resume hint 를 주니, 완료를 선언하지 말고 "아직 진행 중이에요, 계속 확인할게요" 후 같은 명령을 한 번 더 호출해요. 사람 TTY 에서도 같은 명령이 스트림으로 watch 돼요.

5. **Render Korean narration (interactive TTY 전용).** 사람이 TTY 로 watch 할 때만 적용해요 — 에이전트 컨텍스트는 위에서 스냅샷으로 degrade 되니까 narration 대신 단일 상태 요약을 보여줘요. Apply the throttle + phase table from `../deploy/references/recovery-flows.md` ("watch-narration"): one line per ~25s, terminal-state lines are unthrottled. Examples:

   - 0s + `queued` → "배포 요청 받았어요. 잠시 후 빌드 시작해요 (정상)"
   - ~30s + `building` → "30초 경과, 빌드 시작했어요 (정상)"
   - ~1m + `building` → "1분 경과, 빌드 중이에요 (정상). 보통 2~3분 정도 걸려요"
   - ~2m + `pushing_image` → "2분 경과, 이미지 푸시 중이에요 (정상). 거의 다 왔어요"
   - ~3m + `health_check` → "헬스체크 중. 마지막 단계예요"
   - terminal `succeeded` → trigger exit 0 success template
   - terminal `failed` → trigger exit 1/4/5/6/66 template per emitted error

6. **Silent stream guard.** If 60s pass with no NDJSON event, emit "조용하네요. 서버 응답 기다리는 중이에요 (정상). 30초 후 다시 알려줄게요."

7. **User interrupt.** If the user says "그만 봐", "그만", "충분해", "stop watching", terminate the watch process and report the last observed phase. The deploy continues server-side regardless.

8. **On any non-zero exit**, route to `../deploy/references/error-empathy-catalog.md` by exit code:
   - exit 4 → token expired template + AskUserQuestion to run auth login. (canonical 분류는 `axhub-helpers classify-exit "$EXIT" "$STDOUT"` 가 담당해요 — spec 004 Fork-A. 옛 sysexits 65 아님.)
   - exit 5 → resource not found + did-you-mean from `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers list-deployments --app <APP>` (helper-exit 67 OUTPUT 계약은 유지 — INPUT 만 CLI 5)
   - exit 6 → rate limit + Retry-After backoff
   - exit 1 + `error_code = "transport.cli_missing"` → axhub CLI 가 PATH 에 없거나 실행 불가. 사용자에게 `axhub --version` 확인 또는 "설치 도와줘"라고 말하면 이어서 도와줄 수 있다고 안내해요. canonical 표는 `../recover/SKILL.md` (Step 7).
   - exit 1 → transport error; retry the watch once for read paths

## NEVER

- NEVER drop `--json` from `axhub deploy status` (parsing depends on it).
- NEVER echo the raw NDJSON tick stream verbatim — vibe coders cannot read it.
- NEVER auto-trigger `axhub deploy create` from the status path (read-only intent).
- NEVER invent a `deployment_id` when the cache is cold; ask via AskUserQuestion.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, 비대화형 실행) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — cold cache deploy pick → `most_recent` (가장 최근 succeeded), exit-4 re-login → `abort` (subprocess 자동 로그인 안 해요).
- NEVER throttle the terminal-state narration — success/failure must surface immediately.

## Additional Resources

For Korean trigger lexicon (반말 / 존댓말 / demo / 경어 status variants): `../deploy/references/nl-lexicon.md` (intent: status).
For 4-part Korean exit-code templates: `../deploy/references/error-empathy-catalog.md`.
For multi-machine cold cache + watch-narration phase table: `../deploy/references/recovery-flows.md`.
