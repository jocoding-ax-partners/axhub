# 02. Directory Topology

총 724 파일. 27 카테고리 매핑.

## Top-level

```
Repo root/
├─ README.md (en, 425 LOC)               # 슬로건, quick-start, philosophy
├─ README.ko.md (한국어 미러)
├─ CLAUDE.md (4K)                         # dev 환경 ooo 라우팅 표
├─ CONTRIBUTING.md (30K)                  # 풀 개발 가이드
├─ Code-Review-Claude.md (17K)            # 외부 리뷰 보존
├─ Code-Review-Codex.md (13K)             # 외부 리뷰 보존
├─ HANDOFF.md (3.5K)                      # 세션 핸드오프
├─ SECURITY.md (2.6K)
├─ UNINSTALL.md (2.4K)
├─ CODE_OF_CONDUCT.md
├─ LICENSE (MIT)
├─ project-context.md (10K)               # AI 에이전트가 코드 작성 전 읽도록 강제
├─ llms.txt (8.6K) + llms-full.txt (21K)  # LLM 컨텍스트 번들
├─ CHANGELOG.md (12.6K)
├─ pyproject.toml (4.1K)
├─ uv.lock (556K)                         # transitive 잠금
├─ .python-version (5B)
├─ .pre-commit-config.yaml
├─ .gitignore
├─ .env.example
└─ .mcp.json (157B)
```

## Plugin 구조

```
.claude-plugin/
├─ plugin.json
├─ marketplace.json
└─ .mcp.json

hooks/
└─ hooks.json                             # CC hook 3종 등록

commands/ (13)                            # CC 슬래시 명령 stub (5–11 LOC)
├─ cancel.md  evaluate.md  evolve.md  help.md
├─ interview.md  pm.md  ralph.md  run.md
├─ seed.md  setup.md  status.md  tutorial.md
└─ unstuck.md  welcome.md

skills/ (19)                              # 워크플로 skill 본체 (88–602 LOC)
├─ brownfield/SKILL.md (204)
├─ cancel/SKILL.md (104)
├─ evaluate/SKILL.md (120)
├─ evolve/SKILL.md (120)
├─ help/SKILL.md (141)
├─ interview/SKILL.md (337)
├─ pm/SKILL.md (129)
├─ publish/SKILL.md (355)
├─ qa/SKILL.md (176)
├─ ralph/SKILL.md (193)
├─ resume/SKILL.md (116)
├─ run/SKILL.md (291)
├─ seed/SKILL.md (149)
├─ setup/SKILL.md (602)                   # 가장 큰 skill, 6-step 위저드
├─ status/SKILL.md (121)
├─ tutorial/SKILL.md (230)
├─ unstuck/SKILL.md (105)
├─ update/SKILL.md (187)
└─ welcome/SKILL.md (261)
                                          # 합 3941 LOC
```

## Python 코어 — `src/ouroboros/` (259 .py)

