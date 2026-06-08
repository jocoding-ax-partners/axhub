# 옵션3 — bun hook 의존 제거 + cross-platform native dispatch

> **상태**: PLAN-UPDATED (plan-ceo-review SCOPE EXPANSION, 구조 A dependency-gated, 2026-06-08 실행 전 보강 반영)
> **모드**: SCOPE EXPANSION — 사용자가 C+E1+E2+E3 전부 opt-in
> **브랜치**: 새 브랜치 권장 (현 `fix/hide-route-hint-from-user` 는 무관한 dead-code 정리 중)

## 배경 — 원래 문제

매 Bash tool 호출마다 배너:

```
PostToolUse:Bash hook error
Failed with non-blocking status code: /usr/bin/bash: line 1: bun: command not found
```

### 근본 원인

`hooks/hooks.json` 의 PostToolUse[matcher=Bash] 3개 hook 중 1개만 **bare `bun`** 호출:

```json
{ "command": "bun ${CLAUDE_PLUGIN_ROOT}/hooks/post-tool-verify-deploy-artifacts.ts", "timeout": 7 }
```

나머지 hook 은 전부 `bash ... axhub-helpers.sh <sub>` → compiled Rust binary. 이 bun hook 만 아키텍처 일탈.

- **PATH 분리**: Bash tool 은 shell snapshot(`~/.zshrc` source) 으로 `~/.bun/bin` 포함 → `which bun` 성공. hook 은 Claude 프로세스 raw PATH(`/usr/bin/bash`, `~/.bun/bin` 없음) → `bun: command not found`.
- **end-user 영향**: `install.sh`/`install.ps1` 에 bun 설치 0 → vibe-coder 사용자는 Mac·Windows 둘 다 bun 없음. matcher=`Bash` 라 **모든 Bash 호출마다 배너** = product bug (cosmetic, non-blocking).
- **함정**: `.ts` 내부 fail-open `exit 0` + `AXHUB_DISABLE_HOOKS` kill-switch 가 있지만, **bun 자체를 못 찾으면 .ts 가 실행 안 됨** → dispatch 레벨이 깨진 거라 kill-switch 도 무력. `AXHUB_DISABLE_HOOKS=1` 으로 못 막음.

## 결정 요약

| 항목 | 결정 |
|---|---|
| 핵심 방향 | bun 의존 제거 → verify 로직을 `axhub-helpers` Rust subcommand 로 흡수 (전 hook 과 동일 패턴) |
| C | post-tool verify 의 Rust 흡수 + resolver Windows case + ps1 배선 |
| E1 | 전 hook `.ps1` parity (Phase 10 를 SessionStart→전 hook 확장) |
| E2 | resolver 단일소스화 (.sh/.ps1 candidate drift 차단) |
| E3 | deploy verify deepening (network/health/digest) |
| 구조 | **A — dependency-gated.** PR1(bun 제거)은 독립 land. ps1/deep 은 gated. |

### 왜 dependency-gated 인가 (현실 제약)

조사로 드러난 두 gate:

1. **Q15 (open question, 미해결)** — `.omc/plans/open-questions.md:156`: `"shell":"powershell"` 이 macOS/Linux 에서 no-op 인지 매 세션 에러 팝업인지 미검증. 그래서 ps1 파일(`session-start.ps1`, `install.ps1` 등)은 이미 있지만 **hooks.json 에 `"shell":"powershell"` 배선은 0개** (의도적 보류). C/E1 의 hooks.json ps1 배선은 이 spike 에 gated.
2. **backend schema 미lock** — `bootstrap.rs:955` 가 deploy create stdout 을 `["id","deployment_id","deploy_id"]` 다중 키 fallback 으로 파싱 = response 미안정. E3 의 network probe/health/digest 는 status endpoint schema 필요 → schema lock 에 gated.

→ bun 제거(bash dispatch)는 두 gate 와 무관하므로 **즉시 ship**. ps1·deep 은 gate 풀린 뒤.

