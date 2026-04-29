# 19. Quality / CI / Release

## Lint — Ruff

`pyproject.toml [tool.ruff]`:
```toml
line-length = 100
target-version = "py312"
exclude = ["src/ouroboros/_version.py"]
```

`[tool.ruff.lint]`:
```toml
select = [
    "E",    # pycodestyle errors
    "W",    # pycodestyle warnings
    "F",    # Pyflakes
    "I",    # isort
    "B",    # flake8-bugbear
    "C4",   # flake8-comprehensions
    "UP",   # pyupgrade
    "ARG",  # flake8-unused-arguments
    "SIM",  # flake8-simplify
]

ignore = [
    "E501",   # Line too long (formatter 처리)
    "ARG002", # Unused method argument (interface/override 흔함)
    "B017",   # assert-raises-exception
    "B023",   # function-uses-loop-variable
    "B904",   # raise-without-from-inside-except
    "SIM102", # collapsible-if
    "SIM105", # suppressible-exception
    "SIM108", # if-else-block-instead-of-if-exp
    "SIM117", # multiple-with-statements
]
```

`[tool.ruff.lint.isort]`:
```toml
force-single-line = false
force-sort-within-sections = true
known-first-party = ["ouroboros"]
```

`[tool.ruff.lint.per-file-ignores]`:
```toml
"tests/**" = ["ARG001", "ARG002", "E402"]
```

`[tool.ruff.format]`:
```toml
quote-style = "double"
indent-style = "space"
```

## Type Check — mypy

`[tool.mypy]`:
```toml
python_version = "3.12"
ignore_missing_imports = true
disable_error_code = [
    "union-attr",       # x.attr where x is Union
    "arg-type",         # 호출 시 타입 mismatch
    "return-value",     # return 타입 mismatch
    "assignment",       # 변수 할당 타입 mismatch
    "attr-defined",     # 정의 안 된 속성 접근
    "misc",             # 잡다
    "call-arg",         # 함수 호출 인자
    "override",         # method override 시그니처 불일치
    "list-item",        # 리스트 항목 타입
    "dict-item",        # 딕셔너리 항목 타입
    "operator",         # 연산자 적용 타입
    "str-bytes-safe",   # str/bytes 안전성
    "no-any-return",    # Any 반환
    "import-untyped",   # 타입 stub 없는 import
]
```

→ **14 개 disabled** = 사실상 mypy 가 매우 관대. 실 type safety 는 Pydantic frozen + ruff B + 270+ 단위 테스트 로 보강.

→ **개선 권고**: `arg-type`, `return-value`, `assignment` 점진 활성화 가치.

## Test — pytest

`[tool.pytest.ini_options]`:
```toml
asyncio_mode = "auto"                                    # async fixture 자동 감지
asyncio_default_fixture_loop_scope = "function"
testpaths = ["tests"]
python_files = ["test_*.py"]
python_classes = ["Test*"]
python_functions = ["test_*"]
```

`project-context.md` enforce: "asyncio_mode = auto per project-context.md".

## Pre-commit (`.pre-commit-config.yaml`)

```yaml
repos:
  - hooks:
      - id: ruff-format
      - id: ruff-check
      - id: mypy
```

## GitHub Actions

### `.github/workflows/test.yml`

```yaml
name: Tests
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

jobs:
  test:
    runs-on: ubuntu-latest
    timeout-minutes: 15
    strategy:
      fail-fast: false
      matrix:
        python-version: ['3.12', '3.13', '3.14']

    steps:
      - uses: actions/checkout@v4
        with: {fetch-depth: 0}
      - uses: astral-sh/setup-uv@v4
      - uses: actions/setup-python@v5
        with: {python-version: ${{ matrix.python-version }}}
      - run: uv sync --dev
      - run: uv run pytest tests/ --cov=src/ouroboros --cov-report=xml --cov-report=term-missing -v
      - uses: codecov/codecov-action@v3
        if: always()
        with:
          file: ./coverage.xml
          fail_ci_if_error: false
          flags: unittests
          name: codecov-${{ matrix.python-version }}
```

3 Python 매트릭스 + Codecov 업로드. 15분 타임아웃.

### `.github/workflows/lint.yml`

```yaml
jobs:
  ruff:
    steps:
      - uv sync --dev
      - uv run ruff check src/ tests/
      - uv run ruff format --check src/ tests/

  mypy:
    steps:
      - uv sync --dev
      - uv run mypy src/
```

