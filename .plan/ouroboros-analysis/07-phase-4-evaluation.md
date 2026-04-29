# 07. Phase 4 — Evaluation (3-Stage Pipeline)

## 책임

진보적 비용 검증. 싼 것부터 실행, 게이트마다 차단. 비싼 consensus 는 트리거 시만.

## 핵심 모듈 (`src/ouroboros/evaluation/`)

| 파일 | 역할 |
|---|---|
| `pipeline.py` | `EvaluationPipeline`, `PipelineConfig`, `run_evaluation_pipeline()` |
| `mechanical.py` | Stage 1 ($0): `MechanicalConfig`, `CommandResult`, `MechanicalVerifier`, `run_command()`, `parse_coverage_from_output()` |
| `semantic.py` | Stage 2 ($$): `SemanticConfig`, `SemanticEvaluator`, `parse_semantic_response()`, `build_evaluation_prompt()` |
| `consensus.py` | Stage 3 ($$$): `ConsensusConfig`, `ConsensusEvaluator`, `DeliberativeConfig`, `DeliberativeConsensus`, `parse_vote_response()` |
| `trigger.py` | `ConsensusTrigger`, `TriggerConfig`, `TriggerContext` — 6 트리거 평가 |
| `models.py` | `CheckType` (LINT/BUILD/TEST/STATIC/COVERAGE), `EvaluationContext`, `EvaluationResult`, `MechanicalResult` |
| `checklist.py` | 체크리스트 검증 |
| `detector.py` | 언어 자동 감지 (마커 파일) |
| `languages.py` | Python (uv), Rust (Cargo), Go, Zig, Node (npm/pnpm/bun/yarn) 명령 매핑 |
| `verification_artifacts.py` | 검증 산출물 |
| `artifact_collector.py` | Stage 1 산출물 수집 |
| `json_utils.py` | LLM JSON 응답 파싱 |

## 3 Stage 표

| Stage | 비용 | 동작 | 통과 조건 |
|---|---|---|---|
| 1. Mechanical | $0 | LINT, BUILD, TEST, STATIC, COVERAGE (≥70%) | 모든 체크 통과 |
| 2. Semantic | $$ (Standard tier) | AC compliance + score + uncertainty | score ≥ 0.8 + 트리거 없음 |
| 3. Consensus | $$$ (Frontier × 3) | 다수결 또는 deliberative | 2/3 majority |

## Stage 1 — Mechanical

언어 자동 감지 마커:

| 마커 파일 | 언어 |
|---|---|
| `uv.lock` | Python (uv) |
| `pyproject.toml` | Python |
| `Cargo.toml` | Rust |
| `go.mod` | Go |
| `build.zig` | Zig |
| `package-lock.json` | Node (npm) |
| `pnpm-lock.yaml` | Node (pnpm) |
| `bun.lockb` | Node (bun) |
| `yarn.lock` | Node (yarn) |

언어 감지 안 됨 → Stage 1 skip → Stage 2 직행.

`CheckType` enum: `LINT`, `BUILD`, `TEST`, `STATIC`, `COVERAGE`.

Coverage 임계 70%. `parse_coverage_from_output()` 가 도구별 출력 파싱.

### `.ouroboros/mechanical.toml` Override

언어별 명령 override 가능:
```toml
build = "uv run python -m compileall -q src/"
test = "uv run pytest tests/ -x -q"
timeout = 600
```

CI/CD 보안: 실행 가능 명령 allowlist 검증.

### 멀티-AC Stage 1 Reuse (CRITICAL INVARIANT)

`pipeline.py:113-122` 주석:

> **INVARIANT**: Stage 1 checks (lint, build, test, static analysis, coverage) must be **AC-agnostic** — they verify project-wide code quality, not AC-specific behavior. The multi-AC checklist path (`_handle_multi_ac` in `EvaluateHandler`, introduced in #385) relies on this invariant to run Stage 1 exactly once across all ACs and share the result via this parameter.
> 
> If future Stage 1 additions become AC-specific (e.g. AC-tagged test filtering or per-AC coverage thresholds), this dedup becomes incorrect and the multi-AC caller must be updated to run Stage 1 per AC again.

→ **fragile invariant**. 코드 주석으로만 enforce. AC-tagged test filtering 추가 시 dedup 깨짐.

테스트: `tests/unit/evaluation/test_pipeline_stage1_reuse.py`.

## Stage 2 — Semantic

Standard tier, temperature 0.2.

```python
class SemanticEvaluator:
    async def evaluate(self, context) -> Result[tuple[SemanticResult, list[BaseEvent]], ProviderError]: ...

class SemanticResult:
    ac_compliance: bool      # 100% AC 통과
    score: float             # 0–1, weighted evaluation_principles
    uncertainty: float       # 0–1
    drift: float             # 0–1
    rationale: str
```

기준:
- **AC compliance 100%** + score ≥ 0.8 + 트리거 없음 → 승인
- AC compliance 실패 + `trigger_consensus=False` → 즉시 실패
- AC compliance 실패 + `trigger_consensus=True` → Stage 3 으로 second opinion

## Stage 3 — Consensus

### 6 Trigger 우선순위 (`trigger.py`)

```python
class TriggerContext:
    seed_modified: bool                     # 1순위 — Seed immutable 위반
    ontology_changed: bool                  # 2순위
    goal_reinterpreted: bool                # 3순위
    drift_score: float                       # 4순위 — > 0.3
    uncertainty_score: float                 # 5순위 — > 0.3
    lateral_thinking_adopted: bool           # 6순위
    semantic_result: SemanticResult | None
    manual_consensus_request: bool           # context.trigger_consensus
```

`ConsensusTrigger.evaluate(context) -> Result[(TriggerDecision, events), ValidationError]`.

### Simple Mode

3 모델 vote:
- GPT-4o
- Claude Sonnet 4
- Gemini 2.5 Pro

2/3 majority required. `_has_multi_model_credentials()` 가 사전 검증.

```python
class Vote:
    model: str
    verdict: str   # approve | reject
    confidence: float
    reasoning: str
```

### Deliberative Mode

`DeliberativeConsensus` — 역할극:
- **Advocate** — Seed 기준 평가 (`agents/advocate.md`)
- **Devil's Advocate** — ontological challenge (`strategies/devil_advocate.py`)
- **Judge** — 증거 가중 + 최종 결정 (`agents/judge.md`)

```python
def _get_advocate_system_prompt() -> str: ...
def _get_judge_system_prompt() -> str: ...
def _parse_judgment_response(...): ...
```

## Pipeline 흐름 (`pipeline.py`)

```python
class EvaluationPipeline:
    async def evaluate(
        self,
        context: EvaluationContext,
        trigger_context: TriggerContext | None = None,
        *,
        stage1_result: MechanicalResult | None = None,    # 멀티-AC dedup
    ) -> Result[EvaluationResult, ProviderError | ValidationError]:
```

### 1. Stage 1
- `stage1_result` 인자 있으면 재사용 (멀티-AC)
- `_config.stage1_enabled=True` 면 lint/build/test/static/coverage 실행
- 실패 → `_build_result(final_approved=False)` 즉시 return

### 2. Stage 2
- `_semantic.evaluate(context)` 호출
- AC compliance 실패 + `trigger_consensus=False` → 즉시 실패
- AC compliance 실패 + `trigger_consensus=True` → Stage 3 진행 (second opinion)

### 3. Trigger Context 빌드
- `trigger_context` 없음 → 자동 생성
- 있음 + `trigger_consensus=True` → manual_consensus_request 만 머지

### 4. Stage 3 (트리거 시)
- `_trigger.evaluate(trigger_context)` → TriggerDecision
- `should_trigger=True` → `_consensus.evaluate(context, trigger_reason)`
- 결과로 final approval 결정

### 5. 트리거 안 됨 → Stage 2 score 로 결정
- `final_approved = stage2_result.ac_compliance and stage2_result.score >= 0.8`

## EvaluationResult

```python
class EvaluationResult:
    execution_id: str
    stage1_result: MechanicalResult | None
    stage2_result: SemanticResult | None
    stage3_result: ConsensusResult | None
    final_approved: bool
    events: list[BaseEvent]   # all_events including pipeline_completed_event
```

## Failure Reason 우선순위

`_build_result()`:
1. Stage 1 fail → "Stage 1 failed: lint, test"
2. Stage 3 fail (있다면 — Stage 3 가 권위적 verdict 임)
3. Stage 2 fail
4. "Unknown failure"

## 의존 컴포넌트

- LLMAdapter (Stage 2/3)
- 외부 명령 실행 (Stage 1, subprocess)
- EventStore — `evaluation.*` 이벤트
- TriggerContext provider

## Phase 4 ↔ 다른 Phase 연결

- In ← Phase 2 (실행 산출물)
- In ← Phase 0 (Seed.acceptance_criteria, evaluation_principles)
- Out → Phase 5 (성공 시 secondary loop 활성화)
- Re-trigger ← drift, ontology evolution → Stage 3 consensus
- Out → Evolution Loop (실패 → Wonder/Reflect 다음 세대)

## Configuration

```yaml
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
  deliberative_mode: false   # default simple voting
```

## Skill 통합

- `evaluate/SKILL.md` — `ouroboros_evaluate` MCP 도구 호출
- `qa/SKILL.md` — 단일 verdict 별도 (formal evaluate 와 다름)