## PR 로드맵 (의존 그래프)

```
PR1 (bun 제거 + resolver Windows case + E2 1차)  ──┐ 독립, 즉시
                                                    │
PR2 (Q15/US-1000 spike: macOS wrong-OS spawn 검증) ─┤ blocking spike
                                                    ▼
PR3 (C ps1 배선 + E1 전 hook ps1 + E2 단일소스 완성) ── Q15 통과 시
                                                    │
PR4 (E3 deep verify)  ←── backend schema lock 선행  ─┘ 독립 gate
```

PR1 이 원래 문제(배너)를 해결. PR3·PR4 가 stall 해도 PR1 은 land.

---

## PR1 — bun 제거 (즉시, 블로커 0)

### 목표
verify 로직을 Rust 로 흡수하고 hooks.json 을 bash dispatch 로 전환. Mac 배너 즉시 해소, Windows 는 Git Bash 경유(다른 hook 과 동일), bun 의존 0, dispatch fail-open + kill-switch 실작동.

### 작업

1. **신규 `crates/axhub-helpers/src/verify_deploy_artifact.rs`** — `scripts/verify-user-app-artifact.ts`(114줄, 외부 import 0) 포팅:
   - `pub fn verify_user_app_artifact(deploy_stdout: &str) -> VerifyResult { passed: bool, violations: Vec<String> }`
   - `parse_deploy_response(stdout) -> Option<Value>` (best-effort; non-JSON 은 skip)
   - **현재 TS behavior parity 우선**: `manifest_hash` 는 현 `/^[a-f0-9]{64}$/i` 처럼 대소문자 허용 / `state` 는 live·running·deployed·active·ok·succeeded·success / `url` 은 `^https?://` case-insensitive / id 는 `deployment_id` → `deploy_id` → `id` 순서로 첫 present key 만 non-empty string 검사
   - uppercase hash, `success` state, `deploy_id` key 를 regression test 로 잠금 (플랜 초안의 “lowercase only”·`success` 누락·`deploy_id` 누락은 **의도 아님**, TS parity 가 authoritative)
   - **code-share-zero with `release_check`** (도메인 분리 — 주석 보존)

2. **clap 배선** — `cli/args/mod.rs` + `cli/mod.rs` 에 `VerifyDeployArtifact` variant (classify-exit 선례).
   - `cli/mod.rs::classify()` 의 `FailOpenHook` 목록에도 `verify-deploy-artifact` 추가. 누락 시 unknown/bad flag parse error 가 stderr+64 로 새 hook 배너를 만들 수 있음.
   - `USAGE` 문자열, `tests/manifest.test.ts` `knownSubcommands`, hook baseline 도 같이 갱신.

3. **main.rs dispatch arm** (classify-exit/test-classifier PostToolUse stdin 패턴 재사용):
   - 진입부 `hook_safety::is_hook_disabled("post-tool-verify-deploy-artifacts")` → true 면 즉시 종료
   - stdin read → `serde_json` parse (실패 시 fail-open exit 0; unexpected error 는 `hook_safety::append_hook_error("post-tool-verify-deploy-artifacts", err)` 후 exit 0)
   - `tool_input.command` 가 `^\s*axhub\s+deploy\s+create\b` (regex) 아니면 종료
   - `tool_response.exit_code == 0` && `stdout` non-empty 아니면 종료
   - `verify_user_app_artifact(stdout)` → violations 있으면 `{systemMessage, hookSpecificOutput.additionalContext}` 출력 (기존 .ts 메시지 문구·additionalContext 블록 보존)
   - `hook_output.rs` 에 `post_tool_use_context_with_system_message(text, system_message)` 같은 helper 를 추가하거나, dispatch arm 에서 `json!` 직접 구성. 현재 `hook_output.rs` 에는 이 조합 helper 가 없으므로 “기존 helper 호출”로 착각 금지.

