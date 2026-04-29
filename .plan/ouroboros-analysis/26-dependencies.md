# 26. Dependencies Inventory

## Python — `pyproject.toml`

### 필수 (`[project.dependencies]`)

| 패키지 | 제약 | 역할 |
|---|---|---|
| `aiosqlite` | `>=0.20.0,<1.0.0` | async SQLite driver (EventStore) |
| `anyio` | `>=4.0.0,<5.0.0` | async compat layer |
| `pydantic` | `>=2.0.0,<3.0.0` | frozen 모델, 검증 (Seed, BaseEvent, RuntimeHandle 일부) |
| `prompt-toolkit` | `>=3.0.0,<4.0.0` | interactive 입력 |
| `pyyaml` | `>=6.0.0,<7.0.0` | seed YAML, mechanical.toml 일부 |
| `rich` | `>=13.0.0,<15.0.0` | Typer 출력 |
| `sqlalchemy[asyncio]` | `>=2.0.0,<3.0.0` | EventStore + UoW |
| `structlog` | `>=24.0.0,<26.0.0` | 구조화 로깅 + 마스킹 |
| `typer` | `>=0.12.0,<1.0.0` | CLI 프레임워크 |

### 선택 (`[project.optional-dependencies]`)

#### `[claude]`
- `claude-agent-sdk>=0.1.0,<1.0.0`
- `anthropic>=0.52.0,<1.0.0`

#### `[litellm]`
- `litellm>=1.80.0,<=1.82.6` (100+ provider)

#### `[mcp]`
- `mcp>=1.26.0,<2.0.0` (Anthropic MCP SDK)

#### `[tui]`
- `textual>=1.0.0,<9.0.0`

#### `[dashboard]` (legacy alias)
- `streamlit>=1.40.0,<2.0.0`
- `plotly>=5.24.0,<7.0.0`
- `pandas>=2.2.0,<3.0.0`

#### `[all]`
```python
ouroboros-ai[claude,litellm,mcp,tui,dashboard]
```

## Dev (`[dependency-groups]`)

| 패키지 | 제약 | 역할 |
|---|---|---|
| `mypy` | `>=1.19.1` | 타입 검사 (14 disable_error_code) |
| `pre-commit` | `>=4.5.1` | git hook |
| `pytest` | `>=9.0.2` | 테스트 |
| `pytest-asyncio` | `>=1.3.0` | async 테스트 |
| `pytest-cov` | `>=7.0.0` | 커버리지 |
| `ruff` | `>=0.14.11` | lint + format |
| `types-pyyaml` | `>=6.0.12.20250915` | mypy stub |

## Build

| 패키지 | 역할 |
|---|---|
| `hatchling` | build backend |
| `hatch-vcs` | git tag → version (`src/ouroboros/_version.py`) |

## 빌드 설정 트릭

```toml
[tool.hatch.build.targets.wheel]
exclude = ["src/ouroboros/opencode/plugin/**"]

[tool.hatch.build.targets.wheel.force-include]
"skills" = "ouroboros/skills"
"src/ouroboros/opencode/plugin" = "ouroboros/opencode/plugin"
```

이유: 휠에서 중복 ZIP 헤더 (PyPI 거부) 방지 + `importlib.resources.files("ouroboros.opencode.plugin")` 가 ts 파일 찾을 수 있게.

## Rust — `crates/ouroboros-tui/Cargo.toml`

| 크레이트 | 버전 | 역할 |
|---|---|---|
| `superlighttui` | `0.7.1` | TUI 프레임워크 |
| `rusqlite` | `0.33` (features `bundled`) | 동기 SQLite (bundled C 라이브러리) |
| `serde_json` | `1` | 이벤트 payload JSON 파싱 |

Rust edition: `2021`. MSRV: `1.74`.

## TS — `src/ouroboros/opencode/plugin/package.json`

```json
{
  "name": "ouroboros-bridge",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "devDependencies": {
    "@types/bun": "latest",
    "@types/node": "latest"
  }
}
```

런타임 의존: `@opencode-ai/plugin` (peer, OpenCode 자체 제공). 빌드 안 함 — Bun/OpenCode 가 ts 직접 실행.

## Test runner (TS)

`scripts.test = "bun test"` — Bun 1.x.

## Python 3.12+ 신문법 사용

- `Result[T, E]` PEP 695 generic dataclass (`core/types.py:15`)
- `match`/`case` 문
- `tuple[...]` lowercase generic (PEP 585)
- `X | None` 대신 `Optional[X]` 안 씀
- `from __future__ import annotations` 일부 모듈

