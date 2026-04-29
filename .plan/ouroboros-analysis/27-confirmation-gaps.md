# 27. Confirmation Gaps — 확인 못 한 영역 (정직)

> 분석에서 충분히 read 못 한 부분 명시. 사용자가 specific deep-dive 요청 시 이 표 참조. **이번 ralph round (US-001 ~ US-010) 에서 채워진 gap 은 ✅ 마크.**

## 코드 영역

### 거대 파일 부분 미확인

| 파일 | 크기 | 확인 정도 |
|---|---|---|
| `src/ouroboros/orchestrator/runner.py` | 109 KB | ✅ Section 28 — 2722 LOC 정독 (cancellation, system prompt, 메시지 루프, 이벤트 emit) |
| `src/ouroboros/orchestrator/parallel_executor.py` | 144 KB | ✅ Section 28 — 3479 LOC 정독 (재귀 분해, 레벨 코디네이션, leaf evidence) |
| `src/ouroboros/orchestrator/adapter.py` | 60 KB | 정독 (~1500 LOC) |
| `src/ouroboros/cli/commands/init.py` | 29 KB | ✅ Section 28 + 14 — 855 LOC 정독 (인터뷰 루프, PM 시드 자동 검출) |
| `src/ouroboros/mcp/tools/definitions.py` | 11 KB | ✅ Section 12 — 23 핸들러 catalog 확정 |

### 짧은 stub 본문 미확인

| 디렉토리 | 파일 수 | 본문 |
|---|---|---|
| `commands/` | 13 | ✅ Section 13 — 13 stub 본문 정독 + frontmatter + Read SKILL.md 패턴 확인 |
| `skills/*/SKILL.md` | 19 | ✅ Section 13 — qa/publish/pm/brownfield/welcome/tutorial/evolve/status/unstuck/evaluate/update/help/seed/cancel/resume 본문 정독 |

### 모듈 함수 detail 미확인

| 모듈 | 확인 정도 |
|---|---|
| `evolution/wonder.py` | ✅ Section 29 — WonderEngine 본체 |
| `evolution/reflect.py` | ✅ Section 29 — ReflectEngine + similarity ≥ 0.99 stagnation warning |
| `evolution/regression.py` | ✅ Section 29 — RegressionDetector |
| `verification/verifier.py` | ✅ Section 29 — SpecVerifier T1/T2 + ReDoS guard MAX_PATTERN_LENGTH=200 |
| `secondary/scheduler.py` | ✅ Section 29 — SecondaryLoopScheduler + BatchStatus 4-way |
| `mcp/server/security.py` | ✅ Section 29 — SecurityLayer (Auth/Authz/RateLimit token bucket/Validation) |
| `core/git_workflow.py` | ✅ Section 29 — GitWorkflowConfig + 8 PR-detect regex |
| `core/file_lock.py` | ✅ Section 29 — cross-platform fcntl/msvcrt |
| `core/security.py` | ✅ Section 29 — InputValidator |
| `routing/escalation.py` + `downgrade.py` | ✅ Section 29 — FAILURE_THRESHOLD=2, DOWNGRADE_THRESHOLD=5, SIMILARITY_THRESHOLD=0.80 Jaccard |
| `agents/loader.py` | ✅ Section 13 — 2-tier resolution + LRU cache |

## 테스트 영역

| 영역 | 확인 정도 |
|---|---|
| 270+ 테스트 케이스 본문 | 디렉토리 + LOC count + 일부 invariant test 본체 |
| `tests/conftest.py` | ✅ Section 20 — autouse `close_test_owned_stores` + `_TYPER_FORCE_DISABLE_TERMINAL=1` |
| `tests/integration/conftest.py` | ✅ Section 20 — OpenCodeSubprocessStub + 5 fake class |
| `tests/e2e/conftest.py` | ✅ Section 20 — 22 fixture + WorkflowSimulator |
| `tests/unit/evaluation/test_pipeline_stage1_reuse.py` | ✅ Section 20 — Issue #422 Stage 1 reuse invariant |
| `tests/unit/orchestrator/test_inflight_cancellation.py` | ✅ Section 20 — 913 LOC 본체 분석 |
| `tests/unit/orchestrator/test_runner_cancellation.py` | ✅ Section 20 — 684 LOC 본체 분석 |
| `tests/e2e/test_session_persistence.py` | ✅ Section 20 — 678 LOC EventStore replay invariants |
| 개별 assert | spot-check (각 test 의 핵심 assertion 만) |
| Mock / Fixture detail | 정독 (e2e/integration conftest) |
| E2E mcp_bridge_test_config.yaml | 파일 존재만 확인 |
| Rust crate `cargo test` 결과 | 미실행 |
| Bun 테스트 (`ouroboros-bridge.test.ts`) 결과 | 미실행 |

