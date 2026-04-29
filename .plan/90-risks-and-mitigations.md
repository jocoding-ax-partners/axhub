# 위험 매트릭스 + Cross-Phase Themes

/autoplan dual voices (Codex + Claude subagent) 가 3 phase 모두에서 독립적으로 발견한 high-confidence signals 통합.

---

## 1. Cross-Phase Themes (3 phase 동시 발견)

| Theme | CEO | Eng | DX | 강도 | 처리 |
|-------|-----|-----|----|----|------|
| **Premise (통증 driver) 미검증** | ✓✓ | — | — | 매우 높음 | User REJECTED challenge — proceed |
| **Solo maintainer + bus factor 1** | ✓✓ | ✓ | ✓ | 매우 높음 | Phase 3 security-reviewer 강제 |
| **go-keyring/keyring 인터롭 unverified** | — | ✓✓ | ✓✓ | 매우 높음 | DX-4 Phase 0 spike 강제 |
| **Korean UX (해요체) 보존 risk** | — | ✓ | ✓✓ | 높음 | DX-2 messages.rs + Rust tone lint |
| **Test rewrite 비용 (bun:test mock)** | — | ✓✓ | — | 높음 | 추정 8~12주 → 10~14주 update |
| **Windows MSVC + EDR 보존** | — | ✓✓ | ✓ | 높음 | gnu → msvc 강제, V3 cohort QA |
| **Migration UX (axhub update + doctor)** | ✓ | — | ✓✓ | 높음 | DX-5 migrate-rust.md Phase 1 작성 |
| **AXHUB_HELPERS_RUNTIME first-class** | — | — | ✓✓ | 중 | DX-3 README + axhub:doctor |

---

## 2. 위험 매트릭스 (Phase 별)

### Phase 0 (Prerequisite)

| 위험 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| keyring crate 가 go-keyring 호환 안 함 | 중 | 매우 높음 (Phase 3 차단) | DX-4 spike 결과 따라 subprocess 유지 결정 |
| JWE 사용 발견 (예상 0건이지만) | 매우 낮음 | 높음 (+1주) | grep 결과로 즉시 확정 |
| `.tool-versions` 가 mise/asdf 미설치 환경에서 무시됨 | 중 | 낮음 (TTHW 영향) | README 안내 + Docker fallback |

### Phase 1 (Foundation)

| 위험 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| Cargo workspace cache cross-compile 시 conflict | 중 | 중 (CI 시간) | sccache + cargo-chef |
| corpus.jsonl parsing 차이 (TS JSON vs serde) | 낮음 | 높음 (classify 회귀) | parity test 100% 강제 |
| build.rs 가 매 PR 마다 재실행 | 중 | 중 (CI 느림) | rerun-if-changed 정확 |
| TS test parity 매핑 누락 | 높음 | 중 | PR 템플릿 강제 |

### Phase 2 (Stateless)

| 위험 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| **TLS pin 검증 우회 (rustls custom verifier)** | 낮음 | **치명** (MITM) | dangerous API 사용 시 외부 페네스트 |
| AXHUB_ALLOW_PROXY 동작 차이 | 낮음 | 높음 (사용자 차단) | env logic byte-equal test |
| reqwest rustls-tls 미활성 → OpenSSL | 중 | 중 (binary +20MB) | default-features = false 강제 |
| preflight semver prerelease drop 누락 | 중 | 중 (회귀) | bug-for-bug test 강제 |
| timeout 5000ms 차이 | 낮음 | 낮음 | tokio::time::timeout 정확 |

### Phase 3 (Security) — 최고 위험 구간

| 위험 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| **JWT leeway silent widen** | 중 | **치명** (replay window) | leeway=0 lock test 강제 |
| **HMAC key file 권한 회귀** | 낮음 | **치명** (key 도난) | symlink defense + mode 0600 lstat |
| parser fuzz crash (regex DoS) | 중 | 높음 | cargo-fuzz 24h, state machine 재구현 |
| **한국 EDR (V3/AhnLab) 차단 회귀** | 높음 | 매우 높음 (사용자 마찰) | PowerShell 유지 + cohort QA |
| **go-keyring envelope 호환 깨짐** | 중 | 매우 높음 (axhub-cli 단절) | 양방향 read/write test |
| Windows ACL 보안 (token 파일) | 낮음 | 높음 | windows-rs ACL set + admin-deny test |

### Phase 4 (Integration + Ship)

