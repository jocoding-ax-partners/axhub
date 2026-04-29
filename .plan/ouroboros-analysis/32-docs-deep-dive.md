# 32. Docs Deep-Dive — 33 Documentation Files 핵심 요약

> US-005 deep-dive. `docs/` 의 33 markdown 파일 전부 정독 (또는 헤더 grep). 각 문서의 핵심 가치 + 어디서 사용하는지.

## 32.1 디렉토리 구조

```
docs/
├─ README.md                              # 문서 인덱스 (89 LOC)
├─ getting-started.md                     # 단일 onboarding source (413 LOC)
├─ architecture.md                        # 6-phase 아키텍처 (524 LOC)
├─ cli-reference.md                       # CLI 모든 명령/플래그 (780 LOC)
├─ config-reference.md                    # config.yaml 모든 키 (670 LOC)
├─ events.md                              # EventStore 페이로드 스키마 (117 LOC)
├─ platform-support.md                    # OS/Python 호환성 (63 LOC)
├─ runtime-capability-matrix.md           # 4 backend 비교 표 (116 LOC)
├─ api/
│  ├─ README.md                            # API 인덱스 (54 LOC)
│  ├─ core.md                              # Result, Seed, errors (456 LOC)
│  └─ mcp.md                               # MCP 모듈 API (891 LOC)
├─ runtime-guides/
│  ├─ claude-code.md                       # Claude Code 백엔드 가이드 (129 LOC)
│  ├─ codex.md                             # Codex CLI 가이드 (283 LOC)
│  ├─ opencode.md                          # OpenCode 가이드 (370 LOC)
│  └─ hermes.md                            # Hermes 가이드 (70 LOC)
├─ guides/
│  ├─ seed-authoring.md                    # Seed YAML 작성 (745 LOC)
│  ├─ evolution-loop.md                    # Wonder/Reflect 사이클 (140 LOC)
│  ├─ evaluation-pipeline.md               # 3-stage 평가 (541 LOC)
│  ├─ tui-usage.md                         # TUI 키바인딩/스크린 (188 LOC)
│  ├─ mcp-bridge.md                        # 서버-to-서버 MCP (97 LOC)
│  ├─ mcp-best-practices.md                # 외부 MCP 서버 추가 (134 LOC)
│  ├─ qa-backends.md                       # 외부 QA backend (84 LOC)
│  ├─ opencode-subagent-bridge.md          # OpenCode 플러그인 (270 LOC)
│  ├─ ooo-skill-dispatch-router.md         # `ouroboros.router` 공유 (70 LOC)
│  └─ issue-176-subagent-mcp-inheritance.md # Issue 176 분석 (40 LOC)
├─ contributing/
│  ├─ architecture-overview.md             # 모듈 의존도 (174 LOC)
│  ├─ key-patterns.md                      # 7 핵심 패턴 (236 LOC)
│  ├─ testing-guide.md                     # 테스트 작성 (211 LOC)
│  ├─ findings-registry.md                 # docs audit 등록부 (1370 LOC)
│  └─ issue-quality-policy.md              # PRD-lite issue 강제 (29 LOC)
├─ examples/workflows/
│  ├─ research-to-deliverable.md           # Tavily/Context7 워크플로 (82 LOC)
│  └─ design-code-verify.md                # Figma/Context7/OpenCron (93 LOC)
└─ images/PLACEHOLDER_README.md            # 이미지 placeholder (18 LOC)
```

총 33 files, 9547 LOC.

## 32.2 Top-level docs

### `docs/README.md` (89 LOC) — 인덱스

문서 7 카테고리 (Getting Started / Runtime Guides / Architecture / API Reference / Guides / Contributing / Quick Links).

**Key Concepts** 발췌:
- 6 phase: Big Bang / PAL Router / Double Diamond / Resilience / Evaluation / Secondary Loop
- Economic model: FRUGAL 1x (<0.4), STANDARD 10x (<0.7), FRONTIER 30x (critical)
- Core principles: "Frugal by default, rigorous in verification", "Ambiguity ≤ 0.2 게이트", "Lateral thinking 페르소나 rotation"

### `docs/getting-started.md` (413 LOC) — 단일 진실원

> "**Single source of truth for onboarding.** All install and first-run instructions live here."

#### Quick Start 2 경로

```bash
# 권장: Claude Code 플러그인
claude plugin marketplace add Q00/ouroboros
claude plugin install ouroboros@ouroboros
# inside Claude Code:
ooo setup
ooo interview "Build a task management CLI"
ooo run

# 대안: Standalone CLI (Python ≥ 3.12)
pip install ouroboros-ai
ouroboros setup
ouroboros run ~/.ouroboros/seeds/seed_abc123.yaml
```

#### 4 Install 옵션
1. Claude Code Plugin (권장) — Python 불요
2. `pip install ouroboros-ai[claude|litellm|mcp|tui|all]`
3. From source — `git clone + uv sync`
4. One-liner: `curl ... | bash` (auto-detect runtime)

