<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **axhub** (271 symbols, 280 relationships, 1 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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

axhub plugin 은 45 skill 체제에서 **4 skill 체제**로 다이어트했어요: `onboarding` / `init` / `deploy` + 그 셋에 명확히 안 맞거나 의도가 불분명한 axhub 발화를 라이브 `--help` 탐색으로 처리하는 `clarity` 브리지예요. 판정·실행 로직은 plugin 안에 두지 않고 ax-hub-cli (`axhub` 바이너리) 를 직접 호출해요. Rust helper 바이너리 (`crates/axhub-helpers`), 모든 hook, NL routing corpus, scaffold/skill-doctor/lint:keywords 인프라, cosign 멀티-바이너리 릴리즈 파이프라인은 전부 제거됐어요.

## CLI 호출 표면

- skill 들은 흡수된 helper 표면을 `axhub plugin-support <cmd>` (hidden 그룹) 로 호출해요 (`clarity` skill 은 예외 — 공개 표면만 탐색·실행) — 예: `axhub plugin-support onboarding-detect`, `axhub plugin-support preflight`, `axhub plugin-support deploy-prep`. hidden 명령은 외부 무보증이지만 계약 parity 테스트 + 최소 CLI 버전 게이트로 plugin 과 동기화돼요.
- 사용자 가치가 있는 검증 표면만 **공개** 이에요: `axhub deploy verify <deployment-id>`.

## 최소 CLI 버전 게이트

- init·deploy skill 은 **시작 시 `axhub` 존재와 `plugin-support` 기능(preflight) 가드** 로 최소 표면 (흡수 릴리즈 = **0.20.0+**) 을 확인해요. CLI 가 없거나 너무 낮으면 skill 은 멈추고 설치/업그레이드를 안내해요 — 절대 우회하지 않아요.

## 살아남은 quality gate

- `bun run lint:tone --strict` — 모든 한글 텍스트 해요체 0 err (금지: 합니다 / 입니다 / 드립니다 / 당신).
- frontmatter validity check — 4 skill 의 SKILL.md frontmatter 유효성.
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

## Skill routing

이 repo 의 공개 plugin surface 는 `onboarding` / `init` / `deploy` / `clarity` 네 스킬뿐이에요.

Key routing rules:
- 처음 셋업·CLI 설치·로그인·환경 점검 → `onboarding`
- 새 앱 생성·템플릿·bootstrap saga → `init`
- 배포 실행·preview-confirm·verify 기반 성공 선언 → `deploy`
- 그 외 axhub 기능, 상태·로그·환경변수·롤백·모호한 axhub 발화 → `clarity`
