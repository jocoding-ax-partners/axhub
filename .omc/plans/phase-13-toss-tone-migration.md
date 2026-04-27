# Phase 13 — Toss UX Writing Tone Migration

> **Mission**: 모든 한국어 user-facing 문구를 Toss UX Writing 가이드 톤으로 일괄 수정
> **Source of truth**: https://developers-apps-in-toss.toss.im/design/ux-writing.html
> **Mode**: DELIBERATE consensus (large surface, subjective tone, string-equality test risk)
> **Author**: planner (RALPLAN-DR loop, pre-Architect/Critic review)

---

## 1. WebFetch findings — Toss UX Writing 가이드 verbatim rules

다음 10개 룰은 가이드 페이지에서 직접 추출한 verbatim 인용을 포함한다.

### Rule T-01. 모든 문구는 해요체 (verbatim)
> "제품 안의 모든 문구는 '해요체'로 써요"
> "상황, 맥락을 불문하고 모든 문구에 해요체를 적용해 주세요."

→ **금지**: `합니다`, `~시겠어요?`, `~시나요?` (예외 후술), `~하십시오`
→ **허용**: `~해요`, `~예요`, `~돼요`, `~할게요`, `~할 수 있어요`

### Rule T-02. 능동형 우선 (verbatim)
> "능동형 문장을 써주세요"

→ ❌ "배포가 됐어요" → ✅ "배포했어요"
→ ❌ "처리가 완료됐어요" → ✅ "처리했어요"

### Rule T-03. 과거형 `~었` 빼기 (verbatim)
> "~었 빼기"

→ ❌ "발생했었어요" → ✅ "발생했어요"
→ ❌ "끊겼었어요" → ✅ "끊겼어요"

### Rule T-04. 부정형 → 긍정형 전환 (verbatim)
> "부정적 커뮤니케이션을 최대한 줄이고 긍정형으로 바꿔주세요"

→ ❌ "지금은 안 돼요" → ✅ "잠시 후 다시 시도해보세요"
→ ❌ "권한이 없어요" → ✅ "권한을 추가하면 사용할 수 있어요"

### Rule T-05. 다이얼로그 왼쪽 버튼은 `닫기` (verbatim)
> "왼쪽 버튼은 [닫기]로 문구를 통일해주세요"

→ ❌ "취소" → ✅ "닫기" (단, AskUserQuestion abort/dismiss 의도일 때만)
→ 예외: "취소"가 의미상 다른 옵션과 명확히 구분되는 destructive abort 인 경우 (e.g. "강제 다운그레이드 / 취소") 는 의도 보존을 위해 유지 가능 — Toss 가이드도 다이얼로그 한정 룰이지 무차별 ban 은 아님.

### Rule T-06. 과도한 경어 제거 (verbatim)
> "~시겠어요?, 시나요?, ~께 같은 과도한 경어를 쓰지 않아요"
> "계시다 → 있다"
> "여쭈다 → 확인하다, 묻다"

→ ❌ "확인하시겠어요?" → ✅ "확인해보세요"
→ ❌ "받으시겠어요?" → ✅ "받을게요"
→ ❌ "고객님께" → ✅ "고객님에게" (또는 호칭 자체 제거)

**예외 (verbatim)**: `시나요?` / `셨나요?` 는 다음 3가지 한정 사용 허용
1. user context 활용해 혼란 줄일 때
2. 상황을 추정할 때 (clear data 부재)
3. user goodwill 요청할 때 (설문 등)

### Rule T-07. 축약형 통일 — `돼요` (verbatim)
> "되어요" → "돼요"
> "모두 '돼요'로 통일해서 써주세요"

→ ❌ "배포가 되어요" → ✅ "배포가 돼요"

### Rule T-08. 명사 스택 회피 (verbatim)
> "'{명사} + {명사}' 쓰지 않기"
> "{명사}가 {명사}해서" 또는 동사형으로 전환

→ ❌ "배포 진행 상태 확인 결과" → ✅ "배포가 진행 중인지 확인해봤어요"
→ ❌ "토큰 만료 시간 초과" → ✅ "토큰 사용 기한이 지났어요"

### Rule T-09. 호칭 — 액션 중심, 직접 지칭 회피
가이드는 명시적 호칭을 거의 쓰지 않는다 ("당신" 미사용, "고객님" 도 일부 한정). axhub 의 현재 `당신 앱은 안전합니다` → 직접 호칭 제거 후 액션·상태 중심 재진술.

→ ❌ "당신 앱은 안전합니다" → ✅ "앱은 그대로 잘 돌아가고 있어요"
→ ❌ "당신이 잘못한 것이 아닙니다" → ✅ "잘못한 건 없어요"

