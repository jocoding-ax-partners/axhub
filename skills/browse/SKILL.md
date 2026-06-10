---
name: browse
description: '이 스킬은 사용자가 axhub 마켓플레이스의 공개 앱을 검색하거나 부트스트랩 템플릿 목록을 둘러보고 싶어할 때 사용해요. 다음 표현에서 활성화: "앱 둘러봐", "마켓 검색", "공개 앱 찾아", "앱 검색", "다른 사람 앱", "템플릿 목록", "템플릿 뭐 있어", "어떤 템플릿", "marketplace", "discover apps", "search apps", "list templates", 또는 axhub 공개 앱·템플릿 탐색 의도. 내 앱 목록은 apps 스킬, 내 리소스 인벤토리는 my-resources 스킬이에요.'
examples:
  - utterance: "앱 둘러봐"
    intent: "browse marketplace apps"
  - utterance: "마켓 검색"
    intent: "browse marketplace apps"
  - utterance: "템플릿 목록"
    intent: "browse templates"
  - utterance: "템플릿 뭐 있어?"
    intent: "browse templates"
  - utterance: "discover apps"
    intent: "browse marketplace apps"
  - utterance: "list templates"
    intent: "browse templates"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: haiku
---

# Browse marketplace apps and templates

공개 앱과 bootstrap template 을 read-only 로 둘러봐요. 내 앱은 apps, 리소스 인벤토리는 my-resources 가 담당해요.

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

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "검색 의도 확인", status: "in_progress", activeForm: "의도 확인 중" },
     { content: "read-only 조회", status: "pending", activeForm: "조회 중" },
     { content: "결과 요약", status: "pending", activeForm: "요약 중" },
     { content: "다음 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **공개 앱/템플릿을 조회해요.**

   ```bash
   axhub apps discover --q "$QUERY" --category "$CATEGORY" --sort "$SORT" --limit 20 --json
   axhub apps search "$QUERY" --category "$CATEGORY" --sort "$SORT" --visibility public --json
   axhub apps templates list --json
   ```

2. **결과를 짧게 보여줘요.** top 10 과 다음 검색 힌트만 보여줘요.

3. **연결 분기.** 앱 생성은 init/apps, 공개 심사 제출은 publish, 내 앱 목록은 apps 로 넘겨요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 이 read-only skill 은 질문 없이 안전하게 조회만 해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 빈 metadata entry 를 참조해요.

## NEVER

- NEVER 공개 앱/템플릿 탐색을 `apps detect` 로 처리하지 않아요. `apps detect` 는 migrate 스킬의 read-only repo 감지 전용이에요.
- NEVER 공개 앱 탐색을 내 앱 목록처럼 설명하지 않아요.
- NEVER search 결과로 mutation 을 자동 실행하지 않아요.
