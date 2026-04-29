# 08. Reference — Format / Template / Source Mapping

reference 자료를 한 곳에 모음. format spec + 53 파일 source mapping.

---

## SKILL.md Frontmatter Format

YAML frontmatter:

```yaml
---
name: skill-slug                         # 필수, kebab-case
description: |                            # 필수, max 1024 chars, third person
  무엇을 함. Use when [specific triggers].
disable-model-invocation: true            # 선택, 명시적 사용자 invoke 만 허용
---
```

### 관찰 분포 (22 SKILL)

- 모든 22 SKILL 이 `name` + `description` 가짐.
- 4 SKILL 이 `disable-model-invocation: true`: `grill-with-docs`, `setup-matt-pocock-skills`, `zoom-out`, `ubiquitous-language` (deprecated).
- 다른 frontmatter key 없음 — minimal.

### Description 규칙 (`write-a-skill/SKILL.md`)

- max 1024 chars
- third person
- 첫 문장: 무엇을 함
- 둘째 문장: "Use when [specific triggers]"

이유: agent 가 description 만 보고 trigger 판단.

---

## SKILL.md 본문 Template (`write-a-skill/SKILL.md` 권장)

```md
---
name: skill-name
description: Brief description of capability. Use when [specific triggers].
---

# Skill Name

## Quick start
[Minimal working example]

## Workflows
[Step-by-step processes with checklists for complex tasks]

## Advanced features
[Link to separate files: See [REFERENCE.md](REFERENCE.md)]
```

### 100-line rule
SKILL.md 100 lines 초과 시 별도 파일로 split.

(관찰: 본인 `write-a-skill/SKILL.md` 가 117 lines — 자가 모순.)

### Reference one level deep
nested reference (REFERENCE.md → SUB-REFERENCE.md) 금지.

---

## ADR Format (`grill-with-docs/ADR-FORMAT.md`)

### Path
`docs/adr/<NNNN>-<slug>.md` — 4-digit zero-padded sequential.

### Minimal Template
```md
# {Short title}

{1-3 sentences: what's the context, what did we decide, and why.}
```

### Optional sections (genuine value 있을 때만)
- `Status` frontmatter (`proposed | accepted | deprecated | superseded by ADR-NNNN`)
- `Considered Options` — 거절된 alternative 가 기억할 가치 있을 때
- `Consequences` — non-obvious downstream effect

### When to offer (3 모두 true)
1. **Hard to reverse**
2. **Surprising without context**
3. **Result of real trade-off**

### What qualifies
- Architectural shape (monorepo, event-sourced)
- Context 간 통합 패턴 (domain event vs sync HTTP)
- Lock-in 있는 기술 선택 (DB, message bus, auth, deploy target)
- Boundary / scope (Customer 데이터 owner)
- 의도적 obvious-path 이탈 (manual SQL instead of ORM)
- 코드에 안 보이는 제약 (compliance, partner SLA)
- non-obvious 한 거절된 대안 (GraphQL 고려 후 REST)

### Numbering
`docs/adr/` scan 후 max 숫자 + 1.

### Lazy creation
`docs/adr/` 디렉토리는 첫 ADR 필요할 때만 생성.

---

## CONTEXT.md Format (`grill-with-docs/CONTEXT-FORMAT.md`)

### Single-context structure

```md
# {Context Name}

{One or two sentence description of what this context is and why it exists.}

## Language

**Order**:
{Concise description of the term}
_Avoid_: Purchase, transaction

**Invoice**:
A request for payment sent to a customer after delivery.
_Avoid_: Bill, payment request

**Customer**:
A person or organization that places orders.
_Avoid_: Client, buyer, account

## Relationships

- An **Order** produces one or more **Invoices**
- An **Invoice** belongs to exactly one **Customer**

## Example dialogue

> **Dev:** "When a **Customer** places an **Order**, do we create the **Invoice** immediately?"
> **Domain expert:** "No — an **Invoice** is only generated once a **Fulfillment** is confirmed."

## Flagged ambiguities

- "account" was used to mean both **Customer** and **User** — resolved: distinct concepts.
```

### Rules
- **Be opinionated** — 동의어 best 1 + Avoid list
- **Flag conflicts** — Flagged ambiguities section
- **Tight 1-sentence definition** (IS, NOT does)
- **Bold term name** + cardinality
- 도메인 specific only — generic programming (timeout, error type) 제외
- 자연 cluster subheading 으로 — single cohesive 면 flat OK
- **Example dialogue** 필수 — dev × 도메인 expert

