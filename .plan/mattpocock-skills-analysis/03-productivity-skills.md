# 03. Productivity Skills — 3개 상세 분석

비코드 일상 도구. caveman / grill-me / write-a-skill.

---

## 1. `caveman` — 토큰 75% 절감 ultra-compressed 모드

**파일**: `skills/productivity/caveman/SKILL.md` (49 lines)

**Frontmatter**:
```yaml
name: caveman
description: >
  Ultra-compressed communication mode. Cuts token usage ~75% by dropping
  filler, articles, and pleasantries while keeping full technical accuracy.
  Use when user says "caveman mode", "talk like caveman", "use caveman",
  "less tokens", "be brief", or invokes /caveman.
```

**Body**:
> Respond terse like smart caveman. All technical substance stay. Only fluff die.

### Persistence

> ACTIVE EVERY RESPONSE once triggered. No revert after many turns. No filler drift. Still active if unsure. Off only when user says "stop caveman" or "normal mode".

— 상태가 끈질김. 여러 turn 후에도 drift 안 함. 대화 처음 trigger 됐으면 명시적 off 까지 유지.

### Rules

Drop:
- articles (a/an/the)
- filler (just/really/basically/actually/simply)
- pleasantries (sure/certainly/of course/happy to)
- hedging
- 접속사 strip
- 흔한 abbreviation 사용 (DB/auth/config/req/res/fn/impl)
- causality 화살표 (X → Y)
- 한 단어로 충분하면 한 단어

Keep:
- 기술 용어 정확 그대로
- 코드 블록 변경 없이
- error 정확히 인용
- fragment OK

Pattern: `[thing] [action] [reason]. [next step].`

대조:
> Not: "Sure! I'd be happy to help you with that. The issue you're experiencing is likely caused by..."
> Yes: "Bug in auth middleware. Token expiry check use `<` not `<=`. Fix:"

### Examples

**Q: "Why React component re-render?"**
> Inline obj prop -> new ref -> re-render. `useMemo`.

**Q: "Explain database connection pooling."**
> Pool = reuse DB conn. Skip handshake -> fast under load.

### Auto-Clarity Exception

다음 경우 일시 caveman drop:
- security warning
- 비가역 action 확인
- multi-step 시퀀스 (fragment 순서 misread 위험)
- 사용자가 clarify 요청 / 질문 반복

명확한 부분 끝나면 caveman resume.

예시 (destructive op):
```
> **Warning:** This will permanently delete all rows in the `users` table and cannot be undone.
> ```sql
> DROP TABLE users;
> ```
> Caveman resume. Verify backup exist first.
```

**관찰**:
- `disable-model-invocation` 없음 — 사용자 자연어 trigger 작동.
- description 의 trigger phrase 가 매우 broad ("caveman mode" / "less tokens" / "be brief") → 의도치 않은 활성화 위험. 그러나 거부 trigger ("stop caveman") 명확.
- "smart caveman" 강조 — terse 하지만 정확. 잘못된 단순화 X.
- security/destructive 시 자동 clarity 회복 — safety guardrail.

---

## 2. `grill-me` — 비코드 grilling

**파일**: `skills/productivity/grill-me/SKILL.md` (10 lines)

**Frontmatter**:
```yaml
name: grill-me
description: Interview the user relentlessly about a plan or design until reaching
  shared understanding, resolving each branch of the decision tree.
  Use when user wants to stress-test a plan, get grilled on their design,
  or mentions "grill me".
```

**Body** (전체):
> Interview me relentlessly about every aspect of this plan until we reach a shared understanding. Walk down each branch of the design tree, resolving dependencies between decisions one-by-one. For each question, provide your recommended answer.
>
> Ask the questions one at a time.
>
> If a question can be answered by exploring the codebase, explore the codebase instead.

**관찰**:
- `grill-with-docs` 의 첫 paragraph + 일부 — 코드 / 도메인 awareness 빠진 버전.
- 둘 다 "Ask the questions one at a time" + "explore the codebase 가능하면 explore" 가 핵심.
- `grill-with-docs` = grill-me + 도메인 awareness + inline doc update.
- `disable-model-invocation` 없음 — 사용자 invoke 자유.

---

## 3. `write-a-skill` — 새 skill 작성 가이드

**파일**: `skills/productivity/write-a-skill/SKILL.md` (117 lines)

**Frontmatter**:
```yaml
name: write-a-skill
description: Create new agent skills with proper structure, progressive disclosure,
  and bundled resources.
  Use when user wants to create, write, or build a new skill.
```

### Process 3 step

1. **Gather requirements** — 사용자 인터뷰:
   - 무슨 task / 도메인?
   - 구체적 use case?
   - 실행 가능 script 필요? 또는 instruction 만?
   - 참고 자료?

2. **Draft the skill**:
   - SKILL.md (concise)
   - Reference file (500 lines 초과 시)
   - Utility script (deterministic op 필요 시)

3. **Review with user**:
   - use case cover?
   - 빠진 거 / 불명확?
   - 더/덜 detail 어디?

### Skill structure

```
skill-name/
├── SKILL.md           # Main (필수)
├── REFERENCE.md       # Detailed docs (필요 시)
├── EXAMPLES.md        # Usage examples (필요 시)
└── scripts/           # Utility scripts (필요 시)
    └── helper.js
```

### SKILL.md template

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

### Description requirements (가장 중요한 부분)

> The description is **the only thing your agent sees** when deciding which skill to load.

agent 가 다른 모든 설치된 skill 의 description 을 보고 trigger 판단. 그래서 description 이 critical.

**목표**: agent 에게 충분한 정보:
1. 무슨 capability
2. 언제/왜 trigger (specific keyword, context, file type)

**Format**:
- Max 1024 chars
- Third person
- 첫 문장: 무엇을 함
- 둘째 문장: "Use when [specific triggers]"

**Good**:
> Extract text and tables from PDF files, fill forms, merge documents. Use when working with PDF files or when user mentions PDFs, forms, or document extraction.

**Bad**:
> Helps with documents.

bad example 는 agent 가 다른 document skill 과 구분 못 함.

### When to add scripts

Utility script 추가:
- deterministic op (validation, formatting)
- 같은 코드가 반복 생성될 텐데
- error 가 명시적 처리 필요

script 가 token 절약 + 신뢰성 개선 (생성된 코드보다).

### When to split files

별도 파일로 split:
- SKILL.md 100 lines 초과 (✗ 본인 SKILL 117 lines)
- 별개 도메인 (finance vs sales schema)
- advanced feature 가 드물게 필요

### Review checklist

```
[ ] Description includes triggers ("Use when...")
[ ] SKILL.md under 100 lines
[ ] No time-sensitive info
[ ] Consistent terminology
[ ] Concrete examples included
[ ] References one level deep
```

**관찰**:
- progressive disclosure 강조 — "References one level deep" — nested reference X.
- 본인 SKILL.md 가 117 lines 으로 자기 100-line rule 초과 ✗ — 자가 모순 감지.
- description 강조가 합리적 — agent 의 skill 선택은 description 만 봄.
- script 권장 시점 명확 — deterministic + 반복 + error 명시 필요.

---

## Productivity 한 줄 요약

| Skill | 역할 | Disable invoke | LOC |
|---|---|---|---|
| caveman | 토큰 절감 모드 | - | 49 |
| grill-me | 비코드 grilling | - | 10 |
| write-a-skill | skill 작성 가이드 | - | 117 |

3개 skill 모두 `disable-model-invocation` 없어서 자연어로 trigger. 셋 다 코드 외 도구 — 일반 워크플로우 에이드.
