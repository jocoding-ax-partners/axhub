---
name: update
description: '이 스킬은 사용자가 axhub CLI 업데이트를 확인하거나 적용하고 싶어할 때 사용합니다. 다음 표현에서 활성화: "버전 확인", "새 버전", "새 버전 나왔어", "새 버전 받아", "업그레이드", "업그레이드해", "업데이트", "업데이트 있어", "업데이트 적용해주세요", "업데이트 확인해", "업데이트해", "최신", "최신 버전으로 올려주세요", "최신으로 올려", "최신이야", "axhub 새 버전 있어", "axhub 업그레이드 부탁드려요", "brew upgrade 해줘", "CLI 업데이트 부탁드려요", "brew upgrade", "check version", "latest", "new release", "update", "update available", "upgrade", "version", 또는 axhub CLI 버전 관리 요청. PLAN §16.10 에 따라 cosign 서명 검증을 기본 enforce 합니다.'
examples:
  - utterance: "axhub 새 버전 있어?"
    intent: "update axhub CLI"
  - utterance: "update check 해주세요"
    intent: "update axhub CLI"
  - utterance: "upgrade axhub"
    intent: "update axhub CLI"
  - utterance: "brew upgrade"
    intent: "update axhub CLI"
  - utterance: "버전 확인"
    intent: "update axhub CLI"
multi-step: true
needs-preflight: false
allows-dependency-execution: false
model: sonnet
---

# Update (CLI version check + apply)

Check and apply axhub CLI updates with cosign signature verification mandatory by default (PLAN §16.10 / row 59 / row 47).

## Workflow

To handle updates:

0. **Render TodoWrite checklist (vibe coder sees real-time progress).** Call TodoWrite at workflow start:

   ```typescript
   TodoWrite({ todos: [
     { content: "현재 / 최신 버전 비교",        status: "in_progress", activeForm: "버전 확인하는 중" },
     { content: "릴리즈 노트 요약",             status: "pending",     activeForm: "변경사항 정리하는 중" },
     { content: "동의 받고 cosign 검증",         status: "pending",     activeForm: "서명 검증하는 중" },
     { content: "binary 교체",                  status: "pending",     activeForm: "교체 진행하는 중" },
     { content: "결과 안내",                    status: "pending",     activeForm: "마무리하는 중" }
   ]})
   ```

   **TodoWrite status sync:** after every workflow step and after every AskUserQuestion answer, call TodoWrite again with the full current todos array. Mark finished items as `"completed"`, the active item as `"in_progress"`, and untouched items as `"pending"`. Do not leave the initial Step 0 list stale after commands, user answers, or final result.

   **워크플로를 마치면 (마지막 결과 출력 직후) TodoWrite 를 한 번 더 호출해서 모든 todo 를 `"completed"` 로 만들어요.** `in_progress` / `pending` 이 하나라도 남으면 다음 SKILL 이 시작될 때 이 SKILL 의 미완료 todo 가 화면에 그대로 남아 버그처럼 보여요. 종료 시점에 미완료 todo 가 0 개여야 해요.

   각 step 가 끝날 때마다 해당 todo 의 `status` 를 `"completed"` 로 update 해요.

1. **Check for update.** Run `axhub update check --json`:

   ```bash
   axhub update check --json
   ```

   응답을 파싱해요: `{"current": "v0.17.2", "latest": "v0.18.0", "has_update": true}` 또는 `{"has_update": false}`. `has_update: false` 면 이미 최신이에요.

2. **On `has_update: false`.** Tell the user "이미 최신 버전이에요 (v<CURRENT>). 업데이트 안 받아도 돼요." and stop.

