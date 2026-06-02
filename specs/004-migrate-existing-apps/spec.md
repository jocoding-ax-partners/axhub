# Feature Specification: 기존 앱 migrate (Migrate Existing Apps to axhub)

**Feature Branch**: `004-migrate-existing-apps`

**Created**: 2026-06-01

**Status**: Draft

**Input**: User description: "이미 만들어진(axhub saga 미사용) 앱을 axhub 인프라에 배포 가능하도록 받아들이는 기능. 소스 변환 컴파일러가 아니라 migrate → detect → plan → deploy 오케스트레이션. 표준 manifest = `axhub.yaml`. Dockerfile 과 docker-compose 도 감지·지원."

## Clarifications

### Session 2026-06-01

- Q: v1 빌드 전략 — "임의 스택 100% 자동"(D)을 어떻게 달성하나? buildpack 엔진(axhubpack)을 직접 만드나? → A: **v1 = 기존 detect→Dockerfile 생성(backend `generateDockerfile`) 확장 + Dockerfile/docker-compose escape hatch** (= 임의 스택 배포 가능, 새 엔진 0). 본격 buildpack 은 **Railpack `/core`**(MIT, Go, detection/plan 이 `/buildkit` 실행과 분리됨)를 채택해 **build plan → Dockerfile 어댑터로 기존 Kaniko 빌드에 연결**하는 "axhubpack" 으로 확장해요. (※ 처음엔 fast-follow 로 봤으나, 이후 6-언어(Java/Kotlin 포함) 결정으로 axhubpack 을 **v1 감지 엔진으로 채택** — 아래 Clarification 참조.) Railpack 의 BuildKit 실행 레이어는 가져오지 않아요(axhub = Kaniko/Cloud Run). fork 보다 Go dependency 채택을 우선해요(upstream 언어 지원 흡수, fork 유지보수 회피).
- Q: docker-compose 다중 서비스에서 어느 서비스를 배포하나? → A: **`build:` 가 로컬 컨텍스트 + 포트 노출인 서비스를 web(배포 대상)으로 식별**해요. `image:` 전용 서비스(postgres/redis 등)는 외부 백킹으로 취급(자체 외부 데이터 가정과 일관). web 후보가 2개 이상이면 자동 추론하지 말고 사용자에게 확인받아요.
- Q: `axhub.yaml` 정준화하면 기존 `apphub.yaml` 은? → A: **지금 기존 템플릿·in-flight w5-contracts·docs 를 `axhub.yaml` 로 마이그레이션**해요(C). 전환 기간 `apphub.yaml` 은 dual-read 로 계속 수용해서 무중단을 유지해요.
- Q: migrate 진입 소스 범위? → A: **현재 디렉터리(local) + (연결/연결할) GitHub repo**(B) — axhub 의 기존 git-connected 배포 모델과 일관해요. 임의 원격 git URL 직접 clone·prebuilt 이미지 소스는 v1 범위 밖이에요.
- Q: 사용자 코드가 바뀌면 `axhub.yaml` 스냅샷이 stale 되나? → A: **`build.strategy: auto`(기본)** — backend 가 **빌드마다 재감지**(기존 `detector_service` + `generateDockerfile`)해서 코드 변경(dep/framework/package-manager/start)에 자동 적응해요(Railpack zero-config 동작과 동일). `pinned` opt-in 은 명시 고정(예측 가능하나 구조 변경 시 stale → re-migrate 안내). axhubpack(Railpack `/core`)이 이 build-time auto 경로의 **v1 감지 엔진**이에요.
- Q: v1 지원 언어(Node/Python/Go/Ruby/Java/Kotlin) → axhubpack 채택 시점? → A: **axhubpack(Railpack `/core`)을 v1 감지 엔진으로 채택**(option A) — Java/Kotlin(JVM, Maven/Gradle) 같은 감지를 직접 재발명하지 않으려고요. 이 결정이 위 Q1·Q5 의 "axhubpack = fast-follow" 를 **supersede**해요(axhubpack 은 v1). Railpack `/buildkit` 은 미채택, build plan→Dockerfile→기존 Kaniko 유지. Dockerfile/compose 는 escape hatch 로 병행.
- Q: 환경변수의 build-time vs runtime 구분? → A: env 항목에 **`scope: build|runtime|both`(기본 runtime)** — build-scoped 는 빌드 단계에 주입(`NEXT_PUBLIC_*`/`VITE_*` 등 이미지에 baked), runtime-scoped 는 컨테이너 시작 시 주입. 둘 다 배포 전 검증해요(FR-006).
- Q: 감지 신뢰도가 낮을 때 동작? → A: **confidence 기반 분기**(A) — high 면 추정값 보여주고 확인 후 진행, low/모호면 **진행을 막고 명시 입력 요청**(`axhub.yaml`/Dockerfile/직접 입력). silent 진행 금지.
- Q: monorepo / 감지 위치? → A: **monorepo 자동 감지**(B) — 배포 가능한 앱 후보들을 제시하고 사용자가 선택해요. 선택한 각 앱은 **별도 axhub 앱**으로 migrate(단일 앱 = 단일 서비스 모델 유지). 자동 다중 배포는 안 해요.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - 기존 앱을 그대로 axhub 에 올리기 (Priority: P1)

