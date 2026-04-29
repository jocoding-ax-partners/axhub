# 33. CHANGELOG History — 0.x Series Milestones + Removed Features

> US-008 deep-dive. `CHANGELOG.md` 251 LOC 전체 정독 (Unreleased ~ 0.1.0). Keep a Changelog 1.1.0 + Semver 2.0.0 준수.

## 33.1 버전 히스토리 요약

| 버전 | 날짜 | 핵심 milestone |
|---|---|---|
| Unreleased (current) | — | OpenCode subagent bridge plugin v23, lateral_think parallel multi-persona, FREETEXT_FIELDS allowlist |
| 0.14.1 | 2025-02-27 | Interview empty bypass fix + max_turns 1→3 |
| 0.13.4 | 2025-02-24 | EventStore init in ExecuteSeedHandler |
| 0.13.3 | 2025-02-24 | MCP error responses + Pydantic aliases + DependencyAnalyzer fallback |
| 0.13.2 | 2025-02-24 | rate_limit_event handling + uvx syntax |
| Unreleased (older snapshot) | — | Plugin System Phase 1 (Agent/Skill/Orchestration) — `StateStore`/`StateManager`/`RecoveryManager`/`StateCompression` 제거 |
| 0.3.0 | 2026-01-28 | Round limit 제거 + tiered confirmation |
| 0.2.0 | 2026-01-27 | Security module + Input validation + Log sanitization |
| 0.1.1 | 2026-01-15 | Initial release — 6 phase pipeline + Orchestrator |
| 0.1.0 | 2026-01-01 | Project skeleton |

> **노트**: CHANGELOG 의 날짜 형식 inconsistency 발견 — `2025-02-27` (0.14.1) 와 `2026-01-28` (0.3.0) 가 모두 존재. 0.14.x 가 0.3.x 보다 시간상 먼저인지 (2025) 또는 future-dated 오류인지 분명치 않음. 현재 시점 (2026-04-29) 기준 0.14.1 가 가장 최신 안정판으로 보임.

## 33.2 Unreleased — OpenCode Subagent Bridge

### Added

**OpenCode subagent bridge plugin** (Section 16 deep-dive 참조):
- `src/ouroboros/opencode/plugin/ouroboros-bridge.ts`
- MCP `ouroboros_*` tool calls + `_subagent` parameter → OpenCode native Task subagent panes via `session.promptAsync`
- Fire-and-forget dispatch ~10ms return (이전 `session.prompt` blocking 200s+ 대비 20000x 단축)
- `ouroboros setup` 자동 설치
- 가이드: `docs/guides/opencode-subagent-bridge.md`

**lateral_think parallel multi-persona**:
- `ouroboros_lateral_think` 가 `persona="all"` 또는 `personas=["hacker","architect",...]` 수용
- 단일 호출에서 multiple lateral-thinking persona 로 fan-out
- 각 persona 독립 LLM context — anchoring bias 제거
- 새 `_subagents` (plural) JSON contract
- 서버: `build_lateral_multi_subagent()`
- 플러그인: MAX_FANOUT=10 parallel `promptAsync` + per-payload dedupe + error isolation

**Plugin v23 changes**:
- `_subagents` array recognition for parallel fan-out
- Per-payload validation, truncation, dedupe
- 한 dispatch 실패가 나머지 abort 안 함
- 새 metadata fields: `ouroboros_subagents`, `ouroboros_dispatch_errors`
- v22 single-payload `_subagent` contract 와 backwards compatible

### Fixed

**MCP security**:
- `FREETEXT_FIELDS` allowlist for user-input fields (goals, prompts, descriptions)
- shell metacharacters (`;`, `|`, `&`, backtick, `$()`) 가 prose 정상 입력으로 통과
- 구조 필드 (struct fields) 는 strict validation 유지

