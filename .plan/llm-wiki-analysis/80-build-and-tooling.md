# 80 — Build, Bundling, CI Tooling

> 모든 설정값은 파일에서 직접 인용해요. 추측 없이 `file:line` 표기 유지.

## Frontend build chain

### `package.json` scripts (lines 6-15)

| Script | 명령 | 비고 |
|---|---|---|
| `dev` | `vite` | dev server (port 1420 strict) |
| `typecheck` | `tsc --build --pretty` | project references 빌드 |
| `build` | `npm run typecheck && vite build` | 타입체크 → vite 번들 (artifact `dist/`) |
| `preview` | `vite preview` | 정적 서빙 |
| `test` | `npm run test:mocks && npm run test:llm` | mocks 먼저, real-llm 그 다음 |
| `test:mocks` | `vitest run --exclude='**/*.real-llm.test.ts'` | 외부 호출 없는 격리 테스트 |
| `test:llm` | `vitest run real-llm --no-file-parallelism --reporter=verbose` | 실제 LLM 호출 — 직렬 + verbose. `.env.test.local` 의 키 필요 |
| `tauri` | `tauri` | `@tauri-apps/cli` proxy |

### Vite (`vite.config.ts`)

- 플러그인: `@vitejs/plugin-react` + `@tailwindcss/vite` (Tailwind 4 vite plugin)
- alias: `@` → `./src` (`tsconfig.app.json` 과 동일)
- define: `__APP_VERSION__` = `JSON.stringify(pkgJson.version)` — 단일 source of truth (`vite.config.ts:11,22`)
- `clearScreen: false` — Rust 컴파일러 에러가 vite 화면 클리어로 안 가려져요
- `server.port: 1420`, `strictPort: true` — Tauri 가 고정 포트 기대
- `server.host: process.env.TAURI_DEV_HOST || false` — env 있으면 LAN 노출, 없으면 localhost only
- HMR: TAURI_DEV_HOST 있을 때만 ws 모드 (`port: 1421`)
- `server.watch.ignored: ["**/src-tauri/**"]` — Rust 변경은 cargo가 watch
- Vitest 통합: `environment: node`, `setupFiles: ["./src/test-helpers/load-test-env.ts"]` (no-op if `.env.test.local` 없음)

### TypeScript (`tsconfig*`)

- `tsconfig.json` (루트, `files: []`) — references 만 가짐: `app` + `node`
- `tsconfig.app.json` (`include: ["src"]`)
  - target ES2020, lib `ES2020 + DOM + DOM.Iterable`, module ESNext
  - moduleResolution `bundler`, allowImportingTsExtensions, isolatedModules, moduleDetection `force`
  - jsx `react-jsx`, useDefineForClassFields, noEmit
  - strict + noUnusedLocals + noUnusedParameters + noFallthroughCasesInSwitch
  - paths `@/*` → `./src/*`
- `tsconfig.node.json` (`include: ["vite.config.ts"]`)
  - target ES2022, lib ES2023 (Node20 호환)

> 두 cfg 모두 `noEmit` — vite가 transpile 담당, tsc는 type check only.

## Rust build chain (`src-tauri/`)

### `Cargo.toml` 헤더

- crate-types: `staticlib` + `cdylib` + `rlib` (Tauri 정적 라이브러리 + 동적 + Rust 라이브러리 — multi-target)
- 바이너리: `name = "llm-wiki"`, path `src/main.rs`
- build deps: `tauri-build = "2"` — `build.rs:2` 가 `tauri_build::build()` 호출

### Release profile (`Cargo.toml:63-71`)

```toml
codegen-units = 1   # 단일 unit — LTO 효과 극대화
lto = true          # cross-crate inline
opt-level = "s"     # size > speed (앱 번들 크기 우선)
panic = "unwind"    # ← panic_guard.rs 가 잡으려면 필수. abort 였다면 단일 파일 손상 = 앱 죽음
strip = true        # 디버그 심볼 제거
```

> Cargo 주석 (line 63-70): "Unwind (not abort) so third-party parser panics can be caught at the Tauri command boundary via panic_guard and turned into errors. Slightly larger binary, but prevents single-file corruption from killing the app."

### `tauri-plugin-http` features

`features = ["unsafe-headers"]` (`Cargo.toml:30`) — 임의 헤더 허용, **trust boundary 의 의도적 약화**. LLM provider 인증 헤더 + 임의 endpoint 지원하기 위함. capability allowlist 와 함께 04 의 Internal Risk 가 cover.

### pdfium 번들

- Crate: `pdfium-render = "0.9"` (`Cargo.toml:28`)
- 빌드 시 vendored 라이브러리:
  - macOS: `src-tauri/pdfium/libpdfium.dylib` (`tauri.macos.conf.json:4`: `frameworks`)
  - Linux: `src-tauri/pdfium/libpdfium.so` (`tauri.linux.conf.json:3`: `resources`)
  - Windows: `src-tauri/pdfium/pdfium.dll` (`tauri.windows.conf.json:3`: `resources`)
- 04-backend-rust + 08-pdf-ocr-pipeline 가 FFI loader + extern "C" 표면 검증.

### Tauri capabilities (`src-tauri/capabilities/default.json`)

- 윈도우: `["main"]`
- permissions: `core:default`, `opener:default`, `dialog:default`, `store:default`
- `http:default` allowlist: 모든 http/https 패턴 — `http://*`, `http://*/*`, `http://*:*`, `http://*:*/*`, `http://**`, https variants. **사실상 전 인터넷.** 의도적 — LLM provider URL 임의 입력 지원. trust boundary는 사용자 입력에서 시작.

### Tauri config (`tauri.conf.json`)