### Rule T-10. 에러 메시지 구조 (가이드 명시적 패턴)
가이드는 에러 패턴을 verbatim 으로 명시하지 않으나, 부정형 회피 + 긍정 재구성 + 액션 경로 제시 원칙을 추출 가능.

→ **Toss-style error 3-part 권장 형태**:
1. **What** — 무엇이 일어났는지 (긍정 진술, 짧게)
2. **Why** — 사용자 책임 아닌 이유 (선택적, 짧게)
3. **Action** — 다음에 할 자연어 한 문장 (능동형)

→ axhub 의 현행 4-part (감정 / 원인 / 해결 / 다음) 구조와 비교: **Phase 11 4-part 는 `감정` 한 줄을 명시 분리하는 점에서 Toss 보다 더 verbose**. 다만 emotion prefix 자체는 Toss 와 충돌하지 않으며 (Toss 도 `잠깐만요`, `괜찮아요` 사용), 4-part 가 Toss 의 빈 칸을 메우는 axhub-specific extension 으로 정당화 가능. → **유지 권장**.

---

## 2. Tone delta table — current axhub 보이스 vs Toss 타깃

| 차원 | 현재 axhub | Toss 타깃 | 영향 |
|---|---|---|---|
| **Ending form** | `해요체` 주류 + `합니다체` 산발 ("당신 앱은 안전**합니다**", "성공적으로 끝**났습니다**", "작동**합니다**") | 100% `해요체` | 모든 `~합니다`, `~입니다` → `~예요`, `~해요` 변환. 약 80개소. |
| **Honorific** | `~시겠어요`, `~해주세요`, `해주실래요` 혼용 | `시나요?` 3가지 예외만, 평소 제거 | "배포하시겠어요?" → "배포할까요?". "확인하시겠어요?" → "확인해보세요". 약 40개소. |
| **호칭** | "당신", "고객님", "본인" 혼재 ("**당신** 앱은 안전합니다" 11회 카탈로그 + 4회 docs) | 액션 중심, 직접 호칭 회피 | "당신 앱은" → "앱은" 또는 "현재 앱은". 약 25개소. |
| **부정형** | "안 됐어요", "없어요", "막혔어요", "실패했어요" 빈번 | 긍정 재구성 ("잠시 후 다시", "~하면 가능해요") | 상당수 cause 문장. 약 60개소. |
| **명사 스택** | `배포 요청 검증 실패` 같은 4-noun 스택 다수 | 동사형 분해 | catalog.ts cause 문장 다수, 약 30개소. |
| **Punctuation** | `.` 마침표 + `!` 빈번 사용 + `—` em-dash 자주 등장 | 마침표 절제, `!` 축하 한정, em-dash 무난 | 거의 그대로 유지. `!` → `.` 일부 (강조 과잉만). |
| **Number/unit** | 한·영 혼용 ("12분 전", "5단계", "100개", "v0.1.5") | 가이드 미명시 → 현행 유지 | 변경 불요. |
| **Emotion prefix (감정)** | `잠깐만요`, `아이고`, `죄송해요`, `축하해요` (Phase 11 4-part 의 핵심) | Toss 도 `잠깐만요` 사용 패턴 있음. `아이고` 는 가이드 미언급 → 위험. `죄송해요` 는 책임 자기화로 Toss 정신과 부합. | `아이고` → `잠깐만요` 또는 `이상해요` 로 통일. 4-part 자체는 보존. |

---

## 3. Surface inventory — 한국어 string sites (count + 분류)

총 **18개 SKILL/reference 파일 + 7개 src 파일 + 9개 commands + 4개 bin/hooks + 5개 docs + 1개 CHANGELOG + 6개 test 파일** = 약 **50개 파일 / 1,500+ 한국어 문자열**.

### Tier A — 사용자가 직접 보는 런타임 메시지 (highest priority)
`tone migration 의 핵심. 1차 마이그레이션 대상`

| 파일 | 라인 범위 | 문구 종류 | 개수 |
|---|---|---|---|
| `src/axhub-helpers/catalog.ts` | L18–L133 | 4-part empathy templates (exit code 별) | 13 entries × 4 fields = **52 strings** |
| `src/axhub-helpers/keychain.ts` | L59–L130 | macOS/Linux keychain 4-part errors | **7 messages × 4 lines** |
| `src/axhub-helpers/keychain-windows.ts` | L100–L128 | Windows 5종 4-part errors | **5 messages × 4 lines** |
| `src/axhub-helpers/list-deployments.ts` | L126, L144, L166, L176, L186, L196, L209 | API call 1-line errors | **7 strings** |
| `src/axhub-helpers/index.ts` | L194, L259, L345–347, L426, L434, L456 | cmdSessionStart, consent gate, cmdToken | **6 strings** |
| `src/axhub-helpers/preflight.ts` | (CLI version range errors — survey 결과 0 hits → 별도 확인 필요) | — | **0 verified** |

