# 23. Final Plan ADR (Architecture Decision Record)

## Decision

**6-phase pipeline-axis 분석 (Option A) 채택.**

Layer view 는 Section 2 압축 부록.
파일 단위 누락 zero 보장은 Section 2 디렉토리 토폴로지의 카테고리 매핑으로 커버.

## Context

사용자 요청:
> "https://github.com/Q00/ouroboros.git 해당 플러그인 모든 부분 처음부터 끝까지 싹 다 분석해 하나도 빠짐 없이"

명령:
> `/oh-my-claudecode:ralplan` (consensus 모드)

대상:
- 724 파일
- 259 Python 소스
- 19 skills + 21 agents + 13 commands + 10 scripts
- 33 docs + 270+ tests
- Rust TUI crate + TS opencode bridge
- v0.30.0 (PyPI: ouroboros-ai)

## Drivers

1. **사용자 요구 "처음부터 끝까지 싹 다"** — 의미 단위 누락 zero
2. **Ouroboros 의 본질 (시간축 loop)** — Wonder/Reflect 사이클이 이름값
3. **분석 산출물의 가독성** — 700+ 파일 한 줄 listing 은 잡음
4. **추적 가능성** — file:line 또는 docs:section anchor

## Alternatives Considered

### Option A — Pipeline-axis 분석 (채택)

수직 절단 — 6 phase + 진화 loop 따라.

**장점**:
- Ouroboros 의 본질 (loop) 표현
- 시간축 흐름 명확
- Phase 간 연결 명시

**단점**:
- 의존성 그래프 (어떤 모듈이 어떤 모듈 import) 직접 표시 안 됨

### Option B — Layer-axis 분석 (기각, 부록 흡수)

수평 슬라이스 — Plugin/Core/Persistence/Presentation.

**장점**:
- 레이어 의존성 명확
- 같은 책임 레벨 그룹화

**단점**:
- Phase 시간축 손실
- ouroboros 의 본질 (loop) 흐려짐
- "왜 이런 구조인가" 답 안 됨

→ Section 2 디렉토리 토폴로지에 압축 흡수.

### Option C — File-by-file enumeration (기각, Appendix 안전망)

724 파일 전부 한 줄 코멘트.

**장점**:
- 누락 zero 보장 강
- 완전한 카탈로그

**단점**:
- 신호/잡음 비율 폭락
- 사용자가 architecture 못 읽음
- 분석 가치 낮음 (단순 ls 와 차이 작음)

→ Appendix index 안전망으로 흡수 (확인 못 한 것 명시).

## Why Chosen

ouroboros 의 가장 자기-기술적 단어 = "loop". Pipeline-axis 가 그 자체를 미러링.

또한:
- README 가 "Stop prompting. Start specifying" 에 이어 6-phase 표현
- `architecture.md` (525 LOC) 가 Phase 0–5 구조로 작성됨
- `llms-full.txt` 도 Phase 순서로 정리

→ 작성자가 의도한 mental model 가 pipeline 임. 분석도 거기 align.

## Consequences

### ✅ 장점

1. **6 phase 흐름 명확**
2. **Runtime/persistence/MCP/observability 도 각자 별도 section 으로 노출**
3. **각 section 별 deep-dive 진입점 명시**
4. **위험 5종 surface (mypy, 거대 파일, AC-agnostic invariant, OpenCode parity, 자기참조)**
5. **정직한 미커버리지 마킹**
6. **27 documents — 사용자가 specific topic 만 읽기 쉬움**

### ⚠️ 단점

1. **의존성 그래프 (모듈 간 import) 명시 안 함**
   - 완화: `project-context.md` 의 layered dependencies 섹션 참조
   - 완화: 각 phase section 의 "의존 컴포넌트" + "Phase ↔ 다른 Phase 연결"

2. **모든 하위 함수 시그니처 안 다룸**
   - 완화: 핵심 클래스 + 메소드만
   - 완화: deep-dive 진입점 (Section 24)

3. **27 documents 과 많아 보임**
   - 완화: `00-overview.md` 가 단일 entry
   - 완화: 번호로 phase 순서 보존

## Follow-ups (분석 결과의 권고 — Ouroboros 메인테이너 대상)

### 1. mypy disable_error_code 점진 축소

현재 14개 disabled. 점진 활성화 가치:
- `arg-type` — 호출 시 인자 타입 체크
- `return-value` — return 타입 mismatch
- `assignment` — 변수 할당 mismatch

나머지 11개는 maintenance 부담 큼 — 유지.

### 2. Stage 1 AC-agnostic invariant 코드로 enforce

`evaluation/pipeline.py:113-122` 의 주석-only invariant 강화:

```python
class MechanicalConfig:
    is_ac_agnostic: bool = True
    
    def __post_init__(self):
        if not self.is_ac_agnostic:
            raise ValueError("AC-specific Stage 1 not yet supported")
```

→ AC-specific 추가 시 explicit fail 로 catch.

### 3. 거대 단일 파일 점진 분리