#### 핵심 환경 변수

```bash
export ANTHROPIC_API_KEY=...     # Claude flows
export OPENAI_API_KEY=...        # Codex flows
export OUROBOROS_AGENT_RUNTIME=codex  # config 덮어쓰기 (highest priority)
```

#### Resolution order

`OUROBOROS_AGENT_RUNTIME` env → `config.yaml` → `ouroboros setup` 자동 검출.

#### 4 Step 워크플로

```
Step 1: ooo interview "..."        # Socratic + ambiguity ≤ 0.2 게이트
Step 2: ooo run                     # Double Diamond 실행
Step 3: ouroboros monitor           # TUI 모니터 (별도 터미널)
Step 4: ooo evaluate / ooo status / ooo evolve
```

### `docs/platform-support.md` (63 LOC) — OS 호환성

| Platform | Status | Notes |
|---|---|---|
| macOS (ARM/Intel) | Supported | Primary CI |
| Linux x86_64/ARM64 | Supported | Ubuntu 22.04+, Debian 12+, Fedora 38+ |
| Windows (WSL 2) | Supported | recommended Windows |
| Windows (native) | **Experimental** | POSIX path 가정 / Codex CLI 미지원 / `cmd.exe` 미지원 |

Python: 3.12 / 3.13 / 3.14+ 지원. < 3.12 비지원.

Codex CLI native Windows = Not supported. WSL 2 사용 강요.

### `docs/runtime-capability-matrix.md` (116 LOC) — 4 backend 비교

#### Workflow 레이어 (모두 동일)

| 기능 | Claude/Codex/OpenCode/Hermes |
|---|---|
| Seed parsing | Yes |
| AC tree | Yes |
| Event sourcing (SQLite) | Yes |
| Checkpoint / resume | Yes |
| TUI dashboard | Yes |
| Interview | Yes |
| Dry-run | Yes |

#### Runtime 레이어 (다름)

| 측면 | Claude Code | Codex CLI | OpenCode | Hermes |
|---|---|---|---|---|
| Auth | Max Plan | OpenAI API key | Provider keys (OpenCode-internal) | API key 또는 local model |
| Underlying model | Claude (Anthropic) | GPT-5.4+ | Provider-dependent | Provider-dependent or local |
| Tool surface | Read/Write/Edit/Bash/Glob/Grep | Codex-native | 같음 | Custom skills via MCP |
| Sandbox | Claude Code permission | Codex sandbox | OpenCode permission | Hermes permission |
| Cost | Max Plan 포함 | Per-token | Provider 의존 | API/Local 의존 |

→ "**No implied parity**" — 각 runtime 독립 product.

### `docs/architecture.md` (524 LOC, 헤더만 grep — 1차 라운드 cover)

6 phase 본문 + AC tree decomposition + LevelCoordinator + 30 generation cap + 3-stage 평가 + AC tree HUD 본체. Section 5 의 base 자료.

### `docs/cli-reference.md` (780 LOC) — 모든 CLI 명령

#### 명령 카탈로그

```
ouroboros setup                 # 자동 검출 + 설정
ouroboros init                  # 인터뷰 (start / list)
ouroboros run                   # workflow / resume
ouroboros cancel                # execution
ouroboros config                # show / backend / init / set / validate
ouroboros mcp                   # serve / doctor
ouroboros tui                   # monitor
ouroboros pm                    # PM 인터뷰
ouroboros resume                # 세션 재개
ouroboros uninstall             # 깨끗한 제거
ouroboros detect                # backend 검출
```

#### 자세한 옵션 (init start 예시 인용)

```bash
# Shorthand (recommended)
ouroboros init "Build me a CLI"

# Explicit
ouroboros init start "Build me a CLI"

# Claude Code Max Plan (no API key)
ouroboros init start --orchestrator "..."

# Codex backend
ouroboros init start --orchestrator --runtime codex "..."

# Codex LLM for interview
ouroboros init start --llm-backend codex "..."

# Resume
ouroboros init start --resume interview_20260116_120000

# Interactive (no positional)
ouroboros init start
```

→ 모든 명령에 대해 example 다수 제공. Section 14 의 base.

### `docs/config-reference.md` (670 LOC) — config.yaml 모든 키

12 top-level 섹션:
- `orchestrator` (runtime_backend, codex_cli_path)
- `llm` (backend, qa_model)
- `economics` (tier 비용 multipliers)
- `clarification` (ambiguity weights, default_model)
- `execution` (parallel_max_concurrency)
- `resilience` (stagnation/lateral)
- `evaluation` (mechanical/semantic 임계)
- `consensus` (advocate/devil/judge models)
- `persistence` (DB 경로)
- `drift` (가중치, 임계)
- `logging` (level)
- `credentials.yaml` (~/.ouroboros/, chmod 600)

#### Codex Role Override Map

