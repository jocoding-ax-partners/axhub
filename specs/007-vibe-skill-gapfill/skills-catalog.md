# Skills Catalog 007 — 신규 skill 초안 (v0.17.3 source-aligned)

> 이 문서는 scaffold에 부을 **content spec**이에요. 직접 `mkdir`/복붙 금지예요. 반드시 `bun run skill:new <slug>`로 생성한 뒤 placeholder를 채워요. D1 guard, TodoWrite Step 0, canonical preflight block, registry stub은 scaffold가 주입해야 해요.

검증 기준:

- CLI source: `ax-hub-cli` `origin/main` = tag `v0.17.3` (`a5310b6`)
- Snapshot: `/tmp/ax-hub-cli-origin-main-007`
- Evidence: [`source-audit.md`](./source-audit.md)
- Stub 주의: `publish --watch`, `manifest check --baseline`, `dev`, `apps detect`는 현재 구현된 기능처럼 쓰지 않아요.

공통 규약:

- 모든 CLI 호출은 `--json` 또는 JSON envelope 출력이 가능한 형태를 우선해요.
- `needs-preflight: true`는 body 상단 canonical preflight block을 포함해요.
- `multi-step: true`는 Step 0 TodoWrite + 종료 cleanup을 포함해요.
- 모든 AskUserQuestion 앞에 D1 TTY guard를 둬요.
- 모든 mutation은 preview/dry-run → consent-mint → `--execute` 순서예요.
- secret/credential은 argv, TodoWrite, 로그, registry에 값이 남으면 안 돼요. PreToolUse 가 payload digest 를 검증해야 하는 destructive consent 경로는 file+digest 로 고정해요.

---

## P0 — publish

```yaml
name: publish
description: '이 스킬은 사용자가 만든 axhub 앱을 마켓플레이스에 공개 심사로 제출하고 싶어할 때 사용해요. 다음 표현에서 활성화: "앱 공개", "공개해", "게시", "게시해", "마켓에 올려", "퍼블리시", "심사 제출", "심사 올려", "스토어에 올려", "publish", "submit for review", "make public", 또는 axhub 앱 공개 심사 제출 의도.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`publish.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 게시 심사 제출 | `axhub publish --app <slug\|id> --note "<사유>" --json` | POST `/api/v1/apps/{id}/review-requests`; `--note`는 1000자 이하 |

**중요:** help/schema에 `--watch`, `--watch-timeout`이 보이지만 v0.17.3 `run_backend`는 이 값을 읽지 않아요. skill은 제출 후 “심사 요청이 생성됐어요”까지만 말하고, 승인/반려 watch를 약속하지 않아요.

**Workflow**

1. Step 0 TodoWrite: `[앱 확인, 제출 사유 확인, 제출 preview, 동의·실행, 결과 안내]`.
2. preflight로 auth/app context 확인. 앱이 없으면 `axhub apps mine --json` 또는 current manifest에서 후보를 좁혀요.
3. note는 1000자를 넘기지 않게 검사해요.
4. D1 + AskUserQuestion: “이 앱을 마켓플레이스 심사에 제출할까요?” safe default `abort`.
5. consent-mint action `publish_submit`, top-level `app_id`, context `{note_length,note_digest}`.
6. `axhub publish --app "$APP" --note "$NOTE" --json` 실행.
7. 응답의 review/request id가 있으면 보여주고, watch는 현재 CLI 미구현이라 후속 확인 방법만 안내해요.

**registry.json**

```json
"publish": {
  "이 앱을 마켓플레이스 심사에 제출할까요?": {
    "safe_default": "abort",
    "rationale": "게시는 외부 노출 mutation이라 비대화형 자동 제출을 막아요",
    "allowed_safe_defaults": ["abort"],
    "metadata": {"decision_topic": "publish-confirm"}
  }
}
```

**NEVER**

- NEVER `--watch`가 승인/반려까지 polling한다고 말하지 않아요.
- NEVER 비대화형에서 자동 제출하지 않아요.
- NEVER note 1000자 초과를 그대로 보내지 않아요.

---

## P1 — team

