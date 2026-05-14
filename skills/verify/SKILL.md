---
name: verify
description: '이 스킬은 사용자가 배포가 진짜 라이브 됐는지 evidence 기반으로 확인하고 싶어할 때 사용해요. 다음 표현에서 활성화: "확인해", "검증해", "라이브 됐어", "정말 됐어", "진짜 올라갔어", "확실해", "테스트해", "smoke test", "is it live", "check live", "verify", "방금 거 확인해줘". axhub status + axhub logs tail + (선택) health endpoint 호출로 evidence 수집해서 verdict 한 줄로 보여줘요.'
examples:
  - utterance: "방금 거 진짜 됐어"
    intent: "verify last deploy is live"
  - utterance: "라이브 됐어"
    intent: "verify last deploy is live"
  - utterance: "확인해줘"
    intent: "verify last deploy is live"
  - utterance: "verify"
    intent: "verify last deploy is live"
  - utterance: "smoke test"
    intent: "verify last deploy is live"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
---

# Verify

axhub 배포가 진짜 라이브 됐는지 evidence 기반으로 1 화면 verdict 로 답해요.

<!-- AUTHOR: Phase 26 PR 26.4 — vibe coder 가 "방금 거 진짜 됐어?" 라고 물을 때
1. preflight 가 current_app / auth_ok 자동 주입
2. axhub status + axhub logs tail 두 호출로 evidence 수집 (5s timeout 각각)
3. 헬스 endpoint 가 설정돼 있으면 GET 200 추가 검증 (선택)
4. verdict: ✅ 라이브 확정 / ⚠️ 의심 / ❌ 라이브 안 됨 — 한 줄
-->

## Workflow

To verify the latest deploy is live:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','inherit','pipe'],env});const stderrText=String(result.stderr??'');const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText)}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "최근 배포 식별",       status: "in_progress", activeForm: "최근 배포 식별하는 중" },
     { content: "axhub status 호출",    status: "pending",     activeForm: "상태 확인하는 중" },
     { content: "axhub logs tail 확인", status: "pending",     activeForm: "로그 확인하는 중" },
     { content: "verdict 안내",         status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **최근 배포 식별.** preflight 의 `current_app` + `last_deploy_id` 사용해요. 둘 다 비어 있으면 `axhub-helpers list-deployments --app-id=$APP --limit 1` 로 보강해요. 후보 없으면 "최근 배포 없음" 안내 + 종료.

2. **`axhub status --json` 호출 (5s timeout).** 응답 `state` 가 `live` / `running` / `deployed` / `succeeded` 면 health 신호 OK. 그 외 → 의심 사유 기록.

3. **`axhub logs --runtime --tail 50` 호출 (5s timeout).** 마지막 50 라인에서 `ERROR` / `FATAL` 패턴 grep. 한 줄도 없으면 OK. 있으면 first 3 라인을 그대로 quote 해요 (Vibe Coder Visibility 원칙).

4. **(선택) health endpoint GET.** apphub.yaml 에 `health_endpoint` 가 정의돼 있으면 `curl -sS -o /dev/null -w "%{http_code}" $URL` 5s timeout 호출해요. 응답 200 = OK. 그 외 → 의심 사유.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — verify SKILL 의 `health_endpoint_setup` safe_default 는 "skip" 이에요 (헬스 endpoint 미설정 시 axhub status + logs 만으로 verdict).

   verify 도중 health endpoint 가 미설정된 상태에서 사용자에게 setup 을 묻고 싶을 때만 AskUserQuestion 호출해요. 비대화형이면 자동 skip.

   ```json
   {
     "questions": [{
       "question": "헬스 endpoint 가 설정 안 돼 있어요. 지금 설정해서 더 깊게 검증할까요?",
       "header": "헬스 endpoint",
       "multiSelect": false,
       "options": [
         {"label": "skip", "description": "axhub status + logs 만으로 verdict 진행"},
         {"label": "지금 설정", "description": "apphub.yaml 의 health_endpoint 필드 추가 가이드"}
       ]
     }]
   }
   ```

5. **Verdict 한국어 해요체 안내.** 4-part empathy 톤 따라요.

   ```
   ✅ 라이브 확정
     - 앱: <APP_SLUG> (id=<APP_ID>) — <PROFILE>
     - 마지막 배포: <DEPLOY_ID> (<RELATIVE_TIME>)
     - 상태: live / 에러 0 건 / health 200
     - 다음: "방금 거 로그 보여줘" / "방금 거 상태"

   ⚠️ 의심
     - <의심 사유 한 줄>
     - 자세한 로그 보려면 "방금 거 로그 보여줘"
     - 다시 확인하려면 1 분 뒤 "다시 확인해줘"

   ❌ 라이브 안 됨
     - state = <state> (live 아님)
     - 마지막 배포 ID: <DEPLOY_ID>
     - 추적하려면 "왜 실패했어"
   ```

## Examples

### 첫 배포 직후 검증
사용자: "방금 거 진짜 됐어?"
→ verify skill 호출
→ Step 1-5 실행
→ 결과: "✅ 라이브 확정. 마지막 배포 2 분 전, state=live, 에러 0 건."

### CI 자동화
```bash
$ axhub-helpers verify --json --app-id=paydrop
{"state":"live","last_deploy_age_secs":120,"errors":[],"verdict":"passed"}
```

## NEVER

- NEVER `axhub status` 응답 stderr 를 사용자 화면에 그대로 노출해요. JSON / NDJSON / payload / transport 같은 jargon 이 들어가요 (Vibe Coder Visibility 위반).
- NEVER 5s timeout 무시해요. axhub status 가 hang 되면 verdict 를 못 내려요. timeout 도달 시 "의심" verdict 로 표시해요.
- NEVER health endpoint URL 을 사용자 화면에 그대로 출력해요. 회사 endpoint 가 노출될 수 있어요. 응답 code 만 표시해요.

## Additional Resources

- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