Codex 의 4 phase model 별도 override:
```yaml
# ~/.ouroboros/config.yaml
llm:
  backend: codex
  qa_model: gpt-5.4
clarification:
  default_model: gpt-5.4
evaluation:
  semantic_model: gpt-5.4
consensus:
  advocate_model: gpt-5.4
  devil_model: gpt-5.4
  judge_model: gpt-5.4
```

#### Env 변수 (5 카테고리)

1. **Runtime / Backend**: `OUROBOROS_AGENT_RUNTIME`, `OUROBOROS_RUNTIME_BACKEND`
2. **LLM Flow**: `OUROBOROS_LLM_BACKEND`
3. **Phase Models**: 페이즈별 모델 override
4. **MCP Evolution**: MCP 진화 모드
5. **Observability & Agents**: log level
6. **API Keys**: `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GOOGLE_AI_API_KEY`

### `docs/events.md` (117 LOC) — EventStore 페이로드 스키마

#### Versioning

```python
event_version = 0  # legacy (field absent)
event_version = 1  # baseline stable
```

페이로드 JSON 안에 `event_version` 정수. 별도 column 아님 (마이그레이션 회피).

#### 7 stable event types (v1)

| Event type | 핵심 필드 |
|---|---|
| `orchestrator.session.started` | execution_id, seed_id, start_time |
| `orchestrator.session.completed` | summary |
| `orchestrator.session.cancelled` | reason, cancelled_by ("user"/"auto_cleanup"/agent ID) |
| `orchestrator.session.failed` | error |
| `execution.ac.completed` | ac_id, status ("passed"/"failed") |
| `mcp.job.cancelled` | status ("cancelled"), message |
| `orchestrator.progress.updated` | progress (object), progress.runtime_status |

→ stability guarantee: 같은 version 내에서 필드 절대 제거/이름변경 안 함. 새 필드 추가는 OK.

## 32.3 Runtime Guides (4)

### `runtime-guides/claude-code.md` (129 LOC)

> Claude Code Pro/Max Plan + claude-agent-sdk. **API key 불요**.

```yaml
orchestrator:
  runtime_backend: claude
```

`--orchestrator` 플래그 시 default. 도구: Read/Write/Edit/Bash/Glob/Grep.

**비용**: Pro $20/month (lower limits) → **Max plan 권장** (long agentic workflow 시).

### `runtime-guides/codex.md` (283 LOC, headers grep)

Codex CLI 백엔드. `npm install -g @openai/codex` + `OPENAI_API_KEY`.

추천 모델: GPT-5.4 + medium reasoning effort.

`ouroboros setup --runtime codex` 가:
- `~/.codex/config.toml` 에 MCP/env hookup
- `~/.codex/rules/ouroboros.md` 에 rules
- `~/.codex/skills/ouroboros/` 에 skills 설치

### `runtime-guides/opencode.md` (370 LOC, headers grep)

OpenCode runtime. 두 모드:
- **Plugin mode** (`opencode_mode=plugin`, default): TS bridge plugin via fire-and-forget
- **Headless subprocess mode**: 직접 subprocess 호출

Section 16 + Section 28 에 자세히.

### `runtime-guides/hermes.md` (70 LOC)

> NousResearch Hermes Agent (`ouroboros setup --runtime hermes`).

```bash
ouroboros run seed.yaml --runtime hermes
```

세션 추적: `session_id` (Hermes CLI 의 `-Q` quiet mode 출력).

`hermes_cli_path` config 키로 path override.

→ 1차 라운드의 Code-Review-Claude/Codex 가 발견한 H1 (timeout 부재), H2 (depth tracking 부재) 등 결함은 docs 에 미반영. docs 가 capability 과장 — Section 30 참조.

## 32.4 Guides (10)

### `guides/seed-authoring.md` (745 LOC, headers grep) — Seed YAML 완전 가이드

#### Schema (필수 + 선택)

**필수**:
- `goal` (구체적, 측정 가능, 경계 있음)
- `ontology_schema` (name, description, fields)
- `metadata` (ambiguity_score)

**선택 (강력 권장)**:
- `task_type` (default "code", `research`, `analysis`)
- `constraints`
- `acceptance_criteria` (스키마 강제 안 되지만 강력 추천)
- `evaluation_principles`
- `exit_conditions`

#### Good/Bad 예시

```yaml
# Good: 구체, 측정 가능, 경계
goal: "Build a CLI task tracker with SQLite, supports add/list/complete commands"

# Bad: 모호, 무경계
goal: "Build a productivity app"
```

#### 3 완전 예시 (Code / Research / Analysis)

**Code task** — REST API
**Research task** — Technology Comparison
**Analysis task** — Architecture Decision

#### 검증

```bash
# Claude Code path
ooo seed validate

# Standalone CLI path
ouroboros run seed.yaml --dry-run
```

#### Troubleshooting (4 phase 별 실패 모드)

1. Phase 1 (Interview) 실패 — API key 부재, LiteLLM 오류
2. Phase 2 (Ambiguity) 실패 — JSON parse 에러
3. Phase 3 (Seed 생성/저장) 실패 — file system 에러
4. Manually written seeds — yaml 검증

