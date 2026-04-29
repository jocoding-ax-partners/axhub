# 02. Engineering Skills — 9개 상세 분석

각 skill 마다: frontmatter / 의존성 카테고리 / process / 핵심 idea / 보조 doc / 분석가 관찰.

---

## 1. `diagnose` — 어려운 버그용 6단계 진단 루프

**파일**: `skills/engineering/diagnose/SKILL.md` (117 lines) + `scripts/hitl-loop.template.sh` (41 lines)

**Frontmatter**:
```yaml
name: diagnose
description: Disciplined diagnosis loop for hard bugs and performance regressions.
  Reproduce → minimise → hypothesise → instrument → fix → regression-test.
  Use when user says "diagnose this" / "debug this", reports a bug,
  says something is broken/throwing/failing, or describes a performance regression.
```

**Dependency**: Soft. 도메인 glossary + ADR 가 vague reference 로만 등장. setup pointer 없음.

**Process**:

### Phase 1 — Build a feedback loop **(skill 의 본질)**

> "This is the skill. Everything else is mechanical. If you have a fast, deterministic, agent-runnable pass/fail signal for the bug, you will find the cause."

10가지 루프 구축법, 대략 이 순서:
1. Failing test (unit/integration/e2e)
2. Curl/HTTP script
3. CLI invocation + 알려진 good snapshot diff
4. Headless browser script (Playwright/Puppeteer)
5. Captured trace replay
6. Throwaway harness (서비스 1개 + mocked deps)
7. Property/fuzz loop (1000 random inputs)
8. Bisection harness (`git bisect run`)
9. Differential loop (old vs new diff)
10. **HITL bash script** (마지막 수단, `hitl-loop.template.sh` 사용)

루프 자체를 product 처럼 iterate:
- 빠르게 (cache setup, init skip, scope 좁힘)
- 신호 sharper (구체적 symptom 만 assert, "didn't crash" 금지)
- 결정적 (시간 pin, RNG seed, FS 격리, network freeze)

**Non-deterministic bugs**: 깨끗한 repro 가 목표 아님. **재현율** 을 끌어올리기. 1% → 50% → debuggable.

루프 못 만들면 멈추고 시도한 것 listing + (a) 환경 access (b) artifact (c) 임시 prod 계측 권한 요청. **Phase 2 로 넘어가지 마.**

### Phase 2 — Reproduce
- 사용자가 묘사한 **그** failure mode 인지 확인 (다른 bug 가 우연히 옆에 있는 거 아님)
- 여러 번 재현
- 정확한 symptom 캡처

### Phase 3 — Hypothesise
**3-5 ranked hypotheses** before testing any. 단일 가설 anchoring 금지.

각 가설은 **falsifiable**:
> "If <X> is the cause, then <changing Y> will make the bug disappear / <changing Z> will make it worse."

prediction 못 쓰겠으면 그건 vibe — 버리거나 sharpen.

ranked list 를 사용자에게 먼저 보여주기. "we just deployed a change to #3" 같은 도메인 정보로 instant rerank 가능. matt 의 voice: **"Cheap checkpoint, big time saver."** block 하지 말고 사용자 AFK 면 그대로 진행.

### Phase 4 — Instrument
각 probe 는 Phase 3 의 specific prediction 에 매핑. **변수 한 번에 하나만**.

도구 우선순위:
1. Debugger / REPL (한 breakpoint > 10 logs)
2. Hypothesis 구분 boundary 에 targeted log
3. ❌ "log everything and grep"

**모든 debug log 에 unique prefix tag**: `[DEBUG-a4f2]`. cleanup 은 grep 한 번.

**Perf branch**: log 보통 틀림. baseline 측정 (timing harness, `performance.now()`, profiler, query plan) 후 bisect.

### Phase 5 — Fix + regression test

regression test 를 **fix 전에** 작성 — 단 **correct seam** 있을 때만.

correct seam = test 가 call site 에서 발생하는 그대로의 bug pattern 을 exercise 하는 곳. 너무 shallow 한 seam (single-caller test 인데 multi-caller bug, unit test 가 trigger chain replicate 못 함) 은 false confidence.

**correct seam 없으면 그게 finding 자체.** codebase architecture 가 bug lockdown 을 막고 있음. Phase 6 에서 flag.

correct seam 있으면:
1. minimised repro → 그 seam 의 failing test
2. 실패 확인
3. fix 적용
4. 통과 확인
5. Phase 1 의 원본 (un-minimised) 시나리오 재실행

### Phase 6 — Cleanup + post-mortem

요구사항:
- [ ] 원본 repro 가 더는 재현 안 됨
- [ ] regression test 통과 (또는 seam 부재 문서화)
- [ ] 모든 `[DEBUG-...]` 계측 제거 (`grep` prefix)
- [ ] throwaway prototype 제거 또는 명확히 표시된 debug location 으로 이동
- [ ] 맞았던 가설을 commit/PR message 에 기록

**그리고 묻기: 무엇이 이 bug 를 막았을까?** architectural change (good test seam 부재, tangled callers, hidden coupling) 면 `/improve-codebase-architecture` 로 hand off — fix 가 들어간 **후** 에 추천 (시작 때보다 정보 많음).

### `hitl-loop.template.sh`

`step` 헬퍼 = "지시 + Enter 대기", `capture` 헬퍼 = "질문 + 응답을 변수로". 끝에 `KEY=VALUE` print → agent 가 parse. UI clicking 이 필요한 마지막 수단 시나리오.

**관찰**:
- "Phase 1 이 skill 의 본질" 이라고 SKILL.md 가 명시. 나머지는 기계적.
- 6 phase 명시적, 각 phase 의 종료 조건 (`Do not proceed until …`) 명시.
- 시작-멈춤 권한 (못 만들면 사용자에게 명시적 요청) 강조.
- 자동 모드 가능: hypothesis ranked list 사용자 부재 시 그대로 진행.

---

## 2. `grill-with-docs` — Plan 을 도메인 모델 + 코드 + 문서에 stress-test