**Subtotal Tier A: ~77 strings**

### Tier B — Skill descriptions + workflow narration (activation 영향)
`vibe coder 발화 매칭 keyword 가 description 안에 박혀 있어 변경 시 activation regression 우려.`

11개 SKILL.md (apis/apps/auth/clarify/deploy/doctor/logs/recover/status/update/upgrade) 각 L3 description (10–18개 phrase + narrative) + workflow body (KR card 렌더, AskUserQuestion KR labels). 추가로 references: `apis/references/privacy-filter.md` (cross-team consent), `deploy/references/error-empathy-catalog.{md,generated.md}` (4-part spec), `headless-flow.md` (token-paste 5-step), `recovery-flows.md` (recovery menu), `nl-lexicon.md` (665 lines — **변경 금지**, 순수 user 발화 카탈로그).

**Subtotal Tier B: ~600+ Korean strings, lexicon 제외**

### Tier C — Slash command descriptions (활성화 노출)
| 파일 | 종류 |
|---|---|
| `commands/apis.md` `apps.md` `deploy.md` `doctor.md` `help.md` `login.md` `logs.md` `status.md` `update.md` | 각 description 한 줄 + help.md 는 21줄 menu |

**Subtotal Tier C: ~30 strings**

### Tier D — Install / hook 시스템 메시지 (한 번만 보지만 첫인상)
| 파일 | 라인 |
|---|---|
| `bin/install.sh` | L19, L29, L36, L53–L72 (~12 messages) |
| `bin/install.ps1` | L24–L103 (5 multi-line errors) |
| `hooks/session-start.sh` | L25, L29 (2 messages) |
| `hooks/session-start.ps1` | L20–L114 (6 multi-line systemMessage) |

**Subtotal Tier D: ~25 strings**

### Tier E — Docs (긴 글, 자체 톤 일관성 더 중요)
| 파일 | 라인 수 |
|---|---|
| `docs/troubleshooting.ko.md` | 284 lines, ~121 KR lines |
| `docs/vibe-coder-quickstart.ko.md` | 234 lines, ~107 KR lines |
| `docs/org-admin-rollout.ko.md` | 294 lines, ~179 KR lines |
| `docs/marketing/landing-page.ko.md` | (estimated ~200 lines) |
| `docs/marketing/outreach-email.ko.md` | (estimated ~100 lines) |
| `docs/pilot/admin-rollout.ko.md` | (estimated ~150 lines) |
| `docs/pilot/onboarding-checklist.ko.md` | (estimated ~80 lines) |
| `docs/pilot/feedback-template.ko.md` | (estimated ~60 lines) |

**Subtotal Tier E: ~1,000+ KR lines**

### Tier F — CHANGELOG / release notes (이력 — 가벼운 정리만)
`CHANGELOG.md` 5개 KR mention (대부분 release note 본문 자체)

### Tier G — 테스트 fixture / assertion (string-equality 위험)
| 파일 | 영향 |
|---|---|
| `tests/keychain.test.ts` | L77–L87 — `원인:` `해결:` `다음:` markers + emotion prefix regex (`잠깐만요\|아이고\|죄송해요`) |
| `tests/keychain-windows.test.ts` | L100, L101, L104 — `보안 솔루션`, `코드사이닝`, `관리자에게` toContain |
| `tests/classify-exit.test.ts` | L7–L… — 39 KR `toContain` assertions (entire empathy catalog) |
| `tests/list-deployments.test.ts` | L63 — `토큰을 찾을 수 없어요` toContain |
| `tests/manifest.test.ts` | L584 — `이 스킬` prefix regex per SKILL.md description |
| `tests/codegen.test.ts` | L46–L92 — `**감정:**`, `**원인:**`, `**해결:**`, `**버튼:**` markdown markers |
| `tests/session-start-ps1.test.ts` | L53 — `보안 솔루션` toContain |
| `tests/axhub-helpers.test.ts` | L56–L298 — 14 fixture lines with KR slugs/messages (`결제 페이지 버그 수정` 등) |

**Subtotal Tier G: ~70 test assertions, 모든 tone change 마다 동기 update 필요**

---

## 4. Migration options A/B/C/D

### Option A. Big-bang rewrite all files in single PR
**핵심**: 한 PR 에서 Tier A–G 전부 마이그레이션. 단일 ADR + 단일 reviewer 통과.

- **Pros**:
  - 톤 일관성 100% 보장 (중간 단계 없음)
  - lexicon vs description vs catalog 간 cross-reference drift 없음
  - 한 번의 test mock update — 작업자 인지 cost 최소
  - CHANGELOG 한 줄로 사용자 communication 완료