### Multi-context (CONTEXT-MAP.md)

```md
# Context Map

## Contexts

- [Ordering](./src/ordering/CONTEXT.md) — receives and tracks customer orders
- [Billing](./src/billing/CONTEXT.md) — generates invoices and processes payments
- [Fulfillment](./src/fulfillment/CONTEXT.md) — manages warehouse picking and shipping

## Relationships

- **Ordering → Fulfillment**: Ordering emits `OrderPlaced` events; Fulfillment consumes them
- **Fulfillment → Billing**: Fulfillment emits `ShipmentDispatched` events; Billing consumes
- **Ordering ↔ Billing**: Shared types for `CustomerId` and `Money`
```

---

## Agent Brief Template (`triage/AGENT-BRIEF.md`)

```md
## Agent Brief

**Category:** bug / enhancement
**Summary:** one-line description of what needs to happen

**Current behavior:**
Describe what happens now. For bugs, this is the broken behavior.
For enhancements, this is the status quo the feature builds on.

**Desired behavior:**
Describe what should happen after the agent's work is complete.
Be specific about edge cases and error conditions.

**Key interfaces:**
- `TypeName` — what needs to change and why
- `functionName()` return type — what it currently returns vs what it should return
- Config shape — any new configuration options needed

**Acceptance criteria:**
- [ ] Specific, testable criterion 1
- [ ] Specific, testable criterion 2
- [ ] Specific, testable criterion 3

**Out of scope:**
- Thing that should NOT be changed or addressed in this issue
- Adjacent feature that might seem related but is separate
```

### 원칙
- **Durability over precision** — file path / line number 금지
- **Behavioral, not procedural** — WHAT, NOT HOW
- **Complete acceptance criteria** — 각 독립 verifiable
- **Explicit scope boundaries** — gold-plating 방지

---

## Issue Template (`to-issues/SKILL.md`)

vertical slice issue body:

```md
## Parent

A reference to the parent issue on the issue tracker (if the source was an
existing issue, otherwise omit this section).

## What to build

A concise description of this vertical slice. Describe the end-to-end
behavior, not layer-by-layer implementation.

## Acceptance criteria

- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

## Blocked by

- A reference to the blocking ticket (if any)

Or "None - can start immediately" if no blockers.
```

triage label `needs-triage` 적용 (모든 새 issue).

---

## PRD Template (`to-prd/SKILL.md`)

```md
## Problem Statement

The problem that the user is facing, from the user's perspective.

## Solution

The solution to the problem, from the user's perspective.

## User Stories

A LONG, numbered list of user stories. Each:
1. As an <actor>, I want a <feature>, so that <benefit>

## Implementation Decisions

A list of implementation decisions. Can include:
- The modules that will be built/modified
- The interfaces of those modules
- Technical clarifications from the developer
- Architectural decisions
- Schema changes
- API contracts
- Specific interactions

Do NOT include specific file paths or code snippets.

## Testing Decisions

- A description of what makes a good test (only test external behavior)
- Which modules will be tested
- Prior art for the tests

## Out of Scope

## Further Notes
```

---

## `.out-of-scope/<concept>.md` Template (`triage/OUT-OF-SCOPE.md`)

```md
# Concept Name

This project does not support {concept}.

## Why this is out of scope

{Substantive reason — project scope/philosophy, technical constraint, or strategic decision.
Avoid temporary circumstances ("we're too busy") — those are deferrals not rejections.}

```code-sample
// Optional: code samples / examples
```

## Prior requests

- #42 — "..."
- #87 — "..."
- #134 — "..."
```

---

## Triage Notes Template (`triage/SKILL.md`)

```md
## Triage Notes

**What we've established so far:**
- point 1
- point 2

**What we still need from you (@reporter):**
- question 1
- question 2
```

Grilling 중 resolved 된 모두 "established" 에 — 작업 손실 X. 질문은 specific + actionable.

---

## Triage Roles (canonical)

**Category** (한 개):
- `bug`
- `enhancement`

**State** (한 개):
- `needs-triage` — maintainer 평가 필요
- `needs-info` — reporter 응답 대기
- `ready-for-agent` — fully specified, AFK-ready
- `ready-for-human` — human implementation 필요
- `wontfix` — actioned 안 함

