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
| kotlin | filterless_list | non-owner 테이블 무필터 list/count | `.list(ListOptions.create())` | `.where(` 포함 |
| ruby | or_combinator | `or_()` 비푸시 결합자 | `or_(` | `.in_([` |

기계 판정 근거: `sdk/dist/sdk-knowledge/<lang>.md` §6 Live data contract

## 빠른 시작

```bash
# 0. 채점기 회귀 테스트 (bun test 자동 수집 — fixtures.ts 케이스 + 게이트 산술 + frontmatter strip)
bun test tests/e2e-agent-harness/grade.test.ts

# 1. grade.ts smoke 자가검증 (과금 0, 합성 출력으로 채점기 검증 — 같은 fixtures.ts 사용)
bun tests/e2e-agent-harness/grade.ts --smoke

# 2. dry-run (claude 호출 없이 실행 계획 확인 — pack 부재 시 경고만)
bun tests/e2e-agent-harness/run.ts --condition packs-only --dry-run

# 3. smoke 실행 (node packs-only 1회 — 비용 최소화)
bun tests/e2e-agent-harness/run.ts --condition packs-only --smoke

# 4. 전체 packs-only 실행 (6언어 × 1 = 6 claude 호출)
bun tests/e2e-agent-harness/run.ts --condition packs-only

# 5. packs+MCP 실행 (sdk_search MCP 필요 — --mcp-config 필수, 실행 전 헬스체크 hard-fail)
bun tests/e2e-agent-harness/run.ts --condition packs-mcp \
  --mcp-config tests/e2e-agent-harness/mcp-config.json

# 6. A/B 비교 리포트 (양쪽 실행 후)
bun tests/e2e-agent-harness/run.ts --compare-ab

# 7. 기존 output/ 재채점 (과금 0)
bun tests/e2e-agent-harness/run.ts --condition packs-only --grade-only
```

### 환경 변수

| 변수 | 기본값 | 용도 |
|------|--------|------|
| `AXHUB_SDK_PACKS_DIR` | `<repo>/../sdk/dist/sdk-knowledge` | SDK pack 디렉토리 override |
| `AXHUB_E2E_MCP_URL` | mcp-config.json 의 url | MCP 서버 URL/포트 override (effective config 자동 생성) |

## 기계 판정 기준 (Plan §D.4 Architect F3 산술 정정)

```
함정 수 ≥ SDK 언어 수(현 6) AND 각 언어 ≥1 (두 조건 동시 충족)
  — UNCERTAIN (실행 실패/출력 없음) 은 측정된 함정이 아니므로 집계에서 제외
packs+MCP 통과 수 ≥ packs-only 통과 수 (동률 허용)
게이트 PASS = A/B 비교 통과 AND 양 조건 기준 충족 AND 양 조건 UNCERTAIN 0건
```

## 측정 유효성 가드

- **함정 정답 비누출** — TASK.md 의 frontmatter (trap_id/trap_kind 등 채점 메타) 는
  프롬프트 파이프 전에 strip 돼요. 에이전트가 "스스로" 함정을 피하는지가 측정 대상이에요.
- **A/B 조건 대칭** — 양 조건 동일 `--allowedTools` 베이스 + `--strict-mcp-config`
  (전역 MCP 차단). 차이는 mcp-config + `sdk_search` 허용 여부뿐이에요.
- **MCP 헬스체크** — packs-mcp 라이브 실행 전 fetch 로 서버 도달성을 확인하고
  도달 불가면 hard-fail 해요 (서버가 죽어 있으면 B 조건이 A 조건으로 측정되는 것 방지).
- **실패 런 미기록** — claude 비정상 종료 시 response.txt 를 기록하지 않고 stale 파일도
  제거해요. 재채점 시 missing → UNCERTAIN 으로 떨어져 게이트가 막아요.

## 디렉토리 구조

```
tests/e2e-agent-harness/
├── README.md
├── run.ts          # 오케스트레이터 (--condition / --smoke / --dry-run)
├── grade.ts        # 정적 채점기 (과금 0)
├── fixtures.ts     # 합성 채점 케이스 단일 소스 (--smoke + bun test 공유)
├── grade.test.ts   # bun test 회귀 테스트 (자동 수집)
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
0. 빈 출력 → UNCERTAIN (FAIL 둔갑 금지 — 게이트에서 차단)
1. 코드 펜스(``` / ~~~) 내부만 스캔 (미종결 펜스는 EOF 까지 블록으로 간주),
   주석 제거 (줄 시작 `#` `//` `--` `*`, 블록 주석 `/* */` 상태 추적,
   trailing inline 주석 — `://` 프로토콜과 루비 `#{` 보간은 보존)
2. bad_patterns 중 하나라도 매칭 → FAIL (함정에 빠짐)
3. bad 없고 good_patterns 중 하나라도 매칭 → PASS (함정 회피)
4. 둘 다 없음 → ambiguous_default (언어별 기본값, 현재 모두 FAIL)

### java prose 거절 정책

java 함정(raw HTTP `/data/` 직타 요청)에 한해, 응답에 **코드 펜스가 전혀 없고**
good 패턴("use the SDK" / `DataTableClient` 류)이 매칭되면 bad 매칭을 무효화하고
PASS 로 판정해요.

근거: 올바른 거절 응답은 "HttpClient 로 `/data/` 를 직접 치지 말고 SDK 를 쓰라" 는
prose 인데, 이 문장 자체가 bad 패턴(`HttpClient` + `/data/`)에 걸려 FAIL 로 둔갑하는
것을 막기 위함이에요. 코드 펜스가 있으면 코드가 판정 기준이므로 이 정책은 적용되지
않아요 (펜스 안 진짜 bad 코드는 그대로 FAIL).

## 확장 가이드

새 함정 케이스 추가 시:
1. `tasks/<lang>/TASK.md` 에 함정 프롬프트 작성 (frontmatter 의 채점 메타는 실행 시 자동 strip)
2. `grade.ts` 의 `TRAP_RULES` 에 bad/good 패턴 추가
3. `fixtures.ts` 의 `SMOKE_CASES` 에 합성 케이스 추가 — **PASS 방향만 추가하지 말고
   FAIL 방향(펜스 안 진짜 bad → FAIL) 케이스를 함께 추가**해서 채점기 완화를 방지해요
4. `bun test tests/e2e-agent-harness/grade.test.ts` + `bun grade.ts --smoke` 로 검증