**OpenCode bridge robustness (v22)**:
- 어떤 input 에서도 uncaught error 없음
- reject-path logging
- frozen-content guards
- empty-sessionID guard
- client init-order guard
- 5-second FNV-1a prompt dedupe
- 100 KB prompt byte cap with truncation marker
- 사용자에게 보이는 `surfaceErr()` for dispatch failures (silent "dispatched but never ran" 제거) — **NOTE**: CHANGELOG 의 `surfaceErr` 명칭은 실제 코드의 `fail() + notify()` 와 불일치. Section 16.27 정정 표 참조
- 절대 outer try/catch — 플러그인이 opencode runLoop 로 throw 못 함

## 33.3 0.14.1 — Interview Empty Response Fix

### Fixed

- **interview**: ClaudeCodeAdapter 에서 empty response bypass fix — empty content 가 `session_id` 무관 항상 error trigger
- **interview**: Sub-agent turn exhaustion fix — `max_turns` 1→3 으로 증가 (agent 가 tool 사용 후에도 question generate 가능)

### Maintenance

- **style**: 4 파일 `ruff format` 적용
- **ci**: ruff + mypy CI 실패 해결

## 33.4 0.13.x 시리즈 — MCP 안정화

### 0.13.4 (2025-02-24)

- **mcp**: `ExecuteSeedHandler` 가 `OrchestratorRunner` 로 넘기기 전에 `EventStore` 초기화

### 0.13.3 (2025-02-24)

대규모 MCP 안정화 릴리스:
- **mcp**: CLI 의 double-registration 제거 — DI handler 가 empty handler 로 overwrite 되던 버그
- **mcp**: 적절한 MCP error response 반환 (`isError:true`) — 이전엔 success 응답에 error text
- **mcp**: `pydantic.ValidationError` 캐치 in ExecuteSeed/MeasureDrift/Evaluate handlers
- **mcp**: `EvolutionaryLoop.evolve_step` 가 `EventStore` 접근 전에 init
- **mcp**: SSE transport 의 host/port CLI args forward
- **mcp**: dead code 제거 (discarded `EvaluationPipeline`/`LateralThinker` instances)
- **mcp**: `ClaudeAgentAdapter` init 의 invalid `llm_adapter` kwarg 제거
- **orchestrator**: `DependencyAnalyzer` error → all-parallel fallback (crash 대신)
- **seed**: Pydantic alias 추가 (`type` for `field_type`, `criteria` for `evaluation_criteria`)
- **eval**: `EvaluationPipeline`/`SeedGenerator` type annotation 을 `LiteLLMAdapter` → `LLMAdapter` Protocol 변경
- **security**: `InputValidator` 가 nested string value validation (top-level only 였음)
- **security**: `AuthContext.metadata` 에 `MappingProxyType` for frozen dataclass
- **protocol**: `MCPServer` protocol 에 `credentials` param 추가 (impl 와 match)

### Changed

- **build**: `__init__.py` 의 dynamic version via `hatchling` (single source of truth)

### 0.13.2 (2025-02-24)

- **adapter**: Claude Agent SDK 의 unknown message type (`rate_limit_event`) 가 retry logic 와 함께 처리
- **interview**: 첫 응답이 직접 question 이어야 함 (introduction 아님)
- **mcp**: uvx command syntax 정정 — `--python 3.14 --from ouroboros-ai` 사용 (proper version resolution)

## 33.5 Unreleased (older snapshot) — Plugin System Phase 1

> 큰 무명 Unreleased 블록. Phase 1 의 plugin orchestration framework. 0.13.x 시리즈 와 0.3.0 사이에 위치.

### Added — Agent System

`ouroboros.plugin.agents`:
- `AgentRegistry` — `.claude-plugin/agents/` 의 `.md` file dynamic discovery
- `AgentPool` — load balancing + auto-scaling + health monitoring
- `AgentRole` enum — ANALYSIS / PLANNING / EXECUTION / REVIEW / DOMAIN / PRODUCT / COORDINATION
- `AgentSpec` — frozen dataclass (tools / capabilities / model preferences)
- 4 builtin agents: `executor`, `planner`, `verifier`, `analyst`

### Added — Skill System

