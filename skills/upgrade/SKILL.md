---
name: upgrade
description: '이 스킬은 사용자가 axhub Claude Code 플러그인 자체를 업그레이드하거나 최신인지 확인하고 싶어할 때 사용합니다 (CLI 가 아닌 플러그인). 다음 표현에서 활성화: "Claude에 설치된 axhub 플러그인도 최신인지 봐줘", "Claude 플러그인 최신인지 확인해줘", "axhub 플러그인도 최신인지 봐줘", "플러그인 최신인지", "플러그인 최신인지 봐줘", "플러그인 최신인지 확인해줘", "플러그인 새 버전", "플러그인 업그레이드", "플러그인 업데이트", "플러그인 버전", "플러그인 호환", "skills/hooks 최신", "지금 플러그인 버전이 뭐야", "플러그인이랑 호환되는 버전이야", "axhub plugin latest", "axhub plugin update", "axhub plugin upgrade", "axhub plugin version", "plugin latest", "plugin self-upgrade", "plugin update", "plugin upgrade", "plugin version", 또는 axhub 플러그인 self-upgrade 요청. CLI 바이너리를 업그레이드하는 skills/update 와는 별개. PLAN row 28 의 DX-6 fix 를 구현합니다.'
examples:
  - utterance: "플러그인 새 버전"
    intent: "upgrade axhub plugin"
  - utterance: "플러그인 업그레이드"
    intent: "upgrade axhub plugin"
  - utterance: "axhub plugin update"
    intent: "upgrade axhub plugin"
  - utterance: "axhub plugin upgrade"
    intent: "upgrade axhub plugin"
  - utterance: "플러그인 업데이트"
    intent: "upgrade axhub plugin"
  - utterance: "Claude에 설치된 axhub 플러그인도 최신인지 봐줘"
    intent: "check axhub plugin version"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Upgrade (plugin self-upgrade nudge)

Upgrade the axhub Claude Code plugin itself. **Distinct from `skills/update`** — that handles `axhub` CLI binary upgrades; this handles the plugin shipping the skills/hooks/helpers.

