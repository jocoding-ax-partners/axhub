# 01 — Tech Stack & Versions

> **Source pin:** `nashsu/llm_wiki@1434e08` — 모든 버전은 lockfile / manifest 직접 인용해요. 추측 없어요.

## App identity

| 키 | 값 | 출처 |
|---|---|---|
| 제품명 | LLM Wiki | `package.json:2`, `tauri.conf.json:3` |
| 버전 | 0.4.3 | `package.json:4`, `Cargo.toml:3`, `tauri.conf.json:4` |
| Bundle ID | `com.llmwiki.app` | `tauri.conf.json:5` |
| Module type | ES module | `package.json:5` (`"type": "module"`) |
| Crate edition | Rust 2021 | `Cargo.toml:6` |
| Crate name | `llm-wiki` (binary), `llm_wiki_lib` (library) | `Cargo.toml:11-16` |
| Crate types | `staticlib`, `cdylib`, `rlib` | `Cargo.toml:12` |

## Runtime targets

| 환경 | 파일 | 핵심 설정 |
|---|---|---|
| Node CI | `.github/workflows/ci.yml:47` | `actions/setup-node@v4` with `node-version: 20` |
| Node Build | `.github/workflows/build.yml:55` | 동일 — Node 20 |
| Rust toolchain | `.github/workflows/{ci,build}.yml` | `dtolnay/rust-toolchain@stable` (CI), aarch64-apple-darwin target on macos-latest (Build) |
| OS matrix | CI: macos-latest / ubuntu-22.04 / windows-latest. Build: 동일. | `.github/workflows/ci.yml:14`, `build.yml:18-26` |
| Build target window 크기 | 1200×800 resizable | `tauri.conf.json:13-19` |

`.tool-versions` 파일은 repo 루트에 없어요. node 버전은 CI YAML 에만 박혀 있고 mise/asdf 통제 없어요.

## Frontend dependencies (`package.json`)

### Runtime deps

| Package | 버전 | 비고 |
|---|---|---|
| `react`, `react-dom` | `^19.0.0` | React 19 — `.tsx` `jsx: react-jsx` 사용 (`tsconfig.app.json:15`) |
| `@tauri-apps/api` | `^2.10.1` | Tauri 2 IPC client |
| `@tauri-apps/plugin-dialog` | `^2.7.0` | 네이티브 대화상자 |
| `@tauri-apps/plugin-http` | `^2.5.8` | http 권한 (CSP `connect-src` 풀어둠 → `tauri.conf.json:23`) |
| `@tauri-apps/plugin-opener` | `^2.5.3` | 외부 URL/파일 열기 |
| `@tauri-apps/plugin-store` | `^2.4.2` | 영속 KV 저장소 |
| `@base-ui/react` | `^1.3.0` | 헤드리스 primitives |
| `@fontsource-variable/geist` | `^5.2.8` | 가변 폰트 |
| `@milkdown/{kit,plugin-math,react,theme-nord}` | `^7.20.0` (math `^7.5.9`) | 마크다운 에디터 + KaTeX 통합 |
| `@react-sigma/core` | `^5.0.6` | 그래프 뷰 React wrapper |
| `sigma` | `^3.0.2` | 그래프 렌더 엔진 |
| `graphology` | `^0.26.0` | 그래프 자료구조 |
| `graphology-communities-louvain` | `^2.0.2` | Louvain 커뮤니티 탐지 (knowledge-graph 영역의 기둥) |
| `graphology-layout-forceatlas2` | `^0.10.1` | force layout |
| `i18next` | `^26.0.3` | 국제화 |
| `react-i18next` | `^17.0.2` | React 바인딩 |
| `katex` | `^0.16.45` | 수식 렌더 |
| `rehype-katex` | `^7.0.1` | remark/rehype 파이프 |
| `remark-gfm` | `^4.0.1` | GFM |
| `remark-math` | `^6.0.0` | math 토큰 |
| `react-markdown` | `^10.1.0` | 마크다운 → React |
| `react-resizable-panels` | `^4.9.0` | 분할 레이아웃 |
| `tailwindcss` | `^4.2.2` | Tailwind 4 (`@tailwindcss/vite ^4.2.2`) |
| `tailwind-merge` | `^3.5.0` | 클래스 병합 |
| `tw-animate-css` | `^1.4.0` | tailwind 애니메이션 |
| `class-variance-authority` | `^0.7.1` | 변형 클래스 헬퍼 |
| `clsx` | `^2.1.1` | conditional class 헬퍼 |
| `lucide-react` | `^1.7.0` | 아이콘 |
| `shadcn` | `^4.1.2` | shadcn CLI/registry |
| `zustand` | `^5.0.12` | 상태 관리 (`src/stores/*-store.ts` 6개) |

