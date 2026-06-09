---
name: logs
description: '이 스킬은 사용자가 axhub 배포의 빌드 로그 또는 런타임 로그를 보고 싶어할 때 사용합니다. 다음 표현에서 활성화: "런타임 로그", "런타임 로그 봐", "로그", "로그 까봐", "로그 보여주세요", "로그 보여줘", "로그 봐", "방금 거 로그", "빌드 로그", "빌드 로그 봐", "빌드 로그 확인해주세요", "실패 원인 알려주세요", "에러", "에러 로그 보여주세요", "에러 메시지 봐", "에러 봐", "왜 깨졌", "왜 깨졌어", "왜 실패", "왜 실패했어", "왜 안돼", "왜 죽었", "왜 죽었어", "출력", "출력 보여줘", "콘솔", "콘솔 봐", "build output", "console", "console log", "error log", "log", "logs", "runtime log", "tail", "why did", "why did it fail", "why is", "why is it broken", 또는 axhub 로그 조회 요청. 기본값은 빌드 로그이며 명시적 런타임 의도가 있을 때만 pod 로그를 제시합니다.'
examples:
  - utterance: "로그 보여줘"
    intent: "view axhub deployment logs"
  - utterance: "axhub-helpers 빌드 logs 보여"
    intent: "view axhub deployment logs"
  - utterance: "logs"
    intent: "view axhub deployment logs"
  - utterance: "tail logs"
    intent: "view axhub deployment logs"
  - utterance: "런타임 로그"
    intent: "view axhub deployment logs"
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Deploy Logs (follow + classify source)

Stream axhub deploy logs in either build or runtime mode. Default `--source=build` because the most common ask is "왜 빌드 실패했어"; switch to `--source=pod` only when the user explicitly says "런타임 로그", "running logs", "컨테이너 로그".

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To fetch logs:

**Claude Desktop visible contract:** start with `로그를 확인할게요.` when the host permits visible text before tools. Use one Bash tool with the Korean title `로그 확인`. Do not show intermediate resolver text, English planning sentences, JSON field names, raw selector names, command names, or deployment-cache labels to the user.

**Tenant 선택 (axhub-tenant-picker:L1).** axhub-helpers `tenant-resolve` 가 캐시(`.axhub/state/tenant.json`)/tenants list/preflight 로 tenant 를 결정해요. fence 간 env 는 휘발하므로 결정된 tenant 를 캐시에 영속화해서 다음 fence 가 re-read 해요. 명시 `AXHUB_TENANT` override 가 있으면 helper 를 건너뛰어요.

```bash
# axhub-tenant-picker:L1 — thin resolver (위험 로직은 Rust axhub-helpers tenant-resolve 가 소유)
TENANT_CACHE=".axhub/state/tenant.json"
NEEDS_PICK="false"
CANDIDATES_JSON="[]"
# Precedence 1: 명시 AXHUB_TENANT env override → helper 호출 skip
if [ -z "${AXHUB_TENANT:-}" ]; then
  HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
  [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
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

1. **일반 로그 요청은 한 번에 요약해요.** The helper resolves the app, picks the most recent deployment, fetches a bounded log snapshot, redacts secrets, and prints a Korean user-facing summary. Do not narrate resolver steps, cache state, raw deploy-list/log commands, or snapshot fallback mechanics to the user.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   USER_UTTERANCE="<the user's exact latest sentence>"
   "$HELPER" logs-summary --user-utterance "$USER_UTTERANCE"
   ```

   Show the Korean stdout as-is. If it says the app, deployment, or logs are missing, stop there and ask a natural follow-up. For ordinary Claude Desktop log questions, stop after this step.

2. **Advanced manual path for explicit follow/stream/debug requests only.** Look up `dep_<id>` from cache or ask the user:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" resolve --intent logs --user-utterance "$ARGS" --json
   ```

   On `cache_hit: false`, follow `../deploy/references/recovery-flows.md` ("cold-cache"): ask the user which app first (`axhub apps list --json`), then surface the last 3 deploys via the helper:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" list-deployments --app <APP_ID> --limit 3
   ```

   On exit 65 (`list-deployments` helper 의 EXIT_LIST_AUTH OUTPUT 계약 — classify-exit 가 4 로 정규화해요; token missing — Phase 7 US-701 이후엔 SessionStart 가 자동 setup):
   > "토큰을 찾을 수 없어요. 'axhub auth login' 또는 CC 세션 재시작."

   v0.17.4 CLI 는 `axhub deploy list --app <APP> --json` 으로 직접 배포 목록을 조회할 수 있어요 (아래 build-log snapshot fallback 에서도 이 명령을 써요). 위 helper 는 같은 데이터를 canonical CLI wrapper 로 받아 캐시 친화적으로 정리해 주는 경로예요 — auth/transport 정책은 CLI 를 그대로 따라가요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — source pick → `build` (가장 흔한 use case), 전체 보기 → `last 50` (subprocess 에서는 trimmed 만 보여줘요).

