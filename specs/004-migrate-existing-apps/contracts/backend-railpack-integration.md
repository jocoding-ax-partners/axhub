# Contract: backend Railpack 통합 + plan→Dockerfile + preflight env

> 권위 감지 엔진. `github.com/railwayapp/railpack/core` (MIT) in-process.

## 1. 감지 (Railpack core)
```go
res := core.GenerateBuildPlan(app, env, &core.GenerateBuildPlanOptions{
    BuildCommand: "", StartCommand: "", ConfigFilePath: "", // pinned override 시 채움
})
// res.Plan *plan.BuildPlan (추상 step/layer)
// res.DetectedProviders []string  (node/python/golang/ruby/java/...)
// res.Success, res.Logs
```

## 2. plan-preview 엔드포인트 (migrate-time 미리보기)
`POST /api/v1/apps/detect` — 인증된 사용자 전용 read-only endpoint. 앱 등록 전에도 호출하므로 app-admin 권한이 아니라 현재 org/workspace member 권한 + repo 접근 권한으로 검증해요.

입력은 backend 가 실제로 읽을 수 있는 두 형태만 허용해요. local dir path 문자열은 받지 않아요.

```json
{
  "source": {
    "type": "github_repo",
    "owner": "jocoding",
    "repo": "paydrop",
    "ref": "main",
    "path": "."
  }
}
```

또는 helper/skill 이 local dir 을 redacted archive 로 업로드한 뒤:

```json
{
  "source": { "type": "archive", "upload_id": "up_...", "path": "." }
}
```

보안/운영 계약:
- archive size/path/depth 제한, symlink escape 금지, `.git`/secret-like 파일 redaction 기본값.
- rate limit + audit log + request timeout 필수.
- 응답 logs 는 secret redaction 후 반환.
- confidence threshold: `high >= 0.80`, `medium 0.60..0.79`, `low < 0.60`. high 는 확인 후 진행, medium 은 editable 확인 카드, low/모호/필수 command 누락은 차단.

응답:
```json
{
  "detected_providers": ["node"],
  "framework": "nextjs",
  "install": "npm ci", "build": "npm run build", "start": "npm run start",
  "port": 3000, "health_path": null,
  "confidence": 0.92,
  "env_refs": [{"name":"DATABASE_URL","scope":"runtime"},{"name":"NEXT_PUBLIC_API_URL","scope":"build"}]
}
```
- `confidence` high → skill 확인 후 진행 / medium → editable 확인 카드 / low|모호 → 차단 + 명시 입력 요청(FR-003).

## 3. plan → Dockerfile 어댑터 (`railpack_adapter.go`)
- 입력 `plan.BuildPlan` → 출력 Dockerfile 텍스트 → 기존 Kaniko/Cloud Run.
- ladder: `dockerfile` 명시 > Railpack plan 변환 > 기존 `generateDockerfile`(fallback).
- **golden test**: 6언어 fixture → GenerateBuildPlan → adapter → Dockerfile 스냅샷.

## 4. build vs runtime env 주입
- `scope=build`/`both` → Dockerfile `ARG`/`ENV`(빌드 단계 baked).
- `scope=runtime`/`both` → Cloud Run 서비스 env(`injectAppHubEnvVars` 경로).
- 값은 encrypted env store 에서 읽되, build/runtime scope 필터를 통과한 키만 해당 단계로 전달해요.
- 회귀 invariant: runtime-only secret 은 Kaniko/Cloud Build `--build-arg` 에 절대 전달되지 않아요. build-only secret 은 Cloud Run runtime env 에 주입되지 않아요.

## 5. preflight env 교차검증 (`deploy_preflight.go`)
- `RunPreflightChecks` 에 appID + `ListEnvVars(appID)` + env scope 필터 배선(신규).
- `env.required` 중 `scope∈{build,both}` → **빌드 전** 교차검증 / `scope∈{runtime,both}` → **배포 전**.
- 미설정 → 한국어 fail loud("`DATABASE_URL` 필요 — `axhub env set`"). silent 금지(FR-006, SC-003).

## 6. strategy 게이트 (`auto` 빌드시점 재감지)
- `strategy=auto`(기본): manifest 가 있어도 빌드 파이프라인이 Railpack 재감지로 install/build/start 산출(env/port/dockerfile override 는 manifest 값).
- `strategy=pinned`: manifest 의 install/build/start 고정, 재감지 안 함.
- **신규 분기**: 현재 "manifest 있으면 그대로" 로직에 strategy 분기 추가.

## 7. compose
- `deploy_method=compose` → `build:` 로컬+포트노출 서비스 = web 배포 대상. `image:` 전용 = 외부 백킹(provisioning 범위 밖).

## 8. backend storage/API migration
- 기존 `DeploymentEnvVar.Stage` 는 배포 stage(`production`, `system`) 용도라 build/runtime scope 와 혼동하지 않아요.
- 신규 필드 또는 metadata 로 `scope: build|runtime|both` 를 저장하고 API create/update/list 에 노출해요(기본 runtime).
- 기존 env rows 는 migration 에서 `scope=runtime` 으로 backfill 해 backward-compatible 하게 둬요.
- 테스트: create/update/list roundtrip, runtime-only not build-arg, build-only not runtime-env, both 는 양쪽 주입.