- **Cons**:
  - PR diff 거대 (~50 파일, 1,500+ 라인 변경) → reviewer fatigue
  - rollback 시 단일 commit revert = 모든 톤 변화가 사라짐 (회복은 깔끔하나 부분 보존 불가)
  - test mock 업데이트 누락 시 CI red 폭발
  - 한 명의 라이터가 톤 일관성 통제 가능한 인지 한계 (~500 strings) 초과 → 일관성 risk
  - QA 어려움 — 모든 user flow 한꺼번에 스모크해야 함

### Option B. Tiered rollout (errors first → skills → docs)
**핵심**: 4 PR sequence. (1) Tier A+G+Tier-D-runtime errors, (2) Tier B SKILL workflow copy (description 제외), (3) Tier B description + Tier C commands (활성화 영향 격리), (4) Tier E+F docs.

- **Pros**:
  - 각 PR diff 합리적 크기 (~12–15 파일)
  - 활성화 영향 (Tier B description) 을 별도 PR 로 격리 → drift 발생 시 즉시 revert 가능
  - 각 단계 후 통합 smoke test 가능 (PR1 = error path, PR2 = success path, PR3 = onboarding, PR4 = docs)
  - 작업자 인지 부담 분산
  - CHANGELOG 4번 entry — 사용자가 점진 변화를 인지
- **Cons**:
  - PR1 머지 후 PR2 머지 전까지 catalog.ts (Toss 톤) 와 SKILL workflow (구 톤) 가 mixed → 같은 user session 안에서 톤 불일치 노출
  - 4 PR × 각 reviewer cycle = 시간 비용 증가 (1주일 → 3–4주)
  - lexicon ↔ description drift 가 PR boundary 마다 검증 필요
  - tier 간 의존성 (e.g. error-empathy-catalog.md spec 변경이 catalog.ts 에 반영) 해결 순서 신경써야 함

### Option C. Style guide + opt-in rewrite per file as touched
**핵심**: 별도 코드 변경 PR 없이 `docs/STYLE_GUIDE.ko.md` 만 추가. 이후 다른 PR (feature, bug fix) 에서 해당 파일 만질 때마다 작업자가 톤 마이그레이션 함께 반영.

- **Pros**:
  - 0 risk PR — style guide 만 추가, 코드 무변경
  - 자연스러운 마이그레이션 — 활발한 파일부터 변경, 죽은 파일은 그대로
  - 테스트 깨짐 zero (옮기지 않으니까)
  - reviewer cost 분산
  - **vibe coder regression risk = 0** (아무것도 안 바꿈)
- **Cons**:
  - 톤 일관성 영원히 100% 도달 불가 — 항상 mixed
  - 특정 파일 (잘 안 만지는 catalog.ts 일부 entry) 은 1년 뒤에도 구 톤
  - "Toss 톤으로 통일했다" 라고 사용자에게 communicate 불가
  - 작업자마다 해석 편차 → 미세한 톤 drift
  - 마케팅 landing/outreach 같은 1회성 파일은 absorb 안 됨 → 결국 별도 PR 필요

### Option D. Two-tone hybrid (Toss for new-user surfaces, keep current for power-user error catalog)
**핵심**: vibe coder 첫인상 surface (commands/help.md, install scripts, vibe-coder-quickstart.ko.md, /axhub:help, skill descriptions) 만 Toss 마이그레이션. 4-part empathy catalog (catalog.ts, keychain*.ts, error-empathy-catalog.md) 는 Phase 11 톤 보존 — power user 가 디버깅 시 보는 정보 밀도 높은 메시지.

- **Pros**:
  - 4-part empathy template 의 emotional warmth (`잠깐만요. 당신 앱은 안전합니다.`) 보존 → 11pm demo crisis persona 유지
  - keychain 4-part error 의 information density (cause/solve/next) 보존 — Tier A test 깨짐 최소
  - vibe coder 첫 5분 surface 만 Toss tone → 마케팅 메시지 일관 ("Toss 톤으로 만들었어요")
  - lexicon 변경 0
  - test mock update 분량 ~15개로 감소
- **Cons**:
  - "전체를 Toss 톤으로" 미션 자체와 어긋남 — partial migration 으로 분류
  - 사용자가 같은 세션에서 두 톤 경험 (help → Toss, 에러 발생 → Phase 11) → 인지 mismatch
  - "어디까지가 Toss 고 어디서부터 power-user 톤인가" 경계 정의 자체가 다음 phase 부담
  - 마케팅 위치 "Toss 톤" 이지만 사용자가 실제 디버깅 진입 시 mismatch 노출 — credibility 위험

### 추천 (planner 권장 → Architect 검토 대상)

