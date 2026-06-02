# Plan 007 — 바이브코더용 axhub skill gap-fill

> **목표:** 바이브코더가 CLI 세부 문법을 몰라도 자연어로 최신 `ax-hub-cli` 기능을 안전하게 쓰도록, 현재 `ax-hub-cli` **v0.17.3** 명령 표면을 실제 소스와 대조하고 미커버/부분커버/제외 항목을 신규 skill·기존 skill 리팩토링·명시 defer 로 완전히 처분해요.

| 메타 | 값 |
|---|---|
| Spec ID | 007-vibe-skill-gapfill |
| 작성일 | 2026-06-03 |
| 검증 기준 | `ax-hub-cli` `origin/main` = tag `v0.17.3` (`a5310b6 chore(release): axhub-cli 0.17.3`) |
| 검증 스냅샷 | `/tmp/ax-hub-cli-origin-main-007` (`git archive origin/main`) |
| 주의 | 로컬 `/Users/wongil/Desktop/work/jocoding/ax-hub-cli` checkout 은 `fix/manifest-filename-axhub-yaml` 브랜치(`382e145`, workspace `0.17.2`) + 미커밋 변경이 있어서 **최신 기준으로 쓰지 않아요**. |
| 산출물 | `plan.md`, `skills-catalog.md`, `refactor-plan.md`, `source-audit.md` |
| 실행 범위 | 계획 문서만. skill scaffold/구현은 후속 작업이에요. |
| Source of truth | `axhub/src/cli.rs`, `axhub/src/commands/**/*.rs`, `axhub --json-schema`, 현재 `skills/*/SKILL.md` 전수 대조 |

---

## 1. 검증 결론 요약

### 1.1 CLI 표면 수 정정

이전 문서의 “38개 top-level command”는 틀렸어요. v0.17.3 기준은 다음과 같아요.

- `cli.rs` `Command` enum: **43개 variant + Unknown**
- `--json-schema` public command: **39개**
- `cli.rs` hidden root command: `comment`, `ctxdeadline-lint`, `debug`, `like` **4개**

`--json-schema`는 description 이 비어 있어 설명 source 로는 약하지만, command tree/flag inventory 는 유용해요. 동작·stub 여부는 반드시 Rust source 로 재확인해요.

### 1.2 현재 axhub repo skill 표면

현재 `skills/`는 `_template` 포함 **33개 디렉터리**, 실제 user-facing skill 은 **32개**예요.

```
apps auth clarify data deploy doctor env github init install-cli logs
migrate my-resources open profile recover routing-stats setup status
trace update upgrade
axhub-debug axhub-diagnose axhub-plan axhub-review axhub-ship axhub-tdd
enable-statusline karpathy-guidelines using-axhub-quality
```

### 1.3 이번 감사에서 추가로 발견한 핵심 교정

