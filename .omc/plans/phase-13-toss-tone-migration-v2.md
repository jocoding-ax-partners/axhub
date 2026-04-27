# Phase 13 — Toss UX Writing Tone Migration (v2)

> **Mission (revised v2)**: 런타임 + commands + tests + lint **만** Toss UX Writing 톤으로 일괄 마이그레이션. SKILL workflow + docs + marketing + SKILL description 은 Phase 14/15 로 분할 deferred.
> **Source of truth**: https://developers-apps-in-toss.toss.im/design/ux-writing.html (WebFetch 재확인 완료)
> **Mode**: DELIBERATE consensus, **revised v2 — incorporates Round-1 Architect 7 fixes + Critic 5 new defects (10 total fixes)**
> **Author**: planner (RALPLAN-DR loop, post-Critic REJECT round 1/5)
> **Status**: v2 ready for Architect/Critic round 2

## Round-1 fix log (10 items, all addressed in v2)

| # | Source | Fix | Where addressed |
|---|---|---|---|
| 1 | Architect | 4-PR → single-PR-with-lint-gate (synthesis) | §4 + §5 |
| 2 | Architect | T-09/T-10 honest "axhub extension" labels | §1 |
| 3 | Architect | T-02 "능동형 권장 + state desc 예외" | §1 |
| 4 | Architect | manifest.test.ts:584 dropped as constraint | §6 + §7 |
| 5 | Architect | Phase 13 = runtime+commands+tests+lint only | §3 + §7 |
| 6 | Architect | Lint as PR1 prerequisite (commit FIRST) | §5 |
| 7 | Architect | T-06 `시나요?` 3 exceptions WebFetch verbatim | §1 (verified, posted verbatim) |
| 8 | Critic D1 | 취소→닫기 explicit decision-tree rubric | §6 |
| 9 | Critic D2 | Marketing voice ADR before Phase 14 | §3 + §10 follow-up |
| 10 | Critic D3 | `check-skill-keywords-preserved.ts` moved to PR1 prereq | §5 |
| 11 | Critic D4 | Tier E sizes corrected (real wc -l) | §3 |
| 12 | Critic D5 | `skill-noninteractive-guard.test.ts` + `fixtures.test.ts` audited | §7 |

---

## 1. WebFetch findings — Toss UX Writing 가이드 (8 verified rules + 2 axhub extensions)

다음 8개 룰은 가이드에서 직접 추출한 verbatim 인용. T-09, T-10 은 가이드 미명시 axhub 자체 확장.

### T-01. 해요체 (verbatim, Toss-mandated)
> "제품 안의 모든 문구는 '해요체'로 써요" / "상황, 맥락을 불문하고 모든 문구에 해요체를 적용해 주세요."

### T-02. 능동형 권장 + state desc 예외 (verbatim, Toss-mandated, softened — Architect fix #3)
> "제품 안에서 최대한 능동형 문장을 써주세요. 수동형 문장은 특정 상황에서만 쓰는 게 좋아요."

→ 절대 ban 아님. 사용자 책임 아닌 state description (`Keychain 이 잠겨 있어요`, 외부 API `잠시 끊겼어요`) 수동형 허용. keychain 4-part cause 라인이 적용 사례.

### T-03. 과거형 `~었` 빼기 (verbatim)
> "~었 빼기" — "발생했었어요" → "발생했어요"

### T-04. 부정형 → 긍정형 (verbatim)
> "부정적 커뮤니케이션을 최대한 줄이고 긍정형으로 바꿔주세요"

### T-05. 다이얼로그 왼쪽 `닫기` (verbatim)
> "다이얼로그 왼쪽 버튼은 닫기로 문구를 통일해요. 취소는 사용자가 하고 있는 작업이 취소된다고 오해할 수 있어 쓰지 않아요."

→ axhub `취소`/`닫기` decision-tree → §6 (Critic D1).

### T-06. 과도한 경어 제거 + 3 exception (verbatim, Architect fix #7 WebFetch re-verified)
> "~시겠어요?, 시나요?, ~께 같은 과도한 경어를 쓰지 않아요"

**`시나요?`/`셨나요?` 허용 3 exception (verbatim)**:
1. **사용자의 맥락을 활용해서 질문할 때**: "사용자의 당황스러움을 줄일 수 있어요."
2. **사용자의 상황을 추정할 때**: "토스에 명확한 정보가 없어서 사용자에게 직접 판단하게 해야 할 때"
3. **사용자의 선의가 필요할 때**: "설문조사처럼 사용자의 선의를 기대해야 할 때 경어로 정중하게 질문해요."

→ axhub 적용 예시 (Phase 14 범위, Phase 13 밖): consent gate "토큰을 저장할까요?" = case 2; recovery 설문 = case 3.

### T-07. 축약 `돼요` (verbatim)
> "되어요" → "돼요"

