---
name: whatsnew
description: 이 스킬은 사용자가 axhub 의 새 기능, 변경점, 릴리즈 노트, changelog 를 알고 싶어할 때 사용해요. 다음 표현에서 활성화: "뭐 새로 나왔어", "새 기능 뭐야", "release notes", "changelog", "what's new", "whatsnew", "신규 기능", 또는 axhub 변경점 확인 의도. axhub whatsnew 를 read-only 로 호출해요.
multi-step: false
needs-preflight: false
---

# Whatsnew

axhub CLI 가 제공하는 변경점 요약을 read-only 로 보여줘요.

## Workflow

To show what is new:

1. **CLI 명령을 호출해요.**

   ```bash
   axhub whatsnew --json
   ```

   JSON 이 지원되지 않는 CLI 라면 `axhub whatsnew` 로 fallback 하고, 출력은 원문 그대로 길게 붙이지 말고 요약해요.

2. **사용자에게 필요한 변화만 묶어요.** breaking change, migration, security note, new command 를 구분해요.

3. **plugin release 와 CLI release 를 구분해요.** 플러그인 업그레이드 의도면 upgrade skill 로 넘기고, CLI binary 업데이트 의도면 update skill 로 넘겨요.

## NEVER

- NEVER 인터넷 release note 를 임의로 source of truth 로 삼지 않아요.
- NEVER update/upgrade 를 자동 실행하지 않아요.
- NEVER changelog 원문을 과도하게 길게 붙이지 않아요.

## Non-interactive AskUserQuestion guard

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 은 현재 structured AskUserQuestion 을 쓰지 않지만, 질문을 추가할 때는 `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서 안전 기본값을 사용해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 등록해요.
