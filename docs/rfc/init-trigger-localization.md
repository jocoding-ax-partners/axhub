# RFC: init SKILL 의 nl-lexicon trigger 한국어 추가

- 상태: Draft
- 제안일: 2026-05-07
- 대상: `skills/init/SKILL.md` frontmatter `description:` 의 nl-lexicon trigger 어구
- 의존: PR #36 (이전 단계 — body 5 위치 한국어 풀이) + T1 (v0.3.2 doc drift fix) + T3 (L10 jargon 풀이) merge 후

## Motivation

PR #36 에서 init SKILL 본문의 영어 jargon ("scaffold", "apphub.yaml") 을 한국어로 풀이했어요. 본문 surface 는 비개발자 (vibe coder) 에게 친근해졌지만 frontmatter `description:` 의 nl-lexicon trigger 어구는 baseline 보호 위해 영어로 유지했어요.

남은 영어 trigger:
- `"scaffold"` — vibe coder 발화 가능성 0 에 수렴
- `"init"` — 영어 IT 용어, 일부 vibe coder 인지

router-body split-personality (router 는 영어, 본문은 한국어) 가 long-term 정합성 문제로 남아있어요. 이 RFC 는 vibe coder feedback 후 한국어 trigger 를 추가하는 절차를 정의해요.

## Current State

`skills/init/SKILL.md` frontmatter `description:`:

```
"새 앱 만들어줘", "결제 앱 만들어줘", "프로젝트 만들어", "Next.js 앱 만들어줘", "FastAPI 앱 만들어줘", "init", "scaffold", "axhub.yaml 만들어줘", "apphub.yaml 만들어줘"
```

baseline lock: `.omc/lint-baselines/skill-keywords.json` 의 init entry 7 phrase (한국어 phrase 만 lock — `scripts/check-skill-keywords-preserved.ts:34-37` regex `[가-힣]` 필터).

## Proposal

### 1. Additive Only 정책

영어 trigger ("scaffold", "init") **유지** + 한국어 trigger 추가:

후보 한국어 trigger:
- "프로젝트 만들기" — 광범위, 일반적 표현
- "앱 시작해" — 짧고 자연스러움
- "프로젝트 시작해" — "init" 의 의미 보존
- "스캐폴드해" — 영어 어휘를 한국어 표기로 (논쟁적)

`crates/axhub-helpers/src/main.rs:506` 의 `contains_any` OR 로직 활용 — 영어 trigger 제거 X 으로 backwards compat 보장.

### 2. Decision Rule

vibe coder feedback 수집 channel: GitHub issue/discussion (label `nl-lexicon-feedback`).

| feedback 상태 | 4-week 도래 후 결정 |
|---|---|
| ≥3 unique non-bot reactor + 각 comment ≥50 char with concrete use case | 진행 (한국어 trigger 추가) |
| 1-2건 + 4 주 도래 | maintainer review meeting → qualitative judgment |
| 0건 + 4 주 도래 | maintainer review meeting → silent drop 정당화 + decision log entry (`docs/rfc/decisions/init-trigger-localization-decision.md`) |

silent drop 도 정당한 outcome — vibe coder 가 영어 trigger 만으로 충분 만족 시.

### 3. Commit Type 결정

| 변경 종류 | Commit type | bump |
|---|---|---|
| SKILL routing 동작이 새 한국어 발화에 매칭 | `feat:` | minor |
| 단순 baseline cosmetic (routing 영향 0) | `chore(baseline):` | none |

RFC 진행 단계에서 e2e test (`tests/cli_e2e.rs`) 의 한국어 trigger fixture 추가 시 `feat:` 정당.

## Risk

HIGH risk:
- baseline drift — `.omc/lint-baselines/skill-keywords.json` 재캡처 필요
- fixture drift — `tests/cli_e2e.rs:431-459` 14 case 동시 update
- helper crate 영향 — `crates/axhub-helpers/src/main.rs:526-534` lexicon catalog 동기화

확인됨 (architect 검증):
- helper binary CLI **only** (router) 영향. hub-api routing 영향 zero (`lib.rs` 에 `main` 미노출).
- `cargo test --workspace` 가 hub-api wire 호출 X (anti-regression assertion: `cargo test -p axhub-helpers --test cli_e2e -- --nocapture 2>&1 | grep -cE 'hub-api|http://|https://'` = 0).

## Fixture Sync Plan

source of truth: `crates/axhub-helpers/src/main.rs:526-534` lexicon catalog (Rust source).

derive:
- `tests/cli_e2e.rs:431-459` 14 case — init trigger fixture (line 432/434/439) 동기화
- `crates/axhub-helpers/data/catalog.json` (lexicon 보유 시) 동기화
- `.omc/lint-baselines/skill-keywords.json` regen — `bun scripts/check-skill-keywords-preserved.ts --baseline` 후 manual `git diff` review (자동 commit 금지)

## Implementation Steps (deferred)

1. RFC merge (이 PR)
2. vibe coder feedback 수집 phase (4 주)
3. decision rule 적용 (위 표)
4. impl 진행 (별도 PR `feat/nl-lexicon-init-localization`):
   - `skills/init/SKILL.md` frontmatter `description:` 에 한국어 trigger 추가
   - `crates/axhub-helpers/src/main.rs:526-534` lexicon catalog 동기화
   - `tests/cli_e2e.rs:431-459` fixture sync
   - baseline regen + manual review
   - anti-regression verify (cargo test 출력 hub-api/http URL 0회)

## Acceptance (이 RFC PR)

- [x] RFC doc 본 RFC merged (이 PR)
- [ ] feedback channel (GitHub issue label `nl-lexicon-feedback`) 활성화 (PR merge 후 maintainer 작업)
- [ ] 4-week timeout date 결정 + decision log file scaffold

## Rollback

이 RFC merge 후 impl 미수행 시 별도 작업 없음 (doc only).

impl 후 rollback (별도 PR):
1. `cp .omc/lint-baselines/skill-keywords.json.bak .omc/lint-baselines/skill-keywords.json`
2. `git revert <impl commit>` + push

## Out of Scope

- deploy / apps / auth / 다른 SKILL 의 nl-lexicon trigger 한국화 (별도 RFC)
- ax-hub-cli 자체의 trigger phrase (axhub repo 외부)
- **본 RFC merge 자체는 한국어 trigger 추가 commitment 가 아니에요.** decision rule (≥3건 OR 4-week timeout) 통과 후 별도 PR (`feat/nl-lexicon-init-localization`) 으로만 impl 진행해요.

## References

- 선행 PR #36 — body 5 위치 한국어 풀이
- `.omc/lint-baselines/skill-keywords.json` — baseline lock
- `scripts/check-skill-keywords-preserved.ts:34-37` — baseline 추출 regex
- `crates/axhub-helpers/src/main.rs:506` — `contains_any` OR 로직
- `crates/axhub-helpers/src/lib.rs` — `main` 미노출 (hub-api routing 영향 zero 증명)
- `tests/cli_e2e.rs:431-459` — 14 e2e case fixture