### `guides/evolution-loop.md` (140 LOC)

> "Ouroboros eats its tail" — 진화 핵심.

```
Gen 1:  Seed(O₁) → Execute → Evaluate
Gen 2+: Wonder(Oₙ,Eₙ) → Reflect → Seed(Oₙ₊₁) → Execute → Evaluate
... until convergence (≥0.95) or 30 generations
```

#### Convergence 가중치

```
Similarity = 0.5*name_overlap + 0.3*type_match + 0.2*exact_match
```

#### 4 종료 신호

| 신호 | 조건 | Default |
|---|---|---|
| Ontology stability | similarity(Oₙ, Oₙ₋₁) ≥ 0.95 | ≥ 0.95 |
| Stagnation | 3 연속 ≥ 0.95 | 3 gens |
| Oscillation | Gen N ≈ Gen N-2 (period-2) | Enabled |
| Hard cap | 30 generations | 30 |

→ min 2 generations 후에만 1-3 검사.

#### Ralph vs Evolve

| | `ooo evolve` | `ooo ralph` |
|---|---|---|
| Scope | 단일 evolution step | Loop until convergence |
| Session | within current | survives restarts |
| Control | Manual | Automatic (convergence) |

#### "Two Mathematical Gates"

```
1. Ambiguity ≤ 0.2  — Do not build until clear (interview gate)
2. Similarity ≥ 0.95 — Do not stop evolving until stable (convergence gate)
```

### `guides/evaluation-pipeline.md` (541 LOC, headers grep) — 3-stage detail

3 Stage:
- **Stage 1 Mechanical**: Lint/test/build/static/coverage. cost $0
- **Stage 2 Semantic**: LLM 평가 (AC compliance, drift, goal alignment). cost $$
- **Stage 3 Consensus**: Multi-model vote (only if triggered). cost $$$$

#### Stage 3 trigger (6 conditions)

1. Seed modification
2. Ontology evolution
3. Goal reinterpretation
4. Seed drift > 0.3
5. Stage 2 uncertainty > 0.3
6. Lateral thinking adoption

→ 임계 ≥ 1 개 만족 시 Stage 3 실행.

#### 두 모드의 Stage 3

- **Simple consensus** (default): 단순 majority vote
- **Deliberative consensus** (`conditional` 결과 사용): Advocate / Devil / Judge 2-라운드 토론

#### Stage 별 실패 모드 + Configuration + Diagnosing

각 stage 마다 별도 섹션. 매우 detailed.

### `guides/tui-usage.md` (188 LOC)

#### 4 메인 스크린

| Key | 스크린 | 용도 |
|---|---|---|
| `1` | Dashboard | 페이즈 progress + AC 트리 + node detail |
| `2` | Execution | 타임라인 + 페이즈 출력 |
| `3` | `l` Logs | level-colored log viewer |
| `4` | `d` Debug | state inspector + raw events |
| `s` | Session Selector | 세션 전환 |
| `e` | Lineage | 진화 lineage |

#### Dashboard 의 status icons

| Icon | 의미 |
|---|---|
| `○` (dim) | Pending |
| `⊘` (red) | Blocked |
| `◐` (yellow) | Executing |
| `●` (green) | Completed |
| `✖` (red) | Failed |
| `◆` (blue) | Atomic (leaf) |
| `◇` (cyan) | Decomposed (has children) |

#### EventStore polling

500ms 간격 polling → message dispatch → screen handler.

```
EventStore → app._subscribe_to_events() (poll 0.5s)
           → create_message_from_event()
           → post_message() → screen handlers
```

12 message types (PhaseChanged, ACUpdated, WorkflowProgressUpdated, ExecutionUpdated, SubtaskUpdated, DriftUpdated, CostUpdated, ToolCallStarted/Completed, AgentThinkingUpdated, ParallelBatchStarted/Completed).

### `guides/mcp-bridge.md` (97 LOC)

server-to-server MCP 통신.

```
Claude Session (Host)
  └── Ouroboros MCP Server
        ├── MCPBridge → MCPClientManager
        │     ├── openchrome MCP
        │     ├── filesystem MCP
        │     └── database MCP
        └── ExecuteSeedHandler → OrchestratorRunner (merged tools)
              └── Child Agent (native + external tools)
```

#### Config 검색 순서

1. `$OUROBOROS_MCP_CONFIG` env
2. `~/.ouroboros/mcp_servers.yaml`
3. `{cwd}/.ouroboros/mcp_servers.yaml`

#### Limitation

- Evolution loop (`evolve_step`) 가 bridge manager 안 받음 (미구현)
- Dynamic server 추가 안 됨 (initial connection 후)

### `guides/mcp-best-practices.md` (134 LOC)

#### 추천 4 server roles

| 서버 | 사용 시점 |
|---|---|
| OpenCron | 스케줄 브라우저/synthetic check (QA/모니터링만) |
| Figma | 디자인 인스펙션 (read-only) |
| Context7 | 라이브러리/프레임워크 docs (planning/implementation 동안) |
| Tavily | 외부 web research |

