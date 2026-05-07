# ADR 0011 — Bootstrap sealed trait: PlanOnly / Executable

## Status

Proposed — PoC 구현 대기 중.

## Context

`crates/axhub-helpers/src/bootstrap.rs`의 `BootstrapState`는 13개 variant를 가져요.

```rust
// bootstrap.rs:22-36
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapState {
    TemplateRequired,
    ConflictExistingFiles,
    GitInitRequired,
    FirstCommitRequired,
    SubdomainCollision,
    AlreadyDeployed,
    ConsentRequiredAppsCreate,
    ConsentRequiredDeployCreate,
    BackendContractMissingDefaults,
    IdempotencyUnavailable,
    AppRegistered,
    Deploying,
    Deployed,
}
```

PR #41(HI3)은 `Planning` / `Verifying` 단계에서 install 실행 함수가 호출되면 안 된다는 불변식을 **런타임 match**로 강제해요.

```rust
// bootstrap.rs:440-448
fn state_is_terminal_stop(state: BootstrapState) -> bool {
    matches!(
        state,
        BootstrapState::SubdomainCollision
            | BootstrapState::AlreadyDeployed
            | BootstrapState::BackendContractMissingDefaults
            | BootstrapState::IdempotencyUnavailable
    )
}
```

이 guard는 14 variant × 2 함수 매트릭스 테스트(R3 mitigation)로 덮여 있는데, 테스트가 검증하는 것을 컴파일러가 이미 증명할 수 있어요. 런타임 guard는 새 variant 추가 시 누락 가능성이 있고, 매트릭스 테스트는 over-engineered 상태예요.

이 상황은 4번째 hard invariant 후보예요:

| # | Invariant | 현재 enforcement |
|---|-----------|-----------------|
| 1 | Consent HMAC 검증 | Rust type system + parser test |
| 2 | Idempotency key 고정 | Serialize lock test |
| 3 | Terminal stop 분기 | runtime match (HI3, PR #41) |
| **4** | **PlanOnly 단계 install 금지** | **런타임 guard (개선 대상)** |

## Decision

`BootstrapState`에 sealed trait `PlanOnly` / `Executable`을 도입해요.

```rust
mod sealed {
    pub trait Seal {}
}

/// Planning 단계에서만 유효한 state marker.
pub trait PlanOnly: sealed::Seal {}

/// Install 실행이 허용된 state marker.
pub trait Executable: sealed::Seal {}
```

`Planning(DependencyPlan)` / `Verifying(DependencyVerify)` variant는 `PlanOnly`만 impl해요. install 실행 함수 signature에 `<T: Executable>` constraint를 추가하면, 이 두 variant로 호출 시 컴파일 오류가 발생해요.

```rust
// 변경 전
pub fn execute_install(state: BootstrapState) -> BootstrapRun { ... }

// 변경 후
pub fn execute_install<S: Executable>(state: S) -> BootstrapRun { ... }
```

13개 기존 variant는 모두 `Executable`을 impl하고, `PlanOnly`는 신규 추가 `Planning` / `Verifying`에만 부여해요. `sealed` 모듈은 crate 외부에서 trait impl을 막아요.

## Alternatives Considered

### 1. Status Quo (runtime match 유지)

PR #41 HI3 guard를 그대로 유지해요. 새 variant 추가 시 `state_is_terminal_stop` match에 누락되면 런타임에서야 panic이 발생할 수 있어요. 매트릭스 테스트가 이를 잡지만, 테스트가 컴파일러를 대신하는 구조예요.

**기각**: 컴파일러가 증명 가능한 불변식을 테스트로 우회하는 것은 불필요한 complexity예요.

### 2. Newtype wrapper per variant

`Planning(DependencyPlan)`을 별도 newtype으로 감싸서 `execute_install` 인자 타입을 교체해요.

**기각**: variant 수 × wrapper 수만큼 boilerplate가 증가하고, enum 자체의 exhaustive match 이점을 잃어요.

### 3. 본 ADR — Sealed trait (채택)

sealed trait은 Rust에서 관용적인 패턴이에요. 외부 crate impl 금지 + 컴파일타임 constraint + 기존 enum 구조 유지가 모두 가능해요.

## Consequences

### 긍정

- **컴파일타임 불변식**: `Planning` / `Verifying` 단계 install 호출이 컴파일 오류로 차단돼요.
- **Runtime panic 0**: `state_is_terminal_stop` guard 및 해당 panic path가 사라져요.
- **새 variant 안전성**: 신규 variant 추가 시 `PlanOnly` 또는 `Executable` 중 하나를 명시해야 하므로, 누락이 컴파일 오류로 즉시 드러나요.

### Tradeoff

- **~150 line Rust 리팩터**: 13개 기존 variant × trait impl + `execute_install` 류 caller signature 변경 + `sealed` module 추가가 필요해요.
- **Generic 함수 증가**: `execute_install<S: Executable>` 형태로 signature가 복잡해질 수 있어요. 호출부 타입 추론이 실패하는 경우 turbofish가 필요해요.

### 중립

- PR #41 R3 mitigation의 14 variant × 2 함수 매트릭스 테스트 일부를 deprecate할 수 있어요. 컴파일러가 이미 같은 보장을 제공하기 때문이에요.
- `BootstrapState` 자체의 `is_user_decision()` 런타임 메서드는 별도 guard이므로 이 ADR 범위 밖이에요.

## Follow-ups

1. **PoC implementation** — 별도 PR. `sealed` 모듈 + 13 variant trait impl + caller 수정 + 기존 매트릭스 테스트 정리.
2. **Production incident 데이터 수집** — install 명령이 plan 단계 외에서 호출된 사례 데이터를 수집한 뒤 promote 여부를 결정해요.
3. **`state_is_terminal_stop` 제거** — PoC merge 후 해당 함수와 연관 테스트를 정리해요.

## References

- PR #41: BootstrapState 14 variants + HI3 runtime guard 도입
- `crates/axhub-helpers/src/bootstrap.rs:22-36` — `BootstrapState` enum 정의
- `crates/axhub-helpers/src/bootstrap.rs:440-448` — `state_is_terminal_stop` 구현
- [Rust sealed trait pattern](https://rust-lang.github.io/api-guidelines/future-proofing.html#sealed-traits-protect-against-downstream-implementations-c-sealed)