`ouroboros.plugin.skills`:
- `SkillRegistry` — `.claude-plugin/skills/` hot-reloadable discovery
- `MagicKeywordDetector` — `"ooo:"` prefix + trigger keyword routing
- `SkillExecutor` — context-aware execution + history tracking
- `SkillDocumentation` — auto-generated from SKILL.md files
- 9 new execution mode skills:
  - `autopilot` — autonomous execution from idea to working code
  - `ultrawork` — maximum parallelism
  - `ralph` — self-referential loop with verifier (includes ultrawork)
  - `ultrapilot` — parallel autopilot with file ownership partitioning
  - `ecomode` — token-efficient (haiku + sonnet)
  - `swarm` — N coordinated agents via native runtime teams
  - `pipeline` — sequential agent chaining with data passing
  - `tutorial` — interactive guided tour for new users
  - `swarm` (duplicate entry — likely doc error)

### Added — Orchestration

`ouroboros.plugin.orchestration`:
- `ModelRouter` — PAL (Progressive Auto-escalation) routing
- `Scheduler` — parallel task execution + dependency resolution via `TaskGraph`
- `RoutingContext` — complexity-aware routing + learning from history
- `ScheduledTask` — priority + dependencies + timeout

### Added — TUI HUD Components

`ouroboros.tui.components`:
- `AgentsPanel` — real-time agent pool status
- `TokenTracker` — per-agent token usage + cost estimation
- `ProgressBar` — multi-phase + animated spinners
- `EventLog` — scrolling event history + color-coded severity
- `HUDDashboard` — unified HUD screen

### Removed (CRITICAL — dead code 정리)

> **이 4 클래스 전부 삭제. 모든 runtime state EventStore/SQLite 가 관리.**

- `StateStore`
- `StateManager`
- `RecoveryManager`
- `StateCompression`

→ Section 9-recovery.md 의 EventStore-as-source-of-truth 패턴 의 origin.

### Tests

- 161 new tests (149 unit + 12 integration)
- 기존 TUI + tree tests 190 passing 유지
- Total: **1731 passing tests**

## 33.6 0.3.0 (2026-01-28) — Round Limit 제거

### Added — Documentation

- `docs/cli-reference.md` — Complete command reference
- README 의 Prerequisites section (Python 3.14+ requirement)
- Contributing section (Issues + Discussions 링크)
- OSS badges — PyPI version + Python version + License

### Added — Interview System (Tiered Confirmation)

> **MAX_INTERVIEW_ROUNDS = 10 hard limit 제거.** 새 tiered confirmation:

| Round 범위 | 동작 |
|---|---|
| 1-3 | Auto-continue (minimum context gathering) |
| 4-15 | "Continue?" ask after each round |
| 16+ | "Continue?" ask + diminishing returns warning |

새 상수:
- `MIN_ROUNDS_BEFORE_EARLY_EXIT`
- `SOFT_LIMIT_WARNING_THRESHOLD`

### Changed — Interview Engine

- `MAX_INTERVIEW_ROUNDS` hard limit 제거 (was 10)
- `is_complete` — status 만 체크 (user-controlled completion)
- `record_response()` — max round 에서 auto-complete 안 함
- System prompt 단순화 — "Round N" (이전: "Round N of 10")

### Changed — CLI Init Command

- `_run_interview_loop()` helper extract — code duplication ~60 LOC 제거
- State 저장 immediately after status mutation (consistency)
- Welcome message 가 no round limit 반영하도록 update

### Removed

- Korean-language requirement documents (`requirement/` folder)
- Interview engine 의 hard round limit enforcement

### Fixed

- `init.py` interview continuation flow 의 code duplication

## 33.7 0.2.0 (2026-01-27) — Security Module

> Section 31-governance.md + Section 29-small-modules.md 의 InputValidator origin 점.

### Added — Security Module (`ouroboros.core.security`)

**API Key Management**:
- `mask_api_key()` — 안전 mask for logging (last 4 chars 만 표시)
- `validate_api_key_format()` — basic format validation

**Sensitive Data Detection**:
- `is_sensitive_field()` — sensitive field name 탐지 (api_key, password, token, ...)
- `is_sensitive_value()` — secret-like value 탐지
- `mask_sensitive_value()` — masking
- `sanitize_for_logging()` — dict 의 sanitized 사본 생성