**파일**: `skills/engineering/grill-with-docs/SKILL.md` (81 lines) + `ADR-FORMAT.md` (47) + `CONTEXT-FORMAT.md` (77)

**Frontmatter**:
```yaml
name: grill-with-docs
description: Grilling session that challenges your plan against the existing
  domain model, sharpens terminology, and updates documentation (CONTEXT.md, ADRs)
  inline as decisions crystallise.
  Use when user wants to stress-test a plan against their project's language and documented decisions.
disable-model-invocation: true   # ← 명시적 사용자 invoke 만
```

**Dependency**: Soft. setup pointer 없음. 보조 doc 2개 (ADR-FORMAT, CONTEXT-FORMAT) 가 file 포맷 source-of-truth.

**Core instruction** (L7):
> Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.
> Ask the questions one at a time, waiting for feedback on each question before continuing.
> If a question can be answered by exploring the codebase, explore the codebase instead.

**Domain awareness**:

### File structure 인식
- 단일 context: `/CONTEXT.md` + `/docs/adr/`
- 멀티 context: `/CONTEXT-MAP.md` + `src/<context>/CONTEXT.md` + `src/<context>/docs/adr/`
- **lazy creation** — 처음 term resolve 될 때만 만듦.

### Session 중 행동 4가지
1. **Glossary 와 충돌 시 즉시 call out**: "Your glossary defines 'cancellation' as X, but you seem to mean Y — which is it?"
2. **Fuzzy term sharpen**: "You're saying 'account' — do you mean the Customer or the User?"
3. **구체적 시나리오 stress-test**: edge case 시나리오 invent.
4. **코드와 cross-reference**: "Your code cancels entire Orders, but you just said partial cancellation is possible — which is right?"

### Inline doc update
- `CONTEXT.md` 는 그 자리에서 수정 — batch 안 함. implementation detail 은 빼고 도메인 expert 에게 의미 있는 term 만.
- ADR 은 **3가지 모두 true 일 때만** 제공:
  1. **Hard to reverse** (생각 바꾸는 비용이 큼)
  2. **Surprising without context** ("왜 이렇게 했지?")
  3. **The result of a real trade-off** (진짜 대안 + 구체 이유)

3 중 하나라도 빠지면 ADR 없음.

### `ADR-FORMAT.md`

```md
# {Short title}

{1-3 sentences: context, decision, why.}
```

**That's it.** 한 paragraph 가능. 가치는 "결정이 있었다 + 왜" 기록 — section 채우기 아님. Optional sections (Status frontmatter / Considered Options / Consequences) 는 추가 가치 있을 때만.

번호: `0001-slug.md`, `0002-slug.md`. `docs/adr/` 의 max 숫자 + 1.

**ADR 자격**:
- 아키텍처 shape (monorepo, event-sourced)
- Context 간 통합 패턴 (domain event vs sync HTTP)
- Lock-in 있는 기술 선택 (DB, message bus, auth, deploy target)
- Boundary / scope (Customer 데이터 owner)
- 의도적 obvious-path 이탈 (manual SQL instead of ORM)
- 코드에 안 보이는 제약 (compliance, partner SLA)
- non-obvious 한 거절된 대안 (GraphQL 고려 후 REST 선택)

### `CONTEXT-FORMAT.md`

```md
# {Context Name}

{One or two sentence description.}

## Language

**Order**:
{Concise description}
_Avoid_: Purchase, transaction

## Relationships
- An **Order** produces one or more **Invoices**

## Example dialogue
> **Dev:** ...
> **Domain expert:** ...

## Flagged ambiguities
- "account" was used to mean both **Customer** and **User** — resolved: distinct.
```

**Rules**:
- **Be opinionated** — 동의어는 best 하나 + 나머지는 avoid.
- **Flag conflicts** — Flagged ambiguities 에 명시적 해결.
- **Tight definitions** — 1 문장 max. IS 정의, NOT does.
- **Show relationships** — bold term + cardinality.
- **도메인 specific 만** — generic programming 개념 (timeout, error type) 제외.
- **자연스럽게 군집화** — subheading 으로.
- **Example dialogue** — dev + 도메인 expert 간 대화로 term 사용 시연.

**관찰**:
- `disable-model-invocation: true` — 자동 호출 안 됨. grill 은 user 가 명시적으로 원할 때만.
- doc format 을 보조 file 로 분리 — SKILL.md 는 process, format 은 reference. **progressive disclosure** 패턴.
- "If a question can be answered by exploring the codebase, explore the codebase instead" — 사용자 시간 아끼기.

---

## 3. `triage` — 5-role 상태 머신

**파일**: `skills/engineering/triage/SKILL.md` (103 lines) + `AGENT-BRIEF.md` (168) + `OUT-OF-SCOPE.md` (101)

**Frontmatter**:
```yaml
name: triage
description: Triage issues through a state machine driven by triage roles.
  Use when user wants to create an issue, triage issues, review incoming bugs or feature requests,
  prepare issues for an AFK agent, or manage issue workflow.
```

**Dependency**: **Hard**. setup pointer 명시 (L38).

**Mandatory disclaimer** (모든 댓글/이슈 첫 줄):
```
> *This was generated by AI during triage.*
```

### Roles

**Category** (한 개 필수): `bug` / `enhancement`

**State** (한 개 필수): `needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`

State 충돌 시 → flag + maintainer 에게 묻기.

**State transitions**:
- 라벨 없음 → `needs-triage`
- `needs-triage` → 어디든
- `needs-info` → reporter 응답 시 `needs-triage` 복귀
- maintainer override 가능 — 비정상이면 flag

### Invocation

`/triage` + 자연어. 예시:
- "Show me anything that needs my attention"
- "Let's look at #42"
- "Move #42 to ready-for-agent"
- "What's ready for agents to pick up?"

### "What needs attention" 모드

3 bucket, oldest first:
1. **Unlabeled** — 한 번도 triage 안 됨
2. **`needs-triage`** — 진행 중
3. **`needs-info` + reporter 활동 since last triage notes** — 재평가 필요

