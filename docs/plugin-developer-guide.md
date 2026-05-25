# axhub Plugin 개발 가이드

> **수신**: axhub plugin 개발자
> **목적**: AI 코딩 환경 (claude-code · cursor · copilot · windsurf) 에서 사용자가 자기 권한 안 데이터를 바이브코딩으로 자연스럽게 invoke 하게 하는 **plugin 의 v1 설계**.
> **이 한 문서로 plugin 설계·구현 완결**.

---

## 0. 한 줄 요약

> **plugin = AI 에이전트가 바이브코딩으로 생성하는 사용자 코드 안에 "올바른 axhub 인증 + 안전한 SQL invoke" 를 자동으로 박아주는 thin layer.**

핵심 가치 3가지:
1. **인증 자동화** — 사용자가 토큰을 코드 어디에 박을지 고민할 필요 없음. 환경 (axhub 안 vs 로컬) 만 보면 plugin 이 SSO cookie / PAT 자동 선택.
2. **권한 안전** — 권한·마스킹·row_filter 는 backend evaluator 가 자동. plugin/AI 가 우회할 수 없음.
3. **AI 친화 컨텍스트** — `.axhub/` 폴더가 AI 에게 사용자 권한 catalog 를 자동 노출 → AI 가 정확한 column · 정책 hint 로 SQL 생성.

---

## 1. ⭐ 가장 중요 — 토큰 인증의 두 모드 (SSO)

**이 섹션이 plugin 의 본질**. 다른 모든 설계가 여기서 갈린다.

### 1.1 두 환경, 두 인증 방식

```
┌────────────────────────────────────────────────────────────────────────┐
│ Mode A — same-origin · SSO cookie (자동)                                │
│                                                                        │
│   axhub 안에서 실행되는 코드. origin = *.jocodingax.ai                  │
│   ▸ axhub.jocodingax.ai 의 페이지                                       │
│   ▸ axhub-deployed user app (사용자가 axhub 안에 빌드해서 deploy 한 앱) │
│                                                                        │
│   browser 가 _hub_access cookie 를 자동 전송 → 인증 끝.                  │
│                                                                        │
│   사용자 코드 예:                                                       │
│     fetch('/api/v1/...', { credentials: 'include' })                   │
│                                                                        │
│   → 토큰 변수 박을 필요 X. 코드에 자격증명 흔적 0.                       │
└────────────────────────────────────────────────────────────────────────┘

┌────────────────────────────────────────────────────────────────────────┐
│ Mode B — local · PAT (환경변수)                                         │
│                                                                        │
│   axhub 도메인 밖에서 실행되는 코드:                                     │
│   ▸ 로컬 노트북 (VS Code · claude-code 가 만든 로컬 script)             │
│   ▸ 외부 server · CI 파이프라인 · 자기 회사 internal cluster            │
│                                                                        │
│   SSO 없음 — PAT 필수.                                                  │
│                                                                        │
│   사용자 코드 예:                                                       │
│     fetch(url, { headers: { Authorization: `Bearer ${env.PAT}` }})     │
│                                                                        │
│   → PAT 은 env var · keychain 에서 주입. 하드코드 절대 금지.            │
└────────────────────────────────────────────────────────────────────────┘
```

### 1.2 plugin 의 환경 감지 휴리스틱

snippet 생성 시 plugin 이 자동 판단. 우선순위:

| 신호 | 결정 |
|---|---|
| 파일 `.axhub/AXHUB_TARGET` 가 `web-axhub` | **Mode A** |
| 프로젝트 path 에 `app-hub-frontend` / `axhub-deployed` 포함 | **Mode A** |
| `package.json` 의 `homepage` 또는 `vite.config` 의 `base` 가 `*.jocodingax.ai` | **Mode A** |
| 그 외 (Python · 로컬 Next dev · 외부 호스팅 · CI) | **Mode B** |
| 모호 | 사용자에게 prompt → 결과를 `.axhub/AXHUB_TARGET` 에 저장 |

### 1.3 바이브코딩 코드에 박힐 인증 패턴 — 언어별

#### Mode A — TypeScript / React (axhub-deployed app)

