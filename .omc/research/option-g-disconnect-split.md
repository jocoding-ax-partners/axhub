# Option G — github SKILL Step 2 옵션 축소 (3 → 2) UX 영향 조사

**Status:** investigation complete — recommend **DEFER**
**Date:** 2026-05-13
**Plan:** F′ ADR PR-FU-4 follow-up
**Branch:** `chore/option-g-investigation`

## 배경

Plan F′ ralplan loop 의 Critic round-1 에서 \"missed alternative\" 로 지적된 옵션:

> Neither Planner nor Architect considered Option G: delete disconnect from github
> Step 2's option set, since it's a separate destructive flow handled at Step 5
> anyway. This would reduce the 3-option JSON to 2 (list_only / connect),
> eliminating the surface area where the model invents a 4th \"skip\" option.

이 문서는 Option G 의 실제 UX 영향을 조사하고 ship/defer 결정을 기록해요.

## 현재 상태 (post-F′)

`skills/github/SKILL.md` Step 2 AskUserQuestion:

```json
{
  "question": "GitHub 연동 작업을 고를까요?",
  "header": "GitHub",
  "options": [
    {"label": "목록만", "value": "list_only", "description": "연결할 수 있는 GitHub 저장소 목록을 봐요"},
    {"label": "연결",   "value": "connect",   "description": "앱에 GitHub 저장소를 연결해요"},
    {"label": "연결 해제", "value": "disconnect", "description": "exact confirm 과 consent 가 필요해요"}
  ]
}
```

