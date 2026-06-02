# Phase 0 Research: 기존 앱 migrate

> spec.md 의 Clarifications + Outstanding 항목을 결정/스파이크로 정리해요.

## R1. v1 감지 엔진 = axhubpack (Railpack `/core` 채택) — **해소**

- **Decision**: `github.com/railwayapp/railpack/core` 를 backend(Go) 의존성으로 `go get github.com/railwayapp/railpack/core@v0.25.0` exact pin 채택. `GenerateBuildPlan(app, env, opts) *BuildResult` 로 감지+빌드 plan 생성.
- **Rationale** (pkg.go.dev + `go list -m -versions` 검증(2026-06-01 기준 v0.25.0 존재)):
  - public go-gettable Go 패키지(internal 아님), **MIT** → fork 불필요, dependency 로 사용.
  - 출력 = **추상 `plan.BuildPlan`** (BuildKit-LLB 아님) + `DetectedProviders []string` → axhub 의 Kaniko 빌드와 충돌 없이 Dockerfile 로 변환 가능.
  - providers: golang/node/python/**ruby/java**/php/deno/elixir/rust/cpp/dotnet/... → v1 6언어 커버.
  - axhub backend 가 Go 라 동일 언어, in-process 통합 자연스러움.
- **Alternatives rejected**:
  - hard-fork → 유지보수 부담(upstream 언어 추가 못 받음). MIT+go-gettable 이라 불필요.
  - from-scratch axhubpack → 수년짜리 재발명.
  - 기존 `detector_service` preset 만 → Node/Python/Go 만, Ruby/Java/Kotlin(특히 JVM) 직접 작성은 fiddly.
  - BuildKit(Railpack 기본 실행) 채택 → 기존 Kaniko/Cloud Run 전면 교체, 과도.

## R2. `plan.BuildPlan` → Dockerfile 어댑터 — **BLOCKING 스파이크**

- **Decision**: backend `internal/service/deploy/railpack_adapter.go` 신규. `plan.BuildPlan`(step/layer) → Dockerfile 텍스트 → 기존 `generateDockerfile`/Kaniko 경로 재사용.
- **Open**: `plan.BuildPlan` 의 정확한 step/layer struct(설치/빌드/실행/caching 표현)를 읽고 매핑 PoC 필요. 추상 plan 이라 변환 자체는 feasible(검증됨), 매핑 충실도가 스파이크. **US1 구현 전 blocking gate** 로 둬요.
- **Verify**: golden test — 6언어 fixture repo → GenerateBuildPlan → adapter → Dockerfile 스냅샷.

## R3. Kotlin 커버리지 — **BLOCKING 스파이크**

- **Decision(잠정)**: Railpack `java` provider 가 JVM 빌드(Gradle/Maven) 담당 → Kotlin(`build.gradle.kts`)도 Gradle 경로로 커버 추정.
- **Verify**: Kotlin/Gradle-KTS fixture 로 `DetectedProviders` 에 java(또는 kotlin) 잡히고 build/start 명령 정상인지 확인. 미흡 시 provider override 또는 preset 보강. **US1 adapter 구현 전 blocking gate** 로 둬요.

## R4. `build.strategy: auto` backend 배선 — **BLOCKING 스파이크**

- **Decision**: `auto`(기본)는 manifest 가 있어도 빌드 시점에 Railpack 재감지를 돌려 install/build/start 를 산출(코드 변경 자동 적응). `pinned` 은 committed 명령 고정. `env`/`port`/`dockerfile` override 는 두 모드 공통.
- **Open**: backend 현재는 "manifest 있으면 그대로 사용 / 없으면 detector". `strategy` 필드가 "manifest 있어도 재감지" 를 게이트하도록 빌드 파이프라인 분기 추가 필요.
- **Verify**: 같은 repo 에 코드 변경(dep 추가) 후 auto=재감지 반영 / pinned=고정 확인. **빌드 파이프라인 수정 전 테스트 스캐폴드** 로 고정해요.

## R5. detect 배치 (migrate-time 미리보기) — **결정**

- **Decision**: 권위 감지 = **backend**(Railpack /core, Go). migrate-time 미리보기도 backend 의 **`POST /api/v1/apps/detect` plan-preview 엔드포인트**(GitHub repo-ref 또는 업로드 archive → `GenerateBuildPlan` dry → DetectedProviders+commands+confidence 반환)로 통일 — 중복 0.
  - helper(`migrate_plan.rs`)는 **로컬 light pre-scan** 만: monorepo 후보 디렉터리 열거, Dockerfile/compose 존재 여부, env 참조(`process.env.X`/`os.environ`) 스캔. 권위 감지는 안 함.
- **Rationale**: Railpack 이 Go → backend in-process 가 자연스럽고, `auto` 빌드시점 감지와 동일 엔진 재사용(스냅샷 drift 회피).
- **Alternatives rejected**: helper 가 Railpack 로직 미러(중복/drift) → 거부. helper shell-out to railpack 바이너리(추가 배포물) → backend in-process 보다 무거움.

## R6. "데이터 호출 막힘" 근본원인 — **재현 필요(미해소)**

- **Open**: 자체 외부 DB 못 닿는 게 (a) env 미설정 / (b) Cloud Run egress 제한 / (c) `localhost` 바인딩 중 무엇인지 미확정.
- **Plan**: 막힌 앱 1개를 손으로 axhub 에 올려 재현 → 원인 확정 후 migrate 의 안내/검증 강화. env 스코프 검증(R 아래)이 (a)의 절반을 이미 커버.
- **Note**: Cloud Run 은 egress 기본 열림이나 VPC connector egress 설정이 terraform 에 있을 수 있어 코드만으론 확정 불가.

## R7. manifest 이름 = `axhub.yaml` (정준) — **결정(clarify)**

- **Decision**: 신규 생성 = `axhub.yaml`. backend dual-read(`apphub.yaml`+`axhub.yaml`) 유지. 기존 템플릿·w5-contracts·docs 를 `axhub.yaml` 로 마이그레이션, 전환기 `apphub.yaml` 수용.

## R8. env scope (build/runtime) — **결정(clarify)**

- **Decision**: `env.required[]`/`optional[]` 각 항목에 `scope: build|runtime|both`(기본 runtime). build-scoped 는 Kaniko 빌드 단계 ARG/ENV 주입, runtime-scoped 는 Cloud Run 컨테이너 env. preflight 가 build 는 빌드 전·runtime 은 배포 전 교차검증.

## 미해소 요약 (구현 중 스파이크/재현)

| # | 항목 | 유형 |
|---|---|---|
| R2 | plan.BuildPlan→Dockerfile 매핑 충실도 | **US1 전 blocking** PoC+golden |
| R3 | Kotlin/Gradle-KTS 커버 | **US1 전 blocking** fixture |
| R4 | strategy:auto 빌드시점 재감지 배선 | **US1 전 blocking** 테스트 스캐폴드 |
| R6 | 데이터 호출 막힘 원인 | **US2 전** 재현(앱 1개) 또는 env-only scope 축소 |