```ts
/**
 * axhub-qa-mysql / 공개/employees · mode=A · target=web-axhub
 * allowed_columns: id, name, email, department_id, region, title, hired_at
 * masked: name, hired_at (pii)
 */
const TENANT_ID = '<tenant uuid>';

const r = await fetch(
  `/api/v1/tenants/${TENANT_ID}/catalog/resources/axhub-qa-mysql/${encodeURIComponent('공개')}/employees:read`,
  {
    method: 'POST',
    credentials: 'include',           // ← SSO cookie 자동 전송
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      sql: 'SELECT id, name FROM employees LIMIT 100',
      row_limit: 100,
    }),
  },
);
const result = await r.json();
if (!result.allowed) throw new Error(`axhub denied: ${result.deny_reason}`);
```

**Mode A 규칙**:
- `Authorization` 헤더 박지 마라. SSO cookie 만으로 인증.
- `credentials: 'include'` 필수.
- relative path (`/api/v1/...`) 사용 — 같은 origin.

#### Mode B — Python (로컬 / CI)

```python
"""
axhub-qa-mysql / 공개/employees · mode=B · target=local-python
allowed_columns: id, name, email, department_id, region, title, hired_at
masked: name, hired_at (pii)
"""
import os, requests

AXHUB_API    = os.environ.get('AXHUB_API', 'https://axhub-api.jocodingax.ai')
AXHUB_TENANT = os.environ['AXHUB_TENANT']
AXHUB_PAT    = os.environ['AXHUB_PAT']           # ← env var, 하드코드 금지

r = requests.post(
    f"{AXHUB_API}/api/v1/tenants/{AXHUB_TENANT}/catalog/resources/axhub-qa-mysql/공개/employees:read",
    headers={
        "Authorization": f"Bearer {AXHUB_PAT}",
        "Content-Type": "application/json",
    },
    json={"sql": "SELECT id, name FROM employees LIMIT 100", "row_limit": 100},
)
result = r.json()
if not result["allowed"]:
    raise RuntimeError(f"axhub denied: {result['deny_reason']}")
```

#### Mode B — TypeScript / Node.js (로컬 server)

```ts
const AXHUB_API    = process.env.AXHUB_API    ?? 'https://axhub-api.jocodingax.ai';
const AXHUB_TENANT = process.env.AXHUB_TENANT!;
const AXHUB_PAT    = process.env.AXHUB_PAT!;

const r = await fetch(
  `${AXHUB_API}/api/v1/tenants/${AXHUB_TENANT}/catalog/resources/axhub-qa-mysql/${encodeURIComponent('공개')}/employees:read`,
  {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${AXHUB_PAT}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ sql: 'SELECT id FROM employees LIMIT 100', row_limit: 100 }),
  },
);
```

#### Mode B — Go

```go
api := os.Getenv("AXHUB_API")
tenant := os.Getenv("AXHUB_TENANT")
pat := os.Getenv("AXHUB_PAT")
body := strings.NewReader(`{"sql":"SELECT id FROM employees LIMIT 100","row_limit":100}`)

req, _ := http.NewRequest("POST",
    api+"/api/v1/tenants/"+tenant+"/catalog/resources/axhub-qa-mysql/공개/employees:read",
    body)
req.Header.Set("Authorization", "Bearer "+pat)
req.Header.Set("Content-Type", "application/json")
```

#### Mode B — Shell (CI · curl)

```bash
curl -s -X POST \
  -H "Authorization: Bearer $AXHUB_PAT" \
  -H "Content-Type: application/json" \
  "$AXHUB_API/api/v1/tenants/$AXHUB_TENANT/catalog/resources/axhub-qa-mysql/%EA%B3%B5%EA%B0%9C/employees:read" \
  -d '{"sql":"SELECT id FROM employees LIMIT 100","row_limit":100}'
```

### 1.4 절대 금지

- ❌ Mode A 코드에 `Authorization: Bearer ...` — SSO cookie 와 충돌 + PAT 노출 위험
- ❌ Mode B 코드에 PAT 하드코드 (`"Bearer eyJhbG..."`) — git commit · log · screenshot 누설
- ❌ 토큰값 `console.log` / `print` / log file
- ❌ axhub 도메인 외부에 PAT 전달 (proxy · 3rd party)

### 1.5 snippet 상단 주석 표준

생성된 모든 snippet 의 1줄에 mode 명시 — AI 가 같은 파일에 후속 호출 박을 때 mode 헷갈리지 않게:

