# Success Criteria

> v0.2.0 release 시 PASS 해야 하는 모든 검증 항목.

---

## 1. Code quality gates

- [ ] `bun test` ≥640 PASS / 0 FAIL (baseline 570 + 신규 ~70)
- [ ] `bunx tsc --noEmit` clean (TypeScript strict)
- [ ] `cargo test --workspace` clean (Rust parity)
- [ ] `cargo llvm-cov --workspace --fail-under-lines 90` 통과 (cargo coverage)
- [ ] `bun run lint:tone --strict` 0 err (해요체 강제, 한국어 일관성)
- [ ] `bun run lint:keywords --check` PASS (nl-lexicon baseline lock)
- [ ] `bun run skill:doctor --strict` exit 0 (18 SKILL 모두 패턴 통과)
- [ ] `bun run lint:tone:rust --strict` 0 err (Rust 코멘트 톤)

## 2. Performance gates

- [ ] `scripts/benchmark-hooks.ts` PASS:
  - prompt-route p95 < 50ms (no-preflight + needs-preflight + clarify-fallback)
  - preauth-check p95 < 30ms (existing)
  - classify-exit p95 < 10ms (existing)
- [ ] helper bootstrap 평균 cold install ≤8분 (Mac/Linux/Windows)
- [ ] helper fetch-template 평균 ≤10초 (codeload tarball)
- [ ] helper install-deps 평균 ≤2분 (npm install for Next.js stack)

## 3. E2E test matrix

### T1 — read-only (10 case PASS)
- apps list / apis list / deploy list / env list / github repos / profile list / profile current / whatsnew / doctor audit / open --logs

### T2 — mutate (6 case PASS, mock-hub fixture)
- init (Next.js stack 선택 → bootstrap mock → fetch-template → install-deps)
- env set --from-stdin (value argv 노출 안 함 검증)
- github connect (account/repo 선택 → mock 200)
- profile use (config.yaml mutation)
- apis call (consent mint → mock 200, write scope)
- apps delete --dry-run first

### Lifecycle E2E (1 case PASS, multi-step opt-in)
- "init → apps create [별도 turn] → github connect → env set → deploy → open" 6-step chain

### Cross-platform smoke (15 case PASS)
- darwin-arm64 / darwin-amd64 / linux-arm64 / linux-amd64 / windows-amd64 (5 binary)
  × bootstrap / fetch-template / install-deps (3 subcommand)

### Negative test (5 case PASS — nl-lexicon collision)
- "환경" → clarify
- "환경변수 뭐 있어?" → env
- "환경 변수 확인" → env
- "환경 점검해" → doctor
- "회사 endpoint 바꿔" → profile

### Regression IRON RULE (3 case CRITICAL)
- preflight MAX_VERSION 0.11.0 후 v0.1.0 / v0.5.0 / v0.7.5 / v0.9.0 / v0.10.2 모두 in_range
- prompt-route 11→17 enum 후 기존 11 SKILL trigger 유지
- ConsentBinding 4→15 action 후 기존 deploy_create / update_apply / deploy_logs_kill / auth_login mint+verify cycle 변경 X

### SECURITY MUST (3 case CRITICAL)
- env set value 가 argv / ps aux / shell history / telemetry log 에 평문 X
- apis call write 가 consent gate 통과 안 하면 PreToolUse deny
- profile add non-allowlist domain 시 AskUserQuestion warn

**Total**: 10 + 6 + 1 + 15 + 5 + 3 + 3 = **43 critical test gate**

## 4. DX scorecard (DX-revised v3 → v4)

