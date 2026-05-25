---
name: insight
description: '이 스킬은 axhub gateway query 로 실데이터에 집계 SQL 을 돌려 데이터 인사이트를 뽑을 때 사용해요. catalog 로 connector-id 와 path 를 먼저 해석한 뒤 SELECT 집계 쿼리를 실행하고 결과를 한국어 인사이트로 요약해요. 다음 표현에서 활성화: "인사이트 뽑아줘", "인사이트 뽑아봐", "데이터 분석해줘", "분석해줘", "집계해줘", "통계 내줘", "평균 구해줘", "합계 내줘", "이 데이터로 뭐 알 수 있어", "트렌드 봐줘", "분포 알려줘", "gateway query", "group by", "aggregate", "analytics".'
examples:
  - utterance: "이 데이터로 인사이트 뽑아봐"
    intent: "run aggregate SQL via axhub gateway query and narrate insights"
  - utterance: "employees 테이블 부서별 인원 분석해줘"
    intent: "run aggregate SQL via axhub gateway query and narrate insights"
  - utterance: "월별 주문 추이 집계해줘"
    intent: "run aggregate SQL via axhub gateway query and narrate insights"
  - utterance: "run a GROUP BY aggregate on this connector via gateway query"
    intent: "run aggregate SQL via axhub gateway query and narrate insights"
  - utterance: "show me analytics insights from the employees table"
    intent: "run aggregate SQL via axhub gateway query and narrate insights"
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
---

# Insight (gateway query 집계 → 인사이트 서술)

실데이터에서 인사이트를 뽑을 때는 `axhub data list` 로 헤매지 말고, catalog 로 connector-id 와 path 를 해석한 뒤 `axhub gateway query` 로 SELECT 집계 SQL 을 한 번에 돌려서 결과를 한국어 인사이트로 요약해줘요.