4. **hooks.json 배선 전환**:
   ```json
   // 변경 전
   { "command": "bun ${CLAUDE_PLUGIN_ROOT}/hooks/post-tool-verify-deploy-artifacts.ts", "timeout": 7 }
   // 변경 후
   { "command": "bash ${CLAUDE_PLUGIN_ROOT}/hooks/axhub-helpers.sh verify-deploy-artifact", "timeout": 7 }
   ```

5. **resolver Windows case (E2 1차)** — `hooks/axhub-helpers.sh` `resolve_helper` 에 추가:
   ```sh
   MINGW*:*|MSYS*:*|CYGWIN*:*) suffix="windows-amd64" ;;   # uname -s 가 MINGW64_NT 등
   ```
   그리고 candidate 에 `.exe` 변형 추가 (`${ROOT}/bin/axhub-helpers.exe`, `${ROOT}/bin/axhub-helpers-${suffix}.exe`, `${ROOT}/target/release/axhub-helpers.exe`, `${ROOT}/target/debug/axhub-helpers.exe`). 현재 `Darwin|Linux` 만 → release.yml 의 `axhub-helpers-windows-amd64.exe` 를 Git Bash 환경에서 탐지 가능. source checkout Windows 검증도 target `.exe` 후보가 있어야 함. **기존 버그, 전 hook 영향.** (PR3 에서 이 case 를 `runtime_paths` 단일소스로 흡수 — 잠깐 중복, 알려진 revisit.)

5b. **fail-open allowlist (필수 — 누락 시 bun 버그 재현)** — `axhub-helpers.sh` 의 binary-미탐지 fallback `case` arm 에 `verify-deploy-artifact` 추가:
   ```sh
   prompt-route|karpathy-inject|classify-exit|test-classifier|state-update|commit-gate|tdd-inject|verify-deploy-artifact)
     cat >/dev/null || true
     exit 0 ;;
   ```
   안 넣으면 binary 미탐지 시 `*)` → `exit 127` + stderr = **새 배너** (source checkout / `bin/` 미빌드 = maintainer 환경에서 bun 버그 그대로 재현 — dispatch-level non-fail-open, 진단했던 그 결함 부류). **주의**: 이 arm 은 `$1` = subcommand string(`verify-deploy-artifact`) 매칭이지 kill-switch hook name(`post-tool-verify-deploy-artifacts`) 아님 — 둘 다 쓰니 혼동 주의.

6. **삭제 (orphan) + 참조 정리**: `hooks/post-tool-verify-deploy-artifacts.ts`, `scripts/verify-user-app-artifact.ts`. `hooks/_helpers.ts` 는 다른 참조 없으면 삭제 (grep 확인).
   - 함께 갱신/삭제할 known refs: `package.json` `verify:user-app-artifact`, `scripts/lint-hook-inject-shape.ts`, `tests/lock-system-message-korean.test.ts`, `tests/post-tool-verify-deploy-artifacts.test.ts`, `tests/manifest.test.ts`, `docs/architecture.ko.md`, `README.html`, `axhub-features-catalog.html`, `docs/HOOKS.md`.
   - `README.html`/`docs/architecture.ko.md` 의 “TS hook 용 kill-switch 헬퍼” 서술은 제거 또는 Rust-only 서술로 교체. repo tooling 으로서의 Bun 언급은 유지.

7. **테스트**: 기존 TS verify 테스트 → `cargo test` 단위/계약 테스트로 이전 (sanity 4종 + nil/empty/non-JSON shadow path + TS parity edge: uppercase hash / `success` / `deploy_id`). `tests/hooks-additional-context-shape.test.ts`·`tests/ux-autowire-hooks.test.ts`·`tests/manifest.test.ts`·`tests/lock-system-message-korean.test.ts` 영향 갱신.

8. **docs/HOOKS.md** §1 표에 post-tool hook 행을 `axhub-helpers verify-deploy-artifact` 로 추가/갱신.