| 위험 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| **사용자 axhub update 후 token 잃음** | 낮음 | **치명** | go-keyring 호환 + token-file contract test |
| Windows MSVC native runner 비용 | 높음 | 중 | windows-latest matrix (+1주 CI) |
| Authenticode 서명 워크플로 깨짐 | 낮음 | 높음 | signtool MSVC PE32+ 검증 |
| 한국 EDR cohort 회귀 | 중 | 매우 높음 | AXHUB_HELPERS_RUNTIME=ts fallback |
| TS↔Rust 미묘한 동작 차이 | 높음 | 중 | parity 매핑 PR 첨부 + monitor 1주 |

---

## 3. Risk Acknowledged (User accepted)

User 가 premise gate 에서 양 모델의 STOP 권장 거부하고 진행 결정. 다음 위험은 사용자가 internal context 로 판단:

| Risk | 출처 | 가정 |
|------|------|------|
| Premise (통증 driver) 미검증 | Codex+Subagent | 사용자가 internal product priority 로 판단 |
| 8~12주 opportunity cost | Codex+Subagent | 사용자가 product priority 결정권 |
| Solo + bus factor 1 | Subagent | 사용자가 학습/유지보수 의지 보유 |
| consent 보안 회귀 surface | Codex+Subagent | Phase 3 security-reviewer 로 완화 |
| 6-month regret memo plausible | Subagent | rollback 전략 (Phase 4 §6) 으로 완화 |

---

## 4. Mandatory Mitigations (User decision 후에도 강제)

### Phase 1 시작 전

- [ ] DX-1: `.tool-versions`
- [ ] DX-2: messages.rs + Rust tone lint scanner
- [ ] DX-3: AXHUB_HELPERS_RUNTIME 문서화
- [ ] DX-4: keyring ⇄ go-keyring envelope spike (1d)
- [ ] JWE grep (예상 0건)
- [ ] JWT leeway 0 lock test (TS 측 먼저)
- [ ] Bun.* shim crate (spawn_sync)
- [ ] CI rust-ci workflow

### Phase 3 (보안 surface)

- [ ] security-reviewer agent 통과
- [ ] cargo-audit clean
- [ ] cargo-fuzz parser 24h 무결함
- [ ] V3/AhnLab live cohort QA
- [ ] symlink defense test
- [ ] world-readable token 거부 test
- [ ] go-keyring 양방향 호환 test

### Phase 4 (Ship)

- [ ] 1주 monitor 기간 의무
- [ ] AXHUB_HELPERS_RUNTIME=ts fallback 동작
- [ ] Migration guide (DX-5) 완성
- [ ] CHANGELOG honest tradeoff section
- [ ] Bun 참조 inventory (DX-6) 100% 처리

---

## 5. Rollback Triggers

다음 중 1건 발생 시 Phase 4 ship 후 즉시 rollback PR:

- consent JWT verify 실패율 > 0.1%
- keychain read 실패율 > 5% (특정 OS)
- TLS pin 검증 실패 (사용자 한 명이라도)
- 한글 메시지 영어 leak 발견
- V3/AhnLab cohort 차단 보고
- axhub-cli 와 token 공유 깨짐

**Rollback 절차:**

```bash
git revert <merge-commit>
bun run release -- --release-as patch
git push origin main --tags
# 사용자 안내: "AXHUB_HELPERS_RUNTIME=ts axhub update --force-version 0.1.23"
```

---

## 6. Decision Audit Trail

| # | Phase | Decision | Classification | Principle | Rationale | Source |
|---|-------|----------|----------------|-----------|-----------|--------|
| 1 | CEO | premise 미검증 진행 | USER CHALLENGE | User sovereignty | 사용자 internal context 보유 | premise gate |
| 2 | CEO | full Rust 포팅 (partial 거부) | USER CHALLENGE | User sovereignty | "전부 다, 완벽" 명시 | premise gate |
| 3 | Eng | Plan §1-3 정정 (consent ≠ mTLS) | Mechanical | P5 explicit | source 검증 결과 | dual voices |
| 4 | Eng | Windows GNU → MSVC 강제 | Mechanical | P1 completeness | Authenticode + EDR | Eng dual voices |
| 5 | Eng | parser state machine 재구현 | Taste | P5 explicit | linear time + fuzz 친화 | Eng review |
| 6 | DX | AXHUB_HELPERS_RUNTIME first-class | Taste | P1 completeness | rollback 보장 | DX dual voices |
| 7 | DX | messages.rs + Rust tone lint | Mechanical | P1 completeness | 해요체 보존 | DX dual voices |
| 8 | DX | keyring 채택 vs subprocess (Phase 0 spike 결과 따라) | Taste | P3 pragmatic | 실측 후 결정 | DX-4 |
| 9 | Eng | 추정 8~12주 → 10~14주 | Mechanical | P1 completeness | test rewrite + MSVC + DX prereq | dual voices |
| 10 | All | 1주 validation sprint REJECTED | USER CHALLENGE | User sovereignty | 사용자 명시적 거부 | premise gate |