개발자가 이미 만들어 둔 앱(axhub saga 로 scaffold 하지 않은)을 소스 수정 없이 axhub 에 배포해요. axhub 가 앱의 스택/빌드/실행 방식을 자동 감지하고, 추정한 배포 계획을 보여주고, 사용자가 확인하면 배포해요.

**Why this priority**: 이게 기능의 본질 — "이미 가진 앱을 가져와서 올린다". 이것만 있어도 단독 가치를 주는 MVP 예요.

**Independent Test**: 흔한 스택(예: Node 웹앱) 1개를 빈 axhub 앱 없이 migrate → detect → 확인 → 배포 성공까지 단독으로 검증할 수 있어요.

**Acceptance Scenarios**:

1. **Given** axhub saga 로 만들지 않은 기존 웹앱 소스, **When** 사용자가 migrate 를 실행, **Then** axhub 가 프레임워크·빌드·실행·포트를 감지해 배포 계획을 한국어로 보여줘요.
2. **Given** 감지된 배포 계획, **When** 사용자가 확인(동의), **Then** 앱이 소스 수정 없이 axhub 에 배포돼 라이브 URL 을 받아요.
3. **Given** 감지 신뢰도가 낮거나 모호한 앱, **When** migrate, **Then** 진행을 막고 명시 입력(`axhub.yaml`/Dockerfile/직접 입력)을 요청해요. (high-confidence 면 추정값을 확인받고 진행.)
4. **Given** migrate 로 배포된 앱(`strategy: auto`), **When** 사용자가 코드를 바꾸고(dep 추가·start 변경 등) 다시 push, **Then** 빌드 시점에 재감지돼 manifest 수동 수정 없이 재배포돼요.

---

### User Story 2 - 필요한 환경변수 선언 + 배포 전 검증 (Priority: P2)

앱이 자체 외부 DB/API 를 쓰려면 `DATABASE_URL` 같은 환경변수가 필요해요. axhub 가 앱이 필요로 하는 환경변수를 (값이 아니라 이름으로) 알게 하고, 배포 전에 실제로 설정됐는지 확인해서, 안 됐으면 조용히 실패하지 않고 명확히 알려줘요.

**Why this priority**: 기존 앱을 올릴 때 "데이터 호출이 막힘"이 자주 silent runtime 실패로 나타나요. 배포 전에 잡으면 디버깅 지옥을 피해요. P1 배포가 된 다음 신뢰도를 올리는 층이에요.

**Independent Test**: required 환경변수를 선언한 앱을 값 미설정 상태로 배포 시도 → 배포 전에 "X 가 필요한데 안 설정됨" 으로 막히는지 단독 검증할 수 있어요.

**Acceptance Scenarios**:

1. **Given** 앱이 required 환경변수 `DATABASE_URL` 을 선언, **And** 값이 설정 안 됨, **When** 배포, **Then** 배포 전에 "`DATABASE_URL` 설정 필요" 로 막고 설정 방법을 안내해요.
2. **Given** required 환경변수가 전부 설정됨, **When** 배포, **Then** 검증을 통과하고 진행해요.
3. **Given** 환경변수 값(secret), **When** 어디서든, **Then** 값은 commit 되는 `axhub.yaml` 에 안 들어가고 안전하게 별도 보관돼요.

