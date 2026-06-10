---
name: resources
description: '이 스킬은 사용자가 게이트웨이 리소스(외부 DB 테이블/뷰)를 이름 변경·이동·네임스페이스 구성·태그·삭제로 조직하고 싶어할 때 사용해요. 다음 표현에서 활성화: "리소스 이름 바꿔", "리소스 이동", "네임스페이스 만들", "리소스 태그", "리소스 정리", "리소스 삭제", "리소스 등록", "rename resource", "move resource", "namespace", "tag resource", "bulk register", 또는 axhub 게이트웨이 리소스 조직 의도. 리소스 조회/인벤토리는 my-resources, 데이터 읽기는 data 스킬이에요.'
examples:
  - utterance: "리소스 정리하고 싶어"
    intent: "organize gateway resources"
  - utterance: "리소스 이름 바꿔"
    intent: "organize gateway resources"
  - utterance: "리소스 이동"
    intent: "organize gateway resources"
  - utterance: "네임스페이스 만들"
    intent: "organize gateway resources"
  - utterance: "tag resource"
    intent: "organize gateway resources"
  - utterance: "bulk register"
    intent: "organize gateway resources"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Gateway resources organization

게이트웨이 리소스의 namespace, rename, move, bulk-register, delete, tag 작업을 담당해요. 조회는 가능하지만 인벤토리/데이터 읽기는 기존 skill 로 분리해요.

## Claude Desktop Natural-Language Path

For ordinary Claude Desktop prompts such as `리소스 정리하고 싶어`, treat the request as AXHub gateway resource organization, not local file cleanup.

- The first visible chat sentence must be exactly `리소스 정리 방식을 확인할게요.`
- Immediately after that sentence, use exactly one Bash tool titled `리소스 현황 확인`:
  `axhub-helpers resources-summary --user-utterance "<latest user sentence>"`
- After the Bash tool completes, use its Korean stdout as the answer. Do not add a second table, ToolSearch explanation, raw command name, or tool-result interpretation layer.
- Do not say the prompt is `모호` or describe it as an internal ambiguity judgement.
- Do not inspect local files, repo files, `.shim`, `.omc`, QA result files, git status, package files, plugin source files, or local cleanup candidates.
- Check login/workspace/resource availability using Korean Bash titles only: `로그인 상태 확인`, `리소스 현황 확인`, `리소스 변경 준비`, or `리소스 변경 실행`.
- If the user did not name the exact target or action, ask in normal chat: `어떤 정리를 할까요? 목록 확인, 이름 변경, 이동, 태그 정리, 등록, 삭제 중에서 골라주세요.`
- Before delete, move, rename, tag changes, namespace create, or bulk-register, show a Korean preview of the target resource and action, then wait for explicit approval. Do not run a mutation before approval.
- Do not say resource changes are impossible just because the initial status check only listed resources. If a change is requested, ask for the target and approval, then use the mutation flow below.
- Do not call AskUserQuestion, Question, or a question-card tool in Claude Desktop for this first disambiguation; use normal chat so raw question JSON cannot leak.
- Do not write route labels, slash commands, skill names, `workflow`, `워크플로`, `preflight`, `catalog kinds`, `connector/resource`, raw question JSON, command names, raw command lines, raw JSON fields, raw IDs, raw emails, local file paths, local artifact names, or English tool-title fragments in visible text.
- Login checks should say only `로그인되어 있어요` or `다시 로그인이 필요해요`; do not show account email, raw user id, scope, tenant names, or exact expiry unless the user explicitly asks for identity details.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

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

**Tenant grounding.** 리소스 조직 작업은 tenant-scoped 예요. 사용자가 tenant 를 명시하지 않았으면 preflight 의 active team 만 사용하고, active team 이 없으면 실행을 멈춰요. `tenants[]` 첫 항목을 추측해서 쓰지 않아요.

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
     { content: "리소스 resolve", status: "pending", activeForm: "대상 확인 중" },
     { content: "preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "실행 중" },
     { content: "결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **현재 리소스를 조회해요.**

   ```bash
   axhub resources list --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --parent-id "$PARENT_ID" --json
   ```

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

2. **작업을 선택해요.** 비대화형 기본은 `list`예요.

   ```json
   {"questions":[{"question":"리소스 조직 작업을 골라요","header":"리소스","options":[{"label":"목록 보기","description":"read-only 로 확인해요"},{"label":"정리 작업","description":"이름/이동/태그를 준비해요"},{"label":"삭제","description":"삭제 preview 를 준비해요"}]}]}
   ```

3. **mutation 명령.** delete cascade 는 별도 강한 확인 문구를 보여줘요.

   Approval context 은 helper parser 와 같은 action/context 로 맞춰요: namespace 는 `{name,tenant,parent_id?}`, rename/move/delete/tag 는 `{resource_id,tenant,...}`, bulk-register 는 `{connector_id,source,tenant,items_file?,items_digest}` 예요. bulk-register `source` 는 `--items-file` 일 때 `items_file`, `--items-json` 일 때 `items_json` 으로 맞추고, 파일과 inline JSON 모두 `items_digest:"sha256:..."` 를 같이 확인해요.

   ```bash
   ITEMS_DIGEST="sha256:$(shasum -a 256 items.json | awk '{print $1}')"
   axhub resources namespace create --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --name "$NAME" --parent-id "$PARENT_ID" --execute --json
   axhub resources rename "$RESOURCE_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --name "$NEW_NAME" --execute --json
   axhub resources move "$RESOURCE_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --parent-id "$PARENT_ID" --execute --json
   axhub resources move "$RESOURCE_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --root --execute --json
   axhub resources bulk-register --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --connector-id "$CONNECTOR_ID" --items-file items.json --include-columns --execute --json
   axhub resources delete "$RESOURCE_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --cascade --execute --json
   axhub resources tag-attach "$RESOURCE_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --tag-id "$TAG_ID" --execute --json
   axhub resources tag-detach "$RESOURCE_ID" --tenant "$(jq -r '.tenant // empty' .axhub/state/tenant.json 2>/dev/null)" --tag-id "$TAG_ID" --execute --json
   ```


   Registry-backed confirmation questions used by mutation previews:

   - `이 리소스를 변경할까요?`

## NEVER

- NEVER data live read 를 여기서 실행하지 않아요. `data` skill 로 넘겨요.
- NEVER my-resources 인벤토리와 mutation organization 을 섞지 않아요.
- NEVER cascade delete 를 비대화형에서 자동 실행하지 않아요.
