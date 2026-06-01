# Phase 0 Research: 플러그인 로그인 consent deny 수정

**Date**: 2026-06-01 | **Plan**: [plan.md](./plan.md)

Technical Context 의 미해결 결정 4건을 해소해요.

---

## R1. consent 저장 경로 — 폴백을 어디로?

**Decision**: `runtime_root()` 의 폴백을 `std::env::temp_dir()` 에서 `state_root().join("runtime")`(= `$XDG_STATE_HOME/axhub/runtime` 또는 `~/.local/state/axhub/runtime`)로 변경. **`XDG_RUNTIME_DIR` 가 설정된 경우의 분기는 그대로 둠.**

```rust
// crates/axhub-helpers/src/consent/key.rs
pub fn runtime_root() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())          // 빈 문자열 방어 (runtime_paths.rs env_path 패턴과 정합)
        .map(|d| PathBuf::from(d).join("axhub"))
        .unwrap_or_else(|| state_root().join("runtime"))   // 폴백: $TMPDIR → HOME-anchored 안정 경로
}
```

**Rationale**:
- **프로세스 안정성이 실증됨.** HMAC 키는 `state_root()/hmac-key` 에 저장되고, mint(Bash tool 프로세스)와 verify(hook subprocess)가 **같은 키로 서명·검증에 성공**하고 있어요(디버깅 기록 H8 에서 deny 원인은 서명 불일치가 아니라 `claim_pending_token` 의 `read_dir` 가 파일을 못 찾은 것). 즉 `state_root()` 는 두 프로세스에서 동일하게 해석된다는 게 이미 증명됐고, 그 뿌리에 consent 를 두면 mint↔read 가 같은 곳을 봐요. 이게 `CLAUDE_CODE_TMPDIR` 같은 가설 경로보다 강한 근거예요.
- **회귀 0.** 기존 consent E2E 테스트는 전부 `XDG_RUNTIME_DIR` 를 tempdir 로 세팅(`cli_e2e.rs:79-80,1983-84,2050-51…`)해서 **XDG 분기**를 타요. 폴백만 바꾸면 테스트가 타는 경로와 직교 → FR-005 구조적 보장.
- **권한 일관성.** `runtime` 하위 디렉터리는 mint 의 `set_private_dir_mode(&runtime_root())` 로 `0700`. 상위 `state_root()` 는 `load_or_mint_key()`(mint 가 먼저 호출)가 `0700` 으로 생성. consent 파일은 `consent-*.json`, 키는 `hmac-key` 라 파일명 충돌 없음.
- **`runtime` 서브디렉터리 분리** 이유: 키(`hmac-key`)와 consent 를 같은 `state_root()` 직속에 섞지 않고 `runtime/` 으로 나눠 의미 구분 + 스윕 대상 디렉터리를 좁힘.

**Alternatives considered**:
- `CLAUDE_CODE_TMPDIR` 사용 — harness 가 자식에 동일 부여한다고 가정. 기각: (a) Claude Code 전용이라 테스트·타 컨텍스트에서 미설정, (b) 디버깅 기록상 **hook subprocess 가 이 변수를 받는지 미확인**(Bash tool 에만 관측). 검증 불가 가정 위에 보안 게이트를 얹지 않아요.
- 항상 `state_root()/runtime` 사용(XDG_RUNTIME_DIR 무시) — 기각: Linux 동작을 바꿔 기존 테스트(consent 가 runtime tempdir 에 떨어지길 기대)가 깨지고 FR-005 회귀. 폴백만 바꾸는 게 최소·최안전.
- `~/.cache/axhub`(XDG_CACHE) — 기각: cache 는 재생성 가능 데이터용이라 정리 도구가 지울 수 있음. consent 는 보안 민감.

---

## R2. FR-007 — 만료 consent 스윕 범위

**Decision**: claim 경로의 기존 pending 만료 정리를 일반화하고, **mint/preauth 시점에 만료된 `consent-*.json` 전체를 스윕**해요(`consent-pending-*.json` 과 `consent-<session>.json` 모두). 별도 프로세스·스케줄 없음(clarification: opportunistic).

**Rationale**:
- **이미 절반 구현됨.** `claim_pending_token`(jwt.rs:233-235)은 pending 파일 decode 시 `Err("token_expired")` 면 `fs::remove_file` 로 정리해요. 즉 다음 preauth-check 가 claim 경로를 타면 만료 pending 이 정리돼요.
- **빈틈**: 로그인을 중도 포기하면 claim 이 안 돌아 pending 파일이 남고, 기존 session consent 도 만료 후 같은 디렉터리에 남을 수 있어요. 다음 **mint/preauth** 때 같은 디렉터리의 만료분을 스윕하면, 비휘발 경로로 옮긴 뒤에도 누적이 bound 돼요.
- **안전성**: 스윕은 `decode_token_file` 결과가 `Err("token_expired")` 인 파일만 제거. 동시 진행 중인 다른 로그인의 **유효(미만료)** consent 는 건드리지 않음 → 경쟁 무해.
- 구현: `sweep_expired_consent_files(dir, key)` 헬퍼(기존 `decode_token_file` 재사용 + `is_consent_token_path`: `consent-*.json` 매칭)를 `mint_token_to_path` 의 `create_dir_all`/`set_private_dir_mode` 직후와 `claim_pending_token` 의 `read_dir` 직전에 호출. 실패는 무시(best-effort, hook fail-open 정신).

