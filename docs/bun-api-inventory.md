# Bun API inventory for Rust helper port

Phase 0 에서 TypeScript helper 의 Bun 전용 API 를 Rust 매핑으로 잠가요.

| TS 위치 | Bun API | Rust 매핑 | 상태 |
| --- | --- | --- | --- |
| `src/axhub-helpers/index.ts` | `Bun.stdin.text()` | `std::io::stdin().read_to_string()` | Rust `main.rs` 구현됨 |
| `src/axhub-helpers/preflight.ts` | `Bun.spawnSync` | `std::process::Command::output()` | `spawn.rs` 구현됨 |
| `src/axhub-helpers/keychain.ts` | `Bun.spawnSync` | `spawn.rs` + platform runner | Rust keychain runner 구현됨 |
| `src/axhub-helpers/keychain-windows.ts` | `Bun.spawnSync` | `spawn.rs` + PowerShell runner | Rust Windows runner 구현됨 |

검증은 `cargo test -p axhub-helpers` 와 `bun test tests/runtime-fallback.test.ts` 로 해요.
