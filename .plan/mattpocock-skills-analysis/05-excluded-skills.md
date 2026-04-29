# 05. Excluded Skills — Personal + Deprecated

`plugin.json` 에서 빠지고 README.md 에서 빠진 6개 skill — `personal/` 2개, `deprecated/` 4개. CONTEXT.md L9 정책상 두 카테고리는 plugin distribution 에서 명시적으로 제외돼요. 그러나 `link-skills.sh` 는 모두 symlink 하므로 매트 본인 + dev 환경에서는 사용 가능.

분석 가치는 두 가지:
1. **Personal** = 매트 개인 setup 의존 — 다른 user 에게는 일반화 안 됨, 패턴은 흡수 가능
2. **Deprecated** = 후계 skill 로 흡수된 것 — evolution 보여줌

---

## Personal

### 1. `edit-article` — Article 편집

**파일**: `skills/personal/edit-article/SKILL.md` (14 lines)

**Frontmatter**:
```yaml
name: edit-article
description: Edit and improve articles by restructuring sections, improving clarity,
  and tightening prose.
  Use when user wants to edit, revise, or improve an article draft.
```

**Body**:
1. Article 을 heading 기준 section 분할. 각 section 의 main point 생각.

> Consider that information is a directed acyclic graph, and that pieces of information can depend on other pieces of information. Make sure that the order of the sections and their contents respects these dependencies.

   사용자에게 section 확인.

2. 각 section:
   2a. 명확성/일관성/흐름 개선. **paragraph 당 max 240 chars.**

**관찰**:
- 매우 간결 — 14 lines.
- "Information as DAG" framing — section 순서가 dependency respect 하도록.
- 240 char/paragraph rule — 가독성 강제.
- 매트의 individual writing voice 에 맞춰져 있어요. 다른 author 에게는 customize 필요.

---

### 2. `obsidian-vault` — Obsidian vault 관리

**파일**: `skills/personal/obsidian-vault/SKILL.md` (59 lines)

**Frontmatter**:
```yaml
name: obsidian-vault
description: Search, create, and manage notes in the Obsidian vault with wikilinks
  and index notes.
  Use when user wants to find, create, or organize notes in Obsidian.
```

### Vault location

`/mnt/d/Obsidian Vault/AI Research/` — Windows + WSL 환경 추정.

flat at root, 폴더로 조직 안 함.

### Naming

- **Index notes**: 관련 토픽 aggregate (`Ralph Wiggum Index.md`, `Skills Index.md`, `RAG Index.md`)
- **Title Case** all
- 폴더 X — link 와 index note 사용

### Linking

- Obsidian wikilink: `[[Note Title]]`
- 의존/관련 note 는 bottom 에 link
- index note = `[[wikilink]]` list

### Workflows

#### Search

```bash
find "/mnt/d/Obsidian Vault/AI Research/" -name "*.md" | grep -i "keyword"
grep -rl "keyword" "/mnt/d/Obsidian Vault/AI Research/" --include="*.md"
```

또는 Grep/Glob tool 직접.

#### Create
1. Title Case filename
2. unit of learning content (vault rule)
3. bottom 에 wikilink
4. 번호 시퀀스면 hierarchical numbering

#### Find related (backlink)

```bash
grep -rl "\\[\\[Note Title\\]\\]" "/mnt/d/Obsidian Vault/AI Research/"
```

#### Find index notes

```bash
find "/mnt/d/Obsidian Vault/AI Research/" -name "*Index*"
```

**관찰**:
- 매트 본인 vault path 가 hardcoded — 다른 user 가 그대로 못 씀.
- 폴더 대신 index note 패턴 — Obsidian convention.
- "Unit of learning" 표현 — Karpathy LLM Wiki 모델과 비슷.
- Title Case 강제 — 일관성.

---

## Deprecated — 후계 skill 로 흡수됨

`skills/deprecated/README.md` 상단:
> Skills I no longer use.

각 deprecated skill 의 후계 매핑 (분석가 추론):

| Deprecated | 후계 (로 흡수됨) |
|---|---|
| `design-an-interface` | `improve-codebase-architecture/INTERFACE-DESIGN.md` |
| `qa` | `triage` (특히 issue 작성 + AGENT-BRIEF) |
| `request-refactor-plan` | `to-prd` + `to-issues` |
| `ubiquitous-language` | `grill-with-docs/CONTEXT-FORMAT.md` |

### 1. `design-an-interface`

**파일**: `skills/deprecated/design-an-interface/SKILL.md` (94 lines)

**Frontmatter**:
```yaml
name: design-an-interface
description: Generate multiple radically different interface designs for a module
  using parallel sub-agents. Use when user wants to design an API,
  explore interface options, compare module shapes, or mentions "design it twice".
```