---

### User Story 3 - 사용자가 제공한 컨테이너 계약(Dockerfile / docker-compose) 감지·존중 (Priority: P3)

앱에 이미 Dockerfile 또는 docker-compose 파일이 있으면 axhub 가 추론하지 말고 그 파일을 계약으로 감지·존중해요. 둘 다 없으면 프레임워크 자동 감지로 떨어져요.

**Why this priority**: Dockerfile/compose 는 사용자가 이미 "이 앱은 이렇게 빌드·실행된다"고 명시한 계약이에요. 추론보다 우선해야 하고, 자동 감지가 안 되는 임의 스택의 escape hatch 가 돼요.

**Independent Test**: (a) Dockerfile 있는 앱, (b) docker-compose.yml 있는 앱을 각각 migrate → axhub 가 해당 계약을 감지해 빌드 추론을 건너뛰고 그걸로 배포하는지 단독 검증할 수 있어요.

**Acceptance Scenarios**:

1. **Given** repo root 에 Dockerfile, **When** migrate, **Then** 빌드 명령 추론을 건너뛰고 Dockerfile 을 우선 사용해요.
2. **Given** repo 에 docker-compose.yml(compose.yaml/compose.yml 포함), **When** migrate, **Then** compose 를 감지해 compose 배포 방식으로 계획을 세우고, 어떤 서비스를 배포하는지 사용자에게 보여줘요.
3. **Given** `axhub.yaml`·Dockerfile·docker-compose 가 섞여 존재, **When** 배포, **Then** 빌드/배포 우선순위 ladder(FR-008)에 따라 하나를 골라 결정하고 무엇을 쓰는지 사용자에게 알려줘요.

---

### Edge Cases

