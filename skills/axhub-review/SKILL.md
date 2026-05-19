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

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

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
