# 21. Architect Review

> ralplan consensus workflow 의 Architect 단계. 강한 강점 + steelman antithesis + synthesis.

## 강한 강점

### 1. Loop 본질이 코드에 반영됨

Reflect → Wonder → 다음 세대 시드 = 단순 재시도 아님. 진정한 자기 변형.

`reflect.py` 주석 자체 인용:
> *"This is where the Ouroboros eats its tail: the output of evaluation becomes the input for the next generation's seed specification."*

이름값 함. 이름이 마케팅 용어 아님 — 아키텍처가 그대로.

### 2. 이벤트 소싱 전체 아키텍처에 침투

`events/{control,decomposition,evaluation,interview,lineage,ontology}.py` + `persistence/event_store.py` 가 진정한 SSOT.

같은 SQLite (`~/.ouroboros/ouroboros.db`) 를:
- Python TUI 가 0.5s 폴링
- Rust TUI 가 30 ticks (~3s) 폴링
- MCP 서버 가 query_events 도구로 제공
- ralph 가 lineage 재구성에 사용

→ 모든 surface 가 같은 진실 본다.

### 3. Runtime abstraction 진짜 polymorphism

`AgentRuntime` Protocol + `RuntimeHandle` frozen dataclass + `_RUNTIME_HANDLE_BACKEND_ALIASES` 한 곳 매핑.

새 백엔드 (예: GitHub Copilot CLI) 추가 시:
1. `orchestrator/copilot_runtime.py` 작성 (Protocol 만족)
2. `_RUNTIME_HANDLE_BACKEND_ALIASES` 에 alias 추가
3. `runtime_factory.py:resolve_agent_runtime_backend()` 한 줄 추가
4. `config/models.py` Literal 확장
5. `.claude.md` 의 LLM 호출 추상도 별도 (`providers/copilot_adapter.py`)

→ 4 백엔드 동등 비교 가능. 같은 Seed → 다른 엔진 실행.

### 4. 레이트 리밋 처리 깊다

`SharedRateLimitBucket` (`orchestrator/rate_limit.py`) 의 `force_reserve` fallback:

> *"reserve the capacity anyway — otherwise concurrent timeout-fallbacks would all bypass the bucket simultaneously, causing an N× RPM burst to hit the upstream API (worse than starvation per review)."*

→ 단순 timeout-fallback 보다 정교. heartbeat AgentMessage 5s 마다 emit → 사용자 가시.

env override (`OUROBOROS_ANTHROPIC_RPM_CEILING/TPM_CEILING`) — 0 = 무제한.

### 5. 수학적 게이트 명시

마법 숫자 모두 코드 + docs 일치:

| 게이트 | 값 | 출처 |
|---|---|---|
| Ambiguity | ≤ 0.2 | `bigbang/ambiguity.py` |
| Convergence similarity | ≥ 0.95 | `evolution/convergence.py` |
| Drift | > 0.3 | `evaluation/trigger.py` |
| Complexity | 0.4/0.7 | `routing/tiers.py` |
| Coverage | ≥ 0.7 | `evaluation/mechanical.py` |
| Semantic score | ≥ 0.8 | `evaluation/semantic.py` |
| Recursion depth | 5 | `execution/decomposition.py` |
| Compression depth | 3 | `orchestrator/level_context.py` |
| Stagnation thresholds | 3/2/3/3 | `resilience/stagnation.py` |
| Hard cap generations | 30 | `evolution/loop.py` |

추적 가능 + 단일 출처.

### 6. Test pyramid 정상

- Unit: ~220 파일
- Integration: 8
- E2E: 3 (큰 파일 — 431/496/678 LOC)

자체 invariant 검증 테스트 (`test_pipeline_stage1_reuse.py` 가 멀티-AC dedup invariant 검증).

테스트 LOC 가 소스 LOC 와 비슷한 비율 (`opencode-bridge.ts` 22.7K vs `*.test.ts` 22.8K).

### 7. 외부 검증 자료 보존

`Code-Review-Claude.md` (17K) + `Code-Review-Codex.md` (13K) — 다른 모델이 한 코드 리뷰를 레포 내부 보관.

→ 자체 dogfooding. 기억력 보존.

`docs/contributing/findings-registry.md` — 발견 기록 공식 채널.

### 8. Hooks 동작 가벼움

3 hook 모두 짧은 timeout (3-5s):
- LLM 호출 안 함
- 외부 네트워크 1 곳 (PyPI), 5초 timeout, 실패 silent
- 24h 캐시로 불필요 호출 제거
- atomic 캐시 쓰기 (tempfile + Path.replace)

