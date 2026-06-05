# GitHub device flow 즉시 surface — 설계

> **SUPERSEDED (2026-06-05).** detach + tail wrapper 는 채택 안 됐어요. `/axhub:auth` 가 0.15.3+ 에서 fast-exit + `--resume-last` 패턴으로 진화했고 (auth Step 5b/5c), init/github 도 같은 패턴으로 통일했어요 — CLI 가 `device_code_issued` emit 직후 fast-exit (`flow.rs:440-452`, agent context), 에이전트가 challenge 를 surface 한 뒤 사용자 승인 신호를 받으면 `--resume-last` 로 token 교환을 직접 마무리해요 (사용자에게 떠넘기지 않아요). nohup/disown/tail 불필요. 아래 root-cause 분석은 여전히 유효하지만 "안 B 메커니즘" 섹션은 폐기됐어요.

- 날짜: 2026-05-25
- 범위: `skills/init/SKILL.md`, `skills/github/SKILL.md`
- 메커니즘: ~~안 B (detach + tail)~~ → fast-exit + `--resume-last` (auth Step 5b/5c 패턴 이식)

## 문제 (ax-hub-cli 소스로 확정)

`/axhub:init` 과 `/axhub:github` 의 GitHub OAuth device flow 가 시작돼도 사용자가 verification URL + user_code 를 못 받고 오래 대기해요. CLI 가 사람이 쓰는 것처럼 승인을 기다리며 block 하는데, agent 가 그 사이 아무것도 surface 못 하기 때문이에요.

### Root cause

두 경로 모두 `axhub/src/commands/apps/git/flow.rs` 의 `prepare_github_with_device_code` 를 호출해요.

- `bootstrap` → `bootstrap_resolve.rs::resolve_github_installation` 가 **saga 시작 전** 이 함수를 호출 (`start_bootstrap` 보다 먼저).
- `apps git connect` → `prepare_github` 가 직접 호출.

interactive 모드(`--no-input`/`--non-interactive` 아님)에서 GitHub App 미설치 시 흐름:

```rust
events.emit("device_code_issued", ...)?;                 // --json: stdout(println), 비json: stderr
if no_input || non_interactive || ctx.no_input() {
    events.exit_core(Timeout, DEVICE_FLOW_REQUIRED_USER_ACTION, true)?;  // 즉시 EXIT
}
eprintln!("To connect GitHub, visit: {}", verification_uri);  // stderr (interactive 에서만)
eprintln!("Enter code: {}", user_code);                       // stderr
let token = poll_device_token(...).await?;                    // ❗ 승인까지 BLOCK
```

Claude Code Bash tool 은 프로세스 종료까지 stdout/stderr 를 buffer 해요. `poll_device_token` 이 최대 ~15분 block 하니까, 그 안에 emit 된 challenge 가 buffer 에 갇혀 안 보여요 → 사용자는 URL/code 를 못 받고, CLI 는 승인을 기다리며 멈춤 = deadlock.

### 선례

`/axhub:auth` Step 5b 가 `axhub auth login` 의 **동일한** blocking device flow 를 이미 해결했어요: `nohup`+`disown` 으로 detach → log tail 로 URL/code 추출 → 즉시 출력 후 `exit 0` → 다음 bash call 에서 완료 poll. init/github 엔 이 wrapper 가 없어요 (CLI 가 emit 하는데도 surface 메커니즘 부재).

## 핵심 사실 (소스 확정)

1. **stderr 가 신뢰 가능한 surface 채널.** `--json` 모드에서 `device_code_issued` JSON 은 stdout(println — file redirect 시 block-buffered 위험), 하지만 `To connect GitHub, visit:` / `Enter code:` 는 **stderr(eprintln, unbuffered)** 로 항상 나와요. 단 stderr 줄들은 non-interactive-exit 분기 *뒤*라 **interactive 모드에서만** 출력돼요 — 안 B 는 interactive detach 라 OK.
2. **bootstrap `--watch` 는 detach 에 부적합.** `watch_status` 는 중간 stage 를 stream 안 하고 terminal(`done`/`failed`)에서 1회만 출력 + `bootstrap_id` 도 끝까지 숨겨요. → detach 시 `--watch` 빼고 **no-watch** 사용: resolve(device flow block) → `start_bootstrap` → `accepted{bootstrap_id}` 출력 후 **exit** (saga 는 server-side 계속).
3. **github connect 은 saga 없음.** `prepare_github`(device flow block) → connect API → 결과 출력 → exit.

## 설계

### 공통 wrapper 모양 (auth 5b 이식)

```
nohup <cmd> >"$LOG" 2>&1 </dev/null &   # detach, $LOG = mktemp
disown
# ~15–30s tail loop: $LOG 에서 stderr 의 https URL + XXXX-XXXX code grep
#   발견 → 한국어로 surface + "$LOG $PID 다음 step poll 용" 출력 + exit 0
#   process 가 challenge 없이 빨리 exit → device flow 불필요 (App 설치됨) → 정상 경로
#   timeout → CLI 출력 형식 변경 의심, /axhub:doctor 안내 + exit 1
```

### `skills/init/SKILL.md` (Step 6–7)

기존 interactive `bootstrap --execute --yes --watch --json` 를 교체:

