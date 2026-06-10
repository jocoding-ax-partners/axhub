---
name: axhub-diagnose
description: '이 스킬은 내부 AXHub diagnose-loop workflow surface 입니다. Plain Desktop chat must be handled by prompt-route without showing this skill badge; use this only for explicit slash/internal invocation.'
examples:
  - utterance: "/axhub:axhub-diagnose"
    intent: "explicit axhub diagnose workflow"
  - utterance: "internal axhub diagnose loop workflow"
    intent: "explicit axhub diagnose workflow"
  - utterance: "명시적으로 axhub diagnose workflow 실행"
    intent: "explicit axhub diagnose workflow"
  - utterance: "run axhub-diagnose explicitly"
    intent: "explicit axhub diagnose workflow"
  - utterance: "내부 diagnose loop state 갱신 절차"
    intent: "explicit axhub diagnose workflow"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Axhub Diagnose

vibe coder 의 deploy / test 실패를 5-Phase Matt Pocock loop 로 자동 진단해요. 사용자는 명령어 한 줄도 보지 않아요 — y/n 과 paste 만으로 진행돼요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop natural-language contract:** for direct diagnose-loop prompts like `loop 돌려서 원인 찾아줘`, start with exactly `진단 루프를 준비할게요.` Do not mention `axhub-diagnose`, `using-axhub-quality`, slash commands, route labels, quality auto-mode, preflight internals, raw question JSON, raw JSON fields, raw emails, account emails, raw user IDs, account scopes, or English tool-title fragments. If a hypothesis choice is needed, ask in normal Korean chat instead of exposing an AskUserQuestion JSON block. 로그인 확인 결과에는 계정 이메일, raw user id, scope 를 쓰지 말고 `로그인되어 있어요.`처럼 상태만 말해요.

To diagnose a failure:

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

0. **Render TodoWrite checklist.**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

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

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

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
- NEVER session_id="unknown" 환경에서 AllowSession / AllowAlways approval grant 발급.

## Additional Resources

- `references/matt-pocock-diagnose-pattern.md` — 원본 패턴 reference.
- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