#### Security 규칙

- Read-only 토큰 우선 (Figma, docs)
- Browser/filesystem 도구 research-only 워크플로에 부여 금지
- API key 는 env var (`${VAR_NAME}`) 형식
- `mcp_servers.yaml` 을 shared logs 에 노출 금지

#### Reliability 규칙

- Server 별 timeout = expected latency (browser > docs)
- `connection.retry_attempts` ≤ 2-3
- Health checks 만으로 의존하지 말고 per-call timeout 도

#### Workflow → 서버 매핑

| 워크플로 | 추천 서버 |
|---|---|
| Research → Deliverable | Tavily, Context7 |
| Design → Code → Verify | Figma, Context7, OpenCron |
| Library upgrade | Context7, optional Tavily |
| Launch QA | OpenCron, optional Context7 |

### `guides/qa-backends.md` (84 LOC)

> 외부 QA backend 패턴. 2 layers — in-process (`ouroboros_qa`) + upstream MCP.

#### OpenCron backend 예시

```yaml
mcp_servers:
  - name: opencron
    transport: stdio
    command: "<opencron-mcp-command>"
    env:
      OPENCRON_BASE_URL: "${OPENCRON_BASE_URL}"
      OPENCRON_API_KEY: "${OPENCRON_API_KEY}"
    timeout: 60
connection:
  timeout_seconds: 45
  retry_attempts: 2
  health_check_interval: 60
```

#### When to use

- Deployed URL 검증
- 브라우저/엔드포인트 check (로컬 unit test 로 표현 안 되는)
- 스케줄/webhook
- PR QA 증거

#### Avoid

- Pure unit test
- Static code review
- Production write 필요한 check

#### 결과 처리

- 외부 QA 출력 = 증거로만 첨부 (sole source 아님)
- 로컬 mechanical test 우선
- backend unavailable → blocked/inconclusive 처리. silent pass 금지.

### `guides/opencode-subagent-bridge.md` (270 LOC) — 플러그인 가이드

> Section 16 + Section 28 에 자세히. 본 docs 의 추가:

#### 11 도구가 envelope 디스패치

| Tool | Envelope | Child role |
|---|---|---|
| `ouroboros_qa` | `_subagent` | QA judge |
| `ouroboros_lateral_think` (`persona=all`) | `_subagents` | one per persona |
| `ouroboros_lateral_think` (single) | `_subagent` | single persona |
| `ouroboros_interview` | `_subagent` | Socratic interviewer |
| `ouroboros_pm_interview` | `_subagent` | PM interviewer |
| `ouroboros_generate_seed` | `_subagent` | seed architect |
| `ouroboros_execute_seed` | `_subagent` | executor |
| `ouroboros_start_execute_seed` | `_subagent` | bg executor |
| `ouroboros_evolve_step` | `_subagent` | evolution gen |
| `ouroboros_start_evolve_step` | `_subagent` | bg evolution |
| `ouroboros_evaluate` | `_subagent` | evaluator |

#### 환경 변수

| 변수 | Default | 용도 |
|---|---|---|
| `OUROBOROS_CHILD_TIMEOUT_MS` | 1,200,000 (20 min) | per-child timeout |
| `OUROBOROS_SUB_RETRIES` | 2 | extra retries |

#### 설치 보장

- atomic (`os.replace`)
- idempotent (SHA-256 content hash)
- duplicated stale entries 정리 (sudo migrations, XDG shifts, legacy paths)

### `guides/ooo-skill-dispatch-router.md` (70 LOC) — `ouroboros.router`

> 공유 dispatch path. Codex / Hermes / OpenCode 가 같은 router 사용.

#### Setup contract

```bash
ouroboros setup --runtime codex     # 또는
ouroboros setup --runtime hermes    # 또는
ouroboros setup --runtime opencode
```

→ packaged `skills/*/SKILL.md` 설치 + MCP 서버 등록.

#### 5-step deterministic resolution

1. Parse `ooo <skill>` / `/ouroboros:<skill>` 명령 prefix
2. Skill name/alias 를 packaged `SKILL.md` 에 매칭
3. `mcp_tool` / `mcp_args` frontmatter validate
4. `$1`, `$CWD` template substitution
5. Runtime-neutral dispatch metadata 반환

#### Runtime boundary

router 는 stateless. Logging 안 함, AgentMessage assembly 안 함, MCP handler 호출 안 함.

→ Codex/Hermes/OpenCode 책임:
- caller-observable 구조화 logging
- runtime-specific AgentMessage assembly
- `mcp_tool` 호출
- non-dispatch prompt 의 normal runtime path fallthrough

#### Adding commands

`SKILL.md` 만 변경. Runtime parser branch 추가 금지.

```yaml
---
name: run
description: Execute a Seed specification through the workflow engine
mcp_tool: ouroboros_execute_seed
mcp_args:
  seed_path: "$1"
  cwd: "$CWD"
---
```

