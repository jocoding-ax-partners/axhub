---
name: clarify
description: This skill is the fallback for ambiguous axhub-related utterances that did not match deploy, status, logs, apps, apis, auth, update, doctor, recover, or upgrade. Activates on bare verbs without targets, mixed intents, contradictory deixis, or uncertain context like "axhub", "axhub 좀", "axhub 도와줘", "axhub로 뭐 해야 해", "axhub 어떻게 써", "뭔가 잘못된 것 같아", "axhub 관련해서", "axhub 어떻게", "도와줘 axhub", "help me with axhub", "axhub thing", "do something with axhub", or any axhub-context utterance that lacks a clear destination. Presents numbered Korean options and routes to the chosen sibling skill via the Skill tool.
---

# Clarify (fallback router)

When an axhub utterance is ambiguous or no specific sibling skill matched, surface a numbered Korean menu and route to the chosen skill. Never guess silently.

## Workflow

To clarify:

1. **Detect the ambiguity class.** Common cases:
   - Bare verb without target ("axhub 도와줘", "axhub 좀")
   - Mixed intent ("배포 상태 로그 다 보여줘" — could be status OR logs)
   - Contradictory deixis ("그거" with no recent context)
   - Unknown axhub-adjacent term ("axhub 어떻게 써", "axhub thing")

2. **Render numbered Korean menu.** Use AskUserQuestion with the most relevant 4–5 options based on detected class. Default menu:

   ```json
   {
     "question": "어떤 작업을 원하시나요?",
     "options": [
       {"label": "1. 배포 (앱 올리기)", "value": "deploy", "description": "현재 코드를 axhub에 배포"},
       {"label": "2. 상태 확인 (배포 어디까지?)", "value": "status", "description": "진행 중인 배포 추적"},
       {"label": "3. 로그 보기 (왜 실패?)", "value": "logs", "description": "빌드/런타임 로그 확인"},
       {"label": "4. 앱 목록", "value": "apps", "description": "등록된 앱 보기"},
       {"label": "5. 모르겠어요 / 뭐 가능한지 보여줘", "value": "help", "description": "전체 명령 안내"}
     ]
   }
   ```

3. **Context-specific menu.** If the recent-context cache has a deploy in flight, prepend "방금 그 배포" options:

   ```json
   {
     "options": [
       {"label": "방금 배포 상태", "value": "status", "description": "dep_<RECENT_ID> 추적"},
       {"label": "방금 배포 로그", "value": "logs", "description": "dep_<RECENT_ID> 로그"},
       {"label": "다른 작업", "value": "other", "description": "메뉴 다시 보기"}
     ]
   }
   ```

4. **Route to chosen skill.** Use the Skill tool to invoke the sibling skill. Pass the original user utterance forward so the sibling can re-resolve context:

   - `deploy` → invoke skill `axhub:deploy`
   - `status` → invoke skill `axhub:status`
   - `logs` → invoke skill `axhub:logs`
   - `apps` → invoke skill `axhub:apps`
   - `apis` → invoke skill `axhub:apis`
   - `auth` → invoke skill `axhub:auth`
   - `update` → invoke skill `axhub:update`
   - `doctor` → invoke skill `axhub:doctor`
   - `recover` → invoke skill `axhub:recover`
   - `upgrade` → invoke skill `axhub:upgrade`

5. **On `help`.** Render the full axhub command matrix from PLAN §3.1 in plain Korean:

   ```
   axhub로 할 수 있는 일:
     · 배포: "배포해", "올려줘", "프로덕션에 박아"
     · 상태: "어떻게 됐어", "지금 어디까지", "방금 거 됐어"
     · 로그: "왜 실패했어", "로그 봐", "에러 봐"
     · 앱:   "내 앱 보여줘", "앱 뭐 있어"
     · API:  "API 뭐 있어", "엔드포인트 봐"
     · 인증: "로그인", "누구야", "토큰 만료"
     · 업데이트: "새 버전 있어", "업그레이드해"
     · 진단: "axhub 잘 돼", "환경 점검"
     · 복구: "되돌려", "직전 버전으로"
   ```

6. **Out-of-scope detection.** If the utterance contains a non-axhub platform (vercel/netlify/heroku/firebase/k8s/aws), surface:

   > "axhub 말씀이신가요, 아니면 다른 플랫폼인가요? (vercel/netlify는 다른 도구라서 axhub 플러그인은 도와드릴 수 없어요.)"

   Stop without routing.

## NEVER

- NEVER silently guess intent — always surface AskUserQuestion when ambiguity is detected.
- NEVER auto-route without user confirmation (the destructive ops have their own consent gates, but read-only ops still benefit from intent confirmation).
- NEVER suggest more than 5 options at once (vibe coders can't compare beyond 5).
- NEVER include cross-platform deploy targets (vercel/heroku/...) in the menu.
- NEVER skip the help option ("모르겠어요" must always be selectable).

## Additional Resources

For Korean trigger lexicon (all intents): `../deploy/references/nl-lexicon.md`.
For deixis resolution rules ("그거", "방금 거", "어제 거"): `../deploy/references/nl-lexicon.md` (section 11).
For anti-pattern exclusions (non-axhub platforms): `../deploy/references/nl-lexicon.md` (section 13).
For clarify pattern reference: `../deploy/references/recovery-flows.md`.
