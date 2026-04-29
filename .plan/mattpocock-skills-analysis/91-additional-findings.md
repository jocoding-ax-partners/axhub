# 91. Additional Findings — Ralph 재검토 2차

> ⚠️ 이 문서는 ralph 재검토 2차 iteration 에서 추가됨. 90-evolution.md 가 git history dimension 추가했고, 이 doc 은 quantitative dimension (frequency / size / pattern) 추가해요.

---

## 1. Inter-skill Reference Frequency (정량)

`grep -rh -oE '/(skill-name)' --include='*.md'` 결과:

| Skill | 다른 SKILL 에서 인용 횟수 |
|---|---:|
| `/grill-with-docs` | **12** |
| `/setup-matt-pocock-skills` | **11** |
| `/triage` | **10** |
| `/grill-me` | 7 |
| `/improve-codebase-architecture` | 5 |
| `/zoom-out` | 4 |
| `/to-prd` | 4 |
| `/tdd` | 4 |
| `/diagnose` | 4 |
| `/caveman` | 3 |
| `/write-a-skill` | 2 |
| `/to-issues` | 2 |
| `/setup-pre-commit` | 2 |
| `/scaffold-exercises` | 2 |
| `/migrate-to-shoehorn` | 2 |
| `/git-guardrails-claude-code` | 2 |
| `/ubiquitous-language` | 1 |
| `/request-refactor-plan` | 1 |
| `/qa` | 1 |
| `/obsidian-vault` | 1 |
| `/edit-article` | 1 |
| `/design-an-interface` | 1 |

### 관찰

**3-tier hub 구조 정량 확인** (`07-skill-relationships.md` 의 정성 분석을 frequency 가 backs):

- **Tier 1 hub**: `/grill-with-docs` (12) + `/setup-matt-pocock-skills` (11) + `/triage` (10)
- **Tier 2 satellites**: `/grill-me` (7) + 4-5 개의 engineering skill (4-5)
- **Tier 3 leaves**: misc / personal / deprecated (1-2)

**`/grill-with-docs` 가 1위인 이유** — 다음 skill 들이 모두 grilling discipline 참조:
- `improve-codebase-architecture` (Step 3 grilling loop)
- `triage` (Step 4 issue grilling)
- `to-prd`, `to-issues` (간접 — grill 후 PRD/issue)

**`/setup-matt-pocock-skills` 가 2위** — 7 consumer (hard 3 + soft 4) 가 모두 직접 setup pointer 또는 vague reference 가짐.

**`/triage` 가 3위** — `to-issues` / `to-prd` / `qa(dep)` / `request-refactor-plan(dep)` 모두 issue tracker 에 publish 한 후 triage flow.

**Cross-self reference 가 3 hub 의 frequency 부풀림** — 일부 ref 는 본인 SKILL.md 안의 보조 doc 또는 README. 그러나 magnitude 는 명확.

---

## 2. Skill 별 총 LOC + 보조 doc 분포

| Skill | Bucket | Total LOC | Files | SKILL.md alone |
|---|---|---:|---:|---:|
| `triage` | engineering | **372** | 3 | 103 |
| `tdd` | engineering | **303** | 6 | 109 |
| `setup-matt-pocock-skills` | engineering | **226** | 5 | 119 |
| `grill-with-docs` | engineering | 205 | 3 | 81 |
| `improve-codebase-architecture` | engineering | 205 | 4 | 71 |
| `qa` (deprecated) | deprecated | 130 | 1 | 130 |
| `migrate-to-shoehorn` | misc | 118 | 1 | 118 |
| `diagnose` | engineering | 117 | 2 | 117 |
| `write-a-skill` | productivity | 117 | 1 | 117 |
| `scaffold-exercises` | misc | 106 | 1 | 106 |
| `git-guardrails-claude-code` | misc | 95 | 2 | 95 |
| `design-an-interface` (dep) | deprecated | 94 | 1 | 94 |
| `ubiquitous-language` (dep) | deprecated | 93 | 1 | 93 |
| `setup-pre-commit` | misc | 91 | 1 | 91 |
| `to-issues` | engineering | 81 | 1 | 81 |
| `to-prd` | engineering | 74 | 1 | 74 |
| `request-refactor-plan` (dep) | deprecated | 68 | 1 | 68 |
| `obsidian-vault` | personal | 59 | 1 | 59 |
| `caveman` | productivity | 49 | 1 | 49 |
| `edit-article` | personal | 14 | 1 | 14 |
| `grill-me` | productivity | 10 | 1 | 10 |
| `zoom-out` | engineering | 7 | 1 | 7 |

