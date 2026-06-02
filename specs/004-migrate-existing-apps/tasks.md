# Tasks: 기존 앱 migrate (Migrate Existing Apps to axhub)

**Branch**: `feat/migrate-existing-apps` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

**Repos**: axhub plugin(this) · axhub-helpers(Rust, this repo) · app-hub-backend(Go) · ax-hub-cli(Rust workspace). 테스트 = TDD(parser/adapter/API/security golden 먼저).

**User Stories**: US1(P1) 기존 앱 detect→deploy · US2(P2) env scope 선언+검증 · US3(P3) Dockerfile/compose 감지.

---

## Phase 1: Setup

- [ ] T001 [P] Backend: `go get github.com/railwayapp/railpack/core@v0.25.0` exact pin 추가 + `go mod tidy` (app-hub-backend `go.mod`/`go.sum`) — research R1
- [ ] T002 [P] Backend: Railpack compile spike test (`app-hub-backend/internal/service/deploy/railpack_compile_test.go`) — `core.GenerateBuildPlan`, `app.NewApp`, `app.NewEnvironment`, `plan.BuildPlan` import/shape 고정
- [ ] T003 [P] Backend: 6-언어 fixture repo 생성 (`app-hub-backend/internal/service/deploy/testdata/migrate-fixtures/{node,python,go,ruby,java,kotlin}/`) — golden 입력
- [ ] T004 [P] Plugin: `bun run skill:new migrate --model sonnet` 로 `skills/migrate/SKILL.md` 스캐폴드 + registry stub 생성
- [ ] T005 [P] Helper: `crates/axhub-helpers/src/migrate_plan.rs` stub + `main.rs`/typed clap 에 `migrate-plan --dir <path> [--app-path <candidate>] --json` subcommand/USAGE 등록
- [ ] T006 [P] CLI(Rust): ax-hub-cli touchpoint audit — `crates/axhub-manifest`, `axhub/src/commands/*`, `crates/axhub-api` 에 필요한 변경 파일 목록 기록

---

## Phase 2: Blocking Readiness Gates (구현 전 필수)

- [ ] T007 Backend spike: 6 fixture 중 Node+Java+Kotlin 에서 `GenerateBuildPlan` 출력 snapshot 생성, provider 이름/StartCmd/Steps shape 고정 — 실패하면 Railpack version 또는 provider override 결정
- [ ] T008 Backend spike: `plan.BuildPlan` → Dockerfile PoC golden 1차 — Step/Layer/Cache/Deploy.StartCmd 매핑 가능 범위와 fallback 조건 문서화 (R2)
- [ ] T009 Backend spike: Kotlin/Gradle-KTS fixture 감지 확인 — `DetectedProviders` 에 java 또는 kotlin, build/start 정상; 미흡 시 preset/provider override task 추가 (R3)
- [ ] T010 Backend spike: `strategy:auto` 빌드시점 재감지 테스트 스캐폴드 — manifest 존재 + 코드 변경 시 auto=재감지, pinned=고정 기대값 먼저 작성 (R4)
- [ ] T011 Backend API contract test: `POST /api/v1/apps/detect` handler/client 계약 — auth/RBAC, `github_repo|archive` payload, size/path/depth limits, redaction, rate limit, timeout (contracts/backend §2)
- [ ] T012 Backend security test: env scope filtering invariant — runtime-only secret 이 Kaniko/Cloud Build `--build-arg` 로 안 나가고, build-only secret 이 runtime env 로 안 들어감 (contracts/backend §4/§8)

**Checkpoint**: T007-T012 green 또는 명시적 결정 기록 전에는 US1 backend 구현 착수 금지.

---

## Phase 3: Foundational (BLOCKING — manifest/schema/canonical 이름. 모든 스토리가 의존)

