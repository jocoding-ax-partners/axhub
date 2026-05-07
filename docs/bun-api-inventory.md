# Bun API inventory for Rust helper port

Phase 0 에서 예전 TS helper 의 Bun 전용 API 를 Rust 매핑으로 잠갔어요. 현재 사용자 경로는 `crates/axhub-helpers` Rust helper 예요.

| 예전 TS 위치 | Bun API | Rust 매핑 | 현재 상태 |
| --- | --- | --- | --- |
| `index.ts` | `Bun.stdin.text()` | `std::io::stdin().read_to_string()` | Rust `main.rs` 가 맡아요 |
| `preflight.ts` | `Bun.spawnSync` | `spawn_sync_with_timeout` / runner wrapper | `spawn.rs` 가 맡아요 |
| `keychain.ts` | `Bun.spawnSync` | `spawn_sync_with_timeout` + platform runner | Rust keychain runner 가 맡아요 |
| `keychain-windows.ts` | `Bun.spawnSync` | `spawn_sync_with_timeout` + PowerShell runner | Rust Windows runner 가 맡아요 |

검증은 `cargo test -p axhub-helpers` 와 `bun test` 로 해요.
