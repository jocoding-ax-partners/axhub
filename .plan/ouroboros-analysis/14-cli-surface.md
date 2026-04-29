# 14. CLI Surface (Typer)

## 진입점

`pyproject.toml`:
```toml
[project.scripts]
ouroboros = "ouroboros.cli.main:app"
```

또는 `python -m ouroboros` (`__main__.py` → `ouroboros.main` → `cli.main:app`).

## 메인 (`src/ouroboros/cli/main.py`)

Typer app 생성:
```python
app = typer.Typer(
    name="ouroboros",
    help="Ouroboros - Self-Improving AI Workflow System",
    no_args_is_help=True,
    rich_markup_mode="rich",
)
```

## 12 서브 그룹 등록

| 그룹 | 모듈 | 역할 |
|---|---|---|
| `init` | `cli/commands/init.py` (29 KB) | 인터뷰 시작 |
| `run` | `cli/commands/run.py` | 시드 실행 |
| `config` | `cli/commands/config.py` | 설정 관리 |
| `status` | `cli/commands/status.py` | 실행 이력 |
| `cancel` | `cli/commands/cancel.py` | 작업 취소 |
| `mcp` | `cli/commands/mcp.py` + `mcp_doctor.py` | MCP 서버 명령 |
| `setup` | `cli/commands/setup.py` | 런타임 검출 + 설정 |
| `detect` | `cli/commands/detect.py` | 런타임 백엔드 검출 |
| `tui` | `cli/commands/tui.py` | TUI 모니터 |
| `pm` | `cli/commands/pm.py` | PM 트랙 |
| `resume` | `cli/commands/resume.py` | 세션 재개 |
| `uninstall` | `cli/commands/uninstall.py` | 설치 해제 |

## Top-level 편의 별칭

`@app.command(hidden=True)`:
```python
def monitor(backend: str = "python") -> None:
    """Launch the TUI monitor (shorthand for 'ouroboros tui monitor')."""
    tui.monitor_command(backend=backend)
```

`--backend python|slt` — Python Textual vs Rust SLT 바이너리.

## v0.8.0+ shorthand

| Shorthand | 동등 |
|---|---|
| `ouroboros run seed.yaml` | `ouroboros run workflow seed.yaml` |
| `ouroboros init "Build an API"` | `ouroboros init start "Build an API"` |
| `ouroboros monitor` | `ouroboros tui monitor` |

## 버전 콜백

```python
@app.callback()
def main(version: bool | None = typer.Option(None, "--version", "-V", callback=version_callback, is_eager=True)) -> None:
```

`--version` / `-V` → `Ouroboros version 0.30.0` 출력 + exit.

## Quick Start (Help)

```
Quick Start:
    ouroboros init "Build a REST API"     # Start interview
    ouroboros run seed.yaml                # Execute workflow
    ouroboros monitor                      # Launch TUI monitor
```

## Formatters (`cli/formatters/`)

Rich 라이브러리 기반 콘솔 출력:

| 모듈 | 역할 |
|---|---|
| `console.py` | 전역 Console 인스턴스 |
| `panels.py` | Rich Panel 출력 |
| `progress.py` | Progress bar (Rich) |
| `prompting.py` | 사용자 입력 prompt |
| `tables.py` | Rich Table 출력 |
| `workflow_display.py` | 워크플로 단계 시각화 |

## JSONC Parser (`cli/jsonc.py`)

`~/.claude/mcp.json` 등 주석-허용 JSON 처리:
- 라인 주석 `//`
- 블록 주석 `/* */`
- trailing comma

표준 JSON 으로 변환 후 `json.loads`.

## OpenCode Config Writer (`cli/opencode_config.py`)

OpenCode TS plugin 자동 설치:
1. OpenCode config 디렉토리 검출 (플랫폼별)
2. `~/.local/share/opencode/plugins/ouroboros-bridge/` 생성
3. `ouroboros-bridge.ts` + `package.json` + `tsconfig.json` 복사
4. OpenCode config.toml 에 plugin 등록

## init 명령 (`cli/commands/init.py`, 855 LOC, 29 KB) — Section 28 deep-dive

가장 큰 CLI 모듈.

### Sub-commands (직접 등록)

```bash
ouroboros init start "Build a CLI task manager"      # 인터뷰 시작
ouroboros init list                                   # 인터뷰 세션 목록
```

### `_DefaultStartGroup` shorthand

```python
class _DefaultStartGroup(typer.core.TyperGroup):
    """첫 인자가 알려진 sub-command 가 아니면 자동 'start' 로 forward"""
    def get_command(self, ctx, cmd_name):
        cmd = super().get_command(ctx, cmd_name)
        if cmd is not None:
            return cmd
        if cmd_name and not cmd_name.startswith("-"):
            return super().get_command(ctx, "start")
        return None
```

```bash
ouroboros init "Build a CLI task manager"            # = init start "..." (shorthand)
```

### 핵심 enums

```python
class SeedGenerationResult(StrEnum):
    SUCCESS = "success"
    CANCELLED = "cancelled"
    CONTINUE_INTERVIEW = "continue_interview"  # 사용자가 더 인터뷰

class AgentRuntimeBackend(StrEnum):
    CLAUDE = "claude"; CODEX = "codex"
    OPENCODE = "opencode"; HERMES = "hermes"

class LLMBackend(StrEnum):
    CLAUDE_CODE = "claude_code"; LITELLM = "litellm"
    CODEX = "codex"; OPENCODE = "opencode"
```

### `start` 옵션

