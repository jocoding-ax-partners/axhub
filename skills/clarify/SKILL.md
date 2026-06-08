---
name: clarify
description: '이 스킬은 사용자가 axhub에 대해 막연히 도와달라고 하거나 목적지를 정하지 않은 질문을 할 때 사용하는 자연어 질문 카드입니다. 대상이 없는 말, 의도 혼합, 모순된 지시어, 또는 다음과 같은 불확실 컨텍스트에서 활성화: "도와줘 axhub", "뭔가 잘못된 것 같아", "axhub 관련", "axhub 관련해서", "axhub 도와줘", "axhub 좀 도와줘", "axhub 좀", "axhub로 뭐 해야 해", "axhub", "axhub thing", "do something with axhub", "help me with axhub". 화면에는 내부 라우팅 이름을 보이지 말고 사람이 고르기 쉬운 한국어 선택지만 보여줍니다.'
examples:
  - utterance: "도와줘 axhub"
    intent: "disambiguate axhub intent"
  - utterance: "뭔가 잘못된 것 같아"
    intent: "disambiguate axhub intent"
  - utterance: "axhub"
    intent: "disambiguate axhub intent"
  - utterance: "axhub thing"
    intent: "disambiguate axhub intent"
  - utterance: "환경"
    intent: "disambiguate axhub intent"
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: haiku
---

# Clarify (intent chooser)

When an axhub utterance is ambiguous or no specific sibling skill matched, surface a numbered Korean menu and continue from the chosen user-facing intent. Never guess silently.

## Claude Desktop Natural-Language Path

When the user says something broad like `axhub 좀 도와줘`, this is a UX surface, not a route-debug surface.

- First visible chat sentence must be exactly `어떤 걸 도와드릴까요?`
- Use one question card with natural Korean labels and descriptions.
- Do not say the user was "too vague" or "too broad".
- Do not show slash commands, skill names, routing labels, command mappings, or parenthesized internal names in the chat or question-card text.
- Do not include option text like `(doctor)`, `(deploy)`, `(status)`, `(logs)`, `(apps)`, `(help)`, or `axhub:*`.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To clarify:

1. **Detect the ambiguity class.** Common cases:
   - Bare verb without target ("axhub 도와줘", "axhub 좀")
   - Mixed intent ("배포 상태 로그 다 보여줘" — could be status OR logs)
   - Contradictory deixis ("그거" with no recent context)
   - Unknown axhub-adjacent term ("axhub 어떻게 써", "axhub thing")

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — clarify menu → `abort` (모호한 의도라 subprocess 에서는 추측 안 해요).

2. **Render numbered Korean menu.** Use AskUserQuestion with the most relevant 4–5 options based on detected class. Labels and descriptions are user-facing. If the AskUserQuestion schema requires `value`, set each `value` to exactly the same Korean text as its visible `label`; never use skill names or command names as values. Claude Desktop can surface option values when it explains a selection. Default menu:

   ```json
   {
     "question": "어떤 걸 도와드릴까요?",
     "header": "작업 선택",
     "options": [
       {"label": "환경 점검", "value": "환경 점검", "description": "설치, 로그인, 버전 상태를 확인해요"},
       {"label": "앱 배포", "value": "앱 배포", "description": "현재 프로젝트를 올릴 준비를 해요"},
       {"label": "앱과 리소스 조회", "value": "앱과 리소스 조회", "description": "내 앱, 리소스, 테이블을 확인해요"},
       {"label": "문제 원인 보기", "value": "문제 원인 보기", "description": "상태, 로그, 실패 원인을 확인해요"},
       {"label": "처음부터 안내", "value": "처음부터 안내", "description": "가능한 작업을 한눈에 보여줘요"}
     ]
   }
   ```

3. **Context-specific menu.** If the recent-context cache has a deploy in flight, prepend "방금 그 배포" options:

   ```json
   {
     "options": [
       {"label": "방금 배포 상태", "value": "방금 배포 상태", "description": "최근 배포 진행 상황을 확인해요"},
       {"label": "방금 배포 로그", "value": "방금 배포 로그", "description": "최근 배포 로그를 확인해요"},
       {"label": "다른 작업", "value": "다른 작업", "description": "작업 선택지를 다시 보여줘요"}
     ]
   }
   ```

4. **Continue from the selected user-facing intent.** Do not call the Claude Skill tool, do not invoke `/axhub:*`, and do not narrate a route transition. Match by the selected label text and run the narrow helper or natural follow-up below. Pass the original user utterance to helpers only as an argument, never as visible route text.

### Selected Option Handoff

After the user chooses an option, never narrate the route transition. The next visible sentence must be the destination flow's natural first sentence, not a meta sentence. Do not call another skill from this skill.

Do not write phrases like "진행", "스킬 실행", "skill 실행", "skill 호출", "SKILL.md", "읽는 중", "route", "라우팅", "handoff", or an internal command name in visible chat.

For the default menu options:

- If the user chooses "환경 점검":
  - First visible sentence, exactly: `설치 상태를 확인할게요.`
  - Use one Bash tool call. Bash description/title, exactly: `설치 상태 확인`
  - Bash command: `axhub-helpers doctor-summary --user-utterance "<original broad user sentence>"`
  - Copy the Korean stdout as the answer.
- If the user chooses "앱 배포":
  - First visible sentence, exactly: `배포 준비를 확인할게요.`
  - Use one Bash tool call. Bash description/title, exactly: `배포 준비 확인`
  - Bash command: `axhub-helpers deploy-preview-summary --user-utterance "<original broad user sentence>"`
  - Show the Korean preview and ask for explicit approval before any deploy execution.
- If the user chooses "앱과 리소스 조회":
  - First visible sentence, exactly: `앱과 리소스를 확인할게요.`
  - Continue with the app/resource inventory flow without route narration.
- If the user chooses "문제 원인 보기":
  - First visible sentence, exactly: `문제 원인을 확인할게요.`
  - If a recent failed deployment is available, use the failure-cause flow; otherwise ask whether they want status, logs, or configuration.

5. **On `help`.** Render the full axhub command matrix from PLAN §3.1 in plain Korean:

   ```
   axhub로 할 수 있는 일:
     · 배포: "배포해", "올려줘", "프로덕션에 박아"
     · 상태: "어떻게 됐어", "지금 어디까지", "방금 거 됐어"
     · 로그: "왜 실패했어", "로그 봐", "에러 봐"
     · 앱:   "내 앱 보여줘", "앱 뭐 있어"
     · 인증: "로그인", "누구야", "토큰 만료"
     · 업데이트: "새 버전 있어", "업그레이드해"
     · 진단: "axhub 잘 돼", "환경 점검"
     · 복구: "되돌려", "직전 버전으로"
   ```

6. **Out-of-scope detection.** If the utterance contains a non-axhub platform (vercel/netlify/heroku/firebase/k8s/aws), surface:

   > "axhub 말씀이신가요, 아니면 다른 플랫폼인가요? (vercel/netlify는 다른 도구라서 axhub 플러그인은 도와드릴 수 없어요.)"

   Stop without routing.

7. **Audit feedback (final).** 사용자가 disambiguation option 을 고른 뒤 packaged helper 로 fail-soft feedback record 를 남겨요. prompt 원문은 저장하지 않고 helper 가 로컬에서 sha256 hash 로 변환해요. `FINAL_SKILL` 은 위 Step 4 의 선택값이에요.

   ```bash
   ORIGINAL_PROMPT="${ORIGINAL_USER_UTTERANCE:-}"
   FINAL_SKILL="${FINAL_SKILL:-null}"
   HELPER_BIN="${AXHUB_HELPERS_BIN:-}"
   if [ -z "$HELPER_BIN" ] && [ -n "${CLAUDE_PLUGIN_ROOT:-}" ] && [ -x "${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers" ]; then
     HELPER_BIN="${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers"
   fi
   [ -n "$HELPER_BIN" ] && [ -x "$HELPER_BIN" ] || HELPER_BIN="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER_BIN" ] && [ -x "$HELPER_BIN" ] || HELPER_BIN="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   if [ -z "$HELPER_BIN" ]; then
     HELPER_BIN="axhub-helpers"
   fi
   if [ -n "$ORIGINAL_PROMPT" ]; then
     "$HELPER_BIN" audit-clarify --prompt "$ORIGINAL_PROMPT" --chosen "$FINAL_SKILL" >/dev/null 2>&1 || true
   fi
   ```

   이 기록은 `axhub-helpers routing-stats --confused --json` 과 `bun run routing:tune --confused` 의 feedback input 이에요.

## NEVER

- NEVER silently guess intent — always surface AskUserQuestion when ambiguity is detected.
- NEVER auto-route without user confirmation (the destructive ops have their own preview gates, but read-only ops still benefit from intent confirmation).
- NEVER call the Claude Skill tool after the user chooses a menu option; continue inline with the natural first sentence and helper command.
- NEVER suggest more than 5 options at once (vibe coders can't compare beyond 5).
- NEVER include cross-platform deploy targets (vercel/heroku/...) in the menu.
- NEVER skip the help option ("모르겠어요" must always be selectable).
- NEVER include parenthesized internal labels in choices or descriptions.
- NEVER write dismissive "too broad/vague" wording, route names, slash commands, skill names, command names, or implementation values into the visible answer.

## Additional Resources

For Korean trigger lexicon (all intents): `../deploy/references/nl-lexicon.md`.
For deixis resolution rules ("그거", "방금 거", "어제 거"): `../deploy/references/nl-lexicon.md` (section 11).
For anti-pattern exclusions (non-axhub platforms): `../deploy/references/nl-lexicon.md` (section 13).
For clarify pattern reference: `../deploy/references/recovery-flows.md`.
