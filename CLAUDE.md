<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **axhub** (330 symbols, 344 relationships, 1 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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

axhub plugin 은 45 skill 체제에서 **4 skill 체제**로 다이어트했어요: `onboarding` / `init` / `deploy` + 그 셋에 명확히 안 맞거나 의도가 불분명한 axhub 발화를 라이브 `--help` 탐색으로 처리하는 `clarity` 브리지예요. 이후 기존 앱에 실데이터 기반 기능 코드를 생성하는 `development` skill, 비어 있지 않은 기존 로컬 앱을 axhub로 가져오는 `import` skill, CLI·플러그인을 지금 최신으로 올리는 수동 on-demand `update` skill, 배포 실패 원인을 읽기 전용으로 요약하는 `diagnosis` skill 을 더해 현재 8 skill 이에요. 판정·실행 로직은 plugin 안에 두지 않고 ax-hub-cli (`axhub` 바이너리) 를 직접 호출해요. Rust helper 바이너리 (`crates/axhub-helpers`), 모든 hook(이후 auto-update SessionStart 훅 1개만 `hooks/` 로 재도입 — 아래 "자동 업데이트 hook" 참고), NL routing corpus, scaffold/skill-doctor/lint:keywords 인프라, cosign 멀티-바이너리 릴리즈 파이프라인은 전부 제거됐어요.

이 instruction-only diet (단일 SKILL.md 본문 + 라이브 `--help` 디스커버리 + corpus 없는 frontmatter 라우팅 + 작은 N skill) 은 외부 prior art 와 정합해요 — Supabase 의 공식 agent-skills (https://github.com/supabase/agent-skills) 도 같은 패턴(소수 skill · `--help` 디스커버리 · corpus 없는 frontmatter 라우팅)을 채택했어요. 그래서 라우팅 품질은 외부 corpus 가 아니라 frontmatter `description`·`examples` 에 투자해요.

## 자동 업데이트 hook

diet 가 제거한 hook 중 **auto-update SessionStart 훅 1개**만 `hooks/` 로 재도입했어요 (`hooks/hooks.json` + `hooks/auto-update-prompt.md`). Claude Code 가 `hooks/hooks.json` 을 자동 발견해요 — plugin.json 선언은 불필요해요.

- **트리거·throttle:** SessionStart 마다 cheap bash 가 `axhub` 존재 + `~/.axhub/cache/.plugin-update-check` mtime(24h)만 보고, due 면 `auto-update-prompt.md` 를 읽으라는 지시를 emit 해요. 네트워크 호출은 hook 이 아니라 prompt(에이전트)가 해요.
- **CLI 업데이트:** `axhub update check --plugin-version <plugin.json version> --json` 으로 확인 → `has_update && !disabled` 면 `axhub update apply -y` 자동 적용(즉시 반영).
- **플러그인 업데이트:** 같은 응답의 `plugin` 블록이 `has_update` 면 `claude plugin list` 로 scope 감지 후 `claude plugin update axhub@axhub --scope <scope>` 자동 적용 — **재시작해야 반영**돼요.
- **끄기:** `AXHUB_NO_AUTO_UPDATE=1` 이면 자동 적용 없이 안내만 하고 throttle 도 즉시 skip 해요.
- **Windows:** hook 은 `"shell": "bash"` 로 고정했어요 — Windows 에선 Git Bash 로 돌고(없으면 silent PowerShell fallback 대신 깨끗이 skip), `bash`·`find`·`command -v`·`$HOME` 등 Git for Windows 번들 도구만 써요 (jq 같은 외부 의존 없음). prompt 의 `axhub update`/`claude plugin update` 는 에이전트 Bash 도구(= skill 들과 같은 Git Bash 경로)로 실행돼요. 즉 hook 은 skill bash 와 동일한 Git Bash 전제를 따르고, 새 의존(node 등)은 더하지 않아요.
- best-effort·비차단 — 실패·구 CLI·네트워크 오류면 조용히 건너뛰고 사용자의 작업을 막지 않아요. skill 들의 기존 `1a 버전 체크`(10분 TTL, 안내만)와 보완 관계예요.
- **수동 on-demand counterpart:** 같은 update 로직을 사용자가 직접 부르는 진입점은 `update` skill (`skills/update/SKILL.md`) 이에요 — 훅과 달리 24h throttle 없이 바로 확인하고, 최신이어도 결과를 한 줄로 알려요. 둘은 같은 `axhub update` + `claude plugin update` 표면을 공유해요.

## CLI 호출 표면

- skill 들은 흡수된 helper 표면을 `axhub plugin-support <cmd>` (hidden 그룹) 로 호출해요 (`clarity` skill 은 예외 — 공개 표면만 탐색·실행, `diagnosis` skill 은 MCP `deployment_diagnosis` 우선, 없으면 공개 `axhub deploy diagnose`) — 예: `axhub plugin-support onboarding-detect`, `axhub plugin-support preflight`, `axhub plugin-support deploy-prep`. hidden 명령은 외부 무보증이지만 계약 parity 테스트 + 최소 CLI 버전 게이트로 plugin 과 동기화돼요.
- 사용자 가치가 있는 공개 표면은 `axhub deploy verify <deployment-id>` 와 `axhub deploy diagnose` 예요.

## 최소 CLI 버전 게이트

- init·deploy skill 은 **시작 시 `axhub` 존재와 `plugin-support` 기능(preflight) 가드** 로 최소 표면 (흡수 릴리즈 = **0.20.0+**) 을 확인해요. CLI 가 없거나 너무 낮으면 skill 은 멈추고 설치/업그레이드를 안내해요 — 절대 우회하지 않아요.

## 살아남은 quality gate

- `bun run lint:tone --strict` — 모든 한글 텍스트 해요체 0 err (금지: 합니다 / 입니다 / 드립니다 / 당신).
- frontmatter validity check — 8 skill 의 SKILL.md frontmatter 유효성.
- 대표 여정 회귀 — 첫 셋업 → 앱 생성 → 배포 → 상태 확인 경로를 문서·skill 본문·fixture 계약으로 같은 방향에 맞춰요.
- 실제 ax-hub-cli 구현/schema parity/release 는 이 plugin repo 범위 밖 follow-up 으로 남겨요.

## Release flow (commit-and-tag-version, 단순화)

plugin 릴리즈는 `commit-and-tag-version` 기반 2단계 flow 를 유지하되 postbump 이 단순해졌어요 (codegen:version·release:check·5-binary build·bin/ add 전부 제거).

```bash
# step 1 — bump + commit (tag 미생성)
bun run release
# step 2 — CHANGELOG narrative (해요체 1-3 문장) 추가 후 amend
git commit --amend --no-edit -a
# step 3 — tag 생성 + push
bun run release:tag
```

## deploy 성공 선언 규칙

- deploy 성공 선언은 `axhub deploy verify <deployment-id>` **1회 실행으로만** 해요. deployment id 인자는 필수이고 latest 재탐색 경로는 금지예요 — verify exit 0 + 접근 가능 URL 확인 전까지 "배포 성공" 이라고 말하지 않아요.
- **static 앱(deploy_method=static) 배포는 별도 lane** 이에요: 성공 선언을 `apps static deploy --execute` 의 `active_release_id`(activate 성공)로 해요 — static 은 deployment-record 가 아니라 release 라 `deploy verify` 가 404 예요. 위 verify 규칙은 deployment-record 배포(docker/compose)에만 적용돼요. deploy skill 이 resolve 직후 `apps get` 의 `deploy_method` 로 auto-detect 해 이 lane 으로 갈라요.

## Skill routing

이 repo 의 공개 plugin surface 는 `onboarding` / `init` / `deploy` / `import` / `development` / `diagnosis` / `clarity` / `update` 여덟 스킬이에요.

Key routing rules:
- 처음 셋업·CLI 설치·로그인·환경 점검 → `onboarding`
- 빈 디렉토리 새 앱 생성·템플릿·bootstrap saga → `init`
- 비어 있지 않은 기존 로컬 앱의 첫 axhub 연결·첫 배포 가져오기 → `import`
- 배포 실행·preview-confirm·verify 기반 성공 선언 → `deploy` (static 앱은 deploy_method auto-detect 로 독립 static lane: dry-run→`--execute`→`active_release_id` 성공 선언)
- 기존 앱에 실데이터 기반 기능(페이지·화면·대시보드·조회 엔드포인트·CRUD 화면) 코드 생성 → `development` (read 전용 v1)
- 배포 실패 원인 진단·해결 후보 요약 → `diagnosis` (읽기 전용, 재배포·롤백 직접 실행 금지)
- axhub CLI·플러그인을 지금 최신 버전으로 업데이트(수동 on-demand) → `update`
- axhub CLI 운영 명령(테이블/컬럼 생성·환경변수·로그·connector 연결·데이터 조회·롤백·상태)·모호한 axhub 발화 → `clarity` (axhub 명령 실행만, 버전 업데이트는 update·앱 코드 생성은 development·기존 앱 가져오기는 import·배포 실패 원인 진단은 diagnosis 양보)