```
src/ouroboros/
├─ __init__.py                            # _version.py 로드 + main() 진입
├─ __main__.py                            # python -m ouroboros 진입
├─ _version.py (생성)                      # hatch-vcs auto-stamped
├─ sandbox.py                             # ad-hoc sandbox helper
├─ claude_permissions.py                  # Claude Code 권한 프로파일
├─ codex_permissions.py                   # Codex CLI 권한 프로파일
├─ py.typed                                # PEP 561 마커
│
├─ cli/ (28 파일)                          # Typer 진입점
│   ├─ main.py                            # app + 12 서브그룹 등록 (Section 14 참조)
│   ├─ jsonc.py                           # 주석-허용 JSON 파서
│   ├─ opencode_config.py                 # OpenCode plugin 자동 설치
│   ├─ commands/ (12)
│   │   ├─ cancel.py  config.py  detect.py
│   │   ├─ init.py    mcp.py     mcp_doctor.py
│   │   ├─ pm.py      resume.py  run.py
│   │   ├─ setup.py   status.py  tui.py
│   │   └─ uninstall.py
│   └─ formatters/ (6)                    # Rich 출력
│       ├─ panels.py  progress.py  prompting.py
│       ├─ tables.py  workflow_display.py
│       └─ __init__.py (console)
│
├─ core/ (19)                              # 도메인 기반
│   ├─ types.py                           # Result[T,E] PEP 695 generic
│   ├─ errors.py                          # OuroborosError 트리
│   ├─ seed.py                            # 불변 Pydantic Seed
│   ├─ ac_tree.py                          # Acceptance Criteria 트리
│   ├─ context.py                          # Workflow context
│   ├─ directive.py
│   ├─ file_lock.py                        # 다중 프로세스 락
│   ├─ git_workflow.py                     # PR vs main 감지
│   ├─ initial_context.py
│   ├─ json_utils.py
│   ├─ lineage.py                          # 인과 체인
│   ├─ ontology_aspect.py
│   ├─ ontology_questions.py
│   ├─ project_paths.py
│   ├─ retry.py                            # stamina 호환 retry
│   ├─ security.py                          # MAX_* 한계 + 마스킹
│   ├─ text.py
│   ├─ ttl_cache.py
│   ├─ types.py
│   └─ worktree.py                          # TaskWorkspace + heartbeat_lock
│
├─ bigbang/ (10)                           # Phase 0
│   ├─ interview.py                        # InterviewEngine, InterviewState
│   ├─ ambiguity.py                        # AmbiguityScorer, ScoreBreakdown
│   ├─ brownfield.py
│   ├─ explore.py
│   ├─ question_classifier.py
│   ├─ seed_generator.py                   # SeedGenerator + load/save
│   ├─ pm_interview.py
│   ├─ pm_completion.py
│   ├─ pm_document.py
│   └─ pm_seed.py
│
├─ routing/ (5)                            # Phase 1 — PAL Router
│   ├─ router.py                           # PALRouter, RoutingDecision
│   ├─ complexity.py                       # TaskContext, ComplexityScore
│   ├─ tiers.py                            # Tier StrEnum, get_tier_config
│   ├─ escalation.py                       # 2 fail → upgrade
│   └─ downgrade.py                        # 5 success → downgrade
│
├─ execution/ (4)                          # Phase 2 본체
│   ├─ double_diamond.py                   # 4 phase 사이클
│   ├─ decomposition.py
│   ├─ atomicity.py
│   └─ subagent.py
│
├─ resilience/ (3)                         # Phase 3
│   ├─ stagnation.py                       # 4 패턴 + 이벤트
│   ├─ lateral.py                          # 5 페르소나 + 이벤트
│   └─ recovery.py
│
├─ evaluation/ (12)                        # Phase 4
│   ├─ pipeline.py                         # EvaluationPipeline
│   ├─ mechanical.py                       # Stage 1 ($0)
│   ├─ semantic.py                         # Stage 2 ($$)
│   ├─ consensus.py                         # Stage 3 ($$$)
│   ├─ trigger.py                          # 6 트리거 매트릭스
│   ├─ checklist.py
│   ├─ detector.py                         # 언어 자동 감지
│   ├─ languages.py                        # Python/Rust/Go/Zig/Node 명령
│   ├─ models.py                           # CheckType, EvaluationContext
│   ├─ artifact_collector.py
│   ├─ verification_artifacts.py
│   └─ json_utils.py
│
├─ secondary/ (2)                          # Phase 5
│   ├─ todo_registry.py
│   └─ scheduler.py                         # BatchStatus, BatchSummary
│
├─ orchestrator/ (28)                      # 가장 큰 서브패키지
│   ├─ adapter.py                          # ClaudeAgentAdapter (1595 LOC, 60K)
│   ├─ codex_cli_runtime.py                # Codex 어댑터
│   ├─ opencode_runtime.py                 # OpenCode 어댑터
│   ├─ hermes_runtime.py                   # Hermes 어댑터
│   ├─ runtime_factory.py                  # create_agent_runtime
│   ├─ runner.py                            # OrchestratorRunner (109K!)
│   ├─ parallel_executor.py                 # ParallelACExecutor (144K!)
│   ├─ parallel_executor_models.py
│   ├─ coordinator.py                       # LevelCoordinator, FileConflict
│   ├─ dependency_analyzer.py
│   ├─ command_dispatcher.py                # Codex skill-command interception
│   ├─ control_plane.py
│   ├─ events.py                            # orchestrator 이벤트 팩토리
│   ├─ execution_runtime_scope.py
│   ├─ execution_strategy.py                # task_type 별 prompt fragment
│   ├─ heartbeat.py
│   ├─ level_context.py                     # depth 3+ context 압축
│   ├─ mcp_config.py
│   ├─ mcp_tools.py                         # MCPToolProvider
│   ├─ opencode_event_normalizer.py
│   ├─ policy.py                            # PolicyContext, evaluate_capability_policy
│   ├─ rate_limit.py                        # SharedRateLimitBucket
│   ├─ runtime_message_projection.py
│   ├─ session.py                           # SessionRepository, SessionTracker
│   ├─ workflow_state.py                    # AC tracking prompt
│   └─ capabilities.py                       # CapabilityGraph
│
├─ persistence/ (6)                         # Event sourcing
│   ├─ event_store.py                       # SQLAlchemy + aiosqlite
│   ├─ schema.py                            # 단일 events 테이블
│   ├─ checkpoint.py                        # 5분 주기, 3-level rollback
│   ├─ uow.py                                # Unit of Work
│   ├─ brownfield.py                        # 브라운필드 레포 저장소
│   └─ migrations/runner.py
│
├─ events/ (8)                              # 이벤트 정의
│   ├─ base.py                              # BaseEvent (frozen Pydantic, UTC)
│   ├─ control.py
│   ├─ decomposition.py
│   ├─ evaluation.py                        # create_pipeline_completed_event
│   ├─ interview.py
│   ├─ lineage.py
│   └─ ontology.py
│
├─ evolution/ (5)                           # Wonder/Reflect 사이클
│   ├─ loop.py                              # EvolutionaryLoop, StepAction
│   ├─ convergence.py                        # ConvergenceCriteria
│   ├─ projector.py                         # LineageProjector
│   ├─ reflect.py
│   ├─ regression.py
│   └─ wonder.py
│
├─ mcp/ (28)                                # 양방향 MCP 허브
│   ├─ errors.py                            # MCPError 트리
│   ├─ types.py                              # TransportType, MCPServerConfig
│   ├─ job_manager.py                       # async job lifecycle
│   ├─ server/ (4)                           # FastMCP 어댑터
│   ├─ client/ (4)                           # 외부 MCP 풀
│   ├─ bridge/ (3)                           # server-to-server
│   ├─ tools/ (16)                           # 핸들러 + 정의
│   └─ resources/ (1)                        # resource 핸들러
│
├─ providers/ (8)                            # LLM 추상
│   ├─ base.py                              # LLMAdapter Protocol
│   ├─ factory.py
│   ├─ litellm_adapter.py
│   ├─ claude_code_adapter.py
│   ├─ anthropic_adapter.py
│   ├─ codex_cli_adapter.py
│   ├─ codex_cli_stream.py
│   ├─ gemini_cli_adapter.py
│   └─ opencode_adapter.py
│
├─ plugin/                                   # Plugin infra
│   ├─ skills/ (5)                           # registry, executor, keywords, docs
│   ├─ agents/ (3)                           # registry, pool
│   └─ orchestration/ (3)                    # router, scheduler
│
├─ agents/ (21 .md + loader.py)              # 페르소나 prompts
│   ├─ socratic-interviewer.md (3.1K)
│   ├─ ontologist.md (1.4K)
│   ├─ seed-architect.md (2.6K)
│   ├─ evaluator.md (2.2K)
│   ├─ qa-judge.md (1.6K)
│   ├─ contrarian.md (2.2K)
│   ├─ hacker.md (2.0K)
│   ├─ simplifier.md (2.1K)
│   ├─ researcher.md (2.0K)
│   ├─ architect.md (2.0K)
│   ├─ advocate.md (701B)
│   ├─ judge.md (1001B)
│   ├─ breadth-keeper.md (1.7K)
│   ├─ codebase-explorer.md (1.3K)
│   ├─ code-executor.md (376B)
│   ├─ consensus-reviewer.md (715B)
│   ├─ semantic-evaluator.md (1.5K)
│   ├─ ontology-analyst.md (894B)
│   ├─ seed-closer.md (3.2K)
│   ├─ analysis-agent.md (407B)
│   ├─ research-agent.md (500B)
│   └─ loader.py (7.3K)
│
├─ pm/ (3)                                   # PM 트랙
│   ├─ handoff.py
│   └─ renderer.py
│
├─ observability/ (3)
│   ├─ drift.py                             # 3-component 가중 측정
│   ├─ logging.py                            # structlog 설정 + 마스킹
│   └─ retrospective.py
│
├─ tui/ (28)                                 # Textual TUI
│   ├─ app.py                                # TUIState SSOT, 0.5s 폴링
│   ├─ events.py                             # TUIState dataclass
│   ├─ screens/ (10)                          # dashboard v2/v3, execution,
│   │                                          logs, debug, lineage_*,
│   │                                          session_selector, hud_dashboard
│   ├─ widgets/ (8)                           # ac_tree, ac_progress, agent_activity,
│   │                                          cost_tracker, drift_meter, lineage_tree,
│   │                                          parallel_graph, phase_progress
│   └─ components/ (4)                        # agents_panel, event_log,
│                                              progress, token_tracker
│
├─ verification/ (3)                          # extractor, models, verifier
├─ strategies/ (1)                            # devil_advocate
├─ router/ (4)                                # 매직 prefix 라우팅 (이거랑 routing/ 별개)
│   ├─ command_parser.py
│   ├─ dispatch.py
│   ├─ registry.py
│   └─ types.py
├─ codex/ (2)                                  # cli_policy + artifacts
├─ hermes/ (1)                                  # artifacts
├─ skills/ (1)                                  # artifacts (skill 출력 정규화)
├─ config/ (3)                                  # loader + models
└─ opencode/plugin/                              # TS plugin
    ├─ ouroboros-bridge.ts (560 LOC, 22.7K)
    ├─ ouroboros-bridge.test.ts (22.8K)
    ├─ opencode-plugin.d.ts (732B)
    ├─ package.json
    ├─ tsconfig.json
    └─ __init__.py (Python 패키지 마커)
```