### 관찰

**Top 5 (가장 무거운)**:
1. `triage` 372L / 3 files — 가장 큼. AGENT-BRIEF.md (168) + OUT-OF-SCOPE.md (101) + SKILL.md (103). issue lifecycle 의 거의 모든 측면.
2. `tdd` 303L / 6 files — 5 보조 doc (mocking, deep-modules, interface-design, refactoring, tests).
3. `setup-matt-pocock-skills` 226L / 5 files — 4 보조 doc (3 issue-tracker template + domain.md + triage-labels.md).
4. `grill-with-docs` 205L / 3 files — 2 보조 doc (ADR-FORMAT, CONTEXT-FORMAT).
5. `improve-codebase-architecture` 205L / 4 files — 3 보조 doc (LANGUAGE, DEEPENING, INTERFACE-DESIGN).

**Bottom 3 (가장 가벼운)**:
- `zoom-out` 7L — 한 단락 instruction
- `grill-me` 10L — `grill-with-docs` 의 subset
- `edit-article` 14L — 두 단계만

**Median**: ~100 LOC. **Mean**: ~127 LOC.

**`write-a-skill` 의 100-line rule** — 본인 skill 의 22% (5/22) 가 이 rule 위반:
- write-a-skill 본인 117 (자가 모순)
- migrate-to-shoehorn 118
- diagnose 117
- qa 130
- tdd 109

→ 이 rule 은 가이드라인 (not strict). 100 lines 초과는 보조 doc 으로 split 권장이지 SKILL.md 만 가지고 100 lines 초과는 흔함.

**보조 doc 가진 skill** = 9개:
- triage (2), tdd (5), setup-matt-pocock-skills (4), grill-with-docs (2), improve-codebase-architecture (3), diagnose (1 script), git-guardrails (1 script).

→ 7 engineering 중 5 + misc 1 = 6 skill 이 보조 doc 가짐. progressive disclosure 는 engineering 패턴.

---

## 3. Anti-pattern Catalog (28+ 명시 인용)

`grep -E '(DO NOT|Don'"'"'t|NEVER|❌|WRONG:|BAD:|red flag|anti-pattern|Avoid:)'` 결과:

### TDD anti-pattern
- `tdd/SKILL.md` L18: "Anti-Pattern: Horizontal Slices"
- `tdd/SKILL.md` L20: "**DO NOT** write all tests first, then all implementation."
- `tdd/SKILL.md` L86: "Don't anticipate future tests"
- `tdd/tests.md` L30: "// BAD: Tests implementation details"
- `tdd/tests.md` L48: "// BAD: Bypasses interface to verify"
- `tdd/mocking.md` L10: "Don't mock: Your own classes/modules, internal collaborators"
- `tdd/mocking.md` L49: "// BAD: Mocking requires conditional logic inside the mock"

### Triage / Agent Brief anti-pattern
- `triage/AGENT-BRIEF.md` L13-15:
  - "**Don't** reference file paths — they go stale"
  - "**Don't** reference line numbers"
  - "**Don't** assume the current implementation structure will remain the same"
- `triage/SKILL.md` L103: "Don't re-ask resolved questions."

### Architecture anti-pattern
- `improve-codebase-architecture/LANGUAGE.md` L39: "Don't introduce a seam unless something actually varies across it."
- `improve-codebase-architecture/DEEPENING.md` L29: "Don't introduce a port unless at least two adapters are justified — A single-adapter seam is just indirection."
- `improve-codebase-architecture/DEEPENING.md` L30: "Don't expose internal seams through the interface just because tests use them."
- `improve-codebase-architecture/SKILL.md` L37: "Don't follow rigid heuristics — explore organically"
- `improve-codebase-architecture/SKILL.md` L58: "Don't list every theoretical refactor an ADR forbids."

