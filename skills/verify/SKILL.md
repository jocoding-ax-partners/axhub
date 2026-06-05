---
name: verify
description: '이 스킬은 사용자가 배포가 진짜 라이브 됐는지 evidence 기반으로 확인하고 싶어할 때 사용해요. 특히 Claude Desktop에서 "방금 배포 진짜 열리는지 확인해줘" 같은 자연어 요청은 내부 routing/preflight/status/logs 단계를 말하지 않고 단일 검증 요약으로 처리해요. 다음 표현에서 활성화: "방금 배포 진짜 열리는지 확인해줘", "확인해", "검증해", "라이브 됐어", "정말 됐어", "진짜 올라갔어", "확실해", "테스트해", "smoke test", "is it live", "check live", "verify", "방금 거 확인해줘". axhub-helpers verify-summary 가 current CLI deploy list/status/logs 를 호출하고, Desktop에 맞는 한국어 verdict 로 보여줘요.'
examples:
  - utterance: "방금 배포 진짜 열리는지 확인해줘"
    intent: "verify last deploy is live"
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
1. preflight 출력의 current_app / auth_ok 사용
2. helper 가 current CLI deploy list/status/logs 를 호출해 evidence 수집 (5s timeout 각각)
3. 헬스 endpoint 가 설정돼 있으면 GET 200 추가 검증 (선택)
4. verdict: ✅ 라이브 확정 / ⚠️ 의심 / ❌ 라이브 안 됨 — 한 줄
-->

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

**Claude Desktop visible contract:** begin with exactly `배포가 실제로 열리는지 확인할게요.`. For the first Bash tool call, set the tool `description` or title exactly to `배포 검증`. Do not show routing labels, skill names, preflight labels, stale cache IDs, deploy IDs, user emails, raw status names, JSON field names, command names, or intermediate fallback attempts in visible chat.

Use this single helper path for ordinary Claude Desktop verification questions:

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
"$HELPER" verify-summary --user-utterance "$USER_UTTERANCE"
```

Copy the helper's Korean stdout as the answer. For ordinary Claude Desktop verify questions, stop after this step. Do not continue with raw `axhub deploy status`, raw `axhub deploy logs`, `axhub-helpers preflight`, direct `axhub-helpers verify --app-id`, or manual status/log/list fallback unless the helper itself says it could not resolve the app and asks for a specific app.

To verify the latest deploy is live:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "최근 배포 식별",              status: "in_progress", activeForm: "최근 배포 식별하는 중" },
     { content: "axhub deploy status 호출",    status: "pending",     activeForm: "상태 확인하는 중" },
     { content: "axhub deploy logs 확인",      status: "pending",     activeForm: "로그 확인하는 중" },
     { content: "verdict 안내",                status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **최근 배포 식별.** preflight 의 `current_app` + `last_deploy_id` 사용해요. 둘 다 비어 있으면 `axhub-helpers list-deployments --app-id "$APP" --limit 1` 로 보강해요. `--app-id` 가 primary, `--app` 는 alias예요. 후보 없으면 "최근 배포 없음" 안내 + 종료.

2. **`axhub deploy status <DEPLOY_ID> --app <APP> --json` 호출 (5s timeout).** `<DEPLOY_ID>` 는 Step 1 의 `last_deploy_id`. 없으면 `axhub deploy list --app <APP> --json`(`--limit` 없음 — 출력에서 최신 선택)으로 보강해요. `.status` 는 백엔드 free string 이에요. helper `LIVE_STATES`(`live` / `running` / `deployed` / `active` / `ok` / `succeeded`) 중 하나면 live 상태 OK. `.current_stage` 로 단계 안내. 그 외 값은 미라이브 휴리스틱으로 기록해요 — 예를 들어 `pending`/`building`/`deploying` 류면 진행 중 → 의심, `failed`/`stopped` 류면 → 라이브 안 됨. (이 라벨들은 휴리스틱 예시이지, status 가 닫힌 enum 은 아니에요.)

3. **`axhub deploy logs --app <APP> --json` 호출 (5s timeout, app-level).** 로그는 app-level 백엔드 라우트로 받아요. `<DEPLOY_ID>` positional 은 legacy 라 스코핑에 안 써요. 받은 출력에서 마지막 ~50 라인(client-side trim)에서 `ERROR` / `FATAL` 패턴 grep. 한 줄도 없으면 OK. 있으면 first 3 라인을 그대로 quote 해요 (Vibe Coder Visibility 원칙). `--tail` 같은 N-라인 플래그는 CLI 에 없으니 출력을 받아서 직접 마지막 50 라인만 잘라요. `--source` 는 passthrough 문자열(pod/runtime/build 같은 고정 enum 없음)이라 verify 는 지정하지 않고 app-level 로그만 받아요. axhub-cli 0.15.3+ 는 비-TTY/에이전트 컨텍스트에서 watch/follow 를 안전하게 auto-degrade 해요. (verify 는 `--follow` 를 안 써서 항상 단발 스냅샷이고, CLI 가 비-TTY/에이전트 컨텍스트면 자동으로 단일 스냅샷으로 degrade 돼요.)

4. **(선택) health endpoint GET.** axhub.yaml 에 `health_endpoint` 가 정의돼 있으면 `curl -sS -o /dev/null -w "%{http_code}" $URL` 5s timeout 호출해요. 응답 200 = OK. 그 외 → 의심 사유.

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
         {"label": "지금 설정", "description": "axhub.yaml 의 health_endpoint 필드 추가 가이드"}
       ]
     }]
   }
   ```