**Option B (Tiered rollout)** 권장. 이유:
1. Big-bang (A) 는 인지 한계 초과 — 1,500 strings 단일 PR 에서 톤 일관성 통제 불가능
2. Style guide only (C) 는 미션 ("일괄 수정") 위배
3. Hybrid (D) 는 미션 ("모든 한국어") 위배 + 사용자 mismatch
4. Tiered (B) 는 미션 충족 + 인지 부담 분산 + tier 간 cross-check 가능

다만 **PR1 (errors first) 은 Option D 의 위험을 일부 빌려옴** — empathy 4-part 의 emotion prefix 는 보존하되 honorific/ending/호칭만 Toss 화. 즉 B 의 PR1 = "구조 보존, 톤 변경" 으로 정의.

---

## 5. Pre-mortem (DELIBERATE 4 시나리오)

### Scenario PM-1. 4-part empathy template 의 warmth 손실
**가상 시나리오**: PR1 머지 후 catalog.ts 가 Toss 톤으로 바뀜. `잠깐만요. 일시적인 통신 문제예요. 당신 앱은 안전합니다.` → `잠시 통신 문제가 있어요. 앱은 그대로예요.`. 11pm demo persona 사용자가 빨간 글씨를 보고도 "axhub 가 나를 안심시켜줬다" 는 감정을 못 느낌. NPS 가 dropped, 사용자 인터뷰에서 "예전 메시지가 더 따뜻했다" 피드백.

**대응**:
- emotion prefix (`잠깐만요`, `괜찮아요`) **반드시 보존**
- "당신 앱은 안전합니다" → "앱은 그대로 잘 돌아가고 있어요" (호칭만 제거, 안심 의미 보존)
- Architect 가 PR1 review 시 "안심 어휘 retention rate" 체크 — 각 4-part 의 안심 정보가 신·구 모두 존재하는지 verify
- Tier A 마이그레이션 직후 5명 vibe coder 사용자에게 신·구 메시지 A/B test 노출 → tone 선호도 ≥50% Toss-side 일 때만 PR2 진입

### Scenario PM-2. Skill keyword drift — 자연어 활성화 깨짐
**가상 시나리오**: PR3 에서 `skills/auth/SKILL.md` description 에서 "토큰 만료됐어" 가 Toss 톤화로 "토큰 사용 기한이 지났어" 로 바뀜. 사용자 발화 "토큰 만료됐어" 가 더 이상 description 에 없어 Claude 의 skill activation 휴리스틱이 deploy 또는 clarify 로 잘못 라우팅. vibe coder 가 "왜 로그인 안 시켜?" 라고 frustrated.

**대응**:
- **Description 의 KR phrase list (`다음 표현에서 활성화: "..."`) 는 lexicon 카탈로그로 취급 — 변경 금지**
- Description 의 narrative 부분 ("이 스킬은 사용자가...") 만 Toss 톤화
- `nl-lexicon.md` 와 동일 정책 — 발화 카탈로그는 사용자 자연어이지 axhub voice 가 아님
- PR3 시 자동 검사 추가: description 추출된 따옴표 안 phrase 가 PR 전후 동일한지 diff 검증 (`scripts/check-skill-keywords-preserved.ts`)

### Scenario PM-3. Test breakage — string-equality assertion 폭발
**가상 시나리오**: PR1 머지 직후 CI 가 38개 test fail (`classify-exit.test.ts` 39 assertion 중 38 깨짐). 작업자가 mock 업데이트를 누락. main branch 가 red 상태로 24h, 다른 PR 머지 못함.

**대응**:
- PR 작성 시 **반드시 동시 commit**: catalog.ts 변경 + classify-exit.test.ts mock 변경 (한 commit 내)
- pre-commit hook: catalog.ts diff 가 있으면 `bun test tests/classify-exit.test.ts` 자동 실행, fail 시 commit block
- 각 PR 의 success criteria 에 "관련 test 100% green" 명시
- CI fail 시 즉시 revert policy — 24h 룰 적용

### Scenario PM-4. Test mock regression — mock 만 update, source 미변경
**가상 시나리오**: 작업자가 `classify-exit.test.ts` 의 expected 만 새 톤으로 바꾸고 catalog.ts 변경을 깜빡함. 모든 test 통과 (mock 자체가 source 와 일치) 하지만 실제 사용자에게는 구 톤 노출. PR3 머지 후 사용자가 "아무것도 안 바뀌었는데" 라고 보고.

**대응**:
- 각 PR 에 **수동 verify checklist** — `axhub-helpers classify-exit --code 65` 실행 후 stdout 캡처해 PR description 에 paste (실제 source 변경 증거)
- snapshot test 추가 — `bun test:snapshot` 으로 catalog.ts → markdown 렌더링 결과 비교
- PR description template 에 "실제 톤 before/after 5개 샘플" 필수 섹션 추가
- code-reviewer agent 에 verify 단계 추가 — `git grep "당신 앱"` 실행해 잔존 0 확인

