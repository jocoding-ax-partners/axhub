---
name: open
description: '이 스킬은 사용자가 배포된 axhub 앱을 브라우저에서 열어보고 싶어할 때 사용해요. 다음 표현에서 활성화: "결과 봐", "라이브 봐", "브라우저로 열어", "프로덕션 열어", "deploy URL 봐", "open", "open in browser", "metrics 봐", "logs 페이지", 또는 배포 결과 확인 의도. axhub open 을 호출해 read-only 로 URL 을 확인해요.'
multi-step: false
needs-preflight: false
---

# Open

배포된 axhub 앱, logs 페이지, metrics 페이지를 read-only 로 열거나 URL 을 보여줘요.

## Workflow

To open deployed axhub resources:

1. **대상을 확인해요.** 발화에 slug/id 가 있으면 그대로 쓰고, 없으면 최근 deploy cache 또는 current app 을 사용해요.

2. **모드에 맞는 read-only 명령을 실행해요.**

   ```bash
   axhub open "$APP" --json
   axhub open "$APP" --logs --json
   axhub open "$APP" --metrics --json
   ```

3. **브라우저 실행과 URL 표시를 분리해요.** CLI 가 URL 을 반환하면 먼저 URL 을 보여주고, CLI 가 브라우저를 열었다면 열린 대상을 요약해요.

4. **manifest 없음 오류를 친절하게 안내해요.** `apphub.yaml` 또는 `axhub.yaml` 이 없다는 오류면 init skill 또는 apps skill 로 이어가요.

## NEVER

- NEVER deploy, mutate, login 을 대신 실행하지 않아요.
- NEVER `apphub.yaml` 만 정답으로 가정하지 않아요. legacy `axhub.yaml` 도 안내해요.
- NEVER browser open 실패를 deploy 실패로 말하지 않아요.

## Non-interactive AskUserQuestion guard

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 은 현재 structured AskUserQuestion 을 쓰지 않지만, 질문을 추가할 때는 `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서 안전 기본값을 사용해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 등록해요.
