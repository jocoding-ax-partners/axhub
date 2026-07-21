# write 경로 게이트 (v1.1)

development 의 write 는 두 종류로 위험도가 달라요. (a)는 코드 생성, (b)는 development 가 직접 live DB 를 바꾸는 거예요.

## (a) 런타임 CRUD 코드 생성 (기본)

배포된 앱에서 end user 가 데이터를 추가·수정하는 form/mutation 화면 코드를 만들어요. 실제 write 는 앱 런타임이 하고 앱 auth 가 가드해요.

- 기존 앱의 DB/connector/서버 라우트 write 경로를 파라미터화해서 써요 (문자열 결합 금지). 제거된 `@ax-hub/sdk` legacy data-plane write DSL 은 새로 만들지 않아요.
- 입력 validation (필수·타입·길이), 표시값 escape.
- write 상태 UI: 제출 중(중복 제출 방지·disable), 성공, 실패(사람이 알아들을 메시지+재시도). optimistic UI 면 실패 시 롤백.
- 권한: 생성 코드가 쓰는 엔드포인트는 앱 auth_mode 를 따라요.

## (b) 빌드타임 스키마 프로비저닝 (옵트인 + 게이트)

현재 MCP의 `table_list`/`table_get`/`row_list` 계열은 읽기 전용이고, row write 는 배포된 앱 런타임의 `DATABASE_URL` 경로가 정답이에요. development 는 존재하지 않는 `mcp__axhub__table_create`/`column_add`를 찾거나 파라미터를 추측하지 않아요. 새 테이블·컬럼 같은 live 운영 변경은 clarity 의 공개 CLI 탐색·승인 계약으로 한 번 handoff 해요:

1. **존재 우선**: 정확한 `app_id`로 `table_list({app_id})`를 한 번 호출해 이미 있는지 확인해요. 목록 응답에 컬럼이 있으면 같은 table에 `table_get`을 다시 호출하지 않아요. 권한 오류면 다른 파라미터로 재시도하지 않고 bounded CLI/clarity fallback 으로 가요.
2. **preview-confirm AUQ (필수)**: handoff 전에 만들 table_name·컬럼(name/type, 화이트리스트 text/int/bigint/float/bool/timestamptz/uuid/jsonb)·영향(신규만·기존 데이터 무관)을 한국어로 보여주고 명시 승인받아요.
3. **clarity handoff**: 승인된 동일 스키마만 `clarity`가 최신 공개 CLI leaf를 확인해 실행해요. development 가 `axhub --help | grep`, `tables --help | head`, pipe/redirect/복합 shell로 leaf를 탐색하지 않아요. 성공 결과를 받으면 바로 같은 development 흐름으로 돌아와 코드 생성과 테스트를 계속해요.
4. **headless 무변경**: CI·비대화형·AUQ 불가면 스키마 변경을 하지 않아요. 기존 테이블이 없으면 필요한 스키마만 보고하고 멈춰요.
5. **partial-failure 복구**: 테이블 생성 뒤 컬럼 추가가 실패하면 성공한 부분과 실패한 부분을 사람 말로 정확히 알리고, 자동 삭제나 반복 생성을 하지 않아요.
6. **런타임 write**: 앱 코드는 `get_recipe`의 `dynamic-db-crud` 경계와 기존 driver/ORM을 따라 `DATABASE_URL`로 파라미터화 write를 구현해요. legacy data-plane write DSL 은 새로 만들지 않아요.

## 경계 재확인

"테이블 만들어줘" (순수 axhub table 운영) 는 여전히 **clarity** 예요. development 는 **기능을 만들다가 그 기능이 새 테이블을 필요로 할 때만** (b) 게이트로 스키마를 옵트인 생성해요 — 단독 "테이블 만들기" 진입점이 아니에요.
