# 08 — PDF / OCR / Multimodal Ingestion Pipeline

> **Status:** Phase 4 populated. FFI-adjacent doc — pdfium 진입점은 `04-backend-rust.md` 가 1차 risk doc 이고 여기는 caption / dedup / markdown rewrite 측을 다뤄요.

## Purpose

PDF/Office 문서에서 이미지를 뽑아 vision-capable LLM 으로 캡셔닝하고, ingest LLM 이 보는 source markdown 의 `![](path)` 참조에 alt-text 를 채워 넣는 phase 1 + phase 3 멀티모달 파이프라인이에요. 책임은 (1) Rust `extract_and_save_pdf_images_cmd` / `extract_and_save_office_images_cmd` 호출로 raster 이미지를 `<project>/wiki/media/<slug>/img-<N>.<ext>` 에 disk 로 직접 저장, (2) `vision-caption.captionImage` 가 base64 + mediaType + 주변 텍스트 ±150 자 컨텍스트로 vision endpoint 한 번 호출, (3) `image-caption-pipeline.captionMarkdownImages` 가 SHA-256 기반 dedup cache (`<project>/.llm-wiki/image-caption-cache.json`) 를 갱신하면서 markdown 내 모든 image 참조를 enrich, (4) `markdown-image-resolver` 가 webview 렌더링 시 wiki-root-relative URL 을 `convertFileSrc` 절대 경로로 매핑이에요. 외부 신뢰 경계 두 곳: PDFium dynamic library FFI (Rust 측, 04 doc 에서 다룸) 와 vision LLM endpoint (HTTP outbound, 07-llm-integration.md 에서 다룸). 이 도메인의 핵심 invariant: per-image 비용이 SHA-256 dedup 으로 모든 corpus 에 걸쳐 한 번씩만 발생하고, 단일 이미지 caption 실패가 전체 batch 를 abort 시키지 않아야 해요.

## Public Interface

### TypeScript exports

- `captionImage — async (imageBase64: string, mediaType: string, llmConfig: LlmConfig, signal?: AbortSignal, options?: CaptionOptions) => Promise<string> — src/lib/vision-caption.ts:155 — 단일 이미지 + factual prompt 호출, plain-text caption 반환`
- `CAPTION_PROMPT — string — src/lib/vision-caption.ts:69 — context 없는 기본 프롬프트 (factual / 2-4 sentences / no markdown)`
- `buildCaptionPromptWithContext — (before: string, after: string) => string — src/lib/vision-caption.ts:83 — 주변 텍스트 ±150 자 wrapping, "이 텍스트가 관련있을 수도 없을 수도" 명시`
- `captionMarkdownImages — async (projectPath: string, markdown: string, llmConfig: LlmConfig, options?: CaptionPipelineOptions) => Promise<CaptionPipelineResult> — src/lib/image-caption-pipeline.ts:283 — markdown 내 모든 ![](...) 추출 후 SHA-256 dedup → caption → markdown rewrite`
- `loadCaptionCache — async (projectPath: string) => Promise<Map<string, string>> — src/lib/image-caption-pipeline.ts:80 — on-disk cache 읽어 hash → caption 맵 반환`
- `__test — { findImageReferences, sha256OfBase64, MD_IMAGE_RE } — src/lib/image-caption-pipeline.ts:467 — 직접 unit test 용 internal 노출`
- `extractAndSaveSourceImages — async (projectPath: string, sourcePath: string) => Promise<SavedImage[]> — src/lib/extract-source-images.ts:57 — Rust extract command dispatcher (확장자별 PDF/Office 분기)`
- `buildImageMarkdownSection — (images: SavedImage[], captionsBySha?: Map<string, string>) => string — src/lib/extract-source-images.ts:122 — 페이지 그룹별 "## Embedded Images" markdown 섹션 빌드`
- `resolveMarkdownImageSrc — (rawSrc: string, projectPath: string | null) => string — src/lib/markdown-image-resolver.ts:38 — passthrough → absolute → wiki-root-relative 분기 + `convertFileSrc` 변환`

