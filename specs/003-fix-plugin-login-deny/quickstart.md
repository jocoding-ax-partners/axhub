# Quickstart: 재현 → 수정 → 검증

**Date**: 2026-06-01 | **Plan**: [plan.md](./plan.md)

이 문서는 버그를 **fail-before / pass-after** 로 증명하는 절차예요. 핵심 원칙: **수정 전에 먼저 deny 를 재현**해야 진짜 재현과 동어반복(tautology) 테스트를 구분할 수 있어요(advisor).

## 핵심 원인 한 줄

`runtime_root()` 가 `XDG_RUNTIME_DIR` 부재 시 `$TMPDIR` 폴백 → Claude Code 가 mint(Bash tool)와 hook subprocess 에 다른 `$TMPDIR` 부여 → consent 파일 경로 어긋남 → deny.

## 1. 재현 (수정 전 — DENY 여야 함)

버그는 **`XDG_RUNTIME_DIR` 가 unset** 이고 **mint 와 read 의 `$TMPDIR` 가 다를 때**만 나요. 기존 테스트가 전부 `XDG_RUNTIME_DIR` 를 세팅해서 이걸 가렸어요.

수동 재현 (shell):
```bash
BIN=$(ls target/*/axhub-helpers 2>/dev/null | head -1)   # 또는 cargo build 후 경로
ROOT="$(mktemp -d)"
export XDG_STATE_HOME="$ROOT/state"          # HMAC 키·state 격리
TMP_A="$ROOT/tmp-a"
TMP_B="$ROOT/tmp-b"
mkdir -p "$TMP_A" "$TMP_B"
JSON='{"tool_call_id":"pending","action":"auth_login","app_id":"_","profile":"default","branch":"_","commit_sha":"_","context":{}}'

# mint 를 TMPDIR=A 에서
echo "$JSON" | env -u XDG_RUNTIME_DIR TMPDIR="$TMP_A" "$BIN" consent-mint >/dev/null

# preauth-check 를 TMPDIR=B 에서 (다른 $TMPDIR — hook subprocess 모사)
PAYLOAD='{"session_id":"s1","tool_call_id":"t1","tool_name":"Bash","tool_input":{"command":"axhub auth login --force --no-browser --json"}}'
echo "$PAYLOAD" | env -u XDG_RUNTIME_DIR TMPDIR="$TMP_B" "$BIN" preauth-check
# 기대(수정 전): permissionDecision":"deny"  ← 버그 재현
```

## 2. 수정 (Phase 2 implement)

`crates/axhub-helpers/src/consent/key.rs` `runtime_root()` 폴백을 `state_root().join("runtime")` 로 (research R1):
```rust
pub fn runtime_root() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(|d| PathBuf::from(d).join("axhub"))
        .unwrap_or_else(|| state_root().join("runtime"))
}
```

## 3. 검증 (수정 후 — ALLOW 여야 함)

위 §1 재현 절차를 다시 실행 → `permissionDecision":"allow"` 여야 해요(같은 `$TMPDIR` 차이에도 consent 발견).

## 4. 자동 회귀 테스트 (신규 — `tests/cli_e2e.rs`)

기존 consent 테스트가 못 잡은 폴백 케이스를 고정해요:

```rust
// 의사코드 — 실제는 기존 cli_e2e 헬퍼 스타일에 맞춤
#[test]
fn preauth_allows_when_tmpdir_differs_and_xdg_runtime_unset() {
    let root = tempdir();
    let state = root.path().join("state");
    let tmp_a = root.path().join("tmp-a");
    let tmp_b = root.path().join("tmp-b");
    std::fs::create_dir_all(&tmp_a).unwrap();
    std::fs::create_dir_all(&tmp_b).unwrap();
    // mint: TMPDIR=A, XDG_RUNTIME_DIR 제거
    run_with_env_overrides(&BIN, &["consent-mint"], pending_login_json())
        .env("XDG_STATE_HOME", &state)
        .env_remove("XDG_RUNTIME_DIR")
        .env("TMPDIR", &tmp_a);
    // preauth-check: TMPDIR=B (다른 값), 같은 XDG_STATE_HOME
    let out = run_with_env_overrides(&BIN, &["preauth-check"], login_payload())
        .env("XDG_STATE_HOME", &state)
        .env_remove("XDG_RUNTIME_DIR")
        .env("TMPDIR", &tmp_b);
    assert_eq!(decision(&out), "allow");   // 수정 전엔 "deny" → fail-before 증명
}
```

추가 테스트:
- **권한 보존**: 새 경로 `~/.local/state/axhub/runtime` 디렉터리 `0700`, consent 파일 `0600`.
- **TTL 무력화**: 만료(`exp<=now`) consent 는 `$TMPDIR` 무관하게 deny.
- **만료 스윕(FR-007)**: 만료 pending 파일과 만료 session consent 파일 존재 + mint/preauth 1회 → 만료 파일 제거, 미만료 파일 보존 확인.
- **무회귀**: `XDG_RUNTIME_DIR` 세팅된 기존 consent E2E 전부 green 유지.
- **문서 무회귀**: README 의 consent 경로 설명이 `XDG_RUNTIME_DIR` 설정/미설정 두 경로를 모두 설명하고, `${XDG_RUNTIME_DIR:-/tmp}/axhub` 같은 오래된 fallback 안내가 남지 않았는지 확인.

## 5. 게이트 (CLAUDE.md Self-Check)

```bash
cargo test -p axhub-helpers          # 신규 회귀 + 기존 consent E2E
cargo clippy --all-targets -- -D warnings
bunx tsc --noEmit
bun run lint:tone --strict           # 한글 해요체 (신규 메시지 없으면 무영향)
```

## 6. 인덱스/릴리스 (커밋 후)

```bash
npx gitnexus analyze                 # PostToolUse hook 자동 (인덱스 갱신)
# 릴리스는 별도: bun run release → narrative amend → bun run release:tag
```

## P2 검증 (선택, 분리 태스크)

`permissionDecisionReason` 추가 후, deny payload 에 해당 필드 + `systemMessage` **둘 다** 존재 확인. 설치된 Claude Code 에서 실제 deny UI 에 사유가 보이는지 실측(research R3).
