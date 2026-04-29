# 06. Cross-Cutting Themes — Repo 전체에 흐르는 패턴

22 skill 을 가로지르는 반복 idea / 안티패턴 / 관용구. 각 테마는 어느 skill 에 어떻게 등장하는지 cross-reference 해요.

---

## 1. Vertical Slices = Tracer Bullets

> A thin **vertical slice that cuts through ALL integration layers end-to-end**, NOT a horizontal slice of one layer.
> — `to-issues/SKILL.md` L25

### 어디 등장
- **`to-issues`** — issue 분해의 단위 (HITL/AFK + Blocked-by graph)
- **`tdd`** — TDD 의 cycle 단위 (`test1 → impl1 → test2 → impl2`)
- **`to-prd`** — PRD 의 user stories 가 vertical slice 정의
- **`request-refactor-plan`** (deprecated) — Martin Fowler tiny commit (다른 axis 지만 같은 가족)

### 핵심 규칙
- 각 slice 는 schema / API / UI / test 모든 layer 를 narrow 하지만 COMPLETE 통과
- 완성된 slice = demoable / verifiable 단독으로
- 굵은 slice 적게보다 얇은 slice 많이

### Anti-pattern
TDD 의 horizontal slicing — 모든 test 먼저, 모든 impl 다음. → "imagined" behavior test, refactor 깨짐, 헤드라이트 통과.

### Why repo-wide
slice 는 agent (또는 사람) 가 independently grab 가능. parallel work + AFK execution 가능. 여러 issue 가 동시 진행 가능 — 효율 multiplier.

---

## 2. Deep Modules

> small interface + lots of implementation
> — `tdd/deep-modules.md` L3, Ousterhout *A Philosophy of Software Design*

### 어디 등장
- **`tdd`** — Planning 단계에 "deep module (보조 doc `tdd/deep-modules.md`) 기회 식별" — interface design 영향
- **`to-prd`** — module sketch 단계 "deep module 추출 적극적으로"
- **`improve-codebase-architecture`** — "Deepening opportunity" 발굴이 skill 의 목표 자체
- **`design-an-interface`** (deprecated) — Evaluation criteria 의 한 축
- **`request-refactor-plan`** (deprecated) — refactor 평가 기준

### 평가 기준 두 framing

**A. Implementation 시각화 (`tdd/deep-modules.md`)**
```
deep:
┌─────────────────────┐
│   Small Interface   │
├─────────────────────┤
│ Deep Implementation │
└─────────────────────┘

shallow (avoid):
┌─────────────────────────────────┐
│       Large Interface           │
├─────────────────────────────────┤
│  Thin Implementation            │
└─────────────────────────────────┘
```

**B. Leverage 시각화 (`improve-codebase-architecture/LANGUAGE.md`)**
- depth = leverage / unit of interface learned
- 매트가 Ousterhout 원본 (impl-lines / interface-lines ratio) **거부** — padding 보상 위험. depth-as-leverage 채택.

### Deletion test
> Imagine deleting the module. If complexity vanishes, the module wasn't hiding anything (pass-through). If complexity reappears across N callers, the module was earning its keep.

→ shallow 의심 검증법.

### Internal vs external seam
- Deep module 은 internal mockable swappable part 가질 수 있어요.
- 그 part 는 interface 의 일부 아님 — internal seam.
- test 가 internal seam 사용해도 interface 로 expose 금지.

---

## 3. Ubiquitous Language / Domain Glossary (CONTEXT.md)

> "With a ubiquitous language, conversations among developers and expressions of the code are all derived from the same domain model." — Eric Evans, *DDD*
> — README.md L65

### 어디 등장
- **`grill-with-docs`** — CONTEXT.md inline 갱신 (생산자)
- **`improve-codebase-architecture`** — CONTEXT.md vocabulary 사용 + 새 term 발견 시 추가 (생산 + 소비)
- **`diagnose`** — soft reference ("the project's domain glossary")
- **`tdd`** — soft reference (test name + interface vocabulary)
- **`to-prd`** — PRD 내 사용
- **`to-issues`** — issue title + body 사용
- **`triage`** — codebase summary 작성 시 사용
- **`zoom-out`** — module map 의 vocabulary
- **`ubiquitous-language`** (deprecated) — 후계 `grill-with-docs/CONTEXT-FORMAT.md`
- **`qa`** (deprecated) — 후계 `triage`

### 4가지 효과 (README.md L93-96)
1. 변수 / 함수 / 파일 이름 일관 — "shared language" 사용
2. agent 가 codebase navigation 쉬움
3. agent 가 thinking token 덜 씀 — 간결한 언어
4. 매트 표현: "단일 가장 멋진 technique" — 회의/문서가 같은 단어로.