### `guides/issue-176-subagent-mcp-inheritance.md` (40 LOC) — Issue 176 분석

#### Problem

Delegated `ooo run` subagents 가 새 `OrchestratorRunner` 생성 → parent 의 merged MCP tools 누락 → session-bound MCP (예: Chrome DevTools MCP) 가 silently drop.

#### Design (5 step)

1. `ClaudeAgentAdapter.execute_task()` 가 `PreToolUse` hook 등록
2. `ouroboros_execute_seed` 또는 `ouroboros_start_execute_seed` 호출 시 internal-only metadata 주입:
   - parent Claude session ID
   - parent transcript path / cwd / permission mode
   - parent effective tool list
3. `ExecuteSeedHandler.handle()` 가 internal field 읽음 → `RuntimeHandle` 의 `metadata={"fork_session": True}` 로 child Claude run 이 parent session 에서 fork
4. inherited runtime handle + tool list → delegated `OrchestratorRunner`
5. delegated runner 가 inherited tool 을 local tool set 에 merge + inherited handle 을 direct/parallel/coordinator 모두에 전달

#### Compatibility 보존

- public CLI / MCP tool 변경 없음
- `orchestrator.session.*` event schema 변경 없음
- inheritance metadata = internal-only

## 32.5 Contributing (5)

### `contributing/architecture-overview.md` (174 LOC)

#### High-Level Flow

```
User → Phase 0 Big Bang → Seed (Ambiguity ≤ 0.2)
     → Phase 1 PAL Router (Frugal/Standard/Frontier 선택)
     → Phase 2 Double Diamond (decompose + execute via runtime)
     → Phase 3 Resilience (stagnation 감지 + persona 회전)
     → Phase 4 Evaluation (Stage 1 mechanical / Stage 2 semantic / Stage 3 consensus)
     → Phase 5 Secondary Loop (TODO batch 처리)
     → 반복 가능
```

#### Module dependency map

```
                     core/
                  (types, errors, seed)
                  /    |    \
            bigbang/  routing/  execution/
                  \    |    /
                  orchestrator/ (runner, adapter, parallel_executor, execution_strategy)
                       |
                  +----+----+
                  |         |
            evaluation/   resilience/
                  |
            persistence/ (event_store)
                  |
            tui/  +  cli/
```

#### Key module guide (각 패키지 별 표)

`core/`, `evaluation/`, `orchestrator/`, `tui/`, `providers/`, `persistence/` 별로 파일 목록 + 책임.

#### TUI state flow

```
EventStore --> app._subscribe_to_events() (0.5s poll)
            --> create_message_from_event()
            --> post_message() → screen handlers
```

→ `app.py` 가 `_state.ac_tree` 의 SSOT (single source of truth).

### `contributing/key-patterns.md` (236 LOC) — 7 핵심 패턴

| # | 패턴 | 위치 |
|---|---|---|
| 1 | `Result[T, E]` 타입 (예외 대신 expected failure) | `core/types.py` |
| 2 | Frozen dataclass + Pydantic frozen=True (immutability) | 전반 |
| 3 | Event sourcing (append-only) | `events/`, `persistence/event_store.py` |
| 4 | Protocol class (ABC 대신 duck typing) | `orchestrator/execution_strategy.py` |
| 5 | 3-stage evaluation pipeline | `evaluation/pipeline.py` |
| 6 | TUI state management (SSOT in `app.py`) | `tui/app.py`, `tui/events.py` |
| 7 | Seed immutability (frozen + tuple instead of list) | `core/seed.py` |

#### Result 사용 규칙

- 외부 요인 실패 (LLM/I/O/검증) → `Result`
- 절대 실패 안 함 (pure transform) → exception on bug
- `is_ok` / `is_err` 항상 체크 후 `.value`/`.error`

#### Stage 3 trigger 6 conditions (재인용)

1. Seed modification
2. Ontology evolution
3. Goal reinterpretation
4. Seed drift > 0.3
5. Stage 2 uncertainty > 0.3
6. Lateral thinking adoption

#### TUI state rules

- `app.py` 가 `_state` 소유. Screen 은 read 만
- Reactive dict 직접 mutation + 같은 reference 재할당 금지 → 새 dict 생성
- `is not None` 사용 (truthiness 안 됨, falsy-0 함정)
- DashboardScreenV3 는 `_tree` (SelectableACTree) 사용 — `_ac_tree` (legacy) 아님

### `contributing/testing-guide.md` (211 LOC)

#### Test 구조

```
tests/
  conftest.py
  unit/    # fast, no network
    core/, evaluation/, orchestrator/, tui/, ...
  integration/    # real deps
    mcp/
  e2e/
    test_cli_commands.py
    test_full_workflow.py
    test_session_persistence.py
```

#### 명령

```bash
uv run pytest tests/unit/ -v
uv run pytest tests/unit/ --cov=src/ouroboros --cov-report=term-missing
uv run pytest tests/ --ignore=tests/unit/mcp --ignore=tests/integration/mcp --ignore=tests/e2e
```