→ 사용자가 hook 때문에 느려졌다 느낄 일 적음.

### 9. Skill 분리 명확

19 SKILL.md = 사용자 진입점 (자연어 + 슬래시).
21 agent .md = 페르소나 prompt.
13 commands stub = 슬래시 라우터.
3 hook = 자동 라우팅.

각자 책임 단일 + 교체 가능.

### 10. 자기 dogfooding

Ouroboros 가 Ouroboros 의 시드 작성 → 자체 진화 가능. 메타 자기참조.

`.ouroboros/seeds/seed_*.yaml` 캐시된 테스트 시드들이 그 증거.

## 트레이드오프 (Steelman antithesis)

### 1. mypy ignore 14개 = 사실상 type check 없음

`disable_error_code`:
```
union-attr, arg-type, return-value, assignment, attr-defined,
misc, call-arg, override, list-item, dict-item, operator,
str-bytes-safe, no-any-return, import-untyped
```

거대한 코드베이스 (259 .py, runner 109K + parallel_executor 144K) 가 사실 타입 안정성 0.

### Claim 의 강한 형태

mypy 가 실질적 noop. ruff B (bugbear) + Pydantic frozen 모델 + 220+ 테스트만 남음. 함수 시그니처 mismatch / Optional 누락 / dict 타입 불일치 모두 잡히지 않음.

### 반론

- Pydantic frozen 모델이 도메인 객체 검증
- ruff B 가 일부 잡음
- 270+ 단위 테스트로 보강
- Result[T, E] 패턴이 expected failure 명시화

→ 완전한 안정성 아님. 트레이드오프 명시적: 빠른 진화 vs 견고한 타입.

### 2. 거대 단일 파일

| 파일 | 크기 |
|---|---|
| `orchestrator/parallel_executor.py` | 144 KB |
| `orchestrator/runner.py` | 109 KB |
| `orchestrator/adapter.py` | 60 KB |
| `cli/commands/init.py` | 29 KB |
| `evolution/test_evolve_step.py` | 1013 LOC |
| `e2e/test_session_persistence.py` | 678 LOC |

### Claim 의 강한 형태

응집은 있지만 부분 변경 시 인지 부하 폭증. 새 개발자 진입 비용 큼. 코드 리뷰 시 한 PR 이 거대 파일 건드리면 review 어려움.

### 반론

- 이벤트 + Result 타입으로 부수효과 표면화
- 카테고리별 함수 그룹 (`# === Section ===` 주석) 항해 보조
- mypy 가 관대해서 시그니처 변경 영향 추적 못 함 → 큰 파일 분리 시 위험성 큼 (catch-22)
- 테스트 커버리지 높음

→ 분리 가치는 있으나 우선순위 낮음.

### 3. 6단 추상층 진입 비용

```
Skill → MCP → Orchestrator → Phase → Provider → CLI
```

새 개발자 6단 모두 이해해야 한 흐름 파악.

### 반론

- `project-context.md` (10K) + `architecture.md` (525 LOC) + `llms-full.txt` (21K) + `getting-started.md` 4중 번들
- `setup` 스킬 6-step wizard 가 사용자 면에서 보호
- Layered dependencies 명시 (위→아래만 import)

→ 학습 곡선 가파르지만 docs 가 균형 맞춤.

### 4. OpenCode/Codex/Hermes 패리티 약함

| 어댑터 | LOC |
|---|---|
| ClaudeAgentAdapter | 1595 |
| CodexCliRuntime | mid |
| OpenCodeRuntime | mid (subprocess 모드 강제) |
| HermesCliRuntime | mid (가장 신규) |

`docs/runtime-capability-matrix.md` 자체가 비대칭 인정.

### Claim 의 강한 형태

Claude 만 고급 기능 (MCP delegation hook, shared rate limit, fork_session, transcript path). 다른 백엔드는 기본기만.

### 반론

- AgentRuntime Protocol 자체가 추상 baseline
- 호환 안 되는 부분만 어댑터 내부 캡슐화
- 4 백엔드 동시 first-class 광고는 미래 약속 (capability matrix 가 정직히 표시)

→ "동등" 보다 "공존".

### 5. Stage 1 AC-agnostic invariant fragile

`pipeline.py:113-122` 의 주석:

> *If future Stage 1 additions become AC-specific (e.g. AC-tagged test filtering or per-AC coverage thresholds), this dedup becomes incorrect and the multi-AC caller must be updated to run Stage 1 per AC again.*

코드 주석으로만 enforce. 미래 변경 시 silent breakage 가능.

### 반론

- `tests/unit/evaluation/test_pipeline_stage1_reuse.py` 가 검증
- 명시적 INVARIANT 표시
- 변경 시 테스트 fail 로 catch 가능