---

## 6. Test plan (file-by-file impact + 필요 update)

### Unit tests
| Test file | Source dependency | Update 필요 | 카운트 |
|---|---|---|---|
| `tests/classify-exit.test.ts` | `catalog.ts` | 39개 `toContain` assertion 모두 신규 톤으로 교체 | 39 |
| `tests/keychain.test.ts` | `keychain.ts` | emotion regex `(잠깐만요\|아이고\|죄송해요)` → `(잠깐만요\|이상해요\|죄송해요)` (아이고 deprecated). cause/solve/next markers 유지 (구조 변경 X) | 6 |
| `tests/keychain-windows.test.ts` | `keychain-windows.ts` | `"보안 솔루션"`, `"코드사이닝"` toContain — Toss 톤화 후에도 어휘 유지 verify | 4 |
| `tests/list-deployments.test.ts` | `list-deployments.ts` | `"토큰을 찾을 수 없어요"` 유지 또는 Toss 변형 ("axhub 토큰이 없어요") 시 update | 1 |
| `tests/codegen.test.ts` | `error-empathy-catalog.generated.md` | `**감정:**` `**원인:**` 등 4-part marker — 구조 보존이라면 변경 불요 | 0 (구조 보존 시) |
| `tests/manifest.test.ts` | 모든 `SKILL.md` description | `^(This skill\|이 스킬)` regex 유지 → 첫 단어 변경 금지 | 0 (constraint) |
| `tests/session-start-ps1.test.ts` | `hooks/session-start.ps1` | `"보안 솔루션"` toContain — 어휘 유지 verify | 1 |
| `tests/axhub-helpers.test.ts` | `index.ts` cmdResolve 등 | KR slug fixture 14개 — 사용자 발화 fixture 라 변경 X | 0 (fixture as-is) |

### Integration tests
- `tests/run-corpus.sh` — corpus.jsonl 의 사용자 발화는 사용자 voice 이므로 변경 X
- `tests/install.test.sh` — install.sh 의 KR error 메시지 변경 시 grep 패턴 update (~5)
- `tests/install-ps1.test.ts` — install.ps1 KR 메시지 grep update (~5)

### E2E
- 실제 helper 바이너리 빌드 후 `axhub-helpers classify-exit --code 65` 등 실행 → stdout 검증 (각 PR 마다 수동)
- vibe coder 5명 A/B 톤 노출 (PM-1 대응)

### Observability
- `~/.cache/axhub-plugin/empathy-catalog.ndjson` 에 신·구 톤 emit 비율 로깅 (1주 모니터링)
- skill activation telemetry — PR3 머지 전후 skill miss-rate 비교 (PM-2 검증)

---

## 7. PRD stories US-1301 ~ US-1308

### US-1301. 4-part empathy 카탈로그 Toss 톤 변환 (Tier A 핵심)
**As a** vibe coder seeing a deploy error at 11pm before demo
**I want** the error message to use Toss-style 해요체 with 능동형 + 긍정형 framing
**So that** I feel calm and act on the next step without parsing 합니다체 formality
**Acceptance**:
- `src/axhub-helpers/catalog.ts` 13 entries 모두 (a) 어미 `해요체` (b) 호칭 직접 지칭 0회 (c) 부정형 → 긍정형 재구성 (d) emotion prefix 보존 (잠깐만요/괜찮아요/축하해요)
- `tests/classify-exit.test.ts` 39 assertion 모두 신규 톤으로 update 후 green
- `scripts/codegen-catalog.ts` 재실행 후 `error-empathy-catalog.generated.md` 동기화
- 실제 `axhub-helpers classify-exit --code 65` stdout 캡처 PR 첨부

### US-1302. Keychain 에러 4-part Toss 톤 (Tier A 보조)
**As a** Mac/Linux/Windows vibe coder hitting keychain failure
**I want** the secret-storage error to follow Toss casual tone
**So that** I do not feel the system is shouting in formal Korean
**Acceptance**:
- `keychain.ts` 4 errors + `keychain-windows.ts` 5 errors → Toss 톤
- `아이고` 어휘 deprecated → `이상해요` 또는 `잠깐만요` 통일
- `tests/keychain.test.ts` regex update + green
- `tests/keychain-windows.test.ts` 어휘 검증 (보안 솔루션, 코드사이닝) 유지 + green

### US-1303. List/auth/index 1-line errors Toss 톤
**As a** vibe coder hitting transient API errors
**I want** short error lines to be Toss casual
**So that** UX 는 일관 톤 유지
**Acceptance**:
- `list-deployments.ts` 7 strings + `index.ts` 6 strings 모두 Toss 톤
- `tests/list-deployments.test.ts` toContain update + green
- 모든 helper subcommand stdout sample PR 첨부

