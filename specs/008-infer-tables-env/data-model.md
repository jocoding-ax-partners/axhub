# Phase 1 Data Model: `infer-tables-env`

추천은 휘발성 in-memory 구조예요(파일 저장 안 함, FR-015). 아래는 SKILL 이 분석으로 만들어 표시·적용에 쓰는 논리 모델이에요.

## Entities

### Recommendation (추천)

한 프로젝트 분석의 전체 결과.

| Field | Type | Notes |
|-------|------|-------|
| `tables` | TableSuggestion[] | 추론된 테이블 추천 목록 |
| `env_vars` | EnvVarSuggestion[] | 추론된 환경변수 키 추천 목록 |
| `coverage` | CoverageNote | 스캔 범위/미스캔/저신뢰 요약(FR-013) |
| `target_app` | string \| null | cross-check·적용 대상 앱(미선택 시 null, 추천은 read-only 로 가능) |

- 휘발성. 재요청 시 소스 재분석(FR-015).

### TableSuggestion (테이블 추천)

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | 동적 테이블 이름(snake_case 정규화 권장) |
| `columns` | Column[] | 후보 컬럼 |
| `evidence` | SourceEvidence | 이 테이블을 추론한 근거 |
| `status` | `new` \| `exists` \| `needs_review` | cross-check 결과 |
| `confidence` | `high` \| `low` | declarative 아티팩트=high, 코드-only=low |

### Column

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | 컬럼 이름 |
| `type` | AxhubColumnType | 소스 타입 → axhub 타입 매핑(아래) |
| `constraints` | Constraint[] | `required` \| `unique` \| `primary_key` 중 추정된 것 |
| `confidence` | `high` \| `low` | 타입/제약 확신도. low 면 "검토 필요" |

### EnvVarSuggestion (환경변수 추천)

| Field | Type | Notes |
|-------|------|-------|
| `key` | string | 환경변수 키(값 아님, FR-016) |
| `is_secret` | boolean | 이름 휴리스틱으로 시크릿 분류(FR-004) |
| `default_value` | string \| null | 소스상 드러난 **비시크릿** 기본값만(예: PORT=8000). 시크릿이면 항상 null |
| `evidence` | SourceEvidence | 근거 |
| `status` | `new` \| `set` \| `needs_review` | `axhub env list` cross-check |

- **불변식**: `is_secret == true` 이면 `default_value` 는 항상 null. 시크릿 값은 모델에 절대 담지 않음(FR-012, SC-005).

### SourceEvidence (소스 근거)

| Field | Type | Notes |
|-------|------|-------|
| `path` | string | repo-상대 파일 경로 |
| `line` | number \| null | 위치(가능하면) |
| `pattern` | string | 추론 패턴 설명(예: "prisma model", "getenv() 호출") |

- 모든 suggestion 은 evidence 1개 이상 의무(SC-004, 100% 추적 가능).
- **NEVER**: 하드코딩 시크릿 리터럴을 `pattern`/표시에 복사하지 않음(FR-012).

### CoverageNote (커버리지 메모)

| Field | Type | Notes |
|-------|------|-------|
| `scanned` | string[] | 분석한 아티팩트/경로 |
| `skipped` | string[] | 미스캔 영역(크기/동적 패턴) — 조용한 truncation 금지(FR-013) |
| `low_confidence_count` | number | "검토 필요" 항목 수 |

### ApplyPlan (적용 계획)

사용자가 승인한 부분집합. 적용 단계에서만 생성.

| Field | Type | Notes |
|-------|------|-------|
| `tables_to_create` | TableSuggestion[] | status=`new` 만(멱등, FR-009) |
| `env_keys_to_register` | EnvVarSuggestion[] | status=`new` 만. 값은 stdin 입력 또는 건너뜀(FR-016) |
| `results` | ApplyResult[] | 항목별 `success` \| `failed` \| `skipped`(FR-008) |

## 소스 타입 → axhub 동적 테이블 컬럼 타입 매핑

| 소스(declarative) | axhub 컬럼 타입 |
|---|---|
| `String`/`varchar`/`text`/TS `string` | `text` |
| `Int`/`BigInt`/`integer`/TS `number`(정수) | `number` |
| `Float`/`Decimal`/`numeric`/TS `number`(소수) | `number` |
| `Boolean`/`bool`/TS `boolean` | `boolean` |
| `DateTime`/`timestamp`/`date` | `datetime` |
| `Json`/`jsonb`/TS `object` | `json` |
| `enum` | `text`(+ 검토 필요: 허용값 메모) |
| 관계/FK | 외래 참조는 컬럼화하되 "검토 필요"(외부 커넥터면 추천 제외, 스펙 Edge) |

> 매핑이 모호하면 best-guess + `confidence=low`("검토 필요"). 적용 전 사용자 검토.

## 상태 전이

```
분석 → suggestion.status:
   new          (cross-check 결과 미존재)
   exists/set   (이미 있음 → 적용 시 skip, 멱등)
   needs_review (저신뢰 추론 → 사용자 검토 권장)

승인 → ApplyPlan(new 만) → 적용:
   success | failed | skipped(사용자가 값 미입력 등)
```

## Validation rules (요구사항 매핑)

- 모든 suggestion 에 evidence ≥ 1 (SC-004).
- `is_secret` 이면 `default_value`/표시 값 없음 (FR-004/012, SC-005).
- 적용은 `new` 만, `exists/set` 은 skip (FR-009, SC-007).
- target_app 없거나 미인증이면 적용 불가, 추천은 read-only 로 제공 (FR-014).