```
Dimension              | POST   | Trend  |
-----------------------|--------|--------|
Getting Started        |  9/10  |  ↑↑    | (zero-install setup)
API/CLI/SDK            |  8/10  |  →     |
Error Messages         |  9/10  |  →     |
Documentation          |  8/10  |  ↑     | (lifecycle GIF + 5분 시작)
Upgrade Path           |  8/10  |  →     |
Dev Environment        |  9/10  |  ↑↑    | (zero-install)
Community              |  6/10  |  →     | (defer)
DX Measurement         |  7/10  |  →     |

TTHW (cold zero-install) | ~7분 30초 (Competitive tier)
TTHW (warm)              | ~3분
Cold customer            | self-serve via init SKILL Step 3 helper bootstrap
Magical Moment           | 3 vehicle (init detect + preview + open)
Overall DX               | 8.5/10 (B+) — DX v4 target
```

## 5. Release manifest gates

- [ ] `plugin.json` version = `0.2.0`
- [ ] `marketplace.json` version = `0.2.0`
- [ ] `package.json` version = `0.2.0`
- [ ] `plugin.json:features.beta_skills` = `true` (Phase D flip)
- [ ] description 모두 "wraps ax-hub-cli (v0.10.0+)"
- [ ] `bin/install.sh` + `bin/install.ps1` version constant = 0.2.0 (codegen 자동)
- [ ] `src/axhub-helpers/index.ts:251` plugin_version default = "0.2.0" (codegen 자동)
- [ ] `src/axhub-helpers/telemetry.ts` plugin_version constant 일관 (codegen 자동)

## 6. CHANGELOG

- [ ] `CHANGELOG.md` 의 `## [0.2.0]` 섹션 존재
- [ ] auto-bullets (commit-and-tag-version) 위에 "Phase 23 — CLI v0.10 surface coverage + zero-install bootstrap" 한국어 narrative paragraph
- [ ] Test baseline 섹션 명시 (≥640 PASS)
- [ ] Honest tradeoff 섹션 명시 (TTHW cold/warm, admin skeleton, lifecycle harness 신규)

## 7. README + demo

- [ ] README 의 "5분 만에 시작하기" 섹션 존재
- [ ] README 의 status badge 가 v0.2.0 (codegen drift fix)
- [ ] `docs/demo/lifecycle.gif` 존재
- [ ] `docs/demo/README.md` (GIF re-render maintenance) 존재

## 8. Post-release validation (T+1 day)

- [ ] `gh release view v0.2.0` URL 정상
- [ ] 5 binary asset 모두 cosign verify PASS:
  ```bash
  cosign verify-blob \
    --certificate-identity-regexp "https://github.com/jocoding-ax-partners/ax-hub-cli/.github/workflows/release.yml@refs/tags/v.*" \
    --certificate-oidc-issuer https://token.actions.githubusercontent.com \
    --signature axhub-helpers-darwin-arm64.sig \
    axhub-helpers-darwin-arm64
  ```
- [ ] marketplace 에서 plugin v0.2.0 install 가능 (Claude Code 안에서 `/plugin install @jocoding-ax-partners/axhub`)
- [ ] examples repo 의 templates.json 가 helper list-templates 로 fetch 가능
- [ ] fresh Claude Code session + fresh Mac VM 에서 lifecycle ~7분 30초 안 통과 (manual demo)

## 9. Demo gate (vibe coder dogfood)

post-release 1주 내 5명 vibe coder pilot:
- 빈 dir 에서 "Next.js 결제 앱 만들어줘" → init Step 7 까지 완료 ≥4명 (TTHW ≤10분)
- 추가 NL "DB URL 환경변수 추가" → env set 성공 ≥4명
- "배포해" → deploy success ≥4명
- "결과 봐" → open browser 성공 ≥5명
- 1명도 raw axhub CLI 직접 입력 안 함 (NL 100%)

## Verdict criteria

위 9 영역 모두 PASS = v0.2.0 ship 가능.

CRITICAL gate 1 fail = 즉시 release 중단, hotfix → 재시도.

POST-release validation fail (cosign, marketplace) = 즉시 v0.2.1 patch.

vibe coder dogfood < 4/5 = v0.2.5 retrospective + 우선 fix.
