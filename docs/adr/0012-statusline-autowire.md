# ADR-0012: Statusline Autowire (v0.6.0)

## Status

Accepted (2026-05-14, feat/auto-statusline-wire-v0.6.0) — Option B-revised-v2 (dual-channel disclosure + silent merge) 채택. install.sh OR SessionStart 첫 fire (둘 중 먼저 만나는 곳) 에서 disclosure 표시, marker-gated idempotent. runtime merge 는 disclosure marker 존재 후에만.

관련 ADR:
- [ADR-0011](0011-skill-preflight-permission-fallback.md) — SKILL preprocessing preflight 권한 fallback
- Plan B: `.omc/plans/auto-statusline-wire-v0.6.0.md` §H

---

## Context

v0.5.11 + v0.5.12 에서 statusLine 활성화를 사용자가 `/axhub:enable-statusline` 실행 → snippet paste 로 수동 처리했어요. 실측 데이터에서 paste 단계 drop-off 가 발생해 자동화가 필요했어요.

axhub 는 이미 install 시 5개 신뢰 이벤트 (token write, telemetry, Gatekeeper, auth-refresh bg, helper HTTP download+execute) 를 수행해요. `~/.claude/settings.json` 의 statusLine 관리 (6번째 이벤트) 는 magnitude 가 같거나 작은 수준이에요.

Anthropic 에 plugin uninstall hook 이 없어서 plugin 삭제 후 `statusLine.command` 경로가 settings.json 에 남아요. orphan stub 이 axhub side 에서 graceful degradation 을 보장하는 유일한 path 예요.

marketplace install (`/plugin install axhub@axhub`) 이 dominant install path 라서 `install.sh` 만 disclosure channel 로 삼으면 누락이 생겨요 — SessionStart 쪽에서도 disclosure 를 cover 해야 해요.

---

## Decision

**Option B-revised-v2** (silent default-ON + dual-channel disclosure + orphan stub) 를 채택해요.

- **Dual-channel disclosure**: `bin/install.sh` path **또는** SessionStart 첫 fire (marker-gated, 둘 중 먼저 표시되는 곳). 어느 channel 이든 disclosure 표시 후 marker write → 이후 channel skip.
- **Runtime silent merge**: disclosure marker (`~/.local/state/axhub-plugin/install-disclosure-shown.txt`) 존재 시에만 `settings_merge::merge` 자동 실행. marker 부재 시 merge 하지 않고 이번 session 에서 disclosure 만 표시.
- **Opt-out**: `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` env 영구 disable.
- **Orphan stub**: plugin uninstall 후에도 `~/.local/state/axhub-plugin/orphan-stub-statusline.{sh,ps1}` 이 graceful empty output 을 보장.
- **Fail-open**: `hooks/session-start-autowire.{sh,ps1}` 는 항상 exit 0. `AXHUB_DISABLE_HOOKS` / `AXHUB_DISABLE_HOOK=session-start-autowire` / `AXHUB_DISABLE_STATUSLINE_AUTOWIRE` 3 env 모두 지원.

---

## 검증된 가정

