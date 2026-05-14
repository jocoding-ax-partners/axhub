# axhub-helpers settings-merge

`settings-merge` 서브커맨드는 `~/.claude/settings.json` (또는 `.claude/settings.json`) 에 axhub statusLine 설정을 안전하게 병합해요.

---

## v0.6.2 — Plugin-root ambiguity 핫픽스

v0.6.1 이전에는 `statusLine.command` 에 `${CLAUDE_PLUGIN_ROOT}` 리터럴을 기록했어요. axhub + OMC 같이 여러 plugin 이 동시 설치된 환경에서 Claude Code 가 활성 plugin context 로 expand 하면 다른 plugin root 로 해석돼 statusline 이 render 안 되는 production bug 가 있었어요.

v0.6.2 부터 `default_command_path()` 가 **plugin-agnostic orphan stub 절대경로**를 반환해요:

| 플랫폼 | stub 절대경로 |
|--------|--------------|
| macOS / Linux | `~/.local/state/axhub-plugin/orphan-stub-statusline.sh` (또는 `$XDG_STATE_HOME/axhub-plugin/orphan-stub-statusline.sh`) |
| Windows native | `%LOCALAPPDATA%\axhub-plugin\orphan-stub-statusline.ps1` |

stub 자체는 axhub plugin 이 살아 있으면 실제 statusline 으로 위임하고, plugin 이 삭제된 후에는 빈 output exit 0 으로 graceful 하게 처리해요. `statusLine.command` 가 plugin root 가 아닌 user-global state dir 을 가리키기 때문에 어떤 plugin 이 활성이든 경로가 일정해요.

기존 broken settings.json (`${CLAUDE_PLUGIN_ROOT}` literal 이 남아 있는 경우) 은 `axhub-helpers settings-merge --migrate` 로 atomic 치유해요.

---

## --apply

`~/.claude/settings.json` 에 axhub statusLine 을 atomic 병합해요. 7-branch 결정 트리 + `.bak` rollback + flock 으로 safe 해요.

```bash
axhub-helpers settings-merge --apply [--scope user|project|auto] [--dry-run]
```

**사전 조건 (v0.6.2+)**: `--apply` 호출 전 `orphan_stub::install_and_verify()` 가 stub 을 자동 설치해요. stub 설치 실패 시 non-zero exit 으로 실패해요 (fail-closed — manual 호출이므로 non-zero 적절).

### Exit code

| 코드 | 의미 |
|------|------|
| 0 (NoOp) | 이미 axhub-managed statusLine 있어요 — 변경 없음 |
| 2 (Created) | settings.json 만들고 statusLine 추가했어요 |
| 3 (Merged) | 기존 settings.json 에 statusLine 추가했어요 |
| 4 (PreservedOther) | 다른 plugin 의 statusLine 발견 — preserve 했어요. 강제 override 는 재실행 |
| 5 (InvalidJson) | settings.json 파싱 안 돼요 — 직접 수정 후 재시도 |
| 6 (PartialSchema) | 스키마 불완전 — stderr 안내 따라 수동 해결 |
| 7 (Permission) | 파일 권한 오류 — stderr 안내 따라 수동 해결 |

---

## --migrate

기존 `${CLAUDE_PLUGIN_ROOT}` literal 이 남아 있는 settings.json 을 orphan stub 절대경로로 atomic 치환해요. `--scope auto` 시 user scope + project scope 양쪽을 스캔해요.

```bash
axhub-helpers settings-merge --migrate [--scope user|project|auto] [--dry-run] [--yes]
```

| 플래그 | 동작 |
|--------|------|
| `--dry-run` | detection 만 (write 없음) |
| `--yes` | TTY prompt 없이 자동 적용 |
| (없음, TTY 있음) | interactive prompt 후 적용 |
| (없음, TTY 없음) | abort (dry-run 출력 후 exit) |

**git-tracked settings.json 감지 시**: 자동 write 하지 않고 warn-only + 수동 review 안내 (S7 pre-mortem).

### Exit code

| 코드 | 의미 |
|------|------|
| 0 | stale literal 없음 / 이미 stub path |
| 2 | atomic rewrite 완료 (또는 dry-run detection) |
| 3 | git-tracked 파일 — warn-only, 자동 write 거부 |

---

## Scope

| 값 | 대상 파일 |
|----|-----------|
| `user` | `~/.claude/settings.json` |
| `project` | `.claude/settings.json` (현재 repo 루트) |
| `auto` | user + project 양쪽 자동 탐색 |

---

## 예시

```bash
# statusLine 추가 (recommended — stub 자동 설치 포함)
axhub-helpers settings-merge --apply --scope auto

# 기존 ${CLAUDE_PLUGIN_ROOT} literal 자동 치유
axhub-helpers settings-merge --migrate --yes

# migration dry-run (변경 없이 detection 만)
axhub-helpers settings-merge --migrate --dry-run

# project scope 만 마이그레이션
axhub-helpers settings-merge --migrate --scope project --yes
```

---

## 관련 문서

- `docs/HOOKS.md` — session-start-autowire hook (SessionStart 자동 merge)
- `skills/enable-statusline/SKILL.md` — `/axhub:enable-statusline` UX 흐름
- `.omc/plans/statusline-plugin-root-ambiguity-fix-v0.5.14.md` — Option B 설계 ADR
