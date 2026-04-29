# mattpocock/skills 레포 빠짐없는 분석

> Source commit: `b56795b` (HEAD, depth=1 clone @ 2026-04-29)
> Source: https://github.com/mattpocock/skills.git
> 분석 대상: 53 파일, 22 SKILL.md, ~3036 lines

## 분석 목적

`/oh-my-claudecode:ralplan` 컨센서스 루프를 통해 mattpocock/skills 레포의 모든 소스를 빠짐없이 파악하고, axhub repo 작업자 관점에서 적용 가능한 패턴/안티패턴을 정리해요. mattpocock/skills 자체가 axhub 처럼 "Claude Code 용 skill 모음 plugin" 이므로 비교 학습 가치가 커요.

## 문서 구성

| 파일 | 무엇을 다루나 |
|---|---|
| [00-overview.md](./00-overview.md) | Repo 철학, 해결하려는 4가지 실패 모드, 22 skill 한눈에 |
| [01-architecture.md](./01-architecture.md) | Bucket 분류 모델, plugin.json 스코프, link-skills.sh 동작, hard/soft dependency 분리 (ADR-0001), CONTEXT.md/ADR 멀티-컨텍스트 레이아웃 |
| [02-engineering-skills.md](./02-engineering-skills.md) | 9개 engineering skill 의 process / glossary / dependencies / side effects 상세 |
| [03-productivity-skills.md](./03-productivity-skills.md) | caveman / grill-me / write-a-skill 상세 |
| [04-misc-skills.md](./04-misc-skills.md) | git-guardrails-claude-code / migrate-to-shoehorn / scaffold-exercises / setup-pre-commit |
| [05-excluded-skills.md](./05-excluded-skills.md) | personal (2) + deprecated (4) skill 들과 plugin.json 에서 제외된 이유 |
| [06-cross-cutting-themes.md](./06-cross-cutting-themes.md) | Vertical slices, deep modules, ubiquitous language, durable-over-procedural, lazy doc creation, Design It Twice, grilling, 시스템-바운더리에서만 mock 등 반복 패턴 |
| [07-skill-relationships.md](./07-skill-relationships.md) | Skill 간 의존성 그래프 — setup → 7 consumer / grill-with-docs ↔ improve-codebase-architecture / triage → agent brief / to-issues → triage 등 |
| [08-reference.md](./08-reference.md) | Frontmatter / SKILL.md / ADR / CONTEXT.md / Agent Brief / Issue body / PRD / Triage label / source-mapping (53 파일 표) |
| [90-evolution.md](./90-evolution.md) | **(ralph 재검토 1차)** Git history 50+ commit 기반 skill 진화 timeline — 사라진 skill 5개 (`prd-to-plan`, `write-a-prd`, `github-triage`, `triage-issue`, `domain-model`), rename 매트릭스, bucket migration 시점, emergent design 패턴 |
| [91-additional-findings.md](./91-additional-findings.md) | **(ralph 재검토 2차)** 정량 dimension — inter-skill ref frequency 표 (3-tier hub 정량 확인), 22/22 LOC 분포, 28+ anti-pattern 카탈로그 (8 카테고리), description trigger 표, SKILL.md heading 구조 패턴 |
| [99-axhub-takeaways.md](./99-axhub-takeaways.md) | axhub repo 에 그대로 적용 가능한 패턴, axhub 가 이미 더 강한 부분 (scaffold/preflight/registry), 충돌 위험 |

## 빠른 시작

- 처음 보는 사람 → `00-overview.md` → `06-cross-cutting-themes.md`
- skill 작성자 → `08-reference.md` → `02-engineering-skills.md` 의 `write-a-skill` 비교
- axhub 작업자 → `99-axhub-takeaways.md`
- 특정 skill 검색 → `08-reference.md` 의 source-mapping 표

## 분석 방법론

ralplan 의 **Planner → Architect → Critic** 인라인 패스로 구조를 잠그고, 53/53 파일 직접 읽기 후 작성했어요. 각 SKILL.md 의 frontmatter / process step / 보조 doc 을 모두 인용 가능한 수준으로 캡처했어요. 분석 = 직접 인용 + 해석 분리 표기.

## 검증 체크리스트

- [x] 53/53 source 파일 `08-reference.md` source-mapping 표에 등재
- [x] 22/22 SKILL.md 각각 단일 doc 에서 다뤄짐 (engineering 9 / productivity 3 / misc 4 / personal 2 / deprecated 4)
- [x] 5/5 README, 1/1 ADR, 1/1 plugin.json, 2/2 script, 1/1 LICENSE, 1/1 .out-of-scope 표제 파일 cover
- [x] axhub repo 와의 차이점 명시 (`99-axhub-takeaways.md`)
- [x] **(ralph 1차)** Git history 50+ commit evolution 캡처 (`90-evolution.md`)
- [x] **(ralph 2차)** 정량 dimension 추가 (`91-additional-findings.md`) — inter-skill ref frequency / LOC 분포 / anti-pattern 카탈로그 / trigger 표 / heading 패턴

## Ralph 재검토 발견

### 1차 (`90-evolution.md`)
첫 분석은 shallow clone (depth=1) 으로 git history 못 봤어요. unshallow 후 49+ commit 확인 → 5개 사라진 skill (`prd-to-plan` / `write-a-prd` / `github-triage` / `triage-issue` / `domain-model`) + 사라진 보조 doc 2개 (`improve-codebase-architecture/REFERENCE.md`, `grill-with-docs/DOMAIN-AWARENESS.md`) + 디렉토리 migration (flat → bucket) + 어휘 통일 (`backlog → issue tracker`).

### 2차 (`91-additional-findings.md`)
정량 분석 누락 발견 — inter-skill `/command` 인용 frequency 측정 → 3-tier hub 정량 확인 (`grill-with-docs` 12 / `setup-matt-pocock-skills` 11 / `triage` 10). 22/22 SKILL LOC 분포 (median ~100L, max `triage` 372L, min `zoom-out` 7L). 28+ 명시 anti-pattern 인용 카탈로그 (8 카테고리). 22/22 description trigger 표 + heading 구조 패턴.

## 라이선스 / 출처

원본은 MIT License (Copyright 2026 Matt Pocock). 본 분석은 학습 목적 메모이며 원본 콘텐츠를 인용 시 출처 표기해요.