1. **Call 1 — detach + challenge surface.** `nohup axhub apps bootstrap --execute --yes --json ...`(no-watch) 를 $LOG 로 detach. tail loop 로 stderr URL/code 추출.
   - challenge 발견 → 사용자에게 surface 후 `exit 0` (process 는 bg 에서 승인 대기).
   - challenge 없이 빠른 exit → $LOG 의 `bootstrap_id` 캡처 (App 설치됨).
   - `install_url` event(App 자체 미설치) / `ambiguous_installation` → 해당 안내 분기.
2. **Call 2 — bootstrap_id 확보.** 사용자 승인 후 $LOG tail → `accepted{bootstrap_id}` 줄 도착(= resolve 완료 → start_bootstrap) 확인, `bootstrap_id` 추출.
3. **Call 3..N — saga 진행 narrate.** one-shot `axhub apps bootstrap-status $BOOTSTRAP_ID --json`(no-watch) 를 bounded poll, 매 call 빨리 exit 하며 stage 한국어 narrate → terminal(`done`/`failed`)까지.

### `skills/github/SKILL.md` (Step 4)

기존 `axhub apps git connect --app ... --execute --json`(blocking) 를 교체:

1. **consent-mint 분리.** 현재 한 fence 안의 `consent-mint`+`connect` 를 별도 call 로 분리 — PreToolUse consent gate 가 detached connect call 에서 pending token 을 claim 하도록 (auth 5b 와 동일 이유).
2. **Call 1 — detach + challenge surface.** `nohup axhub apps git connect ... --execute --json` 를 $LOG 로 detach, tail loop 로 stderr URL/code surface 후 `exit 0`.
3. **Call 2 — 결과 확인.** $LOG tail 로 terminal connect 결과(success/error) 또는 별도 `axhub apps git status --app $APP_ID --json` 로 연결 확인.

## 제약 — skill 게이트 (Phase 17/18) 유지

기존 skill 편집이라 scaffold 우회 아님. 단 아래 패턴 보존 필수:

- D1 non-interactive AskUserQuestion guard 유지.
- TodoWrite Step 0 + 매 step status sync 유지.
- in-body preflight 블록(`CANONICAL_PREFLIGHT_BLOCK`) 그대로 유지.
- 모든 신규 한글 텍스트 **해요체** (`bun run lint:tone --strict` 0 err).
- frontmatter `description:` 의 nl-lexicon trigger 어구 **불변** (`bun run lint:keywords --check` baseline lock).
- 신규 AskUserQuestion 추가 시 `tests/fixtures/ask-defaults/registry.json` 등록 (현재 설계상 신규 question 없음 — 기존 흐름 내 명령만 교체).

## 검증

- `bun run skill:doctor --strict` exit 0
- `bun run lint:tone --strict` 0 err
- `bun run lint:keywords --check` no diff
- `bun test` 회귀 0 fail
- `bunx tsc --noEmit` clean
- (이상적) GitHub App 미설치 상태로 실제 `/axhub:init` 1회 — $LOG 에 URL/code 가 수초 내 뜨고 surface 되는지 live 확인. authed 계정 있음(giri@jocodingax.ai); throwaway app 생성 주의.

## 구현 중 해소된 확인 사항

- **consent gate**: `consent/parser.rs` 의 `match_known_intent` 에 `apps bootstrap` 미포함 (`_ => is_destructive:false`) → bootstrap 은 consent-mint 불필요 (현 Step 6 도 mint 없이 ship). github `apps git connect` 도 parser 는 구 `github connect` 만 매핑 → 사실상 비-enforced 이지만 기존 consent-mint 를 보존하고 별도 call 로만 분리 (enforced 면 claim timing 정확, 아니면 무해).
- **`ctx.no_input()` (context.rs:116)** = `self.global.no_input || self.global.non_interactive` — 순수 flag 기반(isatty 아님). interactive detach 명령은 두 flag 다 안 주므로 device flow 가 interactive 경로(stderr eprintln URL/code + `poll_device_token` block)를 타요. 따라서 `</dev/null` detach 여도 challenge 가 stderr 로 나와 tail 로 잡히고, 프로세스는 살아서 poll → 승인 후 `bootstrap_id` flush 후 종료 = one-shot. (만약 추후 isatty 기반으로 바뀌어도, exit 전 emit 되는 `device_code_issued` JSON 의 URL/code substring 을 같은 regex 가 잡아 surface 는 유지 — defense-in-depth.)
- **`BootstrapStatusResponse` (apps.rs:143)** 은 `status`(string "done"/"failed")·`repo_full_name`·`app_id`·`stage` 를 flat 으로 가짐. one-shot `bootstrap-status --json` = `{"data":{...flat...}}`. → Step 7 의 repo 추출을 `.data.repo_full_name // .data.status.repo_full_name`(flat 우선, 구 nested fallback)로 수정.

## 검증되지 않은 부분 (live 필요)

- 새 bash wrapper 의 런타임 경로는 정적 분석 기반이에요. 테스트 964 pass 는 skill **구조**만 검증하고 wrapper bash 로직 자체를 실행하지 않아요. GitHub App 미설치 상태의 실제 `/axhub:init` 1회로 $LOG 내 URL/code 가 수초 내 surface 되는지 확인 권장 (PR/commit 에 명시).

## 보류 (YAGNI)

3개 skill(auth/init/github)이 같은 wrapper 를 갖게 되면 공유 shell helper 로 추출 고려 — 지금은 복사, 3번째 사본 시점에 추출.
