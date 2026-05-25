---
name: data
description: '이 스킬은 axhub data catalog search, resource describe, safe SQL read, and snippet generation workflow 에 사용해요. 다음 표현에서 활성화: "데이터 조회해줘", "catalog search", "테이블 설명", "SQL로 읽어줘", "snippet 만들어줘", "describe table", "generate snippet".'
examples:
  - utterance: "orders 데이터 조회해줘"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "이 테이블 읽는 python snippet 만들어줘"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "describe snowflake analytics orders table"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "generate a TypeScript snippet for this catalog resource"
    intent: "query axhub data catalog and generate safe read snippets"
  - utterance: "SQL로 읽어줘"
    intent: "query axhub data catalog and generate safe read snippets"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Data

axhub catalog 를 CLI-only 방식으로 탐색하고, first live read consent 뒤에 read-only invoke 또는 snippet 을 만들어줘요.

## Workflow

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "catalog context 동기화", status: "in_progress", activeForm: "catalog context 동기화 중" },
     { content: "resource 검색과 describe", status: "pending", activeForm: "resource 확인 중" },
     { content: "first live read consent 확인", status: "pending", activeForm: "live read 안전 확인 중" },
     { content: "read invoke 또는 snippet 생성", status: "pending", activeForm: "결과 생성 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **Sync local catalog snapshot.** Use the helper, not MCP server config. Default output is the git toplevel `.axhub/`; use `--out` only when the user gives a separate workspace.

   ```bash
   axhub-helpers sync --target auto --json
   axhub-helpers sync --target local-python --json
   ```

   If sync returns `ambiguous_target`, choose the closest runtime from project evidence. If it returns `identity_changed`, stop before overwrite unless the user explicitly confirms the new principal; non-interactive safe default is skip overwrite.

2. **Search and describe resources before any live read.** Prefer a broad catalog search, then a precise describe. Keep connector/path from catalog output only.

   ```bash
   axhub catalog search --json --limit 200
   axhub catalog get --connector <connector> --path <path> --json
   ```

   Summarize only what is needed: connector, path, kind, allowed_columns, masked columns, row policy, and deny_reason when present. Do not print full `.axhub/catalog.json`.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 각 질문 별 safe_default.

3. **Confirm first live read.** Before the first live read for a resource in this session, show connector, path, SQL, row limit, allowed_columns, masked fields, and why the read is needed. Ask once.

   ```json
   {
     "questions": [{
       "question": "실데이터 read 를 실행할까요?",
       "header": "Live read",
       "multiSelect": false,
       "options": [
         {"label": "Dry-run only", "description": "실데이터를 읽지 않고 SQL, allowed_columns, snippet 만 보여줘요."},
         {"label": "Run read", "description": "표시한 connector/path/SQL/row limit 그대로 한 번만 실행해요."}
       ]
     }]
   }
   ```

   In non-interactive mode, use `Dry-run only`. If the server denies with `allowed:false` or `deny_reason`, show that reason and NEVER retry denied.

4. **Invoke read safely when consent exists.** Live reads must include `--execute --json`, an explicit row limit, and read-only SQL. Keep row limit small unless the user explicitly asks for less-restricted output.

   ```bash
   axhub catalog invoke --connector <connector> --path <path> --action read --sql '<SELECT ...>' --row-limit 100 --execute --json
   ```

   Parse error output through the generated catalog empathy copy. For catalog internal errors, do not retry automatically; re-check `allowed_columns` with `catalog get` first.

5. **Generate snippets from described catalog context.** Use helper templates so auth posture stays target-aware.

   ```bash
   axhub-helpers snippet --mode A --language typescript --target web-axhub --connector <connector> --path <path> --sql '<SELECT ...>' --allowed-columns <csv> --masked <csv>
   axhub-helpers snippet --mode B --language python --target local-python --connector <connector> --path <path> --sql '<SELECT ...>' --allowed-columns <csv> --masked <csv>
   axhub-helpers snippet --mode B --language shell --target local-bash --connector <connector> --path <path> --sql '<SELECT ...>' --allowed-columns <csv>
   ```

   Mode A uses browser cookie auth with `credentials: 'include'`. Mode B uses `AXHUB_PAT` as `X-Api-Key`. Local bash uses `axhub catalog invoke --execute --json` via CLI/keychain and does not print PATs.

6. **Final response.** Return the selected connector/path, row limit, allowed_columns, masked handling, whether live read ran, and exact commands or snippet produced. If no live read ran, say dry-run only.

## Identity-change question

Use this only after `axhub-helpers sync` returns `identity_changed`.

```json
{
  "questions": [{
    "question": "인증 주체가 바뀌었어요. catalog 를 새로 쓸까요?",
    "header": "Identity",
    "multiSelect": false,
    "options": [
      {"label": "Skip overwrite", "description": "기존 catalog 를 보존하고 새 주체 확인을 요청해요."},
      {"label": "Overwrite", "description": "명시 동의가 있을 때만 --allow-identity-change 로 새로 써요."}
    ]
  }]
}
```

## NEVER

- NEVER governance bypass: do not invent policies, scopes, row access, or masked output.
- NEVER path guessing: use connector/path from `catalog search` or `catalog get` only.
- NEVER retry denied: if `allowed:false`, `deny_reason`, 66, or catalog internal error appears, stop and show the reason.
- NEVER run a live read without first live read consent in the current session/resource.
- NEVER omit `--execute --json` for a live `catalog invoke`.
- NEVER exceed the stated row limit or `allowed_columns`.
- NEVER print `.axhub/catalog.json` or hardcode PATs in snippets.

## Additional Resources

- `../deploy/references/error-empathy-catalog.generated.md` — generated exit-code copy from `crates/axhub-helpers/data/catalog.json`.