### Format 규칙 (`CONTEXT-FORMAT.md`)
- **Be opinionated** — 동의어 best 1 + Avoid list
- **Flag conflicts** explicit (Flagged ambiguities section)
- **Tight 1-sentence definition** (IS, NOT does)
- **Bold term name + cardinality** in relationships
- **Example dialogue** dev × 도메인 expert
- 도메인 specific only — generic programming 제외

### Single vs multi context
- Single — 루트 `CONTEXT.md` + `docs/adr/` (대부분)
- Multi — 루트 `CONTEXT-MAP.md` + 각 context `CONTEXT.md` + 각 context `docs/adr/` (모노레포)

---

## 4. Lazy File Creation

> Create files lazily — only when you have something to write.
> — `grill-with-docs/SKILL.md` L47, `setup-matt-pocock-skills/domain.md` L11

### 어디 등장
- **`grill-with-docs`** — `CONTEXT.md` / `docs/adr/` lazy
- **`improve-codebase-architecture`** — CONTEXT.md 새 term 추가 시 file 없으면 생성
- **`triage`** — `.out-of-scope/<concept>.md` 첫 거절 시 생성
- **`setup-matt-pocock-skills/domain.md`** — "If files don't exist, **proceed silently**. Don't flag absence; don't suggest creating upfront. Producer skill creates them lazily when needed."

### 원칙
- file up-front 강제 X
- 사용자가 모르는 file 시스템 강요 X
- 첫 사용 시점에 빈 file 만 보내고 안 채우는 ceremony X

### Side effect
soft dependency 가 graceful degrade — file 부재 시 silent.

---

## 5. Durable, Not Procedural (Issue/Brief Writing)

> The issue may sit in `ready-for-agent` for days or weeks. The codebase will change. Write the brief so it stays useful.
> — `triage/AGENT-BRIEF.md` L7-15

### 어디 등장
- **`triage/AGENT-BRIEF.md`** — agent brief 의 핵심 원칙
- **`to-prd`** — "Do NOT include specific file paths or code snippets. They may end up being outdated very quickly."
- **`to-issues`** — vertical slice template 이 file path 포함 안 함
- **`qa`** (deprecated) — "No file paths or line numbers — these go stale"
- **`request-refactor-plan`** (deprecated) — "Decision Document: Do NOT include specific file paths or code snippets."

### Do
- interface, type, behavioral contract 묘사
- specific type/function signature/config shape 이름
- testable acceptance criteria

### Don't
- file path
- line number
- 현재 구조 가정
- "fix the triage thing" 같은 vague description

### Anti-pattern (`AGENT-BRIEF.md` L146-160)
> **Summary:** Fix the triage bug
> **Files to change:** src/triage/handler.ts (line 150), src/types.ts (line 42)

문제: vague + file/line stale + acceptance criteria 부재 + scope boundary 부재.

---

## 6. Behavioral, Not Procedural

> Describe **what** the system should do, not **how** to implement it.
> — `triage/AGENT-BRIEF.md` L18

### 어디 등장
- **`triage/AGENT-BRIEF.md`** — Behavioral 원칙 명시
- **`tdd/tests.md`** — test 가 WHAT 묘사, NOT HOW
- **`improve-codebase-architecture`** — candidate proposal 이 plain English behavior, code X

### TDD 적용 (`tests.md`)

**Good** — "user can checkout with valid cart"
**Bad** — "checkout calls paymentService.process"

**Bad 더 미묘**:
```typescript
// Bypasses interface
await createUser({ name: "Alice" });
const row = await db.query("SELECT * FROM users WHERE name = ?", ["Alice"]);
```

**Good**:
```typescript
const user = await createUser({ name: "Alice" });
const retrieved = await getUser(user.id);  // interface 통해
```

---

## 7. Mock at Boundaries Only

> Mock at **system boundaries** only:
> — External APIs, DBs (sometimes), Time/randomness, FS (sometimes).
> Don't mock: Your own classes/modules, internal collaborators, anything you control.
> — `tdd/mocking.md` L1-15

### 어디 등장
- **`tdd/mocking.md`** — 핵심 source-of-truth
- **`improve-codebase-architecture/DEEPENING.md`** — Dependency 카테고리 4종 (in-process / local-substitutable / remote-but-owned / true-external) 이 mock 전략 결정
- **`tdd/interface-design.md`** — DI 가 mockability 메커니즘

### Boundary 4 카테고리

| Category | 예시 | 처리 |
|---|---|---|
| In-process | 순수 계산, in-memory | Mock 불필요. interface 직접 test. |
| Local-substitutable | PGLite for Postgres, in-memory FS | stand-in test suite 에서 실행 |
| Remote but owned | microservice, internal API | Port + Adapter (in-memory test, HTTP/gRPC prod) |
| True external | Stripe, Twilio | Injected port + mock adapter |

