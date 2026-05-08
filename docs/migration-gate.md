# Migration Gate — Approach E (NL Routing Redesign)

원본 plan: `.plan/ceo-review-nl-routing/2026-05-07-nl-routing-redesign.md`
Phase 0 (Task 0): `.plan/ceo-review-nl-routing/phases/phase-0-task-0-test-contract.md`

---

## GitNexus Impact

`detect_prompt_route` 와 `cmd_prompt_route` 둘 다 GitNexus impact **CRITICAL** 이에요. `detect_prompt_route` 는 `cmd_prompt_route` 가 직접 호출하고, `cmd_prompt_route` 는 helper binary 의 `run` entry 가 매 hook 호출마다 실행해요. 따라서 Approach E 의 router 단순화 PR 은 아래 6 개 gate 를 모두 통과해야 merge 할 수 있어요.

---

## 6 PR Gate Items

| # | Gate | Pass criterion | Command |
|---|------|---------------|---------|
| 1 | Targeted Rust e2e | exit 0 | `cargo test -p axhub-helpers cli_prompt_route --test cli_e2e` |
| 2 | Workspace test | exit 0 | `cargo test --workspace` |
| 3 | TypeScript test | exit 0 (1 known fail = README/package version mismatch 별개 issue, blocker X) | `bun test` |
| 4 | Hook latency benchmark | p95 < 50ms (이미 enforce, Phase 0 후에도 회귀 X) | `bun run scripts/benchmark-hooks.ts` |
| 5 | Routing-specific 100-tier scorer | accuracy ≥ 95% AND drift ≤ 5% | `bun run routing:drift` 또는 `bun run tests/routing-score.ts --baseline tests/baseline-results.docs-only.100.json --against tests/baseline-results.claude-native.100.json` |
| 6 | High-risk live canary | manual run 4 prompt, consent gate / classify-exit 정상 | manual: "배포해줘" / "이 앱 삭제해" / "DB_URL=foo 설정" / "로그인" |

PR description 의 gate evidence 가 6 개 모두 ✅ 또는 ⚠️ (canary 만 manual) 이어야 reviewer 가 approve 해요.

---

## PR Description Template

PR description 에 이 markdown checklist 를 그대로 넣고 evidence 를 채워요.

```markdown
## Migration Gate Evidence (Approach E)

| # | Gate | Status | Evidence |
|---|------|--------|----------|
| 1 | Targeted Rust e2e | ✅ / ❌ | `cargo test cli_prompt_route` → <pass count> / <duration>s |
| 2 | Workspace test | ✅ / ❌ | `cargo test --workspace` → <pass count> / <duration>s |
| 3 | TypeScript test | ✅ / ❌ | `bun test` → <pass count> / <fail count> |
| 4 | Hook latency | ✅ / ❌ | p50=<X>ms / p95=<Y>ms / max=<Z>ms / threshold=50ms |
| 5 | Routing-score 100 | ✅ / ❌ | accuracy=<X>% / drift=<Y>% / threshold=95% / drift-cap=5% |
| 6 | Canary 4 prompt | ⚠️ manual | <screenshots / terminal capture> |

### Notes

- Gate 4 latency baseline 비교: <prev p95> → <new p95>
- Gate 5 baseline diff 분포: <link to score JSON>
- Gate 6 canary 결과: <consent gate 동작 / 한국어 error 표시 / classify-exit 정상>
```

---

## Canary 4 Prompt — High-risk Live Verification

Approach E 후 *반드시* 사람이 손으로 확인해요. 4 발화 모두 정상 흐름 확인 후 PR merge.

### 1. `"배포해줘"` (deploy)

- 기대 동작: Claude 가 description 매칭 → `/axhub:deploy` 호출 → PreToolUse hook 의 HMAC consent gate 가 deploy command 차단 (사용자 승인 카드 표시) → 사용자 승인 후 axhub deploy 실행 → PostToolUse classify-exit 한국어 결과 안내
- 실패 신호: consent gate 우회 (axhub deploy 가 prompt 직후 자동 실행), classify-exit 가 영어 stdout, deploy intent 잘못 라우팅

### 2. `"이 앱 삭제해"` (apps delete)

- 기대 동작: Claude 가 description 매칭 → `/axhub:apps` 또는 destructive flow → consent gate 가 apps delete 차단 → 승인 후 실행
- 실패 신호: consent gate 우회, 또는 apps list (read-only) 로 잘못 라우팅

### 3. `"DB_URL=foo 설정"` (env set)

- 기대 동작: Claude 가 description 매칭 → `/axhub:env` → consent gate 가 env set 차단 → 승인 후 실행
- 실패 신호: secret 평문 로깅, consent gate 우회

### 4. `"로그인"` (auth login)

- 기대 동작: Claude 가 description 매칭 → `/axhub:auth` → axhub auth login 실행 → 한국어 안내
- 실패 신호: 영어 메시지, login flow 미완료, audit log 에 token 평문 기록

---

## Linked Phase 0 Sub-tasks

이 Migration Gate 가 Phase 0 의 6 sub-tasks 와 연결돼요:

- **0.1** routing-score.ts → Gate 5 의 command 가 호출
- **0.2** 331-row advisory → Gate 5 가 100-row 만 사용 (advisory 모드 X)
- **0.3** cli_e2e.rs contract (Phase 2 dependency) → Gate 1 이 새 계약 test 결과 검증
- **0.4** benchmark-hooks.ts (TODO marker, Phase 2 갱신) → Gate 4
- **0.5** audit privacy disclosure → Phase 3/7 contract (별도 docs/audit-privacy-contract.md)
- **0.6** SessionStart fallback systemMessage → Phase 7 contract (별도 docs/sessionstart-contract.md)
- **0.7** 이 문서 자체 — Migration Gate 정의

---

## How to Re-run

```bash
# Gate 1
cargo test -p axhub-helpers cli_prompt_route --test cli_e2e

# Gate 2
cargo test --workspace

# Gate 3
bun test

# Gate 4
bun run bench:hooks

# Gate 5
bun run test:routing

# Gate 6 — manual checklist (run each in localdev session)
echo "배포해줘" | claude code  # consent gate 확인
echo "이 앱 삭제해" | claude code  # consent gate 확인
echo "DB_URL=foo 설정" | claude code  # secret redact 확인
echo "로그인" | claude code  # auth flow 확인
```

---

## Reversibility

이 Migration Gate 가 fail 하면 Approach E 폐기 후 Approach F (selective hook — 5 high-stakes skill 만 keyword chain 유지) 로 fallback 가능해요. 결정은 측정 데이터 + 사용자 승인 후.

---

## CI gate enforcement (Phase 8)

`.github/workflows/routing-drift.yml` 가 PR 마다 corpus.100 fresh measure 자동 실행해요. drift > 5% 또는 accuracy < 95% 시 PR block.

### Skip override

의도된 drift (예: 새 SKILL 추가 후 baseline 재측정 전 PR) 일 때:

PR title 에 `[skip-routing-gate]` 포함 → workflow skip + 머지 가능.

skip override 사용 시 후속 PR 에서 `bun run measure:baseline` 실행 후 docs-only.100.json 갱신 필수예요.

### 관련

- workflow: `.github/workflows/routing-drift.yml`
- measurement protocol: `docs/baseline-measurement.md`
- script: `scripts/measure-docs-only-baseline.ts`
