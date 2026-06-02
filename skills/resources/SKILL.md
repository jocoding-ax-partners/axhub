---
name: resources
description: '이 스킬은 사용자가 게이트웨이 리소스(외부 DB 테이블/뷰)를 이름 변경·이동·네임스페이스 구성·태그·삭제로 조직하고 싶어할 때 사용해요. 다음 표현에서 활성화: "리소스 이름 바꿔", "리소스 이동", "네임스페이스 만들", "리소스 태그", "리소스 정리", "리소스 삭제", "리소스 등록", "rename resource", "move resource", "namespace", "tag resource", "bulk register", 또는 axhub 게이트웨이 리소스 조직 의도. 리소스 조회/인벤토리는 my-resources, 데이터 읽기는 data 스킬이에요.'
examples:
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

## Workflow

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s
' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d	%s
",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

**Tenant grounding.** 리소스 조직 작업은 tenant-scoped 예요. 사용자가 tenant 를 명시하지 않았으면 preflight 의 active team 만 사용하고, active team 이 없으면 실행을 멈춰요. `tenants[]` 첫 항목을 추측해서 쓰지 않아요.

```bash
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
if [ -z "$TENANT" ]; then
  echo "현재 workspace 를 특정할 수 없어요. workspace skill 로 tenant 를 확인하거나 AXHUB_TENANT 를 명시해요." >&2
  exit 64
fi
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

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
   axhub resources list --tenant "$TENANT" --parent-id "$PARENT_ID" --json
   ```

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

2. **작업을 선택해요.** 비대화형 기본은 `list`예요.

   ```json
   {"questions":[{"question":"리소스 조직 작업을 골라요","header":"리소스","options":[{"label":"목록 보기","description":"read-only 로 확인해요"},{"label":"정리 작업","description":"이름/이동/태그를 준비해요"},{"label":"삭제","description":"삭제 preview 를 준비해요"}]}]}
   ```

3. **mutation 명령.** delete cascade 는 별도 강한 확인 문구를 보여줘요.

   Consent binding 은 helper parser 와 같은 action/context 로 맞춰요: namespace 는 `{name,tenant,parent_id?}`, rename/move/delete/tag 는 `{resource_id,tenant,...}`, bulk-register 는 `{connector_id,source,tenant,items_file?,items_digest}` 예요. bulk-register `source` 는 `--items-file` 일 때 `items_file`, `--items-json` 일 때 `items_json` 으로 맞추고, 파일과 inline JSON 모두 `items_digest:"sha256:..."` 를 같이 mint 해요.

   ```bash
   ITEMS_DIGEST="sha256:$(shasum -a 256 items.json | awk '{print $1}')"
   axhub resources namespace create --tenant "$TENANT" --name "$NAME" --parent-id "$PARENT_ID" --execute --json
   axhub resources rename "$RESOURCE_ID" --tenant "$TENANT" --name "$NEW_NAME" --execute --json
   axhub resources move "$RESOURCE_ID" --tenant "$TENANT" --parent-id "$PARENT_ID" --execute --json
   axhub resources move "$RESOURCE_ID" --tenant "$TENANT" --root --execute --json
   axhub resources bulk-register --tenant "$TENANT" --connector-id "$CONNECTOR_ID" --items-file items.json --include-columns --execute --json
   axhub resources delete "$RESOURCE_ID" --tenant "$TENANT" --cascade --execute --json
   axhub resources tag-attach "$RESOURCE_ID" --tenant "$TENANT" --tag-id "$TAG_ID" --execute --json
   axhub resources tag-detach "$RESOURCE_ID" --tenant "$TENANT" --tag-id "$TAG_ID" --execute --json
   ```


   Registry-backed confirmation questions used by mutation previews:

   - `이 리소스를 변경할까요?`

## NEVER

- NEVER data live read 를 여기서 실행하지 않아요. `data` skill 로 넘겨요.
- NEVER my-resources 인벤토리와 mutation organization 을 섞지 않아요.
- NEVER cascade delete 를 비대화형에서 자동 실행하지 않아요.
