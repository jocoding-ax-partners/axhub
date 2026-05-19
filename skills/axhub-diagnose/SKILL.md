---
name: axhub-diagnose
description: '이 스킬은 사용자가 vibe coding 중 발생한 deploy 또는 test 실패를 5-Phase loop 로 자동 진단 + 자가 복구하고 싶어할 때 사용해요. 다음 표현에서 활성화: "loop 돌려서 찾아줘", "loop 돌려서 원인 찾아", "auto diagnose", "auto-diagnose", "loop-first diagnose", "self-repair loop", "auto repair", "diagnose loop", "5-phase loop", "matt pocock loop", "느낌 loop", "재현 가능한 loop 만들어", "deterministic loop". `trace` 가 단일 진단 1 화면 요약이면, axhub-diagnose 는 build → reproduce → hypothesize → instrument → fix → loop_verify → postmortem 의 5-Phase 반복 + LOOP_VERIFY 회귀 + recurrence detect 까지 한 번에 처리해요.'
examples:
  - utterance: "loop 돌려서 원인 찾아"
    intent: "auto-diagnose failure"
  - utterance: "재현 가능한 loop 만들어줘"
    intent: "auto-diagnose failure"
  - utterance: "5-phase loop 돌려"
    intent: "auto-diagnose failure"
  - utterance: "auto diagnose"
    intent: "auto-diagnose failure"
  - utterance: "loop-first diagnose"
    intent: "auto-diagnose failure"
  - utterance: "run the diagnose loop"
    intent: "auto-diagnose failure"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Axhub Diagnose

vibe coder 의 deploy / test 실패를 5-Phase Matt Pocock loop 로 자동 진단해요. 사용자는 명령어 한 줄도 보지 않아요 — y/n 과 paste 만으로 진행돼요.

## Workflow

To diagnose a failure:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 워크플로 시작 전 자동 실행돼요.

0. **Render TodoWrite checklist.**

   ```typescript
   TodoWrite({ todos: [
     { content: "실패 신호 잡는 중",       status: "in_progress", activeForm: "신호 찾는 중" },
     { content: "원인 가설 정리하기",      status: "pending",     activeForm: "가설 정리 중" },
     { content: "가설 검증하기",          status: "pending",     activeForm: "검증 중" },
     { content: "복구 시도하기",          status: "pending",     activeForm: "복구 중" },
     { content: "결과 정리해서 알려드리기", status: "pending",     activeForm: "마무리 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **Phase 1L — 실패 신호 잡기.** event_log + recovery_scan 으로 직전 실패의 deterministic 재현 시도. 잡히면 Phase 2R 로. 잡히지 않으면 HITL fallback (axhub-helpers diagnose hitl 서브명령) 으로 사용자에게 물어요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조.

2. **Phase 2R — 가설 줄세우기.** catalog + template 으로 3-5 falsifiable hypothesis 생성. 각 가설 If-X-then-Y prediction + falsifier 포함. 사용자에게 선택지 제공:

   ```json
   {
     "questions": [{
       "question": "어떤 가설부터 검증해볼까요?",
       "header": "가설 선택",
       "multiSelect": false,
       "options": [
         {"label": "가설 A 부터 검증", "description": "가장 가능성 높은 원인부터 시도해요"},
         {"label": "다른 가설을 먼저", "description": "list 를 다시 정렬해드려요"},
         {"label": "이 list 에 없는 원인 같아", "description": "추가 정보 캡처할게요 (HITL)"},
         {"label": "검증 건너뛰고 일단 fix 시도", "description": "위험할 수 있지만 빠르게 진행해요"}
       ]
     }]
   }
   ```

3. **Phase 3I — 가설 검증하기.** EnvVarProbe 또는 LoopShadowProbe 로 single-variable 검증. 각 probe 의 apply / revert 는 audit ledger 에 manifest 남겨요. boundary guard 가 touches() 외 파일 변경 감지하면 ProbeBoundaryViolation 으로 거절해요.

4. **Phase 4F — 복구 시도 + LOOP_VERIFY.** 정답 가설의 fix action 을 skills/recover wrapper 로 적용. 직후 Phase 1L loop 재실행 (LOOP_VERIFY). green 이면 Phase 5P 로, red 면 다음 가설로 회귀.

5. **Phase 5P — 정리 + 학습 저장.**
   - audit ledger 의 probe manifest 따라 정확한 cleanup (grep 없음).
   - learnings.jsonl 에 (error_class, cwd_hash, winning_hypothesis, fix_action) 한 줄 emit.
   - 같은 error_class + cwd_hash 가 3 회 (default, AXHUB_RECURRENCE_THRESHOLD override 가능) 누적되면 architectural finding emit + ARCH_HANDOFF 로 종료.

## NEVER

- NEVER grep 으로 사용자 코드에서 debug line 지우기 (probe manifest 만 truth).
- NEVER LOOP_VERIFY 우회 — fix 적용 직후 반드시 loop 재실행.
- NEVER 가설 검증 없이 첫 후보 fix 바로 적용 (사용자가 "검증 건너뛰기" 명시 선택한 경우 제외).
- NEVER session_id="unknown" 환경에서 AllowSession / AllowAlways consent grant 발급.

## Additional Resources

- `references/matt-pocock-diagnose-pattern.md` — 원본 패턴 reference.
- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