- 스택을 자동 감지 못 하는 앱(미지원 언어/프레임워크) → Dockerfile/compose 요청 또는 수동 빌드 설정으로 유도해요. 조용한 실패 금지.
- required 환경변수를 선언했는데 미설정 → 배포 전에 명확히 차단해요 (silent runtime crash 금지).
- **build-scoped required env 미설정** → 빌드가 잘못된 산출물을 silent 하게 만들 수 있어요(예: `NEXT_PUBLIC_API_URL` 빠진 채 baked). 빌드 전에 차단·안내해요.
- 앱이 자체 외부 DB/API 에 접속(외부 네트워크 필요) → 배포된 앱이 외부로 나갈 수 있어야 해요(egress). 막히면 명확히 안내.
- **docker-compose 가 다중 서비스(web + db + redis 등)를 선언** → axhub 는 배포 가능한 web 서비스를 식별해 배포하고, 백킹 서비스(db/redis)의 axhub-managed 프로비저닝은 v1 범위 밖이라 외부로 두는 걸 명확히 안내해요. compose 의 미지원 구성(host network, privileged, 복잡한 volume 등)은 경고해요.
- 앱이 컨테이너 웹서비스 모델에 안 맞음(desktop/mobile/순수 batch/상태 보존 worker) → 범위 밖임을 명확히 안내해요.
- 기존 axhub saga 앱 → migrate 기능 도입 후에도 변경 없이 그대로 배포돼야 해요(무중단).
- **monorepo(여러 앱/패키지)** → 배포 가능한 앱 후보를 제시하고 사용자가 선택해요. 자동으로 전부 배포하지 않고, 선택한 앱마다 별도 axhub 앱으로 등록해요.
- 앱이 `localhost` 에만 바인딩 → 컨테이너에서 도달 불가하니 `0.0.0.0:$PORT` 바인딩 필요를 감지/안내해요.
- **`pinned` 모드에서 코드 구조 변경**(framework 교체·package manager 변경·start command 변경·새 서비스) → committed manifest 가 stale → 빌드가 깨질 수 있어요. staleness 를 감지해 re-migrate/manifest 갱신을 안내해요. `auto` 모드는 빌드마다 재감지라 자동 적응해요. (일반 feature 편집·dep 버전 bump 는 두 모드 다 무영향.)

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: 시스템은 axhub saga 로 scaffold 하지 않은 기존 앱을 배포 소스로 받아들여야 해요.
- **FR-002**: 시스템은 앱의 파일에서 (a) 프레임워크·설치·빌드·실행·포트, (b) repo 의 Dockerfile, (c) docker-compose 파일을 감지해야 해요. **v1 지원 언어 = Node, Python, Go, Ruby, Java, Kotlin** (그 외 스택은 Dockerfile/compose escape hatch 로 수용).
- **FR-003**: 시스템은 배포 전에 추정한 배포 계획을 (감지 신뢰도와 함께) 사용자에게 보여줘야 해요. 신뢰도가 high 면 확인 후 진행하고, low/모호하면 진행을 막고 명시 입력(`axhub.yaml`/Dockerfile/직접 입력)을 요청해야 해요.
- **FR-004**: 사용자는 앱 소스를 수정하지 않고 migrate + 배포할 수 있어야 해요.
- **FR-005**: 시스템은 앱이 필요로 하는 환경변수를 (값이 아니라 이름으로) repo 에 commit 가능한 `axhub.yaml` 에 선언할 수 있게 해야 해요. 각 항목은 `scope: build|runtime|both`(기본 runtime)를 가질 수 있어요 — build-scoped 는 빌드 단계 주입, runtime-scoped 는 컨테이너 시작 시 주입.
- **FR-006**: 시스템은 required 환경변수(build·runtime scope 모두)가 실제 설정됐는지 검증하고, 미설정 시 명확히 차단·안내해야 해요. build-scoped 는 **빌드 전**, runtime-scoped 는 **배포 전** 확인 — silent 실패(잘못된 빌드 산출 / runtime crash) 금지.
- **FR-007**: 시스템은 환경변수 값(secret)을 commit 되는 파일에 저장하지 않고 안전하게 보관해야 해요.
- **FR-008**: 시스템은 빌드/배포 방식을 다음 우선순위 ladder 로 결정해야 해요: ① 명시적 `axhub.yaml` → ② repo 의 Dockerfile → ③ docker-compose 파일 → ④ 프레임워크 자동 감지. 상위가 있으면 하위 추론을 건너뛰고, 무엇을 골랐는지 사용자에게 알려요.
- **FR-009**: 시스템은 docker-compose 앱을 감지해 compose 배포 방식(`axhub.yaml` 의 deploy_method=compose)으로 받아들여야 해요. 배포 대상 web 서비스는 **`build:` 가 로컬 컨텍스트 + 포트 노출인 서비스**로 식별하고(`image:` 전용 서비스는 외부 백킹으로 취급, web 후보 2개+ 면 사용자 확인), 백킹 서비스의 managed 프로비저닝은 범위 밖임을 알려요.
- **FR-010**: 시스템은 기존 axhub saga 앱의 배포를 변경 없이 계속 지원해야 해요(무중단 / backward compatible). 기존 `apphub.yaml` manifest 도 계속 읽혀야 해요.
- **FR-011**: 시스템은 자동 감지 실패 시 조용히 실패하지 않고, Dockerfile/compose 또는 수동 빌드 설정 경로를 안내해야 해요.
- **FR-012**: migrate 된 앱은 axhub governed catalog 데이터 사용을 요구받지 않아야 해요 — 자체 외부 데이터를 허용해요.
- **FR-013**: 모든 상태 변경(앱 등록 / git 연결 / 배포)은 기존 consent 메커니즘을 통과해야 해요 — migrate 가 동의 게이트를 우회하면 안 돼요.
- **FR-014**: migrate 는 기존 axhub 빌드·배포·환경변수 인프라를 재사용해야 하고, 별도 평행 배포 경로를 만들면 안 돼요.
- **FR-015**: `axhub.yaml` 정준화의 일부로 기존 axhub 템플릿·in-flight manifest 작업(w5-contracts)·docs 를 `axhub.yaml` 로 마이그레이션해야 해요. 전환 기간 `apphub.yaml` 은 dual-read 로 계속 수용돼야 해요(무중단).
- **FR-016**: `axhub.yaml` 은 `build.strategy: auto`(기본) | `pinned` 을 지원해야 해요. **auto** = 빌드마다 빌드/실행 명령을 재감지해 코드 변경에 자동 적응해요(`env`·`port` 선언은 override 로 유지). **pinned** = committed `install`/`build`/`start` 를 고정해요. 미지정 시 기본값은 `auto` 예요.
- **FR-017**: 시스템은 monorepo(여러 배포 가능 앱/패키지)를 감지해 후보 목록을 제시하고, 사용자가 선택한 앱(들)을 migrate 해야 해요. 선택한 각 앱은 별도 axhub 앱으로 등록돼요(단일 앱 = 단일 서비스 모델 유지). 자동 다중 배포는 하지 않아요.