### Rust 측 진입점 (04-backend-rust.md 와 공유)

- `extract_pdf_markdown — fn(path: &str, media_dest_dir: Option<&Path>, media_url_prefix: &str, options: &ExtractOptions) -> Result<String, String> — src-tauri/src/commands/extract_images.rs:104 — 텍스트 + 페이지 헤더 + 이미지 ![](url) 인터리브 결합 markdown 생성`
- `extract_pdf_images — fn(path: &str, options: &ExtractOptions) -> Result<Vec<ExtractedImage>, String> — src-tauri/src/commands/extract_images.rs:245 — base64 직렬화된 PDF embedded raster 추출`
- `extract_and_save_pdf_images — fn(path: &str, dest_dir: &Path, rel_to: &Path, options: &ExtractOptions) -> Result<Vec<SavedImage>, String> — src-tauri/src/commands/extract_images.rs:640 — base64 round-trip 없이 disk 직접 저장`
- `extract_office_images — fn(path: &str, options: &ExtractOptions) -> Result<Vec<ExtractedImage>, String> — src-tauri/src/commands/extract_images.rs:347 — DOCX/PPTX/XLSX zip media/ 추출`
- `extract_and_save_office_images — fn(path: &str, dest_dir: &Path, rel_to: &Path, options: &ExtractOptions) -> Result<Vec<SavedImage>, String> — src-tauri/src/commands/extract_images.rs:754 — Office 이미지 disk 직접 저장`

### Disk schema

- `<project>/wiki/media/<slug>/img-<N>.<ext>` — extracted images. PDF 는 항상 PNG 재인코딩, Office 는 원본 codec 보존 (PNG/JPEG/GIF/WEBP/BMP).
- `<project>/.llm-wiki/image-caption-cache.json` — `{ "<sha256>": { caption, mimeType, model, capturedAt } }` JSON map.
- `<project>/.cache/<source-name>.txt` — fs.rs preprocess cache (text-only, mtime 비교).

### IPC payload shape

- `SavedImage` (Rust → TS, `#[serde(rename_all = "camelCase")]` 명시) — `{ index, mimeType, page (Option<u32>), width, height, relPath, absPath, sha256 }`. extract_images.rs:583 의 주석이 "Tauri 의 IPC auto-camelCase 는 COMMAND PARAMETER 만 적용되지 RETURN VALUE 에는 적용 안 됨" 을 명시.
- `ExtractedImage` (Rust → TS, base64 form) — `{ index, mimeType, page, width, height, dataBase64, sha256 }`.
- `FileBase64` (`read_file_as_base64` 반환) — `{ base64, mimeType }`.

## Internal Risk

### unsafe blocks (Rust)

```rust src-tauri/src/commands/extract_images.rs
None observed in this domain.
```

`extract_images.rs` 자체에는 `unsafe` 키워드 0 개. 단 PDFium dynamic library 가 dlopen 으로 로드되는 순간 (`fs.rs:282 pdfium()` → `Pdfium::bind_to_library`) 부터는 safe Rust 의 메모리 안전성 미신을 깨뜨리는 영역에 들어가요. PDFium C++ 구현의 corrupt input 에서 발생하는 panic 이 `panic_guard::run_guarded` 의 `catch_unwind` 로 잡혀 `Result::Err` 으로 변환되는 게 이 도메인의 핵심 안전 invariant 예요.

### `.unwrap()` / `.expect()` chains (Rust)

이 도메인에서 production code path 에 unwrap 이 1 개 있어요:

```rust src-tauri/src/commands/extract_images.rs:405
        let mime_type = mime_type.unwrap();
```

이건 직전 line `if mime_type.is_none() { continue; }` 로 가드되어 있어 panic 불가능한 false positive 예요. 다만 리팩터링 안전성 관점에서 `let Some(mt) = mime_type else { continue; }` 로 elision 하는 게 더 안전. 04 doc Internal Risk 에서도 동일 site 인용했어요.

### `panic!` / `unreachable!` / `todo!` (Rust)