```
* axhub-qa-mysql / 공개/employees · mode=A · target=web-axhub
* allowed_columns: id, name, email, ...
* masked: name, hired_at (pii)
```

---

## 2. plugin 의 역할 4가지

### 2.1 환경 감지 → 올바른 mode 의 snippet 생성

```
plugin snippet <connector>/<path> [--target web-axhub|local-python|local-node|local-go|local-bash|auto]
```

- `--target web-axhub` → §1.3 의 Mode A 코드 그대로
- `--target local-*` → 해당 언어의 Mode B 코드
- `--target auto` (default) → §1.2 휴리스틱 적용

생성 코드 상단 주석에 mode/target/allowed_columns/masked 명시.

### 2.2 `.axhub/` 폴더 — AI 컨텍스트 자동 배치

```
plugin sync
```

가 현재 폴더에 생성:

```
.axhub/
  AXHUB.md            ← AI 가 read 할 규칙 본문 (정적, §4)
  AXHUB_TARGET        ← "web-axhub" 또는 "local-*"
  catalog.json        ← 사용자 권한 안 resource snapshot
```

`catalog.json` 형식:

```json
{
  "generated_at": "2026-05-25T07:30:00Z",
  "api": "https://axhub-api.jocodingax.ai",
  "tenant_id": "...",
  "tenant_slug": "jocodingax",
  "user_email": "hm.joo@jocodingax.ai",
  "target": "local-python",
  "resources": [
    {
      "connector": "axhub-qa-mysql",
      "path": "공개/employees",
      "kind": "mysql-table",
      "tags": ["r-test1"],
      "allowed_columns": ["id","name","email","department_id","region","title","hired_at"],
      "columns": [
        { "name":"id",       "dtype":"int",     "tags":[],      "read":true },
        { "name":"name",     "dtype":"varchar", "tags":["pii"], "read":true,
          "mask_hint":"pii column — 응답값이 null / ●●● 가능" },
        { "name":"hired_at", "dtype":"date",    "tags":["pii"], "read":true,
          "mask_hint":"pii column — 응답값이 null / ●●● 가능" }
      ]
    }
  ]
}
```

> `allowed_columns` 는 backend `GET /catalog/resources/{c}/{p}` 응답의 `permissions.read.allowed_columns` 그대로.
> `mask_hint` 는 plugin 이 column tag (`pii` 등) 기반으로 추정해서 채움.

`.gitignore` 에 `.axhub/catalog.json` 자동 추가 — user_email · tenant_id 등 PII 누설 방지. `AXHUB.md` 와 `AXHUB_TARGET` 은 commit 가능 (정책 정보 없음).

### 2.3 MCP server — claude-code 직접 통합

stdio 모드 MCP server 를 plugin 이 노출. claude-code `~/.claude.json`:

```json
{
  "mcpServers": {
    "axhub": { "command": "<plugin binary>", "args": ["mcp"] }
  }
}
```

노출 tool:

| name | description (claude-code 가 read) | input |
|---|---|---|
| `axhub.search` | 사용자가 권한 가진 데이터 검색. 작업 시작 전 반드시 호출. | `{q?, kind?, connector?}` |
| `axhub.describe` | 단일 resource 의 column 정보 + 정책 hint. | `{connector, path}` |
| `axhub.invoke` | axhub gateway 통해 안전한 SQL 실행. SELECT/WITH 만. | `{connector, path, action:'read', sql, params?, row_limit?}` |
| `axhub.snippet` | 이 resource 의 invoke 코드를 현재 프로젝트 환경 (Mode A/B) 에 맞춰 생성. | `{connector, path, lang?, target?}` |

`axhub.snippet` 이 §1.3 의 패턴을 따라 코드 텍스트 반환 — claude-code 가 그걸 사용자 파일에 박는다.

각 tool 의 description 에 §1 의 두 모드 규칙 1줄 포함 (AI 가 인증 코드 박을 때 헷갈리지 않게).

### 2.4 에러를 친절한 한국어로

