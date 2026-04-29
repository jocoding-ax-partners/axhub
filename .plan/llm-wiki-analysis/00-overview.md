# 00 — llm_wiki Analysis Overview (TOC)

> **Status:** **FINALIZED** (Phase 7 complete). 모든 8 Phase 6 mechanical gate PASS.
> **Source:** `nashsu/llm_wiki@1434e08` (245 files; 91 Rust risk sites locked).
> **Bar 만족 확인:** floor (245 mapped rows × ≥80 char purpose × valid backlink) ✓ + ceiling (5-section template × 7 domain docs) ✓.

## Quick Start (3 분 reading order)

새로 보는 사람:
1. [99-summary.md](99-summary.md) — System Purpose + Top 5 Risks + What's Solid
2. [02-architecture.md](02-architecture.md) — process boundary 다이어그램 + 3 데이터 파이프라인
3. [50-source-mapping.md](50-source-mapping.md) — 245 행 flat index 로 관심 파일 찾기 → 해당 backlink 따라 도메인 doc 으로

디버깅하는 사람:
1. [90-risks-gaps.md](90-risks-gaps.md) — Top 5 + Untested Paths + Suspicious "None observed"
2. 의심 도메인 doc Internal Risk + Evidence 섹션 (file:line 인용)
3. [50-source-mapping.md](50-source-mapping.md) reverse lookup