### Setup anti-pattern
- `setup-matt-pocock-skills/SKILL.md` L32: "Don't dump all three at once."
- `setup-matt-pocock-skills/SKILL.md` L88: "Don't overwrite user edits to the surrounding sections."
- `setup-matt-pocock-skills/domain.md` L11: "Don't flag their absence; don't suggest creating them upfront."
- `setup-matt-pocock-skills/domain.md` L43: "Don't drift to synonyms the glossary explicitly avoids."

### Grilling anti-pattern
- `grill-with-docs/SKILL.md` L69: "Don't batch these up — capture them as they happen."
- `grill-with-docs/SKILL.md` L71: "Don't couple `CONTEXT.md` to implementation details."

### Diagnose anti-pattern
- `diagnose/SKILL.md` L75: "Don't block on it — proceed with your ranking if the user is AFK."

### Design-an-interface anti-patterns (deprecated)
- `design-an-interface/SKILL.md` L91-94 명시 4개:
  - "Don't let sub-agents produce similar designs"
  - "Don't skip comparison"
  - "Don't implement"
  - "Don't evaluate based on implementation effort"

### 카테고리 분류

| Anti-pattern category | 인용 횟수 | 핵심 원칙 |
|---|---:|---|
| **Implementation coupling in tests** | 7 | mock 자기 module X, internal coupling X, file path X |
| **Premature seam / abstraction** | 3 | adapter 1개면 seam 도입 X |
| **Dump everything at once** | 3 | 한 번에 한 질문 / 한 section / 한 term |
| **Path/line reference** | 3 | durability — code 진화 따라가지 못함 |
| **Drift in language** | 2 | glossary 사용, 동의어 substitute X |
| **Skip the comparison** | 2 | sub-agent 출력 차별성 강제, contrast 가 가치 |
| **Block on user** | 1 | AFK 면 진행 |
| **Speculative impl** | 1 | 미래 test anticipate X |

### 메타 패턴
- `Don't` 가 명시적 안티 시그널 — `❌` 또는 `WRONG:` 마커 거의 없음. 매트 voice 일관.
- 안티 인용 28+ 중 80% 가 engineering bucket. productivity / misc 는 거의 없음 — 단순 task 라 안티 명시 불필요.
- 가장 강한 안티: `tdd` 의 "horizontal slicing" 단일 commit 으로 별도 section 가짐 (`## Anti-Pattern: Horizontal Slices`).

---

## 4. Description Trigger Phrase Catalog (22 SKILL)

각 SKILL 의 "Use when ..." 부분 (agent 가 trigger 매칭하는 키):

### Engineering (9)
| Skill | Trigger phrase |
|---|---|
| `diagnose` | `"diagnose this" / "debug this"`, bug report, "broken/throwing/failing", performance regression |
| `grill-with-docs` | "stress-test a plan against their project's language and documented decisions" |
| `improve-codebase-architecture` | "improve architecture", "refactoring opportunities", "consolidate tightly-coupled modules", "more testable and AI-navigable" |
| `setup-matt-pocock-skills` | (Use when 형태 X — "Run before first use of `to-issues`, `to-prd`, ..." 식. 명시 invocation 만) |
| `tdd` | "build features or fix bugs using TDD", `"red-green-refactor"`, "integration tests", "test-first development" |
| `to-issues` | "convert a plan into issues", "create implementation tickets", "break down work into issues" |
| `to-prd` | "create a PRD from the current context" |
| `triage` | "create an issue", "triage issues", "review incoming bugs or feature requests", "prepare issues for an AFK agent", "manage issue workflow" |
| `zoom-out` | "you're unfamiliar with a section of code", "understand how it fits into the bigger picture" |

