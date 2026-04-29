# 06. Phase 3 — Resilience (Stagnation + Lateral Thinking)

## 책임

정체 패턴 감지 + 측면 사고로 돌파.

## 핵심 모듈 (`src/ouroboros/resilience/`)

| 파일 | 역할 |
|---|---|
| `stagnation.py` | `StagnationPattern(StrEnum)`, `StagnationDetection`, `ExecutionHistory`, `StagnationDetector` + 4 이벤트 |
| `lateral.py` | `ThinkingPersona(StrEnum)`, `PersonaStrategy`, `LateralThinkingResult`, `LateralThinker` + 4 이벤트 |
| `recovery.py` | `RecoveryActionKind`, `RecoveryPlanner`, `RecoverySnapshot`, `create_recovery_applied_event()`, `get_run_recovery_protocol_prompt()` |

## 4 Stagnation 패턴

| 패턴 | 검출 방식 | 임계 |
|---|---|---|
| SPINNING | 동일 출력 SHA-256 해시 반복 | 3 회 |
| OSCILLATION | A → B → A → B 교대 | 2 사이클 |
| NO_DRIFT | drift 점수 변화 epsilon < 0.01 | 3 회 |
| DIMINISHING_RETURNS | 진척률 < 0.01 | 3 회 |

검출은 **stateless** — 상태는 `ExecutionHistory` (phase outputs, error signatures, drift scores) 로 전달.

```python
class StagnationDetector:
    def detect(self, history: ExecutionHistory) -> StagnationDetection: ...
```

이벤트 4종:
- `SpinningDetectedEvent`
- `OscillationDetectedEvent`
- `NoDriftDetectedEvent`
- `DiminishingReturnsDetectedEvent`

`create_stagnation_event(pattern)` 팩토리.

## 5 Lateral Thinking 페르소나

| 페르소나 | 전략 | 잘 맞는 패턴 |
|---|---|---|
| HACKER | 비정통 우회 | SPINNING |
| RESEARCHER | 정보 추가 수집 | NO_DRIFT, DIMINISHING_RETURNS |
| SIMPLIFIER | 복잡도 감소 | DIMINISHING_RETURNS, OSCILLATION |
| ARCHITECT | 근본 재구조 | OSCILLATION, NO_DRIFT |
| CONTRARIAN | 모든 가정 도전 | 모든 패턴 |

각 페르소나는 `agents/{persona}.md` 파일에 상세 정의.

`suggest_persona_for_pattern(pattern: StagnationPattern) -> ThinkingPersona` — 매칭 추천.

페르소나는 **해결책이 아니라 사고 prompt 만 생성**. LLM 이 prompt 받아 새 시도.

```python
class LateralThinker:
    async def think(self, persona, problem_context, current_approach) -> LateralThinkingResult: ...

class LateralThinkingResult:
    persona: ThinkingPersona
    thinking: str
    suggested_actions: list[str]
```

이벤트 4종:
- `LateralThinkingActivatedEvent`
- `LateralThinkingSucceededEvent`
- `LateralThinkingFailedEvent`
- `AllPersonasExhaustedEvent`

## 페르소나 동적 로딩

```python
def _get_persona_strategies() -> dict[ThinkingPersona, PersonaStrategy]: ...   # 정적
def _load_persona_strategies_from_md() -> dict[...]: ...                        # 동적
```

커스텀 페르소나 디렉토리 (`OUROBOROS_AGENTS_DIR` env 또는 `.claude-plugin/agents/`) 가 우선 — 사용자가 자기 페르소나 추가 가능.

## v0.30.0 신기능 — Multi-persona Parallel Fan-out

`ouroboros_lateral_think` MCP 도구가 `persona="all"` 또는 `personas=["hacker", "architect", ...]` 수용.

```
build_lateral_multi_subagent() → _subagents (plural) JSON 계약 →
opencode bridge MAX_FANOUT=10 병렬 promptAsync
```

각 페르소나 독립 LLM 컨텍스트 → anchoring bias 제거.

페이로드별 dedupe (FNV-1a 5초), 한 페르소나 실패해도 다른 거 계속.

## Recovery Protocol

`recovery.py`:

```python
class RecoveryActionKind(StrEnum):
    RETRY = ...
    ESCALATE = ...
    LATERAL = ...
    ROLLBACK = ...
    ABORT = ...

class RecoveryPlanner:
    def plan(self, stagnation: StagnationDetection, history: ExecutionHistory) -> RecoveryAction: ...

class RecoverySnapshot:
    """체크포인트 + 적용된 recovery 기록"""

def create_recovery_applied_event(action, reason) -> BaseEvent: ...
def get_run_recovery_protocol_prompt() -> str: ...   # System prompt 에 삽입
```

## 역사적 컨텍스트

`unstuck/SKILL.md` 가 사용자 진입점. "I'm stuck" / "think sideways" 트리거.

## 의존 컴포넌트

- ExecutionHistory — phase 출력 + drift 스코어 history
- LateralThinker — persona prompt 생성 (LLM 호출)
- StagnationDetector — 패턴 매칭 (CPU only, no LLM)
- RecoveryPlanner — 복구 액션 결정

## Phase 3 ↔ 다른 Phase 연결

- In ← Phase 2 (실행 history)
- In ← Phase 1 (Frontier escalation 후 또 실패하면 stagnation)
- Out → Phase 2 (lateral persona prompt → 새 시도)
- Out → Phase 4 (recovery 후 재평가)

## CONTRARIAN 페르소나 예시

`agents/contrarian.md` 의 5단계 도전:
1. Every assumption 명시화 ("we need a database" → 어쩌면 안 필요)
2. Consider opposite (scale → simplicity)
3. Challenge problem statement (잘못된 문제 푸는 거 아닌지)
4. "What if we did nothing?" 검토
5. Invert obvious approach

명시 안 한 가정 노출 → blind spot 발견 → "wrong" 문제가 사실 더 쉬운 경우 발견.

## Configuration

```yaml
resilience:
  spinning_threshold: 3
  oscillation_threshold: 2
  no_drift_threshold: 3
  no_drift_epsilon: 0.01
  diminishing_returns_threshold: 3
  diminishing_returns_rate: 0.01
  max_lateral_attempts: 5      # 모든 페르소나 한 번씩
```
