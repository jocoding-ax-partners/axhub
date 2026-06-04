---
name: routing-stats
description: '이 스킬은 사용자가 axhub plugin 의 routing 통계, 매칭 패턴, 사용 분석을 보고 싶어할 때 사용해요. 특히 Claude Desktop에서 "이번 주 axhub 라우팅 어땠어?" 같은 자연어 요청은 QA 파일 읽기나 repo audit 이 아니라 routing-stats 요약으로 처리해요. 다음 표현에서 활성화: "이번 주 axhub 라우팅 어땠어?", "라우팅 통계", "라우팅 어땠어", "routing stats", "이번 주 routing 어땠어", "지난주 매칭", "어떤 skill 많이 썼어", "axhub routing 분석", "show routing analytics", "view audit summary", "show usage analytics", 또는 routing-stats CLI 의 자연어 invocation 의도. axhub-helpers routing-stats 를 호출하고 결과를 한국어 narrative 로 변환해요.'
multi-step: false
needs-preflight: true
allows-dependency-execution: false
model: haiku
examples:
  - utterance: "이번 주 axhub 라우팅 어땠어?"
    intent: "show axhub routing statistics summary"
  - utterance: "라우팅 통계"
    intent: "show axhub routing statistics summary"
  - utterance: "이번 주 routing 어땠어"
    intent: "show axhub routing statistics summary"
  - utterance: "routing stats"
    intent: "show axhub routing statistics summary"
  - utterance: "show usage analytics"
    intent: "show axhub routing statistics summary"
  - utterance: "axhub routing 분석"
    intent: "show axhub routing statistics summary"
---

# Routing Stats

axhub plugin 의 자연어 routing 결과 통계를 한국어 narrative 로 보여줘요. audit log 7일 보관 + privacy 보장 (sha256 hash 만 저장).

## Claude Desktop contract

For ordinary Claude Desktop routing-stat questions such as `이번 주 axhub 라우팅 어땠어?`, do not read QA result files, repo files, plugin source files, `.omc`, `.claude`, git history, or local project notes.

Start with exactly:

```text
라우팅 통계를 확인할게요.
```

For Bash tool calls, set the tool `description` or title exactly:

```text
라우팅 통계 확인
```

Then run exactly one command:

```bash
axhub-helpers routing-stats --since 7d
```

Answer with a short Korean summary from stdout. Do not show raw command names, internal routing labels, file contents, English tool-title fragments, or QA-result-file findings.

## Workflow

**User-facing handoff language:** slash commands and skill names are internal routing labels. In final guidance for Claude Desktop users, prefer natural phrases the user can say, such as `다시 로그인해줘`, `프로필 전환해줘`, or `업데이트 확인해줘`; do not tell a Desktop user to type `/axhub:*` unless they explicitly ask for slash-command syntax.

To show routing stats:

**Preflight (인증/컨텍스트 확인).** 워크플로를 시작하기 전에 preflight 를 한 번 실행해서 인증 상태와 현재 team/app/env 컨텍스트를 확보해요. 첫 실행이면 Claude Code 가 `axhub-helpers preflight` 실행 허용을 물어요 — '허용' 하면 다음부터 자동으로 진행돼요.

```bash
HELPER="${CLAUDE_PLUGIN_ROOT:+$CLAUDE_PLUGIN_ROOT/bin/axhub-helpers}"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(command -v axhub-helpers 2>/dev/null)"
[ -n "$HELPER" ] && [ -x "$HELPER" ] || HELPER="$(for c in "$HOME"/.claude/plugins/cache/*/*/bin/axhub-helpers "$HOME"/.claude/plugins/cache/*/*/*/bin/axhub-helpers; do [ -x "$c" ] && printf '%s\n' "$c"; done | awk -F/ '{v=$(NF-2);split(v,a,".");printf "%010d%010d%010d\t%s\n",a[1]+0,a[2]+0,a[3]+0,$0}' | sort | tail -n1 | cut -f2-)"
PREFLIGHT_JSON=$("$HELPER" preflight --json 2>/dev/null)
[ -n "$PREFLIGHT_JSON" ] || PREFLIGHT_JSON='{}'
echo "$PREFLIGHT_JSON"
```

