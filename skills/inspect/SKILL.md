---
name: inspect
description: '이 스킬은 사용자가 로컬 axhub.yaml 매니페스트, 현재 CLI 설정, 일반 axhub 상태, 배포 설명·에러 코드를 검증·확인하고 싶어할 때 사용해요. 다음 표현에서 활성화: "매니페스트 확인", "axhub.yaml 검증", "설정 확인", "config 봐", "현재 endpoint 뭐", "CLI 상태", "axhub 상태", "status", "what's the status", "deploy explain", "배포 코드", "manifest validate", "check config", 또는 axhub 매니페스트·설정 조회 의도.'
examples:
  - utterance: "axhub.yaml 검증"
    intent: "inspect axhub configuration"
  - utterance: "매니페스트 확인"
    intent: "inspect axhub configuration"
  - utterance: "CLI 상태 봐"
    intent: "inspect axhub status"
  - utterance: "status"
    intent: "inspect axhub status"
  - utterance: "deploy explain"
    intent: "inspect deploy diagnostics"
  - utterance: "배포 코드 알려줘"
    intent: "inspect deploy codes"
  - utterance: "check config"
    intent: "inspect axhub configuration"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Inspect axhub state

매니페스트, CLI 설정, 일반 상태, 배포 진단을 read-only 로 확인해요. `manifest check --baseline` 은 v0.17.3 에서 성공 경로가 없어 쓰지 않아요.

## Workflow

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "확인 대상 고르기", status: "in_progress", activeForm: "대상 고르는 중" },
     { content: "read-only 명령 실행", status: "pending", activeForm: "조회 중" },
     { content: "결과 요약", status: "pending", activeForm: "요약 중" },
     { content: "다음 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 마지막 결과 출력 직후 TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.**

1. **대상을 고르고 read-only 명령만 실행해요.**

   ```bash
   axhub manifest validate --file axhub.yaml --json
   axhub config explain --json
   axhub status --json
   axhub deploy doctor --app "$APP_ID" --json
   axhub deploy explain --app "$APP_ID" --json
   axhub deploy codes --json
   ```

2. **결과를 한국어로 요약해요.** secret 이 redacted 된 설정만 보여줘요.

3. **충돌 분기.** 사용자가 “배포 상태”를 물으면 deploy status 는 `status` skill 로 넘기고, 일반 CLI 상태는 `axhub status --json` 으로 처리해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 이 read-only skill 은 질문 없이 안전하게 조회만 해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 빈 metadata entry 를 참조해요.

## NEVER

- NEVER `axhub manifest check --baseline` 을 실행하지 않아요.
- NEVER 설정 secret 을 복원하거나 추측하지 않아요.
- NEVER read-only 진단 결과를 mutation 성공처럼 말하지 않아요.