#### asyncio mode

`asyncio_mode = "auto"` → 그냥 `async def test_...` 만 쓰면 됨.

#### Mock pattern

```python
def make_mock_adapter(response_content: str) -> MagicMock:
    adapter = MagicMock()
    adapter.complete = AsyncMock(return_value=Result.ok(
        CompletionResponse(content=response_content, model="test-model")
    ))
    return adapter
```

#### Common pitfalls

1. Falsy-0 — `if current_ac_index:` 가 0 일 때 false. `is not None` 사용.
2. Reactive mutation in Textual — dict in-place + 같은 ref 재할당 금지
3. Frozen dataclass — 생성 후 attribute set 못 함

#### Categories

| Category | 위치 | Speed | 의존 | 실행 시기 |
|---|---|---|---|---|
| Unit | `tests/unit/` | <30s | None | every change |
| Integration | `tests/integration/` | medium | network, MCP | before PR |
| E2E | `tests/e2e/` | slow | full system | before release |

#### CI 명령

```bash
uv run ruff check src/ tests/
uv run ruff format --check src/ tests/
uv run mypy src/ouroboros --ignore-missing-imports
uv run pytest tests/unit/ -v --cov=src/ouroboros
```

→ test_run_workflow_verbose 는 known pre-existing failure — block 안 함.

### `contributing/findings-registry.md` (1370 LOC, headers grep)

> docs audit findings 등록부. **현재 frozen** (successor entity-registry.yaml 계획 중이지만 미작성).

#### Schema

- `gap_type` 값 — staleness (3월 15일 rename, decay 금지)
- `sub_qualifier` 값
- `severity` (CONTRIBUTING.md 기반 critical/high/medium/low)

#### Findings (FIND-018 부터 050까지)

| Finding | 영향 | 상태 |
|---|---|---|
| FIND-018 | README → claude-code 워크플로 mismatch (high) | open |
| FIND-019 | architecture → claude-code 워크플로 mismatch (high) | open |
| FIND-044 | `ooo status` CLI 등가 잘못 (medium) | RESOLVED |
| FIND-045 | runtime guides credentials.yaml cross-link 부재 (medium) | RESOLVED |
| FIND-050 | `ooo update` CLI 등가 부족 (low) | RESOLVED |

#### Schema changelog

v1.0 (2026-03-15) 부터 v1.4 (gap_type rename) 까지 4 버전.

### `contributing/issue-quality-policy.md` (29 LOC) — issue 품질 정책

- Bug report = 재현 + 검증 가능
- Feature request = 문제 + 원하는 결과 + "done" 정의
- Exploratory = Discussion/Discord 우선

#### Maintainer guidance

- 최소 missing structure 만 요청 (idea reject 금지)
- Open-ended brainstorming 은 Discussion 으로 redirect
- 너무 vague 한 issue 에서 implementation 시작 금지

## 32.6 API Reference (3)

### `api/README.md` (54 LOC)

API 인덱스. 2 모듈:
- `ouroboros.core` — Result, Seed, errors
- `ouroboros.mcp` — MCPClientAdapter, MCPServerAdapter, MCPError

### `api/core.md` (456 LOC)

#### Result type API

| 메서드 | 설명 |
|---|---|
| `Result.ok(value)` / `Result.err(error)` | 생성 |
| `is_ok` / `is_err` | property |
| `.value` / `.error` | property (raise on wrong side) |
| `unwrap()` | Ok value or ValueError |
| `unwrap_or(default)` | Ok value or default |
| `map(fn)` / `map_err(fn)` | Transform |
| `and_then(fn)` | flatMap / bind |

#### Error hierarchy

```
OuroborosError
├── ProviderError (provider, status_code)
├── ConfigError (config_key, config_file)
├── PersistenceError (operation, table)
└── ValidationError (field, value, safe_value)
```

#### Seed structure (모두 frozen)

```python
class Seed(BaseModel, frozen=True):
    goal: str                                  # IMMUTABLE
    constraints: tuple[str, ...]               # IMMUTABLE
    acceptance_criteria: tuple[str, ...]       # IMMUTABLE
    ontology_schema: OntologySchema
    evaluation_principles: tuple[EvaluationPrinciple, ...]
    exit_conditions: tuple[ExitCondition, ...]
    metadata: SeedMetadata
```

→ "Direction fields IMMUTABLE. Ontology can evolve with consensus."

#### Type aliases

- `EventPayload = dict[str, Any]`
- `CostUnits = int` (token counts)
- `DriftScore = float` (0.0–1.0)

### `api/mcp.md` (891 LOC, headers grep)

가장 큰 API doc. 모든 MCP 모듈 클래스/메서드 reference.

핵심 카테고리:
- Types (TransportType, ContentType, MCPServerConfig, MCPToolDefinition, MCPToolResult, MCPCapabilities, MCPServerInfo)
- Error hierarchy (MCPError → ConnectionError / TimeoutError / ToolError)
- MCP Client (MCPClientAdapter, MCPClientManager)
- MCP Server (MCPServerAdapter)
- Tool Registry (ToolRegistry + global registry)
- Convenience functions (create_mcp_client, create_ouroboros_server)