| 항목 | 기존 문서/skill 주장 | v0.17.3 source 사실 | 계획 반영 |
|---|---|---|---|
| CLI version 기준 | 로컬 checkout 도 v0.17.3처럼 취급 | 로컬 checkout 은 0.17.2 브랜치, `origin/main`/tag 만 0.17.3 | 검증 기준을 archive snapshot 으로 고정 |
| `publish --watch` | 심사 승인/반려까지 watch 가능 | `publish.rs` help/schema 는 `--watch`를 보이지만 `run_backend`가 무시하고 POST만 해요 | publish skill 은 제출만. watch 는 CLI 구현 전 defer |
| `dev` | 장시간 로컬 프록시 실행 | `dev.rs`는 target 필요 + `axhub dev proxy target=... port=...` 출력 후 종료하는 stub 성격 | 신규 `dev` skill 보류. stub 명령으로 기록 |
| `manifest check --baseline` | read-only baseline 비교 가능 | `manifest.rs`는 `check`에서 항상 validation_error. `--baseline` 성공 경로 없음 | inspect 는 `manifest validate`만 사용. check 는 defer |
| `apps detect` | P4 migrate 확장 후보 | `origin/main` v0.17.3에는 없음. 로컬 브랜치 미커밋/브랜치 기능 | current plan 에서 제외, branch-only future 로 이동 |
| `deploy create --branch` | deploy/migrate skill 이 사용 | `deploy/create.rs`에는 `--branch`가 없고 `--execute`가 있어야 mutation | 기존 deploy/migrate R0 refactor 필수 |
| `deploy cancel --yes` | deploy skill 이 사용 | `deploy/cancel.rs`에는 `--yes`가 없고 `--execute`가 필요 | deploy cancel 은 “covered but stale”, R0 수정 |
| `apps update --field` | apps skill 이 사용 | `apps UpdateArgs`는 `--name`, `--description`, `--visibility` 등 명시 flag. `--field` 없음 | apps R0 refactor 필수 |
| `apps create --yes` | apps skill 이 사용 | `apps CreateArgs`에 `--yes` 없음 | apps R0 refactor 필수 |
| top-level `github` | github skill 이 deprecated 라고 금지 | v0.17.3 top-level `github accounts list`, `github installations repos --installation-id <id>`는 정상 read surface | github skill R0: read surface 복원 + app connection 은 `apps git` 유지 |
| `tables` coverage | create/drop/columns만 중심 | `tables columns remove`, `tables grants issue/list/revoke`도 public | tables 신규 skill 에 포함 |
| `apps bootstrap/bootstrap-status` | 누락 | v0.17.3 public source에 존재하고 watch degrade/override semantics가 실제 구현됨 | 기존 `init` skill 소유로 명시하고 R0에서 현재 계약 재검증 |
| `apps owned/workspace` | 누락 | v0.17.3 public read command. app-shape, no pagination flags | 기존 `apps`/`my-resources` read coverage로 명시 |
| `deploy doctor` | 누락 | public deploy diagnostic surface | 신규 `inspect`에 포함하고 기존 `deploy`/`doctor` 경계 정리 |
| `deploy fleet` | 누락 | multi-app deploy mutation with `--apps`, `--concurrency`, dry-run/execute | operator/bulk mutation으로 명시 defer. 별도 fleet consent model 전까지 신규 skill 금지 |
| top-level `status` | status skill 이 커버한다고 가정 | existing status skill 은 deploy status 전용. `axhub status`는 profile/endpoint/logged_in/apps_count 요약 | inspect 또는 setup/status refactor 에서 일반 상태 조회 추가 |

상세 증거는 [`source-audit.md`](./source-audit.md)에 있어요.

---

## 2. 설계 원칙

1. **소스 우선:** source snapshot + schema + 현재 skill 본문을 모두 대조하고, help 에 보이지만 구현이 비어 있는 flag 는 “stub/defer”로 기록해요.
2. **혼합 추상화:** read/status 는 thin skill, mutation 은 preflight + preview/dry-run + consent + execute 순서로 감싸요.
3. **중복 금지:** 기존 skill 이 정확히 커버하면 신규 생성하지 않아요. 기존 skill 이 stale 이면 신규가 아니라 `refactor-plan.md` R0/R1로 고쳐요.
4. **admin/internal 경계:** admin·hidden·internal·shell-completion·platform-operator 명령은 vibe skill 신규 생성에서 제외하고 이유를 남겨요.
5. **stub 정직성:** visible command 라도 현재 source 가 stub 이면 “쓸 수 있음”으로 포장하지 않아요. 후속 CLI 구현 조건을 명시해요.
6. **기존 authoring 패턴 유지:** `bun run skill:new`, D1 guard, TodoWrite Step 0, canonical preflight, consent token, registry, 해요체, keyword baseline, tests 를 지켜요.

---

## 3. 최신 CLI 커버리지 매트릭스

