---
name: axhub-tdd
description: This skill guides TDD cycles for TDD 로 가, 테스트 먼저, RED GREEN, failing tests first, and refactor safely.
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "TDD 로 가"
    intent: "guide tdd"
  - utterance: "테스트 먼저"
    intent: "write test first"
  - utterance: "RED GREEN"
    intent: "guide tdd"
  - utterance: "write the failing test first"
    intent: "write test first"
  - utterance: "refactor safely"
    intent: "guide refactor"
---

# axhub-tdd

RED → GREEN → REFACTOR 흐름을 지키도록 돕는 quality SKILL 이에요.

## Workflow

To run a TDD cycle:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "target function 또는 module 선택", status: "in_progress", activeForm: "진행 중" },
     { content: "RED 실패 테스트 작성", status: "pending", activeForm: "진행 중" },
     { content: "GREEN 최소 구현", status: "pending", activeForm: "진행 중" },
     { content: "REFACTOR review", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.


1. **Target 선택.** 변경할 함수나 module 을 고르고 expected behavior 를 한 문장으로 고정해요.

2. **RED.** 먼저 실패하는 테스트를 작성하고 실제로 실패하는지 확인해요.

3. **GREEN.** 테스트를 통과시키는 최소 구현만 해요.

Registry keys: tdd.target-confirm.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **REFACTOR.** axhub-reviewer 관점으로 cleanup 을 확인하고 테스트가 계속 통과하는지 다시 확인해요.


## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
