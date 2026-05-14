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
| `token-freshness-gate` | `hooks/token-freshness-gate.sh` | Phase 3.5 deploy gate |
| `session-start-autowire` | `hooks/session-start-autowire.{sh,ps1}` | Claude Code SessionStart — fail-open exit 0; `AXHUB_DISABLE_HOOKS` / `AXHUB_DISABLE_HOOK=session-start-autowire` / `AXHUB_DISABLE_STATUSLINE_AUTOWIRE` 지원; background detach (non-blocking) |

여기서 다루지 않는 helper subcommand (예: `deploy-prep`, `bootstrap`,
`list-deployments`) 는 사용자 명시 호출이라 kill switch 적용 대상 아니에요.

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

### 3.6 SKILL preprocessing `!command` injection layer

SKILL `!command` injection 라인 (예: `!${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json`) 은 §3 의 Rust hook 진입점과 **별개의 layer** 예요. Claude Code 가 SKILL 을 initialize 할 때 preprocessing 단계에서 실행하고, Rust helper `hook_safety::is_hook_disabled()` 진입부 체크가 아닌 Claude Code 권한 게이트가 이 layer 를 통제해요.

이 layer 에도 동일한 fail-open 원칙을 적용해요:

1. **permission denial 감지 시**: strict-anchor regex (`/^Shell command permission check failed.*requires approval/im`) 매칭 → `{"systemMessage":"[axhub] 첫 실행이라 권한이 필요해요. …"}` stdout 출력 + `exit 0`. SKILL Step 0 흐름이 계속돼요.
2. **미매칭 unrecognized stderr**: `process.stderr.write(stderrText)` 로 parent 에 passthrough — silent black hole 방지. ADR-0010 "raw stderr 가 chat 으로 흘러요" 정합이에요.
3. **exit code**: denial 분기 → 0, 미매칭 분기 → helper exit code 그대로 propagate.

구현은 `scripts/codegen-preflight-injection.ts` 의 Node runner (lite variant / deploy variant) 가 single source 로 emit 해요. 상세 결정 근거는 [ADR-0011](adr/0011-skill-preflight-permission-fallback.md) 를 참고해요.

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
회피해요. helper 가 다시 한 번 더 체크하는 건 정합성용 안전망이에요.

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
