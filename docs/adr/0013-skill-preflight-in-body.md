# ADR-0013: SKILL preflight 를 in-body bash 로 이동 (load-time `!command` 주입 폐기)

## Status

Accepted (2026-05-25, Phase 27) — ADR-0011 을 **supersede** 해요. ADR-0011 의 핵심
"검증된 가정 #1" 이 거짓으로 판명돼서, 그 가정 위에 지어진 lite/deploy variant codegen +
strict-anchor denialRegex fallback 전체를 제거하고 preflight 를 workflow body 의 일반
bash 스텝으로 옮겨요.

## Context

ADR-0011 (PR #99) 은 `needs-preflight: true` SKILL 의 첫 실행 raw 영문 "requires approval"
노출을 막으려고 `!${CLAUDE_PLUGIN_ROOT}/bin/axhub-helpers preflight --json` 줄을 cross-shell
Node runner (`!`node -e "..."``) 로 감싸고, 안에서 permission denial 을 strict-anchor regex 로
잡아 한국어 systemMessage 로 바꾸려 했어요.

### ADR-0011 의 거짓 가정

ADR-0011 §검증된 가정 #1 은 이렇게 단언했어요:

> outer Claude Code Bash 권한 게이트가 `!node -e "..."` 자체에는 권한을 묻지 않고,
> inner `spawnSync(helper, ['preflight', '--json'])` 의 stderr 가 surface 에 노출돼요.
> 따라서 B fallback 의 strict-anchor regex 가 inner stderr 를 잡을 수 있어요.

실제 production 에서 이 가정은 **정반대**였어요. 사용자가 본 에러:

```
Shell command permission check failed for pattern "!`node -e "..."`": This command requires approval
```

권한 게이트의 검사 대상은 **outer `node -e "..."` 명령 그 자체**예요. 결과:

1. Claude Code 가 outer `node -e` 를 권한 게이트 → 첫 실행 (allow rule 없음) + prompt 못
   띄우는 SKILL preprocessing 컨텍스트 → 거부.
2. node 가 아예 실행 안 됨 → 안에 있는 한국어 fallback 의 `denialRegex` 는 **절대 도달 못 함**.
3. inner `spawnSync` 는 OS raw spawn 이라 Claude Code 권한 계층을 안 거쳐서, `axhub-helpers`
   는 "Shell command permission check failed" 문자열을 stderr 로 낼 일이 없어요 → `denialRegex`
   는 real 거부에 영원히 미매칭하는 **dead path**.

즉 fallback 이 자기 자신의 거부를 못 잡는 구조적 결함이었어요. node wrapper 로 감싼 탓에
권한 prompt 에 더 길고 못생긴 명령이 노출되기까지 했어요.

### 왜 ADR-0011 의 e2e probe 가 이걸 못 잡았나

`tests/e2e/claude-cli/permission-prompt-surface.test.ts` (지금은 삭제됨) 는 helper stub 이
denial 텍스트를 자기 stderr 로 직접 emit 하도록 조작한 **deterministic mock** 이었어요.
실제 Claude Code 권한 게이트를 한 번도 거치지 않아서, 거짓 가정을 그대로 통과시켰어요.

## 대안 검토 (이번엔 실제 검증)

| 대안 | 결과 |
|---|---|
| **A: `.claude-plugin/plugin.json` 의 `permissions` 필드** (ADR-0011 의 Option A) | **불가능 확정.** Claude Code 플러그인 manifest 스키마에 `permissions` 필드 없음 (공식 문서). ADR-0011 이 Phase 27.y RFC 로 미룬 가정 = 이제 사실로 닫힘. |
| **B: SKILL frontmatter `allowed-tools: Bash(...)`** | `allowed-tools` 는 문서화된 SKILL 필드지만, 자기 자신의 **load-time `!command` 주입**까지 커버하는지는 **undocumented + 문서 자체가 모순** (timing 미정의). 게다가 이 repo 는 Phase 6 Q1 에 `allowed-tools` 를 deploy SKILL 에서 제거하고 금지 test (`tests/manifest.test.ts`) 까지 박았어요. 미검증 동작에 다시 거는 건 ADR-0011 의 실수 반복이라 비채택. |
| **C: PreToolUse hook 자동 승인** | `!command` preprocessing 단계에 PreToolUse hook 이 발동하는지 uncertain (deploy SKILL 의 기존 prose 도 "PreToolUse hook 은 preprocessing 단계에서 trigger 안 해요" 라고 명시). 비채택. |
| **D (채택): in-body bash 스텝** | load-time `!command` 자체를 제거하고 preflight 를 workflow body 의 일반 bash 호출로 이동. normal Bash tool 호출은 default 모드에서 **interactive prompt** (hard-fail 아님) 로 가고, SKILL 자체의 exit-code 핸들링이 적용돼요. **유일하게 문서화된 baseline 동작에만 의존** — undocumented 동작 도박 없음. |

## Decision

1. 모든 `needs-preflight: true` SKILL (15 SKILL + 1 template) 에서 load-time `!command`
   preflight 주입을 제거하고, workflow body 시작부에 canonical in-body preflight 블록을 둬요:

   ```bash
   PREFLIGHT_JSON=$("${CLAUDE_PLUGIN_ROOT:-.}/bin/axhub-helpers" preflight --json 2>/dev/null \
     || axhub-helpers preflight --json 2>/dev/null)
   ```

   단일 소스는 `scripts/preflight-block.ts` 의 `CANONICAL_PREFLIGHT_BLOCK` 예요.

2. `needs-preflight: true` frontmatter 는 **유지하되 의미를 재정의**해요 — "load-time
   `!command` 주입" 이 아니라 "workflow body 에서 `axhub-helpers preflight --json` 을 in-body
   로 실행" 이에요.

3. `scripts/codegen-preflight-injection.ts` (생성형 `!command` 단일소스) + byte-identical lock
   을 폐기해요. `scripts/skill-doctor.ts` 의 preflight 검사를 역전 — `needs-preflight: true` 는
   (a) `!command` 주입이 **없을 것** + (b) body 가 `axhub-helpers preflight --json` 을 호출할 것을
   요구하고, 모든 SKILL 은 dead injection 을 갖지 않아야 해요.

4. deploy SKILL 의 PowerShell `CLAUDE_PLUGIN_ROOT` 셋업 prose (command-lane 용) 는 **유지**해요 —
   주입이 아니라 Windows 실행 lane 셋업이에요.

기존 16 SKILL + template 의 마이그레이션은 일회성 스크립트로 수행했고, ship 후 제거했어요 (단일소스
`scripts/preflight-block.ts` + scaffold + skill-doctor 가 이후를 강제하므로 재사용 불필요).

## Consequences

### + 긍정

- 첫 실행 raw 영문 "requires approval" hard-fail 0 회 — preprocessing 단계 hard-fail 이 normal
  Bash interactive prompt 로 대체돼요.
- 검증 안 된 가정 의존 제거 — ADR-0011 을 낳은 실수 패턴을 반복하지 않아요.
- 거대한 `node -e` blob 이 권한 prompt 에 노출되던 UX 악화 해소.
- codegen + byte-identical lock 제거로 인지 부담/유지보수 감소. canonical block 은 짧고
  사람이 읽을 수 있어서 fragile escaped blob 의 sync 가 불필요해요.

### − 부정 / trade-off

- `${CLAUDE_PLUGIN_ROOT}` 가 SKILL preprocessing 처럼 자동 주입되던 컨텍스트가 사라지고, 모델이
  Step-1 에서 preflight 를 실행한 뒤 그 출력을 읽어요 (호출 1회 추가). SKILL=LLM 지시라 실질 영향 미미.
- default 모드 첫 실행에서 권한 prompt 는 여전히 1회 떠요 (Option A/plugin.json 가 불가능하므로
  TTFD=0 은 달성 불가). 단 hard-fail 이 아니라 정상 prompt 라 사용자가 '허용' 하면 진행돼요.
- end-to-end default-mode 권한 동작은 문서화된 baseline Bash 동작이라 별도 exotic 검증 불필요지만,
  실제 라이브 확인은 default 모드 환경에서 수행해요 (CI e2e harness / 사용자).

## 관련

- [ADR-0011](0011-skill-preflight-permission-fallback.md) — 본 ADR 이 supersede.
- [ADR-0010](0010-stderr-filter-graceful-degradation.md) — axhub binary stderr graceful degradation (다른 layer).
- [docs/HOOKS.md §3.6](../HOOKS.md) — SKILL preprocessing layer 설명 (본 ADR 로 갱신).
- `scripts/preflight-block.ts` — canonical in-body preflight 단일소스 (scaffold + skill-doctor 가 참조).
