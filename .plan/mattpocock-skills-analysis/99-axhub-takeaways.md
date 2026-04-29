# 99. axhub Takeaways — 비교 / 흡수 가능 패턴 / 충돌 위험

axhub repo 도 Claude Code 용 skill plugin 이에요 — mattpocock/skills 와 same primitives, different scale + maturity. 이 doc 은 axhub 작업자가 어느 패턴을 그대로 흡수할지, 어느 부분에 axhub 가 이미 더 강한지, 어느 패턴이 axhub 에 잘못 적용될 위험인지 정리해요.

---

## 두 repo 한 줄 비교

| 차원 | mattpocock/skills | axhub |
|---|---|---|
| **Plugin 성격** | 일상 엔지니어링 도구 22개 | 1 회사 (axhub) deploy / status / logs / apps / apis / auth 등 ops + 도구 |
| **SKILL 수** | 22 (12 distributed) | ~13 (한국어, axhub:* prefix) |
| **언어** | 영어 only | 한국어 (해요체 강제) + 일부 영어 |
| **Build/test** | 없음 — markdown only | bun + TypeScript + 498 test + Cargo + Rust crate + cosign 서명 |
| **CI** | 없음 (LICENSE, plugin.json, link-skills.sh 만) | rust-ci.yml + lint:tone --strict + lint:keywords --check + skill:doctor |
| **Release** | 없음 (no version) | commit-and-tag-version + cosign + 5 cross-arch binary |
| **Skill 작성 도구** | `write-a-skill` (가이드 only) | `bun run skill:new <slug>` scaffold (강제) + `bun run skill:doctor` |
| **Lint baseline** | 없음 | tone (해요체) + keywords (nl-lexicon) + ask-defaults registry |
| **Frontmatter 강제** | name + description + (선택) disable-model-invocation | + multi-step + needs-preflight |
| **Setup 모델** | 1번만 (`/setup-matt-pocock-skills`) | login OAuth, doctor 진단 |
| **Hooks** | 단일 misc skill (git guardrails) | 다수 PostToolUse 훅 (gitnexus auto-update 등) |

axhub 가 **테스트/lint/release 자동화 + 한국어 톤 강제** 측면에서 훨씬 더 무거워요. mattpocock 은 markdown-only minimalism.

---

## 패턴 1: 흡수 가능 (그대로 또는 약간 변형)

### 1.1 Vertical slice = tracer bullet (TDD + to-issues)

axhub 는 phase 별 marker (Phase 22.5, 22.4 등) 진행 중. 새 feature 작업 시:
- mattpocock 패턴을 차용해서 "한 vertical slice = 한 commit = 한 PR" 강제 가능.
- 현재 axhub `.plan/ouroboros-analysis/` 가 phase 별 분할 — 이미 비슷한 정신.

**Action**: phase 안에서 vertical slice 단위 issue 분해 시 `to-issues` 의 HITL/AFK 라벨 + Blocked-by 그래프 차용. axhub `tests/fixtures/` 와 통합 가능.

### 1.2 Lazy file creation + silent on absence

axhub 는 `bun run skill:new` 가 항상 stub 강제 — mattpocock 은 lazy.
axhub 의 모드는 "consistency upfront", mattpocock 모드는 "create when needed".

**Action**: axhub 가 의도적 — scaffold 강제는 D1 sentinel / TodoWrite Step 0 / preflight injection 누락 방지. mattpocock 의 lazy 가 axhub 의 강제와 충돌. 흡수 안 함, 차이 인지.

### 1.3 Durable, not procedural (file path / line number 금지)

axhub 의 issue / PRD / 작업 plan 작성 시 직접 적용 가능:
- `.plan/ouroboros-analysis/` 는 file path 자주 참조 (의도된 — implementation guide)
- 그러나 long-lived issue 또는 cross-phase plan 은 file path 회피.

**Action**: AGENT-BRIEF 패턴을 axhub `tests/fixtures/ask-defaults/registry.json` 같은 config 영역에 적용 — interface (channel name, safe_default, rationale) 묘사, file path 안 묘사.

### 1.4 Mock at boundaries only

axhub 의 Rust crate / TypeScript dual codebase 가 boundary 명확:
- `crates/` ↔ `src/` 사이 boundary 만 mock
- `axhub-helpers/` 내부는 real call

**Action**: mattpocock `tdd/mocking.md` 의 SDK-style API 권장 — axhub 의 `axhub:apps`, `axhub:apis`, `axhub:status` 가 이미 SDK-style (각 endpoint 별 명령). 패턴 일치, 강화 권장.

### 1.5 Disable-model-invocation 활용

axhub SKILL 들 (deploy, recover, update, upgrade, doctor 같은 destructive / multi-step) 은 사용자 의도 강한 task — `disable-model-invocation: true` 고려할만 해요.

현재 axhub `multi-step: true` + `needs-preflight: true` 가 비슷한 가드. 그러나 frontmatter `disable-model-invocation` 추가 시 추가 layer (agent 자동 trigger 방지) — 안전장치.

