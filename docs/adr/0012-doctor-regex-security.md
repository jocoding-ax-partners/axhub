# ADR 0012 — doctor 정규식 보안 평가 및 강화 trigger 조건

## Status

Proposed — 데이터 수집 후 활성화 예정.

## Context

PR #41 (`feat: axhub init dependency-plan helper로 install 가시성 확보`) 은 `scripts/skill-doctor.ts` 에
dep-execution 패턴 검사를 추가했어요. 이 검사는 4개의 ECMAScript 정규식으로 구현되며,
각 정규식은 SKILL 본문에서 `!` prefix 가 붙은 의존성 설치 명령을 탐지해요.

4개 패턴 정의:

| 패턴 ID | 명칭 | 탐지 대상 | fixture |
|---------|------|-----------|---------|
| P1_DIRECT | 직접 실행 | `!npm install`, `!npm i`, `!"npm" install` | dep-exec-f, dep-exec-g |
| P2_SHELL_WRAP | 셸 래퍼 | `!sh -c "npm install"`, `!eval "npm install"` | dep-exec-a, dep-exec-b |
| P3_CHAIN | 체인 실행 | `!corepack enable && npm install`, `!setup; npm i` | dep-exec-c, dep-exec-e |
| P4_INDIRECT | 간접 실행 | `!CMD=npm; $CMD install`, `!npx --package npm npm install` | dep-exec-d, dep-exec-h |

RALPLAN Architect Round 3 (B.3) 에서 두 가지 결함이 발견됐어요:

1. **false-positive 매트릭스 미작성** — 4개 정규식이 code fence 내부 텍스트, 인라인 prose,
   주석 등 비실행 컨텍스트를 잘못 탐지하는 경우의 수가 문서화되지 않았어요.
2. **ReDoS 평가 부재** — Bun 1.x 는 `RegExp.linear` (POSIX NFA linear-time guarantee) 를
   지원하지 않아요. 이로 인해 catastrophic backtracking 이 발생할 수 있는 정규식이
   `bun run skill:doctor` 실행 시 latency spike 를 일으킬 위험이 있어요.

현재 `tests/skill-doctor-dep-execution.test.ts` 는 8개 positive fixture (dep-exec-a ~ dep-exec-h) 로
탐지 정확성을 잠금해요. 하지만 false-positive 방어를 위한 negative fixture 는 prose / code fence
두 케이스만 있으며, line comment skip 과 negative lookahead 범위 분석은 수행하지 않았어요.

## Decision

production 데이터가 아래 trigger condition 을 충족할 때 다음 강화 조치를 채택해요:

1. **false-positive 매트릭스 작성** — 4개 패턴 × {code fence, prose, line comment, YAML value} 조합의
   false-positive 케이스를 열거하고 negative fixture 로 regression lock 해요.
2. **negative lookahead expansion** — P1_DIRECT 와 P2_SHELL_WRAP 에 `` (?!`[^`]*) `` 유형의
   code fence pre-pass guard 를 추가하여 code fence 내부 텍스트를 조기 제외해요.
3. **line comment skip** — `#` 으로 시작하는 행을 패턴 매칭 전에 제거하는 pre-pass 를 추가해요.
4. **ReDoS 정적 분석** — 4개 정규식을 `recheck` 또는 동등한 정적 분석 도구로 평가하고,
   catastrophic backtracking 이 확인된 패턴은 해당 PR 에서 즉시 재작성해요.

trigger condition 이 충족되기 전까지는 현재 4개 정규식과 8개 fixture 상태를 유지해요.

## Drivers

1. PR #41 의 4개 정규식에 대한 false-positive 매트릭스가 작성되지 않았어요 (Architect Round 3 B.3).
2. Bun 1.x 가 `RegExp.linear` flag 를 지원하지 않아 ReDoS catastrophic backtracking 리스크가
   평가되지 않은 채로 CI 에서 실행되고 있어요.
3. production 에서 doctor 패턴 우회 케이스 추가 발견 시 대응 절차가 정의되어 있지 않았어요.

## Alternatives

| 선택지 | 설명 | 기각 이유 |
|--------|------|-----------|
| eager 정규식 강화 | 지금 즉시 negative lookahead 와 pre-pass 추가 | false-positive 데이터 없이 강화하면 탐지율이 낮아질 위험. production 근거 없이 spec 변경 불가. |
| 본 ADR (data-driven, trigger-based) | 수집된 데이터가 trigger condition 충족 시 강화 | **채택** — 현재 8개 fixture 는 충분하고 추가 오탐 보고가 없음. |
| 정규식 폐기 + AST parse | SKILL 본문을 AST 로 파싱하여 `!` prefix 명령 추출 | overkill. Markdown AST 파서 의존성 추가, `skill-doctor` 가 외부 dep 없이 단일 파일로 동작한다는 설계 원칙 위반. |

