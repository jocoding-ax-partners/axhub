---
name: tables
description: '이 스킬은 사용자가 axhub 앱의 동적 테이블을 만들거나 지우거나, 컬럼·권한·행 데이터를 관리하고 싶어할 때 사용해요. 다음 표현에서 활성화: "테이블 만들", "테이블 생성", "동적 테이블", "컬럼 추가", "컬럼 삭제", "행 추가", "행 넣어", "레코드 삽입", "데이터 넣어", "행 삭제", "테이블 권한", "create table", "add column", "insert row", "delete row", 또는 axhub 동적 테이블 스키마·행 관리 의도. 외부 커넥터 SQL 조회·인사이트는 data 스킬이 담당해요.'
examples:
  - utterance: "테이블 만들어"
    intent: "manage dynamic tables"
  - utterance: "컬럼 추가해"
    intent: "manage dynamic tables"
  - utterance: "행 넣어"
    intent: "manage table rows"
  - utterance: "create table"
    intent: "manage dynamic tables"
  - utterance: "insert row"
    intent: "manage table rows"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Dynamic tables and rows

앱 동적 테이블의 schema, grants, row 데이터를 관리해요. destructive DDL/DML 은 dry-run preview 와 consent 뒤에만 `--execute` 해요.

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

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "작업 확인", status: "in_progress", activeForm: "작업 고르는 중" },
     { content: "앱·테이블 resolve", status: "pending", activeForm: "대상 확인 중" },
     { content: "스키마·행 준비", status: "pending", activeForm: "입력 검증 중" },
     { content: "preview", status: "pending", activeForm: "preview 준비 중" },
     { content: "동의 받고 실행", status: "pending", activeForm: "실행 중" },
     { content: "결과 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **작업을 분기해요.** read/schema/row/grant 로 나눠요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 있어요.

2. **AskUserQuestion 기본 분기.** 비대화형 기본은 `read`예요.

   ```json
   {"questions":[{"question":"동적 테이블/데이터 작업을 골라요","header":"테이블","options":[{"label":"조회","description":"read-only 로 확인해요"},{"label":"스키마 변경","description":"테이블/컬럼을 바꿔요"},{"label":"행 변경","description":"row 데이터를 바꿔요"}]}]}
   ```

3. **read 명령.**

   ```bash
   axhub tables list --app "$APP_ID" --json
   axhub tables get "$TABLE" --app "$APP_ID" --json
   axhub tables rows "$APP_ID" "$TABLE" --json
   axhub tables grants list --app "$APP_ID" --table "$TABLE" --json
   axhub data list "$TABLE" --app "$APP_ID" --json
   axhub data count "$TABLE" --app "$APP_ID" --json
   axhub data get "$TABLE" "$ROW_ID" --app "$APP_ID" --json
   ```

4. **schema/row/grant mutation.** create 전 availability 와 column-types 를 확인하고, body JSON 은 로컬에서 먼저 검증해요.

   Consent binding 은 helper parser 와 같은 action/context 로 맞춰요: table/schema 는 `{app_id,table}` 에 컬럼·권한 키를 더하고, row 변경은 `{app_id,table,row_id?,source}` 와 payload identity 를 같이 써요. `--body` 는 `{source:"body",body_digest:"sha256:..."}`, `--body-file` 은 `{source:"body_file",body_file:"row.json",body_digest:"sha256:..."}`, `--batch` 는 `{source:"batch",batch:"rows.jsonl",batch_digest:"sha256:..."}` 로 mint 해요.

   ```bash
   ROW_DIGEST="sha256:$(printf '%s' "$ROW_JSON" | shasum -a 256 | awk '{print $1}')"
   BATCH_DIGEST="sha256:$(shasum -a 256 rows.jsonl | awk '{print $1}')"
   axhub tables check-availability "$TABLE" --app "$APP_ID" --json
   axhub tables column-types --app "$APP_ID" --json
   axhub tables create "$TABLE" --app "$APP_ID" --column 'title:text' --owner-column owner_id --execute --json
   axhub tables drop "$TABLE" --app "$APP_ID" --confirm "$TABLE" --execute --json
   axhub tables columns add "$TABLE" --app "$APP_ID" --name "$COL" --type text --nullable --execute --json
   axhub tables columns remove "$TABLE" --app "$APP_ID" --name "$COL" --execute --json
   axhub tables grants issue "$TABLE" --app "$APP_ID" --principal-id "$PRINCIPAL_ID" --principal-type user --actions read,write --execute --json
   axhub tables grants revoke --app "$APP_ID" --table "$TABLE" --grant-id "$GRANT" --execute --json
   axhub data insert "$TABLE" --app "$APP_ID" --body "$ROW_JSON" --execute --json
   axhub data insert "$TABLE" --app "$APP_ID" --batch rows.jsonl --execute --json
   axhub data update "$TABLE" "$ROW_ID" --app "$APP_ID" --body "$ROW_JSON" --execute --json
   axhub data delete "$TABLE" "$ROW_ID" --app "$APP_ID" --execute --json
   ```


   Registry-backed confirmation questions used by mutation previews:

   - `이 테이블 스키마를 변경할까요?`
   - `이 행 데이터를 변경할까요?`
   - `이 테이블 권한을 변경할까요?`

## NEVER

- NEVER SQL/connector live read 를 이 skill 로 처리하지 않아요. `data` skill 로 넘겨요.
- NEVER drop/remove/delete/revoke 를 비대화형에서 자동 실행하지 않아요.
- NEVER secret-looking row payload 를 chat/log 에 그대로 보여주지 않아요.