범례: ✅ 기존 정확 커버 · 🛠 기존 skill 리팩토링 필요 · 🟢 신규 skill · 🧩 기존/신규 확장 · 🟡 defer · ⛔ 제외

| # | command | 처분 | 담당 | 근거/비고 |
|---|---|---|---|---|
| 1 | `access` | 🟢 | `team` | grant/check/revoke/invite/uninvite. manual parser라 schema 는 `args`만 보임 |
| 2 | `agent` | 🧩 | `setup` 확장 | `agent install --client claude-code\|cursor\|codex`, `agent doctor`, `agent manifest`. setup/doctor 영역 |
| 3 | `admin` | ⛔ | — | platform admin |
| 4 | `apps` | 🛠✅🟢🧩 | `apps`/`init` refactor + `app-lifecycle` + `browse` + `github` | list/get/create/update/delete/owned/workspace/read members는 apps 계열. bootstrap/bootstrap-status는 기존 init 소유. CRUD skill 이 stale(`--yes`, `--field`). fork/suspend/resume 신규. discover/search/templates 신규 browse. git 는 github. purge/sign-icon-upload는 P4 defer |
| 5 | `audit` | ⛔ | — | admin/audit export |
| 6 | `auth` | ✅🟡 | `auth` | login/logout/status/whoami/refresh/pat 커버. oauth client 는 migrate 일부, idp 는 admin/defer |
| 7 | `authz` | ⛔ | — | authorization admin/taxonomy |
| 8 | `cache` | ⛔ | — | internal maintenance |
| 9 | `catalog` | ✅ | `data` | connector catalog read + safe invoke |
| 10 | `categories` | ⛔ | — | admin/taxonomy |
| 11 | `comment` | ⛔ | — | hidden |
| 12 | `completion` | ⛔ | — | shell setup |
| 13 | `completion-data` | ⛔ | — | completion support |
| 14 | `connectors` | 🟢 | `connectors` | list/create/update/delete/discover/credentials-set |
| 15 | `config` | 🟢 | `inspect` | `config explain` |
| 16 | `ctxdeadline-lint` | ⛔ | — | hidden/internal lint |
| 17 | `data` | 🟢 | `tables` | dynamic table row CRUD/count/list/get |
| 18 | `debug` | ⛔ | — | hidden |
| 19 | `deploy` | 🛠🟢🧩🟡 | `deploy` refactor + `rollback` + `inspect` + `github` | create/status/list/logs/watch covered but deploy create/cancel stale. rollback 신규. doctor/explain/codes inspect. git github. fleet는 public bulk mutation이지만 operator/defer |
| 20 | `dev` | 🟡 | — | current source is stub/echo, not long-running proxy. defer until real proxy ships |
| 21 | `doctor` | 🛠 | `doctor` | `--fix`, `--dry-run`, `--send-report`, `--offline` flags exist but fix/send-report no-op 여부 확인 필요. stale “미구현” 문구 제거/정정 |
| 22 | `email-domains` | ⛔ | — | admin/domain operator |
| 23 | `engines` | ✅🧩 | `my-resources`, `connectors` | engines list read |
| 24 | `env` | ✅ | `env` | app env list/get/set/update/delete with stdin secret |
| 25 | `feedback` | 🟡 | — | utility/external feedback, low vibe automation value |
| 26 | `gateway` | 🟡 | `data` future | `gateway query` guarded SQL overlaps data/catalog. defer |
| 27 | `github` | 🛠🧩 | `github` | top-level accounts/installations repos is valid read surface. Existing skill falsely forbids it. App repo connection remains `apps git`; deploy git also add |
| 28 | `init` | 🛠 | `init` | current `axhub init` writes `axhub.yaml`; `--from-template` shown but generic. Existing “never axhub init” needs source-grounded rewrite |
| 29 | `invitations` | 🟢 | `team` | send/list/bulk/cancel/resend |
| 30 | `like` | ⛔ | — | hidden |
| 31 | `manifest` | 🟢🟡 | `inspect` | `manifest validate` works. `manifest check --baseline` currently errors → defer |
| 32 | `members` | 🟢🧩⛔ | `team`, `my-resources` | list/me/resolve read. set-role/deactivate/reactivate admin/excluded |
| 33 | `open` | ✅ | `open` | app/logs/metrics URL open |
| 34 | `profile` | ✅ | `profile` | current/list/add/use/remove |
| 35 | `publish` | 🟢 | `publish` | publication request POST. watch flags visible but ignored in code → no watch contract |
| 36 | `resources` | ✅🟢 | `my-resources` + `resources` | list read covered. rename/move/namespace/bulk-register/delete/tag attach/detach 신규 |
| 37 | `review` | ⛔ | — | marketplace review admin |
| 38 | `status` | 🧩 | `inspect` or `status` refactor | top-level general status not deploy status. Existing status skill needs disambiguation |
| 39 | `support` | 🟡 | `doctor` future | `support diagnose --include-logs --output`; defer/doctor adjacent |
| 40 | `tables` | 🟢 | `tables` | create/drop/columns add/remove/grants/rows/list/get/check-availability/column-types |
| 41 | `tenants` | 🟢⛔ | `workspace` | whoami/list/get read. create/update/delete/icon admin/excluded |
| 42 | `update` | ✅ | `update` | check/apply dry-run/execute |
| 43 | `whatsnew` | 🟡 | — | utility release notes, defer or update extension |


