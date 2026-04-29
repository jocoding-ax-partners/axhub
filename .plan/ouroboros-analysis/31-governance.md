# 31. Governance — SECURITY, HANDOFF, UNINSTALL, CONTRIBUTING

> US-004 deep-dive. 4 governance/process 문서 정독 후 핵심 추출. 프로젝트가 어떻게 보안 대응 / 작업 인계 / 깨끗한 설치 해제 / 기여를 운영하는지.

## 31.1 SECURITY.md (2.6 KB)

### 보안 보고 프로세스

| 항목 | 값 |
|---|---|
| 연락처 | **jqyu.lee@gmail.com** (private) |
| Acknowledgment | 48 시간 |
| 초기 평가 | 7 영업일 |
| 수정 release | 30 일 (severity/complexity 따라) |
| Disclosure | reporter 와 협의해서 timing 결정 |

### 4 Severity 정의

| Severity | 정의 |
|---|---|
| **Critical** | RCE / 자격증명 노출 / 보안 통제 완전 우회 |
| **High** | 권한 상승 / 의미 있는 데이터 누출 / 낮은 복잡도의 DoS |
| **Medium** | 제한적 정보 노출 / 설정 약점 / 사용자 상호작용 필요한 exploit |
| **Low** | 보안 영향 minimal |

### 강제 규칙

- **Public GitHub issue 금지** — vulnerability 보고는 무조건 private email
- 책임 disclosure (responsible disclosure)
- Anonymous credit 가능

### Scope (보안 정책 범위)

```
포함:
  - ouroboros-ai Python 패키지
  - 공식 문서

제외:
  - 3rd-party 플러그인
  - runtime backends (Claude Code, Codex CLI 등)
  - downstream integrations
```

### 사용자 알림 (Security Considerations)

1. **Workflow specifications** = 임의 도구 호출 가능 → 신뢰 안 되는 source 의 workflow 검토 필수
2. **API 키 / 자격증명** = env var 또는 secret store. workflow file / VCS 에 절대 commit 금지
3. **Runtime backends** = 자체 보안 모델 — 각 runtime 의 security guidance 별도 참조

→ 즉 Ouroboros 자체는 "shell 같은" workflow 엔진 — sandbox 안 함. 신뢰 경계가 사용자 측에.

## 31.2 HANDOFF.md (3.5 KB)

### 마지막 업데이트

```
Date: 2026-02-03
Session: Ontological Framework Implementation - Phase 6 Complete
Goal: v0.4.0 에 Ontological Framework 추가
```

### v0.4.0 의 3 새 기능 (당시)

1. **AOP 기반 분석 프레임워크** — 횡단 관심사 (cross-cutting concerns) 모듈화로 재사용 가능한 분석 전략
2. **Deliberative Consensus** — Advocate / Devil / Judge 역할 + 2-라운드 토론
3. **Devil's Advocate Strategy** — 온톨로지 질문으로 "근본 해결책인가?" 검증

### 완료된 8 파일 (당시)

| 파일 | 설명 |
|---|---|
| `core/ontology_questions.py` | 온톨로지 질문 정의 |
| `core/ontology_aspect.py` | AOP 분석 프레임워크 (BaseAnalyzer, AnalysisResult) |
| `evaluation/models.py` | VoterRole, FinalVerdict, DeliberationResult |
| `evaluation/consensus.py` | DeliberativeConsensus 클래스 |
| `strategies/devil_advocate.py` | DevilAdvocateStrategy |
| `tests/unit/evaluation/test_consensus.py` | 32 테스트 |
| `tests/unit/core/test_ontology_aspect.py` | 18 테스트 |
| `tests/unit/core/test_ontology_questions.py` | 23 테스트 |

→ **73 테스트 통과** (consensus + ontology 관련).

### Phase 6 의 코드 리뷰 fix

| 이슈 | 위치 | 해결 |
|---|---|---|
| Exception Handling (ProviderError 중복 래핑) | `consensus.py:731-737` | try/except 제거 — Strategy 가 내부 처리 |
| Unused import | `consensus.py:26` | `build_devil_advocate_prompt` 제거 |
| Import ordering | 3 파일 | `ruff --fix` 자동 |
| Missing `__all__` | `ontology_aspect.py` | 이미 존재 (line 443–454) |

