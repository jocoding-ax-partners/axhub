# Phase 1 Data Model: 기존 앱 migrate

> spec.md Key Entities + Clarifications 를 구체 스키마로. 권위 표현 = `axhub.yaml`(파일) ↔ backend `domain.AppManifest`(Go).

## 1. `axhub.yaml` (정준 manifest)

```yaml
version: axhub/v1            # 스키마 게이트. 없으면 legacy 로 dual-read
name: my-app

runtime:
  port: 3000
  health_path: /health

build:
  strategy: auto            # auto(기본) | pinned
  framework: ""             # 선택 override/hint (Railpack 자동감지가 기본)
  install: ""               # pinned 일 때만 의미
  build: ""
  start: ""
  dockerfile: ""            # 있으면 우선 (ladder ①axhub.yaml ②Dockerfile ③compose ④자동감지)
  deploy_method: docker     # docker | compose
  compose_file: ""          # deploy_method=compose 일 때

env:                        # 이름 계약만 — 값은 axhub env(암호화)
  required:
    - name: DATABASE_URL
      scope: runtime        # build | runtime | both (기본 runtime)
    - name: NEXT_PUBLIC_API_URL
      scope: build
  optional:
    - name: LOG_LEVEL
      scope: runtime

ci:                         # w5-contracts 기존
  commands: []
  timeout: 120              # 1-600s
```

전 필드 optional → 기존(version/env 없는) manifest 무중단.

## 2. Go struct 확장 (`internal/domain/app_spec.go`)

w5-contracts 의 `AppManifest`(name/runtime/build{...,deploy_method,compose_file}/ci) 에 **추가**:

```go
type AppManifest struct {
    Version string             `yaml:"version,omitempty"`   // 신규
    Name    string             `yaml:"name,omitempty"`
    Runtime AppManifestRuntime `yaml:"runtime,omitempty"`
    Build   AppManifestBuild   `yaml:"build,omitempty"`
    CI      AppManifestCI      `yaml:"ci,omitempty"`
    Env     AppManifestEnv     `yaml:"env,omitempty"`       // 신규
}

type AppManifestBuild struct {
    Strategy     string `yaml:"strategy,omitempty"`         // 신규: auto(기본)|pinned
    Framework    string `yaml:"framework,omitempty"`
    Install      string `yaml:"install,omitempty"`
    Build        string `yaml:"build,omitempty"`
    Start        string `yaml:"start,omitempty"`
    Dockerfile   string `yaml:"dockerfile,omitempty"`
    DeployMethod string `yaml:"deploy_method,omitempty"`    // 기존 w5
    ComposeFile  string `yaml:"compose_file,omitempty"`     // 기존 w5
}

type AppManifestEnv struct {                                // 신규
    Required []AppManifestEnvVar `yaml:"required,omitempty"`
    Optional []AppManifestEnvVar `yaml:"optional,omitempty"`
}

type AppManifestEnvVar struct {                             // 신규
    Name  string `yaml:"name"`
    Scope string `yaml:"scope,omitempty"`                   // build|runtime|both (기본 runtime)
}
```

`ToSpecData()` 가 `Version`/`Strategy`/`Env`(scope 별) 를 `AppSpecData` 로 매핑.

## 3. 엔티티

### Existing App (migrate 대상)
- 소스: 현재 디렉터리(local) | (연결/연결할) GitHub repo. monorepo → 배포 가능 앱 후보 다수.
- 속성: 감지 스택, 빌드/실행 설정, listen 포트, Dockerfile/compose 존재 여부.
- 관계: 1 Existing App → 1 axhub 앱(monorepo 면 선택한 후보마다 1 앱).

### Container Contract
- Dockerfile | docker-compose 파일. 감지 대상 + 추론보다 우선(ladder).
- compose: `build:` 로컬+포트노출 서비스 = web(배포 대상), `image:` 전용 = 외부 백킹.

### Env Contract
- required/optional 변수 **이름 + scope**(build|runtime|both, 기본 runtime).
- 값은 manifest 밖(axhub env, 암호화). build-scoped → 빌드 ARG/ENV, runtime-scoped → Cloud Run env.

### Deploy Plan (감지 산출 + 확인 대상)
- 필드: `detected_providers[]`, install/build/start, port, health_path, deploy_method, **confidence**, 감지된 env 참조.
- high-confidence → 확인 후 진행 / low|모호 → 차단 + 명시 입력 요청.

### AppManifest (`axhub.yaml`)
- 위 §1/§2. version/name/runtime/build(strategy)/env(scope)/ci/deploy_method.

## 4. Validation (manifest_parser.go)

| 규칙 | 출처 |
|---|---|
| `build.strategy ∈ {auto, pinned}` (빈값=auto) | FR-016 |
| `env[].scope ∈ {build, runtime, both}` (빈값=runtime) | FR-005 |
| `env[].name` non-empty + 개행 금지(인젝션) | FR-005, 기존 보안 패턴 |
| `deploy_method ∈ {docker, compose}`, `compose_file` 는 compose 일 때만 | 기존 w5 ValidateSpecData |
| env scope 는 env 저장소/API 에도 roundtrip 되어 build/runtime 필터링에 사용 | FR-005~007 |
| runtime-only secret 은 build arg 로 전달 금지, build-only secret 은 runtime env 주입 금지 | FR-006~007 |
| `port 1-65535`, `health_path` 는 `/` 시작 | 기존 |
| unknown 필드 거부(`KnownFields(true)`) | 기존 parser |
| `version` 미인식 값 → 경고(거부 아님, forward-compat) | FR-010 |

## 5. Storage/API migration

- `DeploymentEnvVar.Stage` 같은 기존 배포 stage 필드는 build/runtime scope 와 분리해요.
- env 값 저장소에는 `scope: build|runtime|both` 를 별도 저장(또는 metadata)하고 API create/update/list 에 노출해요.
- 기존 env rows 는 `scope=runtime` 으로 backfill 해서 현재 배포 동작을 보존해요.
- build pipeline 은 `scope∈{build,both}` 만 build args 로 받고, runtime deploy 는 `scope∈{runtime,both}` 만 컨테이너 env 로 받아요.

## 6. State / lifecycle

```
[Existing App] --migrate--> [Deploy Plan(confidence)] --확인--> [axhub.yaml 생성]
   --env 감지--> [axhub env set 안내] --apps create --from-file--> [등록]
   --git connect--> [배포] --(auto strategy)--> push 마다 재감지 재배포
```
