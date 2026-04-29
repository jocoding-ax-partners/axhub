# 04. Phase 1 — PAL Router (Progressive Adaptive LLM)

## 책임

작업 복잡도에 따라 가장 저렴한 모델 티어 선택. **"Frugal first, escalate on failure"**.

## 핵심 모듈 (`src/ouroboros/routing/`)

| 파일 | 역할 |
|---|---|
| `complexity.py` | `TaskContext`, `ComplexityScore`, `estimate_complexity()`, `_normalize_token_count()`, `_normalize_tool_dependencies()`, `_normalize_ac_depth()`, `_validate_task_context()` |
| `tiers.py` | `Tier(StrEnum)` (FRUGAL/STANDARD/FRONTIER), `get_tier_config()`, `get_model_for_tier()`, `validate_tier_configuration()` |
| `router.py` | `PALRouter`, `RoutingDecision`, `route_task() -> Result[RoutingDecision, ValidationError]`, `_select_tier_from_score()` |
| `escalation.py` | 2회 연속 실패 시 상위 티어 |
| `downgrade.py` | 5회 연속 성공 시 하위 티어 |

## Tier 표

| Tier | 비용 | Complexity 임계 |
|---|---|---|
| FRUGAL | 1 × | < 0.4 |
| STANDARD | 10 × | < 0.7 |
| FRONTIER | 30 × | ≥ 0.7 또는 critical |

## Complexity 수식

```
complexity = 0.30 × norm_tokens + 0.30 × norm_tools + 0.40 × norm_depth
```

### 정규화 함수

```python
norm_tokens = min(tokens / 4000, 1.0)   # 4000 token 임계
norm_tools  = min(tools / 5, 1.0)        # 5 tool 임계
norm_depth  = min(depth / 5, 1.0)        # depth 5 임계
```

가중치 분포: depth 가 가장 큰 영향 (40%) — 깊은 AC 트리 = 어려움.

## Escalation 흐름

```
Frugal → Standard → Frontier → Stagnation Event (resilience trigger)
```

조건: 2 회 연속 실패 (configurable threshold).

## Downgrade 흐름

```
Frontier → Standard → Frugal
```

조건: 5 회 연속 성공.

이유: 한 번 어려운 작업 만나면 frontier 로 가지만, 시간 지나서 비슷한 작업 쉬워지면 frugal 로 복귀.

## 패턴 학습

Jaccard similarity ≥ 0.80 의 유사 작업 패턴은 과거 성공 티어를 상속. 즉 같은 종류 작업 두 번째부터 frontier 직행 가능.

## 데이터 흐름

```python
class TaskContext:
    tokens: int          # 작업 토큰 추정
    tools: int           # 필요 tool 수
    ac_depth: int        # AC 트리 깊이
    is_critical: bool    # 명시 critical 마크

class ComplexityScore:
    score: float         # 0–1
    breakdown: dict      # 각 정규화 값
```

`PALRouter.route_task(context) -> Result[RoutingDecision, ValidationError]`:
1. `_validate_task_context(context)` — 음수/None 검증
2. `estimate_complexity(context)` — 점수 계산
3. `_select_tier_from_score(score)` — tier 선택
4. 과거 escalation 이력 + Jaccard similarity 매칭 → tier 상속 후보 선택
5. `RoutingDecision(tier, reason, confidence)` return

## 의존 컴포넌트

- Provider Factory (`providers/factory.py`) — 선택된 tier 의 모델 instance 생성
- EventStore — `routing.tier.selected/escalated/downgraded` 이벤트
- Resilience (Phase 3) — Frontier 에서 또 실패 시 stagnation 이벤트 발화

## Phase 1 ↔ 다른 Phase 연결

- In ← Phase 0 (Seed.task_type)
- Out → Phase 2 Double Diamond (선택된 tier 로 LLM 호출)
- Re-trigger ← Phase 4 Evaluation 실패 (escalation)
- Re-trigger ← Phase 3 Resilience (stagnation 후 재시도)

## CLAUDE.md 의 PAL 표기

README 의 "PAL Router — Frugal (1x) → Standard (10x) → Frontier (30x) with auto-escalation on failure, auto-downgrade on success" 라인이 단일 사실 출처.

## 비용 절감 효과

CHANGELOG 주장: "85% savings on average" (`skills/setup/SKILL.md:48`). 검증 못 함 (실험 데이터 미공개).

## Configuration (`config.yaml`)

```yaml
economics:
  default_tier: frugal
  escalation_threshold: 2     # 연속 실패 횟수
  downgrade_threshold: 5      # 연속 성공 횟수
  jaccard_threshold: 0.80     # 패턴 매칭 임계
```