Exact source subcommand classification (audit-friendly):
- `apps bootstrap` → existing `init` saga coverage.
- `apps bootstrap-status` → existing `init` saga status/watch coverage.
- `apps owned` → existing `apps`/`my-resources` read inventory.
- `apps workspace` → existing `apps`/`my-resources` read inventory.
- `deploy doctor` → `inspect` read diagnostic.
- `deploy fleet` → defer/operator until fleet consent model exists.

---

## 4. 신규 skill wave

> `dev` 는 이전 계획에서 신규 P3였지만, v0.17.3 source 가 long-running proxy 가 아니라 echo/stub 이라 이번 신규 skill 목록에서 제외해요.

| Wave | Skill | 래핑 CLI | 모델 | multi-step | needs-preflight | 상태 |
|---|---|---|---|---|---|---|
| P0 | `publish` | `publish --app --note` | sonnet | true | true | 신규. `--watch` 의존 금지 |
| P1 | `team` | `invitations *` + `members list/me/resolve` + `access *` | sonnet | true | true | 신규 |
| P1 | `tables` | `tables *` + `data *` row CRUD | sonnet | true | true | 신규. columns remove/grants 포함 |
| P1 | `connectors` | `connectors *` + `engines list` | sonnet | true | true | 신규 |
| P2 | `app-lifecycle` | `apps fork/suspend/resume` | sonnet | true | true | 신규. git 연결 제외 |
| P2 | `rollback` | `deploy rollback --app --from-deployment --execute` | sonnet | true | true | 신규. cancel 은 deploy refactor |
| P3 | `inspect` | `manifest validate`, `config explain`, `deploy doctor`, `deploy explain`, `deploy codes`, top-level `status` | haiku | false | false/conditional | 신규 thin + status disambiguation |
| P3 | `workspace` | `tenants whoami/list/get` | haiku | false | true | 신규 thin |
| P4 | `browse` | `apps discover/search/templates list` | haiku | false | true | 신규 read-only |
| P4 | `resources` | `resources rename/move/namespace/bulk-register/delete/tag-*` | sonnet | true | true | 신규 |

**총 신규:** 10개.
**기존 refactor:** R0 7개 이상(`deploy`, `apps`, `migrate`, `recover`, `init`, `doctor`, `verify`, `github`) + R1/R2 전수 라벨/호출 정렬.

---