### Design for mockability

**1. DI**:
```typescript
// Easy mock
function processPayment(order, paymentClient) { ... }

// Hard mock
function processPayment(order) {
  const client = new StripeClient(process.env.STRIPE_KEY);
  ...
}
```

**2. SDK-style > generic**:
- 각 endpoint 별 function — 각 mock 이 specific shape, conditional 없음.
- 단일 generic `fetch(endpoint, options)` 는 mock 안 conditional 강요.

---

## 8. Design It Twice (Parallel Sub-Agents)

> Your first idea is unlikely to be the best. Generate multiple radically different designs, then compare.
> — `design-an-interface/SKILL.md` (deprecated) + `improve-codebase-architecture/INTERFACE-DESIGN.md`

### 어디 등장
- **`improve-codebase-architecture/INTERFACE-DESIGN.md`** — 정교한 후계
- **`design-an-interface`** (deprecated) — 원본

### 패턴
1. Frame problem space (사용자 흡수 + sub-agent 병렬 일)
2. **3+ sub-agent 병렬** Task tool — 각자 radically different constraint:
   - Minimize interface
   - Maximize flexibility
   - Optimize most common case
   - Ports & adapters (cross-seam dependency 시)
3. Sequential present (사용자 흡수)
4. Prose 비교 (depth, locality, seam placement) — 표 X
5. Opinionated 추천 + hybrid propose

### 핵심
- "Be opinionated — 사용자는 menu 가 아니라 strong read 원함"
- LANGUAGE.md + CONTEXT.md vocabulary 둘 다 brief 에 포함

---

## 9. Grilling — Question One at a Time

> Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Ask the questions one at a time.
> — `grill-me/SKILL.md` L7-9, `grill-with-docs/SKILL.md` L9-10

### 어디 등장
- **`grill-me`** — 비코드 grilling
- **`grill-with-docs`** — 코드 + 도메인 awareness
- **`improve-codebase-architecture`** — Step 3 grilling loop
- **`triage`** — `needs-info` 처리 시 grilling 세션
- **`request-refactor-plan`** (deprecated) — Step 4 detailed interview
- **`design-an-interface`** (deprecated) — Step 1 requirements gather

### 원칙
- One question at a time (waiting for feedback)
- 각 질문에 본인 추천 답안 제공
- codebase 로 답할 수 있으면 codebase explore 우선
- decision tree 의 모든 가지 resolve

### `.out-of-scope/question-limits.md` — 의도적 미적용
> The `/grill-me` skill (and grilling sessions inside other skills) does not enforce a maximum number of questions.

> 이유:
> - 어려운 plan 은 50 질문 필요, 쉬운 건 3개. 고정 cap 은 둘 다 안 좋음.
> - escape hatch 이미 있음: 사용자가 stop 하면 그 시점 plan 받음, "wrap up" 하면 모델이 summarize.
> - 자연어 steering 이 의도된 control surface, 숫자 limit 아님.
> - 200 질문 = 사용자 plan 이 진짜 under-specified (working as intended) vs 모델이 redundant 질문 (prompt-quality 문제) — 다른 두 failure mode 를 conflate.

---

## 10. Disable Model Invocation (명시적 Trigger Only)

YAML frontmatter `disable-model-invocation: true` 가진 skill — agent 가 자동 발동 안 함.

### 어디 등장
- `grill-with-docs`
- `setup-matt-pocock-skills`
- `zoom-out`
- `ubiquitous-language` (deprecated)

### Why
이 4개는 **사용자 의도 강한** task — agent 가 추정 발동하면 잘못. 명시적 invoke 만:
- `grill-with-docs` — relentless interview, 의도 외 발동 시 noise
- `setup-matt-pocock-skills` — 새 file 작성 (`docs/agents/...`), 사용자 모르게 X
- `zoom-out` — 사용자가 "I don't know this area" 명시적 표명 시
- `ubiquitous-language` — 새 file 작성 (`UBIQUITOUS_LANGUAGE.md`)

다른 18 skill 은 description trigger 자유 — agent 가 자연어 매치 시 발동.

---

## 11. Hard vs Soft Dependency (Setup Pointer)

ADR-0001 의 핵심 결정. 7 engineering skill 의 setup config 의존:

### Hard (3) — 명시적 setup pointer
> "… should have been provided to you — run `/setup-matt-pocock-skills` if not."

- `to-issues`
- `to-prd`
- `triage`

config 없으면 **잘못된 출력** (틀린 tracker, 틀린 label).

### Soft (4) — vague reference 만
> "the project's domain glossary"
> "ADRs in the area you're touching"

- `diagnose`
- `tdd`
- `improve-codebase-architecture`
- `zoom-out`

config 없어도 작동, 출력이 sharp 덜할 뿐.