### US-1304. Skill workflow narration Toss 톤 (description 제외)
**As a** Claude reading SKILL.md to render a workflow message
**I want** the workflow steps and Korean copy snippets to follow Toss tone
**So that** the rendered AskUserQuestion options/labels feel consistent with command output
**Acceptance**:
- 11 SKILL.md 의 workflow body 변환 (description L3 만 격리 보존)
- AskUserQuestion `label` Toss 화 (e.g. "다시 로그인할까요?" → 유지 가능, "취소" → "닫기" Rule T-05 적용 단 abort 의도일 때만)
- references/*.md 도 동일 적용 (단 nl-lexicon.md 제외)
- description 안의 따옴표 phrase list 0 변경 (PM-2 대응) — 자동 검사 스크립트 실행 결과 PR 첨부

### US-1305. SKILL.md description narrative 부분 Toss 톤 + activation phrase preservation
**As a** Claude doing skill activation routing
**I want** the description's narrative ("이 스킬은 사용자가...") to be Toss tone but keyword phrases preserved
**So that** vibe coder utterance matching keeps the same recall while planner/agent reading the description gets Toss-aligned voice
**Acceptance**:
- 11 SKILL.md description 변환
- 따옴표 안 KR phrase 100% 유지 (diff verify)
- `tests/manifest.test.ts` `이 스킬` prefix regex 그대로 통과 (Toss 화 후에도 첫 단어 보존)
- skill activation 정확도 PR3 머지 후 1주 모니터링 — miss-rate 변화 ≤2%p

### US-1306. Slash command descriptions + help.md menu Toss 톤
**Acceptance**:
- 9 commands/*.md description 한 줄씩 Toss 화
- help.md 21줄 menu 전체 Toss 화 (자연어 예시 따옴표는 발화 lexicon — 보존)
- 실제 `/axhub:help` 출력 PR 첨부

### US-1307. Install/hook 시스템 메시지 Toss 톤
**Acceptance**:
- `bin/install.sh` 12 messages + `bin/install.ps1` 5 multi-line errors + `hooks/session-start.{sh,ps1}` 8 messages
- `tests/install.test.sh` + `tests/install-ps1.test.ts` + `tests/session-start-ps1.test.ts` grep pattern update + green
- 4-part 구조 유지 (원인/해결/다음) — Toss tone × 정보 밀도 trade-off 보존

### US-1308. Docs Toss 톤 일괄 정리
**As a** new vibe coder reading docs/vibe-coder-quickstart.ko.md
**I want** the docs to match the runtime tone exactly
**So that** I do not learn one tone in docs and meet another in CLI
**Acceptance**:
- 5 docs (`vibe-coder-quickstart`, `troubleshooting`, `org-admin-rollout`, marketing/, pilot/) 전체 Toss 변환
- README.md, AGENTS.md, CLAUDE.md 한국어 부분 (있는 경우) 변환
- CHANGELOG entry "Phase 13: Toss UX Writing 톤으로 한국어 문구 일괄 정리" 한국어 요약 추가
- docs-link-audit.sh 통과

---

## 8. ADR — Architecture Decision Record

### Decision
**Tier-rollout 4 PR 시퀀스 (Option B)** 로 axhub plugin 의 모든 한국어 user-facing 문구를 Toss UX Writing 가이드 톤으로 마이그레이션. 단 (a) Phase 11 4-part empathy 구조 (감정/원인/해결/다음) 는 보존하고 어휘만 Toss 화, (b) `nl-lexicon.md` 와 SKILL.md description 안의 발화 카탈로그 phrase 는 변경 금지.

### Drivers
1. **vibe coder UX consistency** — 첫 5분 surface (install/help/quickstart) 와 디버깅 surface (catalog/keychain) 의 톤 일치는 trust thesis 의 핵심
2. **Toss 가이드의 industry-standard 위치** — 한국 vibe coder 가 가장 익숙한 톤. 자체 내부 톤 가이드보다 이미 학습된 expectation 활용
3. **Phase 11 emotional warmth ROI 보존** — Phase 11 에서 감정/원인/해결/다음 4-part 가 11pm crisis persona 의 NPS 를 유의하게 올린 evidence 가 있음 — 이걸 폐기하지 않음
4. **Test 안정성** — 70+ string-equality test 가 산재. tier 분리로 한 PR 당 mock update ≤20 으로 인지 부담 통제

### Alternatives considered
- **A. Big-bang single PR** — 1,500 strings 단일 PR 은 reviewer 인지 한계 초과. test mock 누락 시 main red. 일관성 통제 가능하지만 운영 risk 가 보상 초과.
- **C. Style guide only** — 미션 ("일괄 수정") 미충족. 1년 후에도 mixed. communicate 불가.
- **D. Two-tone hybrid** — power-user vs vibe-coder 경계 정의 자체가 다음 phase 부담. 사용자가 같은 세션에서 두 톤 경험 → credibility 위험.
- **E. (논외) "Toss 톤 부분 적용"** — 미션 위배.

### Why chosen
- B 는 (가) 미션 충족 (나) test 안정성 (다) 인지 부담 분산 (라) 톤 검증 사이클 가능 (PR1 후 5명 A/B test) (마) 부분 revert 가능 — 5축 모두 우위.
- 4-part 보존 결정은 Toss 가이드가 "감정 prefix 자체를 금지" 하지 않으며 (오히려 `잠깐만요` 사용 패턴 명시) Phase 11 의 검증된 ROI 를 retain 가치가 더 크다는 판단.
- Description phrase 보존 결정은 PM-2 의 silent activation drift 가 user-visible 한 모든 톤 작업의 이득을 한 번에 무력화할 위험이기 때문.

### Consequences
**Positive**:
- vibe coder 가 docs → install → 첫 명령 → 첫 에러 모두 일관 Toss 톤 경험
- 마케팅 메시지 "Toss UX Writing 가이드 준수" communicate 가능 — credibility 상승
- Phase 11 emotion prefix 보존으로 감정 ROI 지킴
- 신규 SKILL/카탈로그 추가 시 톤 기준 명확 — 작업자 인지 부담 감소

**Negative**:
- 4 PR cycle 로 마이그레이션 완료까지 3–4주 소요
- PR1–PR2 사이 transient 톤 mix (catalog Toss + skill workflow 구톤) — 2주간 사용자 노출
- Description narrative vs phrase 분리 정책이 작업자에게 학습 부담 — onboarding 문서 필요
- `아이고` 어휘 deprecated → 일부 사용자가 인지하던 emotion prefix 사라짐

**Risk acceptance**:
- A/B test 결과 톤 선호도 50%↓ Toss-side 일 경우 PR1 rollback + 재기획. 미리 stop-loss 정의.
- skill activation miss-rate +2%p↑ 시 PR3 rollback + description 전수 검사.

### Follow-ups
1. `docs/STYLE_GUIDE.ko.md` 신규 — Toss 톤 + axhub 4-part extension 의 통합 작성 가이드. PR4 에 포함.
2. `scripts/check-skill-keywords-preserved.ts` 신규 — description phrase diff 자동 검사. PR3 전제.
3. `scripts/check-toss-tone-conformance.ts` 신규 — `합니다`, `당신`, `~시겠어요` 잔존 0 확인 lint. PR4 전제.
4. PR1 직후 5명 vibe coder A/B test orchestration — Phase 13.5 user research 별도 mission.
5. CHANGELOG `Phase 13: Toss UX Writing 톤 일괄 정리` entry — PR4 에 포함.
6. AGENTS.md / CLAUDE.md 의 한국어 단편 — 작업자용이므로 Tier E 와 별도 PR (PR4 후속) 로 분리 가능.

---

## 9. Open Questions (Architect/Critic 검토 대상)

다음 사항들은 본 plan 에서 명시 결정하지 않았으므로 Architect 단계에서 결론 필요:

1. **AskUserQuestion `취소` → `닫기` 일괄 변환 여부** — Rule T-05 는 다이얼로그 한정이지만 axhub 의 AskUserQuestion 은 모달성격이라 적용 적절. 단 "강제 다운그레이드 / 취소" 같은 destructive abort 옵션은 의미상 "닫기" 가 부적절 — 예외 정책 필요.
2. **`아이고` 어휘 polling** — 가이드 미언급 어휘. 일부 사용자가 emotional warmth 로 인지할 가능성. PM-1 의 A/B test 에 포함시킬지 여부.
3. **CLAUDE.md / AGENTS.md / README.md 한국어 부분** — vibe coder 에게 안 보이는 작업자용 문서. Tier E 포함 여부.
4. **CHANGELOG 한국어 release notes** — 이력 텍스트라 정리 비용 vs 가치 trade-off.
5. **`scripts/codegen-catalog.ts` 의 spec-source** — `error-empathy-catalog.md` (수동 spec) vs `catalog.ts` (코드) 중 어느 쪽이 source of truth? 톤 변경 시 양쪽 동기화 순서 확정 필요.
6. **마케팅 카피 (`docs/marketing/landing-page.ko.md` 등) 의 작성자** — 제품 톤 vs 마케팅 톤 분리 기준이 있는가? Toss 도 마케팅과 product UX 의 톤이 약간 다름.
