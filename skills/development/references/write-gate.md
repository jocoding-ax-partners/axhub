# write 경로 게이트 (v1.1)

development 의 write 는 두 종류로 위험도가 달라요. (a)는 코드 생성, (b)는 development 가 직접 live DB 를 바꾸는 거예요.

## (a) 런타임 CRUD 코드 생성 (기본)

배포된 앱에서 end user 가 데이터를 추가·수정하는 form/mutation 화면 코드를 만들어요. 실제 write 는 앱 런타임이 하고 앱 auth 가 가드해요.

- `@ax-hub/sdk` 의 파라미터화된 write 경로를 써요 (문자열 결합 금지).
- 입력 validation (필수·타입·길이), 표시값 escape.
- write 상태 UI: 제출 중(중복 제출 방지·disable), 성공, 실패(사람이 알아들을 메시지+재시도). optimistic UI 면 실패 시 롤백.
- 권한: 생성 코드가 쓰는 엔드포인트는 앱 auth_mode 를 따라요.

## (b) 빌드타임 스키마 프로비저닝 (옵트인 + 게이트)

기능 저장소용으로 development 가 `mcp__axhub__table_create`/`column_add` 를 **직접 실행**(live DB mutation). MCP 도구는 **단발·즉시 실행·내장 확인 없음** (호출=바로 생성) + **idempotency 파라미터 없음**. 그래서 게이트를 **skill 이 강제**해요:

1. **가용성 확인**: write 도구(`mcp__axhub__table_create` 등)가 세션에 있나? 없으면(operator-off/미등록) (b) 불가 → "이 테이블이 필요해요 — clarity 로 먼저 만들거나, MCP 등록·재시작 후 다시" 안내하고 (a)·기존 테이블 작업으로 degrade.
2. **존재 우선 (check-then-create, idempotency 대체)**: `table_list`/`table_get` 으로 이미 있는지 확인 — 있으면 재사용(생성 안 함). idempotency 가 없으니 이 체크가 중복 생성 방지의 유일한 수단이에요.
3. **preview-confirm AUQ (필수)**: 도구가 무확인 단발이라, 호출 **전에** skill 이 만들 table_name·컬럼(name/type, 화이트리스트 text/int/bigint/float/bool/timestamptz/uuid/jsonb)·영향(신규만·기존 데이터 무관)을 한국어로 보여주고 명시 승인받아요. deploy 의 preview-confirm 패턴 재사용.
4. **headless 무변경 (one-way 안전)**: `! [ -t 1 ]` / `$CI` / `$CLAUDE_NON_INTERACTIVE` / AUQ 불가면 스키마 변경을 **하지 않아요** (safe default = no-mutation). "스키마 생성은 대화형에서만 해요" 안내.
5. **partial-failure 복구**: `table_create` 성공 후 후속(`column_add`/`row_insert`) 실패 시 — raw 숨기고 "table X 는 만들어졌어요. 컬럼 Y 는 실패했어요. 정리하려면 …" 안내(silent 금지). 트랜잭션이 없으니 가장 위험한 연산을 마지막에 두고, 실패 지점을 정확히 보고해요.
6. **권한**: 앱 auth + 사용자 role 이 스키마 변경을 허용하는지 선확인(불가 시 거부 안내).

## 경계 재확인

"테이블 만들어줘" (순수 axhub table 운영) 는 여전히 **clarity** 예요. development 는 **기능을 만들다가 그 기능이 새 테이블을 필요로 할 때만** (b) 게이트로 스키마를 옵트인 생성해요 — 단독 "테이블 만들기" 진입점이 아니에요.
