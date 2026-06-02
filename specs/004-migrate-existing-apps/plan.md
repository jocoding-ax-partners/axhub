# Implementation Plan: 기존 앱 migrate (Migrate Existing Apps to axhub)

**Branch**: `feat/migrate-existing-apps` | **Date**: 2026-06-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/004-migrate-existing-apps/spec.md`

## Summary

기존(axhub saga 미사용) 앱을 소스 수정 없이 axhub 에 배포하는 migrate 기능이에요. 소스 변환 컴파일러가 아니라 **detect → plan → manifest(`axhub.yaml`) → deploy** 오케스트레이션이에요.

기술 접근:
- **v1 감지 엔진 = axhubpack** — `github.com/railwayapp/railpack/core`(MIT, Go) 를 backend 의존성으로 채택. `GenerateBuildPlan()` 이 반환하는 **추상 `plan.BuildPlan`** 을 **Dockerfile 로 변환**(어댑터)해서 기존 Kaniko/Cloud Run 빌드에 연결해요. BuildKit 미채택, fork 미채택.
- 6 언어(Node/Python/Go/Ruby/Java/Kotlin) — Railpack provider 로 커버.
- `axhub.yaml`(정준 manifest): `version` + `env{required,optional,scope}` + `build.strategy: auto|pinned`. backend 가 `apphub.yaml` 도 dual-read.
- 레이어: **skill(thin) → axhub-helpers(Rust) → axhub CLI → backend(Go)**.

## Technical Context

**Language/Version**: TypeScript/Bun (plugin skills + tests), Rust (axhub-helpers + ax-hub-cli workspace), Go 1.22+ (backend app-hub-backend)

**Primary Dependencies**:
- **`github.com/railwayapp/railpack/core` v0.25.0 (MIT, exact pin)** — 언어/프레임워크 감지 + build plan (신규 backend 의존성). API drift 방지를 위해 compile spike 로 `GenerateBuildPlan`/`plan.BuildPlan` 구조를 먼저 고정해요.
- 기존 재사용: Kaniko 빌더 + Cloud Run deployer, `detector_service`(Preset), `generateDockerfile`, consent-mint, encrypted env store, `apps`/`deploy`/`env` CLI

**Storage**: backend encrypted env store(env 값) · `axhub.yaml`(repo, 빌드/런타임/env-이름 계약) · `~/.cache/axhub-plugin`(statusline)

**Testing**: `bun test`(plugin/skill 계약) · `cargo test`(axhub-helpers) · `go test`(backend: parser/adapter/preflight golden)

**Target Platform**: Claude Code (macOS/Linux/Windows) plugin 실행 · Google Cloud Run(배포 런타임) + Kaniko(빌드)

**Project Type**: multi-repo (axhub plugin + app-hub-backend + ax-hub-cli)

**Performance Goals**: migrate→확인→배포 ≤5분(SC-005) · 6-언어 감지 첫 시도 ≥80%(SC-002) · build-scoped env 누락 0건 silent(SC-003) · JVM(Gradle/Maven) 빌드는 더 무거움 — resource_tier 영향 검토

**Constraints**:
- 모든 mutation(apps create / git connect / deploy)은 기존 consent-mint 게이트 통과(FR-013)
- 별도 평행 배포 경로 금지, 기존 인프라 재사용(FR-014)
- backward-compat: 기존 saga 앱 + `apphub.yaml` 무중단(FR-010, SC-004)
- skill = thin orchestration, 테스트 로직은 helper/backend (axhub CLAUDE.md skill-authoring)

**Scale/Scope**: v1 = 6 언어 + Dockerfile/compose escape hatch · single-service deploy(monorepo→후보 선택, 각 앱 별도 등록) · managed resource(DB/redis) 범위 밖

## Constitution Check

`.specify/memory/constitution.md` 는 **미작성 placeholder 템플릿**이라 ratified 원칙 gate 가 없어요. 대신 axhub 의 de-facto governance(루트 `CLAUDE.md`)를 게이트로 적용해요:

| de-facto 원칙 | 본 plan 준수 |
|---|---|
| skill = thin orchestration, 테스트 로직 임베드 금지 | ✅ 감지/생성/검증은 helper(Rust)·backend(Go), skill 은 AskUserQuestion/TodoWrite/렌더만 |
| mutation = consent-mint 게이트 | ✅ apps create / git connect / deploy 기존 게이트 재사용(FR-013) |
| hook fail-open(exit 0) | ✅ 신규 hook 없음(migrate 는 skill 흐름) |
| no source-compiler, container/manifest 모델 | ✅ Railpack plan→Dockerfile→Kaniko, 소스 무변환 |
| simplicity / YAGNI | ✅ managed-resource defer, axhubpack=dependency(fork 아님), 기존 generateDockerfile/Kaniko 재사용 |
| 한글 해요체, scaffold(`bun run skill:new`) | ✅ migrate skill 작성 시 강제 패턴 준수 |

**Gate: PASS** (위반 0). Complexity Tracking 불필요.

## Project Structure

### Documentation (this feature)

```text
specs/004-migrate-existing-apps/
├── spec.md              # 완료 (9 clarifications)
├── plan.md              # 이 문서
├── research.md          # Phase 0 — Railpack 채택 + 어댑터 결정
├── data-model.md        # Phase 1 — axhub.yaml 스키마 + 엔티티
├── contracts/           # Phase 1 — manifest 스키마 / helper subcommand / backend API
│   ├── axhub-yaml-schema.md
│   ├── helper-migrate-plan.md
│   └── backend-railpack-integration.md
└── quickstart.md        # Phase 1 — 사용자 migrate 흐름
```

### Source Code (multi-repo)

```text
# axhub plugin (this repo) — thin orchestration
skills/migrate/
└── SKILL.md             # `bun run skill:new migrate --model sonnet` (scaffold)
                         # detect→plan(confidence)→manifest→env→register→connect→deploy

