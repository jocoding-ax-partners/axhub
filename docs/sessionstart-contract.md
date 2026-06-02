# SessionStart systemMessage Contract — Approach E

원본 plan: `.plan/ceo-review-nl-routing/2026-05-07-nl-routing-redesign.md`
Phase 0 sub-task 0.6 deliverable. Phase 7 (Component 6) implementation contract.

---

## Background

codex Ralph 추가 리뷰 의 Amendment 5 가 발견한 사실: 현재 `crates/axhub-helpers/src/bootstrap.rs` 의 SessionStart systemMessage 는 "axhub helper Rust runtime이에요." 수준이에요. 원본 plan 이 "fallback 안내가 이미 있어요" 라고 가정한 건 사실 X. Phase 7 가 실제로 구현해야 해요.

이 contract 가 Phase 7 의 implementation source of truth 예요.

---

## Base systemMessage (모든 세션)

매 SessionStart 마다 출력해요. 한국어 해요체. 3-4 줄.

```
axhub helper Rust runtime 활성 (v{VERSION}).
막히면 /axhub:help 로 명령 메뉴를, /axhub:clarify 로 모호한 의도 확인을 부탁해요.
라우팅 통계는 axhub-helpers routing-stats 로 봐요.
```

`{VERSION}` 은 `env!("CARGO_PKG_VERSION")` 으로 compile-time 주입.

---

## v0.4.0 첫 세션 magical moment (한 번만)

marker 파일 (`runtime_paths::state_dir() / .v0.4.0-welcome-shown`) 부재 시 base 메시지 *뒤에* 추가 6 줄. marker write 후 다음 세션부터 표시 X.

```
[axhub v0.4.0 첫 세션] 라우팅 똑똑해졌어요.
- Rust 키워드 체인 ~600줄 폐기. Claude 가 SKILL.md description 으로 직접 매칭.
- 메타 질문 ("왜 ~ 키워드 매칭이야?") 자동 처리.
- routing audit log 7일 로컬 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1.
- 짧은 prompt 의 hash 는 익명화 보장 안 돼요.
```

audit privacy 한 줄 (`AXHUB_NO_AUDIT=1` + 익명화 한계) 가 docs/audit-privacy-contract.md 의 Disclosure 위치 #1 와 일치.

---

## Marker 동작

- **위치**: `runtime_paths::state_dir() / .v0.4.0-welcome-shown`
- **내용**: `shown:{ISO8601}` 형식 1 줄 (디버깅 용)
- **권한**: file `0600` (audit log 와 동일 정책)
- **Failure handling**: write 실패 시 graceful — 다음 세션에서 또 알림 표시 OK. user-facing crash X.

```rust
// pseudo-code
let marker = welcome_marker_path("0.4.0");
if !marker.exists() {
    lines.push(v040_welcome_block());
    if let Some(parent) = marker.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&marker, format!("shown:{}", now_iso8601()));
}
```

---

## 시나리오 별 출력

### 시나리오 1 — v0.3.x 사용자가 v0.4.0 첫 install 후 첫 세션

base 3 줄 + blank line + v0.4.0 6 줄 = 총 10 줄. marker write.

### 시나리오 2 — 같은 사용자의 두 번째 세션

base 3 줄만. marker exists.

### 시나리오 3 — v0.5.x 출시 후 v0.5.0 첫 세션

`{WELCOME_VERSION}` const 가 `0.5.0` 으로 갱신되어 있으면 새 marker (`.v0.5.0-welcome-shown`) 부재 → v0.5.0 release notes 표시. 이전 v0.4.0 marker 는 삭제 또는 그대로 두기 (diskspace negligible).

### 시나리오 4 — preflight error (axhub CLI 미설치)

base 3 줄 출력. preflight 결과는 `prompt-route` 의 additionalContext 에 따로 inject 되어 있음 — SessionStart 에선 base 만.

---

## Implementation Outline (Phase 7)

`crates/axhub-helpers/src/bootstrap.rs` 의 `cmd_session_start` 함수:

```rust
const WELCOME_VERSION: &str = "0.4.0";

pub fn cmd_session_start() -> anyhow::Result<i32> {
    let mut lines = vec![
        format!(
            "axhub helper Rust runtime 활성 (v{}).",
            env!("CARGO_PKG_VERSION")
        ),
        "막히면 /axhub:help 로 명령 메뉴를, /axhub:clarify 로 모호한 의도 확인을 부탁해요."
            .to_string(),
        "라우팅 통계는 axhub-helpers routing-stats 로 봐요.".to_string(),
    ];

    let marker_path = welcome_marker_path(WELCOME_VERSION);
    if !marker_path.exists() {
        lines.push(String::new()); // blank separator
        lines.push("[axhub v0.4.0 첫 세션] 라우팅 똑똑해졌어요.".to_string());
        lines.push("- Rust 키워드 체인 ~600줄 폐기. Claude 가 SKILL.md description 으로 직접 매칭.".to_string());
        lines.push("- 메타 질문 (\"왜 ~ 키워드 매칭이야?\") 자동 처리.".to_string());
        lines.push("- routing audit log 7일 로컬 보관 (외부 전송 X). 끄려면 AXHUB_NO_AUDIT=1.".to_string());
        lines.push("- 짧은 prompt 의 hash 는 익명화 보장 안 돼요.".to_string());

        if let Some(parent) = marker_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&marker_path, format!("shown:{}", now_iso8601()));
    }

    let context = lines.join("\n");
    out_json(json!({
        "hookSpecificOutput": {
            "hookEventName": "SessionStart",
            "additionalContext": context,
        }
    }));
    Ok(0)
}
```

`welcome_marker_path(version)` helper 는 `runtime_paths.rs` 안 신규 — `runtime_paths::state_dir().join(format!(".v{}-welcome-shown", version))`.

---

## Test Contract (Phase 7)

`crates/axhub-helpers/tests/cli_e2e.rs` 가 다음 3 test 포함:

1. **`session_start_first_v040_session`** — marker 부재 시 v0.4.0 6 줄 포함 + marker file 생성
2. **`session_start_subsequent_session`** — marker 존재 시 base 3 줄만, v0.4.0 알림 X
3. **`session_start_marker_create_failure`** — marker write 실패 (read-only fs mock) 시 base + v0.4.0 출력 + crash X

추가 lint:

- `bun run lint:tone --strict` 의 한국어 해요체 검증 통과 (위 모든 문구 해요체)

---

## Rollback

이 contract 의 모든 문구는 `bootstrap.rs` 안 string literal 이라 git revert 1 commit 으로 복구 가능해요. marker 파일은 사용자 디스크에 자국 minimal (KB 단위) — downgrade 시 그대로 두어도 무해.

---

## Linked Phase 0 Sub-tasks

- **0.5** audit privacy disclosure → `[axhub v0.4.0 첫 세션]` 블록의 audit + 익명화 한계 문구
- **0.6** SessionStart fallback systemMessage → 이 문서 (base + v0.4.0 magical moment 정의)
- **0.7** Migration Gate → Phase 7 implementation 후 Gate 1 (cli_e2e), Gate 4 (latency) 통과