- bundle.targets: `"all"` (msi/nsis on Windows, dmg on macOS, deb/AppImage on Linux)
- 아이콘: 32×32, 128×128, 128×128@2x, .icns, .ico
- CSP (`tauri.conf.json:23`):
  - `default-src 'self'`
  - `connect-src 'self' https: http:` — 광범위 (LLM endpoint 자유)
  - `img-src 'self' asset: https://asset.localhost` — Tauri asset protocol 사용
  - `media-src 'self' asset: https://asset.localhost`
  - `style-src 'self' 'unsafe-inline'` — Tailwind dynamic class 위해 `unsafe-inline`
- `assetProtocol`: enabled, `scope: ["**"]` — 전체 파일시스템 접근

## Test infrastructure

### 시험 분리 전략

`package.json:11-13` 에서:
- `test:mocks` — `*.real-llm.test.ts` 제외. 결정론적, fast.
- `test:llm` — `real-llm` 만 매칭, `--no-file-parallelism` (LLM rate limit 회피), `--reporter=verbose`.
- 합치면 `test`.

### 시험 종류 (파일 suffix 기준)

- `*.test.ts` — 단위 (mock LLM, mock fs)
- `*.real-llm.test.ts` — 실제 LLM 호출 (`load-test-env.ts` 로 `.env.test.local` 키 로드)
- `*.scenarios.test.ts` — 시나리오 기반 (사전 정의된 입력 매트릭스)
- `*.property.test.ts` — fast-check 속성 테스트
- `*.race.test.ts` — 동시성 / race 검증
- `*.integration.test.ts` — 다중 모듈 결합 시험

세부 분포는 31-testing-strategy 또는 50-source-mapping 의 [test] 태그 행 카운트로 확인.

## CI / Release pipelines

### `ci.yml` (push/PR to main)

매트릭스: `macos-latest`, `ubuntu-22.04`, `windows-latest`. `fail-fast: false`.

각 OS 단계:
1. Checkout (`actions/checkout@v4`)
2. Rust toolchain stable (`dtolnay/rust-toolchain@stable`)
3. protoc 설치 (OS 분기):
   - macOS: `brew install protobuf`
   - Ubuntu: `apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf protobuf-compiler`
   - Windows: `choco install protoc -y`
4. Rust 캐시 (`Swatinem/rust-cache@v2`, `workspaces: src-tauri`)
5. Node 20 (`actions/setup-node@v4`)
6. `npm install`
7. `npx vite build` (frontend)
8. `cargo build` (working-directory: src-tauri)

> CI 가 Tauri bundle 까지 안 만들고 `cargo build` 만 — 빠른 검증.

### `build.yml` (tag `v*` push, workflow_dispatch)

매트릭스 (3 platform + macOS aarch64 target):
- `macos-latest` + `--target aarch64-apple-darwin` + `rust_target: aarch64-apple-darwin`
- `ubuntu-22.04` (no special args)
- `windows-latest` (no special args)

setup 단계는 ci 와 동일 + Rust target 추가 (aarch64-apple-darwin).

빌드: `tauri-apps/tauri-action@v0`. env:
- `GITHUB_TOKEN`
- Apple 코드 서명: `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`

릴리스 동작:
- Tag push (`github.event_name == 'push'`): tagName + releaseName 채워서 GitHub Release 생성. `releaseDraft: false`, `prerelease: false`.
- Manual (`workflow_dispatch`): 빈 tagName/releaseName → tauri-action 이 release upload skip. 별도 step 이 `actions/upload-artifact@v4` 로 .msi/.exe/.dmg/.deb/.AppImage glob 업로드 (14일 보관).

추가 job `package-extension`:
- `needs: build`
- `if: github.event_name == 'push'` — tag-only
- `runs-on: ubuntu-latest`
- 흐름:
  1. checkout
  2. node script 로 `extension/manifest.json` 의 `version` 을 `package.json` 버전으로 덮어씀 (Chrome MV3 numeric-only 버전 강제)
  3. `(cd extension && zip -r ../dist-extension/llm-wiki-extension-${APP_VERSION}.zip . -x "*.DS_Store")`
  4. `gh release upload "${{ github.ref_name }}" dist-extension/*.zip --clobber`

## 핵심 관찰

- **Single source of truth for version**: `package.json` → vite `__APP_VERSION__`, Cargo (수동 sync — drift 위험), Tauri conf (수동 sync), extension/manifest.json (CI 가 push 시점에 덮어씀). 90-risks-gaps 가 drift 가능성 추적해요.
- **`unwind` panic + panic_guard**: 의도적 architecture choice — 04-backend-rust 의 Internal Risk 와 연결.
- **Apple 서명 secrets**: PR 빌드는 서명 못 함 (secrets 없음). manual workflow_dispatch 도 unsigned bundle 만 생산.
- **CI 가 vite build + cargo build 만**: 풀 Tauri 번들 검증은 release 까지 안 함 — drift 위험 후보.
- **테스트 mocks vs llm 분리**: PR 마다 real-llm 안 돌림 (CI workflow 에 `npm test` 호출 없음 — 추가 검증 필요. `.github/workflows/ci.yml` 은 `vite build` + `cargo build` 만 봄).

## Cross-refs

- 의존성/버전: [01-tech-stack.md](01-tech-stack.md)
- 시스템 경계 + IPC: [02-architecture.md](02-architecture.md)
- 패닉/언세이프 처리: [04-backend-rust.md#internal-risk](04-backend-rust.md)
- 시험 분류: [31-testing-strategy.md](31-testing-strategy.md) (부재 시 50-source-mapping 의 .test 행 분류로 대체)
- 확장 trust + zip 패키징: [05-extension.md](05-extension.md)