**Input Validation** — `InputValidator` 클래스 + DoS prevention size limits:

| 상수 | 한도 |
|---|---|
| `MAX_INITIAL_CONTEXT_LENGTH` | 50 KB |
| `MAX_USER_RESPONSE_LENGTH` | 10 KB |
| `MAX_SEED_FILE_SIZE` | 1 MB |
| `MAX_LLM_RESPONSE_LENGTH` | 100 KB |

### Added — Logging Security

- structlog processor chain 의 자동 sensitive data masking
- API key / password / token 자동 redact in 모든 log output
- nested dict recursive sanitization
- Pattern-based detection — `sk-`, `pk-`, `Bearer`, ... 시작 value

### Changed

**Interview Engine**:
- 일관된 size limit 으로 `InputValidator` 사용
- `start_interview()` initial context length validate
- `record_response()` user response length validate

**LiteLLM Adapter**:
- LLM response validate + size limit 초과 시 truncate
- truncation 발생 시 warning log

**CLI Run Command**:
- Seed file size validate before loading
- 과대 seed file 보호

### Security Hardening

- API Keys: Masked in logs, only provider prefix + last 4 chars
- Input Validation: 모든 external input size limit (DoS 방지)
- Log Sanitization: 자동 sensitive data masking
- Credentials Protection: `credentials.yaml` chmod 600 유지

### Tests

- security 모듈 39 tests
- logging sensitive data masking 5 tests
- 1341 tests all passing

## 33.8 0.1.1 (2026-01-15) — Initial Public Release

### Added

> **Section 04~10 의 6 Phase pipeline 의 origin 점.**

- Big Bang (Phase 0) — Interview + Seed generation
- PAL Router (Phase 1) — Progressive Adaptive LLM selection
- Double Diamond (Phase 2) — Execution engine
- Resilience (Phase 3) — Stagnation detection + lateral thinking
- Evaluation (Phase 4) — Mechanical + semantic + consensus
- Secondary Loop (Phase 5) — TODO registry + batch scheduler
- Orchestrator (Epic 8) — Runtime abstraction + orchestration
- CLI interface with Typer
- Event sourcing with SQLite persistence
- Structured logging with structlog

### Fixed

- Various bug fixes + stability improvements (구체적 list 없음)

## 33.9 0.1.0 (2026-01-01) — Project Skeleton

### Added

- 초기 프로젝트 구조
- Core types + error hierarchy
- Basic configuration system

## 33.10 핵심 milestone 요약 (architectural)

### Phase 1 (0.1.x): 6-Phase Pipeline 정립
- 0.1.0 — skeleton
- 0.1.1 — 6 Phase + EventStore + Orchestrator

### Phase 2 (0.2.x): Security 우선순위
- 0.2.0 — `ouroboros.core.security` 모듈 + InputValidator + log sanitization

### Phase 3 (0.3.x): UX — round limit 제거
- 0.3.0 — interview round limit 제거 + tiered confirmation
- → user-driven exploration ↔ machine-enforced limit 의 trade-off 가 user 쪽으로 결정

### Phase 8 (Plugin System): 확장성 + dead code 정리
- `StateStore`/`StateManager`/`RecoveryManager`/`StateCompression` 4 클래스 모두 제거 — EventStore 단일 source of truth 로 통일
- `AgentRegistry` + `AgentPool` + `SkillRegistry` + `ModelRouter` 추가 — extensibility framework
- 9 새 execution mode skill (autopilot / ultrawork / ralph / ultrapilot / ecomode / swarm / pipeline / tutorial)
- TUI HUD components (AgentsPanel / TokenTracker / ProgressBar / EventLog / HUDDashboard)
- 1731 passing tests baseline 확립

### Phase 13 (0.13.x): MCP 안정화 시리즈
- 0.13.2 — adapter rate limit + interview first-question
- 0.13.3 — 12 MCP fix (가장 큰 안정화 릴리스)
- 0.13.4 — EventStore init order

### Phase 14 (0.14.x): Interview robustness
- 0.14.1 — empty response bypass + max_turns 1→3

