# Contract: Apply Handoff (tables/env 위임)

승인 후 적용은 새 mutation 경로를 만들지 않고 기존 `tables`/`env` 스킬·CLI 로 위임해요. 각 mutate 전 `consent-mint` 동의 게이트 + 마스킹 미리보기 + 명시 확인(FR-006/007).

## C-1. 선행 게이트 (적용 진입 조건)

| 조건 | 요구 |
|---|---|
| 인증 | preflight `auth_ok=true`. false 면 적용 차단, 추천은 read-only 유지(FR-014) |
| 대상 앱 | `target_app` 해석됨(`axhub apps list`/resolve). 없으면 선택 유도(FR-014) |
| 최종 미리보기 | 대상 앱 + 생성할 테이블/컬럼 + 등록할 env 키(값 마스킹) 표시 후 명시 확인(FR-007) |
| 거절 | 확인 게이트 거절 시 어떤 변경도 없음(FR-006, SC-006) |

## C-2. 테이블 생성 (스키마 known → 자동)

`status=new` 인 TableSuggestion 만. 멱등(이미 있으면 skip, FR-009).

```bash
# 동의 토큰: action=table_create, top-level app_id, context={table}
# (consent-mint 호출은 tables 스킬 패턴을 따름)
axhub tables create "$TABLE" --app "$APP" --json
axhub tables add-column "$TABLE" "$COL:$TYPE" --app "$APP" --json   # 컬럼·제약 반영
```

> 실제 subcommand 형태는 `tables` 스킬/CLI 계약을 따름. 이 SKILL 은 위임만 하고 새 플래그를 만들지 않음.

## C-3. 환경변수 등록 (키만, 값은 stdin/건너뜀 — FR-016)

`status=new` 인 EnvVarSuggestion 만. 값은 추론하지 않음.

```bash
# 동의 토큰: action=env_set, top-level app_id, context={key}
printf %s "$VALUE" | axhub env set "$KEY" --app "$APP" --from-stdin --json
```

- 각 키마다: 사용자가 값을 stdin 으로 입력 → 등록, 또는 **건너뜀**(결과에 `skipped`).
- 비시크릿 `default_value` 가 있으면 그 값을 미리 채울 후보로 제시(사용자가 수정/수락).
- 시크릿 값은 argv 금지·평문 로그 금지(env 스킬 NEVER 준수).

## C-4. 결과 보고 (항목별, FR-008)

```
적용 결과:
  테이블  orders        → success
  테이블  line_items    → skipped (이미 있음)
  env     DATABASE_URL  → success
  env     STRIPE_KEY    → skipped (사용자가 값 미입력)
```

각 항목 `success` | `failed` | `skipped`. 부분 실패해도 멱등이라 재실행 안전(FR-009).

## C-5. consent action 매핑

| 동작 | consent action | context |
|---|---|---|
| 테이블 생성 | `table_create` | `{table}` |
| 컬럼 추가 | `table_alter` | `{table, column}` |
| env 키 등록 | `env_set` | `{key}` |

> top-level `app_id` 필수. 토큰은 단건 동작에 바인딩(기존 consent-mint 계약).