리팩토링/포팅하는 사람:
1. [PLAN.md §6 ADR](PLAN.md#6-adr) — 분석 결정 사항
2. [01-tech-stack.md](01-tech-stack.md) — 모든 버전 pin
3. [02-architecture.md](02-architecture.md) — 신뢰 경계
4. [80-build-and-tooling.md](80-build-and-tooling.md) — 빌드/CI 의존성

## Reading Order
1. `01-tech-stack.md` — versions, runtime targets
2. `02-architecture.md` — process boundaries, IPC, data flow
3. Domain docs (10..18 — renamed from clusters)
4. `50-source-mapping.md` — exhaustive flat index (the "빠짐없이" floor)
5. `90-risks-gaps.md` — observed gaps + tooling failures
6. `99-summary.md` — TLDR + top 5 risks

## Files in this directory

| File | Purpose |
|------|---------|
| [PLAN.md](PLAN.md) | Approved consensus plan + ADR |
| [_template.md](_template.md) | Canonical 5-section schema for domain docs |
| [00-overview.md](00-overview.md) | This file (TOC) |
| [01-tech-stack.md](01-tech-stack.md) | Tauri 2 / React 19 / TS / Rust / Chrome ext versions |
| [02-architecture.md](02-architecture.md) | Process boundaries, data flow, IPC topology |
| [03-frontend.md](03-frontend.md) | React app shell, routing, state |
| [04-backend-rust.md](04-backend-rust.md) | Tauri commands, FFI, panic_guard — PRIMARY risk doc |
| [05-extension.md](05-extension.md) | Chrome MV3 webclipper |
| [06-data-layer.md](06-data-layer.md) | Storage, persistence, IPC payload schemas |
| [07-llm-integration.md](07-llm-integration.md) | Provider clients, prompt assembly, streaming |
| [08-pdf-ocr-pipeline.md](08-pdf-ocr-pipeline.md) | pdfium FFI, OCR flow, ingestion |
| [09-ui-components.md](09-ui-components.md) | Reusable components, design tokens |
| [50-source-mapping.md](50-source-mapping.md) | Exhaustive 245-row flat index |
| [80-build-and-tooling.md](80-build-and-tooling.md) | Vite, Cargo, Tauri bundle, CI |
| [90-risks-gaps.md](90-risks-gaps.md) | Open questions, tooling failures |
| [99-summary.md](99-summary.md) | Executive summary |

## Domain Mapping (Phase 2 — gitnexus cluster-derived)

> **gitnexus status:** SUCCESS. Indexed `/private/tmp/llm_wiki_inspect` @ `1434e08` → 1,525 nodes / 4,299 edges / **154 communities** / 119 processes / 214 source-graph files (configs/assets excluded by graph builder).
> **Method:** Top-20 communities ranked by `symbolCount` mapped onto best-guess domain doc slots (03..09). Slots 10..18 NOT created (top clusters fit existing slots cleanly; reserved range stays empty).
> **Long-tail:** ~140 small clusters (≤8 symbols each) absorb into best-fit domain doc OR get `[leaf-utility]` tag in `50-source-mapping.md`.

| gitnexus cluster | symbolCount | cohesion | Assigned domain doc |
|---|---:|---:|---|
| comm_46 `Sources` | 52 | 0.59 | [06-data-layer.md](06-data-layer.md) (sources = persisted wiki content + ingest cache) |
| comm_41 `Extension` | 36 | 0.86 | [05-extension.md](05-extension.md) |
| comm_21 `Commands` | 24 | 0.87 | [04-backend-rust.md](04-backend-rust.md) (Tauri command surface) |
| comm_91 `Layout` | 24 | 0.61 | [09-ui-components.md](09-ui-components.md) |
| comm_93 `Layout` | 19 | 0.73 | [09-ui-components.md](09-ui-components.md) |
| comm_58 `Ui` | 17 | 1.00 | [09-ui-components.md](09-ui-components.md) |
| comm_12 `Layout` | 15 | 0.83 | [09-ui-components.md](09-ui-components.md) |
| comm_140 `Commands` | 15 | 0.78 | [04-backend-rust.md](04-backend-rust.md) |
| comm_18 `Cluster_18` | 14 | 0.71 | [03-frontend.md](03-frontend.md) (unlabeled mid-cluster, frontend by inspection) |
| comm_42 `Extension` | 13 | 0.72 | [05-extension.md](05-extension.md) |
| comm_22 `Commands` | 12 | 0.71 | [04-backend-rust.md](04-backend-rust.md) |
| comm_97 `Cluster_97` | 10 | 0.95 | [03-frontend.md](03-frontend.md) |
| comm_141 `Commands` | 10 | 1.00 | [04-backend-rust.md](04-backend-rust.md) |
| comm_143 `Commands` | 10 | 0.79 | [04-backend-rust.md](04-backend-rust.md) |
| comm_55 `Cluster_55` | 9 | 0.74 | [03-frontend.md](03-frontend.md) |
| comm_60 `Cluster_60` | 9 | 0.94 | [03-frontend.md](03-frontend.md) |
| comm_62 `Commands` | 9 | 0.89 | [04-backend-rust.md](04-backend-rust.md) |
| comm_63 `Settings` | 9 | 0.59 | [06-data-layer.md](06-data-layer.md) |
| comm_80 `Cluster_80` | 9 | 0.71 | [03-frontend.md](03-frontend.md) |
| comm_92 `Lint` | 9 | 0.61 | [03-frontend.md](03-frontend.md) |

### Domain doc → cluster coverage

| Domain doc | Clusters absorbed | Combined symbol count |
|---|---|---:|
| 03-frontend.md | comm_18, comm_55, comm_60, comm_80, comm_92, comm_97 | ~60 |
| 04-backend-rust.md | comm_21, comm_22, comm_62, comm_140, comm_141, comm_143 | ~80 |
| 05-extension.md | comm_41, comm_42 | ~49 |
| 06-data-layer.md | comm_46 (Sources), comm_63 (Settings) | ~61 |
| 07-llm-integration.md | _file-driven (smaller clusters: src/lib/llm-*.ts, claude-cli-transport)_ | n/a |
| 08-pdf-ocr-pipeline.md | _file-driven (vision-caption, extract_images, pdfium FFI)_ | n/a |
| 09-ui-components.md | comm_12, comm_58, comm_91, comm_93 | ~75 |

Total mapped (top 20): **~325 symbols** of the 1,525 node graph (≈21%). Long-tail (134 small clusters) absorbed into best-fit domain or `[leaf-utility]` tag during Phase 5 mapping.

## Locked Numbers
- **Denominator:** 245 source files (excl `.git/`, `node_modules/`, `target/`, `dist/`, `build/`).
- **Rust risk sites:** 91 matches of `unsafe \| .unwrap() \| .expect( \| panic! \| Mutex:: \| RwLock::` across `src-tauri/src/**`.