```yaml
name: team
description: '이 스킬은 사용자가 axhub 워크스페이스나 앱에 팀원을 초대하거나, 초대 목록을 보거나, 앱 접근 권한을 주고받고 싶어할 때 사용해요. 다음 표현에서 활성화: "팀원 초대", "초대해", "멤버 초대", "사람 추가", "협업자 추가", "초대 목록", "초대 취소", "접근 권한 줘", "앱 공유", "공유해", "invite", "add member", "team invite", "share app", "grant access", 또는 axhub 팀·접근 관리 의도. 멤버 권한 변경·비활성화는 admin 영역이라 다루지 않아요.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`invitations.rs`, `members.rs`, `access.rs`)

| 동작 | 명령 | read/mutate |
|---|---|---|
| 초대 보내기 | `axhub invitations send <email> --role member\|manager --tenant <t> --json` | mutate, dry-run 없음 |
| 초대 목록 | `axhub invitations list --status pending --expires-within 168h --tenant <t> --json` | read |
| 대량 초대 | `axhub invitations bulk --from-file users.csv --role member --strict --execute --tenant <t> --json` | mutate, dry-run 기본, CSV 최대 200 |
| 초대 취소 | `axhub invitations cancel <id> --execute --tenant <t> --json` | mutate, dry-run 기본 |
| 초대 재발송 | `axhub invitations resend <id> --role member --execute --tenant <t> --json` | mutate, dry-run 기본 |
| 멤버 목록/본인/resolve | `axhub members list --tenant <t> --json`, `members me`, `members resolve <email>` | read |
| 앱 접근 받기 | `axhub access grant --app <id> --json` | self-receive mutate, `--user` 미지원 |
| 접근 확인 | `axhub access check --app <id> --json` | read |
| 접근 반납 | `axhub access revoke --app <id> --execute --json` | self mutate, dry-run 기본 |
| 앱 초대/철회 | `axhub access invite/uninvite --app <id> --user <uuid> --execute --json` | owner/admin mutate, dry-run 기본 |

제외: `members set-role`, `members deactivate`, `members reactivate`.

**Workflow**

1. Step 0 TodoWrite: `[작업 확인, 대상 resolve, preview, 동의·실행, 결과]`.
2. preflight auth/tenant 확인.
3. D1 + AskUserQuestion: 작업 분기(`invite`, `list`, `access`), safe default `list`.
4. 즉시 mutation인 `invitations send`는 dry-run이 없으므로 consent 전 preview card를 더 엄격하게 보여줘요.
5. bulk는 CSV header(`email,role`)와 ≤200을 먼저 검사해요.
6. access grant는 self-receive만 가능해요. 남을 추가하려면 `access invite --user` 또는 상대방의 self-grant를 안내해요.

**registry.json**

```json
"team": {
  "팀·접근 관련 어떤 작업을 할까요?": {"safe_default":"list","rationale":"비대화형 기본은 read-only 목록이에요","allowed_safe_defaults":["list"],"metadata":{"decision_topic":"team-action"}},
  "이 초대를 보낼까요?": {"safe_default":"abort","rationale":"초대 발송은 외부 메일 mutation이에요","allowed_safe_defaults":["abort"],"metadata":{"decision_topic":"invite-confirm"}},
  "이 앱 접근을 변경할까요?": {"safe_default":"abort","rationale":"접근 부여/회수는 mutation이에요","allowed_safe_defaults":["abort"],"metadata":{"decision_topic":"access-confirm"}}
}
```

---

## P1 — tables

```yaml
name: tables
description: '이 스킬은 사용자가 axhub 앱의 동적 테이블을 만들거나 지우거나, 컬럼·권한·행 데이터를 관리하고 싶어할 때 사용해요. 다음 표현에서 활성화: "테이블 만들", "테이블 생성", "동적 테이블", "컬럼 추가", "컬럼 삭제", "행 추가", "행 넣어", "레코드 삽입", "데이터 넣어", "행 삭제", "테이블 권한", "create table", "add column", "insert row", "delete row", 또는 axhub 동적 테이블 스키마·행 관리 의도. 외부 커넥터 SQL 조회·인사이트는 data 스킬이 담당해요.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`tables.rs`, `data.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 목록/상세/rows | `axhub tables list --app <id> --json`, `tables get <table> --app <id> --json`, `tables rows <app> <table> --json` | read |
| 이름 확인/타입 | `axhub tables check-availability <table> --app <id> --json`, `tables column-types --app <id> --json` | read |
| 테이블 생성 | `axhub tables create <table> --app <id> --column 'title:text' --owner-column <col> --description "<d>" --execute --json` | dry-run 기본, `--schema` 가능 |
| 테이블 삭제 | `axhub tables drop <table> --app <id> --confirm <table> --execute --json` | dry-run 기본, `--force` 선택 |
| 컬럼 추가 | `axhub tables columns add <table> --app <id> --name <col> --type <t> --nullable --default <v> --execute --json` | dry-run 기본 |
| 컬럼 제거 | `axhub tables columns remove <table> --app <id> --name <col> --execute --json` | dry-run 기본 |
| 권한 목록/발급/회수 | `axhub tables grants list --app <id> --table <table> --json`, `grants issue`, `grants revoke` | issue/revoke는 dry-run 기본 |
| 행 조회/count/get | `axhub data list/count/get <table> --app <id> --json` | owner-scoped read |
| 행 insert/update/delete | `axhub data insert/update/delete <table> ... --execute --json` | dry-run 기본, batch ≤500 |