### 아키텍처 결정 (인용)

```
1. Devil's Advocate 는 Strategy 객체:
   LLM 호출 대신 DevilAdvocateStrategy.analyze() 사용

2. Strategy 가 에러 처리:
   analyze() 메서드가 LLM 에러를 내부에서 처리 → AnalysisResult.invalid() 반환

3. AnalysisResult.is_valid:
   True = 근본 해결책, False = 증상 치료
```

### 검증 명령

```bash
# 테스트
uv run pytest tests/unit/evaluation/test_consensus.py -v
uv run pytest tests/unit/core/ -v

# 린트
uv run ruff check src/ouroboros/evaluation/ src/ouroboros/core/ src/ouroboros/strategies/
```

### 대기 중 (낮은 우선순위, 미작업)

| 파일 | 설명 |
|---|---|
| `bigbang/ontology.py` | Interview Phase 통합 |
| `bigbang/ambiguity.py` | Ontology Score 가중치 추가 |

### Notes (추론)

- HANDOFF.md 는 단일 timestamp (2026-02-03) — 그 이후 v0.30.0 까지 development 가 많이 진행됨
- 즉 이 HANDOFF 문서는 **stale snapshot** — 더 이상 main 작업 인계 용도로 사용 안 됨 (= 지금은 outdated)
- v0.4.0 이후 변화: Phase 7 ~ Phase 22+ (현재 v0.30.0)

## 31.3 UNINSTALL.md (2.4 KB)

### One-Command 제거

```bash
ouroboros uninstall
```

### 제거 대상 (자동)

```
~/.claude/mcp.json                          # Claude MCP entry
~/.codex/config.toml                         # Codex MCP section
CLAUDE.md `<!-- ooo:START -->` ... `<!-- ooo:END -->`  # CLAUDE.md 통합 블록
~/.codex/rules/ouroboros.md                  # Codex rules
~/.codex/skills/ouroboros/                    # Codex skills
.ouroboros/                                   # 프로젝트 config
~/.ouroboros/                                 # 데이터 디렉토리 (config, credentials, DB, seeds, logs, locks, prefs)
```

### 추가 manual step (uninstall 후)

```bash
uv tool uninstall ouroboros-ai            # 또는: pip uninstall ouroboros-ai
claude plugin uninstall ouroboros         # Claude Code plugin 사용 시
```

### Flags

| Flag | 효과 |
|---|---|
| `-y`, `--yes` | confirmation prompt skip |
| `--dry-run` | 미리보기, 변경 안 함 |
| `--keep-data` | `~/.ouroboros/` 전체 보존 |

### Inside Claude Code (CLI 가 아닌 skill)

```
/ouroboros:setup --uninstall
```

→ MCP 등록 + CLAUDE.md 블록 제거 (interactive). **CLI 플래그 아니라 skill 명령**.

### 13 Path → Creator 매핑

| 경로 | 만든 명령 | 내용 |
|---|---|---|
| `~/.claude/mcp.json` | `ooo setup` / `ouroboros setup` | MCP server entry |
| `~/.codex/config.toml` | `ouroboros setup --runtime codex` | Codex MCP section |
| `~/.codex/rules/ouroboros.md` | `ouroboros setup --runtime codex` | Codex rules |
| `~/.codex/skills/ouroboros/` | `ouroboros setup --runtime codex` | Codex skills |
| `CLAUDE.md` | `ooo setup` | Command reference 블록 |
| `~/.ouroboros/config.yaml` | `ouroboros setup` | Runtime config |
| `~/.ouroboros/credentials.yaml` | `ouroboros setup` | API credentials |
| `~/.ouroboros/ouroboros.db` | First run | EventStore + brownfield registry |
| `~/.ouroboros/seeds/` | `ooo seed` / `ooo interview` | Generated seed specs |
| `~/.ouroboros/data/` | `ooo interview` | Interview state |
| `~/.ouroboros/logs/` | Any run | Log files |
| `~/.ouroboros/locks/` | `ooo run` | Heartbeat locks |
| `~/.ouroboros/prefs.json` | `ooo setup` | Preferences |
| `.ouroboros/` (project) | `ooo evaluate` | Mechanical eval config |