## 문서 영역

| 문서 | 확인 정도 |
|---|---|
| CHANGELOG 200 줄 이후 | ✅ Section 33 — 0.1.0 ~ 0.14.1 전체 정독 |
| `CONTRIBUTING.md` (30 KB) | ✅ Section 31 — 6 doc coverage table + severity rubric + decay 7 check |
| `Code-Review-Claude.md` (17 KB) | ✅ Section 30 |
| `Code-Review-Codex.md` (13 KB) | ✅ Section 30 |
| `HANDOFF.md` (3.5 KB) | ✅ Section 31 — Phase 6 v0.4.0 stale snapshot |
| `SECURITY.md` (2.6 KB) | ✅ Section 31 — jqyu.lee@gmail.com private + 48h/7d/30d SLA |
| `UNINSTALL.md` (2.4 KB) | ✅ Section 31 — 13 path mapping + `--keep-data` |
| 33 docs 본문 | ✅ Section 32 — getting-started / cli-reference / config-reference / events / 3 api / 10 guides / 5 contributing / 2 runtime-guide / 2 examples 정독 |
| `.github/ISSUE_TEMPLATE/*.yml` | ✅ Section 31 (governance) — bug_report (8 필드), feature_request (8 필드), question (4 필드), config (Discussions + Discord 링크) |
| `README.ko.md` (16.9 KB) | ✅ 정독 — 영어 README 와 동일 내용, 한국어 번역 (Wonder→온톨로지, Double Diamond, 9 사고, 18 패키지) |

## CI / 빌드 영역

| 영역 | 확인 정도 |
|---|---|
| `release.yml` 전체 | ✅ Section 19 — 4 OS matrix (5번째 미정의 — Section 19 도 정정) + macOS ad-hoc codesign + pre-release regex 위험 분석 |
| `dev-publish.yml` | ✅ Section 19 — main branch trigger + tag-skip + opencode plugin asset packaging contract (PR #462 regression guard) |
| `.github/ISSUE_TEMPLATE/config.yml` | ✅ Section 31 (이번 round) |
| Rust release 빌드 step detail | ✅ Section 19 — `cargo build --manifest-path crates/ouroboros-tui/Cargo.toml --release --target ...` |
| Codecov 실 커버리지 % | 미확인 (CI 실행 결과 접근 불가) |

## 외부 검증 미확인

| 항목 | 사유 |
|---|---|
| 실제 PyPI 배포 결과 | git clone 만 — PyPI 호출 안 함 |
| Marketplace 검증 | `claude plugin install` 안 실행 |
| 실 Claude Code/Codex/OpenCode/Hermes 실행 결과 | 런타임 환경 설정 안 됨 |
| 실 인터뷰 실행 + ambiguity 점수 변화 | 실 LLM 호출 안 함 |
| 실 진화 loop 30 세대 실행 | 실 LLM 호출 안 함 |
| 비용 절감 85% 주장 검증 | 실험 데이터 미공개 |

## Dynamic Dispatch 영역

| 메커니즘 | 영향 |
|---|---|
| `MCPToolProvider` 동적 도구 발견 | ✅ Section 12 — 23 도구 catalog 확정 |
| `command_dispatcher.py` 동적 라우팅 | Codex 의 슬래시 → MCP 변환 detail (정독 안 함) |
| `agents/loader.py` 동적 페르소나 로드 | ✅ Section 13 — 2-tier (env override → importlib.resources) + LRU cache |
| FastMCP 서버 어댑터 | `mcp/server/adapter.py` 본체 detail (정독 안 함) |

## 과거 버전 영역

| 영역 | 확인 정도 |
|---|---|
| 0.13.x 이전 CHANGELOG | ✅ Section 33 — 0.1.0/0.1.1/0.2.0/0.3.0 + Plugin Phase 1 Unreleased |
| 0.2.0 / 0.3.0 마일스톤 detail | ✅ Section 33 |
| Removed 기능 (`StateStore`, `StateManager` 등 dead code 제거) detail | ✅ Section 33 — Plugin Phase 1 Unreleased block |

## OpenCode Bridge 영역

`src/ouroboros/opencode/plugin/ouroboros-bridge.ts` (560 LOC):
- ✅ Section 16 (이번 round) — 전체 정독
- ✅ id() / fnv() / truncateUtf8() / build() / parse() / notify() / fail() / buildEnvelope() / dupe() / patch() / dispatch() / Plugin hook 9-step 본체 분석
- ✅ Section 16.27 (정정) — surfaceErr() doesn't exist (it's fail() + notify())

