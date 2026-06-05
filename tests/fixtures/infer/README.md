# infer-tables-env fixtures

`infer-tables-env` SKILL 의 분석·추천을 평가하는 골든 fixture 예요. 각 디렉토리는 샘플 앱 소스 + `expected-recommendation.md`(기대 추천 골든)로 이뤄져요.

SKILL 은 결정론 엔진이 아니라 LLM 오케스트레이션(research D1)이라, 이 골든은 유닛테스트가 아니라 **사람·e2e 평가 기준**이에요. spec Success Criteria 와 매핑:

| Fixture | 검증 대상 |
|---|---|
| `nextjs-prisma/` | SC-001 recall — schema.prisma + .env.example → 테이블(컬럼·타입·제약) + env 키 |
| `fastapi-sqlmodel/` | SC-001 + 코드-only 모델은 "검토 필요"(recall 보장 제외) |
| `secret-hardcoded/` | SC-005 — 하드코딩 시크릿 발견·플래그, **값 비노출**(골든에 시크릿 값 없음) |
| `already-configured/` | SC-007 멱등 — 이미 있는 테이블·env 는 "이미 있음", 재적용 0 변경 |

## e2e matrix 배선 (deferred)

`tests/e2e/claude-cli/` 매트릭스에 분석/적용/휘발성 시나리오(T017/T024/T035)를 거는 건 `claude -p` + mock-hub staging 환경이 필요해요. 위 골든이 그 시나리오의 기대 출력 spec 이고, staging 가용 시 case.sh 로 옮기면 돼요. 시크릿 fixture 의 secret 값은 전부 `FAKE_PLACEHOLDER` 라 실제 시크릿이 아니에요.
