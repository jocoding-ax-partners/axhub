# Examples Repo — `templates.json` Manifest (Sibling PR)

> `github.com/jocoding-ax-partners/examples` 에 `templates.json` 신규. helper `list-templates` subcommand 가 이 파일 fetch.

---

## 배경

- ax-hub-cli `init --from-template` = builtin 5 (react-vite / nextjs-minimal / fastapi-uvicorn / go-net-http / docker-stub)
- examples repo = 별도. README "조코딩 AX 파트너스 axhub 위에서 바로 굴러가는 바이브코딩 템플릿 모음"
- 두 source = 동기화 안 됨. 사용자 결정: examples repo = single source of truth (codex CEO F7 응답)

## 작업 (sibling repo PR, ~30분)

### M-1. `templates.json` 신규 파일

**파일**: `github.com/jocoding-ax-partners/examples/templates.json`

**스키마**:
```json
{
  "schema_version": "templates/v1",
  "updated_at": "2026-05-03T00:00:00Z",
  "templates": [
    {
      "slug": "nextjs-axhub",
      "framework": "nextjs",
      "stack": ["node", "react", "tailwind"],
      "description": "Next.js + axhub deploy ready",
      "min_node": "18.0.0",
      "tarball_path": "templates/nextjs-axhub/",
      "manifest": {
        "package_json": true,
        "axhub_yaml": true
      }
    },
    {
      "slug": "fastapi-axhub",
      "framework": "fastapi",
      "stack": ["python", "uvicorn"],
      "description": "FastAPI + uvicorn + axhub",
      "min_python": "3.11.0",
      "tarball_path": "templates/fastapi-axhub/",
      "manifest": {
        "requirements_txt": true,
        "axhub_yaml": true
      }
    },
    {
      "slug": "django-axhub",
      "framework": "django",
      "stack": ["python", "django"],
      "description": "Django 5 + axhub",
      "min_python": "3.11.0",
      "tarball_path": "templates/django-axhub/",
      "manifest": {
        "requirements_txt": true,
        "axhub_yaml": true,
        "manage_py": true
      }
    },
    {
      "slug": "go-axhub",
      "framework": "go-net-http",
      "stack": ["go"],
      "description": "Go net/http REST + axhub",
      "min_go": "1.23.0",
      "tarball_path": "templates/go-axhub/",
      "manifest": {
        "go_mod": true,
        "axhub_yaml": true
      }
    },
    {
      "slug": "react-vite-axhub",
      "framework": "react-vite",
      "stack": ["node", "react", "vite"],
      "description": "React + Vite SPA + axhub",
      "min_node": "18.0.0",
      "tarball_path": "templates/react-vite-axhub/",
      "manifest": {
        "package_json": true,
        "axhub_yaml": true
      }
    }
  ]
}
```

### M-2. 기존 examples repo 디렉토리 정리

현재 examples repo top-level = `.gitignore` 만. 디렉토리 신규 생성:
- `templates/nextjs-axhub/` — Next.js 14 app router + axhub.yaml + package.json
- `templates/fastapi-axhub/` — FastAPI + uvicorn + axhub.yaml + requirements.txt
- `templates/django-axhub/` — Django 5 + axhub.yaml + requirements.txt + manage.py
- `templates/go-axhub/` — Go net/http + axhub.yaml + go.mod
- `templates/react-vite-axhub/` — React + Vite + axhub.yaml + package.json

각 template = 작동하는 minimal hello world + axhub deploy 즉시 가능.

### M-3. README 업데이트

기존 README (8KB):
- 표 의 "어떤 걸 골라야 하나" 섹션 = 5 template 매트릭스 명시 (helper list-templates 출력과 일관)

### M-4. CI 추가 (sibling repo)

- `.github/workflows/templates-validate.yml` — PR 시 `templates.json` schema validation + 각 tarball_path 디렉토리 존재 확인
- 향후 helper smoke 가 examples repo 에 의존 → version pin 검토

## helper 의존 contract

helper `list-templates` subcommand 가 다음 contract 가정:

```
GET https://raw.githubusercontent.com/jocoding-ax-partners/examples/main/templates.json
```

- HTTP 200 + JSON body = success
- HTTP 404/5xx = fallback to ax-hub-cli builtin 5
- JSON parse fail = fallback
- cache TTL 1시간 (~/.cache/axhub-plugin/templates.json)

helper `fetch-template <slug>` subcommand 가 다음 contract 가정:

```
GET https://codeload.github.com/jocoding-ax-partners/examples/tar.gz/main
```

- 100 MB cap
- tar 안의 `tarball_path` (예: `templates/nextjs-axhub/`) 만 cwd 에 extract
- root 의 LICENSE / README.md / .gitignore = skip
- ax-hub-cli `MaxArchiveSize` + path traversal guard 동일 패턴

## Effort

- M-1 (templates.json schema): ~5분
- M-2 (5 template 디렉토리 작성): ~3시간 (각 ~30분, hello world + axhub.yaml + dep manifest)
- M-3 (README 업데이트): ~10분
- M-4 (CI workflow): ~10분
- **Total: ~3시간 30분 (sibling repo)**

## Sibling PR 일정

Phase A0 진입 전 PR 머지 권장 — Phase A0 #5 의 helper list-templates 가 templates.json 의존.

PR 분할:
- PR-1 (templates.json + 1 template 만): helper smoke 가능, ~30분
- PR-2 (4 추가 template): ~2시간 30분
- PR-3 (CI workflow): ~10분

## Validation

- [ ] templates.json schema 검증 PASS
- [ ] 5 template 디렉토리 모두 axhub.yaml + framework dep manifest 포함
- [ ] helper list-templates 가 fetch + parse 가능
- [ ] helper fetch-template <slug> 가 cwd 에 정상 extract
- [ ] 각 template 가 `axhub deploy` 즉시 가능 (hello world + valid axhub.yaml)
