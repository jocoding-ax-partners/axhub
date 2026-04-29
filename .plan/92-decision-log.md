# Decision Log — /autoplan dual voices + user decisions

**Captured:** 2026-04-29
**Branch:** main
**Commit:** 9ff097b

---

## 1. Decisions Summary

| # | Decision | Phase | Class | 출처 | 결과 |
|---|----------|-------|-------|------|------|
| 1 | premise (통증 driver) 미검증 진행 | CEO | USER CHALLENGE | premise gate | 사용자 REJECTED challenge → proceed |
| 2 | full Rust 포팅 (partial/hot-path 거부) | CEO | USER CHALLENGE | premise gate | 사용자 명시 "전부 다, 완벽" |
| 3 | Plan §1-3 정정 (consent ≠ mTLS) | Eng | Mechanical | dual voices | 정정 적용 |
| 4 | Windows GNU → MSVC 강제 | Eng | Mechanical | Eng review | §4.2 수정 적용 |
| 5 | parser state machine 재구현 (vs regex backtracking 1:1) | Phase 3 | Taste | Eng | 재구현 권장, fixture 변경 명시 |
| 6 | AXHUB_HELPERS_RUNTIME first-class env var | Phase 0 | Taste | DX | DX-3 mandatory |
| 7 | messages.rs + Rust tone lint scanner | Phase 0 | Mechanical | DX | DX-2 mandatory |
| 8 | keyring crate 채택 vs subprocess 유지 | Phase 0 | Taste (deferred) | DX-4 spike | spike 결과 따라 결정 |
| 9 | 추정 8~12주 → 10~14주 | All | Mechanical | dual voices | overview/Phase 4 update |
| 10 | 1주 validation sprint 거부 | All | USER CHALLENGE | premise gate | 사용자 명시 거부 |

---

## 2. Phase 1 (CEO Review) — Premise Gate

### 양 모델 합의 권장 (REJECTED by user)

**Codex CEO (8 strategic blind spots):**
1. ROI 미해결인데 "final gate" 가 아닌 "first gate" 여야 함
2. Engineering aesthetics 를 market leverage 보다 우선
3. "Rust port" 가 problem validation 전에 solution 으로 framed
4. 경쟁 위험 = opportunity cost (6개월 후 "다시 썼고 사용자 신경 안 씀")
5. Maintenance drag underpriced (solo + pairing fictional)
6. Security argument inverted (consent 재작성 = fresh auth regression)
7. Gradual = release risk 감소 but strategic risk 증가 (4개월 dual-runtime)
8. Alternatives 너무 빨리 dismissed

**Claude subagent CEO (7 critical+high):**
1. Plan validates itself, then asks if problem exists (Q6 should be Q1)
2. Headline benefits unverified napkin math (cold start 40ms invisible vs 800ms 네트워크)
3. Cheaper alternatives dismissed (Bun --minify + UPX + lazy-load)
4. Bus factor 1, Rust experience unstated
5. Opportunity cost unpriced (1Q feature velocity)
6. 6-month regret memo plausible
7. Strangler Fig framing hides dual-runtime tax

**합의 권장:** 1주 validation sprint —
- `hyperfine` benchmark of top-5 commands
- Bun-optimized binary delta (`--minify` + UPX + lazy-load)
- User signal audit (issues, support, install telemetry)

### User Decision

**Question:** Plan 1주 validation 거치지 않고 바로 Rust 포팅 시작할지?

**Answer:** "ts로 구성된거 전부 다 rust로 완벽 포팅할꺼야"

**Interpretation:**
- USER CHALLENGE REJECTED — 1주 validation 거부
- Scope 축소 거부 (partial/hot-path Rust addon 도 거부)
- "완벽" 강조 = 모든 모듈 Rust, exception 없음

**Risk acknowledged (사용자가 internal context 로 판단):**
- Premise (통증 driver) 미검증
- 8~12주 opportunity cost
- Solo + bus factor 1
- consent 보안 회귀 surface
- 6-month regret 가능성

---

## 3. Phase 3 (Eng Review) — Source-of-Truth Drift

### 양 모델 합의 발견

**핵심:** Plan §1-3 가 imagined helper 를 review. 실제 source 와 다름.

| Plan 주장 | 실제 source | 정정 |
|-----------|-------------|------|
| consent.ts uses mTLS+X509+JWE | jose HS256 only (HMAC) | 의존 정정 |
| TLS+X509 in consent.ts | list-deployments.ts | 위험 재배치 |
| jose clockTolerance 30s | jose default 0 | leeway 0 lock |
| Bun-specific 0건 | 실제 5건 | shim crate 필요 |

**Subagent (10 findings):** C1+C2 critical, H1-H4 high, M1-M4 medium
**Codex (8 findings):** §1.2 매핑 잘못, Phase 3 위험 misstated, JWT leeway, preflight semver under-scoped, test rewrite 어려움, MSVC, EDR, release pipeline shallow

