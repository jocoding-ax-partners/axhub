# 08. Phase 5 — Secondary Loop

## 책임

primary 목표 (모든 AC 통과) 달성 후 누적된 비-블로킹 TODO 들을 배치 처리. focus 유지를 위해 deferred 처리.

## 핵심 모듈 (`src/ouroboros/secondary/`)

| 파일 | 역할 |
|---|---|
| `todo_registry.py` | `TodoRegistry`, 비동기 TODO 등록 |
| `scheduler.py` | `BatchScheduler`, `BatchStatus`, `BatchSummary`, 우선순위 처리 |

## TODO 등록

실행 중 LLM 또는 핸들러가 발견한 개선사항을 primary flow 방해 없이 등록:

```python
class TodoItem:
    description: str
    context: str           # execution_id
    priority: Priority     # HIGH | MEDIUM | LOW
    status: Status         # PENDING | DONE | FAILED | SKIPPED
    discovered_at: datetime
```

`TodoRegistry.register(item)` — async, fire-and-forget.

## 우선순위

| Priority | 처리 순서 | 의미 |
|---|---|---|
| HIGH | 1순위 | critical 개선, 먼저 처리 |
| MEDIUM | 2순위 | 표준 개선, 적당한 영향 |
| LOW | 3순위 | nice-to-have, 낮은 긴급도 |

## Batch 처리

primary 목표 완료 후만 활성:

```python
class BatchScheduler:
    async def run_batch(self, *, skip: bool = False) -> BatchSummary: ...

class BatchStatus(StrEnum):
    COMPLETED = "completed"        # 모두 처리 (일부 실패 가능)
    PARTIAL = "partial"            # 타임아웃 등 조기 중단
    SKIPPED = "skipped"            # 사용자가 skip
    NO_TODOS = "no_todos"          # 처리할 TODO 없음

class BatchSummary:
    status: BatchStatus
    total: int
    success_count: int
    failure_count: int
    skipped_count: int
```

## 비-블로킹 실패

한 TODO 실패해도 나머지 진행. 실패 기록 → Summary.

## 사용자 skip

`--skip-secondary` CLI 플래그 또는 MCP 인자.

## 의존 컴포넌트

- AC Tree (primary 완료 검사)
- EventStore — `secondary.todo.*` 이벤트
- ProviderFactory — 각 TODO 실행 시 LLM

## Phase 5 ↔ 다른 Phase 연결

- In ← Phase 2/4 (실행 중 발견 TODO)
- Activator ← Phase 4 (모든 AC 통과 = primary 완료)
- Out → 새 PR/커밋 또는 다음 evolution 세대 입력

## Configuration

```yaml
secondary:
  enabled: true
  skip_on_user_request: true
  max_batch_duration_seconds: 1800   # 30분 안전선
  priorities_to_process: [HIGH, MEDIUM]   # LOW 자동 skip 옵션
```

## 사용 빈도

명시 안 됨 — 실험적 phase. README/getting-started 에서 강조 적음. `evolve.md` 흐름의 마지막 단계로 자연스럽게 활성.