| 응답 | 동작 |
|---|---|
| `200 {allowed:true}` | 결과 표시 |
| `200 {allowed:false, deny_reason}` | **denied — error 아님**. `deny_reason` 그대로 사용자에게 |
| `401` | 자동 재로그인 안내 — Mode A 면 SSO 리다이렉트, Mode B 면 PAT 갱신 |
| `403` | 권한 요청 안내 |
| `404` | catalog 재조회 안내 |
| `500 internal_error` | §5 의 §5.7.1 참고 — 보통 catalog 미등록 column 사용. "쿼리에 모르는 컬럼이 있어요. `.axhub/catalog.json` 의 `allowed_columns` 만 사용해 주세요." `request_id` 표시 |

retry 자동 금지 — denied 또는 영구 에러일 수 있음.

---

## 3. plugin 호출 흐름 — 한 시나리오로 전체 그림

사용자가 claude-code 에 자연어 요청:
> 직원 데이터에서 부서별 평균 근속 알려줘

```
[1] claude-code 가 .axhub/AXHUB.md read
    → SSO 모드 확인 (.axhub/AXHUB_TARGET = "local-python")
    → 사용자 권한 catalog (.axhub/catalog.json) read
    → 1번 규칙: Mode B 패턴 사용 결정

[2] claude-code 가 MCP tool axhub.search({q:"employees"}) 호출
    → plugin 이 GET /api/v1/tenants/{T}/catalog/resources?search=employees 호출
    → 응답에서 "공개/employees" hit

[3] claude-code 가 axhub.describe({connector:"axhub-qa-mysql", path:"공개/employees"})
    → plugin 이 GET /catalog/resources/{c}/{p}
    → allowed_columns: [id, name, department_id, hired_at, ...]
    → name, hired_at 에 pii tag

[4] claude-code 가 axhub.snippet({connector, path, lang:"python", target:"local-python"})
    → plugin 이 §1.3 의 Python Mode B 패턴 + 사용자 의도에 맞는 SQL 생성:
      """
      sql = '''
        SELECT department_id, AVG(DATEDIFF(NOW(), hired_at)) AS avg_tenure_days
        FROM employees
        GROUP BY department_id
      '''
      """
    → 주석에 mode=B, allowed_columns, masked 명시
    → claude-code 가 이 코드를 사용자 파일에 박음

[5] 사용자가 코드 실행 → AXHUB_PAT env var 로 인증 → axhub backend
    → evaluator 가 row_filter / column mask 자동 적용
    → 결과: 부서별 평균, hired_at 은 mask 됐어도 DATEDIFF 가능
```

---

## 4. `.axhub/AXHUB.md` 본문 — AI 에이전트가 read

`plugin sync` 가 사용자 프로젝트에 그대로 배포. **AI 가 가장 먼저 read 할 문서**.

````markdown
# axhub 사용 규칙 (AI 에이전트 read 용)

## ⭐ 1번 규칙 — 토큰 인증의 두 모드

코드가 어디에서 실행될지에 따라 인증 방식이 다르다.

| 환경 (실행 위치) | 모드 | 인증 |
|---|---|---|
| axhub.jocodingax.ai 또는 axhub-deployed app | **Mode A** | SSO cookie (자동) |
| 로컬 노트북 · 로컬 dev · 외부 server · CI | **Mode B** | PAT (환경변수) |

**판단 우선순위**:
1. `.axhub/AXHUB_TARGET` 파일에 명시된 값.
2. 프로젝트가 axhub 도메인 deploy 예정인 SPA — Mode A.
3. 그 외 — Mode B.
4. 모호하면 사용자에게 묻고 결과를 `AXHUB_TARGET` 에 저장.

**Mode A 코드 형식** (axhub 안에서 실행):
```ts
const r = await fetch(`/api/v1/tenants/${T}/catalog/resources/.../{path}:read`, {
  method: 'POST',
  credentials: 'include',           // SSO cookie 자동
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ sql: '...', row_limit: 100 }),
});
```

**Mode B 코드 형식** (로컬 / 외부):
```python
import os, requests
r = requests.post(
    f"{os.environ['AXHUB_API']}/api/v1/tenants/{os.environ['AXHUB_TENANT']}/catalog/resources/.../{path}:read",
    headers={"Authorization": f"Bearer {os.environ['AXHUB_PAT']}"},
    json={"sql": "...", "row_limit": 100},
)
```