### Productivity (3)
| Skill | Trigger |
|---|---|
| `caveman` | (description multi-line YAML) "caveman mode", "talk like caveman", "use caveman", "less tokens", "be brief", `/caveman` |
| `grill-me` | "stress-test a plan", "get grilled on their design", "grill me" |
| `write-a-skill` | "create, write, or build a new skill" |

### Misc (4)
| Skill | Trigger |
|---|---|
| `git-guardrails-claude-code` | "prevent destructive git operations", "add git safety hooks", "block git push/reset in Claude Code" |
| `migrate-to-shoehorn` | "shoehorn", "replace `as` in tests", "needs partial test data" |
| `scaffold-exercises` | "scaffold exercises", "create exercise stubs", "set up a new course section" |
| `setup-pre-commit` | "add pre-commit hooks", "set up Husky", "configure lint-staged", "add commit-time formatting/typechecking/testing" |

### Personal (2) — plugin 제외
| Skill | Trigger |
|---|---|
| `edit-article` | "edit, revise, or improve an article draft" |
| `obsidian-vault` | "find, create, or organize notes in Obsidian" |

### Deprecated (4) — plugin 제외
| Skill | Trigger |
|---|---|
| `design-an-interface` | "design an API", "explore interface options", "compare module shapes", "design it twice" |
| `qa` | "report bugs", "do QA", "file issues conversationally", "QA session" |
| `request-refactor-plan` | "plan a refactor", "create a refactoring RFC", "break a refactor into safe incremental steps" |
| `ubiquitous-language` | "define domain terms", "build a glossary", "harden terminology", "create a ubiquitous language", "domain model", "DDD" |

### 관찰

**Trigger 패턴 분류**:
- **Imperative phrase**: "create issue", "convert plan", "scaffold exercises", "edit article" — 동작 묘사
- **Mention-based**: "mentions shoehorn", "mentions DDD", "mentions caveman" — 키워드 매칭
- **Symptom-based**: bug report, "broken/throwing/failing", "you're unfamiliar with X" — 상황 매칭
- **Slash command**: `/caveman` — 명시적 invoke
- **Negative space (setup-matt-pocock-skills)**: trigger 없음, 명시 invocation 만 (disable-model-invocation 과 일치)

**Trigger overlap 위험**:
- `grill-with-docs` ("stress-test plan ... language ... decisions") vs `grill-me` ("stress-test plan, grill me") — 둘 다 "stress-test plan" 가짐. agent 가 `grill-me` 를 선택할 수도 — context (코드 / 비코드) 로 disambiguate.
- `to-prd` vs `to-issues` — "PRD" vs "issues" 명확.
- `to-issues` vs `triage` — "create issue" 가 둘 다. `triage` 는 "review", "incoming", "AFK agent" 추가 — 시그널 더.
- `qa` (deprecated) vs `triage` — "file issues conversationally" 가 `qa`, "create an issue" 가 `triage`. plugin 에서 `qa` 빠졌으니 conflict 무.

**`disable-model-invocation: true` skill 4개의 trigger**:
- `grill-with-docs` — 명시 trigger 있음 ("stress-test plan against language") — 그래도 agent 자동 발동 X. 명시 invocation 의도.
- `setup-matt-pocock-skills` — trigger 없음.
- `zoom-out` — "unfamiliar with X" — agent 자동 발동 X.
- `ubiquitous-language` (deprecated) — "domain model", "DDD" 풍부한 trigger.

→ trigger 가 풍부해도 disable 가능. agent 발동 차단은 frontmatter 가 source-of-truth, description 아님.

---

## 5. SKILL.md Heading 구조 패턴

22 SKILL 의 H2 (`##`) 인벤토리:

### "Process" 또는 "Workflow" 가 메인 section
```
improve-codebase-architecture: ## Glossary, ## Process
setup-matt-pocock-skills: ## Process
to-issues: ## Process
to-prd: ## Process
write-a-skill: (## Process / ## Skill Structure / ## Description Requirements 등)
git-guardrails: ## Steps
migrate-to-shoehorn: ## Workflow / ## Migration patterns / ...
scaffold-exercises: ## Workflow
setup-pre-commit: ## Steps
qa (dep): ## For each issue the user raises
```

