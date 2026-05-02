# Phase D — Release

> Phase A0/B/B-test/C 모두 PASS 후. feature flag flip + README + GIF + commit-and-tag-version 자동 chain.

---

## D-1. README + lifecycle GIF + 5분 만에 시작 섹션

**파일**: `README.md`

**현재**: 1.5KB, "wraps ax-hub-cli (v0.1.0+)" 만 (codex DX F5 = drift, README v0.1.22 표시 vs package v0.1.26).

**작업**:

### D-1a. version drift 자동 동기화

- `scripts/codegen-readme-version.ts` 신규 — package.json 의 version 을 README 의 status badge 로 sync
- `package.json:scripts.codegen:version` 의 postbump hook 에 추가
- commit-and-tag-version 시 자동 동기화 (drift 영구 차단)

### D-1b. "5분 만에 시작하기" 섹션 추가

- README 에 다음 섹션 신규:
  ```markdown
  ## 5분 만에 시작하기 (vibe coder)
  
  1. Claude Code download (claude.ai/code)
  2. /plugin add @jocoding-ax-partners/axhub
  3. "결제 앱 만들어줘" 라고 채팅
  4. 기술 스택 골라요 (Next.js / FastAPI / 등)
  5. 자동 설치 + 템플릿 + npm install 끝나면 "배포해" 한 마디
  
  3분 30초 (warm node) / 7분 30초 (cold zero-install) 안에 production live.
  ```

### D-1c. lifecycle demo GIF

- `docs/demo/lifecycle.gif` 신규 — 빈 dir → init → apps create → github connect → env set → deploy → open
- terminal recording: `asciinema rec` → `agg` 변환 → GIF
- 또는 수동 screen recording → ffmpeg GIF
- README 에 embed: `![lifecycle demo](docs/demo/lifecycle.gif)`

### D-1d. GIF maintenance ownership (codex DX F5 응답)

- `docs/demo/README.md` 신설:
  - GIF 재생성 cmd 명시
  - 재생성 trigger: SKILL workflow 변경, lifecycle ux 변경, major version bump
  - 책임자 = release narrative 작성자 (CHANGELOG narrative 와 동시)
- pre-release checklist 에 "GIF re-render 검토" 추가

**effort**: ~30분 (GIF 자체 ~15분 + README 작성 ~15분)

---

## D-2. Feature flag flip

**파일**: `.claude-plugin/plugin.json`

**현재**:
```json
{
  "name": "axhub",
  "version": "0.1.26",
  ...
}
```

**변경 (Phase A0 추가됐던)**:
```json
{
  "name": "axhub",
  "version": "0.2.0",
  ...
  "features": {
    "beta_skills": true     ← Phase A0 default false → Phase D 에서 true 로 flip
  }
}
```

**검증**:
- `AXHUB_PLUGIN_BETA=1` env 와 `plugin.json:features.beta_skills:true` 둘 다 체크하는 logic 이 prompt-route.ts 에 있는지
- flip 후 7 신규 SKILL 의 routing 활성

**effort**: ~5분

---

## D-3. CLI v0.10 호환 안내 + ENFORCE 사전

**파일**: `.claude-plugin/plugin.json` description, `marketplace.json` description, `package.json` description

**변경**:
- "wraps ax-hub-cli (v0.1.0+)" → "wraps ax-hub-cli (v0.10.0+)" (CLI v0.10 surface 활용)
- v0.9.1+ cosign WARN, v0.9.2+ ENFORCE 사전 안내 (update SKILL polish 에 흡수, 여기서는 manifest 만)

**effort**: ~10분 (codegen 자동, 직접 vim X)

---

## D-4. CHANGELOG narrative

**파일**: `CHANGELOG.md`

**현재**: commit-and-tag-version 가 auto-generate 한 bullets.

**작업**:
- auto-bullets 위에 "Phase 23 — CLI v0.10 surface coverage + zero-install bootstrap" 한국어 narrative 추가 (해요체)
- 핵심 메시지:
  - 11 → 18 SKILL: vibe coder 가 NL 로 lifecycle 끝까지
  - cold customer = `init` 한 번이면 모든 dep 자동 install (helper bootstrap)
  - examples repo template scaffold (5 stack)
  - admin onboarding wizard (DX Codex F7)
  - 27 codex finding 통합 (CEO 12 + ENG 8 + DX 7)
- Test baseline 섹션:
  - 570 → ≥640 PASS / 0 FAIL
  - 신규 16 E2E + 64 unit + 3 SECURITY MUST + 3 REGRESSION CRITICAL
- Honest tradeoff 섹션:
  - TTHW: cold ~7분 30초 / warm ~3분 (Competitive tier 통과, Champion <2분 아님)
  - admin SKILL = skeleton, 별도 design pass 필요
  - lifecycle E2E harness 의 multi-step opt-in = 신규 infrastructure
  - feature flag default OFF → Phase D 에서 flip = atomic deploy

**effort**: ~20분

---

## D-5. Release 자동 chain

```bash
# 1. clean working tree 확인
git status

# 2. 모든 PASS 검증
bun run skill:doctor --strict
bun run lint:tone --strict
bun run lint:keywords --check
bun test
bunx tsc --noEmit
cargo test --workspace
scripts/benchmark-hooks.ts

# 3. release 한 줄
bun run release
# 자동 chain:
#  ✓ 3 파일 bump (package/plugin/marketplace) v0.1.26 → v0.2.0
#  ✓ postbump: codegen:version (install.sh/ps1/index.ts/telemetry.ts/README sync)
#               + release:check (5 binary build + version assert)
#  ✓ CHANGELOG.md auto entry
#  ✓ git commit + git tag v0.2.0

# 4. CHANGELOG narrative amend
git commit --amend --no-edit -a   # narrative 흡수

# 5. push
git push origin main --tags
# release.yml 자동 fire — 5 cross-arch binary cosign 서명 + GH release upload

# 6. 검증
gh release view v0.2.0 --json url -q .url
gh release view v0.2.0 --json assets -q '.assets[].name'
# 5 binary asset 확인: helper-darwin-arm64/amd64, helper-linux-arm64/amd64, helper-windows-amd64
```

**effort**: ~15분 (검증 PASS 가정)

---

## Phase D 총 effort

- D-1 ~ D-5 합계 = ~1시간 30분

## Post-release validation

- [ ] `gh release view v0.2.0` URL 정상
- [ ] 5 binary asset 모두 cosign verify PASS
- [ ] marketplace 에서 plugin v0.2.0 install 가능
- [ ] fresh Claude Code 세션에서 `/plugin install @jocoding-ax-partners/axhub` 후 NL "결제 앱 만들어줘" → init SKILL 정상 fire
- [ ] cold customer demo (fresh Mac VM) 에서 lifecycle ~7분 30초 안 통과

## Post-release follow-up

- v0.2.0 release 후 1 week 내 vibe coder 5명 pilot dogfood
- TTHW 실측 (`~/.local/state/axhub-plugin/usage.jsonl` opt-in telemetry)
- DX scorecard 재측정 → /devex-review boomerang (defer 결정 했지만 manual 1회 권장)
- v0.2.5 candidate: agent install (CLI atomic write fix 후), dev SKILL, tables SKILL
