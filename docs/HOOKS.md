# axhub Hook Safety + Fail-open Spec

**Phase 25 PR 25.2 산출물**. axhub Claude Code hook 의 fail-open 계약을
명문화하고 두 단계의 canonical kill switch (`AXHUB_DISABLE_HOOKS`,
`AXHUB_DISABLE_HOOK`) 을 정의해요.

관련 ADR:
- `.plan/matrix-absorption/00-overview.md` §10.6 — Env Var Taxonomy ADR
- `.plan/matrix-absorption/00-overview.md` §10.7 — Stateless 원칙 reconciliation

---

## 1. 적용 범위

다음 axhub hook 진입점 모두 본 spec 따라요.

| 진입점 | 위치 | 호출 시점 |
|---|---|---|
| `session-start` | `hooks/session-start.sh`, `hooks/session-start.ps1`, `axhub-helpers session-start` | Claude Code SessionStart |
| `preauth-check` | `axhub-helpers preauth-check` | PreToolUse (Bash) |
| `prompt-route` | `axhub-helpers prompt-route` | UserPromptSubmit |
| `classify-exit` | `axhub-helpers classify-exit` | PostToolUse (Bash `axhub …`) |
| `verify-deploy-artifact` | `axhub-helpers verify-deploy-artifact` via `hooks/axhub-helpers.sh` | PostToolUse (successful `axhub deploy create`; advisory artifact sanity check) |
| `token-freshness-gate` | `axhub-helpers token-gate` | Phase 3.5 deploy gate. sh body 가 Rust 로 흡수됐어요 (Phase 1.1 sh/ps1 absorption, T3). 기존 env 컨트랙트 (`AXHUB_GATE_*`) 와 exit 65 UNAUTHORIZED 시맨틱 그대로 보존했어요. Windows 사용자도 동일 binary 가 자동 동작해요 (parity gap #1 해소). Phase 4 (F1) 에서 `hooks/token-freshness-gate.sh` thin shim 도 삭제 — SKILL deploy Step 3.5 가 helper 직접 호출. |
| `session-start-autowire` | `hooks/session-start-autowire.{sh,ps1}` | Claude Code SessionStart — fail-open exit 0; `AXHUB_DISABLE_HOOKS` / `AXHUB_DISABLE_HOOK=session-start-autowire` / `AXHUB_DISABLE_STATUSLINE_AUTOWIRE` 지원; background detach (non-blocking) |
| `plugin-drift` | `axhub-helpers plugin-latest-fetch-bg` (SessionStart 에서 detached 스폰) + 드리프트 nudge in `cmd_prompt_route` | 플러그인 버전 드리프트 알림. fetch 는 SessionStart 에 캐시 warm (24h TTL, fail-open), nudge 는 UserPromptSubmit 에서 캐시 비교 후 버전당 1회 주입. `AXHUB_DISABLE_HOOKS` / `AXHUB_DISABLE_HOOK=plugin-drift` 지원 (helper 내부 `is_hook_disabled("plugin-drift")` 게이트). 영구 opt-out 은 `plugin-drift-optout` 마커 |

여기서 다루지 않는 helper subcommand (예: `deploy-prep`, `bootstrap`,
`list-deployments`) 는 사용자 명시 호출이라 kill switch 적용 대상 아니에요.

### 1.1 session-start-autowire 와 settings-merge --migrate (v0.6.2)

`session-start-autowire` 가 `settings-merge --apply` 를 통해 `~/.claude/settings.json` 에 statusLine 을 기록해요. v0.6.1 이전 버전에서 hook 이 `${CLAUDE_PLUGIN_ROOT}` 리터럴을 기록한 경우, `--migrate` subcommand 로 orphan stub 절대경로로 치유해요:

```bash
axhub-helpers settings-merge --migrate --yes
```

이 subcommand 는 hook 이 아니라 사용자 명시 호출이에요. kill switch 무관. 상세 동작은 `docs/settings-merge.md` 를 참고해요.

---

## 2. Kill Switch (canonical)

### 2.1 모든 hook 비활성화

```bash
AXHUB_DISABLE_HOOKS=1
```

`1` / `true` / `yes` / `on` 모두 truthy. SessionStart 부터 PostToolUse 까지
모든 진입점이 즉시 `exit 0` + (필요 시) `{"continue":true}` 또는 `{}`
출력하며 통과해요.

### 2.2 특정 hook 만 비활성화

```bash
AXHUB_DISABLE_HOOK=session-start,preauth-check
```

쉼표로 구분한 hook 이름 목록. 공백 주변은 무시해요 (`"foo , bar"` →
`["foo", "bar"]`). 위 §1 표의 이름과 일치해야 해요.

### 2.3 우선순위

1. `AXHUB_DISABLE_HOOKS` truthy → 모든 hook skip (per-hook 무시)
2. 그 외 `AXHUB_DISABLE_HOOK` csv 매칭 → 해당 hook 만 skip
3. 그 외 default behavior

### 2.4 Legacy alias (6-month deprecation)

```bash
DISABLE_AXHUB=1   # ← v0.8.0 에서 제거 예정
```

이름이 통일되기 전부터 쓰이던 alias 예요. 현재는 honored 되지만 호출 시
stderr 에 한 번 deprecation warning 을 출력해요. 새 자동화 스크립트는
`AXHUB_DISABLE_HOOKS=1` 을 써주세요.

| 변경 시점 | 동작 |
|---|---|
| v0.7.x (현재) | `DISABLE_AXHUB=1` honored + stderr 경고 |
| v0.8.0 (예정) | `DISABLE_AXHUB` 미인식. `AXHUB_DISABLE_HOOKS=1` 만 동작 |

---

## 3. Fail-open 원칙

모든 axhub hook 은 **fail-open** 이에요. 어떤 실패에서도 메인 흐름을
차단하지 않아요. 구체 규칙:

1. **exit code 는 항상 0**. helper binary 부재, 네트워크 실패, config
   손상, 권한 부족, panic — 어떤 사유든 마찬가지예요.
2. **에러 노출은 `systemMessage` 로만**. JSON `{"systemMessage":"..."}` 를
   stdout 에 한 줄 출력하면 Claude Code 가 사용자 채팅 surface 에 노출해요.
3. **panic 금지**. Rust helper 는 모든 진입점에서 `Result<_, anyhow::Error>`
   를 반환하고 panic 가능성이 있는 코드는 `unwrap_or_else` / `?` 로
   감싸요. `std::panic::catch_unwind` 가 필요한 경계는 audit 또는 redact
   같은 외부 입력 처리 site 에 한정해요.
4. **silent skip 패턴**. TTY 없음 / `CLAUDE_PLUGIN_ROOT` 누락 /
   CI 환경 같은 known 비상 컨텍스트에서는 systemMessage 도 출력하지 않고
   조용히 통과해요 (사용자에게 의미 없는 noise 피해요).
5. **debug 흔적 보존**. 실패가 발생하면
   `$XDG_STATE_HOME/axhub-plugin/hook-errors.jsonl` (또는 OS 별 fallback)
   에 atomic append 해요. 이 파일이 손상되더라도 main 흐름은 계속.

### 3.1 hook-errors.jsonl 스키마

```jsonl
{"ts":"2026-05-11T14:00:00+00:00","hook":"classify-exit","error":"stdin read failed: ..."}
{"ts":"2026-05-11T14:01:23+00:00","hook":"session-start","error":"installer path missing"}
```

- 파일 권한: `0o600` (Unix)
- rotation 정책: phase-26 PR 26.1a 의 `atomic_jsonl::rotate_old` 와 동일한
  7-day 정책 적용 (event_log / audit 와 일관).
- 외부 전송 0. 로컬 disk 만.

### 3.6 SKILL preflight 는 in-body bash 스텝 (load-time `!command` 주입 폐기)

`needs-preflight: true` SKILL 은 workflow body 시작부에서 `axhub-helpers preflight --json` 을 일반 bash 스텝으로 실행해요 (`scripts/preflight-block.ts` 의 `CANONICAL_PREFLIGHT_BLOCK` 단일소스). 이건 §3 의 Rust hook 진입점과도, 예전 load-time `!command` 주입과도 별개예요 — normal Bash tool 호출이라 Claude Code 의 표준 권한 흐름 (default 모드에서 interactive prompt) 을 거쳐요.

**과거**: load-time `!`node -e "..."`` 주입 + inner denialRegex fallback 을 썼지만, Claude Code 가 outer `node -e` 명령 자체를 권한 게이트해서 첫 실행에 raw 영문 "requires approval" 로 hard-fail 했고, fallback 은 자기 자신의 거부를 못 잡는 dead path 였어요. 상세는 [ADR-0013](adr/0013-skill-preflight-in-body.md) (supersedes [ADR-0011](adr/0011-skill-preflight-permission-fallback.md)) 를 참고해요.

검사: `scripts/skill-doctor.ts` 가 `needs-preflight: true` SKILL 에 (a) `!command` 주입이 없고 (b) body 가 `axhub-helpers preflight --json` 을 호출하는지 강제하고, 모든 SKILL 에 dead injection 이 없는지 확인해요. preprocessing 단계의 fail-open 계약은 더 이상 별도 layer 가 아니라 normal Bash 권한 흐름으로 흡수됐어요.

### 3.7 onboarding 온보딩의 D1/consent boundary

`skills/onboarding/SKILL.md` 는 사용자-facing 온보딩 단일 진입점이에요. `온보딩`, `처음인데 뭐부터`, `getting started` 같은 말은 onboarding 으로 들어가고, onboarding 이 `detect-first → 첫 gap 처리 → 재감지` 루프로 CLI·auth·git·node·GitHub App·repo/app·의존성·doctor gap 을 닫아요.

hook 관점의 안전 계약은 이래요.

- `claude -p` / CI / headless 에서는 onboarding 의 AskUserQuestion 이 D1 guard 로 safe default 를 사용해요.
- install/update/auth/init/deploy/deps mutation 과 git/node system install 또는 version switch 는 non-interactive 에서 자동 실행하지 않아요. 이 경우 onboarding 은 `SAFE_STOP_NONINTERACTIVE` 또는 `READY_WITH_USER_ACTION` 문구로 멈춰요.
- GitHub 전진배치는 auth 뒤 `install_url` 로 계정레벨 GitHub App 설치만 surface 해요. OAuth device-flow 와 app↔repo connect 는 init/github 단계에 남아요.
- dependency install 은 repo on disk 뒤 lockfile 있을 때만, 명시 consent 뒤, `--ignore-scripts` 를 붙인 command 로만 허용해요. 이 예외는 `scripts/skill-doctor-allowlist.json` 의 `onboarding` allowlist 로 잠겨요.
- 최종 `VIBE_READY` 카드는 확인된 항목만 green 으로 표시해야 해요. deployment URL 만 있고 status/watch evidence 가 없으면 degraded ready 로 낮춰요.

---

## 4. 구현 reference

### 4.1 Rust helper (canonical)

`crates/axhub-helpers/src/hook_safety.rs` 의 `is_hook_disabled(name)` 가
canonical 구현이에요. 새 hook subcommand 추가 시 진입부 첫 줄에서:

```rust
fn cmd_my_new_hook() -> anyhow::Result<i32> {
    if hook_safety::is_hook_disabled("my-new-hook") {
        out_json(json!({}));
        return Ok(0);
    }
    // ... 정상 동작 ...
}
```

`out_json` 의 payload 는 hook 종류별로 spec 이 달라요:
- PreToolUse (`preauth-check`) → `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"}}`
- PostToolUse advisory verifier (`verify-deploy-artifact`) → kill-switch/skip 은 stdout 없이 `Ok(0)`, 의심 신호가 있을 때만 `systemMessage` + `PostToolUse.additionalContext`
- 그 외 → `{}` (no systemMessage, no decision)

### 4.2 Shell hook (mirror)

`hooks/session-start.sh`, `hooks/session-start.ps1`,
`hooks/token-freshness-gate.sh` 모두 stub 시작 부분에서 kill switch
체크해요. POSIX `sh` 패턴:

```bash
if [ "${AXHUB_DISABLE_HOOKS:-0}" = "1" ] || [ "${DISABLE_AXHUB:-0}" = "1" ]; then
  exit 0
fi
case ",${AXHUB_DISABLE_HOOK:-}," in
  *,my-hook,*) exit 0 ;;
esac
```

PowerShell:

```powershell
if ($env:AXHUB_DISABLE_HOOKS -eq '1' -or $env:DISABLE_AXHUB -eq '1') { exit 0 }
if ($env:AXHUB_DISABLE_HOOK) {
  $disabled = $env:AXHUB_DISABLE_HOOK -split ',' | ForEach-Object { $_.Trim() }
  if ($disabled -contains 'my-hook') { exit 0 }
}
```

shell stub 은 일찍 (binary 호출 전) skip 해서 helper 자체 invocation 도
회피해요. `hooks/axhub-helpers.sh` 는 source checkout / clean install 처럼
helper binary 가 아직 없을 때도 context/telemetry 계열 hook 을 fail-open 해요
(`verify-deploy-artifact` 포함). helper 가 다시 한 번 더 체크하는 건 정합성용
안전망이에요.

---

## 5. 테스트 매트릭스

`tests/hooks-kill-switch.test.ts` 가 다음 조합 모두 검증해요.

| 조합 | 기대 |
|---|---|
| 둘 다 unset | hook 정상 동작 |
| `AXHUB_DISABLE_HOOKS=1` | 모든 hook skip |
| `AXHUB_DISABLE_HOOK=session-start` | session-start 만 skip, 나머지 동작 |
| `AXHUB_DISABLE_HOOK=session-start,preauth-check` | 두 hook skip |
| `DISABLE_AXHUB=1` (legacy) | 모든 hook skip + stderr 경고 1 회 |
| `AXHUB_DISABLE_HOOKS=1` + `AXHUB_DISABLE_HOOK=foo` | global 이 per-hook 무시 |

Rust unit test (`hook_safety::tests`) 는 truthy/falsy variant
(`1`/`true`/`yes`/`on` vs `0`/`false`/`no`/`""`) 도 추가로 검증해요.

---

## 6. Env var 종합 (overview §10.6 reference)

axhub 가 인식하는 모든 환경변수의 polarity / scope 는 `.plan/matrix-absorption/00-overview.md` §10.6 의 ADR 표를 따라요. 본 문서는 hook 안전성 영역만 다뤄요.

핵심 polarity 룰:
- destructive disable → `AXHUB_DISABLE_<scope>=1`
- feature enable → `AXHUB_ENABLE_<scope>=1`
- path / threshold override → `AXHUB_<scope>=<value>`

신규 hook 또는 helper 가 자체 kill switch / opt-out 도입 필요할 때는
**먼저 §10.6 의 룰에 맞춰 변수 이름을 정한 뒤** 본 문서 §1 표에 추가
해주세요.

## Phase 26 quality hook contract

axhub quality hooks are fail-open except the explicit PreToolUse review gate, which returns a Claude Code `permissionDecision: ask` for `git commit` / `git push` when the current HEAD or worktree has not been reviewed.

### additionalContext template

Every agent-facing hook context uses tagged English blocks:

```text
<axhub-...>
[axhub hook | purpose]
Observed: concrete state
Suggested: next safe action
Skip: AXHUB_DISABLE_HOOK=<hook-name>
</axhub-...>
```

User-facing warnings stay in Korean `systemMessage`; machine/action context stays in `hookSpecificOutput.additionalContext`.

### Quality hook entries

- `commit-gate` blocks unreviewed `git commit` / `git push` with an ask decision.
- `tdd-inject` reminds on source file writes before tests.
- `test-classifier` records failed test commands in `.axhub-state/quality.json`.
- `state-update --edit-event` updates changed-line counters after source edits.
- `state-update --post-commit-promote` promotes review acknowledgement after a matching commit.

### Opt-outs

- `AXHUB_DISABLE_TRIGGERS=1` disables quality reminders and gates.
- `AXHUB_DISABLE_MEGASKILL=1` disables SessionStart quality context only.
- `AXHUB_DISABLE_KARPATHY=1` disables Karpathy UserPromptSubmit context only.
- `AXHUB_DISABLE_POSTCOMMIT=1` disables post-commit promotion only.
