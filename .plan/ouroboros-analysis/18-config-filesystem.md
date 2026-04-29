# 18. Configuration & Filesystem

## `~/.ouroboros/` 레이아웃

```
~/.ouroboros/
├─ config.yaml                          # 메인 설정
├─ credentials.yaml                     # API 키 (chmod 600)
├─ ouroboros.db                         # SQLite EventStore (Python TUI/Rust TUI 공유)
├─ seeds/                                 # 생성된 시드 YAML
│   └─ seed_<uuid>.yaml
├─ data/                                  # interview 세션 + 기타
│   └─ interview_<id>.json                # drift-monitor 가 보는 곳
├─ logs/
│   └─ ouroboros.log                       # structlog 출력
├─ .env                                    # 선택, 자동 로드
├─ prefs.json                              # star_asked, welcomeCompleted
└─ version-check-cache.json                # 24h PyPI 캐시
```

## Config 섹션 (`~/.ouroboros/config.yaml`)

`src/ouroboros/config/loader.py` + `models.py` 가 typed 스키마 정의.

| 섹션 | 역할 |
|---|---|
| `orchestrator` | 런타임 백엔드 선택, agent 권한 모드 |
| `llm` | 모델 선택, permission mode |
| `economics` | PAL Router tier 정의, escalation 임계 |
| `clarification` | Phase 0 인터뷰 설정 |
| `execution` | Phase 2 Double Diamond 설정 |
| `resilience` | Phase 3 stagnation/lateral thinking |
| `evaluation` | Phase 4 평가 파이프라인 |
| `consensus` | 멀티 모델 합의 설정 |
| `persistence` | SQLite EventStore 설정 |
| `drift` | drift 모니터링 임계 |
| `logging` | 로그 레벨, 경로, 자세함 |

### Minimal config.yaml 예시

```yaml
orchestrator:
  runtime_backend: claude     # claude | codex | opencode | hermes

logging:
  level: info                  # debug | info | warning | error

persistence:
  database_path: data/ouroboros.db
```

### Full config.yaml 예시

```yaml
orchestrator:
  runtime_backend: claude
  permission_mode: acceptEdits         # acceptEdits | bypassPermissions | default
  cli_path: null                        # auto-detect

llm:
  default_model: claude-sonnet-4-6
  default_temperature: 0.2
  max_tokens: 4096

economics:
  default_tier: frugal
  escalation_threshold: 2
  downgrade_threshold: 5
  jaccard_threshold: 0.80
  tiers:
    frugal:
      cost_multiplier: 1
      models: [claude-haiku-4-5]
    standard:
      cost_multiplier: 10
      models: [claude-sonnet-4-6]
    frontier:
      cost_multiplier: 30
      models: [claude-opus-4-7]

clarification:
  ambiguity_threshold: 0.2
  weights:
    greenfield:
      goal: 0.40
      constraint: 0.30
      success: 0.30
    brownfield:
      goal: 0.35
      constraint: 0.25
      success: 0.25
      context: 0.15
  min_rounds_before_early_exit: 3
  soft_limit_warning_threshold: 16

execution:
  max_decomposition_depth: 5
  compression_depth: 3
  parallel_max_concurrency: 10
  ac_timeout_seconds: 600

resilience:
  spinning_threshold: 3
  oscillation_threshold: 2
  no_drift_threshold: 3
  no_drift_epsilon: 0.01
  diminishing_returns_threshold: 3
  diminishing_returns_rate: 0.01
  max_lateral_attempts: 5

evaluation:
  stage1_enabled: true
  stage2_enabled: true
  stage3_enabled: true
  mechanical:
    coverage_threshold: 0.7
    timeout_seconds: 600
  semantic:
    score_threshold: 0.8
    temperature: 0.2
    model_tier: standard

consensus:
  models: [gpt-4o, claude-sonnet-4, gemini-2.5-pro]
  majority_threshold: 0.66
  drift_trigger: 0.3
  uncertainty_trigger: 0.3
  deliberative_mode: false

persistence:
  database_path: ~/.ouroboros/ouroboros.db
  checkpoint_interval_seconds: 300       # 5분
  rollback_depth: 3

drift:
  weights:
    goal: 0.5
    constraint: 0.3
    ontology: 0.2
  threshold: 0.3
  retrospective_every_n_cycles: 5

logging:
  level: info
  path: ~/.ouroboros/logs/ouroboros.log
  verbose: false
```

## 환경 변수