### 검증
- **broken-PATH acceptance (bun 배너 노출했던 그 테스트)**: helper binary 를 일부러 못 찾게 해야 하므로 `CLAUDE_PLUGIN_ROOT` 를 빈 temp dir 로 고정:
  ```bash
  tmp="$(mktemp -d)"
  env -i PATH=/bin:/usr/bin CLAUDE_PLUGIN_ROOT="$tmp" bash hooks/axhub-helpers.sh verify-deploy-artifact </dev/null
  echo $?
  ```
  → `0`, stdout/stderr empty (binary 미탐지 fail-open 실증). 단순 `env -i bash hooks/axhub-helpers.sh ...` 는 repo 의 `target/debug/axhub-helpers` 를 찾아 실행할 수 있어 fallback 검증이 아님.
- `which bun` 없는 환경(`env -i bash -c '...'`)에서 hook 무에러
- `cargo test -p axhub-helpers --test verify_deploy_artifact_test` green
- `bun test` 회귀 0 fail, `bunx tsc --noEmit` clean, `bun run lint:hook-inject` green

---

## PR2 — Q15/US-1000 spike (blocking, ps1 배선 전제)

ps1 hooks.json 배선의 전제. Phase 10 v2 의 US-1000 그대로 실행:
- 최소 stub plugin 으로 macOS·Linux 에서 `"shell":"powershell"` sibling 의 동작 관찰 (silent / visible-error / popup)
- 결과 → `docs/dev/ps-wrong-os-spawn-<date>.md`, open-questions.md Q15 close
- visible-error 면: 각 .ps1 첫 줄에 `if ($PSVersionTable.Platform -ne 'Win32NT' -and $PSVersionTable.PSEdition -ne 'Desktop') { exit 0 }` 가드

popup 이면 PR3 BLOCKED → Anthropic 에스컬레이션.

---

## PR3 — ps1 배선 + E1 전 hook parity (Q15 통과 시)

> **Phase 10 v2 plan (`.omc/plans/phase-10-windows-ps1-hooks-v2.md`) 을 supersede 하지 않고 상속.** Option B(`"shell":"powershell"` sibling) / floor Claude Code 2.1.84 / 4-part Korean `systemMessage` / 모든 catch `exit 0` / 7 pre-mortem 패턴을 그대로 따름. Phase 10 가 SessionStart 만 다뤘으므로, PR3 은 그 패턴을 **나머지 hook 에 확장**.

작업:
1. **post-tool `.ps1`** (C 완성) — PR1 의 Rust subcommand 를 powershell 에서도 호출 (`& "$env:CLAUDE_PLUGIN_ROOT/hooks/axhub-helpers.ps1" verify-deploy-artifact` 또는 hooks.json 에 powershell sibling 직접)
2. **전 hook `.ps1` shim (E1)**: prompt-route, karpathy-inject, commit-gate, tdd-inject, classify-exit, test-classifier, state-update — 각 `axhub-helpers.ps1 <sub>` 로 dispatch (session-start.ps1 패턴)
3. **hooks.json**: 각 hook 에 `"shell":"powershell"` sibling 추가 (US-1003 확장). SessionStart 도 이때 ps1 sibling 배선 (현재 ps1 파일 있으나 미배선).
4. **E2 단일소스 완성**: `axhub-helpers.sh` + 신규 `axhub-helpers.ps1` 의 candidate 로직을 `axhub-helpers --resolve-helper` (Rust `runtime_paths`) 단일 출력에 위임 → bash/ps1 중복 0, drift 차단.
5. **Windows CI 매트릭스**: `routing-drift.yml` 의 windows-tooling-check 를 확장해 .ps1 dispatch 검증 (file-text assertion, Phase 10 US-1004 패턴).

---

## PR4 — E3 deep verify (backend schema lock 시)

> **dependency-gated stub.** backend deploy/status response schema 가 lock 되기 전엔 spec 불가 (네트워크 probe/digest 를 미안정 schema 에 맞추면 보장된 rework). 현재는 blocker 명시 stub 으로만 plan 에 존재.