- [ ] T013 Backend test: manifest parser golden (`app-hub-backend/internal/service/deploy/manifest_parser_test.go`) — version/strategy/env-scope 파싱 + backward-compat(version/env 없음) + unknown-field reject. **먼저 작성, 실패 확인**
- [ ] T014 Backend: `domain.AppManifest` 확장 (`app-hub-backend/internal/domain/app_spec.go`) — `Version`, `Build.Strategy`, `Env{Required,Optional []AppManifestEnvVar{Name,Scope}}` + `ToSpecData` 매핑 (data-model §2)
- [ ] T015 Backend: `ParseManifest` 검증 (`app-hub-backend/internal/service/deploy/manifest_parser.go`) — `strategy∈{auto(기본),pinned}`, `scope∈{build,runtime(기본),both}`, `name` non-empty+개행금지, `KnownFields(true)` 유지 (data-model §4)
- [ ] T016 CLI(Rust) test: `crates/axhub-manifest` canonical filename 전환 — 신규 생성/검색은 `axhub.yaml`, 전환기 `apphub.yaml` dual-read 유지, 둘 다 있으면 `axhub.yaml` 우선 + 경고
- [ ] T017 CLI(Rust): `crates/axhub-manifest/src/lib.rs` + 관련 commands/docs codegen 에 `axhub.yaml` canonical 반영, 기존 `apphub.yaml` 읽기 유지
- [ ] T018 [P] Migration: 기존 템플릿 + w5-contracts + docs 의 user-facing manifest 예시를 `axhub.yaml` 로 전환, `apphub.yaml` 은 legacy/dual-read 문구로만 유지 (FR-015)

**Checkpoint**: T013-T018 green → 스토리 페이즈 착수 가능.

---

## Phase 4: User Story 1 — 기존 앱 detect→plan→deploy (Priority: P1) 🎯 MVP

**Goal**: 기존(non-saga) 앱을 소스 수정 없이 detect→plan(confidence)→확인→register→connect→deploy.
**Independent test**: Node 웹앱을 빈 axhub 앱 없이 `axhub migrate .` → 감지 → 확인 → 배포 성공.

- [ ] T019 [P] [US1] Backend test: railpack adapter golden (`app-hub-backend/internal/service/deploy/railpack_adapter_test.go`) — 6 fixture → `GenerateBuildPlan` → `plan.BuildPlan` → Dockerfile 스냅샷 (T008 결과 반영)
- [ ] T020 [US1] Backend: `railpack_adapter.go` (`app-hub-backend/internal/service/deploy/railpack_adapter.go`) — `plan.BuildPlan` → Dockerfile 변환 + fallback 조건 (contracts/backend §3)
- [ ] T021 [US1] Backend: `GenerateBuildPlan` 빌드 파이프라인 통합 + ladder(explicit manifest override > repo Dockerfile > compose > Railpack auto > 기존 `generateDockerfile` fallback) (`app-hub-backend/internal/service/deploy/`)
- [ ] T022 [US1] Backend: `POST /api/v1/apps/detect` 구현 — `detected_providers`/install/build/start/port/**confidence**/env_refs + redacted logs 반환 (contracts/backend §2)
- [ ] T023 [P] [US1] Helper test: `migrate-plan` pre-scan (`crates/axhub-helpers/tests/cli_e2e.rs`) — monorepo 후보 열거, stack_hint, dockerfile/compose 존재
- [ ] T024 [US1] Helper: `migrate_plan.rs` (`crates/axhub-helpers/src/migrate_plan.rs`) — 로컬 light pre-scan(monorepo 후보, stack hint, container contract 존재). 권위 감지 안 함 (contracts/helper-migrate-plan)
- [ ] T025 [US1] Skill: `skills/migrate/SKILL.md` 워크플로 — preflight→`migrate-plan`→(monorepo 후보 선택)→archive upload 또는 GitHub repo-ref plan-preview→confidence 카드→manifest 생성→`apps create --from-file`→git connect→deploy. 해요체/D1 가드/in-body preflight (FR-001,004,013,014)
- [ ] T026 [US1] Skill: confidence 분기 — high(`>=0.80`)=확인 후 진행 / medium(`0.60..0.79`)=editable 확인 카드 / low(`<0.60`)·모호·필수 command 누락=차단+명시 입력 요청(`axhub.yaml`/Dockerfile/직접) (FR-003)
- [ ] T027 [US1] Skill: monorepo 후보 AskUserQuestion 선택 → 선택 앱마다 별도 등록 (FR-017)
- [ ] T028 [P] [US1] Test: routing corpus (`tests/`) — "기존 앱 올려줘"/"migrate" → migrate skill hit, deploy/apps 로 안 샘

