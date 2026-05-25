# axhub 사용 규칙 (AI 에이전트 read 용)

이 디렉토리는 `axhub-helpers sync` 가 생성해요. 이 프로젝트는 axhub gateway 를 통해 외부 데이터에 접근해요.

- `catalog.json` 은 private 로컬 컨텍스트라 커밋하면 안 돼요 (`.gitignore` 에 자동 등록).
- `AXHUB_TARGET` 은 snippet 생성에 쓰는 런타임 타깃을 기록해요.

## 1번 규칙 — 토큰 인증의 두 모드

코드 실행 환경에 따라 인증 방식이 달라요.

| 환경 | 모드 | 인증 |
|---|---|---|
| axhub.jocodingax.ai 또는 axhub 가 배포한 앱 (origin = `*.jocodingax.ai`) | **A** | SSO cookie (자동) |
| 로컬 · CI · 외부 서버 | **B** | PAT (환경변수 `AXHUB_PAT`) |

판단 우선순위: `.axhub/AXHUB_TARGET` → 프로젝트 환경 → 모호하면 사용자에게 물어요.

**Mode A 코드** (SSO cookie):

```ts
await fetch(`https://axhub-api.jocodingax.ai/api/v1/tenants/${TENANT}/catalog/resources/${connector}/${path}:read`, {
  method: 'POST',
  credentials: 'include',                    // SSO cookie 자동
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ sql, row_limit: 100 }),
});
```

**Mode B 코드** (PAT, `X-Api-Key`):

```python
import os, requests
r = requests.post(
    f"{os.environ['AXHUB_BASE_URL']}/api/v1/tenants/{os.environ['AXHUB_TENANT']}/catalog/resources/{connector}/{path}:read",
    headers={"Content-Type": "application/json", "X-Api-Key": os.environ["AXHUB_PAT"]},
    json={"sql": "...", "row_limit": 100},
)
```

**절대 금지**:

- Mode A 코드에 `X-Api-Key` 나 `Authorization` 헤더를 박지 않아요 (cookie 가 인증해요).
- Mode B 코드에 PAT 를 하드코드하지 않아요 — 환경변수만 써요.
- 토큰을 `console.log` / `print` 하지 않아요.

## 2번 규칙 — 데이터 호출의 단일 진입점

모든 외부 데이터 호출은 `/api/v1/tenants/{TENANT}/catalog/resources/{connector}/{path}:read` 엔드포인트로 해요 (또는 `axhub catalog invoke --action read`, `@ax-hub/sdk` 의 `catalog.invoke`).
raw psql · mysql · 직접 자격증명 사용은 금지예요.

## 3번 규칙 — 무엇을 쓸 수 있나

- `.axhub/catalog.json` 의 `resources[]` 가 권한 있는 모든 리소스예요.
- 각 entry 의 **`allowed_columns`** 가 SQL 작성의 1차 reference 예요.
- 그 외 column 이름은 쓰지 않아요 → `500 internal_error` (외부 DB 의 column-not-found).
- `mask_hint` 있는 column 은 응답값이 `null` / `●●●` / hash 일 수 있어요 — plain 으로 가정하지 않아요.

## 4번 규칙 — SQL 규칙

1. `SELECT` 또는 `WITH` 로 시작해요. INSERT / UPDATE / DELETE / DDL 금지.
2. `FROM` 절의 table 은 호출 path 의 table 명과 일치시켜요 (예: path=`공개/employees` → `FROM employees`).
3. `allowed_columns` 안의 column 만 reference 해요.
4. `:user.<attr>` 토큰을 직접 박지 않아요 — backend 가 row_filter 를 자동으로 결합해요.
5. `row_limit` 을 명시해요.
6. 인사이트/집계도 같은 read 경로로 해요 (`GROUP BY` / `COUNT` / `AVG` / `SUM`) — 마스킹 안 된 `allowed_columns` 만 집계해요.

## 5번 규칙 — 응답 처리

- `allowed=true` → 결과를 가공해요.
- `allowed=false`, `deny_reason="권한이 없거나..."` (HTTP 200) → **사용자에게 generic 메시지를 그대로 보여줘요**. 우회하거나 다른 SQL 을 시도하지 않아요.
- `allowed=false`, `deny_reason="SQL 형식 오류: ..."` → 사용자에게 SQL 수정을 유도해요.
- `404 not_found` (catalog 미등록 path) → `axhub catalog search` 로 다시 확인해요.
- `500 internal_error` → SQL 의 column 이름이 `allowed_columns` 에 있는지 먼저 확인해요. **자동 retry 금지** — 외부 DB 의 영구 에러일 수 있어요.
- mask 적용된 column 의 응답값 (`null` · `●●●` · hashed · `****1234`) 을 plain 처럼 다루지 않아요. mask 종류는 catalog 의 `mask_hint` (소문자 enum) 를 참고해요.

## 금지 사항

- `.axhub/catalog.json` 에 없는 connector/path 호출 시도.
- 결과 데이터를 외부 API 로 forward (사용자 컴플라이언스 확인 뒤에만).
- `password` · `ssn` 같은 column 이 catalog 에 안 보이면 권한이 없는 거예요. 우회하지 않아요.
- Mode A 코드를 Mode B 환경에 복사 (cookie 가 없어서 401).
- Mode B 코드를 Mode A 환경에 복사 (PAT 가 axhub 도메인에 노출 — 보안 사고).
- `/api/v1/tenants/{TENANT}/grants` · `/tags` · `/subjects` · `/connectors` (governance) 직접 호출 → admin 전용 (member 면 403).

## 탐색 명령

- `axhub catalog search --json --limit 200` — 사용 가능한 리소스 발견.
- `axhub catalog get --connector <name> --path <path> --json` — live read 전에 컬럼/정책 확인.
- `axhub catalog invoke --connector <name> --path <path> --action read --sql '<SELECT ...>' --row-limit 100 --execute --json` — first live read consent 뒤에만.
