# 10. Persistence — Event Sourcing Layer

## 책임

모든 상태 변화를 immutable event 로 기록. Append-only. 완전 replay. 체크포인트 + rollback.

## 핵심 모듈 (`src/ouroboros/persistence/`)

| 파일 | 역할 |
|---|---|
| `event_store.py` | `EventStore`, `SessionActivitySnapshot`. SQLAlchemy + aiosqlite |
| `schema.py` | 테이블/컬럼 정의 (`events`) |
| `checkpoint.py` | `CheckpointStore`. 5분 주기, 3-level rollback |
| `uow.py` | Unit of Work (events + checkpoint 원자 커밋) |
| `brownfield.py` | 브라운필드 레포 등록 저장소 |
| `migrations/runner.py` | 스키마 마이그레이션 |

## 단일 `events` 테이블

```sql
CREATE TABLE events (
    id TEXT PRIMARY KEY,           -- UUID
    aggregate_type TEXT,            -- "session" | "execution" | "interview" | "lineage" | ...
    aggregate_id TEXT,              -- 도메인 객체 ID
    event_type TEXT,                -- "domain.entity.verb_past_tense"
    payload TEXT,                   -- JSON
    timestamp REAL,                 -- UTC unix
    consensus_id TEXT NULL          -- 연관 consensus ID (optional)
);
```

## 5 Index

```sql
CREATE INDEX idx_aggregate_type ON events(aggregate_type);
CREATE INDEX idx_aggregate_id ON events(aggregate_id);
CREATE INDEX idx_composite ON events(aggregate_type, aggregate_id);
CREATE INDEX idx_event_type ON events(event_type);
CREATE INDEX idx_timestamp ON events(timestamp);
```

## 이벤트 명명 규약

`domain.entity.verb_past_tense`. 예:

- `orchestrator.session.started`
- `execution.ac.completed`
- `evaluation.stage1.failed`
- `interview.round.recorded`
- `lineage.generation.committed`
- `ontology.concept.weight_modified` ← **GOOD** (정확)
- ~~`ontology.concept.updated`~~ ← **BAD** (모호)
- ~~`ConceptAdded`~~ ← **BAD** (no namespace)

`project-context.md` 가 명시 enforce.

## 이벤트 정의 (`src/ouroboros/events/`)

```
events/
├─ base.py          # BaseEvent (frozen Pydantic, UTC datetime, id, type, aggregate_*, data)
├─ control.py       # control plane 이벤트
├─ decomposition.py # AC 분해 이벤트
├─ evaluation.py    # create_pipeline_completed_event 등
├─ interview.py     # 인터뷰 이벤트
├─ lineage.py       # lineage 이벤트
└─ ontology.py      # 온톨로지 진화 이벤트
```

```python
class BaseEvent(BaseModel, frozen=True):
    id: str                                                  # UUID
    type: str                                                # "domain.entity.verb_past"
    timestamp: datetime = datetime.now(UTC)
    aggregate_type: str
    aggregate_id: str
    data: dict[str, Any]
```

## EventStore API

```python
class EventStore:
    async def append(self, event: BaseEvent) -> None: ...
    async def append_batch(self, events: list[BaseEvent]) -> None: ...
    async def get_events(self, aggregate_id: str) -> list[BaseEvent]: ...
    async def query(self, *, aggregate_type=None, event_type=None,
                    since=None, until=None, limit=None) -> AsyncIterator[BaseEvent]: ...
    async def session_snapshot(self, session_id: str) -> SessionActivitySnapshot: ...
```

## Critical Invariant — `.mappings()` vs `.scalars()`

`project-context.md` 의 anti-pattern:

```python
# DO: Convert at boundary using .mappings()
async def get_events(self, aggregate_id: str) -> list[Event]:
    async with self.session() as session:
        result = await session.execute(
            select(events_table).where(events_table.c.aggregate_id == aggregate_id)
        )
        rows = result.mappings().all()                    # RowMapping → Pydantic
        return [Event.model_validate(dict(row)) for row in rows]

# DON'T: .scalars() with Core tables — returns first column only!
result = await session.execute(select(events_table).where(...))
rows = result.scalars().all()                              # WRONG
return [Event.model_validate(row) for row in rows]         # 실패
```