> Per PLAN DX-6 (row 28): the plugin must surface its own version drift visibly so vibe coders are not left on a stale plugin while CLI moves forward.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To upgrade the plugin:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start:

   **TodoWrite availability:** call TodoWrite only when the current Claude host exposes an actual TodoWrite tool in the available tool list. In Claude Desktop or any host where TodoWrite is absent, do not call TodoWrite, do not create a fallback todo message, and silently continue the workflow; do not mention progress UI availability, missing tools, omitted tools, or internal fallback behavior to the user.

   ```typescript
   TodoWrite({ todos: [
     { content: "현재 플러그인 버전 읽기",        status: "in_progress", activeForm: "현재 버전 보는 중" },
     { content: "최신 릴리즈와 비교",             status: "pending",     activeForm: "비교 진행하는 중" },
     { content: "릴리즈 노트 정리",              status: "pending",     activeForm: "노트 모으는 중" },
     { content: "업그레이드 명령 안내",            status: "pending",     activeForm: "안내 준비하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Read current plugin version.** Fetch from the manifest baked into the plugin:

   Unix / Git Bash:

   ```bash
   cat "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/plugin.json" | jq -r '.version'
   ```

   Windows PowerShell:

   ```powershell
   (Get-Content "$env:CLAUDE_PLUGIN_ROOT\.claude-plugin\plugin.json" -Raw | ConvertFrom-Json).version
   ```

   Cache the result for the session.

2. **Check helper version stamp.** The helper binary embeds the same version:

   Unix / Git Bash:

   ```bash
   HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
   [ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
   "$HELPER" version
   # → "axhub-helpers 0.1.0 (plugin v0.1.0, schema v0)"
   ```

   Windows PowerShell:

   ```powershell
   & "$env:CLAUDE_PLUGIN_ROOT\bin\axhub-helpers.exe" version
   # → "axhub-helpers 0.1.0 (plugin v0.1.0, schema v0)"
   ```

   If plugin.json version ≠ helper version, the install is corrupted; surface: "플러그인 파일이 일치하지 않아요. 재설치를 권해드려요. '/plugin install axhub@axhub --force' 라고 슬래시 명령으로 입력해주세요."

3. **Check marketplace latest.** If the marketplace is reachable, fetch latest:

   Unix / Git Bash:

   ```bash
   cat "${CLAUDE_PLUGIN_ROOT}/.claude-plugin/marketplace.json" | jq -r '.plugins[] | select(.name=="axhub") | (.latest_version // .version // empty)'
   ```

   Windows PowerShell:

   ```powershell
   $entry = ((Get-Content "$env:CLAUDE_PLUGIN_ROOT\.claude-plugin\marketplace.json" -Raw | ConvertFrom-Json).plugins | Where-Object { $_.name -eq "axhub" } | Select-Object -First 1)
   if ($entry.latest_version) { $entry.latest_version } else { $entry.version }
   ```

   On parse failure or absent entry, fall back to: "마켓플레이스 정보를 못 가져왔어요. 수동으로 확인해주세요."

4. **Compare and render Korean diff card:**

   ```
   axhub 플러그인 버전 점검:
     · 현재 설치: v<CURRENT_PLUGIN>
     · 최신 버전: v<LATEST_PLUGIN>
     · CLI 호환:  v<MIN_CLI> ~ v<MAX_CLI> 미만

   <STATE_LINE>
   ```

   `STATE_LINE`:
   - `current == latest` → "이미 최신 플러그인이에요. 업그레이드 안 받아도 돼요."
   - `current < latest` → "새 플러그인이 나왔어요. 업그레이드 권장."
   - `current > latest` → "프리뷰 버전이에요. 안정판 (v<LATEST>)으로 다운그레이드 가능해요."

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — upgrade 명령 안내 → `show` (안내만, destructive 작업 안 해요).

5. **Surface upgrade instructions (manual — Claude Code does not auto-execute plugin self-modification).** AskUserQuestion:

   ```json
   {
     "question": "플러그인 업그레이드 명령 보여줄까요?",
     "header": "업그레이드 안내",
     "options": [
       {"label": "네, 명령 보여줘", "value": "show", "description": "/plugin update 슬래시 명령 안내"},
       {"label": "릴리즈 노트 보기", "value": "notes", "description": "변경사항 자세히"},
       {"label": "지금은 그대로", "value": "skip", "description": "현재 버전 유지"}
     ]
   }
   ```

6. **On `show`.** Render the literal slash command for the user to invoke:

   > "Claude Code 채팅창에 다음 슬래시 명령을 입력해주세요:
   >
   > `/plugin update axhub@axhub`
   >
   > Claude Code 자체가 플러그인 업데이트를 처리해요. 끝나면 새 세션을 시작해주세요."

7. **Telemetry.** Append the dismissal/acceptance to `~/.cache/axhub-plugin/upgrade-prompts.ndjson` (Windows PowerShell: `$env:USERPROFILE\.cache\axhub-plugin\upgrade-prompts.ndjson`) (per row 28: "다시 묻지 않기" preference persistence) so the same nudge does not fire repeatedly within the same plugin version.

8. **Cross-link to CLI upgrade.** If the user actually meant CLI upgrade ("axhub 새 버전 있어"), redirect to `skills/update` via the Skill tool. Detection heuristic: utterance contains "CLI", "binary", "axhub 명령" → that's CLI; "plugin", "플러그인", "skill" → that's this skill.

## NEVER

- NEVER attempt to modify `${CLAUDE_PLUGIN_ROOT}` files directly — plugin self-modification is out of scope for v0.1 (recovery-flows.md "version-skew §3b" rule).
- NEVER auto-execute the slash command on the user's behalf — they must type it themselves.
- NEVER conflate plugin version with CLI version — they upgrade independently and have separate skills.
- NEVER drop the marketplace check silently — if unreachable, tell the user.

## Additional Resources

For Korean trigger lexicon (update intent — shared with CLI upgrade): `../deploy/references/nl-lexicon.md` (section 7).
For version-skew (CLI too new) flow that links to this skill: `../deploy/references/recovery-flows.md` ("version-skew §3b").
For PLAN reference: row 28 (DX-6 plugin self-upgrade nudge), §16.6 (MCP M7 placeholder spec).