## CI Action 의존

| Action | 버전 |
|---|---|
| `actions/checkout` | v4 |
| `astral-sh/setup-uv` | v4 |
| `actions/setup-python` | v5 |
| `codecov/codecov-action` | v3 |
| `actions/upload-artifact` | v4 |
| `actions/download-artifact` | v4 |
| `softprops/action-gh-release` | v2 |

## OS / Runtime 요구

### Python 코어
- macOS (ARM/Intel) — primary
- Linux (x86_64, ARM64) — Ubuntu 22.04+, Debian 12+, Fedora 38+
- Windows (WSL 2) — Linux 빌드 사용 (recommended)
- Windows (native) — experimental

### Rust TUI
- 같은 5 아키텍처 cross-arch 빌드 (release.yml)

### Claude Agent SDK
- Claude Code CLI 설치 + 인증 (Pro/Max Plan 또는 ANTHROPIC_API_KEY)

### Codex CLI
- `codex` 바이너리 설치 + `OPENAI_API_KEY`
- 추천 모델: GPT-5.4 + medium reasoning effort

### OpenCode
- OpenCode CLI 설치
- `ouroboros setup --runtime opencode` 가 bridge plugin 자동 설치

### Hermes
- Hermes CLI 설치

## 외부 서비스

| 서비스 | 사용처 |
|---|---|
| PyPI | `version-check.py`, `update` skill |
| GitHub API | `interview/SKILL.md:33` (releases/latest) |
| GitHub | `setup/SKILL.md:80` (`gh api -X PUT /user/starred/Q00/ouroboros`) |
| Anthropic API | Claude 백엔드 |
| OpenAI API | Codex / GPT-4o / Gemini 일부 |
| Google AI | Gemini 2.5 Pro (consensus) |

## 인증 / 권한

- `ANTHROPIC_API_KEY` 또는 Claude Code CLI 인증
- `OPENAI_API_KEY` (Codex / consensus)
- `GOOGLE_AI_API_KEY` (Gemini, consensus)
- `~/.ouroboros/credentials.yaml` (chmod 600 권장)

## License 호환

- 모든 dep 가 BSD/MIT/Apache 2.0 (자유 라이선스)
- Ouroboros 자체 = MIT

## 잠재적 supply chain 위험

| 패키지 | 위험 |
|---|---|
| `litellm` | 빠른 변경, `<=1.82.6` 상한 — pinning 필요 |
| `claude-agent-sdk` | `0.1.0` 시리즈 — 메이저 변경 가능 |
| `mcp` | `1.x` 시리즈 — Anthropic MCP 진화 중 |
| `superlighttui` | `0.7.x` — 작은 생태계 |
| `textual` | `>=1.0.0,<9.0.0` 광범위 — major 호환성 가능 |

## 의존성 검증

`tests/unit/test_dependencies_configured.py` (87 LOC) — 필수 dep 가 import 가능한지 검증.

## Lock file

`uv.lock` 556 KB — 모든 transitive 잠금. CI 가 `uv sync --dev` 로 재현.

## 패키징 명세

```bash
# 휠 빌드
uv build

# 검증
uv publish --dry-run dist/*

# PyPI publish
uv publish --check-url https://pypi.org/simple/ dist/*
```

`release.yml` 가 자동 수행.

## 사용자 설치 옵션

```bash
# 1. uv (recommended)
uv tool install ouroboros-ai

# 2. pipx
pipx install ouroboros-ai[claude]

# 3. pip
pip install ouroboros-ai[claude]

# 4. curl 한 줄
curl -fsSL https://raw.githubusercontent.com/Q00/ouroboros/main/scripts/install.sh | bash

# 5. Claude Code 마켓플레이스
claude plugin marketplace add Q00/ouroboros
claude plugin install ouroboros@ouroboros
```

`scripts/install.sh` (339 LOC) 가 자동 검출 (uv → pipx → pip).

## 점진 install

`pyproject.toml` 의 extras 가 점진:
1. `pip install ouroboros-ai` — 기본 (CLI + EventStore + 인터뷰)
2. `+[claude]` — Claude Code 백엔드
3. `+[mcp]` — MCP 서버
4. `+[tui]` — Textual TUI
5. `+[litellm]` — 100+ provider
6. `+[all]` — 전체

→ 사용자가 필요한 것만 설치 가능.
