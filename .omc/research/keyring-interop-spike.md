# Keyring interop spike — go-keyring envelope

작성일: 2026-04-29

## 목적

axhub-cli 가 저장하는 `go-keyring-base64:<base64-json>` envelope 를 Rust helper 가 같은 방식으로 해석할 수 있는지 확인해요.

## 현재 세션 검증

- Rust `parse_keyring_value()` 가 `go-keyring-base64:` prefix 를 제거해요.
- base64 JSON 을 decode 하고 `access_token` 을 추출해요.
- `cargo test -p axhub-helpers keychain_parses_go_keyring_envelope` 로 잠갔어요.

## OS matrix

| OS | live keyring read | attribute key | 결정 |
| --- | --- | --- | --- |
| macOS | 현재 세션에서 live axhub-cli login 검증은 보류예요 | `service=axhub` 유지 | subprocess-compatible runner 유지 |
| Linux | Secret Service live 검증은 보류예요 | `service=axhub` 유지 | `secret-tool lookup service axhub` 호환 runner 유지 |
| Windows | Credential Manager live 검증은 보류예요 | `TargetName=axhub` 유지 | PowerShell/PInvoke runner 유지 |

## 결론

Envelope 파싱 계약은 Rust 테스트로 잠겼어요. 실제 OS keyring API 호환성은 Phase 3 live QA 에서 3 OS 별로 재검증해야 해요.