### NOT 제거되는 것

- 프로젝트 source 코드 + git history
- `~/.ouroboros/seeds/` 외부로 복사된 seed YAML
- 패키지 매니저 cache (`uv cache clean ouroboros-ai` 또는 `pip cache purge` 별도 실행)

## 31.4 CONTRIBUTING.md (30 KB) — 핵심 정책

### Quick Setup

```bash
git clone https://github.com/Q00/ouroboros && cd ouroboros
uv sync
uv run ouroboros --version
uv run pytest tests/unit/ -q
```

요구사항: **Python ≥ 3.12 + uv**.

### Workflow 6-step

```
1. Issue 찾기/생성 (Issue Quality Policy 준수)
2. Branch (feat/fix/docs prefix)
3. Code (Result type, Pydantic/frozen dataclass, 테스트 동시 작성)
4. Test (uv run pytest tests/unit/)
5. Lint + Format (ruff check + ruff format + mypy)
6. PR (Closes #N 참조 + 리뷰 응답)
```

### Bug Report 형식 (강제)

```markdown
## Summary [무엇이 깨짐]
## Impact [왜 중요]
## Steps to Reproduce
1. ...
## Expected Behavior
## Actual Behavior
## Acceptance Criteria for Fix
- [ ] ...
## Environment
- Python: 3.12+
- Ouroboros: vN.N.N
- OS: ...
## Logs
[paste error]
```

### Feature Issue 형식 (PRD-lite)

```
1. Problem
2. Why now
3. User / persona
4. Current vs desired behavior
5. Constraints and non-goals
6. Acceptance criteria
```

### Commit Convention

```
<type>(<scope>): <subject>

[optional body]
```

| Type | 사용 시점 |
|---|---|
| `feat` | 새 기능 |
| `fix` | 버그 수정 |
| `docs` | 문서 변경 |
| `chore` | 빌드/툴/dep 업데이트 |
| `refactor` | 동작 안 변경한 리팩토링 |
| `test` | 테스트 변경 |
| `perf` | 성능 개선 |

Common scopes: `cli`, `tui`, `evaluation`, `orchestrator`, `mcp`, `plugin`, `core`.

### Code Style 강제

| 항목 | 값 |
|---|---|
| Line length | 100 |
| Quotes | double |
| Indent | 4 spaces |
| Format tool | ruff |
| Type checker | mypy (Python 3.12 target, missing imports ignored) |
| Lint rules | E/W (pycodestyle), F (pyflakes), I (isort), B (bugbear), C4 (comprehensions), UP (pyupgrade), ARG (unused arguments), SIM (simplify) |
| Python min | 3.12 |

### 4 핵심 패턴 (Key Patterns)

#### 1. Result Type (예외 대신)

```python
def validate_score(score: float) -> Result[float, ValidationError]:
    if 0.0 <= score <= 1.0:
        return Result.ok(score)
    return Result.err(ValidationError(f"Score {score} out of range"))

result = validate_score(0.85)
if result.is_ok:
    process(result.value)
else:
    log_error(result.error.message)
```

#### 2. Frozen Dataclasses

```python
@dataclass(frozen=True, slots=True)
class CheckResult:
    check_type: CheckType
    passed: bool
    message: str
```

#### 3. Event Sourcing

```python
event = create_stage1_completed_event(execution_id="exec_123", ...)
await event_store.append(event)
```

#### 4. Protocol Classes

```python
@runtime_checkable
class ExecutionStrategy(Protocol):
    def get_tools(self) -> list[str]: ...
```

### Documentation Coverage 매핑 (강제 — PR 검토용)

CONTRIBUTING.md 의 핵심 = **각 source file 변경 시 어느 docs 가 동시 업데이트 되어야 하는지** 명시. 6 매핑 테이블:

#### 1. CLI Commands → Doc Mapping

`src/ouroboros/cli/commands/` 의 변경마다 docs 업데이트 강제:

| 소스 파일 | 반드시 업데이트 |
|---|---|
| `init.py` | `docs/cli-reference.md`, `docs/getting-started.md` 인터뷰 워크플로 |
| `run.py` | 같음 — execution 워크플로 |
| `config.py` | `cli-reference.md`, `getting-started.md` 설정 관리 |
| `status.py` | `cli-reference.md` (placeholder 표기) |
| `mcp.py` | `cli-reference.md`, `docs/api/mcp.md` |
| `setup.py` | `cli-reference.md`, `getting-started.md` 셋업 단계 |
| `tui.py` | `cli-reference.md`, `docs/guides/tui-usage.md` |
| `cancel.py` | `cli-reference.md` |

→ status.py, config.py 의 일부 sub-command 가 **placeholder** — docs 도 `[Placeholder — not yet implemented]` 표기 강제.

#### 2. Orchestrator → Doc Mapping

`src/ouroboros/orchestrator/` 의 13 source 파일 → docs 업데이트 강제:

| 소스 | 업데이트 docs |
|---|---|
| `runtime_factory.py` | `runtime-capability-matrix.md`, `runtime-guides/*.md` |
| `adapter.py` (ClaudeAgentAdapter) | `runtime-guides/claude-code.md` permission modes |
| `codex_cli_runtime.py` | `runtime-guides/codex.md` |
| `opencode_runtime.py` | `runtime-capability-matrix.md`, `runtime-guides/opencode.md` |
| `runner.py` | `architecture.md` 라이프사이클, `getting-started.md` 세션 ID 출력 |
| `parallel_executor.py` | `cli-reference.md` `--sequential`, `architecture.md` 병렬 전략 |
| `coordinator.py` (LevelCoordinator) | `architecture.md` 충돌 해결 |
| `session.py` | `cli-reference.md` 세션 ID 형식 |
| `workflow_state.py` | `architecture.md` AC state machine, `tui-usage.md` activity display |
| `dependency_analyzer.py` | `architecture.md` |
| `execution_strategy.py` | `architecture.md` 전략 종류, `seed-authoring.md` |
| `mcp_config.py` / `mcp_tools.py` | `api/mcp.md` |
| `command_dispatcher.py` | `architecture.md` |
| `level_context.py` | `architecture.md` |

**Runtime availability 규칙**: `create_agent_runtime()` 가 `NotImplementedError` 던지는 backend 는 **docs 에 working option 으로 표시 금지**.

#### 3. Configuration → Doc Mapping

10 config 클래스 → docs 매핑 강제:

| 클래스 | config 키 | docs |
|---|---|---|
| `OrchestratorConfig` | `orchestrator.*` | `cli-reference.md`, `README.md` |
| `LLMConfig` | `llm.*` | `architecture.md`, `api/core.md` |
| `EconomicsConfig` / `TierConfig` | `economics.*` | `architecture.md` tier 설명 |
| `ClarificationConfig` | `clarification.*` | `seed-authoring.md` ambiguity threshold |
| `ExecutionConfig` | `execution.*` | `architecture.md` iteration limits |
| `ResilienceConfig` | `resilience.*` | `architecture.md` stagnation/lateral |
| `EvaluationConfig` | `evaluation.*` | `architecture.md` 3-stage |
| `ConsensusConfig` | `consensus.*` | `architecture.md` Stage 3 |
| `DriftConfig` | `drift.*` | `architecture.md` drift |
| `PersistenceConfig` | `persistence.*` | `getting-started.md` DB path |

#### 4. Evaluation Pipeline → Doc Mapping

```
pipeline.py → architecture.md (Stage 1/2/3 설명) + guides/evaluation-pipeline.md
trigger.py → architecture.md (consensus 트리거 임계) + guides/evaluation-pipeline.md
mechanical.py → guides/evaluation-pipeline.md (Stage 1 check list)
models.py → api/core.md (evaluation 결과 타입)
artifact_collector.py → architecture.md
```