**Checkpoint**: US1 단독 배포 성공 = MVP 전달.

---

## Phase 5: User Story 2 — env scope 선언 + 배포 전 검증 (Priority: P2)

**Goal**: required env(이름+scope) 선언, 배포 전 검증, silent 실패 차단.
**Independent test**: required `DATABASE_URL` 미설정 앱 배포 시도 → 배포 전 명확히 차단.

- [ ] T029 [US2-GATE] R6 재현 또는 범위 축소 — 막힌 앱 1개로 env/egress/localhost 원인을 기록. 재현 불가하면 US2/SC 문구를 env-only fail-loud 로 좁힘 (research R6)
- [ ] T030 [P] [US2] Backend test: env scope storage/API roundtrip (`app-hub-backend/internal/server/handler/deploy/env_vars_test.go` 또는 service test) — 기본 runtime, build/runtime/both create/update/list
- [ ] T031 [US2] Backend: env store/API migration — 기존 rows `scope=runtime` backfill, `DeploymentEnvVar.Stage` 와 별도 scope 저장/노출 (contracts/backend §8)
- [ ] T032 [P] [US2] Backend test: preflight env 교차검증 golden (`app-hub-backend/internal/service/deploy_preflight_test.go`) — required(build/runtime) ⊆ env-store, 미설정→fail, build=빌드전·runtime=배포전
- [ ] T033 [US2] Backend: `RunPreflightChecks` 에 appID+`ListEnvVars(appID)`+scope 필터 배선 + `required ⊆ set` 교차검증(scope 별) + 한국어 fail (`app-hub-backend/internal/service/deploy_preflight.go`) (contracts §5, FR-006)
- [ ] T034 [US2] Backend: build vs runtime env 주입 필터링 — `scope∈{build,both}`→Dockerfile ARG/ENV, `scope∈{runtime,both}`→Cloud Run env (`railpack_adapter.go` + `injectAppHubEnvVars`) (contracts §4, FR-005~007)
- [ ] T035 [P] [US2] Helper test: env-ref 스캔 (`crates/axhub-helpers/tests/cli_e2e.rs`) — `process.env.X`/`os.environ`/`ENV[]` + scope 휴리스틱(`NEXT_PUBLIC_`/`VITE_`→build)
- [ ] T036 [US2] Helper: `migrate_plan.rs` env-ref 정적 스캔 + scope 추정 (값 미출력) (contracts/helper-migrate-plan)
- [ ] T037 [US2] Skill: 감지된 required env → `axhub env set` 안내, build-scoped 미설정 시 빌드 전 차단 (FR-006, US2 sc1)

**Checkpoint**: US1 + US2 독립 동작.

---

## Phase 6: User Story 3 — Dockerfile / docker-compose 감지·존중 (Priority: P3)

**Goal**: 사용자 Dockerfile/compose 감지·우선, ladder, compose web-service 식별.
**Independent test**: (a) Dockerfile 앱 → Dockerfile 사용 / (b) compose 앱 → web 서비스 식별 배포.

