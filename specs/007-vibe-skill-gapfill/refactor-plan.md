# Refactor Plan 007-R — 기존 skill 을 현재 CLI(0.17.3)로 전체 리팩토링

> **목표:** 현재 `skills/*/SKILL.md`가 호출하는 모든 `axhub ...` 명령을 `ax-hub-cli` `origin/main` v0.17.3 source와 대조하고, 없는 flag·stub 가정·오래된 버전 문구를 제거해요. 신규 gap-fill과 별개로, 기존 skill 이 지금 사용자에게 거짓 계약을 주지 않게 만드는 유지보수 wave예요.

| 메타 | 값 |
|---|---|
| 기준 CLI | `origin/main` = tag `v0.17.3` (`a5310b6`) |
| 스냅샷 | `/tmp/ax-hub-cli-origin-main-007` |
| 현재 checkout 주의 | 로컬 sibling repo는 0.17.2 브랜치 + 미커밋 변경이라 branch-only 증거로만 취급해요 |
| 산출물 | 이 `refactor-plan.md` + `plan.md` + `skills-catalog.md` + `source-audit.md` |
| 실행 범위 | 계획만. 실제 skill 수정은 후속 작업이에요 |

---

## 1. 방법론

각 skill마다 아래를 반복해요.

1. `rg 'axhub ' skills/<name>/SKILL.md`로 모든 CLI 호출을 추출해요.
2. `source-audit.md` command tree와 Rust source의 `Args`/manual parser를 대조해요.
3. flag가 없으면 문구 수정이 아니라 workflow 수정이에요.
4. help/schema에 있지만 실행 경로가 무시하면 “stub/defer”로 표시해요.
5. D1 guard, TodoWrite, preflight, registry, consent, 해요체, keyword baseline 을 보존해요.
6. 수정 후 skill 단위로 `skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`, `bun test`, `bunx tsc --noEmit`를 돌려요.

---

## 2. R0 — 사용자-노출 거짓 계약 즉시 수정

| skill | 현재 문제 | v0.17.3 source 사실 | 수정 방향 |
|---|---|---|---|
| `deploy` | `axhub deploy create --branch ... --json` 호출 + `--execute` 없음 | `deploy/create.rs`: `--app`, `--commit`, `--force-rebuild`, `--no-retry`, `--dry-run`, `--execute`만 존재. dry-run 기본 | `--branch` 제거, mutation 경로에 `--execute` 추가. branch는 consent/context/metadata로만 보관 |
| `deploy` | `axhub deploy cancel ... --yes --json` | `deploy/cancel.rs`: positional `deployment_id`, `--app`, `--dry-run`, `--execute`; `--yes` 없음 | cancel preview → consent(`deploy_cancel`) → `axhub deploy cancel <id> --app <app> --execute --json` |
| `apps` | `apps create --from-file axhub.yaml --yes`, `apps update <app> --field <field>` | `apps.rs`: create에는 `--yes` 없음. update는 `--name/--description/--visibility/...` 명시 flag, `--field` 없음 | CRUD workflow를 source flag로 재작성. update field-picker는 실제 flag allowlist로 변환 |
| `migrate` | `apps detect`를 current CLI처럼 사용, `deploy create --branch` 사용 | `apps detect`는 v0.17.3 origin/main에 없음. `deploy create --branch` 없음 | remote detect는 helper/local manifest-only로 낮춤. deploy call은 `--commit --execute` only. branch는 git/github 단계에서만 사용 |
| `github` | top-level `axhub github`를 deprecated/거절로 금지 | `github.rs`: `github accounts list`, `github installations repos --installation-id <id>` 정상 read surface. app 연결은 `apps git` | top-level github read discovery를 복원. `apps git` connect/status/update/disconnect는 유지. `deploy git`도 같은 skill로 확장 |
| `recover` | “v0.1.0 CLI rollback 미지원이라 forward-fix” | `deploy rollback`과 `deploy cancel` 존재 | recover는 forward-fix로 남기되 “CLI rollback 없음” 삭제. 특정 deployment_id rollback은 신규 `rollback` skill로 라우팅 |
| `init` | “NEVER axhub init; v1.0.0-rc.1 stub” | `initcmd.rs`: `axhub init --framework --app --target`가 `axhub.yaml`을 씀. 단 `--from-template`는 help에 있으나 실행 로직에서 실질 사용 안 함 | bootstrap saga 우선은 유지 가능하나 `axhub init` 전면 금지는 삭제. manifest-only 초기화 fallback으로 재검토 |
| `doctor` | “--fix/--dry-run/--send-report 미구현 stub” | `doctor.rs` Args에 flag 존재. 현재 run은 `fix/dry_run/send_report`를 의미 있게 쓰지 않고 일반 report 중심 | `--offline`은 사용 가능. `--fix/send-report/dry-run`은 no-op/stub 여부를 정직하게 표기하고 자동 호출 금지 근거를 “미구현 flag 없음”이 아니라 “현재 run에서 side-effect 없음/미검증”으로 변경 |
| `verify` | logs 마지막 50줄을 client-side trim, `--tail` 없음만 강조 | `deploy/logs.rs`: `--tail`은 없지만 `--limit`(1-1000, default 100) 존재 | `axhub deploy logs --app <app> --limit 50 --json` 우선. client trim은 보조 |
| `status` | “status” 발화를 deploy status로만 처리 | top-level `axhub status`는 profile/endpoint/logged_in/apps_count 일반 상태 | status skill 또는 inspect가 “배포 상태 vs CLI 상태”를 분기해야 함 |
| `init` | bootstrap/status coverage가 계획에서 누락됨 | `apps bootstrap`/`bootstrap-status`는 v0.17.3 public source이며 dry-run/execute/watch semantics가 실제 구현됨 | init skill 이 계속 소유하되 source flag와 watch degrade/override 계약을 재검증 |
| `apps` | owned/workspace read commands 누락 | `apps owned`/`apps workspace`는 app-shape public read commands | apps/my-resources coverage에 명시하고 help/trigger에 필요한 경우 추가 |
| `inspect` | deploy doctor 누락 | `deploy doctor --app`는 public diagnostic read surface | inspect에 포함하거나 deploy/doctor skill 경계에 명시 |
| `deploy` | deploy fleet 누락 | `deploy fleet --apps --concurrency --execute` public multi-app mutation | operator/bulk deploy로 defer하거나 별도 fleet consent model 설계 전까지 자동화 금지 |


