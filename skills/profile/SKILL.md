---
name: profile
description: '이 스킬은 사용자가 axhub profile, endpoint, 회사 환경, 현재 프로필을 보거나 바꾸고 싶어할 때 사용해요. 다음 표현에서 활성화: "profile current", "profile list", "회사 endpoint 바꿔", "다른 회사로 바꿔", "endpoint 변경", "사내 endpoint", "프로필 추가", "프로필 사용", 또는 axhub profile 관리 의도. endpoint allowlist 를 확인하고 위험한 endpoint 는 명시 확인해요.'
multi-step: true
needs-preflight: false
---

# Profile

axhub profile 조회, 추가, 전환을 다뤄요. endpoint 변경은 보안 경계라서 allowlist 확인과 명시 confirm 을 요구해요.

## Workflow

To manage profiles:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "profile 의도 확인", status: "in_progress", activeForm: "profile 확인 중" },
     { content: "현재 profile 조회", status: "pending", activeForm: "현재 상태 보는 중" },
     { content: "필요 시 add/use 실행", status: "pending", activeForm: "profile 처리 중" },
     { content: "endpoint 안전 안내", status: "pending", activeForm: "마무리하는 중" }
   ]})
   ```

   같은 순서로 사용자에게 짧은 단계표도 보여줘요:

   ```
   작업 단계
   └ □ profile 의도 확인
     □ 현재 profile 조회
     □ 필요 시 add/use 실행
     □ endpoint 안전 안내
   ```

1. **작업을 분기해요.** 명확한 `current` 또는 `list` 의도는 질문 없이 read-only 로 가요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 작업 선택은 `현재 프로필 보기` 예요.

2. **필요하면 작업을 고르게 해요.**

   ```json
   {
     "question": "프로필 작업을 고를까요?",
     "header": "profile",
     "options": [
       {"label": "현재 프로필 보기", "value": "current", "description": "현재 endpoint 와 profile 을 봐요"},
       {"label": "목록 보기", "value": "list", "description": "저장된 profile 을 봐요"},
       {"label": "추가 또는 전환", "value": "mutate", "description": "consent 와 endpoint 확인이 필요해요"}
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
