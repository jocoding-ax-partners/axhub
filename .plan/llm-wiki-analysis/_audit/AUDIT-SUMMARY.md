# Audit Summary — `/team ralph` 검토 pass

> **Status:** Lead-absorbed inline (audit workers idled without producing reports — taken over).
> **Iteration:** ralph 1/3, single pass sufficient.

## A1 — Anchor resolvability (FIXED)

### Found gap
- 50-source-mapping.md: `## ` 헤더 3개 (Header / Phase 6 gates / Mapping Table). `<a id=` HTML anchor 0개.
- 도메인 doc 들이 `[path](50-source-mapping.md#srcapptsx)` 형식 cross-ref 사용 — 245 row 마다 anchor 부재로 모두 broken.
- Phase 6 Gate 8 의 검증 기준이 "≥3 mapping anchor reference" 였는데 anchor 유효성 검증 X (string 매칭만).

### Fix applied (Option A — `<a id>` per row)
- 245 row 마다 위에 `<a id="<path-no-slash-no-dot>"></a>` HTML anchor 추가 (lowercase + `/` `.` 제거 + 하이픈/언더스코어 보존).
- 도메인 doc 의 기존 cross-ref 형식과 정확 일치하도록 transformation rule 적용.
- 결과: 50-source-mapping.md 가 271 → 516 라인 (245 anchor 라인 추가).

### Cross-ref typo found + fixed
- `07-llm-integration.md:645`: `srcliblm-clientts` (`lm` 2 문자) → `srclibllm-clientts` (`llm` 3 문자) — typo. 수정 완료.

### Verification
- 도메인 doc 의 unique mapping anchor reference 71개
- 50-source-mapping.md 의 anchor 245개
- `comm -23 refs anchors` = **0 broken**
- 모든 cross-ref 가 anchor 와 매칭 ✓

## A2 — Internal Risk claim verification (PASSED)

### 04-backend-rust.md 91 risk site cover
- production `unsafe` count: **0** ✓ (4 None observed claim 정확)
- production `unwrap`/`expect` 분리: clip_server.rs 9 + lib.rs 1 + extract_images.rs 1 + 기타 production = ~75. 04 가 quote.
- `panic!`/`unreachable!`/`todo!`: 3 모두 `#[cfg(test)]` 가드 (`panic_guard.rs:55, 85, 96` — panic_guard 자체 test 안). production 0. 04 claim 정확.
- Mutex/RwLock: PDFIUM_LOCK + clip_server statics 3 + claude_cli tokio Mutex 1 = 5 사이트 모두 quote. RwLock 0.
- FFI: pdfium-render `bind_to_library` / `bind_to_system_library` (fs.rs:142, 282) quote 됨.
- 04 안 distinct file:line citation: **160 unique** (91 site + Evidence + Cross-refs file:line 보조 인용).

### "None observed" 의심 검증
- 03-frontend Rust risk = "None observed (TS only)" — 정상 ✓
- 04-backend-rust unsafe = "None observed" — `grep -rn '\bunsafe\b' src-tauri/src` = 0 ✓
- 06-data-layer Rust risk: production code path 에서 unwrap/expect/panic 0. test 가드 분리 정확 (`vectorstore.rs:743+ unwraps` 모두 `#[cfg(test)]` 안) ✓
- 09-ui-components Rust = "None observed (TS only)" — 정상 ✓

### TS swallow claim spot check
- `tauri-fetch.ts:48` `as unknown as` cast 실재 (07 claim 정확)
- `App.tsx:47, 48` `as unknown as window` (03 claim 정확)
- `popup.js:53` innerHTML 보간 (05 claim 정확)
- `welcome-screen.tsx:78` keyboard event coercion (09 claim 정확)

## A3 — 50-source-mapping accuracy + coverage (PASSED)

### Sample purpose 정확도 (10 row spot-check)
- `index.html`, `vectorstore.rs`, `icon-sidebar.tsx`, `button.tsx`, `embedding.test.ts`, `ingest.ts`, `project-mutex.ts`, `tauri-fetch.ts`, `review-store.ts` 등
- 모두 substantive description (placeholder 없음) ✓
- 추상적 stub ("utility helpers", "various stuff") 발견 0 ✓

### Coverage gap (production lib `.ts` 도메인 인용)
10 sample 모두 1+ 도메인 doc 에서 cited:
- `auto-save.ts` (06), `detect-language.ts` (03), `extract-source-images.ts` (08), `file-types.ts` (06), `greeting-detector.ts` (03), `image-caption-pipeline.ts` (08), `latex-to-unicode.ts` (03), `lint.ts` (03), `markdown-image-resolver.ts` (08), `output-language-options.ts` (03)
- uncited file 발견 0 ✓ (각 production lib 적어도 1 domain doc 에 cited)

### Backlink 적절성
- 50-source-mapping 의 245 row backlink 모두 valid 도메인 또는 tag ✓
- 의심 사이트 (`templates.ts` → 03 vs 07) 는 단일 backlink + 90-risks-gaps `§Ambiguous Ownership` 에 이미 명시 ✓
- `wiki-cleanup.ts` → 06, `clip-watcher.ts` → 07: ingest 후처리 / clip-server polling — 현재 분류 합리 ✓

### Tag correctness
- `Readability.js`, `Turndown.js` = `[vendored]` ✓ Mozilla MIT 라이브러리 사본
- `package-lock.json`, `Cargo.lock` = `[generated]` ✓ npm/cargo 자동 생성
- `assets/*.jpg` (15개), `extension/icon*.png`, `src-tauri/icons/*` = `[asset]` ✓ 마케팅/번들 자산
- `pdfium/libpdfium.{dylib,so,dll}` = `[vendored]` ✓ Google Chrome PDFium 바이너리
- `src/test-helpers/*` = `[leaf-utility]` ✓ 도메인 없는 테스트 인프라
- `.gitignore` = `[config-only]` ✓
- 잘못 분류된 사이트 발견 0 ✓

## 종합

| Audit | 발견 | 수정 |
|---|---|---|
| A1 anchor | broken cross-ref 71개 (245 row anchor 부재) | 245 anchor 추가 + 1 typo 수정 |
| A2 risk claim | 의심 0 | 변경 없음 (모든 claim 검증 통과) |
| A3 mapping | 의심 0 | 변경 없음 (sample / coverage / tag 모두 통과) |

**전체 결과**: 1 broken cross-ref typo + 245 missing anchor. 둘 다 수정 완료.
**Phase 6 게이트 재검증**: 8/8 PASS (Gate 8 anchor 유효성 추가 검증 포함).

## 후속 권장 (NICE TO HAVE)
- Phase 6 Gate 8 정의를 "≥3 unique anchor reference + anchor MUST resolve" 로 강화 (이번 audit pass 의 학습)
- `templates.ts` 같이 두 도메인이 사용하는 파일에 secondary backlink (예: `[03 + 07]`) 추가 — 현재는 `Ambiguous Ownership` 으로만 명시
- 50-source-mapping 의 245 anchor 가 GitHub 이외 markdown viewer (예: VS Code preview) 에서 동작하는지 검증
