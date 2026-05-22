---
name: inventory
description: '이 스킬은 사용자가 본인이 접근 가능한 axhub 리소스 전체 인벤토리를 한눈에 보고 싶을 때 사용해요. 다음 표현에서 활성화: "내 리소스", "내 리소스 봐", "내 리소스 보여", "내 리소스 보여줘", "내 리소스 목록", "리소스 봐", "리소스 보여", "리소스 보여줘", "리소스 목록", "리소스 뭐 있어", "리소스 조회", "뭐 접근 가능", "뭐 접근 가능해", "내가 뭐 봐", "내가 뭐 봐", "내 자산", "내 자산 봐", "내 스코프", "스코프 봐", "권한 봐", "권한 뭐 있어", "접근 가능한 거", "접근 권한", "접근 권한 봐", "쓸 수 있는 거", "available", "inventory", "list resources", "my resources", "my access", "my scope", "what can I access", "what do I have", "what i have access to", "access", "resources", "show my resources", 또는 사용자 scope 의 통합 리소스 카탈로그 조회. team scope 필터로 cross-tenant 데이터 노출 차단해요. 7 family (tenants / apps / members / engines / connectors / resources / catalog kinds) 를 병렬 호출해서 한 응답에 compact 한국어 요약 + drill-down hint 로 렌더해요.'
examples:
  - utterance: "내 리소스 보여줘"
    intent: "list accessible axhub resources"
  - utterance: "뭐 접근 가능해"
    intent: "list accessible axhub resources"
  - utterance: "what can I access"
    intent: "list accessible axhub resources"
  - utterance: "내 스코프 봐"
    intent: "list accessible axhub resources"
  - utterance: "inventory"
    intent: "list accessible axhub resources"
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: haiku
---

# Resource Inventory

사용자가 접근 가능한 axhub 리소스를 7개 family (tenants / apps / members / engines / connectors / resources / catalog kinds) 로 한 번에 조회해서 compact 한국어 요약으로 렌더해요. 읽기 전용, mutation 경로 없음, F4 privacy 로 cross-tenant 데이터 차단해요.

## Workflow

To list resources:

!`node -e "const cp=require('child_process');const env={...process.env};const helper='${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers';const result=cp.spawnSync(helper,['preflight','--json'],{stdio:['inherit','pipe','pipe'],env});const stdoutText=String(result.stdout??'');const stderrText=String(result.stderr??'');if(stdoutText.length>0){process.stdout.write(stdoutText);}const denialRegex=/^(?:Shell|Bash) command permission check failed.*requires approval/im;const cliNotFoundRegex=/\"auth_error_code\":\"cli_not_found\"/;const cliConfigCorruptedRegex=/\"auth_error_code\":\"cli_config_corrupted\"/;const cliRuntimeErrorRegex=/\"auth_error_code\":\"cli_runtime_error\"/;const cliUnavailableRegex=/\"auth_error_code\":\"cli_unavailable\"/;const redactRe=/(sk-[A-Za-z0-9_-]{20,}|github_pat_[A-Za-z0-9_]{20,}|gho_[A-Za-z0-9]{36}|axhub_[A-Za-z0-9]{32,}|Bearer\\s+[A-Za-z0-9._~+\\/-]+=*)/g;if(result.error||(result.status!==0&&denialRegex.test(stderrText))){console.log(JSON.stringify({systemMessage:\"[axhub] 첫 실행이라 권한이 필요해요. Claude Code 가 'axhub-helpers preflight' 실행 허용을 묻는 prompt 가 떠요. '허용' 을 누르면 다음부터 자동으로 진행돼요. (한 번만 진행하면 돼요)\"}));process.exit(0)}else if(result.status!==0&&cliNotFoundRegex.test(stdoutText)){console.log(JSON.stringify({systemMessage:\"[axhub] axhub CLI 가 PATH 에서 안 보여요. /axhub:install-cli 로 설치하거나 (macOS Apple Silicon Homebrew 사용 중이면 /opt/homebrew/bin 이 inherit 안 됐을 가능성). /axhub:doctor 로 진단 가능해요.\"}));process.exit(0)}else if(result.status!==0&&cliConfigCorruptedRegex.test(stdoutText)){console.log(JSON.stringify({systemMessage:\"[axhub] axhub CLI 의 ~/.config/axhub/config.yaml 이 새 schema 와 안 맞아요 (예: user_id 가 UUID/int64 mismatch). /axhub:auth 로 재로그인하면 fresh config 가 작성되면서 자동 fix 돼요.\"}));process.exit(0)}else if(result.status!==0&&cliRuntimeErrorRegex.test(stdoutText)){console.log(JSON.stringify({systemMessage:\"[axhub] axhub CLI 가 실행은 됐지만 비정상 exit 했어요. /axhub:doctor 로 진단해주세요. CLI 자체 버그면 GitHub issue 로 stderr 한 줄 같이 알려주세요.\"}));process.exit(0)}else if(result.status!==0&&cliUnavailableRegex.test(stdoutText)){console.log(JSON.stringify({systemMessage:\"[axhub] axhub CLI 가 감지 안 돼요. /axhub:install-cli 로 OS 별 공식 설치 채널을 안내받거나 /axhub:doctor 로 진단해주세요. (SKILL 흐름은 그대로 진행할 수 있어요 — preflight 가 cli_unavailable 만 알려준 거예요.)\"}));process.exit(0)}else if(stderrText.length>0){process.stderr.write(stderrText.replace(redactRe,'<redacted>'))}process.exit(typeof result.status==='number'?result.status:0)"`