count + 한 줄 summary. maintainer 가 pick.

### "Triage specific issue" 모드 5단계

1. **Gather context** — 전체 issue (body, comments, labels, reporter, dates), 이전 triage notes parse, 코드베이스 탐색 (도메인 glossary), `.out-of-scope/*.md` 읽기 + 이전 거절과 매치 surface.
2. **Recommend** — category + state recommendation + 이유 + 짧은 codebase summary. 대기.
3. **Reproduce (bugs only)** — grilling 전 reporter steps 따라 재현 시도. 결과: 성공 + code path / 실패 / insufficient detail (`needs-info` 신호).
4. **Grill (필요시)** — `/grill-with-docs` 세션.
5. **Apply outcome**:
   - `ready-for-agent` → AGENT-BRIEF 댓글
   - `ready-for-human` → AGENT-BRIEF 와 같은 구조 + delegation 불가 이유
   - `needs-info` → triage notes (template 아래)
   - `wontfix` (bug) → polite 설명 + close
   - `wontfix` (enhancement) → `.out-of-scope/` 작성 + 댓글에서 링크 + close
   - `needs-triage` → role 적용. 진행 중이면 optional 댓글.

### Quick override

"move #42 to ready-for-agent" → maintainer 신뢰, role 직접 적용. grilling 건너뛰기. ready-for-agent 로 갈 때 brief 쓸지 묻기.

### Needs-info template

```md
## Triage Notes

**What we've established so far:**
- point 1

**What we still need from you (@reporter):**
- question 1
```

Grilling 중 resolved 된 건 모두 "established" 에 — 작업 손실 방지. 질문은 specific + actionable, "please provide more info" 금지.

### Resuming

이전 triage notes 있으면 읽고, reporter 응답 체크, 업데이트된 picture 제시 후 진행. resolved question 재질문 금지.

### `AGENT-BRIEF.md` (168 lines, 가장 큰 보조 doc)

**철학**: agent brief 는 contract. 원본 issue body + 토론은 context, agent brief 가 권위.

#### Durability over precision
- **Do**: interface, type, behavioral contract 묘사. specific type/function signature/config shape 이름.
- **Don't**: file path 참조 (stale). line number 참조. 현재 implementation 구조 가정.

#### Behavioral, not procedural
- **Good**: "The `SkillConfig` type should accept an optional `schedule` field of type `CronExpression`"
- **Bad**: "Open src/types/skill.ts and add a schedule field on line 42"

#### Complete acceptance criteria
- 각 criterion 독립적으로 verifiable.
- **Good**: "Running `gh issue list --label needs-triage` returns issues that have been through initial classification"
- **Bad**: "Triage should work correctly"

#### Explicit scope boundaries
gold-plating 방지. "Out of scope" 명시.

#### Template
```md
## Agent Brief

**Category:** bug / enhancement
**Summary:** one-line

**Current behavior:**
**Desired behavior:**
**Key interfaces:**
- `TypeName` — what changes and why
- `functionName()` return type
- Config shape

**Acceptance criteria:**
- [ ] criterion 1

**Out of scope:**
- ...
```

좋은 예시 + 나쁜 예시 둘 다 SKILL 안에 풀 인라인 (각 ~30 lines).

### `OUT-OF-SCOPE.md` — `.out-of-scope/` knowledge base

`.out-of-scope/` 디렉토리 = 거절된 feature 요청의 영구 기록. 두 목적:
1. **Institutional memory** — 왜 거절했는지 (issue close 후에도 lost 안 됨)
2. **Deduplication** — 같은 요청 다시 오면 prior 결정 surface

**구조**: `concept` 별 한 파일 (issue 별 X). 같은 것 요청한 여러 issue 는 하나로 group.

**Format**:
```md
# Dark Mode

This project does not support dark mode or user-facing theming.

## Why this is out of scope

The rendering pipeline assumes a single color palette defined in
`ThemeConfig`. Supporting multiple themes would require:
- A theme context provider wrapping the entire component tree
- Per-component theme-aware style resolution
- A persistence layer for user theme preferences

This is a significant architectural change that doesn't align with the
project's focus on content authoring.

## Prior requests
- #42 — "Add dark mode support"
- #87 — "Night theme for accessibility"
- #134 — "Dark theme option"
```

**Naming**: `<concept>.md` kebab-case, recognizable.

**Reason 작성**:
- 프로젝트 scope/철학
- 기술 제약
- 전략적 결정

**금지**: temporary 환경 ("we're too busy right now") — 이건 deferral 이지 rejection 아님.

**언제 체크**: triage Step 1 (Gather context) 에서 모든 `.out-of-scope/*.md` 읽기. 매칭 시 maintainer 에게 surface (concept similarity, 키워드 아님).

**언제 작성**: enhancement (bug 아님) 거절 시. 매칭 파일 있으면 prior requests 에 append, 없으면 새 파일. 이슈에 댓글 + close.

**삭제**: maintainer 가 마음 바꾸면 파일 삭제. 옛 issue reopen 안 함 (historical record).

**관찰**:
- 5-role 상태 머신이 issue lifecycle 를 명확히 정의. AGENT-BRIEF 가 ready-for-agent 의 contract.
- AI-generated disclaimer 강제 — transparency.
- `.out-of-scope/` 가 institutional memory 메커니즘. 같은 요청 반복 거절 자동화.
- AGENT-BRIEF 의 "no file paths / line numbers" 규칙은 시간이 지나도 brief 가 stale 안 되게 함.

---

## 4. `improve-codebase-architecture` — Deepening 기회 발굴

**파일**: `SKILL.md` (71) + `LANGUAGE.md` (53) + `DEEPENING.md` (37) + `INTERFACE-DESIGN.md` (44)

**Frontmatter**:
```yaml
name: improve-codebase-architecture
description: Find deepening opportunities in a codebase, informed by the domain language
  in CONTEXT.md and the decisions in docs/adr/.
  Use when the user wants to improve architecture, find refactoring opportunities,
  consolidate tightly-coupled modules, or make a codebase more testable and AI-navigable.
```