| 옵션 | 별칭 | 기능 |
|---|---|---|
| `context` (positional) | — | 초기 아이디어/컨텍스트. `@path` → 파일 expansion (1 MB 제한) |
| `--resume` | `-r` | 기존 인터뷰 ID 재개 |
| `--state-dir` | — | state 디렉토리 override (default `~/.ouroboros/data/`) |
| `--orchestrator` | `-o` | Claude Code Max Plan (claude-agent-sdk). API key 불요 |
| `--runtime` | — | workflow 실행 backend (claude/codex/opencode/hermes). `--orchestrator` 없으면 무시 + warning |
| `--llm-backend` | — | 인터뷰/ambiguity/seed 생성 LLM (claude_code/litellm/codex/opencode) |
| `--debug` | `-d` | 콘솔 verbose 로깅 |

### PM seed 자동 검출

```python
seeds_dir = Path.home() / ".ouroboros" / "seeds"
if not _has_dev_seed(seeds_dir):     # seed.json 또는 non-pm yaml 없을 때
    pm_seeds = _find_pm_seeds(seeds_dir)  # pm_seed_*.yaml glob
    if pm_seeds:
        if context:
            # context 있어도 PM seed 발견 → 사용 여부 prompt
            _notify_pm_seed_detected(pm_seeds)
            use_pm = Confirm.ask(...)
        else:
            # context 없으면 PM seed 가 1차 옵션
            selected = _prompt_pm_seed_selection(pm_seeds)
            # 1 개면 yes/no, 2+ 면 번호 + 0=skip
            if selected:
                context = _load_pm_seed_as_context(selected)
                # YAML 본문이 dev interview 의 initial_context 로 자동 주입
```

→ PM 트랙으로 만든 `pm_seed_*.yaml` 가 있으면 dev interview 의 head start 로 자동 사용. README 미공개.

### Force-bypass

```python
FORCED_SCORE_VALUE = 0.19  # ambiguity ≤ 0.2 강제 통과
```

→ `--force` 플래그로 ambiguity 점수 강제 0.19 주입 → seed 생성 게이트 무조건 통과. **이 backdoor 는 docs 미공개**.

### `_run_interview_loop()` (tiered confirmation)

각 라운드 끝에서 사용자 confirmation:
- `[Y]es continue` (기본)
- `[n]o stop`
- `[m]ore detail` — 더 깊이 파고들어 가기

v0.3.0 변경:
- `MAX_INTERVIEW_ROUNDS = 10` 제거 — `is_complete` 가 상태만 검사 (`SOFT_LIMIT_WARNING_THRESHOLD = 16` 만 남음)
- 코드 중복 제거 (~60 라인)

### `list` sub-command

```python
@app.command("list")
def list_interviews(state_dir: ...):
    interviews = asyncio.run(engine.list_interviews())
    for interview in interviews:
        # interview_id | status (green/yellow) | rounds | updated_at
```

`state_dir` default = `~/.ouroboros/data/`.

## run 명령

```bash
ouroboros run workflow seed.yaml
ouroboros run seed.yaml                              # shorthand
ouroboros run --mcp-config mcp.yaml seed.yaml         # 외부 MCP 도구
```

## status 명령

```bash
ouroboros status executions                           # 모든 실행
ouroboros status execution <execution_id>             # 특정 실행
```

(MCP-only drift detection 은 별도 `ouroboros_measure_drift` MCP 도구.)

## cancel 명령

```bash
ouroboros cancel execution <execution_id>
ouroboros cancel execution --all                      # 모든 실행
```

## mcp 명령

```bash
ouroboros mcp serve                                   # stdio 서버
ouroboros mcp serve --transport sse --host 0.0.0.0 --port 8080
ouroboros mcp doctor                                  # 헬스체크
```

## setup 명령

```bash
ouroboros setup                                       # 자동 검출
ouroboros setup --runtime claude
ouroboros setup --runtime codex
ouroboros setup --runtime opencode
ouroboros setup --runtime hermes
```

## detect 명령

```bash
ouroboros detect                                      # 모든 가능 backend
```

## tui 명령

```bash
ouroboros tui monitor                                 # Python Textual (기본)
ouroboros tui monitor --backend slt                   # Rust SLT 바이너리
```

## pm 명령

```bash
ouroboros pm interview "..."                          # PM 트랙
ouroboros pm document <session_id>                    # PRD 생성
```

## resume 명령

```bash
ouroboros resume <session_id>                         # 세션 재개
```

## uninstall 명령

```bash
ouroboros uninstall                                    # 모든 config + MCP + DB 제거
```

`UNINSTALL.md` 가 상세 가이드.

## 의존 컴포넌트

- typer >= 0.12
- rich >= 13.0
- structlog (출력 마스킹)
- pyyaml (시드 파일)

## 검증

`tests/unit/cli/` 25 파일:
- bridge_plugin_hardening, bridge_plugin_lifecycle
- main, init_pm_seed_detection, init_runtime
- mcp_doctor, mcp_nested_guard, mcp_shell_env, mcp_startup_cleanup, mcp_validate_transport_stderr
- opencode_config
- pm, pm_brownfield, pm_completion, pm_interactive_logging, pm_missing_litellm, pm_overwrite, pm_runtime_adapter, pm_select_repos
- cancel, config, doc_commands, jsonc, resume, run_qa, setup, uninstall, tui_command
- formatters/ 6 (console, panels, progress, prompting, tables, workflow_display)

`tests/e2e/test_cli_commands.py` (431 LOC).