이 줄은 Claude Code SKILL preprocessing 으로 워크플로 시작 전에 실행돼요. 출력 (auth_status, current_team_id, current_app, current_env) 이 컨텍스트에 자동 주입돼서 인증/team scope 추출이 별도 호출 없이 끝나요.

1. **인증/Scope 확정.** preflight 결과에서 `auth_ok` 와 `current_team_id` 를 추출해요. 미인증이면 즉시 안내 후 종료:

   ```bash
   AUTH_OK=$(echo "$PREFLIGHT_JSON" | jq -r '.auth_ok // false')
   TEAM_ID=$(echo "$PREFLIGHT_JSON" | jq -r '.current_team_id // empty')
   if [ "$AUTH_OK" != "true" ]; then
     echo '{"systemMessage":"로그인이 필요해요. /axhub:auth 로 로그인하고 다시 호출해주세요."}'
     exit 0
   fi
   ```

2. **7 family 병렬 조회 (fail-soft, 격리 tmp dir).** 각 family 를 백그라운드로 띄우고 wait 로 동기화. tmp dir 은 `mktemp` 로 격리해서 동시 호출 race 차단해요:

   ```bash
   INV_TMP=$(mktemp -d -t axhub-inv-XXXX)
   trap "rm -rf '$INV_TMP'" EXIT

   ( axhub tenants list --json    >"$INV_TMP/tenants.json"    2>"$INV_TMP/tenants.err"    ; echo $? >"$INV_TMP/tenants.code"    ) &
   ( axhub apps mine --json       >"$INV_TMP/apps.json"       2>"$INV_TMP/apps.err"       ; echo $? >"$INV_TMP/apps.code"       ) &
   ( axhub members list --json    >"$INV_TMP/members.json"    2>"$INV_TMP/members.err"    ; echo $? >"$INV_TMP/members.code"    ) &
   ( axhub engines list --json    >"$INV_TMP/engines.json"    2>"$INV_TMP/engines.err"    ; echo $? >"$INV_TMP/engines.code"    ) &
   ( axhub connectors list --json >"$INV_TMP/connectors.json" 2>"$INV_TMP/connectors.err" ; echo $? >"$INV_TMP/connectors.code" ) &
   ( axhub resources list --json  >"$INV_TMP/resources.json"  2>"$INV_TMP/resources.err"  ; echo $? >"$INV_TMP/resources.code"  ) &
   ( axhub catalog kinds --json   >"$INV_TMP/catalog.json"    2>"$INV_TMP/catalog.err"    ; echo $? >"$INV_TMP/catalog.code"    ) &
   wait
   ```

