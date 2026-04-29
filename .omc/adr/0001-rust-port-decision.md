# ADR 0001 — axhub-helpers Rust 포팅 결정

## Status

Accepted on 2026-04-29.

## Context

axhub-helpers 는 Bun single binary 로 배포되고 있어요. `.plan` 의 Rust 포팅 계획은 binary 크기, cold start, 메모리 baseline 개선을 목표로 해요. `/autoplan` review 는 validation sprint 를 권장했지만, 사용자 결정은 full Rust port 진행이에요.

## Decision

점진 포팅으로 진행해요. TypeScript helper 는 Phase 4 전까지 유지하고, Rust workspace 를 `crates/` 아래 추가해요. 전환 기간에는 `AXHUB_HELPERS_RUNTIME=ts|rust|auto` 로 즉시 우회할 수 있게 해요.

## Consequences

- Phase 0/1 부터 Rust tests 와 TypeScript tests 를 함께 유지해요.
- 보안 surface (`consent`, `list-deployments`, `keychain`) 는 회귀 테스트를 먼저 작성해요.
- Windows EDR, cargo-fuzz 24h, 3 OS keyring live cohort 는 현재 macOS 세션에서 완전 검증할 수 없어서 별도 QA ledger 로 남겨요.