5. **Verdict 한국어 해요체 안내.** 4-part empathy 톤 따라요.

   ```
   ✅ 라이브 확정
     - 앱: <APP_SLUG> (id=<APP_ID>) — <PROFILE>
     - 마지막 배포: <DEPLOY_ID> (<RELATIVE_TIME>)
     - 상태: status=active / 에러 0 건 / health 200
     - 다음: "방금 거 로그 보여줘" / "방금 거 상태"

   ⚠️ 의심
     - <의심 사유 한 줄>
     - 자세한 로그 보려면 "방금 거 로그 보여줘"
     - 다시 확인하려면 1 분 뒤 "다시 확인해줘"

   ❌ 라이브 안 됨
     - status = <status> (live 아님)
     - 마지막 배포 ID: <DEPLOY_ID>
     - 추적하려면 "왜 실패했어"
   ```

## Examples

### 첫 배포 직후 검증
사용자: "방금 거 진짜 됐어?"
→ 배포 검증 흐름으로 진행
→ 단일 검증 요약 실행
→ 결과: "✅ 라이브 확정. 앱이 최근 배포 기준으로 열리는 상태예요."

### CI 자동화
```bash
$ axhub-helpers verify --json --app-id paydrop   # --app-id 가 primary, --app 는 alias
{"verdict":"live","state":"active","last_deploy_id":"dep_abc","last_deploy_age_secs":120,"errors":[],"reasons":["상태 healthy, 에러 로그 없음"]}
```
verdict 매핑: `live` → ✅ 라이브 / `suspect` → ⚠️ 의심 / `not_live` → ❌ 안 됨. `reasons` 배열은 verdict 아래 그대로 보여줘요.

## NEVER

- NEVER `axhub deploy status` 응답 stderr 를 사용자 화면에 그대로 노출해요. JSON / NDJSON / payload / transport 같은 jargon 이 들어가요 (Vibe Coder Visibility 위반).
- NEVER 5s timeout 무시해요. axhub deploy status 가 hang 되면 verdict 를 못 내려요. timeout 도달 시 "의심" verdict 로 표시해요.
- NEVER health endpoint URL 을 사용자 화면에 그대로 출력해요. 회사 endpoint 가 노출될 수 있어요. 응답 code 만 표시해요.

## Additional Resources

- `../recover/SKILL.md` (Step 7) — canonical helper `error_code` → user-facing 라우팅 표 (`transport.cli_missing`, `token_invalid`, `response.invalid_json` 등). 이 SKILL 은 그 표를 참조해서 동일하게 분기해요.
- `../deploy/references/error-empathy-catalog.md` — 4-part Korean exit-code template.
- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