**Action**: destructive multi-step skill (axhub:deploy, axhub:recover, axhub:update) 에 `disable-model-invocation: true` 추가 검토. 기존 안전 가드 위에 layer.

### 1.6 Inline doc update (decision crystallize 시 즉시)

axhub `.omc/adr/` 디렉토리 이미 존재 — mattpocock `docs/adr/` 와 같은 패턴. mattpocock ADR-FORMAT 의 minimal 1-paragraph 권장이 적용 가능:
- 현재 axhub ADR 형식 모르지만, mattpocock 의 "3 모두 true 일 때만" 가이드 차용 가능.

**Action**: `.omc/adr/` ADR 작성 시 mattpocock 3 조건 (hard to reverse / surprising / real trade-off) 사용. minimal 1 paragraph 우선 — section ceremony 거부.

### 1.7 Iterate the loop itself (diagnose Phase 1)

axhub test runner (`bun test`) 가 이미 빠름. 그러나 mattpocock 의 `diagnose` 의 메타 원칙 적용:
- 30초 flaky = 0 loop. 2초 deterministic = superpower.
- non-deterministic bug 는 rate 끌어올리기 (1% → 50%).

**Action**: axhub `tests/runtime-fallback.test.ts`, `tests/consent.test.ts`, `tests/lint-toss-tone.test.ts` 가 이미 deterministic 인지 검증. flaky 발견 시 Phase 1 룩의 "rate 끌어올리기" 적용.

---

## 패턴 2: axhub 가 이미 더 강한 부분

### 2.1 Skill scaffold 강제 (mattpocock 보다 우월)

axhub `bun run skill:new <slug>` 자동 추가:
- D1 TTY guard
- TodoWrite Step 0
- `!command` preflight injection
- AskUserQuestion header
- registry stub

mattpocock `write-a-skill` 은 가이드만 — 사용자 직접 작성 → 패턴 누락 위험.

**Verdict**: axhub 의 자동화가 더 우월. mattpocock 패턴 차용 X — 이미 더 정교함.

### 2.2 Lint baseline 자동 검증

axhub `lint:tone --strict` (해요체 강제) + `lint:keywords --check` (nl-lexicon baseline) + `skill:doctor` (D1 sentinel / TodoWrite / preflight 검사):

```
Tone: 합니다 / 입니다 / 시겠어요 / 드립니다 / 당신 / 아이고 → 금지
Use: 해요 / 예요 / 이에요 / 할래요
```

mattpocock 은 lint 없음 — 매트 본인 voice 일관성 manual.

**Verdict**: axhub 가 한국어 multi-author 환경에 적합한 구조. mattpocock 단일-author 라 lint 불필요. 서로 다른 scale.

### 2.3 Release automation

axhub `bun run release`:
- 3 file 동시 bump (package.json + plugin.json + marketplace.json)
- postbump (codegen:version + release:check 5 binary build)
- CHANGELOG entry
- commit + tag
- cosign 서명

mattpocock 은 version 없음.

**Verdict**: axhub 가 distribution 모델 (npm + binary + plugin marketplace) 가짐. mattpocock 은 git clone 만. 다른 사용 모델.

### 2.4 GitNexus 통합 (impact analysis)

axhub `CLAUDE.md` 에 명시:
> MUST run `gitnexus_impact({target: "symbolName", direction: "upstream"})` before editing any symbol.

mattpocock 은 grep + Explore agent 만.

**Verdict**: axhub 가 graph-aware impact analysis 가짐 — 더 정교한 navigation. mattpocock 의 `zoom-out` 보다 강력.

### 2.5 Test scale

axhub: 498 test pass / 0 fail (Phase 18 baseline).
mattpocock: test 없음 (markdown only).

**Verdict**: axhub 가 production code base, mattpocock 은 doc only. 비교 부적절.

---

## 패턴 3: 충돌 위험 (axhub 에 잘못 적용 시 문제)

### 3.1 mattpocock 의 100-line SKILL.md rule

axhub SKILL 들은 종종 100 lines 초과 — 한국어 prose + AskUserQuestion JSON block + multi-step preflight + 본인 deploy 도메인 설명 때문.

**위험**: 100 lines 강제 시 axhub 의 한국어 explainer 와 multi-step process 가 잘림 → user 친화성 ↓.

**해결**: mattpocock rule 무시. axhub 는 doctor 가 패턴 검증 — 길이 가 quality proxy 아님.

### 3.2 mattpocock 의 "lazy file creation" + "silent on absence"

axhub `multi-step: true` + `needs-preflight: true` 가 정반대 — pre-condition 을 항상 검증 + state 누락 시 명시적 에러.

**위험**: silent 진행 → axhub deploy 에서 stale config 위험. mattpocock minimum 으로 후퇴.

**해결**: mattpocock lazy 패턴은 도메인 doc (CONTEXT.md, ADR) 에만 적용. axhub state / preflight / config 에는 절대 X.