```rust src-tauri/src/commands/extract_images.rs
None observed in this domain.
```

이 도메인의 production code 는 `panic!` 매크로를 쓰지 않아요. 대신 third-party 의존성이 panic 할 가능성을 인정하고 `panic_guard::run_guarded` 로 모든 Tauri command 진입점을 감싸요 (extract_images.rs:862, 873, 888, 908). pdfium-render 의 corrupt PDF 처리, `image::load_from_memory` 의 truncated PNG 처리, `zip::ZipArchive::new` 의 malformed central directory 처리 모두 각자의 Result 경로로 처리하지만, 만약 panic 으로 빠져도 catch_unwind 가 잡아요.

### `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)

이 도메인에서 직접 잡는 lock 은 PDFium 직렬화 mutex 한 곳 — `crate::commands::fs::lock_pdfium()` 호출 4 회예요. 이미 04 doc 에 PDFIUM_LOCK 정의는 quote 했고, 여기는 사용 사이트만 짚어요.

```rust src-tauri/src/commands/extract_images.rs:112
    let _guard = crate::commands::fs::lock_pdfium();
```
```rust src-tauri/src/commands/extract_images.rs:256
    let _guard = crate::commands::fs::lock_pdfium();
```
```rust src-tauri/src/commands/extract_images.rs:649
    let _guard = crate::commands::fs::lock_pdfium();
```

호출 사이트 모두 `let _guard = lock_pdfium();` 패턴으로 함수 끝까지 holds. `_guard` 명명이 explicit 해서 drop 시점이 명확해요. 모든 사이트가 `tauri::async_runtime::spawn_blocking` 안에서만 호출되어 `.await` 를 가로지르는 hold 가 없는 점도 확인 (extract_images.rs:861-867 — `spawn_blocking(move || { run_guarded(...) })` 패턴). 비재진입성 (`std::sync::Mutex` 가 reentrant 가 아니라서) 위반은 코멘트로 강조: extract_images.rs:103 의 "Holds the global pdfium lock for its full duration. Callers MUST NOT acquire the lock themselves before calling this (would deadlock — `std::sync::Mutex` is non-reentrant)." 그리고 fs.rs:323-326 의 `extract_pdf_text` 가 의도적으로 lock 을 잡지 않고 `extract_pdf_markdown` 에 위임하는 이유로 같은 invariant 강조.

`vision-caption` / `image-caption-pipeline` / `extract-source-images` / `markdown-image-resolver` 는 모두 TypeScript 라 Rust mutex 사용 0 개. 단 JS 측에서 cache 의 concurrent access 를 다루는 패턴은 image-caption-pipeline.ts:341-348 의 코멘트가 명시 — "JS is single-threaded within a microtask boundary, so the reads/writes themselves don't race, but two concurrent tasks computing the SAME hash may both see no entry and both call the LLM. That's fine — we just spend an extra call. The LATER write wins; both captions are valid anyway." 즉 race 가 functionally idempotent 라는 가정 위에서 lock-free 로 동작해요.

### FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)

이 도메인이 PDFium FFI 의 first-line 사용자예요. PDF 처리 진입점은 모두 `pdfium_render::prelude::*` 를 통해 native PDFium dynamic library 를 호출해요.