### State transitions
- 라벨 없음 → `needs-triage`
- `needs-triage` → 어디든
- `needs-info` → `needs-triage` (reporter 응답 시)
- maintainer override 가능 (비정상 시 flag)

### Mandatory disclaimer (모든 댓글/issue 첫 줄)
```
> *This was generated by AI during triage.*
```

---

## AI-generated 구분 (Mandatory disclaimers)

이 repo 는 한 가지 강제 disclaimer 만 가져요:

| Skill | Disclaimer | Where |
|---|---|---|
| triage | `> *This was generated by AI during triage.*` | 모든 issue 댓글 + 새 issue 첫 줄 |

다른 skill 은 disclaimer 강제 안 함.

---

## Source Mapping — 53 파일 전체 인벤토리

| # | Path | Lines | Type | Key Fact |
|---|---|---:|---|---|
| 1 | `.claude-plugin/plugin.json` | 17 | Plugin manifest | 12 skill 등록 (engineering 9 + productivity 3) |
| 2 | `.out-of-scope/question-limits.md` | 18 | Out-of-scope | "200 questions" 거절 메모 (#44) — grilling cap 안 만듦 |
| 3 | `CLAUDE.md` | 13 | Doc | repo bucket 정책 explanation |
| 4 | `CONTEXT.md` | 26 | Domain glossary | Issue tracker / Issue / Triage role |
| 5 | `LICENSE` | 22 | Legal | MIT 2026 |
| 6 | `README.md` | 172 | Doc | 4 failure mode framing + 22 skill 카탈로그 |
| 7 | `docs/adr/0001-explicit-setup-pointer-only-for-hard-dependencies.md` | 10 | ADR | hard vs soft dependency 명시 split |
| 8 | `scripts/link-skills.sh` | 38 | Bash | 모든 SKILL.md → `~/.claude/skills/<name>/` symlink |
| 9 | `skills/deprecated/README.md` | 8 | Bucket README | 4 deprecated 명시 |
| 10 | `skills/deprecated/design-an-interface/SKILL.md` | 94 | SKILL (deprecated) | "Design It Twice" parallel sub-agent — → improve-codebase-architecture |
| 11 | `skills/deprecated/qa/SKILL.md` | 130 | SKILL (deprecated) | conversational issue 작성 — → triage |
| 12 | `skills/deprecated/request-refactor-plan/SKILL.md` | 68 | SKILL (deprecated) | tiny commit refactor plan — → to-prd + to-issues |
| 13 | `skills/deprecated/ubiquitous-language/SKILL.md` | 93 | SKILL (deprecated) | DDD glossary 추출 — → grill-with-docs/CONTEXT-FORMAT |
| 14 | `skills/engineering/README.md` | 13 | Bucket README | 9 engineering skill |
| 15 | `skills/engineering/diagnose/SKILL.md` | 117 | SKILL | 6단계 진단, Phase 1 (피드백 루프) 핵심 |
| 16 | `skills/engineering/diagnose/scripts/hitl-loop.template.sh` | 41 | Bash | HITL repro template (`step` / `capture` 헬퍼) |
| 17 | `skills/engineering/grill-with-docs/ADR-FORMAT.md` | 47 | Format spec | ADR 1-3 sentence minimal |
| 18 | `skills/engineering/grill-with-docs/CONTEXT-FORMAT.md` | 77 | Format spec | CONTEXT.md / CONTEXT-MAP.md 구조 |
| 19 | `skills/engineering/grill-with-docs/SKILL.md` | 81 | SKILL | grilling + inline doc update, `disable-model-invocation` |
| 20 | `skills/engineering/improve-codebase-architecture/DEEPENING.md` | 37 | Process spec | 4 dependency category 별 처리 |
| 21 | `skills/engineering/improve-codebase-architecture/INTERFACE-DESIGN.md` | 44 | Process spec | 3+ sub-agent 병렬 design |
| 22 | `skills/engineering/improve-codebase-architecture/LANGUAGE.md` | 53 | Glossary | module / interface / depth / seam / adapter / leverage / locality |
| 23 | `skills/engineering/improve-codebase-architecture/SKILL.md` | 71 | SKILL | deepening 발굴 + grilling loop |
| 24 | `skills/engineering/setup-matt-pocock-skills/SKILL.md` | 119 | SKILL | 7 consumer 시드, `disable-model-invocation` |
| 25 | `skills/engineering/setup-matt-pocock-skills/domain.md` | 51 | Doc template | domain doc 소비 규칙 |
| 26 | `skills/engineering/setup-matt-pocock-skills/issue-tracker-github.md` | 22 | Doc template | gh CLI 규약 |
| 27 | `skills/engineering/setup-matt-pocock-skills/issue-tracker-local.md` | 19 | Doc template | `.scratch/` 마크다운 |
| 28 | `skills/engineering/setup-matt-pocock-skills/triage-labels.md` | 15 | Doc template | 5 role × 2 column 매핑 표 |
| 29 | `skills/engineering/tdd/SKILL.md` | 109 | SKILL | red-green-refactor vertical slice, horizontal 안티 |
| 30 | `skills/engineering/tdd/deep-modules.md` | 33 | Concept | small interface + deep impl ASCII |
| 31 | `skills/engineering/tdd/interface-design.md` | 31 | Concept | DI / return-not-side-effect / small surface |
| 32 | `skills/engineering/tdd/mocking.md` | 59 | Concept | boundary 만 mock + SDK-style API |
| 33 | `skills/engineering/tdd/refactoring.md` | 10 | Concept | TDD cycle 후 후보 |
| 34 | `skills/engineering/tdd/tests.md` | 61 | Concept | good vs bad test 예시 |
| 35 | `skills/engineering/to-issues/SKILL.md` | 81 | SKILL | plan → vertical slice issue, Hard dep |
| 36 | `skills/engineering/to-prd/SKILL.md` | 74 | SKILL | 컨텍스트 → PRD, 인터뷰 안 함, Hard dep |
| 37 | `skills/engineering/triage/AGENT-BRIEF.md` | 168 | Doc spec | brief 작성 규칙 + 좋/나쁜 예시 |
| 38 | `skills/engineering/triage/OUT-OF-SCOPE.md` | 101 | Doc spec | `.out-of-scope/` knowledge base |
| 39 | `skills/engineering/triage/SKILL.md` | 103 | SKILL | 5-role 상태 머신, AI disclaimer 강제, Hard dep |
| 40 | `skills/engineering/zoom-out/SKILL.md` | 7 | SKILL | 한 줄 instruction, `disable-model-invocation` |
| 41 | `skills/misc/README.md` | 8 | Bucket README | 4 misc skill |
| 42 | `skills/misc/git-guardrails-claude-code/SKILL.md` | 95 | SKILL | PreToolUse hook 셋업 |
| 43 | `skills/misc/git-guardrails-claude-code/scripts/block-dangerous-git.sh` | 25 | Bash | jq + grep + exit 2 hook 본체 |
| 44 | `skills/misc/migrate-to-shoehorn/SKILL.md` | 118 | SKILL | test `as` → fromPartial / fromAny |
| 45 | `skills/misc/scaffold-exercises/SKILL.md` | 106 | SKILL | aihero 코스 디렉토리 scaffold |
| 46 | `skills/misc/setup-pre-commit/SKILL.md` | 91 | SKILL | Husky + lint-staged + Prettier 셋업 |
| 47 | `skills/personal/README.md` | 6 | Bucket README | 2 personal skill |
| 48 | `skills/personal/edit-article/SKILL.md` | 14 | SKILL (personal) | section 별 240 char/paragraph |
| 49 | `skills/personal/obsidian-vault/SKILL.md` | 59 | SKILL (personal) | Obsidian wikilink + index note |
| 50 | `skills/productivity/README.md` | 7 | Bucket README | 3 productivity skill |
| 51 | `skills/productivity/caveman/SKILL.md` | 49 | SKILL | 토큰 75% 절감 모드 |
| 52 | `skills/productivity/grill-me/SKILL.md` | 10 | SKILL | grill-with-docs subset (비코드) |
| 53 | `skills/productivity/write-a-skill/SKILL.md` | 117 | SKILL | 새 skill 작성 가이드 |

**총 합계**: 53 파일 / 3036 lines.

---

## External Citations (ralph 재검토 추가)

mattpocock/skills 가 인용하는 외부 자료. 분석 doc 에 흩어져 있던 것을 한 곳에 모음.

### 책 / Author

| 자료 | Author | 어디 인용 | 어떤 idea |
|---|---|---|---|
| *The Pragmatic Programmer* | David Thomas & Andrew Hunt | `README.md` failure mode #1, #3 | "no one knows exactly what they want" / "rate of feedback is your speed limit" |
| *Domain-Driven Design* | Eric Evans | `README.md` failure mode #2 | ubiquitous language |
| *A Philosophy of Software Design* | John Ousterhout | `README.md` failure mode #4, `tdd/deep-modules.md`, `improve-codebase-architecture/INTERFACE-DESIGN.md`, `LANGUAGE.md` | deep modules / "Design It Twice" |
| *Extreme Programming Explained* | Kent Beck | `README.md` failure mode #4 | "Invest in the design of the system every day" |
| *(unspecified Refactoring book)* | Martin Fowler | `deprecated/request-refactor-plan/SKILL.md` | "make each refactoring step as small as possible" |

### Concept / Author 만 인용

| Concept | Origin | 어디 |
|---|---|---|
| **Seam** | Michael Feathers (*Working Effectively with Legacy Code*) | `improve-codebase-architecture/LANGUAGE.md` L21 |
| Tracer bullets | (Pragmatic Programmer 에서) | `to-issues/SKILL.md`, `tdd/SKILL.md` |
| Vertical slices | XP / lean | `to-issues/SKILL.md` |
| RED-GREEN-REFACTOR | Kent Beck TDD | `tdd/SKILL.md` |
| Ports & Adapters / Hexagonal | Alistair Cockburn (anonymous 인용) | `improve-codebase-architecture/DEEPENING.md` 카테고리 #3 |

### URL / 외부 link

| Link | 용도 |
|---|---|
| https://www.aihero.dev/s/skills-newsletter | 매트의 newsletter (60k 구독자 — `README.md`) |
| https://github.com/mattpocock/course-video-manager/.../CONTEXT.md | `CONTEXT.md` 예시 (`README.md` BEFORE/AFTER 인용) |
| https://res.cloudinary.com/total-typescript/... | repo header image (light/dark variant) |
| https://www.amazon.co.uk/Pragmatic-Programmer-... | 책 link (4종) |

### `npx skills@latest add mattpocock/skills` (외부 tool)

`README.md` quickstart 의 명령. 이 repo 에 implementation 없음 — 별도 npm 패키지 (skills.sh installer). 분석 범위 밖.

### `@total-typescript/shoehorn` (매트 본인 라이브러리)

`migrate-to-shoehorn` skill 이 사용. `fromPartial()` / `fromAny()` / `fromExact()` 함수 제공. test code only.

### `pnpm ai-hero-cli internal lint` (매트 본인 도구)

`scaffold-exercises` skill 이 통과 대상으로 사용. 별도 도구 — 분석 범위 밖.

---

## Cross-reference Index

### 모든 22 SKILL 위치
- engineering 9: 15, 19, 23, 24, 29, 35, 36, 39, 40 (이 표의 # 열)
- productivity 3: 51, 52, 53
- misc 4: 42, 44, 45, 46
- personal 2: 48, 49
- deprecated 4: 10, 11, 12, 13

### 모든 보조 doc (SKILL 의 자식 file)
- `diagnose`: 16 (script)
- `grill-with-docs`: 17, 18 (format)
- `improve-codebase-architecture`: 20, 21, 22 (process / glossary)
- `setup-matt-pocock-skills`: 25, 26, 27, 28 (template)
- `tdd`: 30, 31, 32, 33, 34 (concept)
- `triage`: 37, 38 (spec)
- `git-guardrails-claude-code`: 43 (script)

### 모든 README
- 5 bucket README: 9, 14, 41, 47, 50
- 1 top-level: 6

### 모든 ADR / Out-of-scope
- ADR: 7 (`docs/adr/0001-...md`)
- Out-of-scope: 2 (`.out-of-scope/question-limits.md`)

### 모든 script
- 8 (`scripts/link-skills.sh`)
- 16 (`hitl-loop.template.sh`)
- 43 (`block-dangerous-git.sh`)

### 모든 manifest / config
- 1 (`plugin.json`)
- 5 (`LICENSE`)
- 3 (`CLAUDE.md`)
- 4 (`CONTEXT.md`)

→ 53/53 cover.
