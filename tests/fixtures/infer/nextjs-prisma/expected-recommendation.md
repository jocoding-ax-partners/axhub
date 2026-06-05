# Expected recommendation — nextjs-prisma

read-only 추천 골든. 분석 후 아무것도 바꾸지 않아요.

## 테이블 추천

| 테이블 | 컬럼(타입·제약) | 근거 | 상태 |
|---|---|---|---|
| `users` | `id`(number, PK) · `email`(text, 필수·고유) · `name`(text) · `created_at`(datetime) | `prisma/schema.prisma` (model User) | 신규 |
| `orders` | `id`(number, PK) · `user_id`(number, 필수) · `amount`(number, 필수) · `status`(text) · `created_at`(datetime) | `prisma/schema.prisma` (model Order) | 신규 |

## 환경변수 추천

| 키 | 시크릿? | 기본값 | 근거 | 상태 |
|---|---|---|---|---|
| `DATABASE_URL` | 예 | — | `.env.example` / datasource url | 신규 |
| `NEXTAUTH_SECRET` | 예 | — | `.env.example` | 신규 |
| `STRIPE_API_KEY` | 예 | — | `.env.example` | 신규 |
| `PORT` | 아니오 | `3000` | `.env.example` | 신규 |

**커버리지**: `schema.prisma` + `.env.example` 분석 · 미스캔 없음 · 검토 필요 1건(`orders.user_id` 는 FK 관계 → number 컬럼으로 추천하되 검토 권장).

> SC-001 recall(테이블 2 + env 4 전부 추론), SC-004 evidence(모든 항목 근거), FR-001 컬럼 타입·제약 데모.