```rust src-tauri/src/commands/extract_images.rs:110
    use pdfium_render::prelude::*;
```
```rust src-tauri/src/commands/extract_images.rs:113
    let pdfium = crate::commands::fs::pdfium()?;
```
```rust src-tauri/src/commands/extract_images.rs:114
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| match e {
            PdfiumError::PdfiumLibraryInternalError(
                PdfiumInternalError::PasswordError,
            ) => format!("PDF is password-protected and cannot be read: '{path}'"),
            _ => format!("Failed to open PDF '{path}': {e}"),
        })?;
```
```rust src-tauri/src/commands/extract_images.rs:170
            let dyn_img = match image.get_raw_image() {
                Ok(b) => b,
                Err(e) => {
                    eprintln!(
                        "[extract_pdf_markdown] page {page_num} image read failed: {e}"
                    );
                    continue;
                }
            };
```
```rust src-tauri/src/commands/extract_images.rs:185
            if let Err(e) = dyn_img.write_to(
                &mut std::io::Cursor::new(&mut png_bytes),
                image::ImageFormat::Png,
            ) {
                eprintln!(
                    "[extract_pdf_markdown] page {page_num} PNG encode failed: {e}"
                );
                continue;
            }
```
```rust src-tauri/src/commands/extract_images.rs:646
    use pdfium_render::prelude::*;
```
```rust src-tauri/src/commands/extract_images.rs:651
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to open PDF '{path}': {e}"))?;
```
```rust src-tauri/src/commands/extract_images.rs:681
            let dyn_img = match image.get_raw_image() {
                Ok(b) => b,
                Err(e) => {
                    filtered_decode_err += 1;
                    eprintln!(
                        "[extract_and_save_pdf_images] page {} image read failed: {e}",
                        page_idx + 1
                    );
                    continue;
                }
            };
```

PDFium 로드 자체는 `crate::commands::fs::pdfium()` (fs.rs:282) 가 OnceLock 으로 캐시. 04 doc 에 verbatim quote 됨. 신뢰 경계: bundled `src-tauri/pdfium/libpdfium.{dylib,so,dll}` 파일 무결성에 의존. 만약 attacker 가 `$PDFIUM_DYNAMIC_LIB_PATH` 환경변수를 컨트롤할 수 있으면 임의 dylib 로드 → 코드 실행 가능. 데스크톱 앱 컨텍스트에선 user 가 이미 같은 권한이라 trust boundary 가 약하지만 elevated launch 시 LPE 벡터가 됨.

PPTX/DOCX zip 진입점 (extract_office_images, extract_and_save_office_images) 은 `zip::ZipArchive::new` (safe Rust) 를 통과해서 FFI 비포함이지만, `image::load_from_memory` 가 stb_image 계열 native 디코더를 wrap 하는 경우 같은 dlopen 경로를 따라가요. `image` crate 0.25 + `default-features = false, features = ["png"]` 설정 (Cargo.toml:56) 이라 PNG only 정적 링크 — Office 측 디코드는 PNG 외 포맷 (JPEG/GIF/WEBP/BMP) 을 만나면 `load_from_memory` 가 Err 반환 후 skip.

### Result swallow (TypeScript)

이 도메인에 production-path 에서 의도적으로 결과를 무시하는 패턴이 여러 개 있어요. 대부분 fault-tolerance 디자인이지만 audit 관점에서 명시.

```typescript src/lib/image-caption-pipeline.ts:103
  } catch (err) {
    // Corrupt cache (e.g. truncated mid-write before we added
    // atomic writes) — start fresh rather than wedging the whole
    // ingest pipeline. Log so it's visible in the activity feed.
    console.warn(
      `[caption-cache] corrupt cache at ${cachePath}, starting empty:`,
      err instanceof Error ? err.message : err,
    )
  }
  return {}
```

corrupt cache 시 silent fallback 으로 빈 cache 반환. console.warn 만 있고 rethrow 없음. user 입장에선 "왜 캡션 캐시 hit 이 안 되지?" 의 직접 단서가 없음 — `[caption-cache] corrupt cache` 메시지가 dev tools 콘솔에만 노출되어 production 사용자는 확인 불가.

```typescript src/lib/image-caption-pipeline.ts:362
    try {
      bytes = await readFileAsBase64(absPath)
    } catch (err) {
      console.warn(
        `[caption-pipeline] failed to read ${absPath}:`,
        err instanceof Error ? err.message : err,
      )
      failed++
      return
    }
```

```typescript src/lib/image-caption-pipeline.ts:402
    } catch (err) {
      console.warn(
        `[caption-pipeline] caption failed for ${absPath}:`,
        err instanceof Error ? err.message : err,
      )
      failed++
    }
```