**Workflow**

1. Step 0 TodoWrite: `[작업 확인, 앱·테이블 resolve, 스키마/행 준비, preview, 동의·실행, 결과]`.
2. preflight auth/app 확인.
3. D1 + AskUserQuestion: 작업 분기(`read`, `schema`, `row`, `grant`), safe default `read`.
4. create 전 `check-availability`와 `column-types`로 검증해요.
5. drop/remove/delete/revoke는 대상 이름/id를 confirmation card에 그대로 보여줘요.
6. row body는 JSON validation 후 넘겨요. secret 값이 행 payload에 있으면 로그에 출력하지 않아요.

**registry.json**

```json
"tables": {
  "동적 테이블/데이터 작업을 골라요": {"safe_default":"read","rationale":"비대화형 기본은 read-only예요","allowed_safe_defaults":["read"],"metadata":{"decision_topic":"tables-action"}},
  "이 테이블 스키마를 변경할까요?": {"safe_default":"abort","rationale":"DDL 변경은 mutation이에요","allowed_safe_defaults":["abort"],"metadata":{"decision_topic":"tables-schema-change"}},
  "이 행 데이터를 변경할까요?": {"safe_default":"abort","rationale":"행 쓰기는 mutation이에요","allowed_safe_defaults":["abort"],"metadata":{"decision_topic":"tables-row-change"}},
  "이 테이블 권한을 변경할까요?": {"safe_default":"abort","rationale":"권한 변경은 mutation이에요","allowed_safe_defaults":["abort"],"metadata":{"decision_topic":"tables-grant-change"}}
}
```

---

## P1 — connectors

```yaml
name: connectors
description: '이 스킬은 사용자가 axhub 외부 데이터베이스 커넥터를 등록·수정·삭제하거나 자격증명을 갱신하고 싶어할 때 사용해요. 다음 표현에서 활성화: "DB 연결", "데이터베이스 연결", "커넥터 추가", "커넥터 만들", "postgres 연결", "mysql 연결", "외부 DB 붙여", "DB 자격증명", "커넥터 삭제", "connector", "connect database", "add connector", "db credentials", 또는 axhub 데이터 커넥터 관리 의도.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`gateway_surface.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 엔진/목록 | `axhub engines list --json`, `axhub connectors list --tenant <t> --enabled-only --json` | read |
| 생성 | `axhub connectors create --tenant <t> --name <n> --engine postgres\|mysql --config-file cfg.json --credentials-file creds.json --execute --json` | dry-run 기본, config/credentials digest 고정 |
| 수정 | `axhub connectors update <id> --tenant <t> --config-file cfg.json --enabled\|--disabled --execute --json` | dry-run 기본 |
| 삭제 | `axhub connectors delete <id> --tenant <t> --execute --json` | dry-run 기본 |
| 스키마 탐색 | `axhub connectors discover <id> --tenant <t> --json` | read |
| 자격증명 갱신 | `axhub connectors credentials-set <id> --tenant <t> --credentials-file creds.json --execute --json` | dry-run 기본, credentials digest 고정 |