## Unit of Work (`uow.py`)

events + checkpoint 원자 커밋:

```python
async with UnitOfWork(event_store, checkpoint_store) as uow:
    for event in events:
        await uow.append(event)
    if should_checkpoint:
        await uow.checkpoint(snapshot)
    # 컨텍스트 종료 시 commit, 실패 시 rollback
```

## CheckpointStore (`checkpoint.py`)

5 분 주기 자동 체크포인트. 3-level rollback depth 지원.

```python
class Checkpoint:
    id: str
    aggregate_id: str
    event_id: str          # 어느 이벤트까지 반영
    snapshot: dict
    timestamp: datetime

class CheckpointStore:
    async def save(self, aggregate_id: str, snapshot: dict, last_event_id: str) -> Checkpoint: ...
    async def load_latest(self, aggregate_id: str) -> Checkpoint | None: ...
    async def rollback(self, aggregate_id: str, depth: int = 1) -> Checkpoint | None: ...
```

압축: 80 % reduction (`llms-full.txt:560`).

## 성능 특성

| 메트릭 | 값 |
|---|---|
| Append latency | < 10 ms p99 |
| Query latency (1000 events) | < 50 ms |
| Storage per event | ~1 KB |
| Checkpoint compression | 80 % |
| Polling refresh (TUI) | 500 ms |
| Event processing | < 100 ms / update |

## 메모리

| 컴포넌트 | 사용량 |
|---|---|
| Base | 50 MB |
| Per session | 10–100 MB (복잡도 따라) |

## 동시성

- Agent pool: 2–10 parallel agents
- Task queue: priority-based async

## 데이터 경로

```
Application code
      ↓ append
  UnitOfWork (transactional)
      ↓
  EventStore.append → events table
      ↓
  CheckpointStore.save (5분마다 또는 3-level depth)
      ↓
  ~/.ouroboros/ouroboros.db (SQLite)
      ↑ poll 0.5s (Python TUI) / 30 ticks (Rust TUI)
  TUI / CLI / MCP / Rust TUI 가 같은 DB 폴링
```

## Brownfield 영속화

`brownfield.py` — 발견된 git 레포를 별도 테이블에 저장:

| 컬럼 | 의미 |
|---|---|
| id | 레포 ID |
| path | 절대 경로 |
| url | GitHub URL |
| is_default | 인터뷰 시 자동 컨텍스트로 사용 |
| last_scanned | 마지막 스캔 시간 |

## Migration

`migrations/runner.py` — 스키마 변경 시 backwards-compat 마이그레이션.

## Replay 사용 사례

1. **Audit trail** — 누가 언제 무엇 했는지 완전 기록
2. **Session resumption** — 세션 종료 후 재개
3. **Retrospective analysis** — `observability/retrospective.py` 가 자동 분석
4. **TUI/Rust TUI lineage view** — 모든 세대 visualize
5. **Ralph stateless cycle** — lineage_id + EventStore 만으로 진화 재개

## 동기 vs 비동기

Append: async. 쿼리: async iterator. UoW context: async.

CPU 작업 (이벤트 직렬화, 체크포인트 압축): `asyncio.to_thread`.

## 의존 컴포넌트

- aiosqlite>=0.20.0
- sqlalchemy[asyncio]>=2.0.0
- structlog (이벤트 로깅 마스킹)

## EventStore ↔ 다른 모듈 연결

- All phase modules → events 발화
- Phase 4 retrospective.py → 패턴 분석
- TUI / Rust TUI → 폴링 → 시각화
- `evolution/projector.py` → lineage 재구성
- `mcp/tools/query_handlers.py` → `ouroboros_query_events` 도구