**절대 금지**:
- Mode A 코드에 `Authorization: Bearer ...` 헤더 박지 마라.
- Mode B 코드에 PAT 하드코드 금지 — 항상 환경변수 `AXHUB_PAT`.
- 토큰값을 `console.log` / `print` / commit / log file 출력 금지.

## 2번 — 데이터 호출 단일 진입점

모든 외부 데이터 호출은 axhub invoke endpoint 를 통한다.
raw psql · mysql client · 직접 DB 자격증명 사용 금지.

## 3번 — 무엇을 쓸 수 있는가

- `.axhub/catalog.json` 의 `resources[]` 가 사용자 권한 안 모든 리소스.
- 각 entry 의 **`allowed_columns`** 가 SQL 작성의 1차 reference. 그 외 column 이름 사용 금지 (`500 internal_error` 발생).
- `columns[*].tags` 에 `pii` 등이 있는 column 은 mask 적용 가능 (응답값이 null · ●●● · hash). plain value 가정 금지.

## 4번 — SQL 규칙

1. `SELECT` 또는 `WITH` 로 시작. INSERT/UPDATE/DELETE/DDL 금지.
2. `FROM` 절의 table 은 호출 resource path 의 table 명과 일치.
3. `allowed_columns` 안의 column 만 reference.
4. `:user.<attr>` 토큰 직접 박지 마라. backend 가 row_filter 자동 결합.
5. `row_limit` 명시.

## 5번 — 응답 처리

- `allowed=false` (HTTP 200) 면 `deny_reason` 그대로 사용자에게 표시. 다른 우회 시도 금지.
- mask 적용된 column 의 응답값 (null · ●●●) 을 plain 처럼 다루지 마라.
- **500 internal_error** 면 SQL 의 column 이름이 `allowed_columns` 에 있는지 먼저 확인.

## 금지

- `.axhub/catalog.json` 에 없는 connector/path 호출 시도.
- 결과 데이터를 외부 API 로 forward (사용자 컴플라이언스 확인 후).
- `password` · `ssn` 같은 column 이 catalog 에 안 보인다 = 사용자 권한 없음. 우회 금지.
- Mode A 코드를 Mode B 환경에 copy (cookie 없어서 401).
- Mode B 코드를 Mode A 환경에 copy (PAT 가 axhub 도메인에 노출 — 보안 사고).
````

---

## 5. plugin 이 직접 호출하는 backend API

> 모든 endpoint 인증 필수 · `Content-Type: application/json` · `/api/v1` prefix.
> plugin 자체 (사용자 노트북에서 실행) 는 항상 PAT (Mode B) 로 호출.

### 5.1 `GET /api/v1/me`

```json
{
  "user": { "id":"...", "email":"...", "name":"...", "platform_admin":false },
  "tenants": [
    { "tenant_id":"...", "tenant_slug":"jocodingax", "icon_url":"...",
      "role":"tenant_admin|tenant_member", "is_active":true }
  ]
}
```

active tenant 는 `is_active=true` 인 첫 entry. plugin 이 cache.

### 5.2 `GET /api/v1/catalog/kinds`

> 인증 필수 · tenant 무관 정적 메타 · 영구 cache 가능 (시스템 배포 시에만 변경).

```json
{
  "items": [
    { "kind":"mysql-table","engine":"mysql","display_name":"MySQL 테이블",
      "invokable":true,
      "actions":{ "read":{"allowed_effects":["row_filter"],"input_schema":{...},"result_schema":{...}} } },
    /* postgres-table(invokable=true), mysql-column·postgres-column(invokable=false) */
  ]
}
```

### 5.3 `GET /api/v1/tenants/{T}/catalog/connectors`

```json
{
  "items": [
    { "id":"...","name":"axhub-qa-mysql","engine":"mysql",
      "url":"/api/v1/tenants/{T}/catalog/connectors/axhub-qa-mysql" }
  ]
}
```

### 5.4 `GET /api/v1/tenants/{T}/catalog/resources`

query: `search` · `kind` · `connector_id` · `limit` (default=50, max=200).

> **plugin 은 항상 `limit=200`** — default 50 은 type='resource' 전체에서 적용돼 column row 까지 합치면 invokable table 이 잘릴 수 있음.