별도 잡 — 병렬 실행.

### `.github/workflows/release.yml`

`v*.*.*` 태그 트리거. Python 3.14 build.

```yaml
on:
  push:
    tags:
      - 'v*.*.*'

permissions:
  contents: write

jobs:
  release:
    steps:
      - uv build                             # wheel + sdist
      - uv publish --dry-run dist/*          # 사전 검증
      - id: prerelease
        run: |
          if [[ "${{ github.ref_name }}" =~ (a|alpha|b|beta|rc|dev) ]]; then
            echo "is_prerelease=true" >> "$GITHUB_OUTPUT"
          else
            echo "is_prerelease=false" >> "$GITHUB_OUTPUT"
          fi
      # ... GitHub release 생성

  build-tui:
    strategy:
      matrix:
        target: [linux-x64, linux-arm64, macos-x64, macos-arm64, windows-x64]
    steps:
      - cargo build --release
      - upload-artifact

  attach-tui-binaries:
    needs: [release, build-tui]
    steps:
      - download-artifact (pattern: ouroboros-tui-*)
      - softprops/action-gh-release@v2 (files: ouroboros-tui-*)

  publish:
    needs: release
    if: success()
    steps:
      - uv build
      - uv publish --check-url https://pypi.org/simple/ dist/*
```

5 cross-arch Rust TUI 바이너리 + PyPI publish.

#### release.yml job DAG

```
release (uv build + dry-run + GH release 생성)
  ├── build-tui (4 matrix runner — 실제 4개 OS, 5번째 매트릭스는 미정의)
  │   ├── macos-latest    → aarch64-apple-darwin
  │   ├── macos-14        → x86_64-apple-darwin
  │   ├── ubuntu-latest   → x86_64-unknown-linux-gnu
  │   └── windows-latest  → x86_64-pc-windows-msvc
  └── attach-tui-binaries (needs: [release, build-tui])
       └── softprops/action-gh-release@v2 — pattern ouroboros-tui-* attach
publish (needs: release, if: success())
  └── uv publish --check-url
```

> **noted**: Section 19 본문은 "5 cross-arch" 라 명시했으나 release.yml matrix 는 4 entries. 5번째 (`linux-arm64`?) 는 axhub 의 자체 release 흐름과 혼동 가능 — Ouroboros 는 4 binary.

#### macOS code signing

```yaml
- name: Re-sign binary (macOS)
  if: runner.os == 'macOS'
  run: codesign --force --sign - ouroboros-tui-${{ matrix.target }}
```

→ ad-hoc signing (`-`) — Apple Developer ID 없이 Gatekeeper 차단 우회 위한 self-sign. 사용자가 `xattr -d com.apple.quarantine` 으로 추가 신뢰 필요할 수 있음.

#### Pre-release 자동 감지

```bash
if [[ "${{ github.ref_name }}" =~ (a|alpha|b|beta|rc|dev) ]]; then
  is_prerelease=true
fi
```

→ tag 명에 `a`/`alpha`/`b`/`beta`/`rc`/`dev` 포함되면 GitHub Release 의 `prerelease: true` flag set. (e.g. `v0.31.0-rc1`)

> **위험**: `(a|...|dev)` regex 가 단순 `a` 한 글자도 match — `v1.0.0-alphabet` 같은 ASCII 가 우연히 prerelease flag 받음. 안전한 안 — `^(.*-(alpha|beta|rc|dev).*)?$` 같은 anchored regex 사용.

### `.github/workflows/dev-publish.yml`

개발 채널 (alpha/beta/rc) — main 브랜치 commit + `release/*-beta` 브랜치 trigger. tag-pushed commit 은 skip (release.yml 가 처리).

```yaml
on:
  push:
    branches: [main, 'release/*-beta']
    tags-ignore: ['v*']

jobs:
  dev-publish:
    steps:
      - Check if dev version (skip if HEAD === tag)
        run: git describe --exact-match HEAD 2>/dev/null
        # 0 exit → tag commit → skip
      - Sync plugin version: python scripts/sync-plugin-version.py --write
      - uv build  # wheel 빌드
      - id: build  # version 추출 (regex `[^-]+-([^-]+)-` 으로 wheel 파일명 parse)
      - Smoke test:
          uv pip install dist/*.whl --system
          # 1) ouroboros.__version__ assert 'dev' in v
          # 2) opencode plugin assets check (PR #462 regression guard):
          #    files('ouroboros.opencode.plugin') 으로 importlib.resources 접근 →
          #    ouroboros-bridge.ts / package.json / tsconfig.json 모두 존재 확인
      - uv publish dist/* (PyPI dev channel)
```

