# claude CLI flags freeze (v2.1.121)

> Phase 22.0 prep — Plan SB-1 / Pre-mortem #4 / P2-11. claude CLI v2.1.121 `--help` 출력 baseline freeze. Phase 22 harness 가 사용하는 모든 argv 가 이 목록 안에서만 선택돼요. minor bump 시 manual schema verification PR 필요.

## Captured

- claude version: `2.1.121 (Claude Code)`
- alias: `claude --dangerously-skip-permissions` (local dev only)
- captured at: 2026-04-28

## Real flags (used by spawn.sh)

| flag | 사용처 | 설명 |
|------|--------|------|
| `-p, --print` | T1 spawn 핵심 | non-interactive print mode |
| `--add-dir <dirs...>` | spawn | 추가 디렉토리 access |
| `--mcp-config <configs...>` | spawn | MCP servers JSON files/strings |
| `--strict-mcp-config` | spawn | `--mcp-config` 외 MCP 차단 |
| `--plugin-dir <path>` | spawn | axhub plugin root 활성화 (repeatable) |
| `--no-session-persistence` | spawn | 세션 저장 안 함 (`-p` 와 함께만) |
| `--output-format <format>` | spawn | `text`, `json`, `stream-json` |
| `--max-budget-usd <amount>` | spawn (cap 0.30) | API 비용 cap (`--print` 와 함께만) |
| `--setting-sources <sources>` | spawn | `user,project,local` |
| `--permission-mode <mode>` | spawn | `acceptEdits`/`auto`/`bypassPermissions`/`default`/`dontAsk`/`plan` |
| `--tools <tools...>` | spawn | built-in tool 제한 (`""` 빈 disable) |
| `--disable-slash-commands` | case-별 (NL only) | 슬래시 명령 차단 |
| `--bare` | NOT used | hooks/LSP/plugin sync skip → SessionStart 무력화 trade-off |
| `--dangerously-skip-permissions` | CI 만 | 권한 prompt 건너뛰기 (sandbox 격리됨) |
| `-v, --version` | smoke | 버전 출력 |
| `-h, --help` | sanity | 이 파일 source |

## Hallucinated flags (DO NOT use — v2.1.121 에 부재)

v1 plan 에서 fabricated 됐던 flag — claude --help 직접 검증 결과 부재 확인:

- `--no-mcp-config` ❌ — 실제 차단은 `--strict-mcp-config` + 빈 `--mcp-config '{}'`
- `--skip-skills "*"` ❌ — 실제는 `--disable-slash-commands` (NL trigger 검증과 충돌이라 case-별 분리)
- `--enable-plugin "<slug>"` ❌ — 실제는 `--plugin-dir <path>` 만

## --output-format json schema (CLAUDE_JSON_SCHEMA.md 참조)

`-p --output-format json` 의 stdout shape 은 별도 `CLAUDE_JSON_SCHEMA.md` 에 freeze. graceful abort sentinel `stop_reason ∈ {abort, user_cancelled, end_turn}` 의 실제 enum 값은 Phase 22.5.5 baseline measurement 단계에서 측정 후 이 schema 에 lock.

## Verification

```bash
# 22.0 sanity check
diff <(claude --version) <(echo "2.1.121 (Claude Code)") || \
  echo "WARN: claude version drift, schema 재검증 필요"
```

minor bump 시 PR 워크플로:
1. `claude --help > /tmp/claude-help.txt`
2. 이 파일과 diff 검토
3. drift 발견 시 본 파일 update + spawn.sh argv 영향 검토
4. CLAUDE_JSON_SCHEMA.md 도 함께 검증