### T-08. 명사 스택 회피 (verbatim)
> "{명사} + {명사} 쓰지 않기"

### T-09. 호칭 — **axhub 자체 확장 (NOT Toss-mandated)** (Architect fix #2)
가이드는 "당신/고객님" 사용을 명시적 ban 하지 않음. axhub 의 `당신 앱은 안전합니다` 제거는 **planner extrapolation** 으로 명시.

### T-10. 에러 4-part 구조 — **axhub Phase 11 자체 확장 (NOT Toss-mandated)** (Architect fix #2)
가이드에 error structure rule 없음. 4-part (감정/원인/해결/다음) 는 Phase 11 검증된 axhub 패턴. "Toss 권장" 표기 금지.

---

## 2. Tone delta table — current vs Toss target (preserved from v1)

| 차원 | 현재 axhub | Toss 타깃 | Phase 13 영향 (runtime+commands+tests 만) |
|---|---|---|---|
| Ending form | `해요체` 주류 + `합니다체` 산발 | 100% `해요체` | catalog/keychain/list-deployments/index 의 `~합니다`, `~입니다` 변환 ~50개소 |
| Honorific | `~시겠어요`, `~해주세요`, `해주실래요` 혼용 | 평소 제거, `시나요?` 3 exception 한정 | runtime: index.ts cmdSessionStart, list-deployments error → ~15개소 |
| 호칭 (T-09 자체확장) | "당신", "고객님", "본인" | 액션 중심 (axhub policy) | catalog.ts 11회 + keychain 4회 → 직접 호칭 제거 |
| 부정형 | "안 됐어요", "막혔어요", "실패했어요" | 긍정 재구성 | catalog cause 문장 ~30개소 |
| 명사 스택 | `배포 요청 검증 실패` 4-noun | 동사형 분해 | catalog cause ~20개소 |
| Punctuation | `.` + `!` + `—` | 마침표 절제, `!` 축하 한정 | 거의 유지, `!` 강조 과잉만 제거 |
| Emotion prefix (Phase 11 4-part) | `잠깐만요/아이고/죄송해요/축하해요` | Toss 도 `잠깐만요` 사용 패턴. `아이고` 미언급 | `아이고` → `잠깐만요`/`이상해요` 통일. 4-part 자체 보존. |

**Phase 13 변경 총량 (Phase 14/15 deferred 제외)**: ~80 strings (catalog) + ~12 (keychain) + ~13 (list/index) + ~30 (commands) + ~25 (install/hook) = **~160 string sites + ~70 test assertion update**

---

## 3. Scope — Phase 13 ONLY (shrunk from v1)

### Phase 13 INCLUDED (this cycle, 1 PR)
- **Tier A runtime**: `src/axhub-helpers/catalog.ts`, `keychain.ts`, `keychain-windows.ts`, `list-deployments.ts`, `index.ts`
- **Tier C slash command descriptions**: 9 `commands/*.md` 한 줄 description + `help.md` 21줄 menu
- **Tier D install/hook**: `bin/install.sh`, `bin/install.ps1`, `hooks/session-start.sh`, `hooks/session-start.ps1`
- **Tier G tests**: `classify-exit.test.ts`, `keychain.test.ts`, `keychain-windows.test.ts`, `list-deployments.test.ts`, `session-start-ps1.test.ts`, `install.test.sh`, `install-ps1.test.ts`, `axhub-helpers.test.ts` (audit only — fixture KR slugs are user voice, no change)
- **Lint infrastructure**: `scripts/check-toss-tone-conformance.ts` + `scripts/check-skill-keywords-preserved.ts` (PR1 prereq, commit BEFORE source edits)