per-image fault-tolerance — 한 이미지의 read 또는 caption 실패가 전체 batch 를 abort 시키지 않게 의도적으로 swallow. `failed` 카운터는 결과 객체에 포함되지만 ingest pipeline 의 activity feed 에만 "captioned 28/30 images — 2 failed" 형태로 노출되고 어떤 파일이 실패했는지는 console only.

```typescript src/lib/image-caption-pipeline.ts:434
    try {
      await writeCache(projectPath, cache)
    } catch (err) {
      console.warn(
        `[caption-pipeline] failed to persist cache:`,
        err instanceof Error ? err.message : err,
      )
    }
```

cache 저장 실패도 swallow. 영향: 다음 ingest 에서 모든 이미지를 다시 caption (cost N×). user 가 인지할 단서는 없음.

```typescript src/lib/extract-source-images.ts:96
  } catch (err) {
    console.warn(
      `[ingest:images] extraction failed for "${fileName}":`,
      err instanceof Error ? err.message : err,
    )
    return []
  }
```

extract 실패 시 빈 배열 반환 + console.warn. 주석 (extract-source-images.ts:51-53) 에 "image extraction failure must NEVER abort the ingest pipeline" 으로 의도 명시. 그러나 결과적으로 "이미지가 정말 없는 PDF" 와 "PDFium 가 segfault 직전이었던 PDF" 가 같은 `[]` 로 보고됨 — distinguishability 손실.

```typescript src/lib/markdown-image-resolver.ts
None observed in this file.
```

markdown-image-resolver 는 fall-through 가 명시적 (passthrough → absolute → wiki-rooted) 이라 swallow 패턴 없음. 단 `projectPath === null` 시 `rawSrc` 그대로 반환하는데 결과적으로 `<img src="media/foo.png">` 같은 broken URL 이 webview 에 들어가 silently 깨진 이미지로 렌더링됨 — 이건 "swallow" 라기보단 "context 없음 = best-effort" 디자인.

vision-caption.ts 는 의도적으로 streamError 를 rethrow 하므로 (vision-caption.ts:204-211) result swallow 없음. 단 streamChat 의 onError callback 으로만 에러를 받아 `streamError` 변수에 담는 패턴이 swallow 와 비슷한 모양 — 만약 onError 가 절대 호출되지 않고 streamChat 이 silent 하게 빈 토큰만 반환하는 edge case 가 있다면 user 는 빈 caption 을 받아요. 주석 (vision-caption.ts:204-211) 에 이 위험을 인지하고 있다고 명시: "Without this re-throw, a 500 from the vision endpoint silently produces empty caption text and the ingest pipeline indexes images as untitled."

## Cross-refs