## 5. 기존 skill 확장/리팩토링과의 경계

- `apps` 기존 skill 은 list/get/create/update/delete/owned/workspace/apps members read 를 유지하되 실제 v0.17.3 flag 로 고쳐요. `discover/search/templates` 는 신규 `browse`, `fork/suspend/resume` 은 `app-lifecycle`, `git` 은 `github`으로 분리해요.
- `deploy` 기존 skill 은 deploy create/status/logs/list/watch/cancel 을 유지하되 `create --execute`, `create`의 없는 `--branch` 제거, `cancel --execute`로 고쳐요. `deploy rollback`은 신규 `rollback`으로 분리하고, `deploy doctor/explain/codes`는 `inspect`가 읽기 진단으로 소유해요. `deploy fleet`는 bulk/operator mutation이라 별도 consent model 전까지 defer해요.
- `github` 기존 skill 은 `apps git connect/status/update/disconnect`를 계속 담당하고, top-level `github accounts list`, `github installations repos --installation-id <id>`도 read discovery로 복원해요. `deploy git configure/connect/disconnect/status`는 같은 skill 에 통합해요.
- `init` 기존 skill 은 `apps bootstrap`/`apps bootstrap-status`의 실제 watch semantics를 소유해요. 단 `axhub init`도 현재 `axhub.yaml`을 쓰므로 manifest-only fallback 가능성을 R0에서 재검토해요.
- `migrate` 기존 skill 은 `apps detect`가 v0.17.3에 없다는 점을 반영해 helper/local detect 또는 manifest-only 경로로 낮춰요. `deploy create --branch`도 제거해요.
- `recover`는 forward-fix(직전 안정 commit 재배포)로 남기되 “CLI rollback 미지원” 문구를 삭제하고 `rollback` skill 로 라우팅하는 조건을 명확히 해요.
- `status`는 deploy status와 top-level general status가 충돌해요. `inspect`가 `axhub status --json`을 소유하거나 status skill 내부에서 “배포 상태 vs CLI 상태”를 분기해야 해요.

상세 리팩토링 순서는 [`refactor-plan.md`](./refactor-plan.md)를 따라요.

---

## 6. Authoring 패턴 체크리스트

신규 skill 은 반드시 scaffold 로 만들어요. 직접 `mkdir skills/<name>` 금지예요.

```bash
bun run skill:new publish --model sonnet
bun run skill:new team --model sonnet
bun run skill:new tables --model sonnet
bun run skill:new connectors --model sonnet
bun run skill:new app-lifecycle --model sonnet
bun run skill:new rollback --model sonnet
bun run skill:new inspect --model haiku --no-multi-step --no-preflight
bun run skill:new workspace --model haiku --no-multi-step
bun run skill:new browse --model haiku --no-multi-step
bun run skill:new resources --model sonnet
```

공통 필수:

- frontmatter: `multi-step`, `needs-preflight`, `allows-dependency-execution`, `model`
- `multi-step: true`: TodoWrite Step 0 + 완료 cleanup
- `needs-preflight: true`: canonical preflight block 유지
- AskUserQuestion: D1 TTY guard + `tests/fixtures/ask-defaults/registry.json`
- mutation: `consent-mint` → preview/dry-run → explicit consent → `--execute`
- secrets: stdin/file만 사용, argv/log/telemetry 평문 금지
- 검증: `bun run skill:doctor --strict`, `bun run lint:tone --strict`, `bun run lint:keywords --check`, `bun test`, `bunx tsc --noEmit`

---

## 7. Consent action registry 초안

