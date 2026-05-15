---
name: axhub-debug
description: This skill analyzes root cause for 디버그해, 왜 안 돼, 에러 원인, failed tests, and regression traces.
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "디버그해"
    intent: "debug failure"
  - utterance: "왜 안 돼"
    intent: "debug failure"
  - utterance: "에러 원인"
    intent: "debug root cause"
  - utterance: "why is this failing"
    intent: "debug failure"
  - utterance: "trace this regression"
    intent: "debug regression"
---

# axhub-debug

테스트 실패나 에러 증상을 가설-증거 방식으로 좁혀서 root cause 후보를 제시해요.

## Workflow

To debug failures:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "증상과 로그 수집", status: "in_progress", activeForm: "진행 중" },
     { content: "axhub-debugger agent 위임", status: "pending", activeForm: "진행 중" },
     { content: "root cause 후보 정리", status: "pending", activeForm: "진행 중" },
     { content: "debug state 갱신", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.


1. **증상 수집.** 실패 command, stack trace, 최근 test output, `git log --oneline -10` 을 확인해요.

2. **불명확한 증상 확인.** 재현 step 이 없으면 AskUserQuestion 으로 재현 상황을 한 번만 물어요.

3. **axhub-debugger agent 위임.** 3-5개 가설, 각 가설의 증거, 다음 probe, confidence 를 요청해요.

Registry keys: debug.context-needed.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **결과 제시.** 🎯 root cause 후보, 🔧 권장 fix, 🔍 추가 probe 를 한국어 해요체로 정리해요.

5. **Terminal state update.** 마지막에 `!axhub-helpers state-update --debug-acknowledged` 를 실행해요.


## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
