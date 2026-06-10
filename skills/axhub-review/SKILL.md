---
name: axhub-review
description: '이 스킬은 내부 AXHub review workflow surface 입니다. Plain Desktop chat must be handled by prompt-route without showing this skill badge; use this only for explicit slash/internal invocation.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "/axhub:axhub-review"
    intent: "explicit axhub review workflow"
  - utterance: "internal axhub review workflow"
    intent: "explicit axhub review workflow"
  - utterance: "명시적으로 axhub review workflow 실행"
    intent: "explicit axhub review workflow"
  - utterance: "run axhub-review explicitly"
    intent: "explicit axhub review workflow"
  - utterance: "내부 review state 갱신 절차"
    intent: "explicit axhub review workflow"
---

# axhub-review

변경 diff 를 한국어 해요체로 검토하고 마지막에 review state 를 갱신해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop natural-language contract:** for direct review prompts like `이 코드 리뷰해줘`, start with `코드 리뷰를 시작할게요.` Then run a Korean-titled scope check: `axhub-helpers review-scope-summary --user-utterance "<latest user sentence>"` with title `변경 범위 확인`. Use that summary to decide whether to ask for scope narrowing or read the actual changed source/config files. Use Korean Bash titles such as `변경 범위 확인` and `리뷰 상태 저장`. In Claude Desktop, review directly in the current session first; use a separate agent only when the user asks for one. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요. 변경 범위 확인할게요.`처럼 상태만 말해요.

To review code quality:

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

`auth_ok` 가 true 면 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요. 변경 범위 확인할게요.`처럼 상태만 말해요. `auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `axhub_bin_invalid` 는 `AXHUB_BIN` 환경변수가 잘못된 경로 (`cli_resolved_path` 값) 를 가리키는 상태라 재설치 대신 `unset AXHUB_BIN` 후 새 세션 재시도 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "diff scope 확인", status: "in_progress", activeForm: "진행 중" },
     { content: "주요 변경 직접 검토", status: "pending", activeForm: "진행 중" },
     { content: "한국어 리뷰 결과 정리", status: "pending", activeForm: "진행 중" },
     { content: "state-update review acknowledged", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **Diff scope 확인.** 먼저 `git diff HEAD --stat -- . ':!desktop-pure-routing-results.md' ':!desktop-*routing-results.md' ':!.axhub-state/**' ':!.omx/**' ':!*test-results*' ':!*.log'` 기준으로 리뷰 대상 파일 수와 줄 수를 확인해요. 이 제외 패턴은 QA 산출물, 상태 파일, 로그를 리뷰 대상에서 빼기 위한 것이며 사용자에게 설명하지 않아요. 100 files 또는 1000 lines 이상이면 파일을 읽기 전에 일반 채팅으로 `변경량이 커서 먼저 범위를 정할게요. 전체를 볼까요, 핵심 파일만 볼까요?`라고 묻고 멈춰요. Claude Desktop 에서는 AskUserQuestion/Question 도구를 쓰지 않아요.

2. **주요 변경 직접 검토.** 현재 세션에서 diff content, repo context, AGENTS.md 규칙을 직접 확인해서 bug, performance, security, style finding 을 정리해요. Claude Desktop 에서는 Task/Subagent/agent 위임을 기본으로 쓰지 않아요. QA 산출물이나 상태 파일을 먼저 읽지 말고, 필터링된 scope 에 포함된 실제 소스/설정 변경만 읽어요. 100 files 또는 1000 lines 이상의 큰 diff 는 먼저 자연어로 범위를 좁힐지 묻고 멈춰요. 사용자가 명시적으로 별도 agent review 를 원한다고 한 경우에만 agent 위임을 고려해요.

3. **결과 정리.** 🔴 P0, 🟡 P1, 🟢 P2 로 나누고 파일:줄, 설명, 권장 fix 를 한국어 해요체로 보여줘요.

Registry keys: review.scope-confirm, review.fix-now-suggest.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **Fix-now 제안.** Finding 이 있으면 AskUserQuestion 으로 지금 고칠지, review-only 로 끝낼지 고르게 해요.

5. **Terminal state update.** 마지막에 `!axhub-helpers state-update --review-acknowledged` 를 실행해요.

## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