**Dependency**: Soft.

### Glossary (`LANGUAGE.md`) — 정확한 vocabulary

이 skill 의 모든 제안은 정확히 이 단어를 사용:

- **Module** — interface + implementation 가진 모든 것 (function, class, package, slice). scale-agnostic.
- **Interface** — caller 가 알아야 하는 모든 것 — type signature + invariant + 순서 + error mode + 필수 config + 성능 특성. ~~"API"~~, ~~"signature"~~ 너무 좁음.
- **Implementation** — module 안의 코드.
- **Depth** — interface 의 leverage. 작은 interface 뒤에 많은 behavior. **deep** = 높은 leverage. **shallow** = interface 가 implementation 만큼 복잡.
- **Seam** (Michael Feathers) — behavior 를 그 자리에서 수정 안 하고 alter 할 수 있는 곳. interface 가 사는 location. ~~"boundary"~~ 는 DDD bounded context 와 overload.
- **Adapter** — seam 에서 interface 를 만족하는 구체. role (어느 슬롯) 묘사, substance 아님.
- **Leverage** — caller 가 depth 에서 얻는 것. 학습한 interface 한 개당 capability 가 N call site × M test 으로 갚음.
- **Locality** — maintainer 가 depth 에서 얻는 것. change/bug/knowledge/verification 이 한 곳에 집중. 한 번 fix 하면 전체 fix.

### Principles

- **Depth is a property of the interface, not the implementation** — deep module 은 내부적으로 작은 mockable swappable part 들로 구성될 수 있어요. 그 part 들은 interface 의 일부 아님. **internal seam** vs **external seam**.
- **The deletion test** — module 삭제 상상. complexity 사라지면 pass-through 였음. complexity 가 N caller 에 다시 나타나면 module 이 work 하고 있던 것.
- **The interface is the test surface** — caller 와 test 가 같은 seam 을 cross. interface 너머를 test 하고 싶으면 module shape 가 잘못됨.
- **One adapter = hypothetical seam. Two adapters = real seam.** — 무언가가 actually vary 하지 않으면 seam 도입 금지.

### Rejected framings (LANGUAGE.md L51-53)

- ~~Depth = implementation-lines / interface-lines ratio~~ (Ousterhout 원본) — implementation padding 보상. 우리는 depth-as-leverage.
- ~~"Interface" = TypeScript `interface` 키워드 / class public method~~ — 너무 좁음.
- ~~"Boundary"~~ — DDD bounded context 와 overload.

### Process (3 단계)

#### 1. Explore
도메인 glossary + ADR 먼저 읽기. 그 후 `Agent` tool with `subagent_type=Explore` — rigid heuristic 안 따르고 organically. Friction 노트:
- 한 개념 이해에 작은 module 여러 개 사이 bouncing?
- shallow module — interface 가 implementation 만큼 복잡?
- pure function 이 testability 만 위해 추출됐는데 진짜 bug 는 호출 방식 (locality 부재)?
- tightly-coupled module 이 seam 너머로 leak?
- 어느 부분 untested 또는 현재 interface 로 test 어려움?

shallow 의심되면 **deletion test** 적용.

#### 2. Present candidates
번호 매긴 list. 각 candidate:
- **Files** — 어떤 file/module
- **Problem** — 현재 architecture 가 friction 일으키는 이유
- **Solution** — 무엇을 바꿀지 plain English
- **Benefits** — locality + leverage 관점, test 개선 관점

**Vocabulary**: CONTEXT.md 도메인 + LANGUAGE.md architecture. "the Order intake module" — NOT "FooBarHandler", NOT "the Order service".

**ADR 충돌**: candidate 가 기존 ADR 모순 시 friction 진짜 클 때만 surface. 명시 mark: _"contradicts ADR-0007 — but worth reopening because…"_. ADR 이 금지하는 모든 이론적 refactor 나열 금지.

interface 제안 아직 안 함. "Which would you like to explore?" 묻기.

#### 3. Grilling loop
candidate 선택 후 grilling. design tree walk — constraint, dependency, deepened module shape, seam 뒤 무엇, 살아남는 test.

**Side effects (inline)**:
- CONTEXT.md 에 없는 concept 명명 → 추가 (`/grill-with-docs` 와 같은 discipline).
- Fuzzy term sharpen → 그 자리에서 update.
- 사용자가 load-bearing 이유로 거절 → "Want me to record this as an ADR so future architecture reviews don't re-suggest it?" — 미래 explorer 가 needed reason 일 때만, ephemeral ("not worth right now") + self-evident 제외.
- 대안 interface 탐색 → `INTERFACE-DESIGN.md`.

### `DEEPENING.md` — Dependency 카테고리 별 deepening 안전 처리

candidate dependency 분류로 testing 전략 결정.

#### 1. In-process
순수 계산, in-memory 상태, no I/O. 항상 deepenable. module 합치고 새 interface 통해 직접 test. adapter 불필요.

#### 2. Local-substitutable
local test stand-in 있는 dependency (PGLite for Postgres, in-memory FS). stand-in 있으면 deepenable. test suite 에서 stand-in 실행. seam 은 internal — module 의 external interface 에 port 없음.

#### 3. Remote but owned (Ports & Adapters)
자기 service 가 network boundary 너머 (microservice, internal API). seam 에 **port** (interface). deep module 이 logic 소유, transport 는 **adapter** 로 inject. test 는 in-memory adapter. prod 는 HTTP/gRPC/queue adapter.

#### 4. True external (Mock)
3rd party (Stripe, Twilio). external 을 injected port 로. test 는 mock adapter.

### Seam discipline

- **One adapter = hypothetical seam. Two = real.** prod + test 가 typical justification.
- **Internal vs external seam.** deep module 은 둘 다 가능. internal 을 test 가 사용해도 interface 로 expose 금지.

### Testing strategy: replace, don't layer

- 옛 shallow module unit test 는 새 interface test 생기면 waste — **삭제**.
- 새 test 는 deepened module interface 에서. **interface = test surface**.
- observable outcome 만 assert, internal state 아님.
- internal refactor 후에도 살아남아야 — 그게 implementation 통과 한 거임.

