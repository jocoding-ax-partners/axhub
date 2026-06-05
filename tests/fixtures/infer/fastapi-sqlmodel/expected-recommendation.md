# Expected recommendation — fastapi-sqlmodel

read-only 추천 골든.

## 테이블 추천

| 테이블 | 컬럼(타입·제약) | 근거 | 상태 |
|---|---|---|---|
| `products` | `id`(number, PK) · `name`(text, 필수) · `price`(number, 필수) · `in_stock`(boolean) · `created_at`(datetime) | `alembic/versions/0001_init.py` (create_table) | 신규 |
| `cart_items` | `id`(number, PK 추정) · `product_id`(number) · `quantity`(number) — **검토 필요** | `app/settings.py` (CartItem class, 코드-only) | 검토 필요 |

## 환경변수 추천

| 키 | 시크릿? | 기본값 | 근거 | 상태 |
|---|---|---|---|---|
| `DATABASE_URL` | 예 | — | `app/settings.py` (getenv) | 신규 |
| `SECRET_KEY` | 예 | — | `app/settings.py` (getenv) | 신규 |
| `PORT` | 아니오 | `8000` | `app/settings.py` (getenv 기본값) | 신규 |

**커버리지**: alembic 마이그레이션(고신뢰) + `settings.py` 분석 · `CartItem` 은 코드-only 모델 → "검토 필요"(SC-001 recall 보장 제외) · 미스캔 없음.

> declarative(alembic)는 high-confidence, 코드-only(CartItem)는 best-effort 라는 D2 절단을 데모. PORT 기본값 8000 prefill(FR-016).
