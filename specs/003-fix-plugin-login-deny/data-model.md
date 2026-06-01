# Phase 1 Data Model: 플러그인 로그인 consent deny 수정

**Date**: 2026-06-01 | **Plan**: [plan.md](./plan.md)

> **요지**: 이 수정은 데이터 **모양(schema)** 을 바꾸지 않아요. consent 토큰의 **저장 위치(경로 해석)** 만 바뀌어요. 새 필드·새 엔티티 없음.

## Entity: Consent Token (변경 없음)

디스크의 `TokenFile`(jwt.rs:58-64)과 그 안의 JWT `Claims`(jwt.rs:42-57). **본 작업으로 필드 추가/삭제/타입 변경 없음.**

### TokenFile (디스크 JSON, `0600`)

| 필드 | 타입 | 의미 | 본 작업 영향 |
|---|---|---|---|
| `token_id` | String (UUID v4) | 토큰 식별자 | 불변 |
| `jwt` | String (HS256) | 서명된 Claims | 불변 |
| `expires_at` | String (RFC3339) | 만료 시각 | 불변 |
| `session_id` | String | 세션 바인딩 (`pending` = 부트스트랩) | 불변 |

### Claims (JWT 페이로드)

`tool_call_id`, `action`, `app_id`, `profile`, `branch`, `commit_sha`, `context`, `synthesized_by_helper`(audit-only), `jti`, `iat`, `exp` — **전부 불변**. 검증 로직(`binding_mismatch_reason`, TTL `exp<=now`, HMAC)도 불변.

### 보안 계약 (불변, FR-003/FR-004)

- **TTL**: 60초 (`mint_token(b, 60)`), `exp <= now` 면 거부.
- **pending single-use**: `/axhub:auth` bootstrap 의 pending consent 는 매칭 시 `fs::remove_file` 로 1회 소비. 기존 session/always decision token semantics 는 본 작업에서 새로 바꾸지 않음.
- **HMAC**: `state_root()/hmac-key` 로 서명·검증.
- **권한**: 파일 `0600`(`write_private_file_no_follow`), 디렉터리 `0700`(`set_private_dir_mode`).

## 변경점: 저장 위치 (경로 해석만)

### `runtime_root()` 해석표 (Before → After)

| 환경 | 변수 | Before | After | 영향 |
|---|---|---|---|---|
| Linux (systemd) | `XDG_RUNTIME_DIR=/run/user/$UID` | `/run/user/$UID/axhub` | **동일** | 무회귀 |
| 테스트 | `XDG_RUNTIME_DIR=<tempdir>` | `<tempdir>/axhub` | **동일** | 기존 E2E 그대로 |
| macOS (Claude Code) | 미설정 → `$TMPDIR` 폴백 | `$TMPDIR/axhub` (**프로세스마다 다름** → 버그) | `~/.local/state/axhub/runtime` (**프로세스 무관**) | **버그 수정** |
| HOME 미설정 (Windows 등) | 미설정 | `$TMPDIR/axhub` | `./.local/state/axhub/runtime`(상대) | R4 경계 — 범위 외 |

### 파일 경로 (위치만 이동, 파일명 불변)

| 파일 | 빌더 | 경로 |
|---|---|---|
| 세션 토큰 | `token_file_path(sid)` | `runtime_root()/consent-{sid}.json` |
| pending 토큰 | `pending_token_file_path(token_id)` | `runtime_root()/consent-pending-{token_id}.json` |

## 상태 전이 (불변, 참고)

```text
[없음] --mint--> [pending consent 파일 (exp=now+60s)]
   pending --claim 매칭(preauth-check)--> [소비/삭제] → allow
   pending --exp 초과--> [만료] --(claim 또는 mint/preauth 스윕)--> [삭제]
   session --exp 초과--> [만료] --(mint/preauth 스윕)--> [삭제]
   pending --binding 불일치--> [유지] → deny (사유 표면화: P2)
```

**FR-007 스윕 추가점**: `mint` / `preauth-check` 진입 시 `runtime_root()` 의 만료 `consent-*.json` 파일 정리(pending 과 session 파일 모두). 유효 토큰은 미영향.

## 영향 받는 심볼 (블라스트 반경)

`runtime_root()` 소비처 = 4곳 (전부 consent 모듈):

| 심볼 | 파일:라인 | 용도 |
|---|---|---|
| `token_file_path` | key.rs:38 | 세션 토큰 경로 |
| `pending_token_file_path` | key.rs:41 | pending 토큰 경로 |
| `mint_token_to_path` | jwt.rs:137-138 | `create_dir_all` + `0700` |
| `claim_pending_token` | jwt.rs:209 | `read_dir` + 스윕 |

→ 외부 모듈 의존 없음. `state_root()`(learning.rs, audit_ledger.rs 사용)는 **변경하지 않음**.
