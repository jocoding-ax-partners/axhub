# Rust helper 전환 가이드

axhub-helpers 는 v1.0 전환 기간 동안 TypeScript helper 와 Rust helper 를 함께 유지해요. 기본값은 `auto` 예요.

## 자동 마이그레이션

- `AXHUB_HELPERS_RUNTIME=auto`: Rust helper 가 설치되어 있으면 Rust 를 쓰고, 없으면 TypeScript helper 로 돌아가요.
- `AXHUB_HELPERS_RUNTIME=rust`: Rust helper 만 실행해요. 없으면 즉시 실패해요.
- `AXHUB_HELPERS_RUNTIME=ts`: TypeScript helper 만 실행해요. 회귀가 보이면 이 값으로 즉시 우회할 수 있어요.

## 호환성 약속

- 기존 슬래시 명령과 자연어 SKILL 흐름은 그대로 유지해요.
- HMAC consent token, TLS pin, keychain token import 계약은 Rust 테스트와 TypeScript 테스트가 같이 잠가요.
- 전환 기간에는 TypeScript helper 를 삭제하지 않아요.

## Rollback

회귀가 보이면 아래처럼 TypeScript runtime 을 강제해요.

```bash
export AXHUB_HELPERS_RUNTIME=ts
```

그 다음 `axhub:doctor` 또는 `axhub-helpers version` 으로 현재 runtime 을 확인해요.

## Known platform 차이

- Windows Credential Manager 는 EDR/AMSI 차단 가능성이 있어요. Rust 포팅 중에도 `AXHUB_TOKEN` 우회 경로를 유지해요.
- Linux Secret Service 없는 headless 환경은 `AXHUB_TOKEN` 또는 token file 경로를 우선 사용해요.