3. **Per-family 그레이스 핸들링.** 각 `.code` 파일 검사. `0` 이면 count + top3 추출. `1`/`64`/`65`/`67`/`68` 등은 다음 표로 한 줄 표기:

   | exit | 의미 | 표시 |
   |---|---|---|
   | `0` | 정상 | `<count>개 — <top3>` |
   | `65` | 미인증 | `(미인증 — /axhub:auth 로 로그인)` |
   | `67` | admin 권한 부족 (PAT-only) | `(관리자 인증 필요 — /axhub:auth login 으로 OAuth 재인증)` |
   | `68` | scope 외 | `(scope 외)` |
   | 그 외 | 기타 오류 | `(조회 불가 — exit N)` |

   카탈로그/엔진/리소스/커넥터는 backend AGENTS.md known limitation 으로 PAT 만 있으면 401 가능 → 친절 안내 출력.

4. **F4 privacy 필터.** 모든 family 응답에서 `team_id != $TEAM_ID` 항목 drop. tenants 자체는 사용자 멤버십 기준이라 filter 면제.

5. **한국어 compact 렌더 (해요체).** 다음 형태로 한 응답에 출력해요:

   ```
   접근 가능 리소스 (team=<slug>):

   ▸ Identity
     • 팀(tenants): 2개  → <slug1>, <slug2>
     • 앱(apps): 5개      → paydrop, checkout, mobile (+2)
     • 멤버(members): 8명 (admin 2 / member 6)

   ▸ Gateway
     • Engines: 4 — postgres, bigquery, snowflake, redshift
     • Connectors: 3 — bigquery-prod, postgres-prod, snowflake
     • Resources: 47 (page 1) — top3: hr.employees, sales.orders, finance.ledger
     • Catalog kinds: 12 — table, view, function, stream, …

   ▸ 앱별 자원 (drill-down)
     • 환경(env) / 테이블(tables) / API(apis) 는 앱 단위라 /axhub:apps 로 앱 고른 뒤 각 SKILL 호출해주세요.

   자세히: /axhub:apps · /axhub:env · /axhub:github · /axhub:deploy
   ```

   각 family 가 0개면 `0개` 로 표기하고 줄 유지. 7개 모두 실패면 종합 안내 한 줄 출력 후 종료.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 은 대화형 질문 prompt 를 호출하지 않아요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 와 대화형 환경 모두에서 동일하게 동작해요. `tests/fixtures/ask-defaults/registry.json` 의 inventory 항목은 no-op stub (질문 없음).

6. **렌더 종료.** Step 5 의 한국어 요약 출력 후 trap 이 tmp dir cleanup. `exit 0`.

## NEVER

- NEVER 한 family 실패로 전체 인벤토리 abort. 항상 7 family 모두 한 응답에 렌더 (성공/실패 혼합).
- NEVER cross-tenant 데이터 (team_id != $TEAM_ID) 화면에 표시. F4 privacy 위반.
- NEVER mutation 호출 (`create` / `update` / `delete` / `set-role` / `bulk-register`) 진입. 이 SKILL 은 read-only.
- NEVER per-app drill (`env list --app X` / `tables list --app X`) 자동 진행. 사용자가 명시적으로 앱 선택 후 다른 SKILL 로 위임.
- NEVER tmp dir cleanup 누락. `trap "rm -rf '$INV_TMP'" EXIT` 강제.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.md` — exit-code 별 4-part 한국어 fallback.
- `../apps/SKILL.md` — apps drill-down (앱 단일 family 상세 + 생성/삭제).
- `../status/SKILL.md` — 배포 상태 추적 (haiku read-only 동일 패턴 참고).
