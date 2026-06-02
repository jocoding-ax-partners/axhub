# Quickstart — 정합 검증 절차

`004-skill-exit-code-alignment` 가 동작하는지 확인하는 절차예요. 구현(`/speckit-tasks` → 실행) 후 이 단계로 SC 를 검증해요.

## 0. 사전 — CLI 실패 신호 관찰 (전략 기반 재확인)

```bash
BIN=/path/to/ax-hub-cli/target/debug/axhub   # 또는 PATH 의 axhub
cd /tmp
"$BIN" deploy status --json            # → error.code:"usage" (exit 64)
"$BIN" deploy list --app no-such-xyz --json   # → error.code:"not_found" (exit 5)
# deauth 후 (T0): 인증 필요한 명령 → error.code:"unauthenticated" (exit 4)
```
기대: 모든 실패가 `{"status":"error","error":{"code":<flat-slug>,...}}` + 숫자 4/5/6/64. 옛 65/67/68 안 나옴.

## 1. SC-002 — 토큰 만료 → 재로그인 (P1, 핵심)

- 만료 토큰 상태에서 `status` 스킬 실행 (또는 `unauthenticated`/exit 4 주입).
- **기대**: 재로그인 안내 (일반 통신-오류 템플릿 아님). 비대화형이면 등록 기본값(abort).
- **현재(회귀 전)**: exit 4 가 catalog "65" 미스 → 일반 fallback. 0% 재로그인.

## 2. SC-001/003/004 — 카탈로그 ↔ CLI 계약 parity

```bash
bun test tests/exit-contract-parity.test.ts   # [신규] catalog 키 ⊆ pinned CLI 계약, 미지 키 0
bun test tests/codegen.test.ts                 # catalog.json ↔ generated.md drift 0
cargo test -p axhub-helpers exit_to            # exit→slug 매핑 단위 (4→unauthenticated 등)
```
기대: catalog 키에 `65/67/68/70/2` 0개; 모든 CLI 계약 코드가 정확히 1개 항목.

## 3. SC-005 — 8 skill 동일 라우팅

```bash
rg -n 'exit 6[5-8]|exit 70|"65"|"67"|"68"' skills/   # 기대: 0 (옛 numeric 잔존 없음)
```
각 skill 의 같은 실패 조건이 같은 slug→템플릿으로 가는지 대조.

## 4. SC-006 — 회귀 0 (전체 게이트)

```bash
cargo test                       # axhub-helpers 단위/통합
bun test                         # ≥ 기존 baseline pass / 0 fail
bunx tsc --noEmit                # clean
bun run lint:tone --strict       # 0 err (해요체)
bun run lint:keywords --check    # no diff (description byte-lock)
# 새/변경 SKILL 있으면:
bun run skill:doctor --strict    # exit 0
```

## 5. SC-007/008 — 추적성 + 가드

- catalog 각 항목이 `cli-error-envelope.md` / `cli-exit-contract.json` 을 인용 (FR-011).
- pinned snapshot 을 일부러 어긋나게 → `exit-contract-parity.test.ts` fail 확인 (가드 작동).

## Done

- [ ] 0~5 전부 기대대로 → spec SC-001~008 충족.
- [ ] `git -C ax-hub-cli` 무수정 (CLI 미변경, Out of Scope).
