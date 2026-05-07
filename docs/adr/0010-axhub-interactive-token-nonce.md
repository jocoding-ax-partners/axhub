# ADR 0010 — AXHUB_INTERACTIVE_TOKEN nonce

## Status

Proposed — PoC 구현 대기 중이에요.

## Context

PR #41 은 `recommended_command` byte-identical 검사를 5번째 hard invariant 로 확보했어요. 그러나 이 검사는 텍스트 비교에 의존해요. SKILL author 가 preview 문구를 조금만 바꿔도 gate 가 통과되고, CI / `claude -p` 같은 subprocess 환경에서는 부모 세션의 stale env var 가 그대로 상속되어 replay attack 창이 열려요.

현재 hard invariant 5개 중 machine-enforceable 한 것은 다음과 같아요.

| # | Invariant | Enforcement |
|---|-----------|-------------|
| 1 | frontmatter doctor | `bun run skill:doctor --strict` |
| 2 | registry enum | `tests/ux-ask-fallback-registry.test.ts` |
| 3 | Rust runtime guard | `cmd_preauth_check` HMAC verify |
| 4 | allowlist governance | `skill-doctor-allowlist.json` |
| 5 | recommended_command byte-identical | NEVER 룰 텍스트 (advisory) |

invariant 5는 3 조건이 동시 충족될 때만 dependency install 실행을 허용해요.

> **PR #41 NEVER 룰 3조건**: SKILL 이 inline `!` prefix 로 실행할 때, (1) frontmatter `allows-dependency-execution: true`, (2) `axhub-helpers dependency-plan` 이 `recommended_command` 를 반환, (3) SKILL 이 실행하는 command 가 `recommended_command` 와 byte-identical — 세 조건이 모두 참일 때에만 예외로 허용해요.

이 3조건은 텍스트 NEVER 룰이라 기계 검증이 없어요. 이 ADR 은 invariant 5를 machine-enforceable HMAC nonce 로 격상하는 설계를 결정해요.

## Decision

`axhub-helpers session-start` 가 세션 시작 시 `AXHUB_INTERACTIVE_TOKEN=<HMAC-SHA256(session_id ‖ secret)>` env var 를 mint 해요. SKILL 이 inline `!` 명령을 실행하기 직전 `axhub-helpers verify-interactive-token` 을 호출하고, nonce 검증이 통과할 때만 install 이 진행돼요.

현재 `session-start` 구현 (`crates/axhub-helpers/src/main.rs:83-92`) 은 systemMessage 만 출력해요.

```
"session-start" => {
    println!("{}", json!({"systemMessage":"axhub helper Rust runtime이에요."}));
    let mut m = Map::new();
    m.insert("event".into(), Value::String("session_start".into()));
    emit_meta_envelope(m).ok();
    Ok(0)
}
```

`cmd_preauth_check` 의 HMAC 패턴 (`crates/axhub-helpers/src/main.rs:382-461`) 은 `ConsentBinding` + `verify_or_claim_token` 조합으로 이미 machine-enforceable gate 를 구현했어요. nonce 설계는 동일 패턴을 재사용해요.

### nonce 생명주기

```
session-start
  └─ mint  AXHUB_INTERACTIVE_TOKEN = HMAC-SHA256(session_id ‖ AXHUB_NONCE_SECRET)
           export → Claude Code env

per-prompt hook (PreToolUse / 또는 SKILL preflight)
  └─ verify AXHUB_INTERACTIVE_TOKEN
       ├─ valid   → allow, per-prompt rotate 옵션
       └─ invalid → deny + systemMessage "replay or stale nonce"

subprocess (CI / claude -p)
  └─ AXHUB_INTERACTIVE_TOKEN 상속 시 session_id 불일치 → deny
```

### 구현 범위

- `crates/axhub-helpers/src/main.rs`: `session-start` 에 nonce mint 추가 (~30 line)
- `crates/axhub-helpers/src/main.rs`: `verify-interactive-token` 신규 subcommand (~50 line)
- `cmd_preauth_check` HMAC 재사용: `verify_or_claim_token` 대신 경량 `verify_nonce` 함수 추출
- SKILL preflight: `!axhub-helpers verify-interactive-token` 1줄 추가 (init / deploy / recover)

총 ~80 line Rust 추가, 기존 ConsentBinding 인프라 재사용이에요.

## Drivers

1. **텍스트 의존성 제거**: PR #41 NEVER 룰 3조건은 SKILL author 의 자유 해석 영역에 있어요. byte-identical 검사가 우회될 수 있어요.
2. **subprocess replay 차단**: CI / `claude -p` 는 부모 세션 env var 를 그대로 상속해요. stale nonce 를 기계적으로 검출해야 해요.
3. **per-prompt rotate 옵션**: session-scoped nonce 로 시작하고 per-prompt rotate 를 추가하면 replay window 를 prompt 단위로 줄일 수 있어요.
4. **기존 패턴 일관성**: `cmd_preauth_check` 의 HMAC + `ConsentBinding` 표준과 동일 인프라를 써요. 새 보안 surface 를 추가하지 않아요.

## Alternatives

### A. 텍스트 NEVER 룰만 유지 (현재)

- 장점: 구현 비용 0
- 단점: machine-enforceable 아님. SKILL author 실수 또는 악의적 우회에 무력해요. subprocess replay 차단 없음.

### B. registry safe_default advisory

- 장점: 기존 인프라 확장
- 단점: advisory 에 그침. gate 를 bypass 하는 코드 경로를 막지 못해요.

### C. 본 ADR — HMAC nonce (선택)

- 장점: machine-enforceable, subprocess replay 차단, 기존 HMAC 인프라 재사용, ~80 line 추가
- 단점: helper SessionStart 갱신 + per-prompt hook 추가 필요. AXHUB_NONCE_SECRET 키 관리 추가

## Consequences

### 긍정

- invariant 5 (`recommended_command` byte-identical) 가 machine-enforceable 로 격상돼요.
- subprocess 환경에서 stale nonce 가 기계적으로 거부돼요.
- 기존 ConsentBinding / HMAC 표준과 일관성을 유지해요.

### 절충 (tradeoff)

- `session-start` 를 갱신하고 per-prompt hook 을 추가해야 해요. (~80 line Rust)
- `AXHUB_NONCE_SECRET` 환경 변수 또는 keychain 항목을 새로 관리해야 해요.
- SKILL preflight 에 `!axhub-helpers verify-interactive-token` 1줄이 추가돼요 (init / deploy / recover).

### 중립

- 기존 `ConsentBinding` / `verify_or_claim_token` 표준과 구조가 같아서 보안 리뷰 범위가 늘지 않아요.
- nonce rotate 정책 (session-scoped vs per-prompt) 은 PoC 결과로 결정해요.

## Trigger

PR #41 Follow-up #2. `AXHUB_INTERACTIVE_TOKEN nonce ADR-2 (helper trust boundary 충분성 검증 후)` 항목이에요.

## Follow-ups

- **PoC 구현** (별도 PR): `session-start` nonce mint + `verify-interactive-token` subcommand + SKILL preflight 1줄. cargo-fuzz 24h 포함.
- **per-prompt rotate 평가**: PoC 운영 후 replay 데이터 수집, rotate 필요 여부 결정.
- **promote 결정**: PoC 검증 후 이 ADR 을 Accepted 로 갱신하고 invariant 목록 공식 업데이트.
- **AXHUB_NONCE_SECRET 키 관리**: keychain 저장 vs 세션 도출 중 하나를 PoC 단계에서 선택.