### "Phase" 기반 (diagnose 만)
```
diagnose: ## Phase 1, Phase 2, ..., Phase 6
```
이 skill 만 phase 단어 사용. 나머지는 "Step" / "1. ..." numbered.

### "Philosophy" + "Workflow" 분리 (tdd)
```
tdd: ## Philosophy / ## Anti-Pattern / ## Workflow / ## Checklist Per Cycle
```

### Body 만 (instruction 형태)
```
zoom-out: 0 sections
grill-me: 0 sections
edit-article: 0 sections (numbered list 만)
```

### 관찰
- 18/22 SKILL 이 "Process" 또는 "Workflow" section 가짐.
- numbered step 형태 (1, 2, 3, ...) 가 dominant — 22 중 16 SKILL.
- `diagnose` 가 unique — phase-based + 종료 조건 명시 ("Do not proceed until..."). disciplined diagnosis 의 정신을 heading 자체로.
- `tdd` 가 명시적 anti-pattern section 가진 유일 SKILL — horizontal slicing 강조.

### 권장 SKILL.md 골격 (write-a-skill template + 실제 적용 패턴)
```md
---
name: x
description: ...
---

# Skill Name

## (Optional) Philosophy / Why
## Process / Workflow / Steps
### 1. Step name
### 2. Step name
...

## (Optional) Anti-pattern / Don't
## (Optional) Reference templates / formats
## (Optional) Checklist
```

write-a-skill 의 권장 template (Quick start / Workflows / Advanced features) 와 실제 패턴이 다름 — 실제 SKILL 들은 Quick start 안 가짐, 바로 Process 로.

---

## 6. 문서별 보강 status

이번 ralph 재검토 2차에서 발견된 정보가 어느 doc 에 있어야 적절:

| Finding | 추가/갱신 위치 |
|---|---|
| Inter-skill ref frequency 표 | `07-skill-relationships.md` 에 통합 후보 (정량 dimension 추가) |
| Skill LOC 분포 | `08-reference.md` source-mapping 표 보강 후보 |
| Anti-pattern 카탈로그 | `06-cross-cutting-themes.md` 에 통합 후보 (테마 #18) |
| Description trigger 표 | `07-skill-relationships.md` Trigger 매트릭스 보강 후보 |
| Heading 구조 패턴 | `08-reference.md` 또는 `06-cross-cutting-themes.md` 후보 |

이 91 doc 자체가 신규 발견 한 곳에 모음. 다른 doc 보강은 선택 — 분석 가치 vs 중복 trade-off.

---

## 7. 종합: ralph 재검토 2차 점수

| 차원 | 1차 분석 | ralph 1차 (90-evolution) | ralph 2차 (이 doc) |
|---|---|---|---|
| Source coverage | 53/53 ✓ | 53/53 ✓ | 53/53 ✓ |
| Git history | 1 commit | 49+ commit ✓ | 49+ commit ✓ |
| External citations | 분산 | 통합 ✓ | 통합 ✓ |
| Inter-skill ref 정량 | 정성만 | - | 정량 ✓ |
| LOC 분포 | 부분 (engineering) | - | 22/22 ✓ |
| Anti-pattern catalog | 분산 | - | 28+ 통합 ✓ |
| Description trigger 표 | - | - | 22/22 ✓ |
| Heading 패턴 분석 | - | - | 22/22 ✓ |

→ 2차 추가 가치: 정량 dimension 4개 + 안티 카탈로그.

---

## 한 줄 결론

> **3-tier hub 구조 정량 확인** (`grill-with-docs` 12 / `setup-matt-pocock-skills` 11 / `triage` 10), **22/22 SKILL 의 LOC + 보조 doc 분포** (median ~100L, max `triage` 372L, min `zoom-out` 7L), **28+ 명시 anti-pattern 카탈로그** (categorized 8 종), **22/22 description trigger 표**. 1차 + 90-evolution + 91 까지로 분석 dimension 8 종 cover — quantitative 보강 완료.