### Dev deps

| Package | 버전 | 비고 |
|---|---|---|
| `@tauri-apps/cli` | `^2.10.1` | `tauri` CLI |
| `@types/node` | `^25.5.2` | (Node 20 런타임 vs 타입 25 — 불일치 주의) |
| `@types/react`, `@types/react-dom` | `^19.0.0` | React 19 타입 |
| `@vitejs/plugin-react` | `^6.0.1` | React 플러그인 |
| `fast-check` | `^4.7.0` | 속성 기반 테스트 (`*.property.test.ts`) |
| `typescript` | `^5.7.3` | TS 컴파일러 |
| `vite` | `^8.0.0` | 번들러 |
| `vitest` | `^4.1.4` | 테스트 러너 (mocks vs real-llm 분리) |

## Rust dependencies (`src-tauri/Cargo.toml`)

| Crate | 버전 / features | 역할 |
|---|---|---|
| `tauri` | `2` (features: `protocol-asset`) | 코어 |
| `tauri-build` | `2` (build-dep) | `build.rs:2` 에서 호출 |
| `tauri-plugin-opener` | `2` | 외부 열기 |
| `tauri-plugin-dialog` | `2.7.0` | 대화상자 |
| `tauri-plugin-store` | `2.4.2` | KV 저장소 |
| `tauri-plugin-http` | `2` (features: `unsafe-headers`) | **`unsafe-headers` 활성** — 임의 헤더 허용. 보안 trade-off |
| `serde` | `1` (`derive`) | 직렬화 |
| `serde_json` | `1` | JSON |
| `chrono` | `0.4` (`clock`) | 시간 |
| `pdfium-render` | `0.9` | **PDF 추출 — pdfium FFI 통로** |
| `tiny_http` | `0.12` | clip_server.rs 내장 HTTP 서버 (extension webclipper 용 endpoint `127.0.0.1:19827`) |
| `zip` | `2` | 압축 처리 |
| `calamine` | `0.34.0` | 엑셀/스프레드시트 파서 |
| `docx-rs` | `0.4.20` | docx 파서 |
| `lancedb` | `0.27.2` | **벡터 DB** (LanceDB Rust 직접 사용) |
| `arrow-array`, `arrow-schema` | `57` | LanceDB 컬럼 백엔드 |
| `futures` | `0.3` | 비동기 trait |
| `tokio` | `1` (features: `process`, `io-util`, `sync`, `macros`, `rt`) | **claude-cli 서브프로세스 IO + 비동기 명령** |
| `which` | `7` | `claude` 바이너리 PATH lookup |
| `uuid` | `1` (`v4`) | ID 생성 |
| `image` | `0.25` (no default, features: `png`) | pdfium RGBA → PNG 재인코딩 (Phase 1 멀티모달) |
| `base64` | `0.22` | Tauri IPC JSON 페이로드 (Vec<u8> → base64) |
| `sha2` | `0.10` | 이미지 dedup 캐시 (Phase 3) |

### Release profile (`Cargo.toml:63-71`)

