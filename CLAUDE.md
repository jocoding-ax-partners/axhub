<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **axhub** (6798 symbols, 13019 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

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

> **Plugin 설계 레퍼런스**: 인증 2모드(SSO cookie / PAT), `.axhub/` AI 컨텍스트, MCP 통합, backend catalog API, v1 DoD 는 [`docs/plugin-developer-guide.md`](docs/plugin-developer-guide.md) 에 한 문서로 정리돼 있어요. plugin 동작/설계를 바꾸기 전에 먼저 읽어요.

새 SKILL 을 `skills/<name>/SKILL.md` 에 작성할 때 **반드시** scaffold 사용. 직접 작성 금지.

## Always Do

- **MUST `bun run skill:new <slug>` 로 스캐폴드 생성** — 직접 `mkdir skills/<name>` 후 SKILL.md 작성 시 Phase 17/18 패턴 (D1 TTY guard / TodoWrite Step 0 / in-body preflight 블록 / AskUserQuestion header / registry stub) 누락되어 CI fail.
- **MUST frontmatter 에 `multi-step:` + `needs-preflight:` 선언** — `multi-step: true` (deploy/recover/update/upgrade/doctor 같이 4+ 단계) 또는 `false` (apps/auth/clarify/logs/status 같이 단일/조회). `needs-preflight: true` (deploy/recover/apps 같이 live state 필요) 또는 `false`.
- **MUST `bun run skill:doctor` 로 패턴 체크** — colored 한글 출력으로 SKILL 별 D1 sentinel / TodoWrite / in-body preflight / **step-numbering collision (FU-3)** 누락 즉시 보임. CI 도 `--strict` mode 로 같은 검사 통과 필수. step-collision 검사는 `## Workflow` body 의 top-level `^N. **` 헤더 중복을 자동 catch (sub-step 인 `3.5. **` 이나 `### Subsection` 안의 local 1./2./.../D1./D2. 는 exempt).
- **MUST `tests/fixtures/ask-defaults/registry.json` 에 AskUserQuestion 별 `safe_default` + `rationale` 등록** — scaffold 가 stub 자동 append. 새 question 추가 시 매번 channel 등록 (drift catch).
- **MUST 모든 한글 텍스트 해요체 통일** — `bun run lint:tone --strict` 0 err 필수. 금지 token: 합니다 / 입니다 / 시겠어요 / 드립니다 / 당신 / 아이고. 사용: 해요 / 예요 / 이에요 / 할래요.
- **MUST nl-lexicon trigger 어구는 frontmatter `description:` 에만** — `bun run lint:keywords --check` 베이스라인 잠금. SKILL body 에서 새 trigger 어구 추가하면 baseline 깨짐.
- **MUST in-body preflight 계약 통과 — `bun run skill:doctor` 가 자동 검사** — `needs-preflight: true` SKILL 은 (a) load-time `!command` 주입이 **없고** (b) body 에 `scripts/preflight-block.ts` 의 `CANONICAL_PREFLIGHT_BLOCK` 을 그대로 포함해야 해요 (scaffold `bun run skill:new` 가 자동 삽입). literal `axhub-helpers preflight --json` 만 쓰면 helper-pick fallback 이 누락되므로, skill-doctor 는 `PREFLIGHT_JSON=$("$HELPER" preflight --json` 할당 signature 로 검증해요 (bare 언급은 통과 못 함). 모든 SKILL 은 폐기된 `!`node -e ...preflight`` dead injection 을 가지면 안 돼요. (load-time `!command` 주입은 첫 실행 권한 hard-fail 로 폐기 — docs/adr/0013 참조, ADR-0011 supersede.)

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
- NEVER `tests/ux-todowrite.test.ts` / `tests/ux-skill-preflight-injection.test.ts` 등에 hardcoded SKILL slug 추가. SKILL 추가/제거는 frontmatter 선언으로 자동 enforce 돼요 (slug 추가 불필요). 단, 패턴 계약 자체가 바뀌면 (예: preflight injection→in-body) assertion 로직은 편집해야 해요.
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
- [ ] (Phase 25 PR 25.5a+) 새 SKILL 또는 마이그레이션 시 `model:` 선언 (`haiku|sonnet|opus`)
- [ ] AskUserQuestion 마다 registry entry 등록

## SKILL Model Routing (Phase 25 PR 25.5a+)

새 SKILL 생성 시 `model:` frontmatter 를 권장해요 (25.5a 부터 scaffold + skill-doctor 가 지원). 기존 19 SKILL 의 일괄 마이그레이션은 25.5a.1 단일-skill A/B test 통과 후 25.5b (haiku) / 25.5c (sonnet) 에서 진행해요.

- **haiku**: read-only / 단순 조회 (status / logs / open / clarify / doctor / routing-stats)
- **sonnet**: multi-step / destructive / interactive (deploy / recover / env / apps / auth / github / init / install-cli / profile / update / upgrade / verify / trace)
- **opus**: 사용 안 함 (axhub 도메인 외 — architecture decision / deep analysis 필요 없음)

```bash
# 새 SKILL scaffold 시 명시
bun run skill:new <slug> --model haiku

# 미명시 시 default = sonnet (mutate-aware 안전 기본값)
bun run skill:new <slug>
```

기존 19 SKILL 은 `model:` 미선언 상태도 `bun run skill:doctor --strict` 통과해요 (Phase 25 PR 25.5a 의 no-op 약속). 선언했다면 `haiku|sonnet|opus` 셋 중 하나여야 해요.

# axhub Release Workflow (Phase 19 v0.1.19+ 자동화)

새 버전 ship 할 때 **반드시** 2단계 flow (`bun run release` → narrative amend → `bun run release:tag`) 사용. 직접 `vim package.json` + `git tag` 절대 금지 (drift 위험).

v0.9.1 이전 flow 는 `commit-and-tag-version` 이 commit + tag 를 동시 생성해서 narrative amend 가 tag 와 분리됐어요 (tag 가 amend 전 commit 가리켜서 release.yml 이 narrative 빈 채로 fire). 이를 막기 위해 `.versionrc.json` 의 `skip.tag=true` + 새 `release:tag` 스크립트로 분리했어요.

## Always Do

- **MUST step 1: `bun run release` 로 bump + commit (tag 미생성)** — `commit-and-tag-version` 이 3 파일 (package.json + plugin.json + marketplace.json) bump + postbump hook (codegen:version + release:check) + CHANGELOG entry + git commit 까지만 수행. tag 는 step 3 에서 생성.
- **MUST step 2: CHANGELOG narrative 추가 + amend** — auto-bullets 위에 Phase NN 한국어 narrative paragraph (해요체) 작성. `git commit --amend --no-edit -a` 로 bump commit 에 narrative 흡수.
- **MUST step 3: `bun run release:tag` 로 tag 생성 + push** — `scripts/release-tag.ts` 가 (a) CHANGELOG 현 버전 섹션의 narrative 길이 ≥50 chars 검증, (b) working tree clean 검증, (c) HEAD 에 `vX.Y.Z` tag 생성, (d) `git push origin main && git push origin vX.Y.Z` 수행. release.yml 가 narrative 포함된 commit 의 tag push 로 fire.
- **MUST Conventional Commits** — `feat:` (minor) / `fix:` (patch) / `chore:` (no-bump) / `docs:` / `test:` / `refactor:` / `perf:` (Performance section). commit-and-tag-version 이 type 으로 bump 결정.

## Release Workflow

```bash
# 1. clean working tree 확인
git status

# 2. step 1 — bump + commit (tag 미생성)
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
#  ✓ git commit (tag 는 안 생성 — .versionrc.json skip.tag=true)

# 3. step 2 — CHANGELOG narrative 추가 (해요체 1-3 문장)
vim CHANGELOG.md   # auto-bullets 위에 Phase NN narrative paragraph + Test baseline + Honest tradeoff sections
git commit --amend --no-edit -a   # bump commit 에 narrative 흡수

# 4. step 3 — tag 생성 + push
bun run release:tag
# 자동 수행:
#  ✓ CHANGELOG narrative 길이 검증 (50 chars 미만이면 fail)
#  ✓ working tree clean 검증
#  ✓ git tag -a vX.Y.Z (HEAD 가리킴)
#  ✓ git push origin main + git push origin vX.Y.Z
#  ✓ release.yml 자동 fire — 5 binary cosign 서명 + GH release + Slack narrative

# 5. release 검증
gh release view vX.Y.Z --json url -q .url
```

## Hotfix Workflow (긴급 fix mid-Phase)

```bash
git commit -am "fix: <urgent issue>"
bun run release -- --release-as patch
# narrative 추가
vim CHANGELOG.md
git commit --amend --no-edit -a
bun run release:tag
```

## Never Do

- NEVER `vim package.json` + `git tag` manual edit. drift + v0.1.14 stale binary 재발 위험.
- NEVER `git push --force` to main. branch protection + hook block.
- NEVER skip CHANGELOG narrative — `release:tag` 가 50 chars 미만 narrative 를 거부해요.
- NEVER step 1 직후 곧바로 `git push --tags` — tag 가 없어서 release.yml 안 fire. step 3 의 `release:tag` 가 tag 생성 + push 를 한 번에 처리.
- NEVER tag 를 step 2 의 amend 전에 생성 — v0.9.1 회귀 시나리오. tag 는 narrative amend 후 HEAD 에 만들어야 해요.
- NEVER use D1 release-please / standard-version / semantic-release — 이미 D2 commit-and-tag-version 결정 (.versionrc.json + ralplan ADR 기록).

## Self-Check Before Push

- [ ] `git status` clean
- [ ] `bun run release` 자동 chain 완료 (3 files + postbump + CHANGELOG + commit + tag)
- [ ] CHANGELOG narrative paragraph (해요체) 추가 + amend
- [ ] `git push origin main --tags` 성공
- [ ] release.yml workflow run completed: success
- [ ] `gh release view vX.Y.Z` 5 cross-arch binaries 확인

# axhub Hook Safety (Phase 25 PR 25.2)

axhub Claude Code hook 의 fail-open 계약을 명문화했어요. 모든 hook 진입점은
`exit 0` 보장이에요. 자세한 spec 은 `docs/HOOKS.md` 를 봐주세요.

## Kill Switch (Env Var Taxonomy ADR §10.6 따름)

```bash
# 모든 axhub hook 비활성화 (canonical)
AXHUB_DISABLE_HOOKS=1

# 특정 hook 만 비활성화 (csv)
AXHUB_DISABLE_HOOK=session-start,preauth-check,prompt-route,classify-exit

# Legacy alias (6 개월 deprecation, v0.8.0 에서 제거)
DISABLE_AXHUB=1   # stderr 에 경고 출력
```

우선순위: `AXHUB_DISABLE_HOOKS` > `AXHUB_DISABLE_HOOK` > legacy.

## Always Do

- **MUST 새 hook subcommand 추가 시 `hook_safety::is_hook_disabled("name")` 진입부 첫 줄에 호출** — Rust helper 는 `crates/axhub-helpers/src/hook_safety.rs` 가 canonical. shell wrapper 가 있으면 동일 패턴 미러.
- **MUST fail-open 원칙 지키기** — 어떤 실패에서도 exit 0. systemMessage 로만 사용자 노출. panic 금지.
- **MUST 새 hook 이름을 `docs/HOOKS.md` §1 표 + 테스트 매트릭스에 추가**.
- **MUST 신규 opt-out / kill switch env 도입 시 §10.6 polarity 룰 (`AXHUB_DISABLE_*` / `AXHUB_ENABLE_*` / `AXHUB_<scope>=<value>`) 따름**.

## Never Do

- NEVER hook 에서 `unwrap()` / `panic!()` — fail-open 깨짐. `Result<>` + `unwrap_or_else` 패턴.
- NEVER non-zero exit — Claude Code 가 main 흐름 차단해요.
- NEVER polarity inconsistent env (`AXHUB_NO_*` / `DISABLE_AXHUB_*` 같은) 도입 — §10.6 ADR 위반.
- NEVER `AXHUB_HOOK_SAFETY_DISABLED` 사용 — ghost variable (zero code matches), 폐기됨. canonical `AXHUB_DISABLE_HOOKS=1` 써요.
- NEVER `DISABLE_AXHUB` 새 자동화에서 사용 — legacy alias, v0.8.0 제거 예정.

## Self-Check Before Adding a Hook

- [ ] `hook_safety::is_hook_disabled("name")` 진입부 호출
- [ ] shell wrapper (`hooks/<name>.sh`, `.ps1`) 있으면 동일 kill switch mirror
- [ ] `docs/HOOKS.md` §1 표에 hook 이름 추가
- [ ] `tests/hooks-kill-switch.test.ts` 매트릭스에 새 hook case 추가
- [ ] `cargo test hook_safety` + `bun test tests/hooks-kill-switch.test.ts` 모두 pass
- [ ] hook 실패 path 에서 `hook_safety::append_hook_error("name", &err)` 호출 확인

<!-- SPECKIT START -->
For additional context about technologies to be used, project structure,
shell commands, and other important information, read the current plan:
`specs/006-verify-skill-cli-alignment/plan.md` (+ research.md, data-model.md,
contracts/verify-cli-contract.md, quickstart.md).
<!-- SPECKIT END -->
