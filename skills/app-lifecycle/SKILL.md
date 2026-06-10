---
name: app-lifecycle
description: '이 스킬은 명시적 호출에서 axhub hosted app lifecycle 절차를 설명해요. Claude Desktop 일반 자연어 요청은 UserPromptSubmit hook의 inline flow가 처리하므로 이 스킬을 직접 활성화하지 않아요.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "앱 잠깐 멈춰줘"
    intent: "suspend hosted app"
  - utterance: "testnextjs 다시 켜줘"
    intent: "resume hosted app"
  - utterance: "앱 복제해줘"
    intent: "fork hosted app"
  - utterance: "fork app"
    intent: "fork hosted app"
  - utterance: "resume app"
    intent: "resume hosted app"
---

# App lifecycle

앱 복제, 일시정지, 다시 켜기를 담당해요. 서비스 상태에 영향을 줄 수 있는 작업이라 영향 안내와 명시적 확인 뒤에만 변경을 실행해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

## Claude Desktop Natural-Language Path

When the user asks in ordinary language, keep the visible flow human-readable. Do not narrate routing, parsing, shell state, or helper internals.

Natural phrases such as `앱 복제`, `앱 포크`, `앱 복사해`, `앱 일시정지`, `앱 멈춰`, `앱 잠깐 멈춰`, `앱 잠시 내려`, `앱 중지`, `앱 정지`, `앱 재개`, `앱 다시 켜`, `앱 다시 올려`, `testnextjs 다시 켜줘`, `testnextjs 멈춰줘`, `fork app`, `suspend app`, `pause app`, and `resume app` should be handled by the prompt-route inline flow in Claude Desktop. If this skill is opened explicitly, still use the same human-readable flow below.

1. **First visible sentence.** Start with exactly one of these natural Korean sentences:
   - pause intent: `<앱 이름> 앱을 잠깐 멈출 준비를 할게요.`
   - resume intent: `<앱 이름> 앱을 다시 켤 준비를 할게요.`
   - fork intent: `<앱 이름> 앱을 복제할 준비를 할게요.`

2. **Tool titles.** For Bash tool calls, set the title/description to Korean user-facing text only:
   - `앱 상태 확인`
   - `앱 찾기`
   - `앱 변경 준비`
   - `앱 변경 실행`
   Do not title tools with internal command names, identifiers, lifecycle verbs in English, permission words, or raw command fragments.

3. **Visible summaries.** Summarize command output in plain Korean. Do not display raw JSON, raw IDs except a short masked suffix when needed, account emails, owner names, raw user IDs, `auth OK`, `current_app`, `app_id`, `resolve`, `preflight`, `runtime`, `dry-run`, `execute`, `suspend`, `resume`, `fork`, or English status fragments. Say:
   - `로그인이 필요해요. 다시 로그인해줘라고 말하면 이어서 진행할 수 있어요.`
   - `로그인 상태를 확인했어요.`
   - `앱을 찾았어요: <앱 이름>. 현재 상태는 <상태>예요.`
   - If another lookup is needed, say `앱을 한 번 더 확인할게요.` Do not describe identifier lookup.
   - pause warning: `멈추면 다시 켤 때까지 서비스 주소에 접속할 수 없어요.`
   - resume warning: `다시 켜도 서비스가 완전히 살아나는 데 시간이 걸릴 수 있어요.`
   - Translate raw enum values into Korean labels only: `private` -> `비공개`, `public` -> `공개`, `development` -> `개발 단계`, `production` -> `운영 단계`. Never write mixed labels such as `비공개 (private)`.

4. **Confirmation card.** Before any change, ask exactly this question and keep option labels natural. If the host supports option values, set each `value` to exactly the same Korean text as its visible `label`:

   ```json
   {"questions":[{"question":"앱 변경을 실행할까요?","header":"앱 확인","options":[{"label":"취소","value":"취소","description":"아무것도 바꾸지 않아요"},{"label":"진행","value":"진행","description":"위에 적은 앱 변경을 실행해요"}]}]}
   ```

5. **After answer.**
   - If the answer is `취소`, say exactly `알겠어요. 아무것도 바꾸지 않았어요.`
   - If the answer is `진행`, do not write any visible sentence before tool calls. Never say `User chose`, `Confirm approval`, `execute suspend`, `execute resume`, `execute fork`, or similar implementation narration. Do not run the app-changing command first. First run the matching internal approval preparation with Bash title `앱 변경 준비`, hide its stdout from chat, then run the matching top-level `axhub` command with Bash title `앱 변경 실행`. The app-changing command also returns raw JSON; redirect that stdout away from the visible tool panel and use the zero exit status as success. Treat a visible `[DESTRUCTIVE] about to run ...` line as a hook notice, not a failure. The first `앱 변경 실행` with exit code 0 is terminal success: do not run another preparation/execution pair, do not re-run the same mutation for verification, and do not continue to a second app-changing command. 같은 변경을 다시 준비하거나 다시 실행하지 않아요. After it succeeds, say exactly one short result sentence in Korean: `<앱 이름> 앱을 잠깐 멈췄어요.` or `<앱 이름> 앱을 다시 켰어요.`
   - If the security gate still blocks the change, do not explain the gate internals. Prepare the same approved change once more and retry once. If it still fails, say exactly `앱 변경을 시작하지 못했어요. 다시 시도해 주세요.`