## Rust crate — `crates/ouroboros-tui/`

```
crates/ouroboros-tui/
├─ Cargo.toml                                # superlighttui 0.7.1, rusqlite 0.33
├─ Cargo.lock
├─ README.md
├─ .gitignore
└─ src/
    ├─ main.rs                                # Rose Pine 테마, 키 핸들링
    ├─ db.rs                                   # rusqlite 직접 폴링
    ├─ mock.rs                                 # 데모 모드
    ├─ state.rs                                # AppState, SessionInfo
    └─ views/
        ├─ mod.rs
        ├─ dashboard.rs
        ├─ execution.rs
        ├─ lineage.rs
        ├─ logs.rs
        └─ session_selector.rs
```

## Scripts — `scripts/` (10)

```
scripts/
├─ install.sh (339 LOC)                       # uv → pipx → pip 자동
├─ session-start.py (38)                      # SessionStart hook
├─ keyword-detector.py (238)                  # UserPromptSubmit hook
├─ drift-monitor.py (64)                      # PostToolUse hook
├─ version-check.py (207)                     # 24h PyPI 캐시
├─ ralph.py (291)                             # MCP stdio 단발 클라이언트
├─ ralph.sh (265)                              # 외부 루프 wrapper
├─ ralph-rewind.py (202)                       # 세대 rollback
├─ sync-plugin-version.py (180)                # plugin.json 버전 sync
└─ mcp-serve.sh (13)                            # MCP 서버 launcher
```

