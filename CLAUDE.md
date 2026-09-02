<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **axhub** (1688 symbols, 1902 relationships, 10 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## When Debugging

1. `gitnexus_query({query: "<error or symptom>"})` — find execution flows related to the issue
2. `gitnexus_context({name: "<suspect function>"})` — see all callers, callees, and process participation
3. `READ gitnexus://repo/axhub/process/{processName}` — trace the full execution flow step by step
4. For regressions: `gitnexus_detect_changes({scope: "compare", base_ref: "main"})` — see what your branch changed

## When Refactoring

- **Renaming**: MUST use `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` first. Review the preview — graph edits are safe, text_search edits need manual review. Then run with `dry_run: false`.
- **Extracting/Splitting**: MUST run `gitnexus_context({name: "target"})` to see all incoming/outgoing refs, then `gitnexus_impact({target: "target", direction: "upstream"})` to find all external callers before moving code.
- After any refactor: run `gitnexus_detect_changes({scope: "all"})` to verify only expected files changed.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Tools Quick Reference

| Tool | When to use | Command |
|------|-------------|---------|
| `query` | Find code by concept | `gitnexus_query({query: "auth validation"})` |
| `context` | 360-degree view of one symbol | `gitnexus_context({name: "validateUser"})` |
| `impact` | Blast radius before editing | `gitnexus_impact({target: "X", direction: "upstream"})` |
| `detect_changes` | Pre-commit scope check | `gitnexus_detect_changes({scope: "staged"})` |
| `rename` | Safe multi-file rename | `gitnexus_rename({symbol_name: "old", new_name: "new", dry_run: true})` |
| `cypher` | Custom graph queries | `gitnexus_cypher({query: "MATCH ..."})` |

## Impact Risk Levels

| Depth | Meaning | Action |
|-------|---------|--------|
| d=1 | WILL BREAK — direct callers/importers | MUST update these |
| d=2 | LIKELY AFFECTED — indirect deps | Should test |
| d=3 | MAY NEED TESTING — transitive | Test if critical path |

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/axhub/context` | Codebase overview, check index freshness |
| `gitnexus://repo/axhub/clusters` | All functional areas |
| `gitnexus://repo/axhub/processes` | All execution flows |
| `gitnexus://repo/axhub/process/{name}` | Step-by-step execution trace |

## Self-Check Before Finishing

Before completing any code modification task, verify:
1. `gitnexus_impact` was run for all modified symbols
2. No HIGH/CRITICAL risk warnings were ignored
3. `gitnexus_detect_changes()` confirms changes match expected scope
4. All d=1 (WILL BREAK) dependents were updated

## Keeping the Index Fresh

After committing code changes, the GitNexus index becomes stale. Re-run analyze to update it:

```bash
npx gitnexus analyze
```

If the index previously included embeddings, preserve them by adding `--embeddings`:

```bash
npx gitnexus analyze --embeddings
```

To check whether embeddings exist, inspect `.gitnexus/meta.json` — the `stats.embeddings` field shows the count (0 means no embeddings). **Running analyze without `--embeddings` will delete any previously generated embeddings.**

> Claude Code users: A PostToolUse hook handles this automatically after `git commit` and `git merge`.

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->

# axhub plugin (diet 체제)

> **정책 기준 문서:** 에이전트 행동 규칙은 `docs/policy/agent-policy.md`, 개발·운영 규칙은 `docs/policy/dev-policy.md`, 사용자 공개 정책은 `POLICY.md` 가 기준이에요. 이 파일의 요약과 다르면 정책 문서를 따라요. 어긋남은 `tests/policy-parity.test.ts` 가 잡아요.

axhub plugin은 instruction-only diet 뒤 현재 **11 skill** (`onboarding` / `bootstrap` / `scaffold` / `plugins` / `deploy` / `up` / `import` / `development` / `diagnosis` / `clarity` / `update`)을 제공해요. `plugins`는 사용자 명시 결정으로 추가됐고 category=plugin인 일반 App의 게시·목록·정확한 version 다운로드를 담당해요. 게시자 관리는 App Console, 승인은 Console Review로 연결하고 판정·실행은 ax-hub-cli(`axhub`)에 둬요.

이 instruction-only diet (단일 SKILL.md 본문 + 라이브 `--help` 디스커버리 + corpus 없는 frontmatter 라우팅 + 작은 N skill) 은 외부 prior art 와 정합해요 — Supabase 의 공식 agent-skills (https://github.com/supabase/agent-skills) 도 같은 패턴(소수 skill · `--help` 디스커버리 · corpus 없는 frontmatter 라우팅)을 채택했어요. 그래서 라우팅 품질은 외부 corpus 가 아니라 frontmatter `description`·`examples` 에 투자해요.

**codex 파생 번들 (AP-20):** 소스는 claude-first 단일 트리 하나이고, OpenAI Codex CLI 용 번들 `plugins/axhub-codex` 는 `scripts/build-plugin-bundle.ts --host codex` 가 파생 생성해요 — 치환 테이블(`CODEX_SUBSTITUTIONS`, longest-first) + `codex-overrides/` 스왑(update lane·훅 프롬프트·README/POLICY codex 판) + 훅 변환(`shell` 키 제거·`commandWindows` 추가·AP-14+AP-19 합본 wrapper·상태 marker `-codex` suffix) + 매니페스트 name 재작성(axhub-codex) + description 재합성으로요. 파생 번들은 직접 수정하지 않고 소스·override 를 고쳐 재생성하며(`bun run plugin:bundle:all`), drift·FORBIDDEN·hash-pin 게이트는 `tests/codex-bundle.test.ts` 가 강제해요. codex 노출 경로는 `.agents/plugins/marketplace.json`(axhub-codex 단일 엔트리, version 생략) 이에요 — codex 는 `.agents` 를 읽으면 legacy `.claude-plugin` 을 완전히 가리고, Claude Code 는 `.agents` 를 읽지 않아요(실측). 그래서 Codex 카탈로그엔 codex 번들만, Claude 엔 `.claude-plugin` 의 axhub 만 떠요. 타 host 가 `.agents` 를 채택하는 순간 axhub 엔트리를 재병기해야 해요 — DP-8 분기 재검증 항목이에요.

## 자동 업데이트 hook

diet 가 제거한 hook 중 **auto-update SessionStart 훅 1개**만 `hooks/` 로 재도입했어요 (`hooks/hooks.json` + `hooks/auto-update-prompt.md`). Claude Code 가 `hooks/hooks.json` 을 자동 발견해요 — plugin.json 선언은 불필요해요.

- **트리거·throttle:** SessionStart 마다 cheap bash 가 `axhub` 존재 + `~/.axhub/cache/.plugin-update-check` mtime(24h)만 보고, due 면 **캐시를 훅이 직접 touch 한 뒤** `auto-update-prompt.md` 를 읽으라는 지시를 emit 해요 — throttle 기준점이 훅 발동 시점이라 에이전트가 지침을 못 따라도 재발동 스팸이 없어요 (offline 실패 시 다음 시도는 24h 뒤 — best-effort 수용). prompt 의 touch 는 이중 안전으로 유지돼요. 네트워크 호출은 hook 이 아니라 prompt(에이전트)가 해요.
- **dev 가드:** `${CLAUDE_PLUGIN_ROOT}/../../.git` 또는 `${CLAUDE_PLUGIN_ROOT}/.git` 이 있으면(레포 체크아웃 in-place 로딩·레포 루트 직접 로딩·dist 검증) auto-update 를 조용히 skip 해요 — 메인테이너 세션 노이즈 방지. end user 는 `~/.claude/plugins/cache/...` 경로라 무영향이에요.
- **CLI 업데이트:** `axhub update check --plugin-version <plugin.json version> --json` 으로 확인 → `has_update && !disabled && !is_downgrade` 면 `axhub update apply --execute --yes` 자동 적용(즉시 반영). `is_downgrade` 는 optional 필드로 부재(구 CLI)=false 취급, true(서버 롤백 배포)면 안내만 해요.
- **플러그인 업데이트:** 같은 응답의 `plugin` 블록이 `has_update` 면 `claude plugin list` 로 scope 감지 → `claude plugin marketplace update axhub` 로 cache 최신화 → `claude plugin update axhub@axhub --scope <scope>` 자동 적용 — **재시작해야 반영**돼요. marketplace 최신화가 실패해도 기존 cache 로 plugin update 를 계속 시도해요. 적용 성공 시 marker(`~/.axhub/cache/.plugin-update-restart`, 내용=받은 버전|scope)를 기록해요.
- **재시작 확인 훅 (4번째 entry):** 재시작 후 SessionStart 가 marker(7일 TTL, mtime)를 감지하면 `plugin-restart-confirm-prompt.md` 지침으로 `claude plugin list` 에서 marker 의 scope 항목(구 marker 는 enabled 최고 semver) ≥ marker 버전을 확인해 한 줄로 닫고 marker 를 삭제해요. 세션당 한 줄 + 명령 1회 상한, 미반영이면 재시작 안내 유지. dev 가드는 이 entry 에 적용하지 않아요 — marker 는 머신 전역이라 어느 세션의 확인도 유효해요.
- **끄기:** `AXHUB_NO_AUTO_UPDATE=1` 또는 marker 파일 `~/.axhub/config/no-auto-update`(모든 훅 kill switch 는 env 에서 `AXHUB_` 를 뗀 소문자-하이픈 marker counterpart 를 가져요) — 둘 중 하나면 훅은 context 를 주입하지 않고 완전히 건너뛰어요(안내 없음). 훅 bash 는 shell profile 을 소싱하지 않아 profile 에만 export 한 GUI 세션에선 env 를 못 볼 수 있어요 — 훅 레벨에선 marker 파일이 그 경우에도 닿는 채널이고, 에이전트 레벨에선 prompt/skill 의 env 분기(에이전트 shell 은 profile 소싱)가 apply 직전 방어선이에요. kill switch 는 이 2-계층이 계약이에요.
- **Windows:** hook 은 `"shell": "bash"` 로 고정했어요 — Windows 에선 Git Bash 로 돌고(없으면 silent PowerShell fallback 대신 깨끗이 skip), `bash`·`find`·`command -v`·`$HOME` 등 Git for Windows 번들 도구만 써요 (jq 같은 외부 의존 없음). prompt 의 `axhub update`/`claude plugin update` 는 에이전트 Bash 도구(= skill 들과 같은 Git Bash 경로)로 실행돼요. 즉 hook 은 skill bash 와 동일한 Git Bash 전제를 따르고, 새 의존(node 등)은 더하지 않아요.
- best-effort·비차단 — 실패·구 CLI·네트워크 오류면 조용히 건너뛰고 사용자의 작업을 막지 않아요. skill 들의 기존 `1a 버전 체크`(10분 TTL, 안내만)와 보완 관계예요.
- **수동 on-demand counterpart:** 같은 update 로직을 사용자가 직접 부르는 진입점은 `update` skill (`skills/update/SKILL.md`) 이에요 — 훅과 달리 24h throttle 없이 바로 확인하고, 최신이어도 결과를 한 줄로 알려요. 둘은 같은 `axhub update` + `claude plugin update` 표면을 공유해요.

## Windows 실행 계약 hook (AP-13)

auto-update 와 나란히 SessionStart 훅이 하나 더 있어요 (`hooks/hooks.json` 두 번째 entry). Windows(`$OS`=Windows_NT) 세션에서만 Git Bash 실행 계약을 always-on 으로 emit 해요 — axhub 명령은 Git Bash 전용(PowerShell 금지), PATH 는 `axhub plugin-support repair-path` 로 영속 등록 + 같은 세션은 bin_path 절대경로로 계속(새 터미널은 다음 세션용), `auth status` 는 로그인한 그 셸에서 검증, 로그인은 단일 폴링 1회, 설치·업데이트 안내는 install.ps1·`axhub update` 만(npm/npx 의 axhub 패키지는 이름 예약 스텁 — 금지). skill 본문이 아니라 hook 에 둔 이유는, 사고처럼 에이전트가 skill 을 안 따르고 free-form 으로 PowerShell 을 쓰는 경로까지 덮기 위해서예요 (skill byte 예산도 안 먹어요).

- hook 은 `$OS` 만 봐요 — 네트워크·`axhub` 바이너리·marker 안 건드리고, non-Windows 는 즉시 exit 0.
- **끄기:** `AXHUB_NO_WINDOWS_CONTRACT=1` 또는 `~/.axhub/config/no-windows-contract`. Windows 전제는 다른 훅과 동일해요 (`"shell": "bash"`, Git Bash 번들 도구만).
- 규칙 본체는 `docs/policy/agent-policy.md` 의 AP-13 이 소유해요 (parity 적용: `hooks/session-windows-contract.sh`, `CLAUDE.md`).

## update-first Code-mode router hook (AP-14)

SessionStart fallback 과 UserPromptSubmit match 가 최신·버전·업데이트 요청에만 라우팅 문맥을 추가해요. Claude Desktop Code 모드에서 `Finding tools`, 전역 `App list (axhub)` 같은 App/MCP 도구, 일반 shell probe 가 먼저 잡히는 것을 막기 위한 guard 예요.

- SessionStart fallback 은 새 Code 세션에 update-first 규칙을 먼저 깔아요. UserPromptSubmit match(`hooks/update-router.sh`)는 훅 입력 JSON 전체가 아니라 **사용자 프롬프트(`"prompt":` 필드 이후 구간)에** `axhub`(대소문자 4-변형)와 최신성 키워드(`최신`, `버전`, `업데이트`, `latest`, `up to date`, `version check`, `update`, `upgrade`)가 함께 있을 때만 `hookSpecificOutput.additionalContext` 를 emit 하고, `"prompt":` 키 부재 시 fail-closed 로 침묵해요. 모든 훅 출력은 `suppressOutput: true` JSON 이라 사용자 화면에 노출되지 않아요 — `systemMessage` 와 plain stdout 은 쓰지 않아요.
- 명령 실행·네트워크·앱 목록 조회를 하지 않아요. `update` 스킬 우선, 첫 visible assistant text `현재 버전을 확인할게요.`, App/MCP 도구 선행 금지만 주입해요.
- update 뒤 같은 원문에 앱 현황 확인이 남으면 존재하지 않는 `axhub app list` 단수 명령을 추측하지 않고 `axhub apps --help` 로 plural 표면을 확인한 뒤 정확히 `axhub apps list --json` 읽기 전용 명령으로 시작해요. `| head`, `2>/dev/null`, `grep`, `sed`, `awk` 같은 shell 후처리는 붙이지 않아요.
- **끄기:** `AXHUB_NO_UPDATE_ROUTER=1` 또는 `~/.axhub/config/no-update-router`.

## 예상 밖 CLI 실패 자동 리포트 hook (AP-19)

다섯 번째 SessionStart entry 는 axhub CLI 가 설치된 세션에 실패 자동 리포트 계약을 always-on 으로 emit 해요. axhub 명령이 문서화된 계약 밖으로 실패하면(panic·비정상 `--json` 출력·문서화된 happy path 의 exit 1/7·재시도에도 재현되는 timeout·help 와 실제 동작 불일치) 실패 직후 `axhub feedback -m <실행한 것·기대한 것·실제 결과>` 1회를 조용히 실행해요 — 실패 직전 명령의 진단(명령 경로·플래그 이름만, 값 미수집, TTL 10분)은 CLI 가 자동 첨부하고, 이슈 생성·중복 병합·제목/라벨은 backend 가 처리해요 (ax-hub-cli 의 `POST /api/v1/feedback` 표면). 예상된 거절은 리포트하지 않아요 — exit 4(미로그인)·exit 64(사용법)·정상 가드 거절(5/6/9/11/12/13/66)·배포/앱 빌드 실패(diagnosis 대상)는 제외이고, `axhub feedback` 자기 실패(명령 없는 구 CLI·backend 미배포·네트워크 오류)는 재시도·언급 없이 버려요 — 이 조용한 실패가 곧 가용성 게이트라 별도 버전 probe 를 두지 않아요. skill 본문이 아니라 hook 에 둔 이유는 AP-13 과 같아요 — free-form 실행 경로까지 덮고 skill byte 예산을 안 먹어요.

- hook 은 CLI 존재만 봐요 (AP-17 의 3-경로: `command -v axhub` → `~/.axhub/bin-path` → canonical `~/.axhub/bin/axhub`(.exe)) — 네트워크·바이너리 실행·marker 접촉 없음, 못 찾으면 즉시 exit 0.
- **끄기:** `AXHUB_NO_FEEDBACK_REPORT=1` 또는 `~/.axhub/config/no-feedback-report`.
- 규칙 본체는 `docs/policy/agent-policy.md` 의 AP-19 가 소유해요 (parity 적용: `hooks/session-feedback-contract.sh`, `CLAUDE.md`, `POLICY.md`). 사용자 공개는 `POLICY.md` 의 "실패 자동 리포트" 섹션이 담당해요.

## CLI 호출 표면

- skill 들은 흡수된 helper 표면을 `axhub plugin-support <cmd>` (hidden 그룹) 로 호출해요 (`clarity` skill 은 예외 — 공개 표면만 탐색·실행, `diagnosis` skill 은 CLI 전용 — MCP `deployment_diagnosis` 가 보여도 호출하지 않고 공개 `axhub deploy status`/`deploy logs`/`deploy diagnose` 만 써요) — 예: `axhub plugin-support onboarding-detect`, `axhub plugin-support preflight`, `axhub plugin-support deploy-prep`. hidden 명령은 외부 무보증이지만 계약 parity 테스트 + 최소 CLI 버전 게이트로 plugin 과 동기화돼요.
- 사용자 가치가 있는 공개 표면은 `axhub deploy verify <deployment-id> --app <app>`, `axhub deploy diagnose`, `axhub plugin publish <path>`, 그리고 onboarding의 AI 활용 기록 옵트인이 쓰는 `axhub axrouter`(status/monitor/consent — 실패 시 조용히 건너뛰는 fail-open)예요.

## 최소 CLI 버전 게이트

- bootstrap·deploy skill은 `plugin-support` 기능(preflight)으로 최소 표면 **0.21.3+**를 확인하고, scaffold는 **0.30.0+**를 확인해요. plugins는 version을 추측하지 않고 필요한 `axhub plugin list|download|publish --help` 성공을 가드해요. CLI가 없거나 기능이 없으면 멈추고 update로 보내며 우회하지 않아요.
- **CLI 경로 해석 (AP-17):** bare `axhub` 실패는 미설치가 아니에요 — 부모 앱(Claude Desktop·VS Code·터미널 앱)이 물려준 낡은 PATH 때문에 설치된 CLI 를 못 찾는 상태가 macOS·Linux·Windows 모두에서 흔해요 (AP-13 은 Windows 전용이라 이 상태를 덮지 못해요). 모든 skill 의 CLI 가드는 `command -v axhub` → 위치 파일 `~/.axhub/bin-path` → canonical `~/.axhub/bin/axhub`(.exe) 순으로 찾고, 디스크에 있으면 재설치·온보딩으로 돌리지 않고 그 절대경로로 `plugin-support repair-path --json` 을 실행해 영속 PATH 를 고친 뒤 같은 세션은 반환된 `bin_path` 절대경로로 이어가요. 세 경로 모두에서 못 찾을 때만 onboarding 을 안내해요.

## 살아남은 quality gate

- `bun run lint:tone --strict` — 모든 한글 텍스트 해요체 0 err (금지: 합니다 / 입니다 / 드립니다 / 당신).
- frontmatter validity check — 11 skill의 SKILL.md frontmatter 유효성.
- 대표 여정 회귀 — 첫 셋업 → 앱 생성 → 배포 → 상태 확인 경로를 문서·skill 본문·fixture 계약으로 같은 방향에 맞춰요.
- `bun run plugin:bundle` — `.claude`, `.omx`, `node_modules`, 인덱스 DB 같은 개발 산출물이 빠진 clean local plugin bundle 을 만들어요. 로컬 Claude Code 검증은 repo 루트가 아니라 `dist/axhub-plugin` 을 써요.
- 실제 ax-hub-cli 구현/schema parity/release 는 이 plugin repo 범위 밖 follow-up 으로 남겨요.

## Release flow (commit-and-tag-version, 단순화)

plugin 릴리즈는 `commit-and-tag-version` 기반 3단계 flow 를 유지하되 postbump 이 단순해졌어요 (codegen:version·release:check·5-binary build·bin/ add 전부 제거).

```bash
# step 1 — bump + commit (tag 미생성)
bun run release
# step 2 — CHANGELOG narrative (해요체 1-3 문장) 추가 후 amend
git commit --amend --no-edit -a
# step 3 — tag 생성 + push
bun run release:tag
```

## deploy 성공 선언 규칙

- deploy 성공 선언은 `axhub deploy verify <deployment-id> --app <app>` **1회 실행으로만** 해요. deployment id 와 app scope 는 필수이고 latest 재탐색 경로는 금지예요 — verify exit 0 + 접근 가능 URL 확인 전까지 "배포 성공" 이라고 말하지 않아요.
- **static 앱(deploy_method=static) 배포는 별도 lane** 이에요: 성공 선언을 `apps static deploy --execute` 의 `active_release_id`(activate 성공)로 해요 — static 은 deployment-record 가 아니라 release 라 `deploy verify` 가 404 예요. 위 verify 규칙은 deployment-record 배포(docker/compose)에만 적용돼요. deploy skill 이 resolve 직후 `apps get` 의 `deploy_method` 로 auto-detect 해 이 lane 으로 갈라요.
- **스테이징 옵트인 앱(AP-25):** `apps get` 의 `staging_enabled=true` 면 배포는 스테이징에만 반영되고 운영은 심사 승인 뒤 promote 로만 바뀌어요. verify 성공 요약은 `deploy verify --json` 의 `environment` 로 갈라 `staging` 이면 운영 미반영과 다음 단계(심사 신청 `axhub publish --app <app> --deployment-id <id> --execute`)를 함께 안내하고 "운영 반영 완료" 라고 말하지 않아요. preview 카드 환경 라벨도 `staging_enabled` 로 `운영`/`스테이징` 을 골라요. `environment` 가 없거나 null 이면 운영으로 간주하지 않아요.

## Skill routing

이 repo의 공개 plugin surface는 `onboarding` / `bootstrap` / `scaffold` / `plugins` / `deploy` / `up` / `import` / `development` / `diagnosis` / `clarity` / `update` 열한 스킬이에요.

Key routing rules:
- 처음 셋업·CLI 설치·로그인·환경 점검 → `onboarding`
- 빈 디렉토리 새 앱 생성·템플릿·bootstrap saga → `bootstrap`
- 사용자 소유 GitHub 저장소에서 새 앱 시작 → `scaffold`
- category=plugin App 게시·목록·정확한 version 다운로드 → `plugins` (Discovery/App Console/Console Review canonical surface)
- 비어 있지 않은 기존 로컬 앱의 첫 axhub 연결·첫 배포 가져오기 → `import`
- 배포 실행·preview-confirm·verify 기반 성공 선언 → `deploy` (static 앱은 deploy_method auto-detect 로 독립 static lane: dry-run→`--execute`→`active_release_id` 성공 선언)
- GitHub 저장소 없이 지금 폴더의 소스를 그대로 올려 배포 → `up` (커밋 상태를 게이트로 쓰지 않고 `deploy-prep` preflight + `axhub up --dry-run` preview 사용, static 앱은 deploy 로 양보)
- 기존 앱에 실데이터 기반 기능(페이지·화면·대시보드·조회 엔드포인트·CRUD 화면) 코드 생성 → `development` (AP-5 read 기본, write 게이트 — 조회 도구는 읽기 전용이고 커넥터 자체는 쓰기 액션을 가질 수 있어요)
- 배포 실패 원인 진단·해결 후보 요약 → `diagnosis` (읽기 전용, 재배포·롤백 직접 실행 금지)
- axhub CLI·플러그인을 지금 최신 버전으로 업데이트(수동 on-demand) → `update`
- axhub CLI 운영 명령(테이블/컬럼 생성·환경변수·로그·connector 연결·권한 확인/부여(대상별 read|write)·데이터 조회·롤백·상태)·모호한 axhub 발화 → `clarity` (axhub 명령 실행만, 버전 업데이트는 update·앱 코드 생성은 development·기존 앱 가져오기는 import·배포 실패 원인 진단은 diagnosis 양보)

**모든 트리거 전제 (AP-11):** 위 규칙은 axhub 맥락(대화의 axhub 언급·현재 폴더의 axhub 연결·직전 axhub 작업)이 있을 때만 유효해요. 맥락 없는 일반 발화("배포해"·"업데이트해줘"·"로그 보여줘")는 실행·안내로 밀어붙이지 않고 한 번 묻거나 종료하며 다른 axhub 스킬로 넘기지 않아요. bootstrap 은 preview-confirm 승인이 backstop 이라 frontmatter 게이트로만 적용해요.

**앱 git backend 선판정 (AP-23):** resume/existing은 public `axhub apps get <app> --json`, fresh bootstrap/onboarding은 read-only `axhub apps git-backend --tenant <tenant> --json`의 top-level `git_backend`만 판정 입력으로 써요. selfhosted는 device flow·GitHub App 설치 대사를 노출하지 않고, non-static deploy는 `axhub repo clone` 뒤 일반 `git push`의 webhook deployment를 기다려요. static은 기존 release lane, GitHub·`legacy_github`는 기존 gate/upload/create 경로를 유지하며 C1/Gitea API를 직접 호출하지 않아요.

**codex 질문 프로토콜·fail-closed (AP-12 codex 판):** codex 판은 선택지를 번호 메뉴로 출력하지 않아요 — 한 문장 확인형 + 추천안 `(추천)` + 답→행동 매핑을 쓰고, 질문한 턴은 도구 호출 없이 끝내요. 네이티브 선택 카드(`[features] default_mode_request_user_input`)가 켜진 세션에서 **빈 답변은 미승인**이고 카드가 열려 있는 동안에는 실행 단계로 넘어가지 않아요. 이 규칙은 세 채널이 겹쳐 소유해요 — always-on 합본 훅(kill switch `AXHUB_NO_QUESTION_PROTOCOL` / `~/.axhub/config/no-question-protocol`), AP-12 정책, 그리고 실행 7스킬 본문의 첫 8,000B. 훅은 미신뢰 세션에서 조용히 꺼지고 본문은 8,000B에서 절단되니 실패 모드가 서로 달라요.

**진입 확인 AUQ (AP-12):** axhub 프로젝트가 확정돼도 배포·생성·가져오기(deploy·bootstrap·import) 실행 전에 interactive 에서는 "axhub로 진행할까요?"를 AskUserQuestion 으로 한 번 더 확인해요("무엇을·어떻게"를 묻는 기존 preview 승인과 별개인 진입 게이트). deploy·import 는 preview 앞 별도 AUQ, bootstrap 은 기존 preview 승인에 통합(byte 예산 포화). headless 는 생략해요.

**Windows 실행 계약 (AP-13):** Windows 에선 axhub 명령을 Git Bash 전용으로 실행해요 (PowerShell 금지). PATH 는 수동 등록 대신 canonical 경로(`~/.axhub/bin/axhub`(.exe))로 `plugin-support repair-path` 를 실행해 영속 등록하고 같은 세션은 repair-path 의 bin_path 절대경로로 계속 진행해요(구 CLI 로 bin_path 가 없으면 새 터미널 안내, 새 터미널은 다음 세션용), `auth status` 는 `auth login` 한 그 셸에서 검증해요 — HOME 없는 PowerShell 의 "미로그인" 은 실패가 아니에요. 로그인은 단일 폴링 `axhub auth login --json` 1 회로 하고 background 재실행은 안 해요. 이 계약에 예외는 없어요 — 공식 설치 채널은 install.sh / install.ps1 뿐이고, npm/npx 의 `axhub`·`axhub-cli` 패키지는 이름 예약 스텁이라 설치·실행·안내에 쓰지 않아요.
