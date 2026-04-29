# 00. Overview — mattpocock/skills 레포 한눈에

## Repo 정체성

> "My agent skills that I use every day to do real engineering — not vibe coding." (README.md L13)

매트 포콕 (Matt Pocock, Total TypeScript / aihero.dev) 의 일상 엔지니어링용 agent skill 모음 이에요. Claude Code 의 `~/.claude/skills/` 에 심볼릭 링크로 설치되는 plugin 형태고, **GSD / BMAD / Spec-Kit** 같은 무거운 process owner 와 정반대 철학 — 작고 (small), 적응 가능하고 (easy to adapt), 조합 가능한 (composable) skill 들이에요.

전체 코퍼스가 작아요: 53 파일 / ~3036 lines / 22 SKILL.md. 의도적으로 가볍게 유지되어 있어요.

## 해결하려는 4가지 Agent 실패 모드

README.md 가 이 repo 의 존재 이유를 4개 failure mode 로 정리해요. 모든 skill 은 이 4개 중 하나를 직접 attack 해요.

### #1. "Agent Didn't Do What I Want" — 정렬 실패

> "No-one knows exactly what they want" — *The Pragmatic Programmer*

**Fix**: **grilling** — agent 가 사용자 plan 의 모든 가지를 끈질기게 질문해요.
- `/grill-me` (비코드 plan)
- `/grill-with-docs` (코드 plan + CONTEXT.md/ADR 동시 갱신)

매트가 "가장 인기 있는 skill" 이라 부르는 것들이고, "every time you want to make a change" 사용 권장.

### #2. "Agent Is Way Too Verbose" — 공통 언어 부재

> "With a ubiquitous language, conversations among developers and expressions of the code are all derived from the same domain model." — Eric Evans, *DDD*

Agent 는 도메인 jargon 을 모르니까 1 단어 대신 20 단어로 빙 돌아 설명해요.

**Fix**: `CONTEXT.md` — repo 의 도메인 용어집. `/grill-with-docs` 가 이걸 inline 으로 갱신해요. 매트의 표현: "It might be the single coolest technique in this repo."

부수 효과:
- 변수 / 함수 / 파일 이름이 공유 언어로 일관됨
- 코드베이스 navigation 향상
- agent 가 사고 토큰을 덜 씀 (간결한 언어 = 적은 토큰)

### #3. "The Code Doesn't Work" — 피드백 루프 부재

> "Always take small, deliberate steps. The rate of feedback is your speed limit." — *The Pragmatic Programmer*

**Fix**: 빠르고 결정적인 피드백 루프. red-green-refactor 가 핵심.
- `/tdd` — 한 번에 vertical slice 한 개씩 빨강→초록→리팩토
- `/diagnose` — 어려운 버그용 6단계 루프 (reproduce / minimise / hypothesise / instrument / fix / regression-test)

### #4. "We Built a Ball Of Mud" — 설계 무관심

> "Invest in the design of the system *every day*." — Kent Beck
>
> "The best modules are deep." — John Ousterhout

Agent 는 코딩 속도를 키워서 software entropy 도 같이 키워요.

**Fix**: 매일 설계에 신경 쓰기.
- `/to-prd` — PRD 만들기 전 어느 module 을 건드릴지 quiz
- `/zoom-out` — agent 가 코드를 시스템 전체 맥락에서 설명
- `/improve-codebase-architecture` — ball of mud 가 된 codebase 구조 회복 (몇 일에 한 번 권장)

## 22 Skill 한눈에 (bucket 별)

### Engineering — 코드 작업용 일상 도구 (9)

| Skill | 한 줄 |
|---|---|
| `diagnose` | 6단계 disciplined 진단 루프 (reproduce → minimise → hypothesise → instrument → fix → regression-test). Phase 1 (피드백 루프 구축) 이 핵심. |
| `grill-with-docs` | grilling 세션 + CONTEXT.md/ADR inline 갱신. 도메인 모델에 plan 을 stress-test. |
| `triage` | 5개 triage role 상태 머신 (`needs-triage` / `needs-info` / `ready-for-agent` / `ready-for-human` / `wontfix`) 으로 issue 처리. AGENT-BRIEF 작성 도움. |
| `improve-codebase-architecture` | "Deepening opportunity" 발굴 — shallow module 을 deep 으로. CONTEXT.md/ADR 인지하면서. |
| `setup-matt-pocock-skills` | 다른 7개 engineering skill 이 의존하는 per-repo config (issue tracker / triage label / domain doc 레이아웃) 시드. |
| `tdd` | red-green-refactor 한 번에 한 vertical slice. horizontal slicing 안티-패턴 금지. |
| `to-issues` | plan/PRD 를 independently-grabbable issue (vertical slice / tracer bullet) 로 분해. |
| `to-prd` | 현재 대화 컨텍스트를 PRD 로 합성 + issue tracker 에 publish. 인터뷰 안 함. |
| `zoom-out` | 단 1 줄 instruction — agent 에게 한 단계 추상화 올라가서 module + caller 지도 그리라. |