```json
{
  "items": [
    {
      "id":"...","connector":"axhub-qa-mysql","connector_id":"...",
      "path":"공개/employees","url":"...",
      "kind":"mysql-table","type":"resource","name":"employees",
      "attributes":{},"tags":[{"id":"...","name":"r-test1"}],
      "permissions":{
        "read":{
          "allowed":true,
          "input_schema":{ "...sql.description":"FROM 절은 호출 resource path 의 테이블과 일치해야 함." },
          "result_schema":{...}
        }
      }
    }
  ],
  "limit":50,"next_cursor":null
}
```

**보장**: 응답의 모든 항목은 invoke 가능 (backend 가 child column 1+ allowed 검증).
**한계**: list 응답엔 `allowed_columns` / `column_masks` / `row_filter` 없음 — detail 호출 필요.

### 5.5 `GET /api/v1/tenants/{T}/catalog/resources/{connector}/{path}`

> path URL-encode. 한국어 가능 (`공개/employees` → `%EA%B3%B5%EA%B0%9C/employees`).

```json
{
  "id":"...","connector":"axhub-qa-mysql","connector_id":"...",
  "path":"공개/employees","url":"...",
  "kind":"mysql-table","type":"resource","name":"employees",
  "attributes":{},"tags":[{"id":"...","name":"r-test1"}],
  "permissions":{
    "read":{
      "allowed":true,
      "allowed_columns":["department_id","email","hired_at","id","name","region","title"],
      "input_schema":{...},"result_schema":{...}
    }
  },
  "ancestors":[{"id":"...","name":"공개","type":"namespace","path":"공개"}],
  "children":[
    { "id":"...","path":"공개/employees/id","kind":"mysql-column","name":"id",
      "attributes":{"dtype":"int","nullable":false},"tags":[{"name":"r-test1"}],
      "permissions":{"read":{"allowed":true}} }
    /* email, hired_at(tags 에 pii), name, region, ... */
  ]
}
```

**list vs detail 차이**:

| 정보 | list (§5.4) | detail (§5.5) |
|---|---|---|
| `permissions.read.allowed` | ✅ | ✅ |
| `permissions.read.allowed_columns` | ❌ | ✅ |
| `input_schema` / `result_schema` | ✅ | ✅ |
| `children[]` | ❌ | ✅ |
| `ancestors[]` | ❌ | ✅ |
| `column_masks` / `row_filter` | ❌ | ❌ (invoke 응답에서 확인) |

plugin 은 detail 의 `allowed_columns` 를 `catalog.json` 에 그대로 옮긴다 — AI 의 1차 SQL hint.

### 5.6 `POST /api/v1/tenants/{T}/catalog/resources/{connector}/{path}:read`

> v1 의 유일한 action = `read`.

request:
```json
{ "sql":"SELECT id, name FROM employees LIMIT 10", "params":[], "row_limit":100 }
```

response — allowed:
```json
{
  "allowed":true,"action":"read",
  "result":{
    "columns":[{"name":"id","data_type":"INT"}, ...],
    "rows":[[1,"주형민"], ...],
    "row_count":2
  },
  "matched_policies":["uuid..."]
}
```

response — denied (HTTP 200, error 아님):
```json
{ "allowed":false,"action":"read","deny_reason":"...","matched_policies":["..."] }
```

### 5.7 에러 envelope (4xx/5xx)

```json
{ "error":{ "code":"...","category":"...","message":"...","retryable":false,"request_id":"..." } }
```

| HTTP | 처리 |
|---|---|
| 200 `allowed:true` | 정상 |
| 200 `allowed:false` | denied (에러 아님) · `deny_reason` 그대로 |
| 400 `invalid_format` | SQL/input 위반 |
| 401 | Mode A 면 SSO 리다이렉트, Mode B 면 PAT 갱신 안내 |
| 403 | 권한 요청 안내 |
| 404 | catalog 재조회 안내 |
| **500 `internal_error`** | §5.7.1 참고 |
| 502/503 | 1회 retry 시도 가능 |

#### 5.7.1 500 의 실제 원인 (v1)

| 시나리오 | 사용자 메시지 |
|---|---|
| **catalog 미등록 column reference** (예: `SELECT password FROM ...`) | "쿼리에 알 수 없는 컬럼이 있어요. `.axhub/catalog.json` 의 `allowed_columns` 만 사용해 주세요." |
| 외부 DB down · network | "데이터 서버 연결 불가. `request_id`={id} 와 함께 문의" |
| 정책 cell 의 actions 깨짐 | "권한 설정 오류. 관리자에게 `request_id` 문의" |

