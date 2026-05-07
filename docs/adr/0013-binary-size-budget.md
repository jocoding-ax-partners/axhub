# ADR 0013 — Binary size budget

## Status

Proposed. v0.3.3 릴리스 완료 후 `binary-sizes.json` 실제 값이 채워지면 Accepted로 promote해요.

## Drivers

1. v0.1.14 stale binary 사고 재발 방지 — `release:check` postbump hook이 버전 일치를 검증하지만 binary 크기 이상 증가는 감지하지 못해요.
2. release.yml 5-arch cosign signing 시간 SLA 영향 — binary가 커질수록 CI 빌드 시간과 GH release 업로드 시간이 늘어나 `vibe coder 5분 배포` SLA에 영향을 줘요.
3. CI 사전 검출 — PR review 시점에 size delta를 가시화해 의존성 실수를 merge 전에 잡아요.

## Context

v0.1.14 릴리스에서 `release:check` postbump hook 없이 버전 bump 만 진행해 stale binary가 GH release에 올라간 사고가 있었어요. 해당 binary는 이전 버전 코드를 그대로 담고 있어 서명 검증은 통과했지만 실제 실행 버전이 달랐어요.

재발 방지로 Phase 19에서 `commit-and-tag-version` postbump hook에 `release:check`를 연결했어요. `release:check`는 5개 아키텍처 binary를 로컬 빌드한 뒤 각 binary의 `--version` 출력이 `package.json` 버전과 일치하는지 단언해요.

그러나 binary 크기 자체에 대한 상한은 정의되지 않았어요. Rust 릴리스 빌드 + strip 후에도 의존성 추가나 feature gate 실수로 binary가 비정상적으로 커질 수 있어요. 이를 사전에 감지할 측정 절차와 CI 트리거 조건이 없었어요.

axhub-helpers는 Claude Code 플러그인 번들에 포함되어 설치 시 다운로드되므로 binary 크기는 `vibe coder 5분 배포` SLA에 직접 영향을 줘요.

## Alternatives

**(a) threshold 없음 (현 상태)** — 기각. binary 크기 이상 증가를 자동으로 감지할 수 없어 v0.1.14와 유사한 배포 품질 저하가 반복될 수 있어요.

**(b) absolute byte 상한 (예: 50 MB cap)** — 기각. 아키텍처별 기대 크기가 다르고, Rust 릴리스 빌드의 절대 크기는 toolchain 업그레이드마다 자연 증가해 고정 cap은 잦은 ADR 개정을 유발해요.

**(c) percentage delta (채택)** — 직전 릴리스 베이스라인 대비 비율로 비교하면 아키텍처 차이와 toolchain 자연 증가를 수용하면서 비정상적인 급증을 감지할 수 있어요. +10% warn / +25% fail은 일반적인 의존성 추가(통상 5% 이내)를 허용하면서 대형 실수(feature gate 오류, debug symbol 포함)를 차단해요.

## Decision

릴리스마다 5개 아키텍처 binary 크기를 측정하고 베이스라인 대비 +10% 초과 시 warn, +25% 초과 시 CI fail로 처리해요.

베이스라인은 `.omc/baselines/binary-sizes.json`에 버전별로 기록해요. 측정 절차와 threshold는 아래 Consequences에 명시해요.

## Consequences

### Measurement procedure

```bash
# 1. 5개 아키텍처 릴리스 binary 빌드 (strip 포함)
#    release.yml의 "Build Rust helper" + "Rename release asset" 단계와 동일 절차

cargo build --release -p axhub-helpers --target x86_64-unknown-linux-gnu
cargo build --release -p axhub-helpers --target aarch64-unknown-linux-gnu   # cross
cargo build --release -p axhub-helpers --target x86_64-apple-darwin
cargo build --release -p axhub-helpers --target aarch64-apple-darwin
cargo build --release -p axhub-helpers --target x86_64-pc-windows-msvc

# non-Windows: strip 적용
strip target/<target>/release/axhub-helpers

# 2. 크기 측정
du -h dist/axhub-helpers-{darwin-arm64,darwin-amd64,linux-amd64,linux-arm64,windows-amd64.exe}
```

### Baseline 기록 형식

`.omc/baselines/binary-sizes.json`:

```json
{
  "version": "0.3.3",
  "measured_at": "2026-05-07T00:00:00Z",
  "baselines": {
    "axhub-helpers-darwin-arm64":       { "bytes": 0, "sha256": "" },
    "axhub-helpers-darwin-amd64":       { "bytes": 0, "sha256": "" },
    "axhub-helpers-linux-amd64":        { "bytes": 0, "sha256": "" },
    "axhub-helpers-linux-arm64":        { "bytes": 0, "sha256": "" },
    "axhub-helpers-windows-amd64.exe":  { "bytes": 0, "sha256": "" }
  }
}
```

`sha256` 는 `checksums.txt`와 교차 검증해요. 실제 값은 v0.3.3 릴리스 완료 후 채워요.

### Trigger condition

| Delta | 동작 |
|-------|------|
| ≤ +10% | pass (허용 범위) |
| +10% ~ +25% | warn — PR body에 크기 변화 기록 필수 |
| > +25% | CI fail — 의존성 감사 후 재빌드 필요 |

- 측정 기준: 직전 릴리스 태그의 `binary-sizes.json` 베이스라인
- Windows binary는 Authenticode 서명 패딩(~4 KB)을 고려해 별도 threshold 적용 없이 동일 기준 사용 (패딩 크기가 threshold 오차보다 작음)
- `release:check` postbump hook이 완료된 뒤 크기 측정을 수행해 stale binary 측정을 방지해요

## Trigger

1. **v0.3.3 릴리스 완료 후** — `binary-sizes.json`에 실제 측정값(bytes + sha256)을 채우고 Status를 Accepted로 promote해요.
2. **v0.3.4 첫 +10% 초과 PR 발생 시** — PR body에 delta 수치와 원인 분석을 기록하는 절차를 enforce해요. 해당 시점에 `release:check` postbump hook에 크기 검사 단계를 추가할지 재검토해요.

### Non-goals

- 이 ADR은 binary 크기를 최소화하는 최적화 방법을 규정하지 않아요.
- 이 ADR은 Windows Authenticode 서명 절차를 변경하지 않아요.
- 이 ADR은 `bun run release` 워크플로 자체를 변경하지 않아요.
