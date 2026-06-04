---
name: workspace
description: '이 스킬은 사용자가 "내가 속한 팀", "내 팀", "내 소속", "내 워크스페이스"처럼 자신의 axhub 팀/워크스페이스/테넌트 membership 목록을 조회하고 싶어할 때 사용해요. 다음 표현에서 활성화: "내가 속한 팀이랑 워크스페이스 보여줘", "내가 속한 팀 보여줘", "내가 속한 워크스페이스", "내 팀이랑 워크스페이스", "내 워크스페이스", "내 워크스페이스 보여줘", "내 워크스페이스 목록", "내 소속", "내 소속 봐", "어느 워크스페이스", "워크스페이스", "워크스페이스 목록", "테넌트", "테넌트 목록", "workspace", "tenant", "my teams and workspaces", "my workspaces". 팀 멤버 초대/권한 변경 같은 관리는 team 스킬, 백엔드 endpoint/profile 전환은 profile 스킬이 담당해요.'
examples:
  - utterance: "내 워크스페이스 보여줘"
    intent: "list axhub workspaces"
  - utterance: "테넌트 목록"
    intent: "list axhub workspaces"
  - utterance: "내 소속 봐"
    intent: "list axhub workspaces"
  - utterance: "내가 속한 팀이랑 워크스페이스 보여줘"
    intent: "list axhub workspaces"
  - utterance: "workspace list"
    intent: "list axhub workspaces"
  - utterance: "my workspaces"
    intent: "list axhub workspaces"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: haiku
---

# Workspace and tenants

현재 사용자의 워크스페이스/테넌트 소속을 read-only 로 보여줘요. endpoint/profile 전환은 profile skill 로 넘겨요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

**Tenant grounding.** 조회 대상 tenant 는 사용자가 고른 `AXHUB_TENANT` 또는 preflight 의 active team 이에요. active team 이 없으면 `tenants list --all` 로 후보만 보여주고 특정 tenant 의 비공개 상세는 추측하지 않아요.

```bash
TENANT="${AXHUB_TENANT:-$(printf '%s
' "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')}"
```

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "소속 확인", status: "in_progress", activeForm: "소속 확인 중" },
     { content: "목록 조회", status: "pending", activeForm: "목록 조회 중" },
     { content: "상세 요약", status: "pending", activeForm: "상세 요약 중" },
     { content: "다음 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **테넌트 소속을 조회해요.**

   ```bash
   if [ -n "$TENANT" ]; then
     axhub tenants whoami --tenant "$TENANT" --json
     axhub tenants get "$TENANT" --json
   else
     axhub tenants whoami --json
   fi
   axhub tenants list --all --json
   ```

2. **현재/후보 workspace 를 요약해요.** slug/id/name/role 만 보여줘요.

3. **전환 요청은 profile skill 로 넘겨요.** 이 skill 은 조회만 담당해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 이 read-only skill 은 질문 없이 안전하게 조회만 해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 빈 metadata entry 를 참조해요.

## NEVER

- NEVER `tenants create/update/delete/icon` 을 vibe skill 로 실행하지 않아요.
- NEVER endpoint/profile 을 여기서 바꾸지 않아요.
- NEVER 다른 tenant 의 비공개 정보를 추측해서 보여주지 않아요.