#### 5. TUI Source → Doc Mapping

`src/ouroboros/tui/` 의 widget/screen 변경 → `docs/guides/tui-usage.md` 업데이트 강제. **`BINDINGS = [...]` 키바인딩 user-visible** → 무조건 문서화.

#### 6. Skills/Plugin → Doc Mapping

```
skills/codex.md → runtime-guides/codex.md
skills/*.yaml → relevant guide
plugin/skills/executor.py → architecture.md skill 실행 모델
plugin/agents/registry.py → architecture.md, runtime-capability-matrix.md
```

### New Command/Flag 체크리스트

```
[ ] cli-reference.md 업데이트 (type, default, 예시 1+)
[ ] getting-started.md 업데이트 (워크플로 변경 시)
[ ] README.md 검토 (day-1 사용 영향 시)
[ ] placeholder/stub 시: > **Note**: This feature is not yet implemented.
```

### New Runtime Backend 체크리스트

```
[ ] runtime-capability-matrix.md 새 row
[ ] runtime-guides/<runtime>.md 새 파일
[ ] cli-reference.md `--runtime` option 설명에 추가
[ ] getting-started.md prerequisites 업데이트
[ ] [Not yet available] / NotImplementedError 마커 제거 (ship 후)
```

### Documentation Issue Severity Rubric (PR-blocking)

| Severity | Label | 정의 | User 영향 | Merge Policy |
|---|---|---|---|---|
| **Critical** | `docs:critical` | Factually wrong — 문서대로 따라가면 실패 (없는 명령/플래그/경로) | 사용자가 docs 따라하다 fail | **Block merge** |
| **High** | `docs:high` | Misleading — 기술적으로는 있으나 confusing, 누락된 step, 미구현 capability 암시 | 사용자가 wrong state 도달 / 잘못된 기대 | **Block merge** unless issue 등록 |
| **Medium** | `docs:medium` | inconsistent style/term, 모호한 phrasing, 엣지 케이스 누락 | mild 혼란 | Non-blocking, next release 전 fix |
| **Low** | `docs:low` | 사소한 cosmetic 갭 | minor friction | Non-blocking |

### Severity 예시

| 예시 | Severity | 이유 |
|---|---|---|
| `cli-reference.md` 가 없는 `--foo` 플래그 listing | Critical | "no such option" 에러 |
| `getting-started.md` 가 `uv sync` 누락 | Critical | ModuleNotFoundError |
| `opencode` 를 `[Not yet available]` 없이 working 으로 listing | High | NotImplementedError |
| `OUROBOROS_AGENT_RUNTIME` 을 `OUROBOROS_RUNTIME_BACKEND` 로 잘못 표기 | High | env var silently 무효 |
| `OUROBOROS_MAX_PARALLEL` 권장하지만 변수 존재 안 함 | High | 잘못된 기대 |
| `economics:` / `evaluation:` config 섹션 docs 부재 | High | 비-default 사용자가 step 누락 |
| `claude-code` vs `claude_code` 일관성 없음 | Medium | 양쪽 다 valid |
| 헤딩 Title vs Sentence case 불일치 | Medium | 스타일 |
| `drift:` minor 섹션 부재 (defaults 안전) | Medium | advanced tuning 만 영향 |
| `ouroboros tui` bare vs `ouroboros tui monitor` 둘 중 하나 부재 | Low | 다른 형식 documented |

### Documentation Decay Detection (CI 체크)

```bash
# Flag parity
uv run ouroboros init --help
# → cli-reference.md 와 비교

# Config key drift
grep -r "opencode_permission_mode\|runtime_backend\|codex_cli_path" docs/

# opencode 가 [Not yet available] 마커 없이 docs 에 등장하는지
grep -rn "opencode" docs/ | grep -v "Not yet available" | grep -v "semantic-link-rot"

# TUI 키바인딩 documented?
grep -rn "BINDINGS" src/ouroboros/tui/screens/

# Skill YAML 파일이 runtime guide 에 mention 됐는지
ls skills/*.yaml
```

