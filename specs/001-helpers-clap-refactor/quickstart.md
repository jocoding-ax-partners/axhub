# Quickstart: axhub-helpers clap 리팩토링 검증

**Date**: 2026-05-29 | **Plan**: [plan.md](./plan.md)

마이그레이션 작업 + 각 wave 의 parity 검증 절차. 모든 명령은 repo root 에서 실행해요.

## 0. 사전 확인

```bash
# 브랜치 + 의존성
git branch --show-current          # 001-helpers-clap-refactor
grep 'clap' Cargo.toml             # clap = { version = "4", features = ["derive"] } (이미 존재)
rustc --version                    # MSRV 1.83 이상 (edition 2021)
```

## 1. 빌드 + 전체 회귀 (parity oracle)

```bash
cargo build -p axhub-helpers
cargo test -p axhub-helpers                       # ★ parity oracle — 모든 wave 후 green 필수
cargo clippy -p axhub-helpers --all-targets -- -D warnings
```

**기대**: 기존 통합 테스트 모음(~28파일)이 통과. 유일 허용 변경 = top-level usage-error wording assert 1~2곳(`cli_e2e.rs` 의 `"unknown subcommand"`). 그 외 0 수정(SC-001).

## 2. fail-open 검증 (class=H, 가장 중요)

fail-open 검증 (SC-003). **scope 주의**: 무인자/unknown-ignore hook 은 bad-flag→0; flag-bearing hook(`state-update`)은 valid hook-flag 경로만 fail-open 이고 malformed→**64 보존**(parity guard, data-model §4):

```bash
BIN=$(cargo build -p axhub-helpers --message-format=json 2>/dev/null | \
      jq -r 'select(.executable!=null and (.target.name=="axhub-helpers")).executable' | head -1)

# 그룹 A — 무인자/unknown-ignore hook: bad-flag 줘도 exit 0
for cmd in session-start prompt-route preauth-check commit-gate tdd-inject \
           classify-exit test-classifier; do
  echo '{}' | "$BIN" "$cmd" --bogus-unknown-flag >/dev/null 2>&1
  echo "$cmd → exit $?"     # 0 이어야 함 (2 도 64 도 아님)
done

# 그룹 B — flag-bearing hook: 유효 hook-flag 실패는 fail-open(0), malformed 는 64 보존
echo '{}' | "$BIN" state-update --edit-event >/dev/null 2>&1; echo "state-update --edit-event → exit $?"  # 0 (hook 경로)
"$BIN" state-update --bogus >/dev/null 2>&1;                  echo "state-update --bogus → exit $?"        # 64 (malformed parity 보존, 0 아님)
# autowire-statusline 은 SessionStart wrapper 가 detached 라 exit 흡수 — 직접 호출은 valid flag 로 확인
"$BIN" autowire-statusline --scope auto --silent >/dev/null 2>&1; echo "autowire → exit $?"               # 0
```

## 3. version 계약 (D5 pre-intercept)

```bash
"$BIN" --version            # stdout: "axhub-helpers X (plugin vX, schema v0)", exit 0
"$BIN" version --quiet      # stdout 빈, stderr 빈, exit 0
"$BIN" --version --quiet    # 동일 (인자 순서 무관)
"$BIN" -v                   # 배너, exit 0
# 자동: cargo test -p axhub-helpers --test version_quiet_test
```

## 4. usage-error exit 64 (D4 remap)

```bash
"$BIN" bogus-subcommand;  echo "exit $?"   # stderr 에 clap 영어 에러, exit 64 (clap 기본 2 아님)
"$BIN";                   echo "exit $?"   # subcommand 없음 → stderr, exit 64
"$BIN" deploy-prep;       echo "exit $?"   # 필수 --intent 누락 → exit 64
```

## 5. 한국어 help/error 보존 (D6, FR-006a)

```bash
"$BIN" routing-stats --help     # PRIVACY 한국어 블록 포함 (clap long_about)
echo '' | "$BIN" consent-mint   # 빈 stdin → 한국어 안내 + exit 65 (handler-level)
"$BIN" post-install --bogus     # 한국어 flag 에러 경로 보존
```

## 6. wave 별 SC-004 진행 확인 (손수 파싱 제거)

```bash
# dispatch 진입점의 per-command 수동 파싱 루프 잔량 (Wave 4 종료 시 0 목표)
grep -c 'while i < args.len()' crates/axhub-helpers/src/main.rs crates/axhub-helpers/src/cli/*.rs 2>/dev/null
```

## 7. 외부 계약 무수정 (SC-002)

```bash
git diff --name-only main -- hooks/ | grep -E 'hooks\.json|\.sh$|\.ps1$' && echo "⚠️ hook 파일 변경됨 — SC-002 위반" || echo "✅ hook 파일 무변경"
```

## 8. binary size delta (D8, blocker 아님 — 측정/보고)

```bash
# clap 링크 전후 비교 (Wave 4)
ls -la "$BIN"
cargo install cargo-bloat 2>/dev/null; cargo bloat -p axhub-helpers --release --crates | head -20
# 5개 cross-arch 타겟 size 는 release dry-run 또는 CI artifact 로 기록
```

## 완료 정의 (per-wave)

각 wave 종료 시:
- [ ] `cargo test -p axhub-helpers` green (1~2 usage-wording assert 외 무수정)
- [ ] `cargo clippy ... -D warnings` 0
- [ ] 해당 wave class=H 명령 fail-open exit 0 (절차 2)
- [ ] `bunx tsc --noEmit` clean (인접 TS hook 무영향 확인)

전체 완료(Wave 4):
- [ ] `while i < args.len()` 카운트 0 (SC-004)
- [ ] `USAGE` 상수 제거, passthrough variant 제거
- [ ] hook 파일 git diff 0 (SC-002)
- [ ] binary size delta 기록 (D8)