3. **Pick source.** Default `--source=build`. Switch to `--source=pod` only when the utterance contains "런타임 로그", "running logs", "컨테이너 로그", "pod logs", or when the deploy is already in a `health_check`/terminal `succeeded` phase. When uncertain, ask once via AskUserQuestion ("빌드 로그 / 런타임 로그 / 둘 다").

   **No-deploy precheck.** Before any `axhub deploy logs ...` call, verify there is a concrete deployment id for the chosen app. Use the helper/CLI list result from Step 1; if it is empty, stop cleanly with "아직 배포가 없어서 로그도 없어요. 먼저 배포를 시작한 뒤 다시 로그를 볼 수 있어요." Do **not** call app-level `axhub deploy logs --app <APP>` without a deploy id: current backend returns exit 7 + API 500 `internal_error` ("로그를 불러오지 못했어요") for no-deploy apps, which is noisy and should not be user-facing.

4. **Stream logs with SSE follow:**

   ```bash
   axhub deploy logs dep_<DEPLOY_ID> --app <APP_ID> --follow --source build --json
   ```

   For pod logs, swap `--source build` with `--source pod`.

   **에이전트 컨텍스트 자동 degrade (axhub-cli 0.15.3+).** `--follow` 를 항상 그대로 전달해요. CLI 가 비-TTY/에이전트 컨텍스트를 자동 감지해서 단일 스냅샷으로 degrade 한 뒤 즉시 종료하니 (`/axhub:logs` 가 더 이상 hang 안 나요), 수동 drop guard 는 불필요해요. 명시적 `--reconnect-attempts N` (N>0) 을 주면 에이전트 컨텍스트에서도 N 회 forward 폴링을 유지해요 (의도된 bounded streaming opt-in). 그 외에 `--no-input` 같은 플래그는 따로 안 붙여도 돼요 — 비-TTY 면 CLI 가 자동 감지하니까요.

   **Build-log snapshot fallback:** The current backend can return `validation.build_logs_require_follow` for `--source build` without `--follow`. In non-interactive mode, do not re-add `--follow` and do not treat this as a user-facing failure. Instead fetch the deployment snapshot and render the embedded build log:

   ```bash
   STATUS_JSON="$(axhub deploy status dep_<DEPLOY_ID> --app <APP_ID> --json)"
   APP_ID="$(printf '%s' "$STATUS_JSON" | jq -r '.app_id')"
   axhub deploy list --app "$APP_ID" --json
   ```

   Select the matching deployment id from `.items[]` (or CLI envelope `.data.items[]`), read `.build_log`, and show the last 50 lines. If `.build_log` is still empty, explain that the backend has no build-log snapshot yet and suggest an interactive `--follow` run.

5. **Handle SSE eof + resume.** Watch for the `eof:true` sentinel — that is the natural terminator, not a transport error. If the stream drops mid-flight, resume once via `Last-Event-ID` (CLI handles this automatically when re-invoked with `--follow`); never attempt a second resume from the agent side (avoids re-spam to the user).

6. **Render trimmed output.** For non-failure logs, show the last 50 lines plus a "전체 보기" AskUserQuestion option. For failure logs, show the last 200 lines and surface the first error-level line at the top with "이 줄에서 멈춘 것 같아요:".

7. **On non-zero exit**, route via `axhub-helpers classify-exit "$EXIT" "$STDOUT"` (spec 004 Fork-A — canonical router) or the catalog `../deploy/references/error-empathy-catalog.md` by current CLI exit code:
   - exit 4 → token expired (was sysexits 65)
   - exit 5 → deploy id not found + did-you-mean (was 67)
   - exit 6 → rate limit (logs is the most rate-limited surface; was 68)
   - exit 1 → transport; allow one retry on read path

8. **No source available.** If both build and pod logs return empty, emit: "아직 로그가 없어요. 배포가 시작되기 전이거나, 빌드 단계가 너무 빨라서 출력이 캡처 안 됐을 수 있어요. 'status'로 단계 먼저 확인해볼래요?"

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