schema lock 후:
- `verify-deploy-artifact --deep`: status endpoint GET, user-app health probe, 호출 간 manifest digest 비교
- 새 실패모드(timeout/flaky) → fail-open 유지, 별도 timeout/retry
- `verify-user-app-artifact.ts` 주석의 deferred 항목(`verify-...-deep`) 실현

---

## Architecture — dispatch before/after

```
BEFORE (혼재 3계층)                      AFTER (PR1)                  AFTER (PR3)
─────────────────────                   ──────────────              ──────────────
session-start → bash .sh ─┐             동일                         bash .sh + ps1 sibling
prompt-route  → bash .sh ─┤→ Rust bin   동일                         + ps1 sibling
classify-exit → bash .sh ─┤  (Git Bash  동일                         + ps1 sibling
post-tool     → bun .ts  ─┘   on Win)   bash .sh → Rust bin          + ps1 sibling
                              ▲          (일탈 제거)                   (Win native, Git Bash 불필요)
                          bun 일탈
```

## Error & Rescue map (PR1)

| codepath | 실패 | 처리 | 사용자 노출 |
|---|---|---|---|
| stdin read | EOF/IO 실패 | exit 0 (fail-open) | 없음 |
| JSON parse | invalid envelope | exit 0 | 없음 |
| helper 미탐지 (resolver) | binary 없음 | `axhub-helpers.sh` 기존 fail-open 분기 (verify-deploy-artifact 는 context hook → exit 0) | 없음 |
| verify_user_app_artifact | violations 발견 | systemMessage + additionalContext | "⚠️ 배포 artifact 검증 의심 신호…" |
| kill-switch | `AXHUB_DISABLE_HOOK=...` | 진입부 종료 | 없음 |

**개선점**: 현재는 bun 미탐지가 dispatch 레벨 hard error(배너). PR1 후엔 `axhub-helpers.sh` 의 fail-open 분기에 `verify-deploy-artifact` 를 context-hook 으로 등록 → binary 없어도 exit 0.

## Failure modes registry

| codepath | failure | rescued | test | user sees | logged |
|---|---|---|---|---|---|
| verify dispatch (bun) — 현재 | bun not found | N ← GAP | N | **배너(매번)** | N |
| verify dispatch (bash) — PR1 | binary not found | Y | Y | 없음 | hook_safety::append_hook_error |
| verify logic | malformed stdout | Y (best-effort skip) | Y | 없음 | — |
| ps1 sibling — Q15 | macOS popup | ? (PR2 spike) | spike | **팝업 위험** | breadcrumb |

## Test plan (eng-review §3 — C: unit + 회귀 + 계약통합 + E2E)

```
verify_user_app_artifact()   [unit]            sanity 4종(manifest_hash sha256 / state enum / url http / id non-empty)
                                               + TS parity edge(uppercase hash / success / deploy_id)
                                               + shadow(nil / empty / non-JSON / malformed)
dispatch arm                 [계약통합 fixture]  kill-switch on→return / non-deploy→skip / exit≠0→skip / empty→skip / violations→systemMessage shape
resolver                     [unit + 회귀]      Windows .exe 탐지(bin + target debug/release) / binary-missing broken-PATH fail-open(exit 0) / bun→bash shape 회귀
E2E                          [manual/gated]     axhub deploy create → hook 발화 → systemMessage
```