`--credentials-stdin`도 CLI에 있지만 destructive consent 경로에서는 PreToolUse 가 stdin payload digest 를 검증할 수 없으므로 skill은 로컬 파일 digest 를 기본으로 해요.

**Workflow**

1. Step 0 TodoWrite: `[작업 확인, 엔진/설정 준비, 자격증명 파일 digest, preview, 동의·실행, 결과]`.
2. preflight auth/tenant 확인.
3. D1 + AskUserQuestion: 작업 분기(`create`, `list`, `manage`), safe default `list`.
4. config는 file/json, credential은 file digest 로 분리해요.
5. create/update/delete/credentials-set은 consent 후 `--execute`.

---

## P2 — app-lifecycle

```yaml
name: app-lifecycle
description: '이 스킬은 사용자가 axhub 앱을 복제하거나 일시정지·재개하고 싶어할 때 사용해요. 다음 표현에서 활성화: "앱 복제", "앱 포크", "앱 복사해", "앱 일시정지", "앱 멈춰", "앱 중지", "앱 재개", "앱 다시 켜", "fork app", "suspend app", "resume app", 또는 axhub 앱 생명주기 관리 의도. GitHub 저장소 연결은 github 스킬이 담당해요.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`apps.rs`, `apps/fork.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 앱 복제 | `axhub apps fork <source> --slug <new> --subdomain <new> --name <name> --tenant <t> --execute --json` | dry-run 기본, `--template`, `--repo-public` 가능 |
| 일시정지 | `axhub apps suspend <app> --execute --json` | dry-run 기본, runtime 영향 |
| 재개 | `axhub apps resume <app> --execute --json` | dry-run 기본, 자동 redeploy 보장 아님 |

**Workflow**

1. Step 0 TodoWrite: `[작업 확인, 앱 resolve, caveat 안내, preview, 동의·실행, 후속 안내]`.
2. preflight auth/app 확인.
3. D1 + AskUserQuestion: 작업 분기(`fork`, `toggle`), safe default `abort`.
4. suspend/resume는 runtime 영향과 redeploy 필요성을 카드에 표시해요.
5. fork는 slug/subdomain/name을 명확히 확인해요.

---

## P2 — rollback

```yaml
name: rollback
description: '이 스킬은 사용자가 특정 이전 배포 상태로 진짜 rollback 하고 싶어할 때 사용해요. 다음 표현에서 활성화: "이전 배포로 롤백", "특정 배포로 되돌려", "그 배포로 롤백", "배포 롤백", "rollback deployment", "rollback to deployment", 또는 axhub deploy rollback 의도. 진행 중 배포 취소는 deploy 스킬, 직전 커밋 재배포 forward-fix는 recover 스킬이 담당해요.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`deploy/rollback.rs`, `deploy/list.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 후보 조회 | `axhub deploy list --app <id> --json` | read |
| 이전 배포로 rollback | `axhub deploy rollback --app <id> --from-deployment <deployment-id> --execute --json` | dry-run 기본, `--from-deployment` 필수 |

`deploy cancel`은 신규 rollback skill 범위가 아니에요. 단 기존 deploy skill은 `--yes`를 제거하고 `--execute`로 리팩토링해야 해요.

**Workflow**

1. Step 0 TodoWrite: `[대상 배포 찾기, rollback 대상 확인, preview, 동의·실행, 상태 안내]`.
2. preflight auth/app 확인.
3. `deploy list`에서 rollback 후보를 보여줘요.
4. D1 + AskUserQuestion safe default `abort`.
5. consent-mint action `deploy_rollback`, context `{from_deployment}`.
6. `axhub deploy rollback --app "$APP" --from-deployment "$DEPLOYMENT" --execute --json`.

---

## P3 — inspect

```yaml
name: inspect
description: '이 스킬은 사용자가 로컬 axhub.yaml 매니페스트, 현재 CLI 설정, 일반 axhub 상태, 배포 설명·에러 코드를 검증·확인하고 싶어할 때 사용해요. 다음 표현에서 활성화: "매니페스트 확인", "axhub.yaml 검증", "설정 확인", "config 봐", "현재 endpoint 뭐", "CLI 상태", "axhub 상태", "deploy explain", "배포 코드", "manifest validate", "check config", 또는 axhub 매니페스트·설정 조회 의도.'
multi-step: false
needs-preflight: false
allows-dependency-execution: false
model: haiku
```

