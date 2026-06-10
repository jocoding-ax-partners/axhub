# Vendored data-contract rules — PROVENANCE

`data-contract-rules.json` 은 손으로 쓰지 않아요. distiller (`gen-sdk-distill.py`) 가
PINNED_SDK.lock.json 의 고정 sha 에서 파생해 emit 한 산출물의 **byte-identical 복사본**이에요.

| 항목 | 값 |
|---|---|
| 원본 경로 | `sdk/dist/sdk-knowledge/data-contract-rules.json` (branch `feat/knowledge-artifacts`) |
| 스키마 | `sdk/dist/sdk-knowledge/schemas/data-contract-rules.schema.json` |
| lock_sha (route_surface_sha) | `8bafa90e7d9319b78514a1e95b19c0fb3b73d558` |
| 룰 수 | 12 (block 9 / advisory 3) |
| 복사일 | 2026-06-10 (re-vendor: 10→12룰) |
| 소비처 | `crates/axhub-helpers/src/ast_validate.rs` (`include_str!`) |

### 12룰 re-vendor (2026-06-10)

distiller 가 신규 emit 한 block 룰 2종 반영:
- `raw-http-axhub-data-endpoint-forbidden` (forbidden_call, 6언어, `DATA_RELIABILITY§wire-paths`) — raw fetch/axios/http 로 axhub data 엔드포인트 직타 금지.
- `use-client-imports-server-only-axhub` (boundary, node, `CANONICAL_WRAPPER§node-server-side-only`) — `"use client"` 컴포넌트의 server-only `@ax-hub/sdk` import 금지.

두 룰 모두 pattern 이 URL/path(`/`)를 타겟 → validator 가 문자열(주석 제외)을 스캔해요.

## 갱신 절차

룰은 vendored 단일 원천이라 직접 편집 금지. SDK pin 이 바뀌면:

1. sdk repo 에서 distiller 재실행 → 새 `data-contract-rules.json` emit.
2. 본 파일을 새 산출물로 **byte-identical** 재복사.
3. 위 표의 `lock_sha` / 복사일 갱신.
4. `cargo test -p axhub-helpers ast_validate` 로 회귀 확인.

각 룰은 `derived_from` (origin 참조) 필수 — 없으면 `ast_validate` 로드가 실패해요
(enforcement drift 차단). `advisory_only:true` 룰은 정적 미결정이라 영구 warn 트랙이고
block 으로 승격하지 않아요.

## Follow-up: PR #198 스택 머지 시

현재 advisory 메시지는 **위임 문구 없는 권고 warn** 만 방출해요 (plan fallback (ii) —
`migrate-data-verify` 가 PR #187←#197←#198 스택 산출물이라 미머지 환경에선 死문구가 되기
때문, `ast_validate.rs` `rule_message` doc-comment 및 `advisory_messages_recommend_without_delegation`
테스트로 잠금). 스택 머지 후 활성화할 것:

1. advisory 메시지에 `axhub-helpers migrate-data-verify` 런타임 위임 문구 추가.
2. 선행 의존성 기계 분기 추가 — `migrate-data-verify --help` 존재 체크(exit 0) 또는
   PR #198 merge commit 의 `git merge-base --is-ancestor` assert 로 위임/권고 모드 자동 전환.
3. 위 잠금 테스트를 분기-aware 로 갱신 (위임 모드에선 위임 문구 포함을 assert).