- **REGRESSION (critical)**: bun→bash 전환이 `manifest`·`ux-autowire-hooks`·`hooks-additional-context-shape` shape 테스트 깨뜨림 → 갱신 필수 (T5c)
- **broken-PATH acceptance**: `env -i PATH=/bin:/usr/bin CLAUDE_PLUGIN_ROOT="$(mktemp -d)" bash hooks/axhub-helpers.sh verify-deploy-artifact </dev/null; echo $?` → `0` + stdout/stderr empty (T4b)
- **kill-switch 매트릭스**: hooks-kill-switch.test.ts 에 verify case (T5b, CLAUDE.md MUST)
- **Korean lock**: `tests/lock-system-message-korean.test.ts` 는 삭제가 아니라 Rust source 또는 fixture stdout 에서 같은 문구를 잠그도록 이전
- **lint-hook-inject**: `scripts/lint-hook-inject-shape.ts` 의 파일 목록에서 삭제된 TS hook 을 Rust source/fixture 로 이전
- **E2E**: CI flaky 회피 위해 manual/release gate (Phase 10 VM-smoke 선례, T5e)

## NOT in scope

- bun 을 PATH 에 넣는 우회(symlink/터미널 실행) — 임시방편, end-user 에 전파 안 됨
- axhub CLI 자체의 bun 의존 (있다면) — 이 plan 은 hook 한정
- E3 deep verify 의 즉시 구현 — schema lock 대기 (PR4 stub)

## What already exists (재사용)

- `hook_output.rs`, `cli_envelope.rs` — hook JSON 출력/envelope
- `hook_safety.rs` — kill-switch + append_hook_error
- `cli/mod.rs` + `cli/args/mod.rs` — clap typed Commands (classify-exit 선례)
- `runtime_paths.rs` — resolver 단일소스 후보 (E2)
- `session-start.ps1`·`install.ps1`·`statusline.ps1` — ps1 패턴 레퍼런스 (E1)
- Phase 10 v2 plan — ps1 sibling 배선 설계 (PR3 상속)
- `routing-drift.yml` windows-tooling-check — Windows CI lane (PR3)

## Open questions / dependencies

- **Q15** (PR2 spike): `"shell":"powershell"` macOS/Linux 동작 — PR3 전제
- **backend schema lock**: E3 전제 (PR4)
- `_helpers.ts` 다른 참조 여부 (PR1 삭제 전 grep)
- `verify-user-app-artifact.ts` 의 정확한 sanity 로직 — 포팅 시 라인 단위 대조

## Implementation Tasks

