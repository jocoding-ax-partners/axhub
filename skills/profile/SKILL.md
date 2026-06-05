---
name: profile
description: '이 스킬은 사용자가 axhub profile, endpoint, 회사 환경, 현재 프로필을 보거나 바꾸고 싶어할 때 사용해요. 다음 표현에서 활성화: "다른 회사", "다른 회사로 바꿔", "사내 endpoint", "엔드포인트 바꿔", "프로필", "프로필 사용", "프로필 추가", "회사 endpoint", "회사 endpoint 바꿔", "endpoint 바꿔", "endpoint 변경", "profile", "profile current", "profile list", 또는 axhub profile 관리 의도. endpoint allowlist 를 확인하고 위험한 endpoint 는 명시 확인해요.'
examples:
  - utterance: "다른 회사"
    intent: "manage axhub profile"
  - utterance: "다른 회사로 바꿔"
    intent: "manage axhub profile"
  - utterance: "profile"
    intent: "manage axhub profile"
  - utterance: "profile current"
    intent: "manage axhub profile"
  - utterance: "사내 endpoint"
    intent: "manage axhub profile"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Profile

axhub profile 조회, 추가, 전환을 다뤄요. endpoint 변경은 보안 경계라서 allowlist 확인과 명시 confirm 을 요구해요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To manage profiles:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "프로필 의도 확인", status: "in_progress", activeForm: "프로필 의도 보는 중" },
     { content: "현재 프로필 보기", status: "pending", activeForm: "현재 상태 보는 중" },
     { content: "필요 시 추가하거나 전환", status: "pending", activeForm: "프로필 처리하는 중" },
     { content: "서버 주소 안전 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **작업을 분기해요.** 명확한 `current` 또는 `list` 의도는 질문 없이 read-only 로 가요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `현재 프로필 보기` 예요.

2. **필요하면 작업을 고르게 해요.**

   ```json
   {
     "question": "프로필 작업을 고를까요?",
     "header": "프로필",
     "options": [
       {"label": "현재 프로필 보기", "value": "current", "description": "지금 연결된 서버 주소와 프로필을 봐요"},
       {"label": "목록 보기", "value": "list", "description": "저장된 프로필 목록을 봐요"},
       {"label": "추가 또는 전환", "value": "mutate", "description": "동의와 서버 주소 확인이 필요해요"}
     ]
   }
   ```

3. **read-only 명령을 실행해요.**

   ```bash
   axhub profile current --json
   axhub profile list --json
   ```

4. **add 는 endpoint 를 확인한 뒤 실행해요.**

   ```bash
   axhub profile add "$NAME" --endpoint "$ENDPOINT" --json
   ```

   allowlist 밖 endpoint 면 위험을 설명하고 exact confirm 을 받아요. `consent-mint` 에 `action=profile_add`, `context={profile,endpoint}` 를 넣어요.

5. **use 는 consent 후 실행해요.**

   ```bash
   axhub profile use "$NAME" --json
   ```

   `consent-mint` 에 `action=profile_use`, `context={profile}` 를 넣어요.

## NEVER

- NEVER endpoint 를 조용히 바꾸지 않아요.
- NEVER allowlist 밖 endpoint 를 정상 production endpoint 처럼 말하지 않아요.
- NEVER profile 전환을 subprocess 에서 자동 실행하지 않아요.
- NEVER auth token 값을 출력하지 않아요.
