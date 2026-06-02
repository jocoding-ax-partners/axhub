# Contract: `axhub-helpers migrate-plan` (Rust, light pre-scan)

> migrate-time 미리보기의 **로컬 light pre-scan**. 권위 감지(Railpack)는 backend 가 함. helper 는 monorepo 후보 열거 + container contract 존재 + env 참조 스캔만.

## 호출
```bash
axhub-helpers migrate-plan --dir <path> --json
axhub-helpers migrate-plan --dir <path> --app-path <candidate-path> --json
```

## 출력 (`schema_version: migrate-plan/v1`)
```json
{
  "schema_version": "migrate-plan/v1",
  "root": "/abs/path",
  "monorepo": true,
  "candidates": [
    {
      "path": ".",
      "stack_hint": "node",
      "has_dockerfile": false,
      "has_compose": false,
      "env_refs": ["DATABASE_URL", "NEXT_PUBLIC_API_URL"]
    },
    {
      "path": "services/api",
      "stack_hint": "go",
      "has_dockerfile": true,
      "has_compose": false,
      "env_refs": ["DB_DSN"]
    }
  ],
  "container_contracts": { "dockerfile": false, "compose": false }
}
```

## 계약
- `stack_hint` = 빠른 휴리스틱(`package.json`→`node`, `requirements.txt`/`pyproject.toml`→`python`, `go.mod`→`go`, `Cargo.toml`→`rust`, `Gemfile`→`ruby`, `pom.xml`/`build.gradle`→`java`, `build.gradle.kts`/`settings.gradle.kts`→`kotlin`, `Dockerfile`/`docker-compose.yml`/`docker-compose.yaml`/`compose.yml`/`compose.yaml`→`container`). **권위 아님** — backend Railpack 이 확정.
- `env_refs` = 소스 정적 스캔(`process.env.X` / `os.environ[...]` / `ENV["X"]` 등) → manifest `env.required` 후보. scope 추정(`NEXT_PUBLIC_`/`VITE_`→build).
- `monorepo`=true 면 `candidates[]` 다수 → skill 이 AskUserQuestion 으로 선택받음.
- `--app-path` 를 주면 `candidates[].path` 와 정확히 일치하는 후보 기준으로 `suggested_manifest` 와 후보별 `env_refs` 를 다시 렌더링함. 절대경로, `..`, Windows drive prefix 같은 root escape 값은 거부함.
- 값은 절대 출력 안 함(이름만). secret 누설 금지.

## skill 흐름에서의 위치
preflight → `migrate-plan`(이 helper) → (monorepo 면 후보 선택) → backend plan-preview(권위 감지) → 확인 카드 → manifest 생성 → `axhub env set` 안내 → `apps create --from-file` → git connect → deploy.
