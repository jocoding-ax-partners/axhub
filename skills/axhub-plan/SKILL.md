---
name: axhub-plan
description: '이 스킬은 내부 AXHub planning workflow surface 입니다. Plain Desktop chat must be handled by prompt-route without showing this skill badge; use this only for explicit slash/internal invocation.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "/axhub:axhub-plan"
    intent: "explicit axhub planning workflow"
  - utterance: "internal axhub planning workflow"
    intent: "explicit axhub planning workflow"
  - utterance: "명시적으로 axhub plan workflow 실행"
    intent: "explicit axhub planning workflow"
  - utterance: "run axhub-plan explicitly"
    intent: "explicit axhub planning workflow"
  - utterance: "내부 plan state 갱신 절차"
    intent: "explicit axhub planning workflow"
---

# axhub-plan

큰 구조 변경 전에 impact 와 단계 계획을 짧게 고정해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop natural-language contract:** for direct planning prompts like `큰 구조 변경 계획 세워줘`, start with exactly `변경 계획을 잡아볼게요.` Do not mention `axhub-plan`, `using-axhub-quality`, slash commands, route labels, quality auto-mode, missing TodoWrite, preflight internals, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, or English tool-title fragments. Use Korean Bash titles such as `계획 범위 확인` and `영향 범위 확인`, and do not implement during this planning workflow. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요.`처럼 상태만 말해요.

To plan architecture changes:

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

`auth_ok` 가 true 면 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요.`처럼 상태만 말해요. `auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `axhub_bin_invalid` 는 `AXHUB_BIN` 환경변수가 잘못된 경로 (`cli_resolved_path` 값) 를 가리키는 상태라 재설치 대신 `unset AXHUB_BIN` 후 새 세션 재시도 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "변경 scope 확인", status: "in_progress", activeForm: "진행 중" },
     { content: "gitnexus impact 분석", status: "pending", activeForm: "진행 중" },
     { content: "3-5단계 계획 작성", status: "pending", activeForm: "진행 중" },
     { content: "승인 후 handoff", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **Scope 확인.** 50+ files, 새 module boundary, public API 변경이면 architectural change 로 표시해요.

2. **Impact 분석.** AGENTS.md 규칙에 따라 GitNexus upstream/downstream blast radius 를 확인해요.

3. **계획 작성.** 3-5 단계로 나누고 각 단계의 validation command 를 붙여요.

Registry keys: plan.step-approve.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **승인 handoff.** AskUserQuestion 으로 계획 유지 / 축소 / 재작성 중 선택해요. 이 SKILL 은 구현을 바로 시작하지 않아요.

## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
