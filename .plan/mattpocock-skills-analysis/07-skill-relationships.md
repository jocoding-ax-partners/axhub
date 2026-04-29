# 07. Skill Relationships — 의존성 / Hand-off 그래프

22 skill (engineering 9 / productivity 3 / misc 4 / personal 2 / deprecated 4) 가 어떻게 서로 호출 / hand-off / 데이터 공유하는지.

---

## Top-down: 핵심 hub

### `setup-matt-pocock-skills` — 7 consumer 의 단일 producer

```
                  setup-matt-pocock-skills
                      (한 번 실행)
                            │
          시드: docs/agents/{issue-tracker, triage-labels, domain}.md
          + CLAUDE.md/AGENTS.md 의 ## Agent skills 블록
                            │
        ┌──────┬──────┬─────┴────┬──────┬──────┬──────┐
        │      │      │          │      │      │      │
   to-issues to-prd triage  diagnose  tdd  improve  zoom-out
   (Hard)  (Hard) (Hard)   (Soft)  (Soft) (Soft)  (Soft)
```

**Hard dependency** — config 없으면 wrong output.
**Soft dependency** — config 없으면 fuzzy 한 output.

### `grill-with-docs` — CONTEXT.md / ADR producer

```
                  grill-with-docs
                  (사용자 호출)
                        │
              CONTEXT.md, docs/adr/<NN>-*.md
                        │
          ┌─────────────┼─────────────┐
          │             │             │
    diagnose          tdd      improve-codebase-architecture
    (도메인 read)  (도메인 read)  (도메인 read + own write)
                        │
                   to-prd, to-issues, triage, zoom-out
                   (도메인 read 만)
```

다른 skill 은 CONTEXT.md / ADR 을 **읽기만**. `grill-with-docs` 와 `improve-codebase-architecture` 만 새 term / ADR 추가 (inline).

### `triage` — issue lifecycle 의 hub

```
to-issues → triage queue (`needs-triage`)
              │
    ┌─────────┼─────────┬──────────────┐
    │         │         │              │
needs-info  ready-for-agent  ready-for-human  wontfix
   ↓             │              │           │
(reporter      AGENT-BRIEF      brief +    .out-of-scope/
 응답)        (실행 contract)   "왜 사람"   (enhancement)
   ↓
needs-triage 복귀

to-prd → triage queue (`needs-triage`)
        (PRD 도 새 issue 로 등록)
```

`to-issues` / `to-prd` 가 `needs-triage` 적용 → 정상 triage flow 진입. `triage` 가 lifecycle 처리.

---

## Skill 별 의존성 매트릭스

행 = 의존하는 skill (A), 열 = 의존되는 skill (B). A → B 는 "A 가 B 의 출력을 소비하거나, B 의 setup 후 작동" 의미.

| A \ B | setup | grill-with-docs | triage | (다른) |
|---|---|---|---|---|
| **diagnose** | Soft | reads CONTEXT.md/ADR | hand-off when arch finding | → improve-codebase-architecture (Phase 6) |
| **grill-with-docs** | - | (자기 자신) | - | (CONTEXT.md / ADR 생산) |
| **improve-codebase-architecture** | Soft | reads + writes CONTEXT.md/ADR | - | → INTERFACE-DESIGN sub-agent (자기 안) |
| **setup-matt-pocock-skills** | (자기 자신) | - | - | (consumer 7 setup) |
| **tdd** | Soft | reads CONTEXT.md/ADR | - | references deep-modules + interface-design (보조 doc) |
| **to-issues** | Hard | reads CONTEXT.md | writes (new issue, `needs-triage`) | - |
| **to-prd** | Hard | reads CONTEXT.md/ADR | writes (new issue, `needs-triage`) | - |
| **triage** | Hard | hand-off (`/grill-with-docs` Step 4) | (자기 자신) | reads `.out-of-scope/`, writes AGENT-BRIEF |
| **zoom-out** | Soft | reads CONTEXT.md (vague) | - | - |
| **caveman** | - | - | - | 독립 (communication 모드) |
| **grill-me** | - | sibling (subset) | - | 독립 |
| **write-a-skill** | - | - | - | 독립 (skill 작성) |
| **git-guardrails** | - | - | - | 독립 (hook 셋업) |
| **migrate-to-shoehorn** | - | - | - | 독립 (test refactor) |
| **scaffold-exercises** | - | - | - | 독립 (aihero 코스용) |
| **setup-pre-commit** | - | - | - | 독립 (Husky 셋업) |
| **edit-article** (personal) | - | - | - | 독립 |
| **obsidian-vault** (personal) | - | - | - | 독립 |
| **design-an-interface** (deprecated) | - | - | - | → improve-codebase-architecture/INTERFACE-DESIGN |
| **qa** (deprecated) | - | - | - | → triage |
| **request-refactor-plan** (deprecated) | - | - | - | → to-prd + to-issues |
| **ubiquitous-language** (deprecated) | - | - | - | → grill-with-docs/CONTEXT-FORMAT |