- See [04-backend-rust.md#purpose](04-backend-rust.md#purpose) — PDFium 직렬화 mutex / FFI 로드 / panic_guard 의 catch_unwind 안전 invariant.
- See [04-backend-rust.md#internal-risk](04-backend-rust.md#internal-risk) — `extract_images.rs:405` unwrap 이 양쪽 doc 에서 공유 quote.
- See [07-llm-integration.md#purpose](07-llm-integration.md#purpose) — `streamChat` 호출 surface, `LlmConfig` 타입, vision-capable 모델 (Claude / GPT / Gemini / Qwen) 라우팅.
- See [06-data-layer.md#purpose](06-data-layer.md#purpose) — `<project>/.llm-wiki/image-caption-cache.json` JSON cache 와 `<project>/wiki/media/<slug>/` disk layout 이 데이터 레이어 schema 일부.
- See [03-frontend.md#purpose](03-frontend.md#purpose) — `convertFileSrc` 사용처, react `<img src=...>` 렌더링.
- Source rows:
  - [src-tauri/src/commands/extract_images.rs](50-source-mapping.md#src-taurisrccommandsextract_imagesrs) — Rust 측 PDF/Office 이미지 추출 + Tauri command bindings.
  - [src-tauri/src/commands/fs.rs](50-source-mapping.md#src-taurisrccommandsfsrs) — `pdfium()` / `lock_pdfium()` / `extract_pdf_text` 진입점.
  - [src/lib/vision-caption.ts](50-source-mapping.md#srclibvision-captionts) — `captionImage` + `CAPTION_PROMPT` + context-aware prompt builder.
  - [src/lib/image-caption-pipeline.ts](50-source-mapping.md#srclibimage-caption-pipelinets) — SHA-256 dedup cache + markdown rewrite + worker pool.
  - [src/lib/extract-source-images.ts](50-source-mapping.md#srclibextract-source-imagests) — TS 측 dispatch + path-shaping.
  - [src/lib/markdown-image-resolver.ts](50-source-mapping.md#srclibmarkdown-image-resolverts) — webview 렌더링 시 URL 변환.
  - [src-tauri/Cargo.toml](50-source-mapping.md#src-tauricargotoml) — `pdfium-render = "0.9"`, `image = { 0.25, default-features = false, features = ["png"] }`, `base64 = "0.22"`, `sha2 = "0.10"`.

## Evidence

- `src/lib/vision-caption.ts:69-70` — `CAPTION_PROMPT` const, factual / 2-4 sentences / no markdown 폼.
- `src/lib/vision-caption.ts:83-104` — `buildCaptionPromptWithContext` wrapper, "(none)" 폴백, "MAY help / MAY ALSO be unrelated" 명시.
- `src/lib/vision-caption.ts:106-137` — `CaptionOptions` interface, default `temperature: 0`, default `maxTokens: 4096` (reasoning model thinking budget 고려).
- `src/lib/vision-caption.ts:155-214` — `captionImage` 의 streamChat 콜백 + 에러 rethrow.
- `src/lib/image-caption-pipeline.ts:51` — cache 경로 `.llm-wiki/image-caption-cache.json` 상수.
- `src/lib/image-caption-pipeline.ts:63-71` — `sha256OfBase64` 가 base64 디코드 후 raw bytes 의 SHA-256 (base64 string 자체의 hash 가 아님).
- `src/lib/image-caption-pipeline.ts:91-110` — corrupt cache silent fallback.
- `src/lib/image-caption-pipeline.ts:141` — `MD_IMAGE_RE = /(!\[)([^\]]*)(\]\()([^)\s]+)(\))/g` regex, HTML / reference-style 미지원 명시.
- `src/lib/image-caption-pipeline.ts:176` — `CONTEXT_CHARS = 150` (이전 500 에서 조정).
- `src/lib/image-caption-pipeline.ts:283-462` — `captionMarkdownImages` 메인 로직, dedup → worker pool → markdown replace.
- `src/lib/image-caption-pipeline.ts:330-331` — concurrency `Math.max(1, options?.concurrency ?? 1)`.
- `src/lib/image-caption-pipeline.ts:341-348` — concurrent cache race 가 허용된다는 주석 ("LATER write wins; both captions are valid anyway").
- `src/lib/image-caption-pipeline.ts:413-423` — shared index pointer worker pool (no fancy queue).
- `src/lib/image-caption-pipeline.ts:444-460` — markdown rewrite 의 alt sanitization (`\r\n` → space, `]` → `)`).
- `src/lib/extract-source-images.ts:38-39` — `SUPPORTED_PDF_EXTS = ["pdf"]`, `SUPPORTED_OFFICE_EXTS = ["pptx", "docx", "ppt", "doc"]`.
- `src/lib/extract-source-images.ts:66-78` — Rust command dispatch (`extract_and_save_pdf_images_cmd` vs `extract_and_save_office_images_cmd`).
- `src/lib/extract-source-images.ts:79-95` — IPC payload validation (camelCase 필드 존재 검증, Rust 측 `#[serde(rename_all = "camelCase")]` 의존).
- `src/lib/extract-source-images.ts:122-174` — `buildImageMarkdownSection` 의 페이지 그룹 빌드 + "Document" (DOCX) 마지막 정렬.
- `src/lib/extract-source-images.ts:153-154` — alt sanitization (caption-pipeline 과 동일 룰).
- `src/lib/markdown-image-resolver.ts:31` — `PASSTHROUGH_RE = /^(https?:|data:|blob:|file:|tauri:)/i`.
- `src/lib/markdown-image-resolver.ts:38-65` — passthrough → absolute → wiki-rooted 분기 + `convertFileSrc` 호출.
- `src/lib/markdown-image-resolver.ts:48-49` — Windows drive letter (`/^[a-zA-Z]:/`) 와 UNC path (`\\\\`) 모두 absolute 로 인식.
- `src-tauri/src/commands/extract_images.rs:30-51` — `ExtractOptions` (min 100×100, max 500), 이전 corpus 측정 결과 기반 디폴트.
- `src-tauri/src/commands/extract_images.rs:53-77` — `ExtractedImage` 의 `#[serde(rename_all = "camelCase")]`.
- `src-tauri/src/commands/extract_images.rs:79-103` — `extract_pdf_markdown` doc 코멘트 (lock 비재진입성, `media_dest_dir = None` 분기).
- `src-tauri/src/commands/extract_images.rs:105-108` — 함수 시그니처 + media_url_prefix 컨벤션.
- `src-tauri/src/commands/extract_images.rs:110-122` — pdfium load + PasswordError 휴리스틱.
- `src-tauri/src/commands/extract_images.rs:130-238` — page-by-page 텍스트 + 이미지 인터리브 main loop.
- `src-tauri/src/commands/extract_images.rs:245-336` — `extract_pdf_images` (base64 form, 동일 lock + load 패턴).
- `src-tauri/src/commands/extract_images.rs:325-331` — `max_images` 캡 도달 시 `break 'pages` (pathological 5000-image PDF 보호).
- `src-tauri/src/commands/extract_images.rs:347-452` — Office zip 파싱 + image::load_from_memory dim 검증.
- `src-tauri/src/commands/extract_images.rs:456-462` — `is_media_path` 의 `ppt/media/` / `word/media/` / `xl/media/` 식별.
- `src-tauri/src/commands/extract_images.rs:464-479` — `guess_mime_from_name` 의 PNG/JPEG/GIF/WEBP/BMP only allowlist (SVG/EMF/WMF skip).
- `src-tauri/src/commands/extract_images.rs:503-561` — `build_pptx_media_slide_map` substring-기반 rels XML 파싱.
- `src-tauri/src/commands/extract_images.rs:583-602` — `SavedImage` `#[serde(rename_all = "camelCase")]` (TS 측 검증과 직결).
- `src-tauri/src/commands/extract_images.rs:604-622` — `save_one_image` 의 `create_dir_all` + `strip_prefix` rel/abs 계산.
- `src-tauri/src/commands/extract_images.rs:640-748` — `extract_and_save_pdf_images` + 진단 카운터 (`total_objects`, `total_image_objects`, `filtered_too_small`, `filtered_decode_err`, `filtered_encode_err`).
- `src-tauri/src/commands/extract_images.rs:847-919` — 4 Tauri command binding, 모두 `spawn_blocking` + `panic_guard::run_guarded`.
- `src-tauri/src/commands/fs.rs:142-160` — PDFIUM OnceLock + PDFIUM_LOCK Mutex 정의 (04 doc 의 evidence 와 cross-link).
- `src-tauri/src/commands/fs.rs:166-170` — `lock_pdfium` 의 poison auto-recover.
- `src-tauri/src/commands/fs.rs:282-309` — `pdfium()` 의 candidate-path → bind chain.
- `src-tauri/src/commands/fs.rs:327-375` — `extract_pdf_text` 의 `<project>/raw/sources/<name>.pdf` heuristic + image dest 결정.
- `src-tauri/Cargo.toml:28` — `pdfium-render = "0.9"`.
- `src-tauri/Cargo.toml:32` — `zip = "2"`.
- `src-tauri/Cargo.toml:46-58` — multimodal deps 코멘트 + `image = { 0.25, default-features = false, features = ["png"] }`, `base64 = "0.22"`, `sha2 = "0.10"`.