### Phase 14 DEFERRED (separate cycle, separate PRs)
- Tier B SKILL workflow body (11 SKILL.md) — large, AskUserQuestion KR labels
- references/*.md (privacy-filter, error-empathy-catalog.md, headless-flow.md, recovery-flows.md)
- Tier E docs (troubleshooting 284, vibe-coder-quickstart 234, org-admin-rollout 294, marketing/landing-page 202, marketing/outreach-email 126, pilot/admin-rollout 110, pilot/onboarding-checklist 108, pilot/feedback-template 97 = **~1,471 KR lines real**)
- Tier F CHANGELOG retroactive cleanup
- **Marketing voice ADR (Critic D2 fix)**: docs/marketing/ 작성자 정의 — Phase 14 entry-criterion 으로 차단

### Phase 15 DEFERRED (post-measurement, run after Phase 13 activation drift data)
- Tier B SKILL.md description **narrative 부분만** Toss 화 (description 안 따옴표 phrase 는 영구 보존)
- 결정 게이트: Phase 13 머지 후 1주 skill activation miss-rate ≤2%p 변화 확인 후 Phase 15 진입

### Critic D4 — Tier E real sizes (corrected)
| 파일 | v1 estimate | 실제 wc -l |
|---|---|---|
| troubleshooting.ko.md | ~121 | **284** |
| vibe-coder-quickstart.ko.md | ~107 | **234** |
| org-admin-rollout.ko.md | ~179 | **294** |
| marketing/landing-page.ko.md | ~200 (est) | **202** |
| marketing/outreach-email.ko.md | ~100 (est) | **126** |
| pilot/admin-rollout.ko.md | ~150 (est) | **110** |
| pilot/onboarding-checklist.md | ~80 (est) | **108** |
| pilot/feedback-template.md | ~60 (est) | **97** |
| **합계** | ~1,000 | **1,471** |

→ Phase 14 절대 1 PR 으로 못 들어감. Phase 14 도 sub-PR 분할 필요 (planner 이후 mission).

---

## 4. Options collapsed — 2 viable (v1's 4 → 2)

Round-1 후 Option C (style-guide-only) 와 Option D (two-tone hybrid) 는 invalidated:
- **Option C invalidated**: 미션 ("일괄 수정") 위배 + Critic 의 "tone consistency 영원히 도달 불가" 지적
- **Option D invalidated**: 같은 세션 두 톤 mismatch + credibility risk + power/vibe 경계 정의 자체가 다음 phase 부담

남은 2 viable 비교:

### Option A revised — Single-PR with lint-gate (synthesis adopted, chosen)
**핵심**: 1 PR, 두 atomic commit. Commit 1 = lint scripts only (source 무변경, baseline 확립). Commit 2 = source + tests + commands 동시. lint `--strict` pass 가 머지 전제.

- **Pros**: Critic recommendation + Architect synthesis 부합. 톤 일관성 100%. Phase 13 scope shrink 로 ~160 strings + 70 test = reviewer-tractable. 단일 reviewer cycle. rollback 단일 PR revert.
- **Cons**: lint script 자체 버그 → mitigated by `tests/lint-toss-tone.test.ts` 동봉. 5명 A/B test 가 머지 후 → mitigated by PR desc 의 before/after 5개 sample.

### Option B — Tiered 3-PR (lint → runtime → tests-update)
- **Cons (decisive)**: PR2 머지 후 PR3 전 main red 위험. Architect fix #1 synthesis 위배.

→ **Decision: Option A revised** 채택.

---

## 5. PR plan (1 PR for Phase 13)

### PR `phase-13/toss-tone-runtime-pr1`
**Branch**: `phase-13-toss-tone-runtime-pr1`
**Reviewer cycle**: 1
**Estimated diff**: ~160 source strings + ~70 test assertions + 2 new lint scripts ≈ **30-35 files**

#### Commit 1 (PRE-REQUISITE — MUST commit FIRST, source unchanged)
**Files**:
- `scripts/check-toss-tone-conformance.ts` (new) — searches for `합니다`, `입니다`, `당신` (T-09 자체확장 잔존), `~시겠어요`, `~시나요?` (밖 3 exception 는 allowlist), `되어요`, `~었어요` 잔존 in **Tier A + Tier C + Tier D** files only (Phase 13 범위)
- `scripts/check-skill-keywords-preserved.ts` (new) — extracts double-quoted KR phrases inside `description:` of all 11 SKILL.md, saves snapshot to `.lint/skill-keywords.snapshot.json`. Subsequent runs diff against snapshot. Phase 13 commit 1 establishes baseline; Phase 13 source changes don't touch SKILL.md descriptions so diff MUST be 0.
- `tests/lint-toss-tone.test.ts` (new) — calls both scripts, asserts zero violations across Phase-13 file scope only
- `package.json` add `"lint:tone": "bun scripts/check-toss-tone-conformance.ts && bun scripts/check-skill-keywords-preserved.ts"`

**Acceptance for commit 1**:
- `bun lint:tone` exits 0 (baseline) — baseline computed from CURRENT axhub source which includes `합니다`/`당신`/etc, but lint script must START in "warn-only mode" with `--strict` flag opt-in. Baseline commit asserts `--strict` finds N violations where N matches expected v1 count. Commit 2 then drives N → 0.
- Equivalent: lint script has `--baseline <count>` mode; commit 1 records baseline (e.g. `--baseline 160`); commit 2 asserts violations==0 with `--strict`.
- `bun test tests/lint-toss-tone.test.ts` green
- 0 source files modified outside `scripts/` and `tests/`

#### Commit 2 (atomic source + test + command + install update)
**Files** (Phase 13 scope):
- `src/axhub-helpers/catalog.ts` (13 entries, 4-part rewrite, 호칭 제거, 어미 변환)
- `src/axhub-helpers/keychain.ts` (4 messages × 4 lines)
- `src/axhub-helpers/keychain-windows.ts` (5 messages × 4 lines)
- `src/axhub-helpers/list-deployments.ts` (7 strings)
- `src/axhub-helpers/index.ts` (6 strings)
- `commands/apis.md` `apps.md` `deploy.md` `doctor.md` `help.md` `login.md` `logs.md` `status.md` `update.md` (description 한 줄 + help.md 21줄 menu)
- `bin/install.sh` (12 messages)
- `bin/install.ps1` (5 multi-line errors)
- `hooks/session-start.sh` (2 messages)
- `hooks/session-start.ps1` (6 multi-line systemMessage)
- `tests/classify-exit.test.ts` (39 assertion update)
- `tests/keychain.test.ts` (regex update + cause/solve/next markers preserved)
- `tests/keychain-windows.test.ts` (어휘 검증 grep update — 보안 솔루션, 코드사이닝, 관리자에게)
- `tests/list-deployments.test.ts` (1 toContain update)
- `tests/session-start-ps1.test.ts` (1 toContain update)
- `tests/install.test.sh` (~5 grep pattern update)
- `tests/install-ps1.test.ts` (~5 grep update)
- `scripts/codegen-catalog.ts` 재실행 후 `error-empathy-catalog.generated.md` 동기화 (commit 2 의 same atomic)

**Acceptance for commit 2**:
- `bun lint:tone --strict` exits 0 (violations 0)
- `bun test` 전체 green
- `axhub-helpers classify-exit --code 65` stdout 캡처 PR description 첨부
- `axhub-helpers keychain-test` (or equivalent) stdout 캡처 첨부
- `git grep "당신 앱"` returns 0
- `git grep "합니다\\."` in Phase-13 file list returns 0
- `scripts/check-skill-keywords-preserved.ts` exits 0 (description phrase 변경 0 — Phase 13 SKILL.md 안 만짐)

#### NOT in PR1 (Phase 13)
- 11 SKILL.md (workflow body + description) → Phase 14/15
- references/*.md → Phase 14
- docs/ → Phase 14
- CHANGELOG retro → Phase 14
- nl-lexicon.md → 영구 변경 금지

---

## 6. 취소 → 닫기 explicit rubric (Critic D1 fix)

### Background
catalog.ts 현황: 6× `취소`, 1× `닫기`. v1 plan 은 "destructive abort 인 경우 유지" 라고 했으나 "destructive abort" 정의가 없어 작업자 해석 편차.

### Decision tree (Phase 13 적용 — 작업자 mechanical follow)

```
For each AskUserQuestion option label = "취소":
  Q1: 이 option 을 누르면 user-initiated mutation 이 일어나는가?
      (e.g. "강제 다운그레이드 / 취소" 의 취소 = 다운그레이드 자체를 abort = mutation O)
   ├─ YES (mutation 발생): 유지 "취소" — Toss 가이드 말한 "사용자가 하고 있는 작업이 취소" 와 일치하므로 의도 명확 표기
   └─ NO (단순 dismiss/back, mutation 없음):
      Q2: 이 dialog 가 modal 인가 (다른 작업 차단 중)?
       ├─ YES: → "닫기" (Toss T-05 직접 적용)
       └─ NO (예: hint, FYI, non-modal banner): → "닫기" (default)
```

### Concrete catalog.ts mapping (commit 2 결과 명시)

| catalog entry | 현재 label | Q1 답 | Q2 답 | 변경 |
|---|---|---|---|---|
| AUTH_TOKEN_MISSING (consent gate) | "취소" | NO (단순 dismiss, 토큰 미저장) | YES (modal) | → **"닫기"** |
| DEPLOY_REQUEST_VALIDATION_FAILED | "취소" | NO (재시도 안 함 = back) | YES | → **"닫기"** |
| FORCE_DOWNGRADE_CONFIRM | "취소" | YES (다운그레이드 자체 abort) | YES | **유지 "취소"** |
| TOKEN_EXPIRED_RELOGIN | "취소" | NO (재로그인 안 함 = dismiss) | YES | → **"닫기"** |
| RECOVERY_REINSTALL_PROMPT | "취소" | NO | YES | → **"닫기"** |
| UPDATE_AVAILABLE_OPT_IN | "취소" | NO | NO (banner) | → **"닫기"** |

→ commit 2 결과: 6× `취소` → 1× `취소` (FORCE_DOWNGRADE 만) + 5× `닫기`. 합계 6× `닫기` (기존 1 + 신규 5). PR description 표 첨부 필수.

→ 작업자 해석 편차 0 보장.

---

## 7. Test plan v2 (file-by-file impact, including Critic D5 audit)

### Unit tests (Phase 13 in-scope)
| Test file | Source dependency | Update | 카운트 |
|---|---|---|---|
| `tests/classify-exit.test.ts` | catalog.ts | 39 toContain new tone | 39 |
| `tests/keychain.test.ts` | keychain.ts | regex `(잠깐만요\|아이고\|죄송해요)` → `(잠깐만요\|이상해요\|죄송해요)`. 4-part markers preserved | 6 |
| `tests/keychain-windows.test.ts` | keychain-windows.ts | 어휘 retain check `보안 솔루션`, `코드사이닝`, `관리자에게` | 4 |
| `tests/list-deployments.test.ts` | list-deployments.ts | `토큰을 찾을 수 없어요` toContain — Toss 변형 시 update | 1 |
| `tests/session-start-ps1.test.ts` | hooks/session-start.ps1 | `보안 솔루션` toContain | 1 |
| `tests/codegen.test.ts` | error-empathy-catalog.generated.md | 4-part markers `**감정:** **원인:** **해결:** **버튼:**` 보존 (구조 유지) | 0 |
| `tests/manifest.test.ts` | 모든 SKILL.md | **L584 regex `^(This skill\|이 스킬)/` 는 prefix 만 constraint, NOT tone (Architect fix #4). Phase 13 SKILL.md 안 만짐 → 영향 0** | 0 |

### Critic D5 missed-tests audit (added in v2)
| Test file | 실제 내용 | Phase 13 영향 |
|---|---|---|
| `tests/skill-noninteractive-guard.test.ts` (49 lines) | `[ -t 1 ]`, `WATCH=--watch`, `FOLLOW=--follow`, `-z "$CI"`, `CLAUDE_NON_INTERACTIVE` shell-token assertion. **0 KR strings**. | **영향 0** — Phase 13 SKILL.md 안 만지고 shell guard 토큰만 assert. |
| `tests/fixtures.test.ts` (105 lines) | `tests/fixtures/*.json` 에서 parser semantics (is_destructive, action, app_id) assert. **Korean fixture slugs (`결제 페이지 버그 수정` 등) 는 사용자 발화 voice (parser input) 이지 axhub voice 아님.** | **영향 0** — fixture KR slug = user utterance corpus. nl-lexicon 과 동일 정책. 변경 금지. |
| `tests/axhub-helpers.test.ts` | 14 fixture lines KR (사용자 발화 fixture) | **영향 0** — 사용자 voice |

### Integration / shell tests (Phase 13 in-scope)
- `tests/install.test.sh` — install.sh KR error grep ~5 update
- `tests/install-ps1.test.ts` — install.ps1 KR grep ~5 update
- `tests/run-corpus.sh` — corpus.jsonl 사용자 발화, 변경 X

### E2E (Phase 13)
- `axhub-helpers classify-exit --code 65/66/68` stdout 캡처 (각 PR mandatory)
- 실제 install.sh 실행 stdout 캡처

### Observability (Phase 13)
- `~/.cache/axhub-plugin/empathy-catalog.ndjson` 신·구 톤 emit 비율 1주 모니터링
- skill activation telemetry — Phase 13 SKILL.md 안 만지므로 baseline 만 측정 (Phase 15 결정 게이트 input)

### Phase 13 test plan total
- Unit: 50 assertion update (39 + 6 + 4 + 1)
- Integration: ~10 grep pattern update
- E2E: 4 stdout capture
- Lint: 2 new scripts + unit test
- **No SKILL.md test impact** (Phase 13 안 만짐)
- **fixtures.test.ts / skill-noninteractive-guard.test.ts: 0 update** (Critic D5 verified — non-impact)

---

## 8. Pre-mortem v2 (4 linguistic-focused scenarios)

### PM-1. 4-part empathy 의 warmth 손실 (preserved from v1)
**가상**: catalog.ts Toss 화 후 `잠깐만요. 일시적인 통신 문제예요. 당신 앱은 안전합니다.` → `잠시 통신 문제가 있어요. 앱은 그대로예요.`. 11pm demo persona 가 안심 못 느낌.

**대응**:
- emotion prefix (`잠깐만요`, `괜찮아요`, `축하해요`) **반드시 보존** — lint script 가 catalog.ts 의 emotion field 첫 단어 검사
- "당신 앱은 안전합니다" → "앱은 그대로 잘 돌아가고 있어요" (호칭만 제거, 안심 의미 보존)
- PR description 에 before/after 5개 sample 필수 (commit 2 acceptance)
- 머지 후 1주 vibe coder feedback collect — NPS dropped 시 stop-loss revert

### PM-2. 취소→닫기 작업자 해석 편차 (NEW — Critic D1 motivated)
**가상**: 작업자 A 는 RECOVERY_REINSTALL_PROMPT 를 "재설치 안 함 = mutation 의 abort" 로 해석해 "취소" 유지. 작업자 B 는 "재설치 trigger 가 user action 이고 dismiss = no-op" 로 해석해 "닫기" 변환. 결과 inconsistent UI.

**대응**:
- §6 의 decision-tree 를 PR1 commit 1 의 lint script 로 mechanize: catalog.ts 의 각 AskUserQuestion entry 에 `mutation_on_select: true|false` 메타 필드 추가, lint 가 "mutation_on_select=false 면 label 은 '닫기' 강제" 검사
- §6 표를 PR description 에 paste — reviewer 1 에 의해 manual 검증 cross-check
- 작업자 자유 해석 zero — mechanical rubric

### PM-3. Test breakage — string-equality assertion 폭발 (preserved)
**가상**: catalog.ts 변경 commit 후 39 test fail. 작업자가 mock update 누락. main red 24h.

**대응**:
- commit 2 가 atomic — source + test 동일 commit 내. mock 누락 시 commit 자체 reject
- pre-commit hook: catalog.ts diff 있으면 `bun test tests/classify-exit.test.ts` 자동 실행
- CI fail 시 즉시 revert policy (24h 룰)
- lint script `bun lint:tone --strict` 가 mock-source mismatch 도 cross-check

### PM-4. Mock-only update — source 미변경 silent regression (preserved)
**가상**: 작업자가 classify-exit.test.ts expected 만 새 톤으로 바꾸고 catalog.ts 깜빡함. CI green 하지만 사용자에 구 톤.

**대응**:
- commit 2 acceptance: 실제 helper 바이너리 빌드 후 stdout 캡처 PR 첨부 (실제 source 변경 증거)
- snapshot test: `bun test:snapshot` 으로 catalog.ts → markdown 렌더링 결과 비교
- code-reviewer agent verify: `git grep "당신 앱"` returns 0 + `git grep "합니다\\."` in Tier A returns 0
- lint script `--strict` 가 source-side string scan 이므로 mock 만 update 한 case 자동 detect

→ **EDR/proxy 등 인프라 risk 는 Phase 13 범위 무관 (linguistic-only PR), 의도적으로 omit (Critic 지시 부합).**

---

## 9. PRD stories US-1301 ~ US-1306 (Phase 13 ONLY)

→ US-1307 (install/hook) 는 Phase 13 retain (Tier D 는 runtime+install 묶음). v1 의 US-1308 (docs) 는 Phase 14 로 이전.

### US-1301. 4-part empathy catalog Toss 톤 변환 (Tier A 핵심)
**As** vibe coder seeing deploy error at 11pm before demo
**I want** error to use Toss 해요체 + 능동형 (state desc 예외) + 긍정형
**So that** I feel calm, parse 메시지 빨리, 다음 액션 즉시 인지
**Acceptance**:
- catalog.ts 13 entries: (a) 어미 100% 해요체 (b) 직접 호칭 (`당신`) 0회 (c) 부정형 → 긍정형 (d) emotion prefix (잠깐만요/괜찮아요/축하해요) 보존 (e) `mutation_on_select` 메타 필드 추가
- §6 decision-tree 로 5× `취소` → `닫기`, 1× `취소` 유지 (FORCE_DOWNGRADE)
- `tests/classify-exit.test.ts` 39 assertion update + green
- `bun lint:tone --strict` 0 violation in catalog.ts
- `axhub-helpers classify-exit --code 65/66/68` stdout 캡처 PR 첨부

### US-1302. Keychain 4-part Toss 톤
**Acceptance**:
- keychain.ts 4 errors + keychain-windows.ts 5 errors → Toss 톤
- `아이고` deprecated → `이상해요` 또는 `잠깐만요` 통일
- `tests/keychain.test.ts` regex update + green
- `tests/keychain-windows.test.ts` 어휘 retain check (보안 솔루션, 코드사이닝, 관리자에게) green
- 4-part 구조 (감정/원인/해결/다음) 보존 — codegen.test.ts 0 update

### US-1303. List/index 1-line errors
**Acceptance**:
- list-deployments.ts 7 strings + index.ts 6 strings → Toss
- `tests/list-deployments.test.ts` toContain update + green
- `axhub-helpers token`, `axhub-helpers list` stdout sample PR 첨부

### US-1304. Slash command descriptions + help.md menu
**Acceptance**:
- 9 commands/*.md description 한 줄 → Toss
- help.md 21줄 menu Toss (자연어 예시 따옴표 = 발화 lexicon 보존)
- `/axhub:help` stdout PR 첨부

### US-1305. Install/hook 시스템 메시지
**Acceptance**:
- bin/install.sh 12 + bin/install.ps1 5 multi-line + hooks/session-start.{sh,ps1} 8 → Toss
- `tests/install.test.sh` + `tests/install-ps1.test.ts` + `tests/session-start-ps1.test.ts` grep update + green
- 4-part 구조 보존 (Tier D 도 information density 유지)

### US-1306. Lint infrastructure (PR1 commit 1 prerequisite)
**As** future axhub maintainer adding catalog entry
**I want** automated tone conformance check
**So that** drift 발생 시 PR 머지 차단
**Acceptance**:
- `scripts/check-toss-tone-conformance.ts` (new) — Phase 13 file scope, `--strict` mode 0 violation
- `scripts/check-skill-keywords-preserved.ts` (new) — baseline snapshot, Phase 13 동안 diff 0
- `tests/lint-toss-tone.test.ts` — both scripts green
- `package.json` `"lint:tone"` script
- 취소→닫기 mechanical check (catalog.ts mutation_on_select 메타 검사)
- **commit 1 으로 source 무변경 baseline 확보** — commit 2 머지 차단 게이트

### NOT IN PHASE 13 (deferred)
- US-1307 (Install): **retained in Phase 13** (Tier D 는 install runtime — vibe coder 첫 5분)
- US-1308 (Docs Toss + CHANGELOG + marketing) → **Phase 14**
- US-130x (SKILL workflow + references) → **Phase 14**
- US-130y (SKILL description narrative) → **Phase 15** (post-measurement)

---

## 10. ADR v2 — incorporates Round-1 10 fixes + scope shrink

### Decision
**Single-PR with lint-gate** (Option A revised) 로 Phase 13 = runtime + commands + install/hook + tests + lint scripts 만 Toss UX Writing 톤으로 마이그레이션. SKILL workflow + SKILL description + references + docs 는 Phase 14/15 deferred. catalog 4-part 구조는 axhub 자체 확장 (T-10) 으로 명시 보존, 호칭 제거 (T-09 자체 확장) 도 명시 표기. lint scripts (`check-toss-tone-conformance.ts` + `check-skill-keywords-preserved.ts`) 가 source 변경 commit 의 머지 prerequisite.

### Drivers
1. vibe coder 첫 5분 surface (install/help/runtime) 톤 일치 = trust thesis
2. Toss 가이드 industry-standard 한국 vibe coder expectation
3. Phase 11 emotional warmth ROI 보존 — 4-part 구조 폐기 zero
4. Test 안정성 — Phase 13 scope shrink 로 ~70 assertion (1,500 아님), atomic commit
5. Lint mechanization — 작업자 해석 편차 zero, 향후 entry 자동 게이트

### Alternatives considered
- A v1 (4-PR tiered): 머지 사이 mixed-tone → invalidated by Architect #1
- B v1 (Big-bang 4-tier): 1,500 strings reviewer 한계 초과 → invalidated
- C (Style guide only): "일괄 수정" 미션 위배 → invalidated
- D (Two-tone hybrid): power/vibe 경계 정의 부담 + credibility risk → invalidated
- B revised (3-PR atomic split): PR2-PR3 main red 24h risk → invalidated

### Why chosen
- Synthesis: lint commit-FIRST + atomic source commit = Architect #1 + Critic recommendation 동시 충족
- Scope shrink: ~160 strings + 70 assertion = reviewer-tractable
- Lint mechanization: 취소→닫기 (D1) + skill keyword preserve (D3) 자동 게이트
- Tier E real 1,471 lines disclosure (D4) → Phase 14 sub-PR 분할 사전 input
- Marketing voice ADR (D2) Phase 14 entry-criterion → Phase 13 unblock
- T-09/T-10 honest "axhub 자체 확장" labeling (Architect #2) → "Toss 시켰다" 거짓 제거

### Consequences
**Positive**:
- vibe coder install → first command → first error 일관 Toss 톤 (Phase 13 surface 만)
- Lint script 가 향후 catalog 추가 entry 의 drift 자동 차단
- 취소/닫기 mechanical rubric 으로 작업자 해석 편차 0
- Phase 14/15 진입 조건 명시 (activation drift ≤2%p) → silent SKILL.md regression 방지
- ADR honest labeling 으로 향후 Toss 가이드 업데이트 시 axhub 자체 확장 부분 보존 가능

**Negative**:
- vibe coder 가 docs (Phase 14 보류) 와 runtime (Phase 13 완료) 사이 transient 톤 mismatch ~2-4주
- Phase 14/15 일정이 Phase 13 머지 후 1주 measurement 에 의존 → blocking dependency
- Lint script 자체 maintenance 부담 (false positive 발생 시 작업자 escape hatch 없음)

**Risk acceptance**:
- vibe coder NPS drop 5%↓ 시 Phase 13 PR rollback + 재기획. Phase 14/15 자동 차단.
- skill activation miss-rate +2%p 시 Phase 15 영구 cancel — SKILL description 변경 포기.
- Tier E real size 1,471 lines 가 Phase 14 1 PR 불가능 — Phase 14 planner 가 sub-PR 분할 mission 으로 재명시.

### Follow-ups
1. **Phase 14 marketing voice ADR (Critic D2 entry criterion)** — `docs/marketing/` 작성자 정의 (제품 톤 vs 마케팅 톤 분리 정책). Phase 14 진입 차단 게이트.
2. **Phase 14 sub-PR 분할 plan** — Tier E 1,471 lines 를 ~3-4 PR 으로 split (troubleshooting+vibe-quickstart+org-rollout 한 PR, marketing 별도 PR, pilot 별도 PR, SKILL workflow 별도 PR).
3. **Phase 15 entry gate** — Phase 13 머지 후 1주 skill activation telemetry 결과 (miss-rate ≤2%p) 충족 시에만 Phase 15 SKILL.md description narrative 변경.
4. **`docs/STYLE_GUIDE.ko.md`** — Phase 14 후반 작성, Toss 8 룰 + axhub 자체 확장 (T-09 호칭, T-10 4-part) 명시, lint scripts 사용법 포함.
5. **CHANGELOG `Phase 13: Toss UX Writing 톤 (런타임)`** — Phase 13 PR 머지 시 entry 추가 (한국어 짧게).
6. **vibe coder A/B test (PM-1 대응)** — Phase 13 PR 머지 후 5명 코호트 톤 선호도 ≥50% Toss 검증, fail 시 stop-loss revert.

---

## 11. Open Questions (Phase 14 planner — NOT Phase 13 blockers)

1. `아이고` 어휘 polling — Phase 13 deprecated 후 vibe coder 1주 feedback, Phase 14 반영
2. CLAUDE.md/AGENTS.md/README.md 한국어 단편 — Phase 14 docs sub-PR 포함 여부
3. CHANGELOG retroactive 정리 — Phase 14 separate sub-PR
4. `codegen-catalog.ts` SoT — Phase 13 가정: catalog.ts == SoT, Phase 14 spec 정리 시 ADR
5. 마케팅 카피 작성자 (Critic D2) — Phase 14 entry criterion
6. Phase 15 SKILL.md description narrative — miss-rate +2%p 시 영구 cancel

---

## 12. ASCII verification proof

```
[Round-1 Critic REJECT 1/5]
        |
        v
[Architect 7 fixes + Critic 5 new defects = 10 required fixes]
        |
        v
+--------------------------------------------------+
| v2 Plan (this document)                          |
|                                                  |
| 1. WebFetch re-verified T-06 3 exceptions        |
|    -> §1 verbatim posted (Architect fix #7)      |
| 2. T-09/T-10 honest "axhub extension" labels     |
|    -> §1 (Architect fix #2)                      |
| 3. T-02 softened (state desc 예외 명시)          |
|    -> §1 (Architect fix #3)                      |
| 4. manifest.test.ts:584 dropped as constraint    |
|    -> §6 + §7 (Architect fix #4)                 |
| 5. Phase 13 = runtime+commands+tests+lint ONLY   |
|    -> §3 (Architect fix #5)                      |
| 6. Lint as PR1 commit-1 prerequisite             |
|    -> §5 (Architect fix #6)                      |
| 7. Synthesis adopted (single-PR + lint-gate)     |
|    -> §4 + §5 (Architect fix #1)                 |
| 8. 취소→닫기 explicit decision-tree rubric       |
|    -> §6 (Critic D1)                             |
| 9. Marketing voice ADR Phase 14 entry criterion  |
|    -> §3 + §10 follow-up #1 (Critic D2)          |
| 10. check-skill-keywords-preserved.ts PR1 prereq |
|    -> §5 (Critic D3)                             |
| 11. Tier E real wc -l (1,471 not 1,000)          |
|    -> §3 (Critic D4)                             |
| 12. fixtures.test.ts +                           |
|     skill-noninteractive-guard.test.ts audited   |
|    -> §7 (Critic D5)                             |
+--------------------------------------------------+
        |
        v
[v2 ready for Architect/Critic round 2]

Phase 13 scope (final):
  +-- Tier A runtime: catalog/keychain*/list-deployments/index.ts (~95 strings)
  +-- Tier C commands: 9 *.md + help.md menu (~30 strings)
  +-- Tier D install/hook: install.{sh,ps1} + session-start.{sh,ps1} (~25 strings)
  +-- Tier G tests: 7 unit/integration test updates (~70 assertions)
  +-- Lint: 2 new scripts + 1 unit test
  TOTAL: ~30-35 files, ~160 source strings, ~70 test assertions
  PR count: 1 (two atomic commits inside)
  Reviewer cycle: 1
  Estimated review time: 4-6 hours

Phase 14 deferred:
  +-- Tier B SKILL workflow + references (~600 strings, 11 SKILL.md)
  +-- Tier E docs (~1,471 KR lines, 8 files — sub-PR split required)
  +-- Tier F CHANGELOG retroactive
  +-- Marketing voice ADR (entry criterion)

Phase 15 deferred (post-measurement):
  +-- SKILL.md description narrative (gated by ≤2%p activation miss-rate)
```

---

**End of v2.** All 10 round-1 fixes addressed. Ready for Architect/Critic round 2.