## Workflow

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
     { content: "connector-id 와 path 해석",  status: "in_progress", activeForm: "connector-id 와 path 해석 중" },
     { content: "접근 가능 컬럼 확인",          status: "pending",     activeForm: "허용 컬럼 확인 중" },
     { content: "집계 쿼리 실행 동의 확인",     status: "pending",     activeForm: "실행 동의 확인 중" },
     { content: "gateway query 집계 실행",      status: "pending",     activeForm: "집계 쿼리 실행 중" },
     { content: "인사이트 요약 안내",           status: "pending",     activeForm: "인사이트 요약하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

1. **connector-id 와 path 를 catalog 로 해석해요.** `axhub gateway query` 는 `--connector-id` (UUID) 와 `--path` 를 요구해요. 추측하지 말고 catalog 출력에서만 가져와요.

   ```bash
   axhub catalog connectors --json
   axhub catalog resources --json --limit 200
   ```

   - `catalog connectors` 결과의 `id` 가 `--connector-id` 에 들어갈 UUID 예요. 사용자가 커넥터를 지목하지 않았으면 활성 커넥터를 보여주고 어느 것을 볼지 확인해요.
   - `catalog resources` (또는 `catalog search`) 결과에서 사용자가 말한 테이블/리소스에 맞는 `path` 를 골라요 (예: `employees`, `orders`).
   - 후보가 여러 개면 connector + path 후보를 짧게 보여주고 하나를 확정해요. 후보가 하나뿐이면 그대로 진행해요.

2. **접근 가능 컬럼을 확인해요.** 집계 SQL 을 짜기 전에 어떤 컬럼이 열려 있고 무엇이 마스킹/차단인지 확인해요.

   ```bash
   axhub catalog get --connector <connector-id> --path <path> --json
   ```

   `allowed_columns`, masked 컬럼, row policy, `deny_reason` 만 요약해요. SELECT 집계는 **열려 있는 컬럼으로만** 작성해요. 마스킹된 컬럼(예: salary, email)은 의미 있는 집계가 안 되니 그 컬럼 기준 집계는 피하고, 사용자가 원하면 마스킹 때문에 불가하다고 안내해요.

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — 집계 쿼리 실행 동의 → `dry_run` (subprocess 에서는 실데이터를 실행하지 않고 SQL 만 보여줘요).

3. **집계 쿼리 실행 동의를 받아요.** `axhub gateway query --execute` 는 실데이터를 읽는 호출이라, 실행 전에 connector-id, path, SELECT SQL, row limit, 사용할 allowed_columns 를 한 번 보여주고 동의를 받아요.

   ```json
   {
     "questions": [{
       "question": "실데이터에 이 집계 쿼리를 실행할까요?",
       "header": "집계 쿼리",
       "multiSelect": false,
       "options": [
         {"label": "Dry-run only", "description": "실데이터를 읽지 않고 SQL, allowed_columns, row limit 만 보여줘요."},
         {"label": "Run query", "description": "표시한 connector-id/path/SQL/row limit 그대로 한 번만 실행해요."}
       ]
     }]
   }
   ```

   비대화형 모드에서는 `Dry-run only` 로 진행해요. 서버가 `allowed:false` 나 `deny_reason` 으로 거부하면 그 이유를 보여주고 절대 재시도하지 않아요.

4. **gateway query 로 집계를 실행해요.** SELECT 집계 SQL 만 써요 (gateway query 는 기본 SELECT-only 라 `--allow-non-select` 는 절대 안 써요). row limit 은 명시하고, 동의가 있을 때만 `--execute` 를 붙여요.

   ```bash
   # dry-run (기본): SQL guard 만 검증, 백엔드 호출 없음
   axhub gateway query --connector-id <connector-id> --path <path> \
     --sql 'SELECT department, COUNT(*) AS headcount FROM employees GROUP BY department ORDER BY headcount DESC' \
     --row-limit 100

   # 동의 후 실행
   axhub gateway query --connector-id <connector-id> --path <path> \
     --sql 'SELECT department, COUNT(*) AS headcount FROM employees GROUP BY department ORDER BY headcount DESC' \
     --row-limit 100 --execute --json
   ```

   `--execute` 가 없으면 dry-run 이라 SQL guard 통과만 확인하고 끝나요. 실제 인사이트는 동의 후 `--execute --json` 으로 받은 결과 행에서 나와요. 오류가 나면 exit code 로 라우팅해요 (65 → `/axhub:auth` 재로그인, 67 → connector/path 다시 확인, 68 → Retry-After 백오프, `allowed:false`/`deny_reason` → 재시도 금지).

5. **인사이트를 한국어로 요약해요.** raw JSON 을 그대로 쏟지 말고 vibe coder 가 바로 이해할 형태로 정리해요:

   - 핵심 수치를 GFM 마크다운 표로 (예: 부서 / 인원, 또는 월 / 합계). 셀이 길면 ~50자에서 잘라요.
   - 표 아래에 1~3문장으로 가장 큰 값, 눈에 띄는 편중/추세, 빈 그룹 같은 발견을 짚어줘요.
   - 마지막에 실행한 connector-id, path, SQL, row limit, 그리고 실데이터 실행 여부(실행 / dry-run)를 한 줄로 남겨요. dry-run 만 했으면 "실데이터는 안 읽었어요" 라고 분명히 적어요.

   **워크플로를 마치면 (마지막 인사이트 요약 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** 종료 시점에 미완료 todo 가 0 개여야 해요.

## NEVER

- NEVER `axhub data list` 로 인사이트를 뽑으려고 헤매기 — connector-id/path 는 catalog 로 해석하고 집계는 `axhub gateway query` 로 해요.
- NEVER connector-id 나 path 를 추측 — `catalog connectors` / `catalog resources` 출력에서만 가져와요.
- NEVER `--allow-non-select` 사용 — 인사이트는 읽기라 SELECT 집계만 해요. INSERT/UPDATE/DELETE 금지.
- NEVER `--execute` 를 동의 없이 (대화형에서) 실행 — 비대화형 기본값은 dry-run 이에요.
- NEVER 마스킹/차단 컬럼으로 집계 강행 — `allowed:false` / `deny_reason` / 66 / catalog 오류가 나오면 멈추고 이유를 보여줘요. 거부된 쿼리는 재시도하지 않아요.
- NEVER row limit 을 빼거나 `allowed_columns` 를 넘어선 컬럼으로 쿼리.
- NEVER raw NDJSON/JSON 결과를 그대로 출력 — 한국어 인사이트 요약으로 humanize 해요.

## Additional Resources

- `../deploy/references/nl-lexicon.md` — 활성화 trigger 어구 추가 시 참조.
- `../deploy/references/error-empathy-catalog.generated.md` — generated exit-code 공감 카피.
- 단순 조회 / 테이블 describe / read snippet 생성은 `/axhub:data` (catalog invoke 기반) 를 써요. insight 는 gateway query 집계 전용이에요.
