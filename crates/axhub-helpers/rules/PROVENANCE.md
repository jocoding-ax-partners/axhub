# Vendored data-contract rules — PROVENANCE

`data-contract-rules.json` 은 손으로 쓰지 않아요. distiller (`gen-sdk-distill.py`) 가
PINNED_SDK.lock.json 의 고정 sha 에서 파생해 emit 한 산출물의 **byte-identical 복사본**이에요.

| 항목 | 값 |
|---|---|
| 원본 경로 | `sdk/dist/sdk-knowledge/data-contract-rules.json` (branch `feat/knowledge-artifacts`) |
| 스키마 | `sdk/dist/sdk-knowledge/schemas/data-contract-rules.schema.json` |
| lock_sha (route_surface_sha) | `8bafa90e7d9319b78514a1e95b19c0fb3b73d558` |
| 룰 수 | 21 (block 18 / advisory 3) |
| 복사일 | 2026-06-10 (re-vendor: 12→21룰, F1 FP 수정분) |
| 소비처 | `crates/axhub-helpers/src/ast_validate.rs` (`include_str!`) |

### 21룰 re-vendor (2026-06-10, F1 distiller FP 수정)

distiller regex FP 수정분 반영 (sdk repo `feat/knowledge-artifacts`, CE 리뷰 5건):
- **or/not/cursor 룰 언어별 분리** (12→21룰의 본체) — 전언어 공통 `\bor\s*\(`/`\bnot\s*\(`
  가 Python `not (a)`, Ruby `(a) or (b)` boolean 키워드에 FP 를 내서 언어별 SDK 표면
  시그니처로 분리: node `or(`/`not(`, go `Or(`/`Not(`, python/ruby `or_(`/`not_(`,
  jvm `Ops.or(`/`Ops.not(`. cursor 도 node/ruby `after:`, python `after=`, go `After:`,
  jvm `.after(` 로 분리.
- **email-domains lookbehind 방향 수정** — lookahead `(?!\s*/api/v1)` (항상 통과)
  → invite-links 와 같은 lookbehind `(?<!/api/v1)` 로. 정상 prefix URL FP 해소.
- **use-client 따옴표 무관** — `['\"]use client['\"]` 로 단따옴표 `'use client'` 도 검출.

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

**완료 (2026-06-10, v0.10.0 스택 머지 후 활성화)** — advisory 메시지는 권고("권장")와 함께
`axhub-helpers migrate-data-verify` 런타임 검증 안내를 방출해요. 처리 내역:

1. ✓ advisory 메시지에 `axhub-helpers migrate-data-verify` 런타임 위임 문구 추가
   (`ast_validate.rs` `rule_message`).
2. ✓ 선행 의존성 기계 분기는 **불필요 판정** — `migrate-data-verify` 가 같은 바이너리의
   dispatch 라 항상 존재해요. `--help` 존재 체크·merge-base assert 는 위임 대상이 별도
   PR 스택에 있던 시절의 안전장치였어요.
3. ✓ 잠금 테스트를 `advisory_messages_delegate_to_migrate_data_verify` 로 갱신
   (위임 문구 포함을 assert).