**CLI 매핑** (`manifest.rs`, `config.rs`, `status.rs`, `deploy/doctor.rs`, `deploy/codes.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 매니페스트 검증 | `axhub manifest validate --file axhub.yaml --json` | read. `--file` 생략 시 current dir dual-read |
| 설정 설명 | `axhub config explain --json` | read, secret redacted |
| 일반 CLI 상태 | `axhub status --json` | profile/endpoint/logged_in/apps_count |
| 배포 진단 | `axhub deploy doctor --app <id> --json` | read diagnostic |
| 배포 spec 설명 | `axhub deploy explain --app <id> --json` 또는 `axhub deploy --explain --json` | read |
| 배포 코드 참조 | `axhub deploy codes --json` | read |

**중요:** `axhub manifest check --baseline`은 현재 v0.17.3에서 성공 경로가 없어요. 이 skill에 넣지 않아요.

---

## P3 — workspace

```yaml
name: workspace
description: '이 스킬은 사용자가 자신의 axhub 워크스페이스나 테넌트 목록·소속·상세를 보고 싶어할 때 사용해요. 다음 표현에서 활성화: "워크스페이스", "내 워크스페이스", "워크스페이스 목록", "테넌트", "테넌트 목록", "어느 워크스페이스", "내 소속", "workspace", "tenant", "my workspaces", 또는 axhub 워크스페이스 조회 의도. 백엔드 endpoint/profile 전환은 profile 스킬이 담당해요.'
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: haiku
```

**CLI 매핑** (`tenants.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 현재 소속 | `axhub tenants whoami --tenant <t> --json` | read |
| 목록 | `axhub tenants list --all --json` | read |
| 상세 | `axhub tenants get <slug\|id> --json` | read |

제외: `tenants create/update/delete/icon`.

---

## P4 — browse

```yaml
name: browse
description: '이 스킬은 사용자가 axhub 마켓플레이스의 공개 앱을 검색하거나 부트스트랩 템플릿 목록을 둘러보고 싶어할 때 사용해요. 다음 표현에서 활성화: "앱 둘러봐", "마켓 검색", "공개 앱 찾아", "앱 검색", "다른 사람 앱", "템플릿 목록", "템플릿 뭐 있어", "어떤 템플릿", "marketplace", "discover apps", "search apps", "list templates", 또는 axhub 공개 앱·템플릿 탐색 의도. 내 앱 목록은 apps 스킬, 내 리소스 인벤토리는 my-resources 스킬이에요.'
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: haiku
```

**CLI 매핑** (`apps.rs`, `apps/templates.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 공개 앱 탐색 | `axhub apps discover --q <query> --category <slug> --sort <key> --limit N --created-within-days N --json` | read |
| 검색 호환 | `axhub apps search <query> --category <slug> --sort <key> --visibility public --json` | read |
| 템플릿 목록 | `axhub apps templates list --json` | read. list만 존재 |

`apps detect`는 v0.17.3 origin/main에 없으므로 browse/migrate 어디에도 current command로 넣지 않아요.

---

## P4 — resources

```yaml
name: resources
description: '이 스킬은 사용자가 게이트웨이 리소스(외부 DB 테이블/뷰)를 이름 변경·이동·네임스페이스 구성·태그·삭제로 조직하고 싶어할 때 사용해요. 다음 표현에서 활성화: "리소스 이름 바꿔", "리소스 이동", "네임스페이스 만들", "리소스 태그", "리소스 정리", "리소스 삭제", "리소스 등록", "rename resource", "move resource", "namespace", "tag resource", "bulk register", 또는 axhub 게이트웨이 리소스 조직 의도. 리소스 조회/인벤토리는 my-resources, 데이터 읽기는 data 스킬이에요.'
multi-step: true
needs-preflight: true
allows-dependency-execution: false
model: sonnet
```

**CLI 매핑** (`gateway_surface.rs`)

| 동작 | 명령 | 비고 |
|---|---|---|
| 목록 | `axhub resources list --tenant <t> --parent-id <id> --json` | read |
| 네임스페이스 생성 | `axhub resources namespace create --tenant <t> --name <n> --parent-id <id> --execute --json` | dry-run 기본 |
| 이름 변경 | `axhub resources rename <resource_id> --tenant <t> --name <new> --execute --json` | dry-run 기본 |
| 이동 | `axhub resources move <resource_id> --tenant <t> --parent-id <id> --execute --json` 또는 `--root` | dry-run 기본, xor |
| 대량 등록 | `axhub resources bulk-register --tenant <t> --connector-id <id> --items-file items.json --include-columns --execute --json` 또는 `--items-json <json>` | dry-run 기본 |
| 삭제 | `axhub resources delete <resource_id> --tenant <t> --cascade --execute --json` | dry-run 기본, cascade 주의 |
| 태그 | `axhub resources tag-attach/tag-detach <resource_id> --tenant <t> --tag-id <id> --execute --json` | dry-run 기본 |

**Workflow**

1. Step 0 TodoWrite: `[작업 확인, 리소스 resolve, preview, 동의·실행, 결과]`.
2. preflight auth/tenant 확인.
3. D1 + AskUserQuestion: 작업 분기(`list`, `organize`, `delete`, `tag`), safe default `list`.
4. delete cascade는 별도 강한 확인 문구를 보여줘요.
5. bulk-register items-json/file은 JSON validation 후 실행해요.

---

## P4 — 기존 skill 확장/수정 요약

| 대상 | 추가/수정 | 비고 |
|---|---|---|
| `init` | `axhub apps bootstrap --execute --watch --watch-timeout ...` + `apps bootstrap-status` | 기존 init saga 소유. watch degrade/override semantics를 0.17.3 source 기준으로 유지 |
| `apps` | `apps owned`, `apps workspace`, `apps members` read + CRUD flag 정정 | `--yes`, `--field` 제거. owned/workspace는 read inventory로 명시 |
| `setup` | `axhub agent install --client claude-code\|cursor\|codex`, `axhub agent doctor`, `axhub agent manifest` | agent command는 setup/doctor 인접 |
| `github` | top-level `github accounts list`, `github installations repos --installation-id <id>` read 복원 + `apps git update` + `deploy git configure/connect/disconnect/status` | 기존 “top-level github deprecated” 문구 제거 |
| `deploy` | `deploy cancel` command 수정 + `deploy fleet` defer 문구 | `--yes` 제거, `--execute` 사용. fleet는 별도 consent model 전까지 자동화 금지 |
| `apps` | CRUD flag 수정 + optional `apps members` read | `--field`, `--yes` 제거 |
| `inspect/status` | top-level `axhub status --json` 소유 | deploy status와 일반 CLI 상태 분기 |
| `migrate` | `apps detect` 제거 또는 branch-only future로 표시 | v0.17.3 origin/main 미포함 |


Exact source subcommand classification (audit-friendly):
- `apps bootstrap` → existing `init` saga coverage.
- `apps bootstrap-status` → existing `init` saga status/watch coverage.
- `apps owned` → existing `apps`/`my-resources` read inventory.
- `apps workspace` → existing `apps`/`my-resources` read inventory.
- `deploy doctor` → `inspect` read diagnostic.
- `deploy fleet` → defer/operator until fleet consent model exists.

---

## Defer / exclude

| surface | 처분 | 이유 |
|---|---|---|
| `deploy fleet` | defer | public command지만 multi-app bulk mutation이라 별도 fleet consent/action model 전까지 자동화 금지 |
| `dev` | defer | current command is echo/stub, not a persistent proxy |
| `publish --watch` | defer | help-only/stub; run ignores it |
| `manifest check --baseline` | defer | current command always validation_error |
| `apps detect` | defer | not in origin/main v0.17.3 |
| `apps purge`, `apps sign-icon-upload` | defer | hard-delete/icon upload polish, lower priority |
| `auth idp`, `authz`, `admin`, `audit`, `review`, `categories`, `email-domains` | exclude | admin/operator |
| hidden/internal/completion/cache | exclude | not vibe skill surface |
