---
name: recover
description: '이 스킬은 사용자가 직전 배포를 되돌리거나 이전 버전으로 복원하고 싶어할 때 사용해요. 다음 표현에서 활성화: "되돌", "되돌려", "롤백", "롤백 부탁드립니다", "롤백해", "마지막 정상", "마지막 정상 빌드로", "마지막 정상 빌드로 돌려주세요", "망했어 되돌려", "방금 거 되돌려", "방금 배포 취소해", "배포 취소", "복구", "복구해", "안정 버전", "안정 버전으로", "어제 거로 돌려줘", "이전 버전", "이전 버전으로", "이전 버전으로 되돌려주세요", "잘 되던 버전으로 돌려", "직전 버전", "직전 버전으로 돌려줘", "직전 안정 버전으로 복구 부탁", "forward fix", "hot fix", "hotfix", "redeploy previous", "restore", "restore previous", "revert", "roll back", "rollback", "undo", "undo deploy", 또는 이전 상태 복원 의도. 참고: 특정 배포로 되돌리는 `deploy rollback` 은 rollback 스킬이 담당하고, 이 스킬은 직전 안정 commit 재배포 forward-fix 를 담당해요.'
examples:
  - utterance: "되돌"
    intent: "rollback axhub deployment"
  - utterance: "되돌려"
    intent: "rollback axhub deployment"
  - utterance: "forward fix"
    intent: "rollback axhub deployment"
  - utterance: "hot fix"
    intent: "rollback axhub deployment"
  - utterance: "롤백"
    intent: "rollback axhub deployment"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Recover (forward-fix-as-rollback)

Restore the previous known-good deploy by **redeploying the prior commit**, not by reversing the current one. The current CLI also has `axhub deploy rollback`; use the rollback skill for deployment-id rollback. This recover skill stays focused on forward-fix redeploy of a known-good commit.

> "이건 진짜 rollback이 아니라 forward-fix예요 — 직전 안정 커밋을 다시 배포하는 방식이에요. 결과는 같지만 새 배포가 한 건 더 생겨요."

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

## Claude Desktop natural-language path

For ordinary Claude Desktop prompts such as `방금 배포 되돌려줘`, `방금 거 되돌려줘`, `직전 안정 버전으로 복구해줘`, or `잘 되던 버전으로 돌려`, start with exactly:

`되돌릴 수 있는 배포를 확인할게요.`

Then make exactly one Bash call with the tool description/title exactly:

`배포 되돌리기 확인`

Run:

```bash
axhub-helpers rollback-summary --user-utterance "<latest user sentence>"
```

Copy the Korean stdout as the answer, then stop. Do not read the rest of this workflow, do not call TodoWrite, do not run preflight/list/rollback/recover/create directly, and do not mention `rollback`, `recover`, slash commands, skill names, route labels, preflight, raw deploy IDs, raw commit hashes, raw status names, `commit_not_found`, `no-op`, app IDs/slugs, or English tool-title fragments in visible text. Any actual rollback or redeploy is destructive/external and must wait until the user sees a Korean preview and explicitly approves in a later turn.

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