- [ ] **T1 (P1)** — verify_deploy_artifact.rs 포팅 (`scripts/verify-user-app-artifact.ts` → Rust, sanity 4종 + TS parity edge uppercase hash/`success`/`deploy_id` + shadow path). Verify: `cargo test -p axhub-helpers --test verify_deploy_artifact_test`
- [ ] **T1b (P1, eng-review §1)** — PR1 은 verify 가 stdin 을 inline 파싱(`cli_envelope.rs` `string_at_any` 재사용). **IF** 구현 중 classify-exit/test-classifier 가 동일 envelope 공유로 확인되면(conf 6 premise 검증) → `parse_post_tool_payload(stdin) -> {command, exit_code, stdout}` 공유 파서로 추출 + 3 caller 동시 마이그레이션. **ELSE** inline 유지 — single-use 추상화 금지(CLAUDE.md). Verify: `cargo test`
- [ ] **T2 (P1)** — clap `VerifyDeployArtifact` variant + `USAGE` + `cli/mod.rs::classify()` FailOpenHook 목록 + main.rs dispatch (T1b 공유 파서 → regex → verify → JSON output/helper, 진입부 `is_hook_disabled`, 실패 path `append_hook_error` per CLAUDE.md). Verify: `cargo test`
- [ ] **T3 (P1)** — hooks.json `bun`→`bash axhub-helpers.sh verify-deploy-artifact`. Verify: `tests/manifest.test.ts`
- [ ] **T4 (P1)** — resolver Windows case + `.exe` candidate (`axhub-helpers.sh`: bin suffixed/unsuffixed + target debug/release). Verify: shellcheck + Git Bash 탐지 수동. (PR3 가 runtime_paths 단일소스로 흡수 예정)
- [ ] **T4b (P1, 누락 금지)** — `axhub-helpers.sh` fail-open case arm 에 `verify-deploy-artifact` 추가 (`$1` subcommand 매칭, hook name 아님). binary 미탐지 시 exit 0 보장 — 안 하면 bun 버그 재현. Verify: `env -i PATH=/bin:/usr/bin CLAUDE_PLUGIN_ROOT="$(mktemp -d)" bash hooks/axhub-helpers.sh verify-deploy-artifact </dev/null; echo $?` → `0`, stdout/stderr empty
- [ ] **T5 (P2)** — orphan 삭제 (post-tool .ts, verify .ts, _helpers.ts if unused) + docs/HOOKS.md/architecture/README.html/features catalog/package script 참조 정리. Verify: `bun test`, `bunx tsc --noEmit`
- [ ] **T5b (P1, eng-review §2/CLAUDE.md)** — `tests/hooks-kill-switch.test.ts` 매트릭스에 `verify-deploy-artifact` case 추가 (CLAUDE.md MUST). Verify: `bun test tests/hooks-kill-switch.test.ts`
- [ ] **T5c (P1, eng-review §3/REGRESSION)** — hooks.json `bun`→`bash` 전환 회귀: `tests/manifest.test.ts`·`tests/ux-autowire-hooks.test.ts`·`tests/hooks-additional-context-shape.test.ts` (hooks.json shape 검증) 갱신. Verify: `bun test`
- [ ] **T5d (P2, eng-review §3)** — hook 계약 통합테스트: PostToolUse envelope fixture → `verify-deploy-artifact` subcommand → systemMessage/additionalContext JSON shape 검증 + Korean lock + lint-hook-inject shape 이전 (backend 불필요). Verify: `cargo test` (fixture-driven), `bun run lint:hook-inject`
- [ ] **T5e (P3, eng-review §3/gated)** — 실 backend E2E (manual/release-gated, Phase 10 VM-smoke 선례 — CI flaky 회피). `axhub deploy create` → hook 발화 검증, 증빙을 release notes 에 첨부
- [ ] **T6 (P1, gated)** — PR2: Q15/US-1000 spike → `docs/dev/ps-wrong-os-spawn-*.md`
- [ ] **T7 (P2, gated)** — PR3: 전 hook .ps1 + hooks.json powershell sibling + E2 단일소스. Verify: routing-drift.yml windows lane
- [ ] **T8 (P3, gated)** — PR4: deep verify (schema lock 후)

## GSTACK REVIEW REPORT

| Review | Trigger | Why | Runs | Status | Findings |
|--------|---------|-----|------|--------|----------|
| CEO Review | `/plan-ceo-review` | Scope & strategy | 1 | reviewed | SCOPE EXPANSION, 4 scope opt-in (C+E1+E2+E3), 구조 A dependency-gated |
| Eng Review | `/plan-eng-review` | Architecture & tests (required) | 2 | reviewed | 1차: stdin DRY/test 깊이/kill-switch/shape. 2차(2026-06-08): TS parity edge, FailOpenHook classify, true binary-missing probe, Windows target `.exe`, 삭제 참조 범위 보강 |
| Design Review | `/plan-design-review` | UI/UX gaps | 0 | — | UI scope 없음 → skip |
| DX Review | `/plan-devex-review` | Developer experience gaps | 0 | n/a | internal hook dispatch, dev-facing surface 없음 → applicability gate exit |

- **UNRESOLVED**: 0 (D1·D2·D3·E1·E2·E3 + eng D1·D2 모두 응답)
- **CROSS-MODEL**: advisor(stronger reviewer) 4회 검토 — fail-open allowlist gap(T4b), E1 Phase10 상속, E3 schema-gate 반영
- **VERDICT**: CEO + ENG reviewed — PR1 구현 준비 완료(단, 본 문서의 2026-06-08 보강 항목까지 포함해 실행). 잔여: stdin DRY(conf 6/10)는 구현 시 classify-exit 정독으로 확정. E2E(T5e) manual-gated.