```toml
[profile.release]
codegen-units = 1
lto = true
opt-level = "s"           # size-optimized
panic = "unwind"          # ← panic_guard.rs 가 잡으려면 unwind 필수
strip = true
```

> 주석에 명시: `panic = "unwind"` 는 panic_guard 가 third-party 파서 panic 을 Tauri command 경계에서 Result 로 전환할 수 있게 함. abort 였다면 단일 파일 손상이 앱을 죽임.

## Chrome extension (`extension/manifest.json`)

| 키 | 값 |
|---|---|
| `manifest_version` | 3 (MV3) |
| 이름 | LLM Wiki Clipper |
| 버전 | 0.1.0 — **앱 0.4.3 과 따로 관리됨**, build.yml 이 release 시점에 `package.json` 버전으로 덮어씀 |
| `permissions` | `["activeTab", "scripting"]` (좁음 — 좋음) |
| `host_permissions` | `["http://127.0.0.1:19827/*"]` — Tauri 호스트 endpoint (`tiny_http` 내장 서버) |
| `web_accessible_resources` | `Readability.js`, `Turndown.js` for `<all_urls>` |

## Tauri config (`tauri.conf.json` + 플랫폼별)

| 키 | 값 |
|---|---|
| schema | `https://schema.tauri.app/config/2` |
| `devUrl` | `http://localhost:1420` |
| `frontendDist` | `../dist` |
| `beforeDevCommand` | `npm run dev` |
| `beforeBuildCommand` | `npm run build` |
| 윈도우 | 1200×800, resizable, not fullscreen |
| CSP | `default-src 'self'; connect-src 'self' https: http:; img-src 'self' asset: https://asset.localhost; media-src 'self' asset: https://asset.localhost; style-src 'self' 'unsafe-inline'` — `connect-src` 가 `https:` `http:` 모두 허용 (LLM 임의 endpoint 지원), `style-src` 에 `unsafe-inline` 있음 |
| Asset protocol | enabled, `scope: ["**"]` (전체 파일시스템 접근 — `tauri-plugin-http unsafe-headers` 와 함께 신뢰 경계 약점 후보) |
| 번들 타깃 | `"all"` (msi/nsis/dmg/deb/AppImage) |
| 아이콘 | 32×32, 128×128, 128×128@2x, .icns, .ico |

### 플랫폼별 번들

| OS | 파일 | pdfium 처리 |
|---|---|---|
| macOS | `tauri.macos.conf.json` | `frameworks: ["pdfium/libpdfium.dylib"]` |
| Linux | `tauri.linux.conf.json` | `resources: ["pdfium/libpdfium.so"]` |
| Windows | `tauri.windows.conf.json` | `resources: ["pdfium/pdfium.dll"]` |

### Tauri capabilities (`src-tauri/capabilities/default.json`)

```json
"permissions": [
  "core:default",
  "opener:default",
  "dialog:default",
  "store:default",
  { "identifier": "http:default", "allow": [
    {"url": "http://*"}, {"url": "http://*/*"}, {"url": "http://*:*"}, {"url": "http://*:*/*"},
    {"url": "http://**"}, {"url": "https://*"}, {"url": "https://*/*"},
    {"url": "https://*:*"}, {"url": "https://*:*/*"}, {"url": "https://**"}
  ]}
]
```

> http allowlist 가 사실상 전 인터넷 — LLM provider 임의 URL 지원 위함. 신뢰 경계는 사용자 입력에서 시작함.

## TypeScript config

| 파일 | target | lib | 모듈 시스템 | 핵심 옵션 |
|---|---|---|---|---|
| `tsconfig.json` (project refs) | n/a | n/a | n/a | references → `app` + `node` |
| `tsconfig.app.json` | ES2020 | ES2020 + DOM + DOM.Iterable | bundler resolution, isolatedModules, jsx react-jsx | `strict`, `noUnusedLocals`, `noUnusedParameters`, `noFallthroughCasesInSwitch`, `noEmit`, `useDefineForClassFields`, `moduleDetection: force`, path alias `@/*` → `./src/*` |
| `tsconfig.node.json` | ES2022 | ES2023 | 동일 bundler | `vite.config.ts` 만 포함 |