### `INTERFACE-DESIGN.md` — "Design It Twice" parallel sub-agent 패턴

#### 1. Frame the problem space
sub-agent 띄우기 **전** — chosen candidate 의 user-facing 설명:
- 새 interface 가 만족할 constraint
- 의존하는 dependency + DEEPENING.md 의 어느 카테고리
- 거친 illustrative code sketch (proposal 아님, constraint 구체화 도구)

사용자에게 보여주고 즉시 step 2 진행. 사용자는 sub-agent 가 일하는 동안 읽고 생각.

#### 2. Spawn sub-agents
3+ sub-agent 병렬 (Agent tool). 각자 **radically different** interface.

각 agent 에 별도 technical brief — file path, coupling detail, dependency category, seam 뒤 무엇.
- Agent 1: "Minimize the interface — 1-3 entry points max. Maximise leverage per entry."
- Agent 2: "Maximise flexibility — many use case + extension."
- Agent 3: "Optimise for the most common caller — default case trivial."
- Agent 4 (해당 시): "Ports & adapters around cross-seam dependencies."

**LANGUAGE.md + CONTEXT.md vocabulary 둘 다 brief 에 포함** — 일관 명명.

각 agent 출력:
1. Interface (type, method, param + invariant, 순서, error mode)
2. Usage example
3. Seam 뒤에 숨긴 것
4. Dependency 전략 + adapter
5. Trade-off — leverage 높은 곳 / 얇은 곳

#### 3. Present and compare
순차 제시 (사용자 흡수). prose 로 비교 — depth (leverage), locality (change 집중), seam placement.

본인 추천. 합쳐서 hybrid 가 좋으면 propose. **Be opinionated** — 사용자는 menu 가 아니라 strong read 를 원함.

**관찰**:
- 4개 보조 doc 이 progressive disclosure 의 모범 — SKILL.md 71 lines 가 process, LANGUAGE 53 이 vocabulary, DEEPENING 37 이 dependency 처리, INTERFACE-DESIGN 44 가 sub-agent 패턴.
- Glossary 가 아주 strict — substitute term 명시 거부.
- `improve-codebase-architecture` 가 `/grill-with-docs` 와 같은 inline doc update 패턴 사용. CONTEXT.md / ADR 둘 다 단일 grilling 루프에서 갱신.

---

## 5. `setup-matt-pocock-skills` — 7 consumer 의 단일 setup

**파일**: `SKILL.md` (119) + `domain.md` (51) + `issue-tracker-github.md` (22) + `issue-tracker-local.md` (19) + `triage-labels.md` (15)

**Frontmatter**:
```yaml
name: setup-matt-pocock-skills
description: Sets up an `## Agent skills` block in AGENTS.md/CLAUDE.md and `docs/agents/`
  so the engineering skills know this repo's issue tracker (GitHub or local markdown),
  triage label vocabulary, and domain doc layout.
  Run before first use of `to-issues`, `to-prd`, `triage`, `diagnose`, `tdd`,
  `improve-codebase-architecture`, or `zoom-out` — or if those skills appear to be missing
  context about the issue tracker, triage labels, or domain docs.
disable-model-invocation: true
```

**역할**: 7개 engineering skill 이 의존하는 per-repo config 시드.

### Process 5 step

#### 1. Explore
- `git remote -v`, `.git/config` — GitHub repo? 어느?
- `AGENTS.md`, `CLAUDE.md` 루트 — 존재? `## Agent skills` section 이미 있나?
- `CONTEXT.md`, `CONTEXT-MAP.md` 루트
- `docs/adr/`, `src/*/docs/adr/`
- `docs/agents/` — 이전 출력 있나?
- `.scratch/` — local-markdown convention 사용 중?

#### 2. Present + ask (한 번에 한 section)

**Section A — Issue tracker.**

> Explainer: "issue tracker" 는 이 repo 의 issue 가 사는 곳. `to-issues` / `triage` / `to-prd` / `qa` 가 read/write — `gh issue create` 부를지, `.scratch/` 마크다운 쓸지, 사용자가 묘사한 workflow 따를지 알아야 함.

기본 자세: GitHub. `git remote` 가 GitHub 가리키면 propose. 아니면:
- **GitHub** — repo 의 GitHub Issues
- **Local markdown** — `.scratch/<feature>/` (solo project / GitHub remote 없는 repo)
- **Other** (Jira, Linear, etc.) — 1 paragraph workflow 묘사 → freeform prose 기록

**Section B — Triage label vocabulary.**

> Explainer: `triage` 가 incoming issue 를 상태 머신 통과시킬 때 label (또는 issue tracker 의 등가물) 적용 — **실제 configured 된** 문자열과 일치해야 함. repo 가 다른 label 이름 쓰면 (e.g. `bug:triage` 대신 `needs-triage`) 매핑.

5 canonical role + default name 1:1 매핑 표.

**Section C — Domain docs.**

> Explainer: 일부 skill (`improve-codebase-architecture`, `diagnose`, `tdd`) 이 `CONTEXT.md` 도메인 언어 + `docs/adr/` 결정 읽음. global one 인지 multiple 인지 알아야 올바른 곳 봄.

- **Single-context** — 루트 `CONTEXT.md` + `docs/adr/`. 대부분.
- **Multi-context** — 루트 `CONTEXT-MAP.md` + 각 context `CONTEXT.md` (모노레포).

#### 3. Confirm + edit
draft 보여주기:
- `## Agent skills` 블록 (CLAUDE.md 또는 AGENTS.md 들어갈)
- `docs/agents/issue-tracker.md`, `triage-labels.md`, `domain.md` 내용

사용자 편집 허용.

#### 4. Write

**File 선택**:
- `CLAUDE.md` 있으면 그것
- 없고 `AGENTS.md` 있으면 그것
- 둘 다 없으면 사용자에게 물어보기 (절대 자동 선택 X)