3. **On `has_update: true`.** Render Korean upgrade card:

   ```
   새 버전이 나왔어요.
     · 현재 버전: v<CURRENT>
     · 새 버전:   v<LATEST>
     · 변경사항:  <RELEASE_NOTES_FIRST_LINE>

   업데이트할래요?
   ```

   **Non-interactive AskUserQuestion guard (D1):** 이 SKILL 의 모든 AskUserQuestion 호출은 대화형 모드를 가정해요. `if ! [ -t 1 ] || [ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 인 subprocess (`claude -p`, CI, headless) 에서는 AskUserQuestion 호출을 건너뛰고 안전한 기본값으로 진행해요. 기본값은 `tests/fixtures/ask-defaults/registry.json` 참조 — apply 확인 → `skip` (binary 교체는 subprocess 에서 자동 안 해요).

   AskUserQuestion:

   ```json
   {
     "question": "axhub CLI 업그레이드해요?",
     "header": "업그레이드",
     "options": [
       {"label": "네, 업그레이드", "value": "apply", "description": "안전 검증 후 업데이트해요"},
       {"label": "릴리즈 노트 보기", "value": "notes", "description": "변경사항 자세히 보기"},
       {"label": "지금은 그대로", "value": "skip", "description": "현재 버전 유지"}
     ]
   }
   ```

4. **On `apply`.** Preview the apply plan first:

   ```bash
   axhub update apply --dry-run --json
   ```

   preview JSON 을 보여줘요 — `current`/`latest`, `is_downgrade`(true 면 execute 에 `--force` 필요), `feed_base`, `next_step`. 그다음 명시적 동의를 받아요:

   ```json
   {
     "question": "지금 업데이트 적용할래요?",
     "header": "업데이트 적용",
     "options": [
       {"label": "적용", "value": "적용", "description": "binary 교체를 진행해요"},
       {"label": "취소", "value": "취소", "description": "지금은 하지 않아요"}
     ]
   }
   ```

   **Non-interactive guard (D1):** `[ -n "$CI" ] || [ -n "$CLAUDE_NON_INTERACTIVE" ]` 면 → 기본값 "취소" (registry key: `update.apply_consent`).

   **On "적용":** cosign 은 기본 enforce 라 env 접두 없이 실행해요:

   ```bash
   axhub update apply --execute --yes --json
   ```

   CLI 가 swap 전에 서명을 검증해요. exit 0 (`{"applied":true, "install_kind":"self_replace", "current":..., "latest":..., "binary":"<path>"}`) 이면: "업그레이드 완료! v<LATEST> 깔렸어요 (`<binary>`). 새 터미널을 열거나 `axhub --version` 으로 확인하면 돼요." 라고 안내해요.

   exit 0 이 아니면 아래 종료 코드별로 분기해요 (계약: `ax-hub-cli/docs/cli-exit-codes.md`).

5. **exit 14 (digest mismatch).** 다운로드 파일 SHA256 이 릴리스 매니페스트 핀과 안 맞아요 — **변조 신호**. 즉시 중단하고 `--force` 로도 진행하지 말아요. 사용자에게 안내해요:

   > "보안 검증 실패(파일 변조 가능성). 절대 강제 진행하지 말고 회사 IT/보안팀에 즉시 알려주세요. axhub 는 현재 버전으로 계속 써도 돼요."

6. **exit 15 (swap failed).** 원자적 바이너리 교체가 실패했어요. **자동 재시도하지 말아요** (바이너리가 부분 교체됐을 수 있어요). `~/.axhub/bin/axhub.<old>.bak` 백업이 있으면 복구에 쓸 수 있어요. `/axhub:doctor` 로 라우팅해요.

7. **exit 66 — `error.subcode` 로 분기해요.**

   - `update.downgrade_blocked`: 구버전 설치가 차단됐어요. 사용자가 다운그레이드를 원하면 `axhub update apply --execute --force --yes --json` 를 안내해요 — `--force` 는 다운그레이드 게이트만 풀고 **cosign 검증은 그대로 유지**해요.
   - `update.cosign_enforce_failed`: cosign enforce 실패예요. **하드 스톱**, 어떤 우회도 제시하지 말아요. `../deploy/references/error-empathy-catalog.md` 의 cosign 항목으로 라우팅하고 안내해요:

     > "보안 검증 실패. 절대 강제 진행하지 말고 회사 IT/보안팀에 즉시 알려주세요. axhub 는 현재 버전으로 계속 써도 돼요."

8. **exit 4 (미인증).** 토큰 문제예요. `axhub auth login` 으로 유도해요.

9. **그 외 비정상 종료 (1 / 10 / 64).** `../deploy/references/error-empathy-catalog.md` 로 코드별 라우팅해요. (10 타임아웃: apply 전송 실패는 자동 재시도하지 말아요 / 64: 인자 교정 / 1: 일반 오류.)

## command coverage (현 CLI 기준)

`apply` 전 preview 에서 보여줄 것:

- 서명 검증 모드 (기본 cosign enforce — env 토글 없음)
- 대상 버전 + 설치 경로 (`~/.axhub/bin`)
- `is_downgrade` 여부 (true 면 execute 에 `--force` 필요)
- rollback 가능 여부 (`axhub.<old>.bak`)

현 CLI 가 지원하는 명령만 써요:

```bash
axhub update check --json
axhub update apply --dry-run --json
axhub update apply --execute --yes --json
axhub update apply --execute --force --yes --json   # 다운그레이드 시에만 (cosign 은 그대로 유지)
```

존재하지 않는 unsigned bypass env 는 언급하지 말아요.

## NEVER

- NEVER apply 호출에 존재하지 않는 env 접두를 붙이기 — `AXHUB_REQUIRE_COSIGN` / `AXHUB_ALLOW_UNSIGNED` / `AXHUB_DISABLE_AUTOUPDATE` 는 CLI 에 없어요. cosign 은 기본 enforce 라 env 불필요.
- NEVER `--force` 를 cosign 우회로 제시 — `--force` 는 다운그레이드 게이트만 풀고 서명 검증은 절대 안 풀어요.
- NEVER exit 14 (digest mismatch) 나 exit 66 `update.cosign_enforce_failed` 를 지나쳐 계속 진행.
- NEVER `axhub update apply` 를 명시적 AskUserQuestion 확인 없이 호출.
- NEVER `update apply` 전송 실패(exit 10 / 15)에 자동 재시도 (바이너리가 부분 교체됐을 수 있어요).
- NEVER `axhub update apply --execute` 를 `axhub update apply --dry-run --json` preview 없이 호출.

## Additional Resources

For Korean trigger lexicon (update intent): `../deploy/references/nl-lexicon.md` (sections 7a/7b).
exit 66 cosign / digest 템플릿: `../deploy/references/error-empathy-catalog.md`.
종료 코드 계약: `ax-hub-cli/docs/cli-exit-codes.md` (0/1/4/10/14/15/64/66, exit 66 subcode = `update.downgrade_blocked` / `update.cosign_enforce_failed`).