NEVER include parenthesized internal labels such as `(suspend)`, `(resume)`, `(preflight)`, or `(execute)` in visible chat.
NEVER mention internal authorization primitives, token words, permission-decision details, helper binding details, JSON, schema, fixture, helper source, or English implementation words in visible chat.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s
' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d	%s
",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `axhub_bin_invalid` 는 `AXHUB_BIN` 환경변수가 잘못된 경로 (`cli_resolved_path` 값) 를 가리키는 상태라 재설치 대신 `unset AXHUB_BIN` 후 새 세션 재시도 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

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

**Tenant grounding.** `fork` 는 destination tenant 를 mutation context 에 묶어요. 사용자가 tenant 를 명시하지 않았으면 preflight 의 active team 만 사용하고, active team 이 없으면 실행을 멈춰요. `tenants[]` 첫 항목을 추측해서 쓰지 않아요.

```bash
# tenant fence re-read — fence 간 env 휘발, .axhub/state/tenant.json 재읽기 (L1 이 영속화한 값)
AXHUB_TENANT="${AXHUB_TENANT:-$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null || true)}"
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
if [ -z "$TENANT" ]; then
  echo "현재 workspace 를 특정할 수 없어요. workspace skill 로 tenant 를 확인하거나 AXHUB_TENANT 를 명시해요." >&2
  exit 64
fi
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "작업 확인", status: "in_progress", activeForm: "작업 고르는 중" },
     { content: "앱 확인", status: "pending", activeForm: "앱 확인 중" },
     { content: "영향 안내", status: "pending", activeForm: "영향 안내 중" },
     { content: "변경 확인", status: "pending", activeForm: "변경 확인 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "앱 변경 실행 중" },
     { content: "후속 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

6. **작업을 분기해요.** fork/suspend/resume 중 하나예요. 내부 action 이름은 명령 구성에만 쓰고, Desktop 사용자에게는 복제/잠깐 멈춤/다시 켜기로 말해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

7. **AskUserQuestion 으로 작업을 확인해요.** 비대화형 기본은 `취소`예요.

   ```json
   {"questions":[{"question":"앱 변경을 실행할까요?","header":"앱 확인","options":[{"label":"취소","value":"취소","description":"아무것도 바꾸지 않아요"},{"label":"진행","value":"진행","description":"위에 적은 앱 변경을 실행해요"}]}]}
   ```

8. **CLI 명령.**

   변경 명령은 AskUserQuestion 에서 `진행`을 받은 뒤 정확히 한 번만 실행해요. 준비용 helper 호출이나 별도 approval subcommand 는 없어요. `--app` 값은 resolved UUID 가 아니라 바로 다음 `axhub apps suspend|resume ...` 명령에 들어갈 literal 앱 인자와 정확히 같아야 해요. 앱 변경 실행 명령의 raw JSON stdout 은 사용자 도구 패널에 남기지 않도록 `>/dev/null` 로 버려요. 첫 번째 `앱 변경 실행` 이 exit code 0 으로 끝나면 성공으로 보고 즉시 결과 문장으로 마무리해요.

   Pick exactly one branch. Between the user's `진행` answer and the Bash tool call, do not write a visible chat sentence.

   Pause execution:

   ```bash
   APP_ARG="${APP_ARG:-testnextjs}"  # literal argument used in the axhub apps command, not a resolved UUID
   axhub apps suspend "$APP_ARG" --execute --json >/dev/null
   ```

   Resume execution:

   ```bash
   APP_ARG="${APP_ARG:-testnextjs}"  # literal argument used in the axhub apps command, not a resolved UUID
   axhub apps resume "$APP_ARG" --execute --json >/dev/null
   ```

   Fork execution:

   ```bash
   axhub apps fork "$SOURCE_APP" --slug "$NEW_SLUG" --subdomain "$NEW_SUBDOMAIN" --name "$NAME" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --execute --json >/dev/null
   ```

9. **후속 안내.** resume 은 자동 redeploy 를 보장하지 않으니 필요하면 `deploy` skill 로 이어가요.

## NEVER

- NEVER GitHub repo connect/disconnect 를 여기서 처리하지 않아요. `github` skill 로 넘겨요.
- NEVER suspend/resume 를 read-only 처럼 표현하지 않아요.
- NEVER 비대화형에서 앱 runtime 변경을 자동 실행하지 않아요.