`CLAUDE.md` 있는데 `AGENTS.md` 만들기 (또는 반대) 절대 금지 — 항상 기존 것 편집. 기존 `## Agent skills` 블록 있으면 in-place update — append 안 함. 주변 사용자 편집 보존.

**Block 형식**:
```md
## Agent skills

### Issue tracker
[one-line summary]. See `docs/agents/issue-tracker.md`.

### Triage labels
[one-line summary]. See `docs/agents/triage-labels.md`.

### Domain docs
[one-line summary — "single-context" or "multi-context"]. See `docs/agents/domain.md`.
```

3 doc 파일 작성 — 보조 template (`issue-tracker-github.md` / `issue-tracker-local.md` / `triage-labels.md` / `domain.md`) 시드.

"Other" issue tracker 면 `docs/agents/issue-tracker.md` 를 사용자 묘사로 from scratch.

#### 5. Done
완료 알림 + 어느 engineering skill 이 이걸 읽을지 알림. 사용자가 직접 `docs/agents/*.md` 편집해도 된다 안내. re-run 은 issue tracker 변경 또는 from scratch 재시작 때만.

### `domain.md` (보조)

- 탐색 전 `CONTEXT.md` 또는 `CONTEXT-MAP.md` 읽기.
- `docs/adr/` 의 관련 ADR 읽기. 멀티 context 면 `src/<context>/docs/adr/` 도.
- 파일 부재 시 **proceed silently** — flag 안 함, upfront 만들기 제안 안 함. producer skill (`/grill-with-docs`) 가 lazy 생성.
- output 의 도메인 concept 명명은 glossary term 사용. 없으면 신호 — language invent (재고) 또는 진짜 gap (`/grill-with-docs` 노트).
- ADR 충돌 시 명시적 surface: _"Contradicts ADR-0007 — but worth reopening because…"_

### `issue-tracker-github.md` / `issue-tracker-local.md` (보조)

위 01-architecture.md 에 정리된 명령어 / 규약. 두 file 의 이름은 사용자 선택에 따라 그 중 하나가 `docs/agents/issue-tracker.md` 로 복사.

### `triage-labels.md` (보조)

5 canonical role × 2 column (matt 의 / 우리 tracker 의) 표. 오른쪽 column 만 편집.

**관찰**:
- 7 consumer 가 single setup 으로 통합 — DRY.
- `disable-model-invocation: true` — 명시적 사용자 invoke 만. 매번 자동 호출 안 됨.
- 보조 doc 4개가 template 으로 번들 — `docs/agents/` 에 시드.
- "Other" tracker 도 freeform 으로 지원 — Jira/Linear 도 cover.
- CLAUDE.md/AGENTS.md 둘 중 어느 걸 편집할지 결정 logic 명시 — 둘 다 없으면 묻기. 기존 컨텐츠 안 덮어쓰기.

---

## 6. `tdd` — Vertical slice red-green-refactor

**파일**: `SKILL.md` (109) + `tests.md` (61) + `mocking.md` (59) + `interface-design.md` (31) + `deep-modules.md` (33) + `refactoring.md` (10)

**Frontmatter**:
```yaml
name: tdd
description: Test-driven development with red-green-refactor loop.
  Use when user wants to build features or fix bugs using TDD,
  mentions "red-green-refactor", wants integration tests, or asks for test-first development.
```

**Dependency**: Soft.

### Philosophy

> **Core principle**: Tests should verify behavior through public interfaces, not implementation details. Code can change entirely; tests shouldn't.

**Good test** (= integration-style):
- 진짜 code path 를 public API 로 exercise
- WHAT (capability), NOT HOW
- spec 처럼 읽힘 — "user can checkout with valid cart"
- internal refactor 살아남음

**Bad test** (= implementation-coupled):
- internal collaborator mock
- private method test
- DB 직접 query 같은 external means 로 verify
- internal function rename 시 깨짐

### Anti-pattern: Horizontal Slices

> **DO NOT write all tests first, then all implementation.**

이건 horizontal slicing — RED 를 "모든 test 작성" 으로, GREEN 을 "모든 코드 작성" 으로 다루는 것.

문제:
- bulk test 는 *imagined* behavior test, *actual* 아님
- 결국 _shape_ test (data structure, function signature), user-facing behavior 아님
- 진짜 변경에 insensitive — behavior 깨졌는데 통과, fine 한데 fail
- 헤드라이트 지나서 commit — implementation 이해하기 전에 test 구조 결정

**올바른**: vertical slice tracer bullet. 한 test → 한 implementation → 반복. 각 test 가 이전 cycle 에서 배운 것에 반응.

```
WRONG (horizontal):
  RED:   test1, test2, test3, test4, test5
  GREEN: impl1, impl2, impl3, impl4, impl5

RIGHT (vertical):
  RED→GREEN: test1→impl1
  RED→GREEN: test2→impl2
  ...
```

### Workflow 4 step

#### 1. Planning
- 도메인 glossary 사용 (test name + interface vocabulary 일치). ADR respect.
- [ ] 사용자에게 interface 변경 확인
- [ ] 사용자에게 어느 behavior test 할지 확인 (priority)
- [ ] deep module (보조 doc `tdd/deep-modules.md`) 기회 식별
- [ ] testability (보조 doc `tdd/interface-design.md`) 위한 interface 디자인
- [ ] behavior list (implementation step 아님)
- [ ] plan 사용자 승인

질문: "What should the public interface look like? Which behaviors are most important to test?"

> **You can't test everything.** critical path + complex logic 에 집중.

#### 2. Tracer Bullet
하나의 behavior 에 대한 ONE test:
```
RED:   첫 behavior test → 실패
GREEN: minimal code → 통과
```

end-to-end path 가 work 함을 증명.

#### 3. Incremental Loop
나머지 각 behavior 에 대해:
```
RED: 다음 test → 실패
GREEN: minimal code → 통과
```

규칙:
- 한 번에 한 test
- 현재 test 만 통과시킬 만큼만 코드
- 미래 test anticipate 금지
- observable behavior 만

