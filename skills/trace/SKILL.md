---
name: trace
description: '이 스킬은 사용자가 배포 실패 원인을 추적하고 싶어할 때 사용해요. 다음 표현에서 활성화: "왜 실패", "왜 안돼", "왜 죽었", "왜 깨졌", "왜 멈췄", "원인 알려줘", "디버그", "추적해", "분석해", "trace", "debug deploy", "why failed", "what went wrong", "diagnose". event_log + runtime_log + audit 3 source 통합 분석으로 phase duration anomaly + last error + routing context 를 1 화면 요약해요.'
examples:
  - utterance: "배포 실패 원인 알려줘"
    intent: "summarize recent failed deploy cause"
  - utterance: "왜 실패했어"
    intent: "summarize recent failed deploy cause"
  - utterance: "원인 알려줘"
    intent: "summarize recent failed deploy cause"
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

배포가 왜 실패했는지 evidence 3 source (event_log + runtime_log + audit) 통합 분석으로 1 화면 안내해요. (현행 backend 엔 build-log API 가 없어서 runtime_log 를 쓰고, 빌드 단계 실패는 event_log `failure_reason` 으로 안내해요.)

<!-- AUTHOR: Phase 25 PR 25.4 — vibe coder 가 "왜 실패했어" 라고 물을 때
1. preflight 출력의 current_app / last_deploy_id 사용 (없으면 list-deployments 의 마지막 Failed)
2. event_log (phase 전환 + duration) + runtime_log (현행 `axhub deploy logs`, 마지막 ERROR/WARN) + audit (routing context) 3 source
3. ERROR pattern catalog 매칭 → references/error-patterns.md 의 4-part empathy entry 출력
4. 다음 액션 권유 (axhub env / axhub recover / 직접 수정 등)
-->

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

## Claude Desktop Natural-Language Path

For ordinary Claude Desktop failure-cause questions, stop after this step. Examples: `배포 실패 원인 알려줘`, `왜 실패했어`, `원인 알려줘`.

1. Say exactly one visible sentence before the tool call:
   `배포 기록을 확인할게요.`
2. Run exactly one Bash tool call:
   - title/description exactly: `배포 기록 확인`
   - command exactly: `axhub-helpers trace-summary --user-utterance "<latest user sentence>"`
3. Copy the helper's Korean stdout as the answer.

Do not show routing labels, slash commands, skill names, `preflight`, deploy IDs, raw status names, JSON field names, `failure_reason`, `matched_patterns`, `build_log_errors`, raw tables, QA result files, plugin source inspection, command names, or English tool-title fragments. If the helper says there is no recent failed deployment, report that calmly and stop; do not run the multi-step trace workflow to manufacture a failure.

To trace a failed deploy:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `axhub_bin_invalid` 는 `AXHUB_BIN` 환경변수가 잘못된 경로 (`cli_resolved_path` 값) 를 가리키는 상태라 재설치 대신 `unset AXHUB_BIN` 후 새 세션 재시도 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

0. **Render TodoWrite checklist (vibe coder sees real-time progress).**

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "대상 deploy 식별",        status: "in_progress", activeForm: "deploy 식별하는 중" },
     { content: "event_log + runtime_log + audit 수집", status: "pending", activeForm: "evidence 수집하는 중" },
     { content: "error pattern 매칭",       status: "pending",     activeForm: "패턴 분석하는 중" },
     { content: "4-part empathy 안내",      status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

1. **대상 deploy 식별.** preflight 의 `current_app` + `last_deploy_id` 사용해요. 없으면 `axhub-helpers list-deployments --app "$APP" --limit 5` 에서 마지막 Failed entry 의 deploy_id 추출. 이 helper 는 JSON 을 기본 출력하고 `--json` flag 를 받지 않아요. 앱도 모호하면 AskUserQuestion 으로 앱을 먼저 고르고, 후보 0 → "추적할 실패 배포 없음" 안내 + 종료.

2. **3 source 수집 (sequential, 5s timeout per source, 평균 15s 상한).**
   - **A: event_log** — `axhub-helpers trace --deploy-id=$ID --app "$APP" --json` 호출 (내부에서 event_log read + 현행 `axhub deploy logs` 런타임 로그 + audit read 다 함)
   - **B: runtime_log** — A 가 포함 (helper 가 현행 `axhub deploy logs` 런타임 로그를 spawn + NDJSON `message` 파싱). 마지막 ERROR/WARN 최대 5 줄. 빌드 단계 실패로 런타임 로그가 비면 event_log `failure_reason` 으로 fallback 매칭
   - **C: audit** — A 가 포함. recent routing context (prompt_hash + is_axhub_related)

3. **Error pattern 매칭.** helper 가 event_log `failure_reason`(authoritative) + 런타임 로그의 ERROR/WARN 라인을 대상으로 **이미 매칭**해서 JSON `matched_patterns` 로 줘요 — 그걸 그대로 써서 `references/error-patterns.md` 의 entry (env_not_found / oom / module_not_found / network_timeout / dependency_install_failed / docker_image_pull_failed / port_already_in_use / build_command_failed) 를 출력해요. `build_log_errors` 는 사용자 화면 인용용(최대 5줄)이지 매칭 소스가 아니에요. JSON `warnings` 에 `runtime_log_unavailable` / `runtime_log_probe_*` / `runtime_log_unparseable` / `runtime_log_schema_*` 가 있으면 evidence 불완전 상태로 안내하고 event_log `failure_reason` 을 우선 사용해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 `trace` 채널 참조 — 추적 대상 질문(question text `"최근 Failed 배포가 여러 개예요. 어떤 거 추적할까요?"`)의 safe_default 는 "abort" (대상이 모호하면 비대화형 환경에선 추적 중단).

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
→ 배포 기록 확인
→ 결과 (4-part empathy):
   잠깐만요. 빌드 중 'env: STRIPE_KEY not found' 발견했어요.
   STRIPE_KEY 환경변수를 axhub env 에 등록해주세요.
   다음: "환경변수 추가해줘"

### CI 자동화
```bash
$ axhub-helpers trace --json --deploy-id=dep-abc
{"deploy_id":"dep-abc","last_phase":"push","failure_reason":"env: STRIPE_KEY not found",
 "phase_durations":[...],"build_log_errors":["ERROR env: STRIPE_KEY not found"],
 "matched_patterns":["env_not_found"],"warnings":["runtime_log_unavailable: ..."]}
```

## NEVER

- NEVER raw runtime_log stderr 를 사용자 화면에 그대로 노출해요. ERROR/WARN 라인 max 5 까지만 인용해요 (Vibe Coder Visibility).
- NEVER axhub 내부 deploy_id 를 prompt 에 echo 해요. routing audit hash 와 cross-correlate 가능성 있어요.
- NEVER 5s timeout 무시. axhub logs 가 hang 되면 evidence 불완전 상태로 안내해요.

## Additional Resources

- `references/error-patterns.md` — 8+ entry 4-part empathy catalog
- `../recover/SKILL.md` (Step 7) — canonical helper `error_code` → user-facing 라우팅 표. trace 가 helper 의 transport/auth 실패를 만났을 때 그 표 그대로 안내해요.
- `../deploy/references/error-empathy-catalog.md` — exit-code 별 4-part 템플릿
- `../deploy/references/nl-lexicon.md` — trigger 어구 추가 시 참조
