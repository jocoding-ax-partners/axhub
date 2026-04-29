# 09. Evolutionary Loop (Wonder / Reflect Cycle)

## Ouroboros 의 본질

이름 그대로 — 자기 꼬리를 먹는 뱀. 평가 결과가 다음 세대 시드의 입력. **재시도 ≠ 진화**. 핵심 인용 (`reflect.py`):

> *"This is where the Ouroboros eats its tail: the output of evaluation becomes the input for the next generation's seed specification."*

## 흐름

```
Gen 1: Interview → Seed(O₁) → Execute → Evaluate
Gen 2: Wonder → Reflect → Seed(O₂) → Execute → Evaluate
Gen 3: Wonder → Reflect → Seed(O₃) → Execute → Evaluate
...
정지: 온톨로지 수렴 (similarity ≥ 0.95) 또는 hard cap 30 세대
```

## 핵심 모듈 (`src/ouroboros/evolution/`)

| 파일 | 역할 |
|---|---|
| `loop.py` | `EvolutionaryLoop`, `EvolutionaryLoopConfig`, `GenerationResult`, `EvolutionaryResult`, `StepAction(StrEnum)` (CONTINUE/CONVERGED/STAGNATED/FAILED), `StepResult` |
| `convergence.py` | `ConvergenceSignal`, `ConvergenceCriteria` |
| `projector.py` | `LineageProjector` (이벤트 → lineage 재구성) |
| `reflect.py` | Reflect 단계 (이번 세대 분석) |
| `wonder.py` | Wonder 단계 ("아직 모르는 것?") |
| `regression.py` | 세대 간 회귀 감지 |

## 수렴 수식

```
Similarity = 0.5 × name_overlap + 0.3 × type_match + 0.2 × exact_match
```

| 컴포넌트 | 가중치 | 측정 |
|---|---|---|
| Name overlap | 50 % | 같은 필드명 존재 여부 |
| Type match | 30 % | 공유 필드의 같은 타입 |
| Exact match | 20 % | name + type + description 모두 동일 |

## 게이트 + 안전선

| 신호 | 조건 | 의미 |
|---|---|---|
| **Convergence** | similarity ≥ 0.95 | 온톨로지 안정 — 진화 정지 |
| **Stagnation** | 3 세대 연속 ≥ 0.95 | 추가 진화 의미 없음 |
| **Oscillation** | Gen N ≈ Gen N-2 (period-2) | 두 디자인 사이 핑퐁 |
| **Repetitive feedback** | 3 세대 70% 질문 중복 | Wonder 가 같은 질문 반복 |
| **Hard cap** | 30 세대 도달 | 안전 밸브 |

## 예시

```
Gen 1: {Task, Priority, Status}
Gen 2: {Task, Priority, Status, DueDate}     → similarity 0.78 → CONTINUE
Gen 3: {Task, Priority, Status, DueDate}     → similarity 1.00 → CONVERGED
                                                  ↓
                                              Loop 정지
```

## EvolutionaryLoop API

```python
class EvolutionaryLoopConfig:
    max_generations: int = 30
    convergence_threshold: float = 0.95
    stagnation_window: int = 3

class GenerationResult:
    generation: int
    seed: Seed
    execution_result: OrchestratorResult
    evaluation_result: EvaluationResult
    convergence_signal: ConvergenceSignal | None

class EvolutionaryResult:
    generations: tuple[GenerationResult, ...]
    final_action: StepAction
    convergence_reached: bool
    final_lineage_id: str

class StepAction(StrEnum):
    CONTINUE = "continue"
    CONVERGED = "converged"
    STAGNATED = "stagnated"
    FAILED = "failed"

class EvolutionaryLoop:
    async def evolve_step(self, lineage_id, seed_content=None, *, execute=True, parallel=True, skip_qa=False) -> StepResult: ...
    async def evolve_until_convergence(...) -> EvolutionaryResult: ...
```