### Key Entities *(include if feature involves data)*

- **기존 앱 (Existing App)**: migrate 대상. 소스 = 현재 디렉터리(local) 또는 (연결/연결할) GitHub repo (monorepo 면 배포 가능 앱 후보 다수 → 사용자 선택). 속성: 스택, 빌드/실행 설정, listen 포트, Dockerfile/compose 존재 여부.
- **배포 manifest (`axhub.yaml`)**: 앱의 빌드·런타임·환경변수·배포방식(deploy_method: docker|compose) 계약을 선언하는 파일. repo 에 commit 가능하고 사용자가 보고 편집할 수 있어요. axhub 의 표준(정준) manifest 이름이에요.
- **환경변수 계약 (Env Contract)**: required / optional 변수 이름 목록 + 각 항목의 `scope`(build|runtime|both, 기본 runtime). `axhub.yaml` 에 이름·scope 만 선언, 값은 밖에 안전하게 별도 보관해요.
- **컨테이너 계약 (Container Contract)**: 사용자 제공 Dockerfile 또는 docker-compose 파일. 감지 대상이며 추론보다 우선해요.
- **배포 계획 (Deploy Plan)**: 감지 결과 + 신뢰도 + 선택된 빌드/배포 방식. 배포 전에 사용자 확인을 받는 대상이에요.

## 구현 경계 (Implementation Boundaries) *(constraint)*

> migrate 은 단일 skill 이 아니라 axhub 의 기존 4-레이어(skill → helper → CLI → backend)에 걸쳐 전달돼요. axhub skill-authoring 규칙상 테스트되는 로직은 skill 에 둘 수 없어서, 아래 책임 분리가 강제예요. (이 표는 사용자 요청으로 명시한 제약이고, 상세 설계는 `/speckit-plan` 에서 확정해요.)

| 단계 | 담당 레이어 | 신규/재사용 |
|---|---|---|
| 로컬 스택 + Dockerfile + compose 감지 | axhub-helpers | 신규 |
| env 참조 스캔 | axhub-helpers | 신규 |
| `axhub.yaml` 생성 | axhub-helpers (snippet/sync 패턴) | 신규 |
| plan 미리보기 + 사용자 확인 | skill | 신규 (thin) |
| consent 발급 | axhub-helpers (consent-mint) | 재사용 |
| 앱 등록 / git 연결 / 배포 / env 값 설정 | axhub CLI | 재사용 |
| `axhub.yaml` 스키마 (version/env/compose) | backend | 신규 |
| 배포 전 env 검증 | backend | 신규 |

**경계 규칙**:
- skill = 얇은 오케스트레이션만. 감지·생성·검증 로직 임베드 금지(테스트 불가 → CI fail).
- 모든 mutation(등록/연결/배포)은 consent 게이트 통과.
- 기존 빌드·배포·env 인프라 재사용, 별도 평행 경로 금지.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 개발자가 기존 앱을 소스 수정 0줄로 axhub 라이브까지 올릴 수 있어요.
- **SC-002**: v1 지원 입력(Node·Python·Go·Ruby·Java·Kotlin / Dockerfile / docker-compose)의 자동 감지가 첫 시도에 ≥80% 맞아요.
- **SC-003**: required 환경변수 미설정으로 인한 silent 배포 실패가 0건 — 항상 배포 전에 표면화돼요.
- **SC-004**: 기존 axhub saga 앱의 배포 성공률이 migrate 기능 도입 후에도 100% 유지돼요(무중단).
- **SC-005**: 감지가 맞는 경우, 사용자가 migrate → 확인 → 배포를 5분 이내에 끝낼 수 있어요.
- **SC-006**: `auto` 모드에서 migrate 후 코드 변경(dep 추가·start 변경 등) → push 재배포가 manifest 수동 수정 없이 성공해요.