→ 그럼에도 invariant 가 코드 자체로 enforced 안 되는 건 위험.

### 6. 자기참조 위험

Ouroboros 가 Ouroboros 시드 만들 때:
- consensus 트리거 → Ouroboros 자기 평가
- drift 엔진 → 자기 자신 측정
- evolution loop → 자기 자신 진화

무한 루프 보호: 30 세대 cap + ralph max_iterations.

### 반론

- 충분히 큰 안전망
- EventStore 가 모든 세대 보존 → 디버그 가능
- 30 세대 + 5 lateral persona = 무한루프 위험 작음

→ 이론적 위험. 실제로는 ralph max_iterations 가 더 빨리 끊음.

### 7. CHANGELOG 의 v0.30.0 vs Unreleased

분석 시점 main HEAD = v0.30.0 + Unreleased 변경 (opencode bridge multi-fanout 등).

### 반론

PyPI 사용자는 stable v0.30.0 만 받음. 이 분석이 일부 미릴리즈 기능 포함 — 명시 필요.

→ 위험 작음. CHANGELOG 자체가 명시.

### 8. Welcomed but unconfirmed: 비용 절감 85% 주장

`skills/setup/SKILL.md`: "Cost optimization (85% savings on average)".

### 반론

실험 데이터 미공개. PAL Router 의 frugal-first 전략은 합리적이지만 85% 는 마케팅 숫자일 가능성.

→ 분석 결과: 검증 못 함, 의심.

## Synthesis (해소)

핵심 트레이드오프 = **빠른 진화 vs 견고한 타입 안정성**.

Ouroboros 는 명시적으로 빠른 반복 우선:
- mypy 관대 → 런타임 Result + Pydantic + structlog 마스킹으로 보강
- 거대 파일 → 이벤트 소싱이 부수효과 명시화
- 4 런타임 → Protocol + alias 매핑으로 한 곳 라우팅
- AC-agnostic invariant → 테스트 + 명시 주석

이는 일관 디자인 결정 — 디자이너가 의식적으로 선택한 것. "왜 이런가" 답할 때:

> Ouroboros 자체가 self-improving workflow 라서 자기 자신도 빨리 진화해야 함. 엄격한 type 체계는 진화 속도 늦춤. 대신 EventStore + Result 패턴 + 풍부한 테스트로 안정성 확보.

## Architect 합의

**설계 sound**.

다만 점진 개선 가치 있는 항목:

1. `[tool.mypy]` 의 `disable_error_code` 14개 중:
   - `arg-type` 활성화 가치 (호출자 타입 체크)
   - `return-value` 활성화 가치 (return 타입)
   - `assignment` 활성화 가치 (변수 할당)
   - 나머지 11개는 maintenance 부담 큼 — 유지

2. `pipeline.py:113-122` Stage 1 AC-agnostic invariant 를 코드로 강제:
   - `MechanicalConfig.is_ac_agnostic = True` 같은 명시 flag
   - AC-specific 추가 시 flag 반전 → 멀티-AC 경로가 다르게 동작

3. `runner.py` 109K + `parallel_executor.py` 144K 함수 그룹 분리 검토:
   - 응집은 있지만 인지 부하 큼
   - 점진 분리 (한 기능 단위씩) 가능

4. OpenCode `opencode_mode="subprocess"` 하드코딩 → config 화:
   - plugin 모드 실험 용이
   - 현재는 명시 결정 (composition root 분리), 시간 지나면 재고

5. Hermes 어댑터 (가장 신규) 의 e2e 적용 시점 확인.

6. `event_store.py` 의 `.scalars()` vs `.mappings()` 함정 → lint rule 화 검토:
   - `project-context.md` 에 명시 anti-pattern
   - 새 핸들러 추가 시 자동 차단 가치

## 가장 인상적

OpenCode bridge TS plugin 의 fire-and-forget dispatch:
- `session.prompt` 200s+ 블로킹 → `session.promptAsync` ~10ms
- v22/v23 hardening (모든 reject path 로깅, 100KB cap, FNV dedupe, fail()+notify())
- 절대 outer try/catch — opencode runLoop 으로 throw 안 됨

→ "정직한 안정성" — 알려진 silent failure 시나리오 모두 명시 처리.

## 가장 위험

`mypy disable_error_code` 14개 + 144K parallel_executor 단일 파일 + Stage 1 AC-agnostic invariant fragile.

세 위험 모두 "빠른 진화" 트레이드오프의 대가. 디자이너가 의식 선택했지만 시간 지나면 부채로 변환 가능.
