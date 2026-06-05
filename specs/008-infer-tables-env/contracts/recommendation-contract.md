# Contract: Recommendation Output (분석 산출)

이 SKILL 은 외부 API 를 노출하지 않아요. "계약"은 (1) 분석이 만드는 추천의 구조와 (2) 사용자에게 보여주는 표현 규칙이에요. 휘발성(파일 저장 안 함, FR-015).

## C-1. 추천 표 표현 (read-only, 사람 검토용)

분석 후 항상 다음 두 섹션을 한국어 GFM 표로 보여줘요. 각 항목은 근거 의무(SC-004).

**테이블 추천**

| 테이블 | 컬럼(타입·제약) | 근거 | 상태 |
|---|---|---|---|
| `orders` | `id`(number, PK) · `title`(text, 필수) · `created_at`(datetime) | `prisma/schema.prisma:12` (prisma model) | 신규 |
| ... | ... | ... | 이미 있음 / 검토 필요 |

**환경변수 추천**

| 키 | 시크릿? | 기본값 | 근거 | 상태 |
|---|---|---|---|---|
| `DATABASE_URL` | 예 | — | `app/db.py:5` (getenv) | 신규 |
| `PORT` | 아니오 | `8000` | `app/main.py:20` (getenv 기본값) | 신규 |

**커버리지 한 줄**: 분석한 아티팩트 / 미스캔 영역 / "검토 필요" N건(FR-013).

## C-2. 표현 불변식 (NEVER)

- 시크릿 *값* 평문 노출 금지. `시크릿?=예` 면 `기본값` 칸은 항상 `—`(FR-004/012, SC-005).
- 하드코딩 시크릿 발견 시 근거에 그 리터럴 복사 금지 — "환경변수로 옮기세요" 보안 발견으로만 표기(FR-012).
- 모든 항목에 근거(파일·위치) 표기. 근거 없으면 항목을 내지 않음(SC-004).
- 추론 0건이면 표 대신 "추론된 것이 없어요" + 다음 행동 제안(FR-011).
- 이 단계는 read-only — 어떤 mutate 도 하지 않음(FR-006).

## C-3. 상태 판정 (cross-check)

| 상태 | 판정 |
|---|---|
| 신규 | `axhub tables list`/`axhub env list` 에 없음 → 적용 후보 |
| 이미 있음 | 이미 존재 → 적용 시 skip(멱등, FR-009) |
| 검토 필요 | 저신뢰(코드-only 모델, 모호 타입/제약) → 사용자 검토 권장 |

target_app 미선택/미인증이면 cross-check 없이 추천만 제시하고 상태는 "미확인"으로 표기(FR-014).

## C-4. 입력(분석 대상) 우선순위 — declarative 우선(D2)

1. `schema.prisma`, Prisma/Alembic 마이그레이션, `.env.example`/`.env.sample` → `confidence=high`
2. ORM 클래스·런타임 `getenv`/`process.env` → `confidence=low`("검토 필요", SC-001 recall 보장 제외)
3. 외부 커넥터를 가리키는 모델 → 새 테이블로 추천하지 않음(스펙 Edge)