`auth_ok` 가 false 면 먼저 인증 상태를 설명하고, 로그인이 필요할 때는 `다시 로그인해줘`라고 말하면 된다고 안내해요. `auth_error_code` 가 있으면 자연어로 복구 안내를 붙여요: `cli_not_found`/`cli_unavailable` 는 CLI 설치 안내, `cli_config_corrupted` 는 재로그인 안내, `cli_too_old` 는 업데이트 안내. 치명적이지 않으면 워크플로를 계속 진행해요.

1. **CLI 호출.** `axhub-helpers routing-stats --since 7d --json` 호출해요.

   AXHUB_NO_AUDIT=1 환경 변수 set 일 때 audit_disabled=true JSON 반환해요. 그 경우 사용자에게 "audit log 가 비활성이에요" 안내 후 종료해요.

2. **JSON parse + 한국어 narrative.** total_prompts / axhub_related rate / auth_failed / prompt_length p50/p95 / cli_versions / top_axhub_hashes / decision_counts / ignore_rate 추출 후 다음 형식으로 요약해요:

   ```
   지난 7일 prompt {total} 개 처리했어요.
   axhub 관련: {axhub_related} ({rate}%)
   결정 타입: axhub {decision_counts.axhub} / yield {decision_counts.yield} / ignore {decision_counts.ignore} / ask {decision_counts.ask} / explicit {decision_counts.explicit}
   non-axhub ignore 율: {ignore_rate}%
   auth 실패: {auth_failed} 회
   prompt 길이 p50/p95: {p50}/{p95} bytes
   CLI 버전: {versions}
   ```

   `decision_counts` 는 공유 routing-decision 함수의 결정 타입 분포예요 (axhub/yield/ignore/ask/explicit). `ignore_rate` 는 non-axhub 프로젝트 pass-through 비율 신호예요. 결정 타입 데이터가 없으면 (legacy audit) 해당 줄은 생략해요. 상위 axhub 관련 prompt hash 가 있으면 (~5 개) 추가해요. 없으면 생략해요.

3. **Privacy 안내.** 출력 끝에 한 줄 추가해요: "audit log 는 로컬 7일 보관, 외부 전송 X. AXHUB_NO_AUDIT=1 으로 끄거나 axhub-helpers cleanup-audit --all 으로 전체 삭제 가능해요."

4. **후속 옵션 안내.** 사용자가 더 보고 싶어 하면 안내해요:
   - 상세 architecture: `docs/routing.md`
   - confused 매칭만 보기: `axhub-helpers routing-stats --confused`
   - 7일 이상 audit 정리: `axhub-helpers cleanup-audit`
   - 전체 audit 삭제: `axhub-helpers cleanup-audit --all`

   대화형 모드에서는 AskUserQuestion 으로 다음 선택지를 보여줘요:

   ```json
   {
     "question": "다음에 뭘 볼까요?",
     "header": "다음",
     "options": [
       {"label": "끝", "value": "done", "description": "요약만 보고 종료해요."},
       {"label": "상세 문서", "value": "docs", "description": "docs/routing.md 를 안내해요."},
       {"label": "confused 보기", "value": "confused", "description": "clarify 발동 hash 만 보여줘요."}
     ]
   }
   ```

## NEVER

- NEVER prompt 원문 출력 X — audit log 는 sha256 hash 만 저장해요.
- NEVER 외부 endpoint 전송 X — 모든 데이터 로컬 디스크 only 예요.

## Non-interactive AskUserQuestion guard

**Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 의 routing-stats 항목을 참조해요.

## Additional Resources

- `../../docs/routing.md` — Approach E architecture + audit schema 상세.
- `../../docs/migration-gate.md` — routing-drift CI gate.
