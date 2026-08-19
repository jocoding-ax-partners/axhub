# connector_query 안전

development 가 외부 connector 데이터를 읽을 때(MCP `connector_query` 또는 CLI fallback) 지키는 안전 규칙이에요. connector 는 읽기 전용 추상화지만, 생성·실행하는 쿼리를 좁게 유지해요.

## 규칙

- **SELECT-only.** 조회만 해요. INSERT/UPDATE/DELETE/DDL 은 development v1 범위 밖이에요.
- **LIMIT 필수.** 모든 탐색 쿼리에 LIMIT 을 붙여요 (스키마 파악·미리보기는 작은 표본이면 충분). 무제한 스캔 금지.
- **timeout.** 느린 쿼리는 짧은 timeout 으로 끊고, 실패하면 degrade(스키마만/사용자에 질문)해요.
- **임의 SQL passthrough 금지.** 사용자 발화를 그대로 SQL 로 흘리지 않아요. 필요한 컬럼·필터만 구조적으로 구성해요.
- **무관 리소스 금지.** 사용자가 지목한 connector/리소스만 조회하고, grant 안 받은 리소스는 건드리지 않아요.
- **join 보수적.** 명시 확인 전에는 단일 리소스 조회를 기본으로 해요.

생성하는 앱 코드의 런타임 쿼리도 같은 원칙을 따라요 — 기존 앱의 DB/connector 경로를 파라미터화해서 쓰고, LIMIT/페이지네이션을 넣어요. `@ax-hub/sdk` 의 legacy data-plane DSL 은 제거됐으므로 새 런타임 read 경로로 만들지 않아요.
