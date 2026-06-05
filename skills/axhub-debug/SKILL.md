---
name: axhub-debug
description: '이 스킬은 내부 AXHub debug workflow surface 입니다. Plain Desktop chat must be handled by prompt-route without showing this skill badge; use this only for explicit slash/internal invocation.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "/axhub:axhub-debug"
    intent: "explicit axhub debug workflow"
  - utterance: "internal axhub debug workflow"
    intent: "explicit axhub debug workflow"
  - utterance: "명시적으로 axhub debug workflow 실행"
    intent: "explicit axhub debug workflow"
  - utterance: "run axhub-debug explicitly"
    intent: "explicit axhub debug workflow"
  - utterance: "내부 debug state 갱신 절차"
    intent: "explicit axhub debug workflow"
---

# axhub-debug

테스트 실패나 에러 증상을 가설-증거 방식으로 좁혀서 root cause 후보를 제시해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop natural-language contract:** for direct debug prompts like `왜 테스트가 깨지는지 디버그해줘`, start with exactly `원인을 좁혀볼게요.` Do not mention `axhub-debug`, `using-axhub-quality`, slash commands, route labels, quality auto-mode, missing TodoWrite, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, or English tool-title fragments. Use Korean Bash titles such as `문제 신호 확인`, `최근 실패 확인`, and `디버그 상태 저장`. In Claude Desktop, debug directly in the current session first; do not use Task/Subagent/agent delegation, agent tool calls, or visible `디버그 위임` unless the user explicitly asks for a separate agent. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요.`처럼 상태만 말해요.

To debug failures:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
SAFE_PREFLIGHT_JSON=$(printf '%s' "$PREFLIGHT_JSON" | jq 'del(.user_email, .user_id, .email, .account_email, .scope, .scopes)' 2>/dev/null)
[ -n "$SAFE_PREFLIGHT_JSON" ] || SAFE_PREFLIGHT_JSON='{"auth_ok":false,"auth_error_code":"preflight_summary_unavailable"}'
echo "$SAFE_PREFLIGHT_JSON"
```

`auth_ok` 가 true 면 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요.`처럼 상태만 말해요. `auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "증상과 로그 수집", status: "in_progress", activeForm: "진행 중" },
     { content: "원인 가설 직접 검토", status: "pending", activeForm: "진행 중" },
     { content: "root cause 후보 정리", status: "pending", activeForm: "진행 중" },
     { content: "debug state 갱신", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **증상 수집.** 실패 command, stack trace, 최근 test output, `git log --oneline -10` 을 확인해요.

2. **불명확한 증상 확인.** 재현 step 이 없으면 AskUserQuestion 으로 재현 상황을 한 번만 물어요.

3. **원인 가설 직접 검토.** 현재 세션에서 3-5개 가설, 각 가설의 증거, 다음 probe, confidence 를 정리해요. Claude Desktop 에서는 Task/Subagent/agent 위임을 기본으로 쓰지 않아요.

Registry keys: debug.context-needed.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **결과 제시.** 🎯 root cause 후보, 🔧 권장 fix, 🔍 추가 probe 를 한국어 해요체로 정리해요.

5. **Terminal state update.** 마지막에 `!axhub-helpers state-update --debug-acknowledged` 를 실행해요.

## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