→ Section 12 의 base.

## 32.7 Examples (2)

### `examples/workflows/research-to-deliverable.md` (82 LOC)

> Research → 구체 artifact (report/plan/spec/comparison matrix).

#### 추천 MCP servers

- `tavily` — 웹/source 발견
- `context7` — 라이브러리/프레임워크 docs

#### Seed shape

```yaml
goal: "Create a sourced implementation brief for adding streaming responses."
task_type: research
constraints:
  - "Use official documentation for API behavior."
  - "Separate facts from recommendations."
  - "Include links or source identifiers for non-obvious claims."
acceptance_criteria:
  - "Brief explains the recommended implementation path."
  - "Brief lists risks, unknowns, and verification steps."
  - "All current API claims are attributed to fetched sources."
ontology_schema:
  name: "implementation_brief"
  fields:
    - {name: "recommendation", field_type: "markdown", ...}
    - {name: "sources", field_type: "list", ...}
    - {name: "risks", field_type: "list", ...}
metadata:
  ambiguity_score: 0.15
```

#### QA quality bar

```
The brief must distinguish sourced facts from recommendations,
cite external claims, and include enough implementation detail
for an engineer to start.
```

### `examples/workflows/design-code-verify.md` (93 LOC)

> UI 작업 — Figma 디자인 source → implementation → 로컬 + 브라우저 QA.

#### 추천 MCP servers

- `figma` — 디자인 인스펙션
- `context7` — 프레임워크 docs
- `opencron` — synthetic 브라우저 QA (deployed/staging URL)

#### Seed shape

```yaml
goal: "Implement the settings panel from the referenced Figma frame."
task_type: code
constraints:
  - "Use the existing component system."
  - "Do not add new styling primitives unless required by the design."
  - "Keep keyboard and responsive behavior intact."
acceptance_criteria:
  - "Implemented UI matches the referenced layout, spacing, and states."
  - "Local component tests or app tests pass."
  - "Browser QA confirms the panel renders at desktop and mobile sizes."
```

#### Execution notes (5 step)

1. **Pull only the relevant Figma frame** — entire file 안 받음 (global tokens 의존 시 외 제외)
2. Use Context7 when framework/library behavior uncertain
3. Implement against existing local components first
4. 로컬 테스트 → 브라우저/synthetic QA
5. No deployed URL → keep QA local, "external verification not applicable"

#### QA evidence pattern

```
Run the settings-panel smoke check.
Return viewport, URL, status, duration, and any visual or interaction failure.
```

## 32.8 33 docs 의 거대 패턴

### 1. "Single source of truth" 정책

- `getting-started.md` 가 onboarding SSOT — 다른 모든 docs 가 link back
- `architecture.md` 가 system design SSOT
- `cli-reference.md` 가 command surface SSOT
- `config-reference.md` 가 config keys SSOT

→ docs 끼리 cross-link 강제. 중복 정보 = drift 위험 (CONTRIBUTING.md 의 Decay Detection 이 catch).

### 2. Runtime별 guides + capability matrix 분리

- `runtime-capability-matrix.md` = 비교 표 (single page)
- `runtime-guides/<backend>.md` = backend별 detail (4 파일)

→ 사용자가 quick comparison vs deep-dive 선택 가능.

### 3. API + Guides + Reference 3-tier

```
api/             # programmer API (Python)
guides/          # workflow patterns
config-reference/cli-reference  # surface 모든 옵션
```

### 4. Examples 에 워크플로 패턴 명시

`examples/workflows/` 는 추천 MCP server 조합 + Seed YAML shape + QA pattern 모두 제공. README → guide → example 의 점진적 detail.

### 5. Contributing 에 docs decay 강조

- `findings-registry.md` 1370 LOC — docs audit 등록부 (현재 frozen, successor 계획)
- `issue-quality-policy.md` 29 LOC — PRD-lite issue 강제
- `testing-guide.md` 의 falsy-0 / Reactive mutation 함정

→ "docs 도 코드와 같은 등급" 가치관.

## 32.9 미발견 영역 (이번 라운드도 못 본 것)

- `architecture.md` 본문 detail (524 LOC) — 1차 라운드에서 일부 cover
- `findings-registry.md` 본문 (1370 LOC) — 헤더만 grep
- `cli-reference.md` line-by-line 모든 플래그 (780 LOC) — 큰 줄기만 헤더 grep
- `config-reference.md` 모든 config 키 detail (670 LOC) — 헤더 + 일부
- `api/mcp.md` 모든 클래스 detail (891 LOC) — 헤더만 grep
- `runtime-guides/codex.md`, `runtime-guides/opencode.md` 본문 detail
- `images/PLACEHOLDER_README.md` — placeholder 만, content 없음

→ 거대 docs 의 모든 line 정독은 별도 라운드 또는 사용자 specific deep-dive 시.