**Alternatives considered**: SessionEnd hook 정리 — 기각(clarification 에서 사용자가 거부). 별도 GC 데몬 — 과설계.

---

## R3. P2 — deny 사유 표면화 (additive, 검증 필요)

**Decision**: `cmd_preauth_check` 의 deny 출력에 `hookSpecificOutput.permissionDecisionReason` 을 **추가**. 기존 `systemMessage` 는 **제거하지 않음**(belt-and-suspenders).

```jsonc
// deny 출력 (P2 적용 후)
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "permissionDecisionReason": "<format_preauth_deny_hint(...) 한국어 사유>"  // 추가
  },
  "systemMessage": "<동일 사유>"  // 유지
}
```

**Rationale**:
- axhub 는 **이미 사유를 만들어요.** `format_preauth_deny_hint`(parser.rs:302)가 "이 명령은 사전 승인이 필요해요. 먼저 '로그인해'라고 말해서 승인 카드를 받으세요." 를 반환하고, cmd_preauth_check(main.rs:1094)가 `systemMessage` 로 emit 해요. 디버깅 기록의 "이유 텍스트 없음" 은 axhub 가 사유를 안 만든 게 아니라 **Claude Code 가 deny 시 `systemMessage` 를 사용자에게 안 보여준** 정황이에요.
- Claude Code PreToolUse 계약에서 deny 사유의 canonical 필드는 `hookSpecificOutput.permissionDecisionReason` 로 확인했어요(2026-06-01 검토). 그 필드에 사유를 싣고, 사용자-visible prose 채널인 `systemMessage` 도 유지해 surface 차이를 흡수해요.

**🔬 구현 후 검증**: 설치된 Claude Code 표면에서 `permissionDecisionReason` + `systemMessage` 조합이 실제 deny UI 에 기대대로 보이는지 smoke 로 확인해요. 필드명은 더 이상 차단성 미정 사항이 아니지만, UI 표면화는 버전별 차이가 있을 수 있어요.

**P1 독립성**: P2 는 **정당한** deny(카드 없이 login, TTL 만료) 의 진단성만 개선해요. P1(runtime_root 수정)이 들어가면 happy-path 의 부당 deny 자체가 사라지므로, **P2 없이도 로그인은 통과**해요. tasks 에서 P2 는 분리된 후순위 태스크.

**Alternatives considered**: deny 사유를 main 흐름 stdout 으로 출력 — 기각(hook 출력 계약 위반, PreToolUse 는 JSON 한 덩어리).

---

## R4. Windows / HOME 경계 (명시만, 범위 외 수정)

**Finding**: `key.rs:9-13` 의 `home_dir()` 는 **`HOME` 만** 확인하고, 미설정 시 상대경로 `"."` 를 반환해요. 반면 `runtime_paths.rs:86-96` 의 `home_dir()` 는 `USERPROFILE`/`HOMEDRIVE`+`HOMEPATH` 도 처리해요. 따라서 `HOME` 이 없는 Windows 에서 `state_root()`(및 본 수정의 폴백)는 **CWD 상대경로로 degrade** 해서 프로세스 간 분기를 재유발할 수 있어요.

**Decision**: 본 수정의 **범위 외**로 남기되 명시. 근거: (a) 주 타깃은 macOS, (b) 본 수정은 **이미 출시된 HMAC 키와 동일한 `HOME` 의존성**을 쓸 뿐 — *더 나빠지지 않음*(키가 동작하는 환경에선 consent 도 동작). 두 `home_dir()` 정합화는 별도 작업.

**Action**: spec Assumptions 의 "Windows/Linux 도 같은 전략이 안전" 가정에 이 단서를 단다(아래). tasks 에 선택적 follow-up 으로 기록 가능.

---

## R5. gitnexus 도구명 메모 (implement 단계용)

`CLAUDE.md` 가 symbol 편집 전 `gitnexus_impact` 강제. 실제 MCP 도구명은 **`mcp__gitnexus__impact`** (이 세션 `ToolSearch "gitnexus_impact"` 는 미스). 본 plan 의 수동 4-call-site grep(`runtime_root`: key.rs:38,41 / jwt.rs:137-138,209)은 동등 근거지만, implement 단계에서 `mcp__gitnexus__impact({target:"runtime_root", direction:"upstream"})` 로 mandate 준수.

---

## 미해결 → 해소 매핑

| 미해결 항목 | 해소 |
|---|---|
| 저장 경로 선택 (spec Deferred) | R1 — `state_root().join("runtime")` 폴백 |
| FR-007 스윕 구체화 | R2 — mint/preauth 시 만료 `consent-*.json` opportunistic |
| P2 출력 필드 | R3 — `permissionDecisionReason` 추가, systemMessage 유지, UI smoke 검증 |
| 크로스플랫폼 안전성 가정 | R4 — Windows HOME 경계 명시, 범위 외 |

NEEDS CLARIFICATION 잔여: 없음(R3 은 구현 후 UI smoke task 로 전환, plan 진행 차단 아님).