## Docs — `docs/` (35)

```
docs/
├─ README.md  architecture.md (525 LOC)  cli-reference.md
├─ config-reference.md  events.md  getting-started.md
├─ platform-support.md  runtime-capability-matrix.md
├─ images/ (ouroboros.png + PLACEHOLDER_README)
├─ api/ (core.md  mcp.md  README.md)
├─ examples/
│   ├─ mcp-config.yaml
│   └─ workflows/ (research-to-deliverable, design-code-verify)
├─ guides/
│   ├─ evaluation-pipeline.md  evolution-loop.md  mcp-best-practices.md
│   ├─ mcp-bridge.md  ooo-skill-dispatch-router.md
│   ├─ opencode-subagent-bridge.md  qa-backends.md
│   ├─ seed-authoring.md  tui-usage.md
│   └─ issue-176-subagent-mcp-inheritance.md
├─ runtime-guides/ (claude-code, codex, hermes, opencode)
└─ contributing/
    ├─ architecture-overview.md  findings-registry.md
    ├─ issue-quality-policy.md   key-patterns.md
    └─ testing-guide.md
```

## Tests — `tests/` (270+ 파일, 7797+ LOC)

```
tests/
├─ conftest.py
├─ test-execution-plan.md
├─ unit/ (~220 .py)                            # src/ 미러 구조
│   ├─ agents/, bigbang/, cli/, codex/, config/
│   ├─ core/, evaluation/, events/, evolution/
│   ├─ execution/, hermes/, mcp/, observability/
│   ├─ orchestrator/, persistence/, plugin/
│   ├─ pm/, providers/, resilience/, router/
│   ├─ routing/, scripts/, secondary/, skills/
│   ├─ tui/
│   └─ top-level (16 파일): convergence(636), evolve_step(1013!),
│      graceful_shutdown(495), projector_rewind(471), verification(454),
│      dashboard(323), ralph_parser(184), regression(132), 등
├─ integration/ (8)
│   ├─ conftest.py (179 LOC)
│   ├─ mcp/ (5)                                # bridge_server_to_server, client, server
│   ├─ plugin/test_orchestration.py
│   ├─ test_cancel_subprocess_termination.py
│   ├─ test_codex_cli_passthrough_smoke.py
│   ├─ test_codex_skill_smoke.py
│   ├─ test_codex_skill_fallback.py
│   └─ test_entry_point.py
├─ e2e/ (3 큰 파일)
│   ├─ conftest.py (534)
│   ├─ test_cli_commands.py (431)
│   ├─ test_full_workflow.py (496)
│   ├─ test_session_persistence.py (678)
│   └─ mcp_bridge_test_config.yaml
└─ fixtures/
    ├─ router/skills/frontmatter-body/run/SKILL.md
    └─ test_atomicity_seed.yml
```

## Examples — `examples/` (8)

```
examples/
├─ task_manager/                               # Python 예제
│   ├─ models.py  cli.py  storage.py
│   ├─ __init__.py  __main__.py  README.md
├─ coordinator_test_seed.yaml
├─ parallel_subac_test_seed.yaml
├─ subac_test_seed.yaml
├─ dummy_seed.yaml
└─ test_display.py
```

## GitHub — `.github/`

```
.github/
├─ workflows/
│   ├─ test.yml                                # 3.12/3.13/3.14 매트릭스 + Codecov
│   ├─ lint.yml                                # ruff + mypy
│   ├─ release.yml                             # PyPI + 5 cross-arch Rust TUI
│   └─ dev-publish.yml                          # alpha/beta/rc
└─ ISSUE_TEMPLATE/
    ├─ bug_report.yml
    ├─ feature_request.yml
    ├─ question.yml
    └─ config.yml
```

## Shipped 데모 — `.ouroboros/`

```
.ouroboros/
├─ mechanical.toml                             # Stage 1 override 예시
└─ seeds/
    ├─ seed_78c8e6e41813.yaml                  # 캐시된 테스트 시드
    └─ seed_73827177a2a3.yaml
```