crates/axhub-helpers/src/
├── migrate_plan.rs       # 신규: 로컬 pre-scan(monorepo 후보, 빠른 stack 힌트), env 참조 스캔
└── main.rs              # subcommand 등록 (migrate-plan)

# app-hub-backend (Go) — detection engine + schema + build
internal/domain/app_spec.go              # version / env(scope) / strategy 필드 추가
internal/service/deploy/
├── railpack_adapter.go  # 신규: railpack/core GenerateBuildPlan → plan.BuildPlan → Dockerfile
├── detector_service.go  # 기존 preset (fallback / override)
└── manifest_parser.go   # ParseManifest + env scope / strategy 검증
internal/service/deploy_preflight.go     # env required ⊆ env-store 교차검증 (build / runtime scope)

# ax-hub-cli (Rust) — command/client integration + manifest canonicalization
crates/axhub-manifest/src/lib.rs     # canonical filename: axhub.yaml, dual-read apphub.yaml
axhub/src/commands/                  # apps create --from-file / git connect / deploy / env (기존 명령 재사용)
crates/axhub-api/                    # plan-preview API client 타입/호출 추가 시 위치
```

**Structure Decision**: multi-repo. migrate 의 "두뇌"(감지 plan)는 **backend(Go)** 에 둬요 — Railpack `/core` 가 Go 라 backend 가 자연스러운 위치이고, `auto` strategy 의 빌드 시점 재감지도 어차피 backend 에 있어야 해요. plugin skill 은 thin orchestration, axhub-helpers 는 migrate-time 미리보기용 light pre-scan(monorepo 후보 열거 등)만 담당해요.

## Complexity Tracking

> Constitution Check PASS — 위반 없음. 빈 표.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| (없음) | — | — |

## Phase 0: Outline & Research → [research.md](./research.md)

해소 대상 unknown:
1. **[해소] Railpack `/core` 채택 가능성** — public go-gettable Go pkg(v0.25.0, MIT), `GenerateBuildPlan`→추상 `plan.BuildPlan`. fork/BuildKit 불필요.
2. **[BLOCKING spike] `plan.BuildPlan` → Dockerfile 어댑터** — plan step/layer 구조 매핑을 PoC + golden 으로 먼저 고정해요.
3. **[BLOCKING spike] Kotlin 커버리지** — Railpack `java` provider 가 `build.gradle.kts`/Kotlin 을 감지하는지 fixture 로 먼저 확인해요.
4. **[BLOCKING spike] `strategy: auto` backend 배선** — manifest 존재해도 빌드 시점 재감지하는 분기를 구현 전에 테스트 스캐폴드로 고정해요.
5. **[BLOCKING contract] detect 배치** — 미리보기는 `POST /api/v1/apps/detect` 단일 route 로 고정하고 auth/RBAC/payload/limit/redaction/rate-limit 를 계약화해요.
6. **[pre-US2 reproduce] 데이터 호출 막힘 근본원인** — egress/localhost/env 중 막힌 앱 1개 재현으로 확정하거나, US2/SC 를 env-only fail-loud 로 명시 축소해요.

## Phase 1: Design & Contracts → [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

- `data-model.md`: `axhub.yaml`(AppManifest) 확장 스키마 + Env Contract(scope) + Deploy Plan(confidence) + Existing App(monorepo 후보) + Container Contract(Dockerfile/compose).
- `contracts/`: (a) `axhub.yaml` 스키마, (b) helper `migrate-plan` subcommand I/O, (c) backend Railpack 통합 + plan→Dockerfile + preflight env 교차검증.
- `quickstart.md`: `axhub migrate .` → 감지 → 확인 → env set → 배포 사용자 흐름.


## Ralph Review Readiness Gates (2026-06-01)

Architect review 결과 `CHANGES_REQUESTED` 로 확인된 구현 전 gate 예요. 아래가 해소되기 전에는 구현 착수 대신 plan/tasks 갱신을 먼저 해요.

1. **Cross-repo reality gate**: `ax-hub-cli` 는 Go 가 아니라 Rust workspace 예요. CLI 관련 작업은 `crates/axhub-manifest`, `axhub/src/commands/*`, `crates/axhub-api` 기준으로 작성하고, `apphub.yaml`→`axhub.yaml` canonical 전환은 Foundational 로 이동해요.
2. **Railpack exact-pin gate**: `v0.25.0` exact pin 으로 compile spike 를 먼저 돌려 `GenerateBuildPlan`/`plan.BuildPlan`/providers 출력 shape 를 검증해요.
3. **Adapter/auto gate**: R2/R3/R4 는 Polish 가 아니라 US1 backend 구현 전 blocking gate 예요.
4. **Env security gate**: build/runtime/both scope 는 저장소/API/빌드 args/런타임 env 필터링까지 포함해요. runtime secret 이 Docker build arg 로 전달되지 않는 회귀 테스트를 먼저 추가해요.
5. **Detect API gate**: `apps:detect` 또는 `deploy-prep` 같은 열린 선택지를 제거하고 `POST /api/v1/apps/detect` 로 계약을 고정해요. local dir 은 backend 가 직접 읽을 수 없으므로 archive upload 또는 GitHub repo-ref 중 하나의 payload 로만 전달해요.

## Phase 2

작업 분해는 [tasks.md](./tasks.md) 에서 TDD 순서(parser/adapter/API/security golden → 구현)로 관리해요. Ralph review 이후 readiness gate 는 구현 전 필수 조건이에요.