## Skill 본문 영역

19 SKILL 중 본문 정독한 것:

| Skill | 본문 정도 |
|---|---|
| `interview`, `run`, `ralph`, `setup` | 전체 정독 (이전 round) |
| `qa`, `publish`, `pm`, `brownfield`, `welcome`, `tutorial`, `evolve`, `status`, `unstuck`, `evaluate`, `seed`, `update`, `help`, `cancel`, `resume` | ✅ Section 13 (이번 round) — 본문 정독 |

## Examples 영역

| 파일 | 확인 정도 |
|---|---|
| `examples/dummy_seed.yaml` | ✅ HelloWorld minimal seed (1 AC + ambiguity=0.1) |
| `examples/coordinator_test_seed.yaml` | ✅ 4 AC + Coordinator file conflict 테스트 (AC2+AC3 동시 config.py 수정) |
| `examples/parallel_subac_test_seed.yaml` | ✅ 3 independent AC (utils/ string/date/file helpers) — sub-AC decomposition + parallel 테스트 |
| `examples/subac_test_seed.yaml` | ✅ 4 complex AC (Todo CLI) + ambiguity=0.3 + expected_decomposition_depth=2 |
| `examples/test_display.py` | ✅ WorkflowDisplay simulation (100 message + AC_START/AC_COMPLETE marker injection) |
| `examples/task_manager/` | ✅ Typer CLI (cli.py 277 LOC) + Task dataclass + JSON storage (~/.task_manager/tasks.json) |

## 단언적 주장의 검증 필요