NEVER 룰 (F′ + FU-3 enforcement):
- 4번째 옵션 invent 금지 (\"지금은 스킵\" 등)
- backend 가 git_connection_required (HTTP 422) 로 거절

## Option G 변형

### G1: Step 2 → 2 옵션 + intent-based disconnect routing

```json
{
  "question": "GitHub 연동 작업을 고를까요?",
  "header": "GitHub",
  "options": [
    {"label": "목록만", "value": "list_only", ...},
    {"label": "연결",   "value": "connect",   ...}
  ]
}
```

Step 1.5 신규: 사용자 utterance 가 disconnect 의도 (\"끊\", \"해제\", \"disconnect\") 를 포함하면 Step 2 skip → Step 5 disconnect flow 직행.

### G2: 별도 SKILL 분리 (`github-disconnect`)

`skills/github-disconnect/SKILL.md` 신설 → description trigger = \"끊\", \"해제\", \"disconnect\".
`skills/github/SKILL.md` 의 disconnect path (Step 5) 제거.

## 평가

### Pros (Option G 채택 시)

| 영향 | 설명 |
|------|------|
| **모델 4번째 옵션 invent 표면 축소** | 옵션이 2개면 4번째 invent 가 더 부자연스럼 (3 → 2 boundary 효과 미미하지만 0이 아님) |
| **vibe coder UX simplify** | 첫 진입 시 가장 흔한 의도 (목록 보기 / 연결) 만 노출. disconnect 는 destructive 이라 첫 화면에서 굳이 보일 이유 약함 |
| **disconnect 의도 명확성** | \"github\" 라고 했는데 무심코 disconnect 누르는 사고 방지 |

### Cons (Option G 채택 시 위험)

| 위험 | 영향 | 강도 |
|------|------|------|
| **discoverability 손실** | disconnect 옵션을 모르는 사용자가 어떻게 도달하는지 불명확. \"github 끊\" 라고 발화해야 하는데 그 trigger 가 description 에만 있음 | 중 |
| **routing 복잡성 증가** | Step 1.5 intent detection 추가 = SKILL 흐름이 1단계 깊어짐. F′ 가 막 정리한 closed-form 패턴과 정반대 방향 | 중 |
| **테스트 표면 확장** | tests/github-skill-step-2-options.test.ts 가 2 옵션 expect 로 변경 + Step 1.5 intent detection 회귀 테스트 신규 필요 | 낮음 |
| **G2 (별도 SKILL) 의 nl-lexicon collision 위험** | github / github-disconnect 두 SKILL 의 trigger phrase 가 겹치면 description quality lint 실패 (\"github\" 단일 단어 trigger 중복) | 중 |
| **기존 사용자 기억 깨짐** | github SKILL 진입 시 항상 3 옵션 보던 user 가 갑자기 2 옵션 + intent-based routing 으로 바뀜. behavioral drift | 낮음 |
| **F′ + FU-3 가 이미 4번째 invent 차단함** | NEVER 룰 + machine-enforcement test 가 substrate-level 차단. Option G 의 marginal benefit 작음 | 높음 |

## ROI 분석

- **Marginal benefit**: 모델 invent 표면 축소 (이미 NEVER 룰 + test 로 차단됨) → 5% 추가 안전성
- **Marginal cost**:
  - SKILL routing 1 step 추가 (Step 1.5 intent detection)
  - 신규 회귀 테스트 2-3 개
  - registry rationale 갱신 + allowed_safe_defaults 변경
  - 기존 사용자 mental model 변경
  - documentation 업데이트
- **결론**: cost > benefit. Option G 는 \"nice-to-have\" 이지 \"must-have\" 가 아님.

## 비교: Option B/F′ vs Option G

| 차원 | F′ (현재) | F′ + Option G |
|------|-----------|---------------|
| Step 2 옵션 수 | 3 | 2 |
| 4번째 invent 차단 메커니즘 | NEVER prose + machine test (FU-3) | 위 + 옵션 set 자체 축소 |
| disconnect discoverability | Step 2 picker 에서 즉시 보임 | utterance 기반 (intent detection) |
| Routing 복잡도 | flat 7 step | 7 step + Step 1.5 분기 |
| 신규 SKILL 필요 | 없음 | (G2 한정) github-disconnect SKILL 신설 |
| nl-lexicon baseline | unchanged | 영향 (G2 한정) |

## 결정: DEFER (이번 PR scope 안 implementation 안 함)

**채택 안 하는 이유:**
1. F′ + FU-3 의 NEVER 룰 + machine-enforcement 가 이미 substrate-level 에서 4번째 invent 를 차단해요. Option G 의 marginal benefit 가 작아요.
2. Step 1.5 intent detection 추가는 F′ 의 closed-form 단순화 방향과 충돌해요.
3. discoverability 손실 risk 가 marginal benefit 보다 커요.
4. 기존 사용자 behavioral drift 는 회피 가치 있어요.

**향후 재검토 trigger:**
- F′ + FU-3 적용 후 6개월 안에 4번째 invent 회귀 사례 발견 → Option G 재검토
- vibe coder 사용자 feedback 에서 disconnect 옵션 misclick 사고 보고 → Option G 재검토
- github SKILL 의 list_only / connect 흐름이 disconnect 보다 압도적으로 자주 쓰인다는 telemetry 확보 → Option G 재검토

## ADR

- **Decision:** Option G 를 이번 PR 에서 implement 하지 않아요. F′ + FU-3 의 substrate fix 가 충분해요.
- **Drivers:** marginal benefit 작음 / routing 복잡도 증가 / discoverability 손실 / behavioral drift.
- **Alternatives considered:** G1 (intent-based routing) / G2 (별도 SKILL 분리) / status quo (F′ + FU-3 만).
- **Why status quo:** F′ NEVER 룰 + FU-3 machine-enforcement 가 4번째 invent 의 root cause 를 substrate 레벨에서 막음. Option G 의 추가 fix 는 redundant 하면서 신규 위험 (discoverability / routing complexity) 을 도입.
- **Consequences:**
  - (+) ship cost 0 — 코드 변경 없음
  - (+) F′ 의 closed-form / surgical 원칙 보존
  - (−) 향후 4번째 invent 가 재발하면 fall-back option 으로 G 가 남아 있음 (이 문서가 cache)
  - (−) Option G 의 marginal benefit 은 미실현
- **Follow-ups:** 6개월 telemetry / feedback 모니터링 후 재평가.

## Test/Code Impact (이 PR)

이 PR 은 research artifact 만 deliver:
- ADD: `.omc/research/option-g-disconnect-split.md` (이 문서)
- 코드 변경 없음
- 테스트 변경 없음
- registry 변경 없음

따라서 lint/doctor/test 회귀 0.