핵심 아이디어 (Ousterhout "Design It Twice"):
- 첫 아이디어가 best 인 경우 드물어요.
- 3+ sub-agent 병렬, 각자 radically different 한 design.
- 비교 후 합성.

**Workflow**:
1. **Gather requirements** — module 의 problem / caller / 핵심 op / 제약 / hide vs expose.
2. **Generate (parallel sub-agents)** — Task tool 로 3+ 동시.
   - Agent 1: "Minimize method count - 1-3 max"
   - Agent 2: "Maximize flexibility"
   - Agent 3: "Optimize most common case"
   - Agent 4: "[specific paradigm/library inspiration]"
3. **Present** — interface signature / usage example / what hides / trade-offs (한 design 씩 순차).
4. **Compare** — interface simplicity / general vs specialized / impl efficiency / depth / ease of correct use vs misuse. prose, 표 X.
5. **Synthesize** — 흔히 best = 여러 option 합성.

**Evaluation criteria** (Ousterhout):
- Interface simplicity
- General-purpose
- Implementation efficiency
- Depth (small interface + significant complexity)

**Anti-patterns**:
- sub-agent 가 비슷한 design 생성 — radical difference 강제
- 비교 skip — 가치는 contrast
- implementation 안 함 — interface shape only
- impl effort 로 평가 X

**왜 deprecated** — 후계 `improve-codebase-architecture/INTERFACE-DESIGN.md` 가 같은 패턴을 더 정교하게:
- Frame the problem space (사용자 흡수 시간 + sub-agent 병렬 일)
- 4 종류 constraint (ports & adapters 추가)
- LANGUAGE.md + CONTEXT.md vocabulary 명시
- Trade-off 출력 형식 정형화 (5 항목)
- 권장 hybrid

### 2. `qa`

**파일**: `skills/deprecated/qa/SKILL.md` (130 lines)

**Frontmatter**:
```yaml
name: qa
description: Interactive QA session where user reports bugs or issues conversationally,
  and the agent files GitHub issues. Explores the codebase in the background for
  context and domain language.
  Use when user wants to report bugs, do QA, file issues conversationally, or mentions "QA session".
```

**핵심 아이디어**: 사용자가 conversational 로 problem 묘사 → agent 가 명료화 + 코드 탐색 (background) + GitHub issue 작성.

**Process**:

#### 1. Listen + lightly clarify
사용자 plain 묘사. **2-3 short clarifying question 만**:
- 기대 vs 실제
- repro step (obvious 아니면)
- consistent vs intermittent

over-interview 금지.

#### 2. Explore in background
Agent (subagent_type=Explore) 백그라운드 — fix 찾는 게 아니고:
- 도메인 언어 학습 (UBIQUITOUS_LANGUAGE.md)
- feature 가 무엇 해야 하는지 이해
- user-facing behavior boundary

issue 자체는 file/line/internal 참조 안 함.

#### 3. Single vs breakdown
Breakdown 시점:
- 여러 독립 영역
- 분리 가능 concern
- 여러 별개 failure mode

Single 유지:
- 한 behavior, 한 곳
- 모든 symptom 이 같은 root behavior

#### 4. File issue(s)

durable, user perspective.

**Single template**:
```
## What happened
## What I expected
## Steps to reproduce
1. ...
## Additional context
```

**Breakdown template**:
```
## Parent issue
## What's wrong
## What I expected
## Steps to reproduce
## Blocked by
- #N (또는 "None — can start immediately")
## Additional context
```

dependency 순서로 publish.

#### Rules:
- file path / line number 금지 (stale)
- 도메인 언어 (UBIQUITOUS_LANGUAGE.md)
- behavior 묘사, 코드 X
- repro step 필수
- 30초 readable

**왜 deprecated** — 후계 `triage` 가 같은 작업 + 5-role 상태 머신 + AGENT-BRIEF + `.out-of-scope/` 통합. `qa` 는 issue 작성만, `triage` 는 lifecycle 전체.

### 3. `request-refactor-plan`

**파일**: `skills/deprecated/request-refactor-plan/SKILL.md` (68 lines)

**Frontmatter**:
```yaml
name: request-refactor-plan
description: Create a detailed refactor plan with tiny commits via user interview,
  then file it as a GitHub issue.
  Use when user wants to plan a refactor, create a refactoring RFC,
  or break a refactor into safe incremental steps.
```

**Process** (8 step, skip 가능):

1. 사용자에게 long detailed problem + solution idea.
2. Repo 탐색 — assertion verify.
3. 다른 option 고려했나? present alternatives.
4. **Detailed thorough interview** about implementation.
5. Scope 망치질 — 무엇 변경 / 무엇 안 변경.
6. **Test coverage 확인** — 부족하면 사용자 plan 묻기.
7. Tiny commit plan — Martin Fowler:
   > "make each refactoring step as small as possible, so that you can always see the program working."