Exact source subcommand classification (audit-friendly):
- `apps bootstrap` → existing `init` saga coverage.
- `apps bootstrap-status` → existing `init` saga status/watch coverage.
- `apps owned` → existing `apps`/`my-resources` read inventory.
- `apps workspace` → existing `apps`/`my-resources` read inventory.
- `deploy doctor` → `inspect` read diagnostic.
- `deploy fleet` → defer/operator until fleet consent model exists.

---

## 3. R1 — 라벨 bump/소폭 수정

| skill | 항목 | 조치 |
|---|---|---|
| `logs`, `status`, `deploy`, `init`, `github`, `auth`, `verify` | `axhub-cli 0.15.3+` 라벨 | behavior가 현재 source와 맞는지 확인한 뒤 `0.17.3` 기준 설명으로 갱신 |
| `deploy` | `AXHUB_DEPLOY_PREP=0` legacy fallback | helper가 안정화됐으면 legacy fallback 축소/삭제. 단 안전 경로는 유지 |
| `apps`, `deploy`, `doctor` | “v0.2.0 command coverage polish” 제목 | 버전 라벨 제거 또는 `0.17.3 source-aligned`로 갱신 |
| `open` | `apphub.yaml` legacy dual-read | `manifest.rs`/manifest crate 기준으로 canonical `axhub.yaml`, legacy `apphub.yaml` 경고 문구 유지 여부 확인 |
| `upgrade` | 예시 `axhub-helpers 0.1.0 (plugin v0.1.0)` | 현재 package/plugin/helper version 예시로 갱신 |

---

## 4. R2 — 모든 기존 skill CLI-call 전수 대조

| 분류 | skill | 검증 초점 |
|---|---|---|
| HIGH(R0) | deploy, apps, migrate, github, recover, init, doctor, verify, status, inspect | 없는 flag/stub/trigger 충돌 제거 + bootstrap/owned/workspace/deploy doctor/fleet 분류 반영 |
| MEDIUM | logs, auth, open, upgrade, my-resources, data | 0.17.3 flag, schema envelope, degrade 정책, top-level github/resources/catalog 변경 확인 |
| LOW | env, profile, update, install-cli, setup, clarify, routing-stats, trace | 호출 존재 여부와 helper 계약 확인 |
| CLI 비의존 | enable-statusline, karpathy-guidelines, using-axhub-quality, axhub-* quality skills, _template | 별도 품질/문구만 유지 |

`my-resources`는 `tenants list`, `apps mine`, `members list`, `engines list`, `connectors list`, `resources list`, `catalog kinds`를 병렬 호출하므로 schema envelope 변화에 특히 주의해요.

---

## 5. R3 — 신규 gap-fill과 계약 통일

신규 10개 skill이 들어오면 기존 skill과 아래 정책을 통일해요.

- read-only: `--json`, no consent, no mutation fallback
- mutation: dry-run/preview first, D1 AskUserQuestion, consent token, `--execute`
- secrets: stdin/file only, no argv/log/TodoWrite leak
- app/tenant resolution: helper/preflight result를 우선하고, CLI slug→UUID resolver가 있는 명령은 CLI에 맡겨요
- trigger conflict:
  - `recover` = forward-fix/redeploy previous commit
  - `rollback` = `deploy rollback --from-deployment`
  - `deploy cancel` = in-flight deployment cancellation
  - `data` = catalog/read/insight
  - `tables` = dynamic table DDL + row writes
  - `status` = deploy progress unless user asks CLI/account/profile 상태, then `inspect`/top-level status
  - `apps bootstrap/bootstrap-status` = existing `init` saga, not a new skill
  - `apps owned/workspace` = apps/my-resources read inventory
  - `deploy doctor` = inspect/deploy diagnostic read
  - `deploy fleet` = deferred operator bulk mutation until fleet consent exists

---

## 6. 착수 순서

1. R0 deploy/apps/migrate/github/recover/init/doctor/verify/status/inspect 문서·workflow 수정
2. R0 수정 후 targeted tests + `skill:doctor --strict`
3. P0/P1 신규 skill scaffold/구현
4. R1 라벨 bump
5. R2 전수 CLI-call 대조
6. R3 신규/기존 계약 통합

---

## 7. 완료 self-check

- [ ] 그 skill 의 모든 `axhub` 호출이 source-audit command tree에 존재해요
- [ ] mutation 호출에는 `--execute`가 필요할 때만 붙고, preview 단계은 dry-run으로 남아요
- [ ] 없는 flag(`--branch` on deploy create, `--yes` on apps/deploy cancel, `--field` on apps update, `apps detect` on v0.17.3)는 사라졌어요
- [ ] `apps bootstrap/bootstrap-status`, `apps owned/workspace`, `deploy doctor`, `deploy fleet`가 명시 처분돼요
- [ ] help-only/stub flag는 구현된 기능처럼 설명하지 않아요
- [ ] registry/keyword/tone 테스트가 green이에요
- [ ] 신규 gap-fill과 trigger 경계가 충돌하지 않아요