1. **Trust event magnitude 비교**: install (1-5) ≥ (6) settings merge — disclosure 가 일관된 channel. 별도 runtime prompt 는 magnitude inconsistency.
2. **Marketplace install path**: `/plugin install axhub@axhub` 는 `install.sh` 를 호출하지 않아요. SessionStart-side disclosure 가 이 경로를 cover.
3. **Anthropic uninstall hook 부재**: plugin 삭제 후 SessionStart hook 자체가 사라지므로 self-healing 불가능. orphan stub 만이 graceful degradation path.
4. **TTY context in SessionStart**: SessionStart hook 는 subprocess 라 `isatty(stdout)` 가 false 일 가능성. `CLAUDE_PARENT_TTY` env 의존 또는 systemMessage fallback (Open Q §I #1).

---

## Alternatives Considered

| 선택지 | 기각 사유 |
|---|---|
| **Option A — deferred consent prompt** (SessionStart 첫 trigger 시 [Y/n]) | (5) helper-download 가 prompt 없이 진행되는데 (6) settings merge 만 prompt 하면 magnitude 비례성 깨져요. 1-prompt friction 의 justification 부족. |
| **Option B — silent without disclosure** | trust event 누적 시 사용자 알 권리 침해 + 투명성 의무 위반. |
| **Option B-revised (single-channel install.sh disclosure)** | marketplace install 이 `install.sh` 안 호출 → disclosure 누락 → silent mutation 위반 재발. iter 2 Architect Q2 에서 기각. |
| **Option C — post-uninstall self-healing via SessionStart self-detect** | plugin uninstall 후 SessionStart hook 자체가 사라짐 — self-detect 불가능. |
| **Option D — Anthropic upstream uninstall hook 요청 후 ship** | upstream 기다리는 동안 friction 유지. orphan stub 으로 axhub side mitigation 가능. |

---

## Why Chosen

- Trust event magnitude 일관성: install (1-5) 와 (6) 이 동일 disclosure channel 에서 처리돼요.
- **Dual-channel idempotent disclosure**: marketplace install path 도 SessionStart-side 가 cover → single-channel narrative 균열 없음.
- Runtime zero friction: disclosure marker 존재 후 silent merge → drop-off 없음.
- Orphan stub: plugin lifecycle 외 안전망 — plugin 삭제 후 graceful (empty output, no error).
- Foundation reuse: v0.5.13 `settings_merge::merge` pub API 를 그대로 reuse → drift 없음, test surface 축소.

---

## Consequences

### Positive

- 사용자 friction 0 (runtime): disclosure 한 번 후 자동 관리.
- Trust event magnitude 일관성: (1)-(6) 모두 동일 channel.
- v0.5.13 foundation reuse: 7-branch 결정표 그대로 활용.
- Orphan stub: uninstall 후 settings.json 경로가 남아도 graceful.

### Negative / Risk

- **install path 의존**: `install.sh` / `install.ps1` 우회 사용자 (manual binary copy 등) 는 install-side disclosure 미수신. SessionStart-side 가 보완.
- **dotfile sync**: chezmoi / Dotbot 사용자 의 `~/.claude/settings.json` git track 시 propagate 가능. README `Trust & Uninstall` 섹션 + `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 가이드로 mitigate.
- **TTY context**: SessionStart hook subprocess 에서 `isatty` 항상 false 가능. `CLAUDE_PARENT_TTY` env 의존 (Open Q #1 해결 전 fallback: systemMessage only).
- **Schema breaking change**: 미래 Claude Code `statusLine` schema 변경 → schema lock + release-check detection 책임.
- **Self-healing 불가**: Iter 1 Principle #7 (`Self-healing on plugin missing`) 제거. plugin uninstall 후 SessionStart hook 자체가 사라지므로 self-detect 불가능. orphan stub 이 진짜 mechanism.

---

## Follow-ups

| ID | 내용 | 버전 |
|---|---|---|
| FU-1 | `axhub-helpers self-cleanup` SKILL — post-uninstall manual cleanup (orphan stub + state dir + settings.json statusLine 복원) | v0.6.1 patch |
| FU-2 | Anthropic upstream uninstall hook 요청 — plugin lifecycle 표준화 issue 제출 | 비동기 |
| FU-3 | `AXHUB_ENABLE_STATUSLINE_AUTOWIRE_AUTOCLEAN=1` env — dotbot sync 사용자 대상 auto-revert on disable | v0.6.2 patch |
| FU-4 | `_axhub_managed: true` extra field 도입 — Claude Code schema validation 안전성 Context7 검증 후 | v0.7.0 minor |

---

## Pre-mortem Scenarios Coverage

| 시나리오 | Mitigation |
|---|---|
| S1 — Invalid JSON silent overwrite | foundation Branch 6 atomic abort — broken JSON 시 write 안 해요 |
| S2 — Inter-plugin race | foundation Branch 5 preserve + warning — 다른 plugin statusLine 발견 시 preserve |
| S3 — Claude Code schema breaking change | schema version lock + Context7 정기 체크 + release-check |
| S4 — Dotbot/chezmoi sync | README warning + `AXHUB_DISABLE_STATUSLINE_AUTOWIRE=1` 가이드 |
| S5 — Subprocess inheritance race | flock + marker mtime (60s window) — child 는 marker 발견 시 skip + write 안 해요 |
| S6 — 2-scope install | scope-aware marker (`auto-wire-done-user.json` / `...-project.json`) 로 isolation |
