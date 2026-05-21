---
name: axhub-ship
description: This skill prepares PR and release readiness for 배포 준비, PR 만들어, 릴리즈, push gate, and ship review.
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
examples:
  - utterance: "배포 준비"
    intent: "prepare ship"
  - utterance: "PR 만들어"
    intent: "prepare pull request"
  - utterance: "릴리즈"
    intent: "prepare release"
  - utterance: "push gate"
    intent: "prepare ship"
  - utterance: "ship review"
    intent: "prepare ship"
---

# axhub-ship

review 상태를 확인하고 PR body 또는 release narrative 를 한국어로 준비해요.

## Workflow

To prepare a PR or release:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','pipe','pipe'],env});const stdoutText=String(result.stdout??'');const stderrText=String(result.stderr??'');if(stdoutText.length>0){process.stdout.write(stdoutText);}const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const cliUnavailableRegex=/\"auth_error_code\":\"cli_unavailable\"/;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(result.status!==0&&cliUnavailableRegex.test(stdoutText)){console.log(JSON.stringify({systemMessage:\"[axhub] axhub CLI 가 감지 안 돼요. `/axhub:install-cli` 로 OS 별 공식 설치 채널을 안내받거나 `/axhub:doctor` 로 진단해주세요. (SKILL 흐름은 그대로 진행할 수 있어요 — preflight 가 cli_unavailable 만 알려준 거예요.)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "변경 scope 확인", status: "in_progress", activeForm: "진행 중" },
     { content: "review 통과 여부 확인", status: "pending", activeForm: "진행 중" },
     { content: "axhub-shipper agent 위임", status: "pending", activeForm: "진행 중" },
     { content: "PR 또는 release 준비", status: "pending", activeForm: "진행 중" },
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.


1. **변경 scope 확인.** `git log --oneline main..HEAD` 와 `git diff main..HEAD --stat` 로 변경 범위를 확인해요.

2. **review 상태 확인.** `.axhub-state/quality.json` 의 `review_commit_sha` 와 HEAD 를 비교해요. 다르면 AskUserQuestion 으로 axhub-review 먼저 / skip 중 선택해요.

3. **axhub-shipper agent 위임.** commit log, diff stat, PR template 를 전달해서 Korean PR body 를 받아요.

Registry keys: ship.review-missing.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 질문 별 safe_default.

4. **PR 또는 release 준비.** `gh pr create` 나 release workflow 전 필요한 body, test plan, risk/migration 섹션을 제시해요.

5. **Terminal state update.** 마지막에 `!axhub-helpers state-update --shipped` 를 실행해요.


## NEVER

- NEVER skip the preflight evidence for this workflow.
- NEVER include AI attribution such as Generated with Claude Code or Co-Authored-By unless the user explicitly asks.
- NEVER hide uncertainty; mark confidence and next evidence clearly.
