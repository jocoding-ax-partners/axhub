---
name: axhub-plan
description: This skill plans architectural changes for 플랜 짜줘, 계획 세워, 큰 구조 변경, impact analysis, and staged execution.
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "플랜 짜줘"
    intent: "plan change"
  - utterance: "계획 세워"
    intent: "plan change"
  - utterance: "큰 구조 변경"
    intent: "plan architecture"
  - utterance: "impact analysis"
    intent: "plan impact"
  - utterance: "staged execution"
    intent: "plan steps"
---

# axhub-plan

큰 구조 변경 전에 impact 와 단계 계획을 짧게 고정해요.

## Workflow

To plan architecture changes:

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
