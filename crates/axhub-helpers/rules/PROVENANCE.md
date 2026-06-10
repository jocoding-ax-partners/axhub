# Vendored data-contract rules — PROVENANCE

`data-contract-rules.json` 은 손으로 쓰지 않아요. distiller (`gen-sdk-distill.py`) 가
PINNED_SDK.lock.json 의 고정 sha 에서 파생해 emit 한 산출물의 **byte-identical 복사본**이에요.

| 항목 | 값 |
|---|---|
| 원본 경로 | `sdk/dist/sdk-knowledge/data-contract-rules.json` |
| 스키마 | `sdk/dist/sdk-knowledge/schemas/data-contract-rules.schema.json` |
| lock_sha (route_surface_sha) | `8bafa90e7d9319b78514a1e95b19c0fb3b73d558` |
| 복사일 | 2026-06-10 |
| 소비처 | `crates/axhub-helpers/src/ast_validate.rs` (`include_str!`) |

## 갱신 절차

룰은 vendored 단일 원천이라 직접 편집 금지. SDK pin 이 바뀌면:

1. sdk repo 에서 distiller 재실행 → 새 `data-contract-rules.json` emit.
2. 본 파일을 새 산출물로 **byte-identical** 재복사.
3. 위 표의 `lock_sha` / 복사일 갱신.
4. `cargo test -p axhub-helpers ast_validate` 로 회귀 확인.

각 룰은 `derived_from` (origin 참조) 필수 — 없으면 `ast_validate` 로드가 실패해요
(enforcement drift 차단). `advisory_only:true` 룰은 정적 미결정이라 영구 warn 트랙이고
block 으로 승격하지 않아요.
