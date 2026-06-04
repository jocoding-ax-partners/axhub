---
name: open
description: '이 스킬은 사용자가 배포된 axhub 앱이나 앱 콘솔 페이지를 브라우저에서 열어보고 싶어할 때 사용해요. 특히 "라이브 페이지 열어봐" 같은 자연어 요청은 QA 파일 읽기나 Chrome MCP 탐색이 아니라 이 read-only open 흐름으로 처리해요. 다음 표현에서 활성화: "결과 봐", "라이브 봐", "라이브 페이지 열어봐", "브라우저로 열", "브라우저로 열어", "프로덕션 열", "프로덕션 열어", "deploy URL 봐", "logs 페이지", "metrics 봐", "deploy url", "open", "open in browser", 또는 배포 결과 확인 의도. axhub open 을 호출해 read-only 로 URL 을 확인해요.'
examples:
  - utterance: "결과 봐"
    intent: "open axhub deployment in browser"
  - utterance: "라이브 페이지 열어봐"
    intent: "open axhub deployment in browser"
  - utterance: "라이브 봐"
    intent: "open axhub deployment in browser"
  - utterance: "deploy url"
    intent: "open axhub deployment in browser"
  - utterance: "open"
    intent: "open axhub deployment in browser"
  - utterance: "브라우저로 열"
    intent: "open axhub deployment in browser"
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Open

배포된 axhub 앱, logs 페이지, metrics 페이지를 read-only 로 열거나 URL 을 보여줘요.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To open deployed axhub resources:

**Claude Desktop visible contract:** start with `앱 페이지를 확인할게요.` when the host permits visible text before tools. Use one Bash tool with the Korean title `앱 페이지 확인`. Do not show QA-result-file reads, Chrome MCP discovery, ToolSearch narration, raw command names, JSON field names, or internal routing labels to the user.

1. **일반 open 요청은 한 번에 요약해요.** The helper resolves the current app, calls the canonical CLI open command, and prints a Korean user-facing summary. Do not inspect QA result files, `.omc`, `.claude`, plugin cache files, git logs, Chrome MCP state, or browser extension state.

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   USER_UTTERANCE="<the user's exact latest sentence>"
   "$HELPER" open-summary --user-utterance "$USER_UTTERANCE"
   ```

   Show the Korean stdout as-is. If it says the app cannot be resolved or the page cannot be opened, stop there and ask a natural follow-up. For ordinary Claude Desktop open/browser questions, stop after this step.

2. **대상을 수동 확인해야 하는 명시적 고급 요청에서만** 발화에 slug/id 가 있으면 그대로 쓰고, 없으면 최근 deploy cache 또는 current app 을 사용해요.

3. **모드에 맞는 read-only 명령을 실행해요.**

   ```bash
   axhub open "$APP" --json
   axhub open "$APP" --logs --json
   axhub open "$APP" --metrics --json
   ```

4. **브라우저 실행과 URL 표시를 분리해요.** CLI 가 URL 을 반환하면 먼저 URL 을 보여주고, CLI 가 브라우저를 열었다면 열린 대상을 요약해요.

5. **manifest 없음 오류를 친절하게 안내해요.** `axhub.yaml` 또는 legacy `apphub.yaml` 이 없다는 오류면 init skill 또는 apps skill 로 이어가요.

## NEVER

- NEVER deploy, mutate, login 을 대신 실행하지 않아요.
- NEVER `apphub.yaml` 만 정답으로 가정하지 않아요. 정준 `axhub.yaml` 과 legacy `apphub.yaml` dual-read 를 안내해요.
- NEVER browser open 실패를 deploy 실패로 말하지 않아요.

## Non-interactive AskUserQuestion guard

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 은 현재 structured AskUserQuestion 을 쓰지 않지만, 질문을 추가할 때는 `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서 안전 기본값을 사용해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 에 등록해요.
