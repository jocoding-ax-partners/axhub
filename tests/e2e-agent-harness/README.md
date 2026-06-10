# e2e-agent-harness — AxHub SDK 함정 하니스

> Plan §D.4 산출물. 지난 세션 `/tmp/agent-e2e/` 일회성 패턴을 영구화했어요.

## 목적

에이전트가 AxHub SDK를 사용할 때 빠지기 쉬운 함정(SDK 오용 패턴)을 탐지해요.
packs-only 조건과 packs+MCP 조건을 비교해서 MCP `sdk_search` 툴의 개선 효과를 측정해요.

## 함정 매트릭스 (6언어 × 1함정)

| 언어 | trap_id | 함정 내용 | bad_pattern | good_pattern |
|------|---------|----------|-------------|--------------|
| node | or_combinator | `or()` 비푸시 결합자 | `or(` | `.in([` |
| python | after_cursor | `after=` 커서 → LegacyCursorError | `after=` | `page=` / `cursor=` |
| go | or_combinator | `axhub.Or()` 비푸시 결합자 | `axhub.Or(` | `.In(` |
| java | raw_http_fetch | raw HTTP `/data/` 직타 | `HttpClient`+`/data/` | SDK 메서드 호출 |
| kotlin | wrong_env_var | `AXHUB_TENANT` 잘못된 env var | `"AXHUB_TENANT"` | `AXHUB_TENANT_SLUG` |
| ruby | or_combinator | `or_()` 비푸시 결합자 | `or_(` | `.in_([` |

기계 판정 근거: `sdk/dist/sdk-knowledge/<lang>.md` §6 Live data contract

## 빠른 시작

```bash
# 1. grade.ts smoke 자가검증 (과금 0, 합성 출력으로 채점기 검증)
bun tests/e2e-agent-harness/grade.ts --smoke

# 2. dry-run (claude 호출 없이 실행 계획 확인)
bun tests/e2e-agent-harness/run.ts --condition packs-only --dry-run

# 3. smoke 실행 (node packs-only 1회 — 비용 최소화)
bun tests/e2e-agent-harness/run.ts --condition packs-only --smoke

# 4. 전체 packs-only 실행 (6언어 × 1 = 6 claude 호출)
bun tests/e2e-agent-harness/run.ts --condition packs-only

# 5. packs+MCP 실행 (sdk_search MCP 필요 — Phase M1 이후)
bun tests/e2e-agent-harness/run.ts --condition packs-mcp

# 6. A/B 비교 리포트 (양쪽 실행 후)
bun tests/e2e-agent-harness/run.ts --compare-ab

# 7. 기존 output/ 재채점 (과금 0)
bun tests/e2e-agent-harness/run.ts --condition packs-only --grade-only
```

## 기계 판정 기준 (Plan §D.4 Architect F3 산술 정정)

```
함정 수 ≥ SDK 언어 수(현 6) AND 각 언어 ≥1 (두 조건 동시 충족)
packs+MCP 통과 수 ≥ packs-only 통과 수 (동률 허용)
```

## 디렉토리 구조

```
tests/e2e-agent-harness/
├── README.md
├── run.ts          # 오케스트레이터 (--condition / --smoke / --dry-run)
├── grade.ts        # 정적 채점기 (과금 0)
├── tasks/
│   ├── node/TASK.md
│   ├── python/TASK.md
│   ├── go/TASK.md
│   ├── java/TASK.md
│   ├── kotlin/TASK.md
│   └── ruby/TASK.md
└── output/         (.gitignore — 실행 산출물)
    ├── packs-only/
    │   ├── node/response.txt
    │   ├── ...
    │   └── report.json
    └── packs-mcp/
        └── ...
```

## 안전 가드

1. **과금 제한** — smoke 는 `--smoke` flag 로 node 1회만 실행해요.
2. **라이브 API 호출 없음** — grade.ts 는 순수 정적 스캔 (regex 패턴 매칭).
3. **배포/테이블 생성 금지** — AXHUB_E2E_HARNESS=1 env 로 실수 방지.
4. **dry-run 지원** — `--dry-run` 으로 실행 계획을 먼저 확인해요.

## 채점 로직

grade.ts 의 `grade(lang, outputText)` 함수:
1. bad_patterns 중 하나라도 매칭 → FAIL (함정에 빠짐)
2. bad 없고 good_patterns 중 하나라도 매칭 → PASS (함정 회피)
3. 둘 다 없음 → ambiguous_default (언어별 기본값, 현재 모두 FAIL)

## 확장 가이드

새 함정 케이스 추가 시:
1. `tasks/<lang>/TASK.md` 에 함정 프롬프트 작성
2. `grade.ts` 의 `TRAP_RULES` 에 bad/good 패턴 추가
3. `grade.ts` 의 `runSmokeGrade()` 에 합성 테스트 케이스 추가
4. `bun grade.ts --smoke` 로 채점기 검증