## Vite config (`vite.config.ts`)

- 플러그인: `@vitejs/plugin-react`, `@tailwindcss/vite`
- alias: `@` → `./src`
- 정의: `__APP_VERSION__` = package.json version (Settings UI 가 단일 소스로 사용 — `vite.config.ts:11,22`)
- Dev server: `port: 1420` strict, `clearScreen: false` (Rust 에러 안 가림)
- HMR: `TAURI_DEV_HOST` env 있을 때만 ws 모드 활성 (모바일/원격 dev)
- Watch ignore: `**/src-tauri/**` (Rust 빌드는 Cargo 가 watch)
- Vitest: `environment: node`, setupFiles `./src/test-helpers/load-test-env.ts` (`.env.test.local` 로더)

## shadcn config (`components.json`)

- style: `base-nova`
- tsx: true, rsc: false
- baseColor: `neutral`, cssVariables: true
- iconLibrary: `lucide`
- aliases: components → `@/components`, utils → `@/lib/utils`, ui → `@/components/ui`, lib → `@/lib`, hooks → `@/hooks`

## CI / Build summary

| Workflow | 트리거 | 런너 | 단계 핵심 |
|---|---|---|---|
| `ci.yml` (CI) | push/PR to main | macos-latest / ubuntu-22.04 / windows-latest | rust-toolchain stable → protoc 설치 (각 OS 분기) → libwebkit2gtk on Ubuntu → Swatinem/rust-cache (workspaces: src-tauri) → Node 20 → `npm install` → `npx vite build` → `cargo build` (`src-tauri/`) |
| `build.yml` (Build & Release) | tag `v*` push, `workflow_dispatch` | 동일 매트릭스 + macOS aarch64 target | 동일 setup → `tauri-apps/tauri-action@v0` 로 번들. Apple 서명 시크릿 (CERT/PASS/SIGNING_IDENTITY/ID/PASSWORD/TEAM_ID) 사용. Tag push 시 GitHub Release 생성 + 자산 업로드. workflow_dispatch 시 release skip + `actions/upload-artifact` 로 .msi/.exe/.dmg/.deb/.AppImage 업로드 (14일 보관). 추가 job `package-extension` 이 `extension/manifest.json` 의 version 을 package.json 으로 덮어쓰고 zip 만들어 release 에 첨부 |

## 핵심 highlights

- **Tauri 2** + **React 19** + **Tailwind 4** — 모두 최신 메이저
- **Milkdown 7.20** — MDX 풍 마크다운 에디터 + 수식 + nord 테마
- **Sigma 3** + **graphology 0.26** + **Louvain communities 2.0** — knowledge-graph 의 핵심 의존성
- **LanceDB 0.27** — Rust 측에서 직접 사용 (TS 측은 Tauri command 경유)
- **pdfium-render 0.9** — Rust 에서 PDF 추출, plat-별 vendor 라이브러리 번들
- **i18next 26** + **react-i18next 17** — en + zh 두 로케일 (`src/i18n/`)
- **fast-check 4** — `.property.test.ts` 속성 기반 테스트
- **`unsafe-headers`** feature on `tauri-plugin-http` + **`scope: ["**"]`** on assetProtocol — 신뢰 경계가 의도적으로 넓음. 04-backend-rust 와 06-data-layer 의 Internal Risk 섹션과 cross-ref 해요.

## Cross-refs

- 빌드/번들 흐름: [80-build-and-tooling.md](80-build-and-tooling.md)
- 시스템 경계: [02-architecture.md](02-architecture.md#process-boundaries)
- Rust risk surface: [04-backend-rust.md#internal-risk](04-backend-rust.md)
- 확장 trust boundary: [05-extension.md#internal-risk](05-extension.md)