7 검사:
1. **Flag parity** — `--help` vs cli-reference.md
2. **Placeholder honesty** — implementation `# Placeholder` 면 docs 도 `[Placeholder...]`
3. **Runtime parity** — claude/codex/opencode 모두 working 이면 runtime-guides/*.md 존재
4. **Config key drift** — `models.py` 변경 후 docs grep
5. **TUI 키바인딩** — `screens/*.py:BINDINGS` 변경 시 `tui-usage.md` 동기화
6. **Skills registry drift** — 새 `skills/*.yaml` 추가 시 runtime guide mention
7. **Orchestrator new file** — 새 `.py` 추가 시 Mapping 테이블 업데이트

## 31.5 Code of Conduct 핵심

### Pledge

```
참가자 nationality / age / disability / ethnicity / gender / level / appearance / race / religion / orientation 무관 차별 없는 환경
```

### Acceptable

- 존중 + 포용
- 건설적 비판 수용
- 커뮤니티 best 우선
- 공감

### Unacceptable

- 괴롭힘 / 트롤 / 비하
- 개인/정치적 공격
- 공/사 괴롭힘
- private 정보 무단 게시
- 기타 부적절

### Enforcement

- maintainer 가 비-conduct 기여 제거/편집/거부 가능
- 문의: GitHub issue 에 `conduct` 라벨

## 31.6 Getting Help 채널

| 채널 | 용도 |
|---|---|
| GitHub Issues | 버그 / feature 요청 |
| GitHub Discussions | 질문 / 아이디어 |
| Security Reports | SECURITY.md 이메일 |
| Code of Conduct | issue 에 `conduct` 라벨 |

## 31.7 License

```
MIT License — 기여하면 MIT 로 라이선스됨
```

## 31.8 4 docs 의 거버넌스 모델 합성

```
SECURITY (vulnerability) ─→ 48h ack → 7d assess → 30d fix
                            (private email; public issue 금지)

HANDOFF (work-in-flight)  ─→ 단일 timestamp snapshot
                            (현재는 stale — v0.4.0 시점)

UNINSTALL (cleanup)       ─→ 1-command 자동
                            (13 path mapping; data 보존 옵션)

CONTRIBUTING (process)    ─→ Issue → Branch → Code → Test → Lint → PR
                            (Documentation Coverage 강제 — 6 mapping table + severity rubric + decay detection)
```

→ 거버넌스 4 facet:
1. **방어**: SECURITY (외부 위협 대응)
2. **인계**: HANDOFF (in-flight 작업 transfer — 현재 stale)
3. **퇴장**: UNINSTALL (사용자 이탈 시 깨끗한 제거)
4. **참여**: CONTRIBUTING (외부 기여자 onboarding + 품질 보장)

## 31.9 학습된 구조적 패턴

### Documentation Coverage 의 시사점

CONTRIBUTING.md 가 가장 많은 분량 (30 KB > 다른 3 파일 합쳐 10 KB) → **이 프로젝트는 docs drift 를 critical risk 로 본다**. 6 mapping table + severity rubric + decay detection 모두 docs 가 코드와 같은 등급 of importance 라는 제도화.

### HANDOFF.md 가 stale 한 이유

- v0.4.0 (2026-02-03) 작성 후 v0.30.0 까지 update 안 됨
- 새 패러다임: HANDOFF 보다 git log + CHANGELOG.md + Issue tracker 가 진짜 "in-flight" 작업 transfer
- HANDOFF.md 는 도구가 아닌 **artifact of one specific session** — 더 이상 maintenance 안 함

### SECURITY 의 보수적 scope

- ouroboros-ai 패키지 + 공식 docs 만
- runtime backends + 3rd-party 플러그인 모두 제외
- → "우리는 workflow 엔진. 신뢰 경계는 사용자 측" — Ouroboros 는 sandbox 안 함

### UNINSTALL 의 깔끔함

- **13 path 모두 자동 정리** + data 보존 옵션
- 사용자 lock-in 없음 — 언제든 깨끗하게 떠날 수 있음
- 신뢰 신호: 깔끔한 uninstall = 깔끔한 install