### Plan 정정 적용 (사용자 승인 — Approve as-is)

- §1.1 inventory 수정 (consent HMAC only, list-deployments TLS+X509)
- §1.2 매핑 수정 (JWE 미사용 확인, jose JWS-only, getrandom for HMAC, nix(O_NOFOLLOW))
- §4.2 Windows GNU → MSVC 강제 (Authenticode + EDR)
- 현실 추정 8~12주 → 10~14주

---

## 4. Phase 3.5 (DX Review) — Developer-Hostile

### 양 모델 합의

**Subagent (8 findings, 평균 4.2/10):** TTHW 2/10, error messages 3/10, docs 3/10, upgrade 4/10, escape 4/10
**Codex (6 findings):** "weak DX for solo-maintained 4-month dual-runtime migration. Developer-hostile."

**합의 권장 (DX prerequisite 6개):**
1. DX-1: `.tool-versions` (mise/asdf pin)
2. DX-2: `messages.rs` 중앙화 + Rust tone lint scanner
3. DX-3: `AXHUB_HELPERS_RUNTIME=ts|rust|auto` 문서화
4. DX-4: keyring crate ⇄ go-keyring envelope 1d spike (Phase 1 prerequisite)
5. DX-5: `docs/migrate-rust.md` skeleton (Phase 1 작성, Phase 4 fill)
6. DX-6: Bun 참조 inventory (Phase 4 checklist)

### User Decision

**Question:** 6 DX prerequisite 강제할지?

**Answer:** "Approve as-is" (모든 correction 적용)

**결과:** DX-1, DX-2, DX-3, DX-4 mandatory. DX-5, DX-6 recommended.

---

## 5. Cross-Phase Themes (high-confidence signals)

| Theme | CEO | Eng | DX | 강도 | 처리 |
|-------|-----|-----|----|----|------|
| Premise 미검증 | ✓✓ | — | — | 매우 높음 | User REJECTED → proceed |
| Solo + bus factor 1 | ✓✓ | ✓ | ✓ | 매우 높음 | Phase 3 security-reviewer |
| go-keyring 인터롭 unverified | — | ✓✓ | ✓✓ | 매우 높음 | DX-4 spike |
| Korean UX (해요체) 보존 | — | ✓ | ✓✓ | 높음 | DX-2 |
| Test rewrite (bun:test mock) | — | ✓✓ | — | 높음 | 추정 update |
| Windows MSVC + EDR | — | ✓✓ | ✓ | 높음 | gnu→msvc, V3 cohort QA |
| Migration UX | ✓ | — | ✓✓ | 높음 | DX-5 |
| AXHUB_HELPERS_RUNTIME first-class | — | — | ✓✓ | 중 | DX-3 |

---

## 6. Final Approval Gate

### Question: 어떻게 주시겠어요?

**Options:**
- A) Approve as-is (모든 correction 적용 후 시작)
- B) Override 몇 개
- C) Interrogate
- D) Revise

**User Decision: A — Approve as-is**

**결과:**
- 8 plan correction 자동 적용
- 6 DX prerequisite 의 4 mandatory + 2 recommended
- Phase 1 시작 가능 (Phase 0 prerequisite 완료 후)

---

## 7. Review Logs (gstack)

```jsonl
{"skill":"plan-ceo-review","status":"issues_open","unresolved":7,"critical_gaps":2,"mode":"SELECTIVE_EXPANSION"}
{"skill":"plan-eng-review","status":"issues_open","unresolved":10,"critical_gaps":2,"issues_found":10}
{"skill":"plan-devex-review","status":"issues_open","initial_score":4,"overall_score":4,"product_type":"cli","tthw_current":"6-8min","tthw_target":"3min","unresolved":8}
{"skill":"autoplan-voices","phase":"ceo","consensus_confirmed":0,"consensus_disagree":6}
{"skill":"autoplan-voices","phase":"eng","consensus_confirmed":1,"consensus_disagree":5}
{"skill":"autoplan-voices","phase":"dx","consensus_confirmed":1,"consensus_disagree":5}
```

---

## 8. Restore Point

원본 plan (review 전 380줄):
`/Users/wongil/.gstack/projects/jocoding-ax-partners-axhub/main-autoplan-restore-20260429-112948.md`

전체 review (1,060줄):
`.omc/plans/rust-port-plan.md`

Phase 별 분리 (이 디렉터리):
`.plan/00-overview.md` ~ `92-decision-log.md`

---

## 9. Next Action

1. 본 decision log + Phase 0 prerequisite (`.plan/01-phase-0-prerequisite.md`) 검토
2. ADR 작성: `.omc/adr/0001-rust-port-decision.md` (Q1~Q6 답)
3. Phase 0 진행 (4 mandatory + DX-4 spike, 1주)
4. Phase 0 완료 시 Phase 1 GitHub issue 3건 (redact/catalog/telemetry) 생성