| 주장 | 출처 | 검증 강도 |
|---|---|---|
| Cost 절감 85% | `setup/SKILL.md:48` | 마케팅 숫자, 실험 미공개 |
| `ouroboros_query_events` MCP 도구 노출 | ✅ Section 12 — definitions.py 직접 확인 |
| `ouroboros_session_status` MCP 도구 노출 | ✅ Section 12 — 같음 |
| `ouroboros_execute_seed` MCP 도구 노출 | ✅ Section 12 — handler 직접 확인 |
| 30 세대 hard cap | ✅ Section 25 정정 — `MAX_GENERATIONS = 30` 직접 확인 |
| 6 consensus trigger 우선순위 | docs `architecture.md:271` | 강함 |
| 5 페르소나 affinity 표 | ✅ Section 13 + 29 + agents/*.md 직접 확인 |
| RuntimeHandle alias 정규화 | `adapter.py:223-280` 직접 read | 강함 |
| `MAX_DEPTH=2` (NOT 5) | ✅ Section 25 정정 — parallel_executor.py 직접 확인 |
| 23 MCP tool (NOT 21) | ✅ Section 12 정정 — definitions.py 직접 확인 |
| `surfaceErr()` 존재 (틀림) | ✅ Section 16 정정 — `fail()` + `notify()` 임 |

## 이번 round 채워지지 않은 gap (next ralph round 후보)

### 코드

- ✅ `claude_permissions.py` + `codex_permissions.py` — Section 34 추가
- ✅ `mcp/bridge/bridge.py` + `mcp/tools/bridge_mixin.py` — Section 34 추가
- ✅ Rust TUI views (top-level signature + body partial) — Section 34 추가
- ✅ Python TUI widgets/screens 카탈로그 — Section 34 추가
- `mcp/server/adapter.py` 본체 (FastMCP 어댑터)
- `command_dispatcher.py` 동적 라우팅 detail
- `MCPClientManager` 본체 (`mcp/client/manager.py`)
- `MCPBridgeConfig` (`mcp/bridge/config.py`)
- 8 widget body (LOC 만 카탈로그)
- 10 screen body (LOC 만 카탈로그) — 특히 dashboard_v3.py 32.6 KB
- Rust views 의 helper body (시그니처만)
- Orchestrator 21 잔여 file (heartbeat / runtime_scope / opencode_event_normalizer / parallel_executor_models)
- Router (`router/dispatch.py` + `router/command_parser.py`)
- Verification (`verification/extractor.py`)
- 270+ 테스트 케이스 의 개별 함수 본체 (현재는 spot-check + invariant test 4개만)
- `playground/` 의 example model + config

### 문서

- `docs/marketing/` (Section 32 가 metadata 만 capture)
- `docs/screenshots/` 의 capture script
- `docs/videos/` 의 production script

### 외부

- 실 PyPI / Marketplace 배포 검증
- 실 Claude Code / Codex CLI / OpenCode / Hermes 런타임 실행
- Codecov 실 커버리지 %
- Rust crate `cargo test`
- Bun `ouroboros-bridge.test.ts`
- 실 LLM 호출 + 비용 절감 85% 검증
- 실 진화 loop 30 세대 실행

## 권장 사용 방법

이 문서는 **deep-dive 진입 시 위 영역 제외 명시** 용도. 사용자 specific 요청 시:

1. 위 표에서 영역 식별
2. INDEX.md 의 topic / file path navigation 과 cross-reference
3. 추가 read/grep/실행 필요한지 결정

## 다음 deep-dive 추천 (값어치 큰 순)

1. ~~`runner.py` 메시지 루프~~ ✅ Section 28
2. ~~`parallel_executor.py` 재귀 분해~~ ✅ Section 28
3. ~~`opencode-bridge.ts` 440 LOC~~ ✅ Section 16
4. ~~CHANGELOG 0.x 시리즈~~ ✅ Section 33
5. ~~Code-Review-Claude vs Codex~~ ✅ Section 30
6. ~~CONTRIBUTING.md 30 KB~~ ✅ Section 31
7. ~~10 docs/guides~~ ✅ Section 32
8. ~~`tests/e2e/test_session_persistence.py`~~ ✅ Section 20
9. ~~31 미독 docs~~ ✅ Section 32
10. **Rust crate views/** (TUI 시각화 detail) — 다음 round
11. **Marketing / Screenshots / Videos docs** — 다음 round
12. **mcp/server/adapter.py 본체** — 다음 round

## 이번 ralph round 의 deliverable

- ✅ Section 35 — Residual modules (parallel_executor_models / execution_runtime_scope / verification/extractor / MCPClientManager) — Round 3
- ✅ Section 34 — TUI views + permissions + MCP bridge body (Round 2 review gap-finder 결과)
- ✅ Section 28 — runner + parallel_executor + init.py deep-dive
- ✅ Section 29 — small modules (Wonder/Reflect/Regression/Verifier/Scheduler/InputValidator/Escalation/Downgrade)
- ✅ Section 30 — external reviews (Claude vs Codex)
- ✅ Section 31 — governance (SECURITY/HANDOFF/UNINSTALL/CONTRIBUTING)
- ✅ Section 32 — docs deep-dive (33 docs)
- ✅ Section 33 — CHANGELOG history (0.1.0 ~ 0.14.1)
- ✅ Section 13 보강 — agents/loader.py + 13 commands + 21 agent personas
- ✅ Section 12 보강 — 23 MCP tool catalog 정정
- ✅ Section 16 보강 — opencode-bridge.ts 540 LOC body + surfaceErr 정정
- ✅ Section 5 보강 — parallel_executor body + AC outcome 5-way
- ✅ Section 14 보강 — _DefaultStartGroup + PM seed auto-detect
- ✅ Section 19 보강 — release.yml + dev-publish.yml detail
- ✅ Section 20 보강 — conftest 계층 + 4 invariant test 본체 분석
- ✅ Section 25 정정 — MAX_DEPTH 5→2 + 추가 magic numbers
- ✅ Section 27 update — 이번 round 채운 gap 마킹
- ✅ INDEX.md — 33 section navigation hub