retry 자동 금지 — 영구 에러 가능성.

---

## 6. v1 알려진 한계 — plugin 이 사용자에게 안내

| 한계 | 영향 | plugin 대응 |
|---|---|---|
| SQL parser 부재 | `FROM other_table` 시 column 우연 일치 시 부분 응답 | snippet 의 SQL 생성 시 항상 `FROM <path 의 table 명>` 강제 |
| catalog 미등록 column → 500 | safesql 검수가 denied list 기반이라 catalog 에 없는 column 통과 → DB column-not-found | catalog 의 `allowed_columns` 를 snippet 주석에 박아 AI 가 그 외 column 안 쓰게 |
| `column_masks` list 응답 미노출 | mask 정보 사전 확인 불가 | column tag (`pii` 등) 로 mask 가능성 추정 + 응답에서 확정 |
| catalog default limit=50 | column row 많은 tenant 에서 누락 | plugin 은 항상 `limit=200` |
| mask 값이 cell 에 잘못 저장돼있을 수 있음 | backend evaluator fail-safe redact 적용 | 응답값이 ●●● 일 수 있음 안내 |
| 외부 도메인 Mode A 미지원 | CORS 차단 | 외부 도메인은 Mode B 만 |

---

## 7. Deliverable (v1)

| 항목 | 형태 | 우선순위 |
|---|---|---|
| **MCP server** (stdio) | claude-code 등록용. `axhub.search/describe/invoke/snippet` 4 tool 노출 | **P0** |
| **`plugin sync`** | `.axhub/{AXHUB.md, AXHUB_TARGET, catalog.json}` 생성 | **P0** |
| **`plugin snippet`** | Mode A / Mode B × Python·TS·Go·Shell 매트릭스. 상단 주석에 mode 명시 | **P0** |
| **인증 보조** | PAT 저장 (OS keychain · `~/.config/axhub/credentials.json` fallback) | **P0** |
| `@axhub/sdk-{python,node,go}` | invoke wrapper 라이브러리 | P1 |
| brew tap / scoop bucket | 비-npm 설치 경로 | P1 |

---

## 8. DoD (Definition of Done)

- [ ] MCP server 가 claude-code 등록 후 4 tool 노출
- [ ] PAT 가 OS keychain 저장 (fallback `~/.config/axhub/credentials.json` `chmod 600`)
- [ ] **`snippet --target web-axhub` 의 코드에 PAT 변수 안 박힘** — `credentials:'include'` + relative path 만
- [ ] **`snippet --target local-*` 의 코드에 PAT 하드코드 안 됨** — env var (`AXHUB_PAT`) 만
- [ ] snippet 상단 주석에 `mode=A` 또는 `mode=B` + `allowed_columns` + `masked` 명시
- [ ] `sync` 가 `.axhub/AXHUB.md` + `AXHUB_TARGET` + `catalog.json` 정상 생성
- [ ] `catalog.json` 의 `allowed_columns` 가 backend detail 응답과 일치
- [ ] 500 internal_error 에 친절한 한국어 안내 + `request_id` 표시
- [ ] denied 응답 (HTTP 200 `allowed:false`) 도 panic 없이 안내
- [ ] 401 토큰 만료 시 자동 재로그인 (Mode A 리다이렉트 / Mode B prompt)
- [ ] e2e 테스트 — staging PAT 로 §3 의 전체 흐름 통과

---

## 9. 참고

- backend repo: `github.com/jocoding-ax-partners/ax-hub-backend` (SPEC 260·307 다 구현)
- prod base URL: `https://axhub-api.jocodingax.ai`
- staging base URL: 별도 — axhub product team 에 문의
- MCP spec: <https://modelcontextprotocol.io>

---

이 가이드 받은 개발자는 **§1 (SSO 두 모드)** 부터 시작. 그 뒤에 §2.1 (`snippet`) → §2.2 (`sync`) → §2.3 (MCP) → §4 (`.axhub/AXHUB.md`) → §8 DoD 통과.

backend API 변경이 필요한 경우 (예: list 응답에 `allowed_columns` 추가) backend 팀과 먼저 조율.
