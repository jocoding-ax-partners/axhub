# 01. Plugin Identity & Distribution

## Manifest 파일 3종

### `.claude-plugin/plugin.json`
```json
{
  "name": "ouroboros",
  "version": "0.30.0",
  "description": "Self-improving AI workflow system. Crystallize requirements before execution with Socratic interview, ambiguity scoring, and 3-stage evaluation.",
  "author": {"name": "Q00", "email": "jqyu.lee@gmail.com"},
  "repository": "https://github.com/Q00/ouroboros",
  "license": "MIT",
  "keywords": ["workflow","requirements","socratic","interview","seed","evaluation","self-improving","drift-detection"],
  "skills": "./skills/",
  "mcpServers": "./.mcp.json"
}
```

### `.claude-plugin/marketplace.json`
- `$schema`: anthropic.com/claude-code/marketplace.schema.json
- `owner.name`: Q00
- `plugins[0]`: name=ouroboros, version=0.30.0, source=`./`, category=development
- tags: requirement-engineering, self-improving, crystallization

### `.claude-plugin/.mcp.json` + 루트 `.mcp.json`
```json
{"mcpServers": {"ouroboros": {"command": "uvx", "args": ["--from", "ouroboros-ai[mcp,claude]", "ouroboros", "mcp", "serve"]}}}
```
루트 `.mcp.json` 만 추가로 `"timeout": 600` 명시.

## 4 배포 채널

1. **Claude Code 마켓플레이스**: `claude plugin marketplace add Q00/ouroboros && claude plugin install ouroboros@ouroboros`
2. **PyPI**: `pip install ouroboros-ai` (또는 uv/pipx)
3. **curl 한 줄**: `curl -fsSL https://raw.githubusercontent.com/Q00/ouroboros/main/scripts/install.sh | bash`
4. **로컬 dev**: `uv sync --all-groups`

## Optional Extras (`pyproject.toml`)

```toml
[project.optional-dependencies]
claude = ["claude-agent-sdk>=0.1.0,<1.0.0", "anthropic>=0.52.0,<1.0.0"]
litellm = ["litellm>=1.80.0,<=1.82.6"]
dashboard = ["streamlit>=1.40.0,<2.0.0", "plotly>=5.24.0,<7.0.0", "pandas>=2.2.0,<3.0.0"]
mcp = ["mcp>=1.26.0,<2.0.0"]
tui = ["textual>=1.0.0,<9.0.0"]
all = ["ouroboros-ai[claude,litellm,mcp,tui,dashboard]"]
```

레거시 호환: `[dashboard]` 별칭 유지 (마이그레이션 중).

## 빌드 시스템

```toml
[build-system]
requires = ["hatchling", "hatch-vcs"]
build-backend = "hatchling.build"

[tool.hatch.version]
source = "vcs"
[tool.hatch.version.raw-options]
version_scheme = "guess-next-dev"
local_scheme = "no-local-version"

[tool.hatch.build.hooks.vcs]
version-file = "src/ouroboros/_version.py"
```

`src/ouroboros/_version.py` 는 git tag 에서 자동 생성. `__init__.py` 가 fallback 으로 `importlib.metadata.version("ouroboros-ai")` 사용.

## Hook 등록 (`hooks/hooks.json`)

```json
{
  "hooks": {
    "SessionStart": [{"matcher": "*", "hooks": [{"type": "command", "command": "python3 \"${CLAUDE_PLUGIN_ROOT}/scripts/session-start.py\"", "timeout": 5}]}],
    "UserPromptSubmit": [{"matcher": "*", "hooks": [{"type": "command", "command": "python3 \"${CLAUDE_PLUGIN_ROOT}/scripts/keyword-detector.py\"", "timeout": 5}]}],
    "PostToolUse": [{"matcher": "Write|Edit", "hooks": [{"type": "command", "command": "python3 \"${CLAUDE_PLUGIN_ROOT}/scripts/drift-monitor.py\"", "timeout": 3}]}]
  }
}
```

타임아웃 짧음 (3-5s) — hook 가 LLM 호출 안 함, 로컬 파일 검사만.

## 패키징 force-include 트릭

```toml
[tool.hatch.build.targets.wheel]
exclude = ["src/ouroboros/opencode/plugin/**"]

[tool.hatch.build.targets.wheel.force-include]
"skills" = "ouroboros/skills"
"src/ouroboros/opencode/plugin" = "ouroboros/opencode/plugin"
```

이유: 휠에서 중복 ZIP 헤더 (PyPI 거부) 방지 + `importlib.resources.files("ouroboros.opencode.plugin")` 가 ouroboros-bridge.ts 찾을 수 있게.

## Pre-commit (`.pre-commit-config.yaml`)

ruff format + ruff check + mypy.

## Python 버전 정책

`pyproject.toml`: `requires-python = ">=3.12"`.
CI 매트릭스 (test.yml): Python 3.12 / 3.13 / 3.14.
Release 빌드 (release.yml): Python 3.14.
Install.sh 검사 순서: python3.14 → python3.13 → python3.12 → python3 → python.