| 파일 | 현재 | 분리 후보 |
|---|---|---|
| `parallel_executor.py` | 144 KB | atomicity 판정 / dependency 그래프 / leaf evidence 추출 / report 렌더링 |
| `runner.py` | 109 KB | system prompt 빌드 / 메시지 루프 / cancellation 관리 / event emission |
| `adapter.py` | 60 KB | 어댑터 본체 / RuntimeHandle / rate limit / message 변환 |

응집은 있지만 인지 부하 큼.

### 4. OpenCode `opencode_mode="subprocess"` 하드코딩 → config 화

`runtime_factory.py:91`:
```python
return OpenCodeRuntime(
    cli_path=cli_path or get_opencode_cli_path(),
    opencode_mode="subprocess",   # 하드코딩
    **runtime_kwargs,
)
```

→ config.yaml `orchestrator.opencode_mode: "subprocess" | "plugin"` 옵션화.

### 5. Hermes 어댑터 e2e 적용 시점 확인

`HermesCliRuntime` 가 가장 신규. e2e (`tests/e2e/`) 에 적용됐는지 검증.

### 6. `event_store.py` 의 `.scalars()` vs `.mappings()` lint rule

`project-context.md` 의 anti-pattern 을 ruff custom rule 로 enforce:

```python
# DO: .mappings()
result = await session.execute(select(events_table).where(...))
rows = result.mappings().all()

# DON'T: .scalars() with Core tables
result = await session.execute(select(events_table).where(...))
rows = result.scalars().all()   # WRONG
```

### 7. CHANGELOG `[Unreleased]` 정리

분석 시점 main HEAD 의 v0.30.0 + 추가 변경 (opencode bridge multi-fanout, FREETEXT_FIELDS) → 다음 release 에 정식 버전 부여.

### 8. 비용 절감 85% 주장 검증

`skills/setup/SKILL.md` 의 "Cost optimization (85% savings on average)" → 실험 데이터 공개 또는 주장 완화.

## 확인 안 한 영역 (정직)

| 영역 | 사유 |
|---|---|
| `runner.py` 의 300 라인 이후 | 109 KB 중 1/4 만 read, 나머지는 grep 함수 시그니처만 |
| `parallel_executor.py` 144 KB | 함수 시그니처만 (`grep`), 상세 분기 미확인 |
| `cli/commands/init.py` 29 KB | 시그니처 미확인 |
| `mcp/tools/definitions.py` 11 KB | 도구 이름은 핸들러 이름 + skill 트리거에서 역추적 (`grep '"name":'` 빈 결과 — 다른 형식 사용 추정) |
| 22 commands stub | 5–11 LOC 짜리 짧은 stub — 제목과 위임만 확인, 본문 미독 |
| `tests/` 270+ 파일 모두 | 디렉토리 + LOC count 만, 개별 케이스 미확인 |
| CHANGELOG 200 줄 이후 | 0.13.x 이전 변경사항 미독 |
| Rust crate `cargo test` 결과 | 미실행 |
| Bun 테스트 결과 (`ouroboros-bridge.test.ts`) | 미실행 |

이 영역 더 깊이 보려면 사용자가 specific 요청 부탁.

## Status

- ralplan consensus workflow: **COMPLETED**
- Planner draft: ✓
- Architect review: ✓ (Section 21)
- Critic evaluation: ✓ APPROVE (Section 22)
- Final ADR: ✓ (이 문서)
- 사용자 final approval: 사용자 결정

## Workflow 단계 매핑 (`/oh-my-claudecode:ralplan`)

| 단계 | 이 분석에서 |
|---|---|
| 0. Company context | skip (`.claude/omc.jsonc` + `~/.config/claude-omc/config.jsonc` 부재) |
| 1. Planner draft + RALPLAN-DR | Overview + 6 phase 절단 결정 + Pre-mortem 3 시나리오 |
| 2. User feedback (--interactive 만) | non-interactive → auto proceed |
| 3. Architect review | Section 21 |
| 4. Critic evaluation | Section 22 |
| 5. Re-review loop | 1 iteration 으로 APPROVE — loop 안 돎 |
| 6. Final output | 27 documents + 이 ADR |

## 다음 단계 (사용자 선택)

| 선택 | 액션 |
|---|---|
| **Approve + 더 깊이** | Section 24 deep-dive 진입점 사용 |
| **Approve + 메인테이너 전달** | Follow-up 8개를 Q00 에게 issue 또는 PR |
| **Re-review** | 특정 section 다시 분석 (`/ralplan` 재실행) |
| **Skip** | 분석 그대로 사용 |

## 결론

**Ouroboros 는 "specification-first AI workflow" 라는 단일 철학에 24개 서브시스템이 일관 정렬된 큰 시스템.**

가장 인상적: `opencode-bridge.ts` 의 fire-and-forget dispatch (200s → 10ms) + ralph stateless cycle (EventStore lineage 재구성).

가장 위험: `mypy disable 14개` + `parallel_executor.py 144K` + `Stage 1 AC-agnostic invariant fragile`.

분석 완료.