---

## Hand-off (명시적)

### Diagnose Phase 6 → improve-codebase-architecture

`diagnose/SKILL.md` L116-117:
> **Then ask: what would have prevented this bug?** If the answer involves architectural change (no good test seam, tangled callers, hidden coupling) hand off to the `/improve-codebase-architecture` skill with the specifics. Make the recommendation **after** the fix is in, not before — you have more information now than when you started.

### improve-codebase-architecture Step 3 → grill-with-docs / INTERFACE-DESIGN

`improve-codebase-architecture/SKILL.md` L67-72:
> - **Naming a deepened module after a concept not in `CONTEXT.md`?** Add the term — same discipline as `/grill-with-docs`.
> - **Sharpening a fuzzy term?** Update `CONTEXT.md` right there.
> - **User rejects the candidate with a load-bearing reason?** Offer an ADR.
> - **Want to explore alternative interfaces?** See `INTERFACE-DESIGN.md`.

### Triage Step 4 → grill-with-docs

`triage/SKILL.md` L70:
> 4. **Grill (if needed).** If the issue needs fleshing out, run a `/grill-with-docs` session.

### Triage Step 5 → `.out-of-scope/`

`triage/SKILL.md` L75-76:
> - `wontfix` (enhancement) — write to `.out-of-scope/`, link to it from a comment, then close.

### to-issues / to-prd → triage

publish 시 `needs-triage` 라벨 적용 → `triage` 가 처리.

`to-issues/SKILL.md` L57:
> Apply the `needs-triage` triage label so each issue enters the normal triage flow.

`to-prd/SKILL.md` L20:
> Apply the `needs-triage` triage label so it enters the normal triage flow.

---

## Trigger 자연어 / 명시적 invoke 매트릭스

| Skill | description trigger | disable-model-invocation |
|---|---|---|
| diagnose | "diagnose this" / "debug this" / bug report / "broken/throwing/failing" / perf regression | - |
| grill-with-docs | "stress-test plan against language and documented decisions" | ✓ |
| triage | "create issue" / "triage" / "review incoming bugs" / "AFK agent" / "issue workflow" | - |
| improve-codebase-architecture | "improve architecture" / "refactoring opportunities" / "consolidate" / "more testable" / "AI-navigable" | - |
| setup-matt-pocock-skills | (description 만) | ✓ |
| tdd | "TDD" / "red-green-refactor" / "integration tests" / "test-first" | - |
| to-issues | "convert plan to issues" / "create implementation tickets" / "break down work" | - |
| to-prd | "create PRD from current context" | - |
| zoom-out | (사용자 명시 invoke 만) | ✓ |
| caveman | "caveman mode" / "less tokens" / "be brief" / `/caveman` | - |
| grill-me | "stress-test plan" / "get grilled on design" / "grill me" | - |
| write-a-skill | "create skill" / "write skill" / "build skill" | - |
| git-guardrails-claude-code | "prevent destructive git" / "git safety hooks" / "block git push/reset" | - |
| migrate-to-shoehorn | "shoehorn" / "replace as in tests" / "partial test data" | - |
| scaffold-exercises | "scaffold exercises" / "exercise stubs" / "new course section" | - |
| setup-pre-commit | "pre-commit hooks" / "Husky" / "lint-staged" / commit-time formatting | - |
| edit-article (personal) | "edit/revise/improve article draft" | - |
| obsidian-vault (personal) | "Obsidian" notes find/create/organize | - |
| design-an-interface (deprecated) | "design API" / "interface options" / "design it twice" | - |
| qa (deprecated) | "report bugs" / "QA" / "file issues conversationally" / "QA session" | - |
| request-refactor-plan (deprecated) | "plan refactor" / "refactoring RFC" / "safe incremental steps" | - |
| ubiquitous-language (deprecated) | "domain terms" / "glossary" / "harden terminology" / "DDD" / "domain model" | ✓ |

---

## File 시스템 발자국

