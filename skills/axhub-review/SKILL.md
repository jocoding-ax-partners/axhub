---
name: axhub-review
description: This skill reviews axhub code for 리뷰해줘, 코드 봐줘, PR 검토, megaskill review, and quality gate findings.
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "리뷰해줘"
    intent: "review code"
  - utterance: "코드 봐줘"
    intent: "review code"
  - utterance: "PR 검토"
    intent: "review pull request"
  - utterance: "review this diff"
    intent: "review code"
  - utterance: "quality gate findings"
    intent: "review quality gate"
---

# axhub-review

변경 diff 를 한국어 해요체로 검토하고 마지막에 review state 를 갱신해요.

## Workflow

To review code quality:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/axhub/axhub/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "diff scope 확인", status: "in_progress", activeForm: "진행 중" },
     { content: "axhub-reviewer agent 위임", status: "pending", activeForm: "진행 중" },
     { content: "Korean review 결과 정리", status: "pending", activeForm: "진행 중" },
     { content: "state-update review acknowledged", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **Diff scope 확인.** `git diff HEAD --stat` 기준으로 파일 수와 줄 수를 확인해요. 100 files 또는 1000 lines 이상이면 AskUserQuestion 으로 전체 review / 핵심 파일 / scope 축소 중 선택해요.

2. **axhub-reviewer agent 위임.** diff content, repo context, AGENTS.md 규칙을 전달해서 bug, performance, security, style finding 을 받아요.

3. **결과 정리.** 🔴 P0, 🟡 P1, 🟢 P2 로 나누고 파일:줄, 설명, 권장 fix 를 한국어 해요체로 보여줘요.

Registry keys: review.scope-confirm, review.fix-now-suggest.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **Fix-now 제안.** Finding 이 있으면 AskUserQuestion 으로 지금 고칠지, review-only 로 끝낼지 고르게 해요.

5. **Terminal state update.** 마지막에 `!axhub-helpers state-update --review-acknowledged` 를 실행해요.

## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
