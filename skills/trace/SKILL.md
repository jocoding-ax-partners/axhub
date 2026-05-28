---
name: trace
description: '이 스킬은 사용자가 배포 실패 원인을 추적하고 싶어할 때 사용해요. 다음 표현에서 활성화: "왜 실패", "왜 안돼", "왜 죽었", "왜 깨졌", "왜 멈췄", "원인 알려줘", "디버그", "추적해", "분석해", "trace", "debug deploy", "why failed", "what went wrong", "diagnose". event_log + build_log + audit 3 source 통합 분석으로 phase duration anomaly + last error + routing context 를 1 화면 요약해요.'
examples:
  - utterance: "왜 실패했어"
    intent: "trace last failed deploy"
  - utterance: "원인 알려줘"
    intent: "trace last failed deploy"
  - utterance: "왜 안돼"
    intent: "trace last failed deploy"
  - utterance: "trace"
    intent: "trace last failed deploy"
  - utterance: "debug deploy"
    intent: "trace last failed deploy"
multi-step: true
needs-preflight: true
model: sonnet
allows-dependency-execution: false
---

# Trace

배포가 왜 실패했는지 evidence 3 source (event_log + build_log + audit) 통합 분석으로 1 화면 안내해요.

<!-- AUTHOR: Phase 25 PR 25.4 — vibe coder 가 "왜 실패했어" 라고 물을 때
1. preflight 출력의 current_app / last_deploy_id 사용 (없으면 list-deployments 의 마지막 Failed)
2. event_log (phase 전환 + duration) + build_log (마지막 ERROR/WARN) + audit (routing context) 3 source
3. ERROR pattern catalog 매칭 → references/error-patterns.md 의 4-part empathy entry 출력
4. 다음 액션 권유 (axhub env / axhub recover / 직접 수정 등)
-->

## Workflow

To trace a failed deploy:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers"; [ -x "$HELPER" ] || HELPER="axhub-helpers"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 `/axhub:auth` 로 로그인을 안내하고, `auth_error_code` 가 있으면 그에 맞게 안내해요 (`cli_not_found`/`cli_unavailable` → `/axhub:install-cli`, `cli_config_corrupted` → `/axhub:auth` 재로그인, `cli_too_old` → `/axhub:upgrade`). 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   ```typescript
   TodoWrite({ todos: [
     { content: "대상 deploy 식별",        status: "in_progress", activeForm: "deploy 식별하는 중" },
     { content: "event_log + build_log + audit 수집", status: "pending", activeForm: "evidence 수집하는 중" },
     { content: "error pattern 매칭",       status: "pending",     activeForm: "패턴 분석하는 중" },
     { content: "4-part empathy 안내",      status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **대상 deploy 식별.** preflight 의 `current_app` + `last_deploy_id` 사용해요. 없으면 `axhub-helpers list-deployments --app "$APP" --limit 5 --json` 에서 마지막 Failed entry 의 deploy_id 추출. 앱도 모호하면 AskUserQuestion 으로 앱을 먼저 고르고, 후보 0 → "추적할 실패 배포 없음" 안내 + 종료.

2. **3 source 수집 (sequential, 5s timeout per source, 평균 15s 상한).**
   - **A: event_log** — `axhub-helpers trace --deploy-id=$ID --app "$APP" --json` 호출 (내부에서 event_log read + current CLI deploy logs build + audit read 다 함)
   - **B: build_log** — A 가 포함 (helper 가 spawn). 마지막 ERROR/WARN 최대 5 줄
   - **C: audit** — A 가 포함. recent routing context (prompt_hash + is_axhub_related)

3. **Error pattern 매칭.** `references/error-patterns.md` 의 8+ entry (env_not_found / oom / module_not_found / network_timeout / dependency_install_failed / docker_image_pull_failed / port_already_in_use / build_command_failed) 중 build_log_errors 에서 매칭되는 것 선택.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — trace SKILL 의 `trace_target_selection` safe_default 는 "abort" (대상이 모호하면 비대화형 환경에선 추적 중단).

4. **여러 후보 deploy 가 있을 때만 AskUserQuestion.**

   ```json
   {
     "questions": [{
       "question": "최근 Failed 배포가 여러 개예요. 어떤 거 추적할까요?",
       "header": "추적 대상",
       "multiSelect": false,
       "options": [
         {"label": "가장 최근", "description": "가장 최신 Failed deploy_id"},
         {"label": "직접 입력", "description": "deploy_id 명시"}
       ]
     }]
   }
   ```

5. **4-part empathy 안내 (한국어 해요체).** `references/error-patterns.md` 의 매칭 entry 그대로 출력. 매칭 없으면 generic 안내.

   ```
   <감정 — 잠깐만요 / 지금 안 돼요>
   <원인 — phase + 패턴 + raw error 인용>
   <액션 — 다음에 할 수 있는 명령>
   <다음 버튼 — slash skill 권유 or plain 한국어>
   ```

## Examples

### 배포 실패 후 추적
사용자: "왜 실패했어?"
→ trace skill 호출
→ Step 1-5 실행
→ 결과 (4-part empathy):
   잠깐만요. 빌드 중 'env: STRIPE_KEY not found' 발견했어요.
   STRIPE_KEY 환경변수를 axhub env 에 등록해주세요.
   다음: /axhub:env 또는 "환경변수 추가해줘"

### CI 자동화
```bash
$ axhub-helpers trace --json --deploy-id=dep-abc
{"deploy_id":"dep-abc","last_phase":"push","failure_reason":"env: STRIPE_KEY not found",
 "phase_durations":[...],"build_log_errors":["ERROR env: STRIPE_KEY not found"],
 "matched_patterns":["env_not_found"]}
```

## NEVER

- NEVER raw build_log stderr 를 사용자 화면에 그대로 노출해요. ERROR/WARN 라인 max 5 까지만 인용해요 (Vibe Coder Visibility).
- NEVER axhub 내부 deploy_id 를 prompt 에 echo 해요. routing audit hash 와 cross-correlate 가능성 있어요.
- NEVER 5s timeout 무시. axhub logs 가 hang 되면 evidence 불완전 상태로 안내해요.

## Additional Resources

- `references/error-patterns.md` — 8+ entry 4-part empathy catalog
- `../recover/SKILL.md` (Step 7) — canonical helper `error_code` → user-facing 라우팅 표. trace 가 helper 의 transport/auth 실패를 만났을 때 그 표 그대로 안내해요.
- `../deploy/references/error-empathy-catalog.md` — exit-code 별 4-part 템플릿
- `../deploy/references/nl-lexicon.md` — trigger 어구 추가 시 참조