## Lineage 재구성 (Stateless 핵심)

`LineageProjector` 가 EventStore 의 모든 `lineage.*` 이벤트 → 시간순 재구성.

각 evolve_step 호출 자체가 stateless — 입력 (lineage_id) + EventStore 만 있으면 어디서든 재개.

→ 머신 재부팅, 다른 세션, ralph 모드 모두 transparent.

## Ralph 통합 (`scripts/ralph.py` + `skills/ralph/SKILL.md`)

### Skill 모드

```python
while iteration < max_iterations:
    job = await start_evolve_step(lineage_id, seed_content, execute=True)
    job_id = job.meta["job_id"]
    cursor = job.meta["cursor"]
    
    while not terminal:
        wait = await job_wait(job_id, cursor, timeout_seconds=120)
        cursor = wait.meta["cursor"]
        terminal = wait.meta["status"] in ("completed", "failed", "cancelled")
        # 레벨 단위 진척 보고
    
    result = await job_result(job_id)
    qa_verdict = parse_qa(result)
    
    if qa_verdict == "pass":
        break
    iteration += 1
```

EventStore 가 lineage 재구성 → "stop" 후 "continue" 가능 → `ouroboros_query_events(aggregate_id=<lineage_id>)`.

### Script 모드 (`scripts/ralph.py` 291 LOC)

MCP stdio 단일 호출 클라이언트.

```python
async def connect_and_run(args) -> dict[str, Any]:
    server_params = StdioServerParameters(command=args.server_command, args=args.server_args)
    async with stdio_client(server_params) as (r, w):
        async with ClientSession(r, w) as session:
            await session.initialize()
            return await _call_evolve(session, args)
```

정규식으로 markdown 파싱:
- `_GENERATION_RE` — `## Generation (\d+)`
- `_ACTION_RE` — `**Action**: (\w+)`
- `_SIMILARITY_RE` — `**Convergence similarity**: ([\d.]+)%`
- `_NEXT_GEN_RE` — `**Next generation**: (\d+)`
- `_LINEAGE_RE` — `**Lineage**: (\S+)`
- `_QA_SECTION_RE` — `### QA Verdict\s*\n([\s\S]*)`

Stagnation 감지 시 `ouroboros_lateral_think({persona: "contrarian"})` 호출 + retry (max 2).

Exit codes:
- 0 = 성공
- 1 = MCP 연결 실패
- 2 = 인자 오류
- 3 = 도구 레벨 에러

### `scripts/ralph.sh` (265 LOC)

외부 루프 wrapper — `ralph.py` 단발 호출 반복.

### `scripts/ralph-rewind.py` (202 LOC)

세대 rollback — 특정 generation 으로 복귀.

## 의존 컴포넌트

- EventStore (lineage 재구성)
- SeedGenerator (다음 세대 시드)
- EvaluationPipeline (이번 세대 평가)
- OntologySchema (수렴 측정)

## Wonder / Reflect 페르소나 (`agents/`)

명시 페르소나 파일 없음 — `evolution/wonder.py` 내장 prompt.

Wonder 단계 질문: *"What do we still not know?"* — 다음 세대 시드 위한 미해결 ambiguity 식별.

Reflect 단계: 이번 세대 결과 요약 + 학습 추출.

## CLAUDE.md 슬로건

> *Convergence is reached when ontology similarity ≥ 0.95 — when the system has questioned itself into clarity.*
> 
> *Two mathematical gates, one philosophy: do not build until you are clear (Ambiguity ≤ 0.2), do not stop evolving until you are stable (Similarity ≥ 0.95).*

## 한계 / 위험

- **Self-referential 위험**: Ouroboros 가 Ouroboros 의 시드 만들면 → consensus + drift 엔진이 자기 자신 평가. 무한루프 보호 = 30 세대 cap + ralph max_iterations.
- **Wonder 가 같은 질문 반복** 시 repetitive_feedback 감지 — Contrarian 페르소나 자동 발화.