→ **Wheel asset packaging contract**: opencode bridge 의 3 asset (TypeScript 소스 + manifest + tsconfig) 가 importlib.resources 으로 접근 가능해야 함. PR #462 에서 hatch include 규칙이 non-Python 자산을 누락한 회귀 발견 후 추가된 가드.

## Issue Templates (`.github/ISSUE_TEMPLATE/`)

- `bug_report.yml`
- `feature_request.yml`
- `question.yml`
- `config.yml`

## Commit / PR 정책

`project-context.md` Anti-Patterns:
1. Zombie objects (ORM 외부 누출 금지)
2. God-Contexts (`GodContext` 객체 금지)
3. Ambiguous event verbs (`updated`/`processed` 금지)
4. Async wrapper lie (`async def` 안 CPU-bound 금지 → `asyncio.to_thread`)
5. Silent failures (`except: pass` 금지)
6. God objects (`utils.py`, `manager.py`, `helper.py` 금지 — 구체 이름 사용)

## Async / I/O 규칙

```python
# DO: 비동기 I/O
async def fetch_completion(messages) -> Result[Response, Error]:
    return await llm_adapter.complete(messages)

# DO: CPU-bound 동기
def parse_seed(yaml_content) -> Seed:
    return Seed.model_validate(yaml.safe_load(yaml_content))

# DO: 비동기 컨텍스트 + thread pool
async def process_heavy():
    return await asyncio.to_thread(heavy_cpu_computation)

# DON'T: 이벤트 루프 블록
async def bad():
    result = heavy_computation()    # BLOCKS!
```

## Naming Conventions

| 컴포넌트 | 포맷 | 예시 |
|---|---|---|
| Files | `snake_case.py` | `pal_router.py` |
| Classes | `PascalCase` | `EffectiveOntology` |
| Functions | `snake_case` | `calculate_drift` |
| Variables | `snake_case` | `current_context` |
| Constants | `UPPER_CASE` | `MAX_AC_DEPTH` |
| Events | `dot.notation.past_tense` | `ontology.concept.added` |
| DB Tables | `snake_case`, plural | `events`, `checkpoints` |
| JSON Fields | `snake_case` | `seed_id`, `created_at` |

## Import 규칙

```python
# DO: 절대 import 만
from ouroboros.core.seed import Seed
from ouroboros.core.types import Result

# DON'T: 패키지 간 상대 import
from ..core.seed import Seed             # 금지
from .router import PALRouter            # 같은 패키지 내만 OK
```

## Layered Dependencies

```
CLI Layer (cli/)
    ↓ import
Application Layer (execution/, bigbang/, secondary/)
    ↓
Domain Layer (core/, routing/, evaluation/, resilience/, consensus/)
    ↓
Infrastructure Layer (providers/, persistence/, observability/, config/)
```

규칙:
- Lower 레이어 NEVER import upper
- Domain phases NEVER 서로 직접 import
- Phase 간 통신은 ExecutionEngine + 이벤트만

## Phase Protocol

```python
class IPhase(ABC):
    @abstractmethod
    async def execute(self, ctx: PhaseContext) -> PhaseResult:
        """첫 줄에서 ctx.payload 검증 필수:
        
            input_data = RoutingInput.model_validate(ctx.payload)
        """
```

## Release 흐름 (Manual)

1. PR merged to main
2. Tag push: `git tag v0.31.0 && git push --tags`
3. `release.yml` 자동 fire:
   - PyPI publish
   - 5 cross-arch Rust TUI 바이너리
   - GitHub release
4. `session-start.py` 가 다음 세션부터 24h 후 사용자에게 알림

## Test Coverage 정책

`common/testing.md` (위치 확인 못 함, 글로벌 CLAUDE.md 에 언급):
- 최소 80%
- Unit + Integration + E2E 모두 필수
- TDD: RED → GREEN → REFACTOR → 80%+ 검증

`pyproject.toml` 에 명시 임계는 없음 — Codecov 가 추적만.

## CI 시간

- Test (3 매트릭스): 15분 타임아웃 (실 5–10분 추정)
- Lint: 1–2분
- Release: 10–20분 (Rust 5 빌드 포함)