#### 4. Refactor
모든 test 통과 후:
- [ ] 중복 추출
- [ ] module 깊게 (complexity 를 simple interface 뒤로)
- [ ] SOLID 자연스러우면 적용
- [ ] 새 코드가 기존 코드에 대해 드러내는 것 고려
- [ ] 각 refactor step 후 test 실행

> **Never refactor while RED.** GREEN 먼저.

### Per-cycle checklist

```
[ ] Test describes behavior, not implementation
[ ] Test uses public interface only
[ ] Test would survive internal refactor
[ ] Code is minimal for this test
[ ] No speculative features added
```

### `tests.md` (보조) — 좋은 vs 나쁜 test 예시

**Good**:
```typescript
test("user can checkout with valid cart", async () => {
  const cart = createCart();
  cart.add(product);
  const result = await checkout(cart, paymentMethod);
  expect(result.status).toBe("confirmed");
});
```

특징: behavior, public API, refactor 살아남음, WHAT not HOW, 1 logical assertion.

**Bad**:
```typescript
test("checkout calls paymentService.process", async () => {
  const mockPayment = jest.mock(paymentService);
  await checkout(cart, payment);
  expect(mockPayment.process).toHaveBeenCalledWith(cart.total);
});
```

red flag: internal mock, private test, call count assert, behavior 변경 없이 깨짐, HOW name, external means.

**더 미묘한 bad → good**:
```typescript
// BAD: interface 우회해서 verify
test("createUser saves to database", async () => {
  await createUser({ name: "Alice" });
  const row = await db.query("SELECT * FROM users WHERE name = ?", ["Alice"]);
  expect(row).toBeDefined();
});

// GOOD: interface 통해 verify
test("createUser makes user retrievable", async () => {
  const user = await createUser({ name: "Alice" });
  const retrieved = await getUser(user.id);
  expect(retrieved.name).toBe("Alice");
});
```

### `mocking.md` (보조) — system boundary 에서만 mock

Mock 함:
- External API (payment, email)
- DB (가끔 — test DB 선호)
- 시간/randomness
- File system (가끔)

Mock 안 함:
- 자기 class/module
- internal collaborator
- 자기가 통제하는 모든 것

**Mockability 위한 디자인**:

1. **Dependency injection**:
```typescript
// Easy
function processPayment(order, paymentClient) {
  return paymentClient.charge(order.total);
}

// Hard
function processPayment(order) {
  const client = new StripeClient(process.env.STRIPE_KEY);
  return client.charge(order.total);
}
```

2. **SDK-style > generic fetcher**:
```typescript
// GOOD: 각 function 독립 mockable
const api = {
  getUser: (id) => fetch(`/users/${id}`),
  getOrders: (userId) => fetch(`/users/${userId}/orders`),
  createOrder: (data) => fetch('/orders', { method: 'POST', body: data }),
};

// BAD: mock setup 안에 conditional 필요
const api = {
  fetch: (endpoint, options) => fetch(endpoint, options),
};
```

SDK 접근: 각 mock 이 한 specific shape 반환, test setup conditional 없음, 어느 endpoint 를 exercise 하는지 보임, endpoint 별 type 안전성.

### `interface-design.md` (보조) — Testability 위한 interface 3 원칙

1. **Accept dependencies, don't create them** — DI.
2. **Return results, don't produce side effects** — `calculateDiscount(cart): Discount` > `applyDiscount(cart): void`.
3. **Small surface area** — fewer methods = fewer test, fewer params = simpler setup.

### `deep-modules.md` (보조)

Ousterhout 의 정의 시각화:
```
┌─────────────────────┐
│   Small Interface   │  ← Few methods, simple params
├─────────────────────┤
│                     │
│  Deep Implementation│  ← Complex logic hidden
│                     │
└─────────────────────┘
```

vs shallow:
```
┌─────────────────────────────────┐
│       Large Interface           │
├─────────────────────────────────┤
│  Thin Implementation            │
└─────────────────────────────────┘
```

interface 디자인 시 묻기:
- method 수 줄일 수 있나?
- param 단순화할 수 있나?
- 더 많은 complexity 안에 숨길 수 있나?

### `refactoring.md` (보조) — TDD cycle 후 후보

- **Duplication** → function/class 추출
- **Long methods** → private helper (test 는 public interface 에 유지)
- **Shallow modules** → 합치거나 deepen
- **Feature envy** → data 사는 곳으로 logic 이동
- **Primitive obsession** → value object
- **Existing code** 새 코드가 problematic 으로 드러낸 것

**관찰**:
- 5개 보조 doc — 모두 짧음 (10-61 lines). progressive disclosure 모범.
- "horizontal slicing" anti-pattern 명시적 거부 — 매트의 분명한 입장.
- system boundary 만 mock 원칙 — own classes/modules 절대 mock 금지.
- SDK-style API 권장이 mock 단순화 메커니즘.
- TDD 와 deep module + interface design 이 단일 skill 에 통합됨 (tightly coupled).

---

## 7. `to-issues` — Plan → vertical slice issue

**파일**: `skills/engineering/to-issues/SKILL.md` (81 lines)

**Frontmatter**:
```yaml
name: to-issues
description: Break a plan, spec, or PRD into independently-grabbable issues
  on the project issue tracker using tracer-bullet vertical slices.
  Use when user wants to convert a plan into issues, create implementation tickets,
  or break down work into issues.
```

**Dependency**: **Hard**. setup pointer 명시 (L10).

### Process 5 step

#### 1. Gather context
대화 컨텍스트에서 작업. 사용자가 issue 참조 (number/URL/path) 전달 시 issue tracker 에서 fetch — full body + comment.

#### 2. Explore the codebase (선택)
이미 안 했으면 도메인 glossary + ADR 인식 탐색.

#### 3. Draft vertical slices

> Each issue is a thin **vertical slice that cuts through ALL integration layers end-to-end**, NOT a horizontal slice of one layer.

slice 종류:
- **HITL** — human interaction 필요 (architectural decision, design review)
- **AFK** — human interaction 없이 implement + merge

