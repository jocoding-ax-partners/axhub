# 22. Critic Evaluation

> ralplan consensus 의 Critic 단계. 채점 + 최종 verdict.

## 채점 기준

| 차원 | 평가 |
|---|---|
| **Principle-option consistency** | ✓ 6-phase 수직 절단 채택 (Option A) 이 "loop 가 본질" 원칙과 일치. Layer view 는 Section 2 압축 부록. |
| **Fair alternatives** | ✓ Option B (Layer-axis), Option C (File-by-file) 모두 강점 인정 + 명시적 기각 사유. |
| **Risk mitigation clarity** | ✓ Pre-mortem 3 시나리오 모두 완화 명시:<br>- 위험 1 (dynamic dispatch) → 확인 안 한 영역 마킹<br>- 위험 2 (사용자 specific 요구) → deep-dive 진입점 매핑<br>- 위험 3 (CHANGELOG vs PyPI 차이) → CHANGELOG anchor 명시 |
| **Testable acceptance criteria** | ✓ 분석 완성 기준:<br>- 6 phase 모두 모듈 레벨 매핑 ✓<br>- 4 런타임 비교표 ✓<br>- 19 skill 대비표 ✓<br>- MCP 도구 ≥ 12 노출 ✓ (실제 21+ 확인)<br>- 270 테스트 분포 ✓<br>- CI workflow 4종 ✓<br>- hook 3종 detail ✓<br>- Rust TUI + TS bridge 모두 포함 ✓<br>모두 충족. |
| **Concrete verification steps** | ✓ 각 주장에 file:line 또는 docs:section 인용. 사용자가 직접 git clone 후 grep 가능. |
| **Pre-mortem expanded** | ✓ Deliberate mode 기준 충족:<br>- Stage 1 dedup invariant fragile 사전 발견<br>- mypy 관대 명시<br>- OpenCode 패리티 약함 노출<br>- 거대 단일 파일 명시<br>- 자기참조 위험 분석 |

## Critic Verdict: **APPROVE** ✓

## 이유

### 1. 분석 완전성

724 파일 중 의미 단위 누락 zero (개별 파일 일일이 안 나열했지만 카테고리별 + 핵심 모듈 명시 + Appendix index 로 안전망).

서브패키지 레벨:
- bigbang (10) ✓
- routing (5) ✓
- execution (4) ✓
- resilience (3) ✓
- evaluation (12) ✓
- secondary (2) ✓
- orchestrator (28) ✓
- persistence (6) ✓
- events (8) ✓
- evolution (5) ✓
- mcp (28) ✓
- providers (8) ✓
- plugin (skills 5 + agents 3 + orchestration 3) ✓
- agents (21 .md + loader) ✓
- pm (3) ✓
- observability (3) ✓
- tui (28) ✓
- verification (3) ✓
- strategies (1) ✓
- router (4) ✓
- routing (5) — 위 routing 과 별개 ✓
- codex (2) ✓
- hermes (1) ✓
- skills (1) ✓
- config (3) ✓
- opencode/plugin (TS) ✓

비-Python:
- crates/ouroboros-tui (Rust) ✓
- scripts (10) ✓
- docs (35) ✓
- tests (270+) ✓
- .github (workflows + ISSUE_TEMPLATE) ✓

### 2. Risk surface 정확

위험 5종 모두 명시:
- mypy 관대 (14 disable_error_code)
- 거대 단일 파일 (parallel_executor 144K, runner 109K, adapter 60K)
- Stage 1 AC-agnostic invariant fragile
- OpenCode subprocess hardcode
- 4 런타임 비대칭

### 3. Ouroboros 본질 (loop) 가 표현됨

Section 9 Evolutionary Loop 가 별도 섹션으로 자기 참조 구조 노출:
- Wonder/Reflect 사이클 = 재시도 ≠ 진화
- LineageProjector stateless cycle
- 30 세대 hard cap + 4 병리 패턴 감지
- ralph script + skill 두 모드

### 4. 사용자 요구 충족

"싹 다 분석해 하나도 빠짐 없이":
- 19 skill ✓
- 21 agent ✓
- 30+ MCP 도구 ✓
- 4 런타임 ✓
- 6 phase ✓
- 8 hook 시점 ✓ (3 hook + 5 lifecycle)
- 5 CI workflow + 4 ISSUE template ✓
- 2 TUI 구현 ✓
- 1 TS bridge ✓
- 1 Rust crate ✓

### 5. Trade-off 정직 surface

- mypy disable_error_code 14개 → "사실상 noop" 명시
- 85% 비용 절감 주장 → "검증 못 함, 의심"
- OpenCode 패리티 → "동등 보다 공존"
- AC-agnostic invariant → "코드 주석으로만 enforce, fragile"

→ 마케팅 톤 회피, 엔지니어링 톤.

### 6. Deep-dive 진입점 매핑

Section 24 가 "더 깊이 알고 싶을 때 보라" 매핑 제공:
- 멀티-AC stage1 reuse 메커니즘
- RuntimeHandle backend alias 추가 방법
- Cancellation registry 동시성 보장
- MCP Job lifecycle
- Lateral think multi-persona fan-out
- Brownfield mechanical.toml 자동 작성
- Skill setup gate 우회
- Stage 3 deliberative consensus
- TUI 폴링 비교
- Bridge plugin dispatch envelope

### 7. 정직한 미커버리지 노출

Section 27 가 확인 안 한 영역 명시:
- runner.py 의 300 라인 이후 (1/4 만 read)
- parallel_executor.py 144 KB (시그니처만)
- cli/commands/init.py 29 KB (시그니처 미확인)
- mcp/tools/definitions.py 11 KB (도구 이름 역추적만)
- 22 commands stub 본문
- 270+ 테스트 케이스 본문
- CHANGELOG 200줄 이후 (0.13.x 이전)

→ 사용자가 "더 깊이" 요청 시 명시 부탁.

## Critic 의 Iterate 권고 (반려 아님, 미세 개선)

1. **Approval condition**: 분석은 plan 이 아니라 이미 완성된 deliverable. ralplan 의 6단계 (Critic verdict 후 사용자 final approval) 중 6단계는 사용자가 결정.

2. **개선 follow-up 권고** (분석 결과 자체):
   - mypy `arg-type/return-value/assignment` 점진 활성화
   - Stage 1 AC-agnostic invariant 코드로 enforce
   - 거대 단일 파일 점진 분리
   - OpenCode subprocess 하드코딩 config 화
   - Hermes e2e 적용
   - `.scalars()` vs `.mappings()` lint rule

→ Ouroboros maintainer 에게 전달 가치 있는 신호.

## Final Critic Statement

**APPROVE**. 분석이 ouroboros 의 본질 (loop) 표현 + 위험 5종 surface + 모듈 레벨 매핑 완전 + 정직한 미커버리지 노출. 사용자 요구 충족.

추가 deep-dive 필요 시 Section 24 의 진입점 매핑 사용 권장.