### Why
- token 효율 — soft 에 setup pointer 안 cargo cult
- 인지 부하 — 사용자가 setup 강요 안 받음
- graceful degrade — soft 는 lazy creation + silent absence 와 페어

---

## 12. Triage 관련 의무 disclaimer

> Every comment or issue posted to the issue tracker during triage **must** start with this disclaimer:
> ```
> > *This was generated by AI during triage.*
> ```
> — `triage/SKILL.md` L9-15

### 어디 등장
- `triage` only — agent brief / triage notes / wontfix 댓글 모두

### Why
transparency. 매트가 AI-generated 컨텐츠 명시 — human 검토 가능. trust boundary 명확.

---

## 13. Iterate the Loop Itself (Diagnose 의 메타 원칙)

> Treat the loop as a product. Once you have _a_ loop, ask:
> - Can I make it faster?
> - Can I make the signal sharper?
> - Can I make it more deterministic?
> — `diagnose/SKILL.md` L34-41

### 어디 등장
- `diagnose` Phase 1
- `tdd` 도 비슷한 정신 — incremental cycle 자체가 product

### 적용
- 30초 flaky loop 는 무 loop 와 다름없어요.
- 2초 deterministic loop = debugging superpower.
- 50% flake = debuggable, 1% = 안 됨 — rate 끌어올리기.

---

## 14. Inline Doc Update (Side Effects During Conversation)

> When a term is resolved, update `CONTEXT.md` right there. Don't batch these up — capture them as they happen.
> — `grill-with-docs/SKILL.md` L67-71

### 어디 등장
- `grill-with-docs` — CONTEXT.md / ADR
- `improve-codebase-architecture` — CONTEXT.md / ADR
- `triage` — `.out-of-scope/`

### Why
batch 하면 잊힘. 결정의 컨텍스트가 그 시점에 가장 fresh — 즉시 capture.

---

## 15. Self-Application

매트가 본인 skill 의 패턴을 본인 repo 에 적용:
- `CONTEXT.md` (skills repo 의) 가 `CONTEXT-FORMAT.md` 따름
- `docs/adr/0001-...md` 가 `ADR-FORMAT.md` minimal template 따름
- `.out-of-scope/question-limits.md` 가 `OUT-OF-SCOPE.md` format 따름

이건 dogfooding 신호 — 매트 자신이 sharp 한 도구로 일하고 있어요.

---

## 16. Minimal Where Possible

- `zoom-out/SKILL.md` — 7 lines. 단일 instruction.
- `grill-me/SKILL.md` — 10 lines.
- `improve-codebase-architecture/LANGUAGE.md` 의 ADR-0001 — 10 lines.
- ADR template — 1-3 sentence 가능.

작게 유지 — README.md L17:
> These skills are designed to be small, easy to adapt, and composable.

---

## 17. Composability — Skill 간 Hand-off

skill 들이 명시적 hand-off 패턴 가짐:

- `diagnose` Phase 6 → `/improve-codebase-architecture` (architectural 발견 시)
- `improve-codebase-architecture` Step 3 grilling → `/grill-with-docs` 와 같은 discipline
- `triage` Step 4 → `/grill-with-docs`
- `to-issues` → `triage` (모든 새 issue 가 `needs-triage`)
- `to-prd` → `triage` (모든 새 PRD 가 `needs-triage`)

자세한 dependency graph 는 `07-skill-relationships.md`.

---

## 17가지 테마 한 줄 요약

| # | Theme | 핵심 한 문장 |
|---|---|---|
| 1 | Vertical slices | layer-by-layer 아니고 end-to-end thin slice |
| 2 | Deep modules | small interface + deep impl, leverage 측정 |
| 3 | Ubiquitous language | CONTEXT.md 가 도메인 어휘 source-of-truth |
| 4 | Lazy file creation | 처음 쓸 때만 만들기, silent on absence |
| 5 | Durable not procedural | file path / line number 금지 |
| 6 | Behavioral not procedural | WHAT 묘사, NOT HOW |
| 7 | Mock at boundaries only | 자기 modules 절대 mock 금지 |
| 8 | Design It Twice | 3+ sub-agent radically different |
| 9 | Grilling one-at-a-time | 결정 트리 가지마다 resolve |
| 10 | disable-model-invocation | 명시적 invoke skill 4종 |
| 11 | Hard vs soft dependency | ADR-0001 의 명시적 split |
| 12 | AI disclaimer | triage transparency 강제 |
| 13 | Iterate the loop | 피드백 루프 자체가 product |
| 14 | Inline doc update | batch 금지, 즉시 capture |
| 15 | Self-application | 본인 repo 가 본인 skill 적용 |
| 16 | Minimal where possible | 7-line skill 부터 1-paragraph ADR 까지 |
| 17 | Composability | skill 간 명시적 hand-off |
