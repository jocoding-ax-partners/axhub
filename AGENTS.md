<!-- codegraph:start -->
# CodeGraph — Code Intelligence

The monorepo is indexed by CodeGraph (`codegraph_*` MCP tools, server `codegraph serve --mcp`). Consult it BEFORE editing, not during.

- **Before editing a function, class, or method**, run `codegraph_impact` (or `codegraph_callers`) on it and report the blast radius to the user.
- **"How does X work?"** — start with `codegraph_context`, then ONE `codegraph_explore` for the symbols it surfaces. Don't grep + read in a loop.
- **"How does X reach Y?"** — `codegraph_trace` from→to returns the whole path in one call.
- Symbol lookup: `codegraph_search`; one symbol's source: `codegraph_node`; directory listing: `codegraph_files`.
- Results look stale or "Not initialized"? Check `codegraph_status`; Read files the staleness banner lists.
- Index lives at the monorepo root `.codegraph/` (gitignored). One-time setup: `codegraph init --index` at the monorepo root. New worktrees get a copy automatically via the lefthook `post-checkout` hook.
<!-- codegraph:end -->

# axhub plugin (diet 체제)

이 repo는 axhub plugin이에요. 현재 공개 surface는 **11 skill** (`onboarding` / `bootstrap` / `scaffold` / `plugins` / `deploy` / `up` / `import` / `development` / `diagnosis` / `clarity` / `update`)이에요. `plugins`는 deploy_method=plugin인 일반 App의 게시·목록·정확한 version 다운로드를 맡고, 게시자 관리는 App Console, 승인은 Console Review로 연결해요. plugin은 판정·실행 로직을 직접 갖지 않고 ax-hub-cli(`axhub` 바이너리)를 호출해요.

제거된 시스템 (재추가 금지): Rust helper 바이너리 (`crates/axhub-helpers`), 범용 NL routing corpus, scaffold / skill-doctor / lint:keywords 인프라, cosign 멀티-바이너리 릴리즈 파이프라인. 훅은 cheap bash guard 로 제한해요: auto-update, onboarding resume, Windows 실행 계약, update-first Code-mode router guard 만 허용돼요.

## skill 이 CLI 를 부르는 법

- 흡수된 helper 표면은 `axhub plugin-support <cmd>` (hidden 그룹) 로 호출해요 — 예: `onboarding-detect`, `preflight`, `deploy-prep`.
- 공개 검증·진단 표면은 `axhub deploy verify <deployment-id> --app <app>` 와 `axhub deploy diagnose` 예요.
- bootstrap·deploy skill은 시작 시 `axhub` 존재와 `plugin-support` 기능(preflight)을 확인해 최소 표면 v0.21.3+를 가드하고, scaffold는 v0.30.0+를 가드해요. plugins는 version 추측 대신 필요한 `axhub plugin list|download|publish --help` 성공을 가드하고, 기능이 없으면 `update`로 보내며 우회하지 않아요.
- 저장소 provider 판정은 resume/existing에서 public `axhub apps get <app> --json`, fresh bootstrap/onboarding/scaffold에서 read-only `axhub apps git-backend --tenant <tenant> --json`의 top-level `git_backend`만 써요. 기존 코드 import는 `plugin-support import`가 같은 입력으로 경로를 정해요. selfhosted는 GitHub 인증/App 설치 대사를 건너뛰고, non-static deploy는 `axhub repo clone` 뒤 일반 `git push` webhook 경로를 써요. static은 기존 release lane을 유지하며 Gitea API를 직접 호출하지 않아요.

## 변경 검증

```bash
bun test               # skill / e2e 회귀
bun run lint:tone --strict   # 해요체 0 err
bunx tsc --noEmit      # 타입 clean
bun run plugin:bundle  # clean local plugin bundle 생성
```

11개 skill의 frontmatter validity check와 대표 e2e flow도 살아남은 quality gate예요.

대표 여정 회귀는 **첫 셋업 → 앱 생성 → 배포 → 상태 확인**을 문서·skill 본문·fixture 계약으로 같은 방향에 맞추는 방식이에요. 실제 ax-hub-cli 구현/schema parity/release 는 이 repo 범위 밖 follow-up 으로 남겨요.

## Never Do

- NEVER helper 바이너리 (`crates/axhub-helpers`) 나 hook / NL routing / scaffold 인프라 재추가 — diet 결정 위반.
- NEVER 명시적 결정 없이 skill을 추가하지 않아요 — `scaffold`·`plugins`·`up`은 사용자 명시 결정으로 추가됐고 현재 11 skill 체제를 유지해요.
- NEVER 최소 CLI 기능 게이트를 우회하지 말아요.
- NEVER Claude Code local plugin 을 repo 루트에서 직접 설치/검증하지 말아요 — `bun run plugin:bundle` 후 `dist/axhub-plugin` clean bundle 을 써요.
- NEVER deploy 성공 선언(docker/compose deployment-record lane)을 `axhub deploy verify <deployment-id> --app <app>` 1회 실행 없이 — deployment id 와 app scope 필수, latest 재탐색 금지. static 앱(deploy_method=static)은 별도 lane 이라 `deploy verify` 가 404 라서 `active_release_id`(activate 성공)로 선언해요.
- NEVER release 를 manual `vim package.json` + `git tag` 로 — `bun run release` → narrative amend → `bun run release:tag` 3단계 flow (단순화된 postbump) 만 써요.
