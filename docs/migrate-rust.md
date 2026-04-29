# Rust helper 전환 가이드

axhub-helpers 는 Rust helper 를 기본 binary 로 사용해요. Bun 은 repo 스크립트와 전환 fallback 검증에 남아 있고, 사용자 release artifact 는 Rust native binary 예요.

## 자동 마이그레이션

```bash
axhub update
```

업데이트는 helper binary 만 교체해요. 토큰, profile, app 설정은 그대로 유지해요.

## Runtime 선택

```bash
export AXHUB_HELPERS_RUNTIME=auto   # 기본값, Rust helper 우선
export AXHUB_HELPERS_RUNTIME=rust   # Rust helper 강제
export AXHUB_HELPERS_RUNTIME=ts     # monitor window rollback
```

- `auto`: 현재 release 에서는 Rust helper 를 우선 사용해요.
- `rust`: Rust helper 가 없으면 바로 실패해요.
- `ts`: TypeScript fallback 만 사용해요. 회귀가 보일 때 임시 우회용이에요.

## 호환성 약속

### 그대로 유지해요

- Token/env 계약: `AXHUB_TOKEN`, `AXHUB_ENDPOINT`, `AXHUB_ALLOW_PROXY`, `AXHUB_HELPERS_RUNTIME`.
- Consent token: HS256, zero leeway, 60초 TTL, session/tool-call binding.
- keychain read path: macOS Keychain, Linux Secret Service, Windows Credential Manager guidance.
- Hub API fallback: bearer token 전송 전 TLS SPKI pin 확인.
- Hook JSON schema: `hookSpecificOutput` / `systemMessage` 구조 유지.
- 한국어 user-facing 메시지: 해요체 톤 유지.

### 바뀌어요

- helper release artifact 는 Bun-compiled binary 가 아니라 Rust native binary 예요.
- release workflow 는 Bun cross compile 대신 Rust target matrix 로 5개 binary 를 만들어요.
- `bun run build` 는 Cargo wrapper 로 동작해요. Bun 은 helper compile 이 아니라 repo script runner 로만 남아요.

## Rollback

회귀가 보이면 TypeScript fallback 을 강제해요.

```bash
export AXHUB_HELPERS_RUNTIME=ts
axhub:doctor
```

문제가 release artifact 자체라면 이전 서명 release 로 되돌려요.

```bash
export AXHUB_HELPERS_RUNTIME=ts
axhub update --force-version 0.1.23
```

## Platform notes

- macOS: Keychain live read smoke 통과했어요.
- Linux: Docker 안에서 Secret Service live read smoke 통과했어요. headless 환경은 `AXHUB_TOKEN` fallback 을 유지해요.
- Windows: Credential Manager parser/runner branch 는 테스트돼요. V3/AhnLab cohort 는 실제 Windows/EDR 환경에서 매 release 수동 확인이 필요해요.

## 검증 baseline

- `cargo test --workspace`.
- `cargo llvm-cov --workspace --fail-under-lines 90`.
- `bun test` / `bun run test:plugin-e2e:t1` / `bun run test:plugin-e2e:t2`.
- `bun run release:check` 로 host Rust artifact 와 release matrix wiring 을 확인해요.
