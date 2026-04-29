<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **axhub** (2071 symbols, 3328 relationships, 108 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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

# axhub Skill Authoring (Phase 17/18 강제)

새 SKILL 을 `skills/<name>/SKILL.md` 에 작성할 때 **반드시** scaffold 사용. 직접 작성 금지.

## Always Do

- **MUST `bun run skill:new <slug>` 로 스캐폴드 생성** — 직접 `mkdir skills/<name>` 후 SKILL.md 작성 시 Phase 17/18 패턴 (D1 TTY guard / TodoWrite Step 0 / `!command` preflight injection / AskUserQuestion header / registry stub) 누락되어 CI fail.
- **MUST frontmatter 에 `multi-step:` + `needs-preflight:` 선언** — `multi-step: true` (deploy/recover/update/upgrade/doctor 같이 4+ 단계) 또는 `false` (apis/apps/auth/clarify/logs/status 같이 단일/조회). `needs-preflight: true` (deploy/recover/apis/apps 같이 live state 필요) 또는 `false`.
- **MUST `bun run skill:doctor` 로 패턴 체크** — colored 한글 출력으로 SKILL 별 D1 sentinel / TodoWrite / `!command` preflight 누락 즉시 보임. CI 도 `--strict` mode 로 같은 검사 통과 필수.
- **MUST `tests/fixtures/ask-defaults/registry.json` 에 AskUserQuestion 별 `safe_default` + `rationale` 등록** — scaffold 가 stub 자동 append. 새 question 추가 시 매번 channel 등록 (drift catch).
- **MUST 모든 한글 텍스트 해요체 통일** — `bun run lint:tone --strict` 0 err 필수. 금지 token: 합니다 / 입니다 / 시겠어요 / 드립니다 / 당신 / 아이고. 사용: 해요 / 예요 / 이에요 / 할래요.
- **MUST nl-lexicon trigger 어구는 frontmatter `description:` 에만** — `bun run lint:keywords --check` 베이스라인 잠금. SKILL body 에서 새 trigger 어구 추가하면 baseline 깨짐.

## Skill Authoring Workflow

```bash
# 1. Scaffold 생성 (mutate-aware safe defaults)
bun run skill:new my-skill

# Read-only SKILL 인 경우 flag opt-out:
bun run skill:new my-readonly --no-multi-step --no-preflight

# 2. skills/my-skill/SKILL.md TODO placeholder 채우기
#    - description: nl-lexicon 활성화 trigger 어구
#    - workflow Step 1..N
#    - AskUserQuestion JSON block (필요 시)

# 3. 진단 + lint + test
bun run skill:doctor          # 패턴 누락 colored 출력
bun run lint:tone --strict    # 톤 0 err
bun run lint:keywords --check # nl-lexicon 베이스라인 lock
bun test                      # 회귀

# 4. 모두 green 이면 commit
```

## Never Do

- NEVER `mkdir skills/foo && touch skills/foo/SKILL.md` 후 빈 SKILL.md 직접 작성. scaffold 우회 시 패턴 누락.
- NEVER `tests/ux-todowrite.test.ts` / `tests/ux-skill-preflight-injection.test.ts` 등에 hardcoded SKILL slug 추가. frontmatter 선언으로 자동 enforce — test 코드 편집 불필요.
- NEVER `tests/fixtures/ask-defaults/registry.json` 에 AskUserQuestion 등록 없이 SKILL ship. `tests/ux-ask-fallback-registry.test.ts` 가 drift catch.
- NEVER `description:` 의 nl-lexicon trigger 어구 변경 — baseline 깨짐. 필요 시 `.omc/lint-baselines/skill-keywords.json` baseline 재캡처 (rare event).
- NEVER lint:tone scope 에 SKILL frontmatter 포함 시키기 — `description:` 은 nl-lexicon trigger 라 byte-identical lock 우선.

## Self-Check Before Finishing Skill

- [ ] `bun run skill:doctor --strict` exit 0
- [ ] `bun run lint:tone --strict` 0 err
- [ ] `bun run lint:keywords --check` no diff
- [ ] `bun test` ≥498 pass / 0 fail (Phase 18 baseline)
- [ ] `bunx tsc --noEmit` clean
- [ ] frontmatter `multi-step:` + `needs-preflight:` 선언
- [ ] AskUserQuestion 마다 registry entry 등록

# axhub Release Workflow (Phase 19 v0.1.19+ 자동화)

새 버전 ship 할 때 **반드시** `bun run release` 사용. 직접 `vim package.json` + `git tag` 절대 금지 (drift 위험).

## Always Do

- **MUST `bun run release` 로 자동 bump** — `commit-and-tag-version` (D2) 가 3 파일 (package.json + plugin.json + marketplace.json) bump + postbump hook (codegen:version + release:check) + CHANGELOG entry + git commit + tag 자동 chain.
- **MUST CHANGELOG 본문 narrative 추가** — auto-bullets 위에 Phase NN 한국어 narrative paragraph (해요체) 작성. `git commit --amend --no-edit -a` 로 tag commit 에 narrative 합침.
- **MUST `git push origin main --tags`** — release.yml 가 tag push 시 자동 fire (5 cross-arch binary cosign 서명 + GH release upload).
- **MUST Conventional Commits** — `feat:` (minor) / `fix:` (patch) / `chore:` (no-bump) / `docs:` / `test:` / `refactor:` / `perf:` (Performance section). commit-and-tag-version 이 type 으로 bump 결정.

## Release Workflow

```bash
# 1. clean working tree 확인
git status

# 2. release 한 줄 (auto-bump from commit history since last tag)
bun run release
# 또는 explicit:
bun run release -- --release-as patch    # 0.1.X → 0.1.X+1
bun run release -- --release-as minor    # 0.1.X → 0.2.0
bun run release -- --release-as major    # 0.X.Y → 1.0.0
bun run release -- --release-as 0.1.20   # explicit version

# 자동 수행:
#  ✓ 3 files bump (package/plugin/marketplace)
#  ✓ postbump: codegen:version (install.sh/ps1/index.ts/telemetry.ts 동기화)
#               + release:check (5 binary build + version assert — v0.1.14 stale binary 재발 방지)
#  ✓ CHANGELOG.md auto entry (Conventional Commits parse)
#  ✓ git commit + git tag vX.Y.Z

# 3. CHANGELOG narrative 추가 (해요체 1-3 문장)
vim CHANGELOG.md   # auto-bullets 위에 Phase NN narrative paragraph + Test baseline + Honest tradeoff sections
git commit --amend --no-edit -a   # tag commit 에 narrative 흡수

# 4. push
git push origin main --tags
# release.yml 자동 fire — 5 binary cosign 서명 + GH release v0.1.X 생성

# 5. release 검증
gh release view vX.Y.Z --json url -q .url
```

## Hotfix Workflow (긴급 fix mid-Phase)

```bash
git commit -am "fix: <urgent issue>"
bun run release -- --release-as patch
git push origin main --tags
```

## Never Do

- NEVER `vim package.json` + `git tag` manual edit. drift + v0.1.14 stale binary 재발 위험.
- NEVER `git push --force` to main. branch protection + hook block.
- NEVER skip CHANGELOG narrative — auto-bullets 만으로는 Phase 의미 전달 부족.
- NEVER tag without push (local-only tag = release.yml 안 fire).
- NEVER use D1 release-please / standard-version / semantic-release — 이미 D2 commit-and-tag-version 결정 (.versionrc.json + ralplan ADR 기록).

## Self-Check Before Push

- [ ] `git status` clean
- [ ] `bun run release` 자동 chain 완료 (3 files + postbump + CHANGELOG + commit + tag)
- [ ] CHANGELOG narrative paragraph (해요체) 추가 + amend
- [ ] `git push origin main --tags` 성공
- [ ] release.yml workflow run completed: success
- [ ] `gh release view vX.Y.Z` 5 cross-arch binaries 확인
