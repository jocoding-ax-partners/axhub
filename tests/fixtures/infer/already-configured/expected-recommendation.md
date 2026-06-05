# Expected recommendation — already-configured

`current-state.json` 의 기존 상태와 cross-check 한 결과. 모든 항목이 이미 있어서 적용하면 0 변경(멱등, SC-007).

## 테이블 추천

| 테이블 | 컬럼(타입·제약) | 근거 | 상태 |
|---|---|---|---|
| `orders` | `id`(number, PK) · `amount`(number, 필수) · `status`(text) · `created_at`(datetime) | `prisma/schema.prisma` (model Order) | 이미 있음 |

## 환경변수 추천

| 키 | 시크릿? | 기본값 | 근거 | 상태 |
|---|---|---|---|---|
| `DATABASE_URL` | 예 | — | `.env.example` / datasource url | 이미 설정 |

**커버리지**: schema + env 분석 · 신규 0건 · 이미 있음 2건.

## 적용 결과 (승인 시)

```
테이블  orders        → skipped (이미 있음)
env     DATABASE_URL  → skipped (이미 설정)
```

> SC-007 멱등: `신규` 항목 0 → 적용해도 생성/변경 0. 재실행도 동일(0 변경).
