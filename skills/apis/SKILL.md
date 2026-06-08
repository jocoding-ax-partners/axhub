---
name: apis
description: '이 스킬은 사용자가 axhub 앱에서 사용할 수 있는 API·endpoint·service catalog 를 보고 싶거나, 등록된 API endpoint 를 호출하고 싶어할 때 사용해요. 다음 표현에서 활성화: "API 뭐 있어", "어떤 API 있어", "API 목록", "API 카탈로그", "쓸 수 있는 API", "엔드포인트 뭐 있어", "endpoint list", "available endpoints", "list apis", "service catalog". API 목록은 read-only 이고, API call 은 preview 와 명시 확인 뒤에만 실행해요.'
examples:
  - utterance: "axhub 앱이 어떤 API 쓸 수 있는지 보여줘"
    intent: "list available API endpoints"
  - utterance: "API 목록 보여줘"
    intent: "list available API endpoints"
  - utterance: "available endpoints"
    intent: "list available API endpoints"
  - utterance: "show api catalog"
    intent: "list available API endpoints"
  - utterance: "이 API 호출해"
    intent: "call API endpoint after approval"
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# APIs

axhub 앱에서 사용할 수 있는 API/service catalog 를 최신 CLI 의 catalog resource 표면으로 보여줘요. 최신 CLI 에는 구 `axhub apis list|call` 명령이 없으므로, API 목록 의도는 `axhub catalog resources` 로 처리하고 실데이터 read/invoke 는 `data` skill 의 approval 흐름으로 넘겨요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Visible response contract for Claude Desktop:** internal JSON field names, CLI flags, and workflow stage names are parsing notes, not user copy. For natural API-list prompts such as `쓸 수 있는 API 뭐 있어?`, the first visible chat sentence must be exactly `사용 가능한 API를 확인할게요.` Do not say `preflight`, `catalog 조회`, `items 비어`, `items.length`, or `--search` in visible chat prose. Tool/Bash titles should also stay natural: use `로그인 상태 확인` for the helper auth/context check and use `사용 가능한 API 확인` for the catalog command, not `axhub preflight 인증/컨텍스트 확인` or `axhub API/service catalog 목록 조회`. If the catalog response is empty, say this kind of sentence: `현재 권한에서 바로 사용할 수 있는 API 카탈로그는 없어요.` Do not include raw email addresses, app slugs, profile IDs, team IDs, or `current_app` values in that API-list answer unless the user explicitly asked about identity, login state, team, or app context. Then give natural next steps such as `검색어를 알려주면 더 좁혀볼게요`, `커넥터 목록을 먼저 볼 수 있어요`, or `내 리소스 보여줘라고 말하면 전체 인벤토리를 볼 수 있어요`.

**Auth/context check (internal).** 워크플로를 시작하기 전에 helper 로 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 이 단락 제목과 command name 은 내부 실행 절차예요. 사용자에게는 `로그인 상태를 먼저 확인할게요.` 정도로만 자연스럽게 말하고, `preflight` 라는 단어를 보이는 진행 문구로 쓰지 않아요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요. API/service catalog 조회는 read-only 라서 `profile`, `team`, `current_app`, 또는 버전 경고가 비어 있어도 중단하지 말고 최신 CLI 의 `axhub catalog resources --json --limit 50` 까지 실행해요.

1. **API/service catalog 조회.** 사용자가 “어떤 API”, “endpoint list”, “service catalog” 처럼 목록·카탈로그를 물으면 read-only 명령만 실행해요. 특정 검색어가 있으면 `--search "$QUERY"` 를 붙이고, 없으면 기본 50개만 봐요.

   ```bash
   axhub catalog resources --json --limit 50
   axhub catalog resources --json --search "$QUERY" --limit 50
   ```

   결과는 connector, path, kind, description, allowed action/read 가능 여부 중심으로 10개 이하 표로 요약해요. 최신 CLI 응답의 source of truth 는 top-level `items` 배열이에요. 이 field name 은 내부 파싱용으로만 사용하고 사용자에게 그대로 말하지 않아요. `items.length > 0` 이면 preflight 의 `current_app`/`profile` 값과 이름이 달라도 빈 결과로 취급하거나 다시 필터링하지 말고, CLI 가 이미 권한과 스코프를 적용한 결과로 보고 반환된 resource 를 그대로 요약해요. `items` 가 비어 있을 때만 `현재 권한에서 바로 사용할 수 있는 API 카탈로그는 없어요.` 라고 자연어로 안내해요. 다른 팀 또는 권한 밖 resource 를 추측하지 않아요. `axhub apis list` 는 최신 CLI 에 없는 구 명령이므로 절대 실행하지 않아요.

2. **특정 resource 확인.** 사용자가 특정 endpoint/resource 호출을 명시하면 먼저 목록 결과에서 connector/path 를 확인해요. connector/path 가 확정되면 read-only describe 만 실행해요.

   ```bash
   axhub catalog get --connector "$CONNECTOR" --path "$PATH" --json
   ```

   실데이터 read, 집계, snippet, `catalog invoke --execute` 가 필요하면 이 skill 을 멈추고 `skills/data/SKILL.md` 를 로드해요. 최신 CLI 에는 generic `axhub apis call` 이 없으므로 API call 을 만들거나 preview 용으로 호출하지 않아요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — API call 확인 → `abort`.

3. **API call 요청은 data skill 로 넘겨요.** 사용자가 그래도 “이 API 호출해”라고 명시하면 connector/path, action, 예상 SQL 또는 body 필요 여부를 보여주고, 이 skill 에서 직접 실행하지 말지 확인해요. 호출을 원하면 `data` skill 로 이어서 first live read approval 를 받아요.

   ```json
   {
     "questions": [{
       "question": "이 API를 호출할까요?",
       "header": "API 호출",
       "multiSelect": false,
       "options": [
         {"label": "호출", "value": "call", "description": "표시한 endpoint/method/body 로 한 번 호출해요."},
         {"label": "취소", "value": "abort", "description": "호출하지 않아요."}
       ]
     }]
   }
   ```

   승인되면 `data` skill 의 `axhub catalog invoke --execute --json` 흐름으로 전환해요. 이 skill 에서는 `preview 확인` 나 live call 을 직접 실행하지 않아요.

## NEVER

- NEVER 권한 밖 API 나 다른 팀 endpoint 를 추측하지 않아요.
- NEVER 최신 CLI 에 없는 `axhub apis list` 또는 `axhub apis call` 을 실행하지 않아요.
- NEVER `axhub catalog invoke --execute` 를 이 skill 에서 직접 실행하지 않아요. live read 는 `data` skill 로 넘겨요.
- NEVER API response 에 secret-looking 값이 있으면 그대로 길게 출력하지 않아요.
- NEVER body inline 으로 secret 을 명령줄에 넣지 않아요. 파일과 digest 중심으로 다뤄요.
