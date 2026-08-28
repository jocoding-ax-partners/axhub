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

이 repo는 axhub plugin이에요. 현재 공개 surface는 **10 skill** (`onboarding` / `bootstrap` / `scaffold` / `plugins` / `deploy` / `import` / `development` / `diagnosis` / `clarity` / `update`)이에요. `plugins`는 category=plugin인 일반 App의 게시·목록·정확한 version 다운로드를 맡고, 게시자 관리는 App Console, 승인은 Console Review로 연결해요. plugin은 판정·실행 로직을 직접 갖지 않고 ax-hub-cli(`axhub` 바이너리)를 호출해요.

제거된 시스템 (재추가 금지): Rust helper 바이너리 (`crates/axhub-helpers`), 범용 NL routing corpus, scaffold / skill-doctor / lint:keywords 인프라, cosign 멀티-바이너리 릴리즈 파이프라인. 훅은 cheap bash guard 로 제한해요: auto-update, onboarding resume, Windows 실행 계약, update-first Code-mode router guard 만 허용돼요.

## skill 이 CLI 를 부르는 법

- 흡수된 helper 표면은 `axhub plugin-support <cmd>` (hidden 그룹) 로 호출해요 — 예: `onboarding-detect`, `preflight`, `deploy-prep`.
- 공개 검증·진단 표면은 `axhub deploy verify <deployment-id> --app <app>` 와 `axhub deploy diagnose` 예요.
- bootstrap·deploy skill은 시작 시 `axhub` 존재와 `plugin-support` 기능(preflight)을 확인해 최소 표면 v0.21.3+를 가드하고, scaffold는 v0.30.0+를 가드해요. plugins는 version 추측 대신 필요한 `axhub plugin list|download|publish --help` 성공을 가드하고, 기능이 없으면 `update`로 보내며 우회하지 않아요.
- 저장소 provider 판정은 resume/existing에서 public `axhub apps get <app> --json`, fresh bootstrap/onboarding에서 read-only `axhub apps git-backend --tenant <tenant> --json`의 top-level `git_backend`만 써요. selfhosted는 GitHub 인증/App 설치 대사를 건너뛰고, non-static deploy는 `axhub repo clone` 뒤 일반 `git push` webhook 경로를 써요. static은 기존 release lane을 유지하며 Gitea API를 직접 호출하지 않아요.

## 변경 검증

```bash
bun test               # skill / e2e 회귀
bun run lint:tone --strict   # 해요체 0 err
bunx tsc --noEmit      # 타입 clean
bun run plugin:bundle  # clean local plugin bundle 생성
```

10개 skill의 frontmatter validity check와 대표 e2e flow도 살아남은 quality gate예요.

대표 여정 회귀는 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**을 문서·skill 본문·fixture 계약으로 같은 방향에 맞추는 방식이에요. 실제 ax-hub-cli 구현/schema parity/release 는 이 repo 범위 밖 follow-up 으로 남겨요.

## Never Do

- NEVER helper 바이너리 (`crates/axhub-helpers`) 나 hook / NL routing / scaffold 인프라 재추가 — diet 결정 위반.
- NEVER 명시적 결정 없이 skill을 추가하지 않아요 — `scaffold`와 `plugins`는 사용자 명시 결정으로 추가됐고 현재 10 skill 체제를 유지해요.
- NEVER 최소 CLI 기능 게이트를 우회하지 말아요.
- NEVER Claude Code local plugin 을 repo 루트에서 직접 설치/검증하지 말아요 — `bun run plugin:bundle` 후 `dist/axhub-plugin` clean bundle 을 써요.
- NEVER deploy 성공 선언(docker/compose deployment-record lane)을 `axhub deploy verify <deployment-id> --app <app>` 1회 실행 없이 — deployment id 와 app scope 필수, latest 재탐색 금지. static 앱(deploy_method=static)은 별도 lane 이라 `deploy verify` 가 404 라서 `active_release_id`(activate 성공)로 선언해요.
- NEVER release 를 manual `vim package.json` + `git tag` 로 — `bun run release` → narrative amend → `bun run release:tag` 3단계 flow (단순화된 postbump) 만 써요.