8. GitHub issue 작성:
   ```
   ## Problem Statement (developer perspective)
   ## Solution (developer perspective)
   ## Commits (LONG detailed, 각 commit 이 working state)
   ## Decision Document (module / interface / clarification / arch / schema / API / interaction)
   ## Testing Decisions (좋은 test 정의 / module / prior art)
   ## Out of Scope
   ## Further Notes (optional)
   ```

**왜 deprecated** — 후계 `to-prd` (PRD 작성) + `to-issues` (vertical slice) 가 같은 일을 더 정교하게:
- `to-prd` 는 인터뷰 안 함 (현재 컨텍스트 합성). `request-refactor-plan` 은 detailed interview — `grill-me` / `grill-with-docs` 쪽으로 분리.
- `to-issues` 는 vertical slice, `request-refactor-plan` 은 tiny commit (다른 axis). vertical slice 가 더 기본.
- "Decision Document" + "Testing Decisions" 은 `to-prd` 의 "Implementation Decisions" + "Testing Decisions" 로 fold.

### 4. `ubiquitous-language`

**파일**: `skills/deprecated/ubiquitous-language/SKILL.md` (93 lines)

**Frontmatter**:
```yaml
name: ubiquitous-language
description: Extract a DDD-style ubiquitous language glossary from the current conversation,
  flagging ambiguities and proposing canonical terms. Saves to UBIQUITOUS_LANGUAGE.md.
  Use when user wants to define domain terms, build a glossary, harden terminology,
  create a ubiquitous language, or mentions "domain model" or "DDD".
disable-model-invocation: true
```

**Process**:
1. 대화 scan — 도메인 noun/verb/concept.
2. **Identify problems**:
   - 같은 단어 → 다른 개념 (ambiguity)
   - 다른 단어 → 같은 개념 (synonym)
   - vague / overload term
3. opinionated canonical glossary propose.
4. **`UBIQUITOUS_LANGUAGE.md`** 작성 (cwd).
5. 대화 inline summary.

**Output Format**:
```md
# Ubiquitous Language

## Order lifecycle
| Term      | Definition                          | Aliases to avoid     |
| **Order** | A customer's request to purchase    | Purchase, transaction|

## People
| **Customer** | A person/org placing orders         | Client, buyer, account|

## Relationships
- An **Invoice** belongs to exactly one **Customer**

## Example dialogue
> **Dev:** "When a **Customer** places an **Order**..."
> **Domain expert:** ...

## Flagged ambiguities
- "account" was used to mean both **Customer** and **User** — distinct.
```

**Rules**:
- Be opinionated
- Flag conflicts explicitly
- 도메인 expert 관련 term 만
- Tight definitions (1 sentence max)
- Show relationships (bold + cardinality)
- 도메인 specific 만 (generic programming 제외)
- Multiple table 자연스러우면
- Example dialogue 작성

**Re-running**:
1. 기존 file 읽기
2. 새 term 합치기
3. 정의 evolve
4. ambiguity re-flag
5. dialogue 재작성

**왜 deprecated** — 후계 `grill-with-docs/CONTEXT-FORMAT.md` 로 흡수:
- `UBIQUITOUS_LANGUAGE.md` 파일 이름이 `CONTEXT.md` 로 변경 — 더 짧음, 멀티 context (`CONTEXT-MAP.md`) 지원.
- 표 형식 → bold prose 형식 (`**Term**: definition / _Avoid_: ...`).
- standalone glossary 추출 → grilling session inline 갱신 (즉시 capture, batch X).
- 별도 skill → 다른 skill 의 process 일부 (lazy creation 패턴).

**관찰**:
- 모든 deprecated 가 후계 skill 의 부분으로 흡수됨 — 단순히 삭제가 아닌 evolution.
- 매트가 의도적으로 deprecated 보존 — 학습 자료 + 아이디어 archive.
- 이전 vocabulary 유지: `qa` 의 `UBIQUITOUS_LANGUAGE.md` 참조는 stale (지금은 CONTEXT.md).

---

## Excluded 한 줄 요약

| Skill | Bucket | 운명 | LOC |
|---|---|---|---|
| edit-article | personal | 매트 개인 article 작성 | 14 |
| obsidian-vault | personal | 매트 개인 vault | 59 |
| design-an-interface | deprecated | → improve-codebase-architecture/INTERFACE-DESIGN | 94 |
| qa | deprecated | → triage (lifecycle 전체) | 130 |
| request-refactor-plan | deprecated | → to-prd + to-issues | 68 |
| ubiquitous-language | deprecated | → grill-with-docs/CONTEXT-FORMAT | 93 |

총 6 skill, 458 lines. plugin distribution 에서 빠지지만 repo 안에 archive — evolution + 학습 자료.