## Consequences

**긍정적 결과:**

- trigger 기반 강화로 근거 없는 spec 변경을 방지해요.
- false-positive 매트릭스가 작성되면 negative fixture 가 증가하고 regression 안전망이 강화돼요.
- ReDoS 평가 결과가 기록되어 향후 정규식 수정 시 참조할 수 있어요.

**tradeoff:**

- 데이터 수집 인프라가 필요해요 — `skill-doctor` 실행 시 false-positive 보고를 수집하려면
  telemetry hook 또는 production lint failure 추적 체계가 있어야 해요.
- trigger condition 충족 전까지 강화가 지연돼요. 이 기간 동안 false-positive 가 발생해도
  자동 수정되지 않아요.

**중립:**

- 현재 4개 정규식의 충분성 평가는 이 ADR 에서 확정하지 않아요.
  trigger condition 충족 후 별도 PR 에서 평가 결과를 첨부해요.

## Trigger Condition

다음 조건 중 하나라도 충족되면 이 ADR 을 활성화하고 강화 작업을 시작해요:

- **false-positive 보고 ≥ 3건** — `bun run skill:doctor` 실행 결과에서 탐지되었으나
  실제로는 무해한 코드(code fence 내부, prose, comment 등)에 대한 보고가 3건 이상 누적됐을 때.
- **ReDoS 의심 latency 발견** — CI 또는 로컬에서 `skill:doctor` 가 특정 SKILL 처리 시
  비정상적으로 높은 latency (예: 단일 SKILL > 500ms) 를 기록했을 때.
- **PR review 에서 doctor 패턴 우회 명시** — PR review 댓글 또는 RALPLAN round 에서
  "doctor 패턴을 우회하는 방법이 있다" 는 취지의 지적이 등장했을 때.
- **Bun runtime 의 RegExp linear flag 지원** — Bun 이 POSIX NFA linear-time guarantee 를
  제공하는 `RegExp.linear` 또는 동등한 API 를 공식 지원하기 시작했을 때.

## Pattern Reference

### P1_DIRECT — 직접 실행

```
/^!\s*(?:"npm"|'npm'|npm)\s+(?:install|i|ci|add)\b/m
```

탐지 예시:
- `!npm install` (dep-exec-g)
- `!npm i` (dep-exec-g)
- `!"npm" install` (dep-exec-f)

현재 미처리 케이스: backtick 내부 `` `npm install` ``, YAML value 중 `command: npm install`.

### P2_SHELL_WRAP — 셸 래퍼

```
/^!\s*(?:sh|bash|zsh)\s+-c\s+|^!\s*eval\s+/m
```

탐지 예시:
- `!sh -c "npm install"` (dep-exec-a)
- `!eval "npm install"` (dep-exec-b)

현재 미처리 케이스: `!env sh -c "..."`, `!command sh -c "..."`.

### P3_CHAIN — 체인 실행

```
/^!.*(?:&&|;)\s*(?:npm|yarn|pnpm|bun)\s+(?:install|i|add)\b/m
```

탐지 예시:
- `!corepack enable && npm install` (dep-exec-c)
- `!setup; npm i` (dep-exec-e)

현재 미처리 케이스: 개행으로 분리된 멀티라인 체인.

### P4_INDIRECT — 간접 실행

```
/^!.*(?:\$[A-Z_]+\s+install|npx\s+--package\s+npm)/m
```

탐지 예시:
- `!CMD=npm; $CMD install` (dep-exec-d)
- `!npx --package npm npm install` (dep-exec-h)

현재 미처리 케이스: `$PKG_MGR ci`, `npx --yes npm install`.

## Follow-ups

- `scripts/skill-doctor.ts` 에 telemetry hook 추가 — false-positive 보고를 집계할 수 있도록
  실행 결과를 structured log 로 기록해요.
- negative fixture 4~6개 추가 (B.3 mitigation) — line comment, YAML value, backtick inline
  컨텍스트에 대한 negative case 를 `tests/fixtures/skill-doctor/` 에 추가해요.
- ReDoS 정적 분석 결과를 이 ADR 에 appendix 로 첨부해요 (trigger 충족 시).

## References

- PR #41: `feat: axhub init dependency-plan helper로 install 가시성 확보`
- `scripts/skill-doctor.ts` — dep-execution rule 구현 (Phase 19)
- `tests/skill-doctor-dep-execution.test.ts` — 8개 positive fixture + 4개 negative case
- `tests/fixtures/skill-doctor/dep-exec-{a-h}.md` — 패턴별 positive fixture
- RALPLAN Architect Round 3 B.3 — false-positive 매트릭스 미작성 지적
- ADR 0009: Free-form preview policy (consent gate 참조)
