# Development SDK/DB 표면 · 배포 준비 점검 상세

development SKILL 의 5단계(SDK/DB 표면 확인)와 11.5단계(배포 준비 점검)가 로드하는 내부 reference 예요.

## 5. SDK/DB 표면 확인 (현재 SDK 우선)

데이터 접근 코드를 짜기 전에 현재 앱이 실제로 쓰는 DB/connector 경로와 설치된 SDK 버전을 확인해요. `@ax-hub/sdk` 3.x 는 legacy `/data` 데이터플레인을 제거했어요. 따라서 `sdk.data`, `sdk.tenant(...).app(...).data`, `defineSchema`, `where`, `discover()`, `data.table(...)` 같은 예전 typed data DSL 을 생성하지 않아요.

- 앱 런타임 기능 코드는 우선 **기존 앱의 데이터 접근 방식**(예: 서버 라우트, DB client, connector helper, ORM, fetch wrapper)을 따라요. SDK가 아니라 앱 코드의 실제 패턴이 런타임 authority 예요.
- `@ax-hub/sdk` 로 DB를 확인해야 하는 경우는 control-plane 성격의 raw DB introspection 으로 제한해요: `sdk.apps.rawDb.tables(appId)` 와 `sdk.apps.rawDb.tableRows(appId, table, { page, perPage, environment })`. 이 표면은 app id + owner/admin 권한이 필요하고, 앱 런타임 CRUD DSL 이 아니에요.
- SDK 문서나 MCP 검색 결과가 `defineSchema`/`where`/`tenant().app().data` 를 제안하면 stale 정보로 취급하고, 설치된 SDK README/CHANGELOG 또는 현재 package export 로 다시 확인해요.
- 외부 connector 접근처럼 SDK로 풀 문제가 아니면 `connector_list`/`connector_resources`(MCP) 또는 CLI fallback 으로 실제 리소스와 샘플만 확인하고, 생성 코드는 앱의 기존 connector/DB 패턴에 맞춰요.

## 11.5. 배포 준비 점검 (infer-tables-env 연계) 상세

verify 통과 후, deploy 핸드오프 전에 **방금 생성한 코드가 실제로 참조하는 테이블·환경변수**를 스캔해 빠진 게 있는지 확인해요 — 코드 분석이지 전용 CLI 명령이 아니에요(deploy 의 infer-tables-env 와 같은 성격). 비차단이고, 빠진 걸 찾으면 development 가 가진 게이트로 **그 자리에서** 메워 배포 왕복을 없애요. 이건 (b) write-gate 의 탐지 프론트엔드예요 — 사용자가 "테이블 만들어줘" 라고 명시 안 해도, 생성코드가 없는 테이블을 참조하면 능동 감지해 게이트로 연결해요.

- **빠진 테이블** (코드가 참조하는데 `table_list`/CLI 에 없음) → `references/write-gate.md` 의 (b) 게이트로 연결해요 ("이 기능엔 `X` 테이블이 필요해요 — 만들까요?" → preview-confirm). deploy 는 테이블을 못 만들지만 development 는 (b) 게이트로 만들 수 있어요.
- **빠진 환경변수** (코드가 읽는데 `env_var_list`/CLI 에 없음) → "이 기능엔 `Y` env 가 필요해요" 한 줄 안내 후 clarity/deploy 에서 설정하도록 이어줘요. `env_var_set` 은 operator-gated 라 development 가 **자동 설정하지 않아요**.
- **headless/비대화형** (AUQ 불가): 기본은 스캔 결과만 보고하고 **아무것도 바꾸지 않아요** (스키마·env 무변경 safe default, deploy headless 계약과 동일). 단, 사용자가 같은 요청에서 `production mutation 허용`, `테이블 생성까지 진행`, `전부 실행`처럼 명시 권한을 줬고 필요한 테이블/컬럼이 구체적으로 결정됐으면, preview JSON 을 먼저 보고한 뒤 CLI `--execute` 로 생성할 수 있어요. 이 경우 idempotency key 를 쓰고, create 후 rows/list 로 검증해요.
- 점검을 마치면 deploy 핸드오프 맥락에 **"배포 준비 점검 완료"** 를 남겨, deploy 의 사전 점검 질문이 **중복되지 않게** 해요 (`../deploy/references/session-carryover.md`).
