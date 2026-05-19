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

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

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