## Assumptions

- 대상 앱은 포트를 listen 하는 웹 서비스라 subdomain 으로 배포 가능해요. desktop/mobile/순수 batch 는 v1 범위 밖이에요.
- 대상 앱은 자체 외부 데이터(외부 DB/API)를 써요. axhub governed catalog 데이터 소비는 migrate 의 전제가 아니에요(별도 관심사).
- migrate 소스 = **현재 디렉터리(local) 또는 (연결/연결할) GitHub repo** — axhub 의 기존 git-connected 배포 모델과 일관돼요(Clarification B). 임의 원격 git URL 직접 clone·prebuilt 이미지 소스는 v1 범위 밖이에요.
- axhub 의 기존 컨테이너 빌드 + 배포 + 환경변수 secret 보관 인프라를 재사용해요.
- **표준 manifest 이름 = `axhub.yaml`** (정준). backend 는 이미 `apphub.yaml` 과 `axhub.yaml` 을 둘 다 읽어요. 정준화의 일부로 기존 템플릿·w5-contracts·docs 를 `axhub.yaml` 로 **마이그레이션**하고(Clarification C), 전환 기간 `apphub.yaml` 은 dual-read 로 계속 수용해 무중단을 유지해요. (manifest 예제 doc 의 "어느 이름이 정준인가" open 질문을 `axhub.yaml` 로 확정.)
- **docker-compose 는 v1 범위 안** — 감지 + compose 배포 방식 수용. 단 다중 서비스의 백킹(db/redis) axhub-managed 프로비저닝은 범위 밖이라, 그 서비스들은 외부로 둬요(자체 외부 데이터 가정과 일관). compose 의 web 서비스 → axhub 단일 배포 대상 매핑의 상세는 `/speckit-plan` 에서 확정.
- managed 데이터 리소스(DB/cache) 프로비저닝은 v1 범위 밖.
- 빌드 전략 + v1 감지 엔진: `build.strategy: auto`(기본)는 **빌드마다 재감지**해서 코드 변경에 자동 적응해요(Railpack zero-config 모델). **v1 감지 엔진 = axhubpack** — **Railpack `/core`**(MIT, Go — axhub backend 와 동일 언어; detection/plan 이 `/buildkit` 실행과 분리)를 채택해 build plan→Dockerfile 어댑터로 기존 Kaniko/Cloud Run 빌드에 연결해요. 이로써 v1 6개 언어(Node/Python/Go/Ruby/Java/Kotlin)를 Railpack provider 로 커버하고, JVM(Maven/Gradle) 같은 감지를 직접 재발명하지 않아요. `pinned` 은 committed 명령 고정. Railpack 의 `/buildkit` 실행 레이어는 안 가져와요(axhub = Kaniko/Cloud Run). hard-fork 보다 Go dependency 채택을 우선하고, Dockerfile/compose escape hatch 는 그대로 둬요.
- detect 로직의 위치는 확정됐어요: 권위 감지와 migrate-time 미리보기는 backend `POST /api/v1/apps/detect` 가 담당하고, 입력은 backend 가 읽을 수 있는 `github_repo` 또는 redacted `archive` payload 로 제한해요. `axhub-helpers migrate-plan` 은 monorepo 후보·Dockerfile/compose 존재·env 참조를 찾는 local light pre-scan 만 담당하고, 권위 감지를 미러링하지 않아요.
- 실제 "데이터 호출 막힘" 의 근본 원인(미설정 env / egress 제한 / `localhost` 바인딩 중 무엇인지)은 구현 전 막힌 앱 1개 재현으로 확정해야 해요. 본 spec 은 셋 다 사용자에게 표면화하는 것을 요구해요.