| 변수 | 의미 |
|---|---|
| `ANTHROPIC_API_KEY` | Claude API 키 |
| `OPENAI_API_KEY` | OpenAI API 키 |
| `OUROBOROS_AGENT_RUNTIME` | 런타임 백엔드 override (claude / codex / opencode / hermes) |
| `OUROBOROS_AGENTS_DIR` | 커스텀 페르소나 디렉토리 |
| `OUROBOROS_ANTHROPIC_RPM_CEILING` | RPM 한계 (0 = 무제한) |
| `OUROBOROS_ANTHROPIC_TPM_CEILING` | TPM 한계 (0 = 무제한) |
| `OUROBOROS_CHILD_TIMEOUT_MS` | OpenCode bridge 자식 timeout (기본 20분) |
| `TERM=xterm-256color` | TUI 터미널 호환 |
| `XDG_CONFIG_HOME` | OpenCode bridge config (Linux) |
| `APPDATA` | OpenCode bridge config (Windows) |
| `HOME` / `USERPROFILE` | 홈 디렉토리 |

## `.env` 자동 로드

`~/.ouroboros/.env` 가 있으면 자동 로드 (dotenv 패턴).

## `credentials.yaml`

`chmod 600` 강제 — API 키 별도 파일:
```yaml
anthropic:
  api_key: sk-ant-...

openai:
  api_key: sk-...

google:
  api_key: ...
```

`config.yaml` 에서 `${env:VAR_NAME}` 같은 expansion 지원.

## `prefs.json`

Setup wizard + welcome flow 가 사용:
```json
{
  "star_asked": true,
  "welcomeCompleted": true,
  "brownfield_defaults": [6, 18, 19]
}
```

## 보안 한계 (`src/ouroboros/core/security.py`)

DoS 방어:
| 상수 | 값 | 목적 |
|---|---|---|
| `MAX_INITIAL_CONTEXT_LENGTH` | 50,000 chars | 인터뷰 초기 컨텍스트 |
| `MAX_USER_RESPONSE_LENGTH` | 10,000 chars | 인터뷰 응답 |
| `MAX_SEED_FILE_SIZE` | 1,000,000 bytes | 시드 YAML 파일 |
| `MAX_LLM_RESPONSE_LENGTH` | 100,000 chars | LLM 응답 절단 |

API 키 마스킹:
```python
mask_api_key(key)              # 마지막 4 글자만 표시
validate_api_key_format(key)
is_sensitive_field(name)
is_sensitive_value(value)
mask_sensitive_value(value)
sanitize_for_logging(d)         # 재귀 dict 정화
```

자동 마스킹 — structlog 프로세서 체인에 등록:
- `sk-` 시작 → REDACTED
- `pk-` 시작 → REDACTED
- `Bearer ` 시작 → REDACTED
- `api-`/`token `/`secret_` → REDACTED

`AuthContext.metadata` — `MappingProxyType` (frozen).

`InputValidator` — DoS 방지 + 중첩 dict 재귀 검증 (`[0.13.3]` fix).

`ValidationError._SENSITIVE_FIELDS`:
```python
frozenset({"password", "api_key", "secret", "token", "credential",
           "auth", "key", "private", "apikey", "api-key"})
```

`safe_value` 속성 — 로깅 시 마스킹된 표현 return.

## File Lock (`core/file_lock.py`)

다중 프로세스 안전 인터뷰/시드 락. heartbeat 으로 stale lock 자동 해제.

## Worktree (`core/worktree.py`)

```python
class TaskWorkspace: ...
def heartbeat_lock(...): ...
def release_lock(...): ...
```

git worktree 격리 — 평행 실행 시 충돌 방지.

## Git Workflow (`core/git_workflow.py`)

`run/SKILL.md` 가 사용:
- `CLAUDE.md` 에서 git 워크플로 선호 읽음
- PR 워크플로 + 현재 main/master → feature branch `ooo/run/<session_id>` 자동 생성
- 선호 없음 → 현재 브랜치 사용

## TTL Cache (`core/ttl_cache.py`)

라이브러리 의존성 없이 internal TTL 캐시 — version-check, ToolSearch 결과 등.

## Configuration 검증

`tests/unit/config/`:
- `test_loader.py`
- `test_loader_env.py`
- `test_models.py`

## Mechanical TOML Override

`.ouroboros/mechanical.toml` (프로젝트 루트):
```toml
build = "uv run python -m compileall -q src/"
test = "uv run pytest tests/ -x -q"
timeout = 600
```

CI/CD 보안: 실행 가능 명령 allowlist 검증.

## Setup 시 생성 파일

`skills/setup/SKILL.md` 가 자동 생성:
1. `~/.claude/mcp.json` (MCP 서버 등록)
2. `~/.ouroboros/prefs.json` (`star_asked`, `welcomeCompleted`)
3. `~/.ouroboros/` 디렉토리 자체
4. CLAUDE.md `<!-- ooo:START/END -->` 블록 (옵션)
5. CLAUDE.md.bak (백업)

## Uninstall

`UNINSTALL.md` + `cli/commands/uninstall.py`:

제거:
- `~/.claude/mcp.json` 의 `ouroboros` 항목
- CLAUDE.md `<!-- ooo:START/END -->` 블록
- (옵션) `~/.ouroboros/` 전체 (사용자 확인)

유지:
- 플러그인 파일 자체 (`claude plugin uninstall ouroboros` 별도)
- 사용자 프로젝트 데이터