| skill | consent action(s) |
|---|---|
| publish | `publish_submit` |
| team | `invitation_send`, `invitation_bulk`, `invitation_cancel`, `invitation_resend`, `access_grant`, `access_revoke`, `access_invite`, `access_uninvite` |
| tables | `tables_create`, `tables_drop`, `tables_column_add`, `tables_column_remove`, `tables_grant_issue`, `tables_grant_revoke`, `data_insert`, `data_update`, `data_delete` |
| connectors | `connector_create`, `connector_update`, `connector_delete`, `connector_credentials_set` |
| app-lifecycle | `apps_fork`, `apps_suspend`, `apps_resume` |
| rollback | `deploy_rollback` |
| resources | `resource_namespace_create`, `resource_rename`, `resource_move`, `resource_bulk_register`, `resource_delete`, `resource_tag_attach`, `resource_tag_detach` |
| inspect/workspace/browse | read-only, 불필요 |

---

## 8. Defer / exclude ledger

| command/surface | 처분 | 이유 |
|---|---|---|
| `dev` | defer | 현재 source 는 실제 프록시가 아니라 target/port echo 후 종료 |
| `publish --watch`, `--watch-timeout` | defer | help 에 있지만 `publish.rs` 실행 경로가 무시함 |
| `manifest check --baseline` | defer | 현재 `manifest check`는 항상 validation_error |
| `apps detect` | defer | `origin/main` v0.17.3에 없음. 로컬 브랜치/미커밋 기능 |
| `apps purge` | defer/admin-like | soft-delete 이후 UUID-only hard purge. vibe 자동화 우선순위 낮음 |
| `deploy fleet` | defer/operator | multi-app deploy mutation(`--apps`, `--concurrency`)이라 별도 fleet consent/action model 전까지 자동화 금지 |
| `apps sign-icon-upload` | defer | 아이콘 업로드 signing flow. app metadata polish 로 후순위 |
| `auth oauth client` | partial/defer | migrate OAuth app setup에는 필요하지만 일반 vibe skill 로는 고급 |
| `auth idp`, `authz`, `admin`, `audit`, `review`, `categories`, `email-domains` | exclude | admin/platform operator |
| `gateway query` | defer | catalog/data read와 중복 + SQL safety 별도 설계 필요 |
| `feedback`, `support diagnose`, `whatsnew` | defer | utility/read-report, 낮은 gap-fill 우선순위 |
| hidden `comment`, `debug`, `like`, `ctxdeadline-lint` | exclude | hidden/internal |
| `completion`, `completion-data`, `cache` | exclude | shell/internal support |

---

## 9. 실행 순서

1. **R0 refactor 먼저:** 현재 skill 이 잘못된 CLI 명령을 실행하는 `deploy`, `apps`, `migrate`, `github`, `recover`, `init`, `doctor`, `verify`를 먼저 고쳐요.
2. **P0/P1 신규:** `publish`, `team`, `tables`, `connectors`부터 scaffold/구현해요.
3. **P2/P3 신규:** `app-lifecycle`, `rollback`, `inspect`, `workspace`.
4. **P4 신규/확장:** `browse`, `resources`, `setup agent`, `github deploy git`, status disambiguation.
5. **R2 전수 재검증:** 남은 기존 skill 의 모든 `axhub ...` 호출을 schema/source와 다시 diff해요.
6. **R3 통합:** 신규/기존이 동일한 dry-run/consent/preflight/degrade/secret 정책을 쓰도록 정렬해요.

---

## 10. 완료 기준

- `source-audit.md` command tree와 plan matrix가 서로 맞아요.
- 모든 public non-admin command/subcommand 는 신규/기존/확장/defer/exclude 중 하나로 처분돼요. 특히 `apps bootstrap/bootstrap-status`, `apps owned/workspace`, `deploy doctor`, `deploy fleet`도 빠지지 않아요.
- `skills-catalog.md`의 CLI flag 는 v0.17.3 source에 존재하는 것만 써요.
- stub/help-only flag(`publish --watch`, `manifest check`, `dev`)는 구현된 것처럼 쓰지 않아요.
- 기존 skill 리팩토링 대상은 `refactor-plan.md` R0/R1/R2에 모두 들어가요.