### 3.3 mattpocock 의 `qa` (deprecated) "over-interview 금지"

axhub 의 deploy / recover 같은 destructive op 는 over-interview 가 안전 메커니즘:
- "cosign 서명 검증 — 회사 보안 정책 호환" (axhub:update)
- "destructive 작업" 명시
- AskUserQuestion 다수

**위험**: mattpocock "2-3 short clarifying question only" 차용 시 axhub 의 안전 가드 약화.

**해결**: clarify skill (axhub:clarify) 처럼 명시적 "ambiguity 해소" 만 mattpocock 패턴 차용. destructive op 는 over-interview 유지.

### 3.4 mattpocock 의 "Mock at boundaries only" 의 주관적 해석

axhub Rust crate ↔ TS code boundary — internal collaborator 처럼 보이지만 실제로 process boundary. mattpocock 정의에 따라 mock 가능.

**위험**: axhub 작업자가 mattpocock "자기 modules 절대 mock 금지" 를 over-apply → cross-language boundary 도 안 mock → integration test 불가능.

**해결**: mattpocock 의 boundary 정의 (process / network / true external / time / random) 를 axhub 에 맞게 확장 — Rust ↔ TS = process boundary, mock 가능.

---

## 직접 차용 추천 (priority 순)

### P0 — 즉시 차용

1. **mattpocock 의 ADR `3 조건` 가이드** → axhub `.omc/adr/` 작성 시 적용. minimal 1-paragraph.
2. **AGENT-BRIEF 의 "behavioral, not procedural" 원칙** → axhub long-lived issue / PRD 에 적용.
3. **Triage 의 AI-generated disclaimer** 강제 → axhub 가 GitHub 댓글 / issue 작성 자동화 도입 시 흡수.

### P1 — 단기 평가

4. **`disable-model-invocation: true`** → axhub 의 destructive multi-step skill 에 추가 layer 검토.
5. **CONTEXT.md 의 "Avoid" 동의어 패턴** → axhub `CONTEXT.md` 가 있다면 (현재 GitNexus 사용 중) 차용. nl-lexicon baseline 의 token 금지 list 와 통합 가능.
6. **Iterate-the-loop 메타 원칙** → axhub test 의 flaky 가 발견되면 적용.

### P2 — 장기 / 선택

7. **Vertical slice issue template** → phase-by-phase 작업이 vertical slice issue 단위로 분해될 때.
8. **`.out-of-scope/<concept>.md` 패턴** → axhub 가 거절된 feature 요청 archive 필요할 때.
9. **mattpocock 4 failure mode framing** → axhub README 의 "왜 이 plugin 이 존재" 섹션 작성 시 차용.

### 차용 안 함 (axhub 가 이미 더 강함)

- skill scaffolding (axhub `skill:new` 가 이미 우월)
- lint baseline (axhub `lint:tone` / `lint:keywords` 가 이미 강함)
- release automation (axhub `commit-and-tag-version` + cosign 우월)
- code intelligence (axhub GitNexus 우월)

### 차용 금지 (axhub 에 잘못 적용 위험)

- 100-line SKILL.md rule (axhub 의 한국어 explainer 잘림)
- lazy file creation (axhub state 안전 가드 약화)
- over-interview 금지 원칙 (axhub destructive op 안전 약화)

---

## axhub 작업자 체크리스트

mattpocock/skills 분석을 axhub 에 적용 시:

- [ ] `.omc/adr/` 새 ADR 작성 — `ADR-FORMAT.md` minimal 1-paragraph 채택?
- [ ] long-lived issue / PRD 작성 — file path / line number 회피?
- [ ] `tests/fixtures/ask-defaults/registry.json` 갱신 시 — interface 묘사 (channel + safe_default + rationale), file path 묘사 X?
- [ ] destructive skill (axhub:deploy / recover / update) 에 `disable-model-invocation: true` 추가?
- [ ] axhub `CONTEXT.md` 또는 nl-lexicon baseline — "Avoid" 동의어 패턴 통합?
- [ ] flaky test 발견 시 — `diagnose` Phase 1 의 rate 끌어올리기 적용?
- [ ] mattpocock 100-line rule 무시? ✓ (axhub 한국어 explainer 보존)
- [ ] mattpocock lazy file creation 무시? ✓ (axhub preflight 우선)

---

## 한 줄 결론

> **axhub 는 production-grade plugin (build / test / release / lint / scaffold 모두 자동화), mattpocock/skills 는 markdown-only minimalist toolkit.** 두 repo 의 _기술 mechanism_ 은 서로 보완 X (axhub 가 압도적). 그러나 mattpocock 의 _철학_ — durable not procedural / behavioral not procedural / vertical slice / minimal ADR / disable invocation 명시 / "Iterate the loop itself" — 은 axhub 의 한국어 multi-step skill 작성에 직접 가치 있어요. 흡수 가능 패턴 9개 (P0×3 / P1×3 / P2×3) 식별, 충돌 위험 3개 회피.