AFK 선호.

```
<vertical-slice-rules>
- Each slice delivers a narrow but COMPLETE path through every layer (schema, API, UI, tests)
- A completed slice is demoable or verifiable on its own
- Prefer many thin slices over few thick ones
</vertical-slice-rules>
```

#### 4. Quiz the user

번호 매긴 list. 각 slice:
- **Title**: short
- **Type**: HITL/AFK
- **Blocked by**: 다른 slice (있으면)
- **User stories covered**: 어느 user story (있으면)

질문:
- granularity 적절? (너무 굵음/잘음)
- dependency 관계 맞음?
- 합치거나 split?
- HITL/AFK marking 맞음?

승인까지 iterate.

#### 5. Publish

승인된 slice 마다 publish. body template:

```md
<issue-template>
## Parent
parent issue 참조 (source 가 기존 issue 면, 아니면 omit)

## What to build
이 vertical slice 의 concise 설명. end-to-end behavior 묘사 — layer-by-layer impl 아님.

## Acceptance criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

## Blocked by
- blocking ticket 참조 (있으면)
또는 "None - can start immediately" if 없음.
</issue-template>
```

`needs-triage` triage label 적용 → 정상 triage flow.

dependency 순서로 publish (blocker 먼저) → "Blocked by" 에 real issue ID 참조.

**parent issue 수정/close 금지.**

**관찰**:
- vertical slice 강조 — `tdd` 의 같은 vocabulary.
- AFK 우선 — 자율 agent 가 grab 가능.
- triage flow 강제 진입 — triage skill 과 통합.
- parent 보존 — destructive 안 함.

---

## 8. `to-prd` — 컨텍스트 → PRD

**파일**: `skills/engineering/to-prd/SKILL.md` (74 lines)

**Frontmatter**:
```yaml
name: to-prd
description: Turn the current conversation context into a PRD and publish it
  to the project issue tracker.
  Use when user wants to create a PRD from the current context.
```

**Dependency**: **Hard**. setup pointer 명시 (L8).

### Core directive
> Do NOT interview the user — just synthesize what you already know.

### Process 3 step

1. Repo 탐색 + 도메인 glossary 사용 + ADR respect.
2. **Major module sketch** — 만들거나 수정할 모듈. 적극적으로 isolation 으로 test 가능한 deep module 추출 기회 봄.
   - deep module = simple, testable interface 안에 많은 functionality, 자주 안 변함.
   - 사용자에게 module 이 expectation 일치 확인 + 어느 module 에 test 작성할지 확인.
3. PRD 작성 (template) → issue tracker publish + `needs-triage` 적용.

### PRD template

```md
<prd-template>
## Problem Statement
사용자가 직면한 problem, user perspective.

## Solution
problem 의 solution, user perspective.

## User Stories
LONG numbered list. 각 story:
1. As an <actor>, I want a <feature>, so that <benefit>

<user-story-example>
1. As a mobile bank customer, I want to see balance on my accounts,
   so that I can make better informed decisions about my spending
</user-story-example>

extensive — 모든 측면 cover.

## Implementation Decisions
implementation 결정. 포함:
- 만들거나 수정할 module
- 수정할 module interface
- developer 의 기술 clarification
- 아키텍처 결정
- schema 변경
- API contract
- 구체적 interaction

specific file path / code snippet 금지 — 빨리 stale.

## Testing Decisions
testing 결정. 포함:
- 좋은 test 가 무엇인지 (external behavior 만, implementation detail 아님)
- 어느 module test 할지
- prior art (codebase 의 비슷한 type test)

## Out of Scope

## Further Notes
</prd-template>
```

**관찰**:
- "Don't interview" 가 `grill-me` / `grill-with-docs` 와 차별화. PRD 는 합성, grill 은 발굴.
- file path / code snippet 금지 — durability (AGENT-BRIEF 와 같은 원칙).
- Implementation Decisions 가 architecture 결정 record — ADR 의 sibling.
- Testing Decisions 가 PRD 안에 — test plan 이 first-class.
- triage flow 강제 (`needs-triage`) — to-issues 와 같은 패턴.

---

## 9. `zoom-out` — 한 줄 instruction

**파일**: `skills/engineering/zoom-out/SKILL.md` (7 lines)

**Frontmatter**:
```yaml
name: zoom-out
description: Tell the agent to zoom out and give broader context or a higher-level perspective.
  Use when you're unfamiliar with a section of code or need to understand
  how it fits into the bigger picture.
disable-model-invocation: true
```

**Body** (전체):
> I don't know this area of code well. Go up a layer of abstraction. Give me a map of all the relevant modules and callers, using the project's domain glossary vocabulary.

**Dependency**: Soft (vague reference — "the project's domain glossary").

**관찰**:
- 가장 작은 SKILL — 7 lines.
- `disable-model-invocation: true` — 명시적 사용자 invoke 만.
- repo 의 minimalism 철학을 단일 skill 에 응축. SKILL.md 가 큰 파일 강요 안 함.

---

## Engineering 한 줄 요약

| Skill | 역할 | Hard/Soft | Disable invoke | LOC | 보조 doc |
|---|---|---|---|---|---|
| diagnose | 6단계 진단 | Soft | - | 117 | 1 (script) |
| grill-with-docs | Grilling + doc inline | Soft | ✓ | 81 | 2 |
| triage | 5-role 상태 머신 | Hard | - | 103 | 2 |
| improve-codebase-architecture | Deepening 발굴 | Soft | - | 71 | 3 |
| setup-matt-pocock-skills | 7 consumer 시드 | - (생산자) | ✓ | 119 | 4 |
| tdd | Red-green-refactor vertical | Soft | - | 109 | 5 |
| to-issues | Plan → vertical slice | Hard | - | 81 | - |
| to-prd | 컨텍스트 → PRD | Hard | - | 74 | - |
| zoom-out | 1줄 abstraction shift | Soft | ✓ | 7 | - |

**평균**: SKILL.md ~84 lines, 1.6 보조 doc/skill.