각 skill 이 만들거나 읽는 file:

| File | Producer | Consumer |
|---|---|---|
| `CLAUDE.md` (또는 `AGENTS.md`) `## Agent skills` 블록 | setup-matt-pocock-skills | (모든 engineering skill 이 implicit ref) |
| `docs/agents/issue-tracker.md` | setup-matt-pocock-skills | to-issues, to-prd, triage, qa(dep) |
| `docs/agents/triage-labels.md` | setup-matt-pocock-skills | triage, to-issues, to-prd |
| `docs/agents/domain.md` | setup-matt-pocock-skills | diagnose, tdd, improve-codebase-architecture, zoom-out |
| `CONTEXT.md` (또는 `CONTEXT-MAP.md` + per-context) | grill-with-docs, improve-codebase-architecture | (모든 engineering skill, soft + hard 둘 다) |
| `docs/adr/<NN>-*.md` | grill-with-docs, improve-codebase-architecture | diagnose, tdd, improve-codebase-architecture, to-prd, to-issues, triage |
| `.out-of-scope/<concept>.md` | triage | triage (next time) |
| `.scratch/<feature>/PRD.md` | to-prd (local-markdown tracker) | (사용자, agent) |
| `.scratch/<feature>/issues/<NN>-<slug>.md` | to-issues (local-markdown) | triage (local) |
| GitHub issue (gh CLI) | to-issues, to-prd, qa(dep) | triage (gh tracker) |
| GitHub issue 댓글 | triage | (사용자, reporter) |
| `.husky/pre-commit` | setup-pre-commit | (git) |
| `.lintstagedrc` | setup-pre-commit | lint-staged |
| `.prettierrc` | setup-pre-commit | Prettier |
| `.claude/hooks/block-dangerous-git.sh` | git-guardrails-claude-code | Claude Code (PreToolUse) |
| `.claude/settings.json` (hooks) | git-guardrails-claude-code | Claude Code |
| `~/.claude/skills/<name>/` (symlink) | scripts/link-skills.sh | Claude CLI |
| `UBIQUITOUS_LANGUAGE.md` (deprecated, → CONTEXT.md) | ubiquitous-language(dep) | qa(dep) |

---

## Producer / Consumer 도식 (전체)

```
┌──────────────────────────────────────────────────┐
│ User triggers / invokes via slash command         │
└──────────────────────────────────────────────────┘
                    │
       ┌────────────┼────────────────┬────────────┐
       │            │                │            │
       ▼            ▼                ▼            ▼
  setup-matt   grill-with-docs    triage        write-a-skill
   (한 번)      (CONTEXT.md /     (issue       (새 SKILL 작성)
       │       docs/adr/ 갱신)     lifecycle)
       │            │                │
   docs/agents/     CONTEXT.md   .out-of-scope/
       │            docs/adr/    AGENT-BRIEF
       │            │                ▲
       └─consumed by ──┐             │
                       ▼             │
       ┌──────────┬─────────┬────────┴────┬──────────┐
       │          │         │             │          │
   to-issues  to-prd   diagnose    improve-cb   tdd, zoom-out
   (Hard)    (Hard)    (Soft)      (Soft+생산)  (Soft)
       │          │         │             │
       └────┬─────┘         │             ▼
            │               │      DEEPENING / INTERFACE-DESIGN
            ▼               │      (자기 안 sub-agent 패턴)
       triage queue         │
       (needs-triage)       │
            │               │
            ▼               ▼
       (lifecycle)   architectural finding hand-off
```

---

## 독립 skill (다른 skill 참조 안 함)

- `caveman` — 단일 communication 모드, 다른 skill 무관.
- `git-guardrails-claude-code` — Claude Code hook 셋업, 단독.
- `migrate-to-shoehorn` — 단일 test refactor task.
- `scaffold-exercises` — aihero 코스 도메인 specific.
- `setup-pre-commit` — Husky 셋업, 단독.
- `edit-article` (personal) — 단일 article task.
- `obsidian-vault` (personal) — Obsidian 도메인 specific.

이 7개는 plug-and-play — 다른 skill 없이도 작동.

---

## 그래프 한 줄 요약

> **2 hub** (`setup-matt-pocock-skills` for config / `grill-with-docs` for domain doc) + **1 lifecycle hub** (`triage`) + **명시적 hand-off chain** (diagnose → improve-codebase-architecture / to-issues → triage / to-prd → triage / improve-codebase-architecture → grill-with-docs) + **7 독립 skill**.