- [ ] T038 [P] [US3] Backend test: ladder + compose web-service 식별(`build:`로컬+포트=web, `image:`전용=외부) + Dockerfile 우선 (`app-hub-backend/internal/service/deploy/railpack_adapter_test.go`)
- [ ] T039 [US3] Backend: compose 감지 + `deploy_method=compose`, web 서비스 식별, 백킹=외부(provisioning 범위 밖) (`app-hub-backend/internal/service/deploy/`) (contracts §7, FR-009)
- [ ] T040 [US3] Backend: build/deploy ladder ①`axhub.yaml` 명시 override ②Dockerfile ③compose ④Railpack auto (`railpack_adapter.go`) (FR-008)
- [ ] T041 [US3] Helper: `has_dockerfile`/`has_compose` pre-scan 결과 (`migrate_plan.rs`)
- [ ] T042 [US3] Skill: ladder + compose web-service 선택 결과를 사용자에게 표시(무엇을 쓰는지) (FR-008, US3 sc2/3)

**Checkpoint**: 3 스토리 모두 독립 동작.

---

## Phase 7: Polish & Cross-Cutting

- [ ] T043 [P] Backward-compat test: 기존 saga 앱 + legacy `apphub.yaml` 여전히 배포 (go test + Rust CLI tests + bun test) (SC-004)
- [ ] T044 [P] 한국어 error-empathy-catalog 항목 추가 — migrate 실패(빌드/감지/env 누락) (`skills/deploy/references/error-empathy-catalog.md` regen)
- [ ] T045 [P] AskUserQuestion registry 등록 (`tests/fixtures/ask-defaults/registry.json`) — monorepo 선택/confidence 확인/env 질문
- [ ] T046 [P] Skill quality gates: `bun run skill:doctor --strict`, `lint:tone --strict`, `lint:keywords --check`, `bun test`, `bunx tsc --noEmit` (skills/migrate)
- [ ] T047 Quickstart 검증: `quickstart.md` 흐름을 fixture 앱으로 e2e 실행
- [ ] T048 Success Criteria measurement: 6-언어/Dockerfile/compose fixture 첫 시도 감지율 ≥80%, migrate→확인→deploy ≤5분 smoke 측정 (SC-002, SC-005)

---

## Dependencies & 실행 순서

```
Setup(T001-T006)
  → Readiness Gates(T007-T012, BLOCKING)
  → Foundational(T013-T018, BLOCKING)
       ↓
   US1(T019-T028)  →  US2(T029-T037)  →  US3(T038-T042)
       ↓                         ↓
              Polish(T043-T048)
```

- **Readiness Gates(T007-T012)** 는 Railpack/API/env security 현실성을 먼저 고정해요.
- **Foundational(T013-T018)** 은 manifest 스키마 + `axhub.yaml` canonical 전환이라 모든 스토리를 차단해요.
- US1 → US2 → US3 우선순위 순. US2/US3 는 US1 의 manifest/adapter 위에 얹힘(env·compose 는 adapter 확장).
- R6(데이터 호출 막힘)은 US2 진입 전 재현 또는 scope 축소가 필요해요.

## 병렬 기회

- **Setup**: T001-T006 대부분 [P] (다른 repo/파일).
- **Readiness Gates**: T007/T008/T009/T010/T011/T012 는 서로 다른 테스트/계약이라 병렬 가능하되, T008 결과는 T019/T020 이 소비해요.
- **테스트 우선(TDD)**: 각 페이즈의 `[P]` test task(T019/T023/T030/T032/T035/T038)를 impl 전 병렬 작성.
- **Polish**: T043-T046 대부분 [P].
- 같은 파일 수정 task(예: `railpack_adapter.go` T020/T034/T040)는 순차.

## MVP scope

**US1(T001-T028)** = 최소 제품: 기존 앱 detect→확인→배포. env scope(US2)·compose(US3) 없이도 단독 가치. 단, T007-T012/T013-T018 gate 는 MVP 전 필수예요.

## 구현 전략

1. Setup + Readiness Gates + Foundational → green.
2. **US1 끝까지 = MVP ship** (Node 앱 1개 배포 성공).
3. US2(env scope) → US3(compose) 증분.
4. Polish + SC measurement. 각 스토리는 독립 배포/테스트 가능.
