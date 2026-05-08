# docs-only baseline measurement protocol

axhub plugin 의 라우팅 정확도 측정 baseline 중 `baseline-results.docs-only.{20,100}.json` 의 fresh ground truth 측정 protocol 이에요. v0.5.0 부터 routing-drift CI gate 의 source of truth.

## 목적

`baseline-results.docs-only.*.json` = Claude 가 SKILL.md `description` field 만 보고 매칭한 결과 (Approach E 의 ideal). production 의 routing 정확도 (claude-native baseline) 와 비교해서 drift 측정.

기존 baseline 은 M1.5 historical fixture (사용자 검토 X). v0.5.0 은 fresh ground truth 필요 — 이 docs 가 측정 절차.

## Protocol — 4 Step

### 1. Claude 추천

`scripts/measure-docs-only-baseline.ts` 가 `tests/corpus.100.jsonl` 의 100-tier row 각각 처리해요. v0.5.0 현재 100-tier 는 base 100 + meta_question 11 = 111 row 예요.

- 18 SKILL.md 의 frontmatter description 만 read
- LLM call (Claude Sonnet 4.6 / temperature=0):
  > "이 발화: '{utterance}' / 후보 SKILL description 18 개 / 가장 적합한 SKILL name 또는 null (axhub 도구 호출 의도 X)"
- 추천 결과 + 추천 confidence (high / medium / low)

### 2. 사용자 review

script 가 stdin 통해 사용자에게 row 1 개씩 제시:

```
Row 23/100
utterance: "결제 페이지 라이브로 띄워봐"
expected_skill (corpus): deploy
Claude 추천: deploy (confidence: high)
[a] accept Claude / [o] override / [n] null / [s] skip
>
```

옵션:
- `a` (accept): Claude 추천 그대로 ground truth
- `o` (override): 사용자가 다른 SKILL name 입력 → ground truth
- `n` (null): axhub 도구 호출 의도 X → fired_skill = null
- `s` (skip): 이 row 의 ground truth 미결정 (다음 measurement 까지 보류)

### 3. JSON 정규화

script 가 `tests/baseline-results.docs-only.100.json` 으로 결과 정규화:

```json
[
  {
    "utterance_id": "T1",
    "fired_skill": "apps",
    "actual_tool_calls": [],
    "required_consent_seen": false,
    "notes": "docs-only fresh baseline 2026-05-XX. reviewer: accept (claude high)."
  },
  ...
]
```

`notes` field 가 reviewer decision (accept / override / null) + timestamp 보존. 후속 measurement 시 drift 분석 가능.

### 4. timestamp + reviewer metadata

JSON 의 첫 entry 에 measurement metadata:

```json
[
  {
    "_metadata": {
      "measured_at": "2026-05-08T12:00:00Z",
      "reviewer": "wongil",
      "corpus_version": "corpus.100.jsonl@<git-hash>",
      "skills_version": "<git-hash>",
      "claude_model": "claude-sonnet-4-6",
      "rows_measured": 100,
      "rows_skipped": 0,
      "decisions": {"accept": 87, "override": 8, "null": 5}
    }
  },
  /* ... 100 rows ... */
]
```

routing-score.ts 가 `_metadata` entry 를 첫 entry 로 인식 + skip (regular row 만 score).

## Reviewer 책임

Reviewer (보통 SKILL 작성자 또는 maintainer) 의 결정 기준:

| Decision | 언제 | 예시 |
|----------|------|------|
| accept | Claude 추천이 명백히 옳을 때 | "axhub 로그 보여줘" → logs (high confidence) |
| override | Claude 추천이 명백히 틀렸을 때 | "이거 어떻게 동작해?" → null (Claude 추천: deploy, override) |
| null | axhub 도구 호출 의도 X | meta_question / off-topic / 모호한 발화 |
| skip | reviewer 가 결정 못 함 (corpus row 자체 모호) | 다음 measurement 까지 보류 + corpus.100 row 자체 재검토 시그널 |

## Frequency

- v0.5.0 release 마다 1 회
- Phase 1 (codegen) 또는 Phase 9 (examples) 의 description 갱신 후 필수 재측정
- routing-drift CI gate 가 stale baseline 으로 false-positive block 일 때 재측정 시그널

## Ground truth 권위

- ground truth = 사용자 review 결과 (Claude 추천 X)
- corpus.{20,100,jsonl}.jsonl 의 expected_skill 와 다를 수 있음 (corpus 의 expected_skill 도 작성 시점의 reviewer 결정이라 drift 가능)
- disagreement 시 사용자 final 결정 → corpus row 의 expected_skill 갱신도 함께 PR

## 자동화 한계

- Claude 가 description 만 본 결과는 "추천" — ground truth 가 아니에요
- LLM 의 비결정성 (temperature=0 이라도 약간의 sampling 변동 가능) 은 reviewer review 로 흡수
- script 가 사용자 stdin 의존 → CI 에서 자동 실행 X (manual measurement)
- routing-drift CI gate 는 commit 된 baseline (이 protocol 결과) 로만 측정해요. CI 안에서 Claude API fresh measurement 는 실행하지 않아요.

## 사용

```bash
bun run measure:baseline                    # full pipeline (corpus.100, stdin review)
bun run measure:baseline -- --corpus tests/corpus.20.jsonl  # 20-row tier
bun run measure:baseline -- --skip-prompt   # 자동화 mode (Claude 추천 그대로 accept, reviewer skip — drift 측정 시 evidence 약함)
```

`--skip-prompt` 는 emergency CI mode 만 사용. 정식 measurement 는 stdin review 필수.

## 관련

- script: `scripts/measure-docs-only-baseline.ts`
- output: `tests/baseline-results.docs-only.{20,100}.json`
- consumer: `tests/run-corpus.sh --vs claude-native --score` (Phase 8.3 의 routing-drift CI gate)
- corpus source: `tests/corpus.{20,100,jsonl}.jsonl`
- vision context: `.plan/ceo-review-nl-routing/2026-05-08-routing-95pct-vision.md`