### Phase Unreleased (current): OpenCode 통합
- OpenCode subagent bridge plugin (Section 16 deep-dive)
- lateral_think parallel multi-persona
- FREETEXT_FIELDS allowlist
- Plugin v22→v23 evolution

## 33.11 제거된 기능 list (Removed)

| 버전 | 제거된 항목 | 대체 / 이유 |
|---|---|---|
| Plugin Phase 1 | `StateStore` | EventStore (SQLite) 가 단일 source of truth |
| Plugin Phase 1 | `StateManager` | 동상 |
| Plugin Phase 1 | `RecoveryManager` | EventStore replay 으로 대체 |
| Plugin Phase 1 | `StateCompression` | 불필요 — SQLite 가 관리 |
| 0.3.0 | `MAX_INTERVIEW_ROUNDS` (=10) hard limit | tiered confirmation 으로 대체 |
| 0.3.0 | `requirement/` 폴더 (Korean docs) | 영어 docs 로 통일 |
| 0.13.3 | discarded `EvaluationPipeline`/`LateralThinker` instances | dead code |
| 0.13.3 | `llm_adapter` kwarg in `ClaudeAgentAdapter` init | invalid |

## 33.12 CHANGELOG 의 inconsistency / 위험 신호

### 날짜 형식 inconsistency
- 0.14.1 = `2025-02-27`
- 0.13.x = `2025-02-24`
- 0.3.0 = `2026-01-28`
- 0.2.0 = `2026-01-27`
- 0.1.1 = `2026-01-15`

→ 시간상 0.14.x (2025) 가 0.3.x (2026) 보다 먼저인지, 0.14.x 가 future-dated 오류인지 분명치 않음. 진단:
- (a) 0.1 → 0.2 → 0.3 (2026-01) 순서 후 매우 빠른 0.13 → 0.14 (2025-02) 점프 = 비정상
- (b) 0.14.x 가 실제로는 2026-02-27 이라면 — typo
- (c) Plugin Phase 1 에서 1731 tests baseline → 0.13.x 에서 다시 안정화 → 0.3.0 에서 round limit 제거 = chronological order 와 충돌

→ **CHANGELOG 의 날짜 정합성 점검 필요** (외부 review C-level finding 후보).

### Unreleased 블록 두 개

> 가장 최근 Unreleased (OpenCode bridge) + 옛 Unreleased (Plugin System Phase 1) 둘 다 동시 존재.

→ 두 번째 Unreleased 는 어떤 버전으로 ship 됐는지 불명. 명시적 release section 으로 변환 필요.

### Skill list 중복

> 9 새 skill list 에 `swarm` 두 번 등장 ("N coordinated agents using native runtime teams" 와 "Team coordination mode")

→ doc error.

## 33.13 외부 review 와 cross-reference

### Section 30 (External Reviews) 와의 일관성

CHANGELOG 의 Unreleased (OpenCode bridge) 가 Section 30 의 Hermes integration audit 과 **다른 PR**:
- Section 30 = Hermes runtime + skills package shadowing (Phase 21 이후)
- CHANGELOG Unreleased = OpenCode subagent bridge plugin v22/v23

→ 두 PR series 가 진행 중 / 별개 lane.

### Section 29 (small modules) 와 cross-reference

- 0.2.0 의 `InputValidator` = Section 29.7 의 `InputValidator` 본체 (50KB / 10KB / 1MB / 100KB 한도)
- 0.13.3 의 "Validate nested string values in InputValidator, not just top-level" = Section 29 의 nested validation 패턴

### Section 27 (gaps) update 후보

CHANGELOG 정독 으로 새로 채워진 영역:
- StateStore / StateManager / RecoveryManager 가 **언제 제거됐는지** = Plugin Phase 1 (Unreleased older block)
- MAX_INTERVIEW_ROUNDS hard limit 의 **제거 시점** = 0.3.0 (2026-01-28)
- InputValidator 의 **정확 도입 시점** = 0.2.0 (2026-01-27)
- 1731 passing tests baseline 의 **확립 시점** = Plugin Phase 1