To recover:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start so the user sees rollback steps as a journey:

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "직전 안정 배포 찾기",          status: "in_progress", activeForm: "이전 succeeded deploy 찾는 중" },
     { content: "이전 배포 정보 정리",        status: "pending",     activeForm: "commit 메타데이터 모으는 중" },
     { content: "rollback 카드 보여드리기",      status: "pending",     activeForm: "rollback 카드 준비하는 중" },
     { content: "동의 받고 forward-fix 시작",    status: "pending",     activeForm: "forward-fix 시작하는 중" },
     { content: "재배포 진행 보기",             status: "pending",     activeForm: "재배포 모니터하는 중" },
     { content: "결과 안내",                    status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Read last-known-good from local cache.** The deployments cache holds `(deployment_id → app_id → commit_sha → status)` per machine:

   ```bash
   cat ~/.config/axhub/deployments.json
   ```

   Find the most recent entry where `status == "succeeded"` for the current app. If the cache is cold, fall back to:

   ```bash
   "$HELPER" list-deployments --app <APP_ID> --limit 10
   ```

   helper 는 current `axhub deploy list --app <APP> --json` 을 감싼 CLI wrapper 예요. On exit 65 (`list-deployments` helper 의 EXIT_LIST_AUTH OUTPUT 계약 — classify-exit 가 4 로 정규화해요; token missing — Phase 7 US-701 이후엔 SessionStart 자동 setup):
   > "토큰을 찾을 수 없어요. 'axhub auth login' 또는 CC 세션 재시작."

   Filter to `status == "succeeded"` and pick the second-most-recent (the most recent succeeded ≠ current state because the user wants to back out).

2. **Resolve the prior commit context.** For the chosen deploy, surface:
   - `commit_sha` (short)
   - `commit_message` (first line)
   - `created_at` (humanized: "어제 14:30", "30분 전")
   - `deployment_id` of the original deploy

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — rollback 확인 → `abort` (destructive 작업이라 subprocess 자동 진행 안 해요), pick_other → `abort`.

3. **Render transparent confirmation card.** AskUserQuestion:

   ```
   직전 안정 배포로 되돌릴게요 (forward-fix 방식):
     · 앱:    <APP_SLUG> (id=<APP_ID>)
     · 커밋:  <PREV_SHA> — "<COMMIT_MSG>" (<RELATIVE_TIME>)
     · 원래 배포: <DEPLOY_ID>

   ⚠️ 진짜 rollback이 아니에요. 이전 커밋을 새로 배포하는 거예요.
      이전 배포는 그대로 기록에 남고, 새 배포 한 건이 추가됩니다.

   진행할까요?
   ```

   Options:

   ```json
   {
     "question": "직전에 잘 됐던 버전으로 다시 올릴까요?",
     "header": "rollback 확인",
     "options": [
       {"label": "네, 직전 커밋으로", "value": "confirm", "description": "<PREV_SHA>를 새로 배포"},
       {"label": "다른 커밋 고르기", "value": "pick_other", "description": "최근 succeeded 배포 목록에서 선택"},
       {"label": "취소", "value": "abort", "description": "아무것도 안 함"}
     ]
   }
   ```

4. **On `confirm`.** Run deploy create with the prior commit:

   ```bash

   axhub deploy create --app "$APP_ID" --commit "$PREV_SHA" --execute --json
   ```

5. **Auto-watch.** Capture the new `dep_<id>` and route to `skills/status` for narrated tracking. Do not block on completion — the user wanted the recovery action; status is a courtesy follow-up.

6. **On `pick_other`.** Surface the last 5 succeeded deploys from `${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers list-deployments --app "$APP_ID" --limit 5` via AskUserQuestion. Repeat from step 3 with the chosen commit.

7. **On non-zero exit from create**, route to `../deploy/references/error-empathy-catalog.md`. The `validation.deployment_in_progress` case is especially relevant here (user might recover during another deploy) — follow `../deploy/references/recovery-flows.md` ("deployment_in_progress") and offer to watch the in-flight deploy first.

   **canonical helper exit-code → user-facing route map** (PR #149 / review #5). 이 표는 axhub-helpers 가 emit 하는 `error_code` 별 다음 행동을 정의해요. status/verify/trace 스킬도 같은 표를 참조해요 (이 SKILL 이 정본; 다른 SKILL 은 cross-link).

   | `error_code` | helper exit | 사용자 안내 |
   |---|---|---|
   | `token_invalid` | 65 | "다시 로그인해줘"라고 말하면 재인증할 수 있다고 안내해요. 4-part empathy 템플릿 참조. |
   | `not_found` | 67 | did-you-mean 으로 가까운 슬러그 제시 + `apps` 스킬로 라우팅해요. |
   | `validation.app_id_invalid` | 1 | helper 가 argv 형태로 거부한 케이스. 정상 슬러그 형식 (`[A-Za-z0-9_-]{1,64}`) 으로 다시 받아요. |
   | `transport.timeout` | 1 | 일시적 hang. 재시도 1회 + 그래도 실패면 네트워크 / CLI 버전 확인을 안내해요. |
   | `transport.cli_missing` | 1 | `axhub` 바이너리가 PATH 에 없거나 실행 불가. 사용자에게 `axhub --version` 으로 확인하고, 안되면 "설치 도와줘" 또는 "처음 쓰는데 뭐부터 하면 돼?"라고 말하면 재설치를 도울 수 있다고 안내해요. |
   | `response.invalid_json` | 1 | CLI 가 exit 0 인데 stdout 이 JSON 이 아닌 경우. CLI 버전 mismatch 가능성 — "업데이트 확인해줘" 또는 "설치 상태 진단해줘"라고 말하면 점검할 수 있다고 안내해요. |
   | `response.error_envelope_unknown_shape` | 1 | CLI 가 알수없는 error envelope 모양으로 응답. CLI 가 helper 보다 최신일 가능성 — "플러그인 최신인지 봐줘"라고 말하면 점검할 수 있다고 안내해요. |
   | `cli.exit_<N>` | 1 | catch-all (signal kill 등). retry 1회 + 그래도 실패면 "설치 상태 진단해줘"라고 말하면 진단할 수 있다고 안내해요. |

8. **No prior succeeded deploy found.** Surface: "되돌릴 수 있는 직전 안정 배포를 못 찾았어요. 이 앱의 첫 배포이거나, 모든 이전 배포가 실패한 상태일 수 있어요. 'logs'로 현재 배포 원인 먼저 볼래요?"

9. **Cache last-deploy for statusline (Phase 17 US-1707).** After Step 5 terminal status, write the recovery summary so statusline readers can show it across sessions. The Bash block below is for POSIX/Git Bash/WSL tool execution; native Windows statusLine wiring must use the documented helper/PowerShell path only after the Windows packaging spike promotes it:

   ```bash
   mkdir -p ~/.cache/axhub-plugin
   cat > ~/.cache/axhub-plugin/last-deploy.json <<JSON
   {"deployment_id":"$NEW_DEPLOY_ID","status":"$TERMINAL_STATUS","commit_sha":"$PREV_SHA","app_slug":"$APP_SLUG","timestamp":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","kind":"recover"}
   JSON
   ```

## NEVER

- NEVER claim this is a real rollback — always say "forward-fix" / "직전 커밋 재배포" in the confirmation card.
- NEVER skip the AskUserQuestion confirmation — destructive op needs explicit yes.
- NEVER auto-pick the most-recent succeeded deploy without showing the candidate to the user (commit_sha + commit_message in the card).
- NEVER drop `--json` from `axhub deploy create`.

## Additional Resources

For Korean trigger lexicon (recover/rollback intent): `../deploy/references/nl-lexicon.md` (section 10).
For 4-part Korean exit templates: `../deploy/references/error-empathy-catalog.md`.
For deployment_in_progress flow during recovery: `../deploy/references/recovery-flows.md` ("deployment_in_progress").