### Productivity — 비코드 일상 도구 (3)

| Skill | 한 줄 |
|---|---|
| `caveman` | 토큰 75% 절감하는 ultra-compressed 모드. fragment 허용, 기술 용어 그대로. |
| `grill-me` | `/grill-with-docs` 의 비코드 버전 — 단순 grilling. |
| `write-a-skill` | 새 skill 작성 가이드 (frontmatter / progressive disclosure / 100줄 제한 / 시작/구조/리뷰 체크리스트). |

### Misc — 가끔 쓰는 도구 (4)

| Skill | 한 줄 |
|---|---|
| `git-guardrails-claude-code` | Claude Code PreToolUse hook 으로 위험한 git 명령 (`push` / `reset --hard` / `clean -f` / `branch -D` / `checkout .`) 차단. bash script 번들. |
| `migrate-to-shoehorn` | 테스트 파일에서 `as Type` 을 `@total-typescript/shoehorn` 의 `fromPartial()` / `fromAny()` 로 마이그레이션. |
| `scaffold-exercises` | exercise directory 구조 scaffold + `pnpm ai-hero-cli internal lint` 통과시키기. |
| `setup-pre-commit` | Husky + lint-staged + Prettier + typecheck/test 의 pre-commit 훅 셋업. |

### Personal — 매트 개인용, plugin 에서 제외 (2)

`edit-article`, `obsidian-vault` — 개인 article 작성 / Obsidian vault (`/mnt/d/Obsidian Vault/AI Research/`) 관리.

### Deprecated — 더 이상 안 쓰는 것, plugin 에서 제외 (4)

`design-an-interface`, `qa`, `request-refactor-plan`, `ubiquitous-language` — 후계 skill (`improve-codebase-architecture`/`INTERFACE-DESIGN.md`, `triage`, `to-issues`, `grill-with-docs`) 로 흡수됨.

## 작동 모델 한 문장

> 사용자가 `npx skills@latest add mattpocock/skills` 로 설치 → `/setup-matt-pocock-skills` 한 번 실행 → 7개 engineering skill 이 그 repo 의 issue tracker / label / domain doc 위치를 알게 됨 → 일상 작업에서 `/grill-with-docs` / `/tdd` / `/diagnose` / `/triage` / `/to-prd` / `/to-issues` / `/improve-codebase-architecture` / `/zoom-out` 호출.

## 핵심 idea (다음 doc 으로 넘어가기 전)

1. **작은 SKILL.md, 큰 보조 doc** — `setup-matt-pocock-skills/SKILL.md` 119 lines, 보조 4개 (domain.md / triage-labels.md / issue-tracker-{github,local}.md). progressive disclosure.
2. **disable-model-invocation: true** — 일부 skill (`grill-with-docs`, `setup-matt-pocock-skills`, `zoom-out`, `ubiquitous-language`) 은 명시적 사용자 invoke 만 허용. agent 자동 발동 안 됨.
3. **Lazy file creation** — `CONTEXT.md` / `docs/adr/` / `.out-of-scope/` 모두 처음 필요할 때 만들어요. up-front 강제 안 함.
4. **Hard vs soft dependency 의식적 분리** — ADR-0001 에 명시. `to-issues` / `to-prd` / `triage` 는 setup 없으면 잘못된 출력 (label 문자열 모름). `diagnose` / `tdd` / `improve-codebase-architecture` / `zoom-out` 은 graceful degrade.
5. **Vertical slice = tracer bullet** — `to-issues`, `tdd` 모두에서 사용. horizontal slicing (모든 test 먼저, 모든 impl 다음) 명시적 anti-pattern.

상세는 다음 문서들로 이어져요.
