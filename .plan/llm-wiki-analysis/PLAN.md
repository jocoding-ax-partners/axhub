# Plan: Exhaustive Source Analysis of nashsu/llm_wiki

**Status:** APPROVED via /ralplan consensus (Planner + Architect + Critic, iteration 2).
**Mode:** SHORT (no `--deliberate` flag).
**Output root:** `/Users/wongil/Desktop/work/jocoding/axhub/.plan/llm-wiki-analysis/`
**Source root:** `/tmp/llm_wiki_inspect/` @ HEAD `1434e08`
**Canonical denominator (locked):** **245 files** (after excluding `.git/`, `node_modules/`, `target/`, `dist/`, `build/`)
**Extension breakdown:** 135 ts, 44 tsx, 14 json, 13 rs, 11 jpg, 7 png, 4 md, 3 js, 2 yml, 2 html, 1 toml, 1 so, 1 lock, 1 ico, 1 icns, 1 gitignore, 1 dylib, 1 dll, 1 css, 1 LICENSE
**Internal risk sites (Rust, locked):** **91** matches of `unsafe | .unwrap() | panic! | Mutex:: | RwLock:: | .expect(` across `src-tauri/src/**`
**gitnexus availability:** confirmed in PATH

---

## 1. Goal & Scope

Produce a deliverable that answers the user's "1부터 10까지 빠짐없이" demand at two levels:

1. **Per-file (mechanical floor):** every one of the 245 source files has a row in `50-source-mapping.md` with `path | ≥80-char purpose | (backlink to a domain doc OR explicit [leaf-utility] tag)`. This is the "exhaustive" floor — no file is silently dropped.
2. **Per-domain (semantic ceiling):** every domain doc instantiates the canonical 5-section template:
   - **Purpose** — one paragraph: what this slice of the system does.
   - **Public Interface** — exported APIs, Tauri commands, IPC contracts, React props.
   - **Internal Risk** — verbatim quotes of `unsafe`, `.unwrap()`, `.expect()`, `panic!`, `Mutex::lock`, `RwLock::write`, FFI loads (pdfium), Result-swallow points (`let _ = ...`, empty `catch`, `as any`, console-only error logs). Non-negotiable. The user's "exhaustive" intent demands we audit failure modes, not just happy paths.
   - **Cross-refs** — links to other domain docs and `50-source-mapping.md` rows.
   - **Evidence** — `path:line` citations for every claim.

**Out of scope:** modifying `llm_wiki` source, running its build, third-party-library deep-dives (note role, don't audit code).

---

## 2. Output Layout (22 files)

```
.plan/llm-wiki-analysis/
├── PLAN.md                     # This file — approved plan + ADR
├── 00-overview.md              # TOC-FIRST: links to every other file. Phase 1 stub, Phase 7 final.
├── _template.md                # Canonical 5-section schema. Phase 1.5 deliverable.
├── 01-tech-stack.md            # Tauri 2 / React 19 / TS / Rust / Chrome ext versions, build pipeline
├── 02-architecture.md          # System diagram, process boundaries, data flow
├── 03-frontend.md              # React 19 app shell, routing, state, theming (template-instantiated)
├── 04-backend-rust.md          # Tauri commands, panic_guard, clip_server, FFI; PRIMARY risk doc
├── 05-extension.md             # Chrome MV3 ext: background/content/popup; secondary risk doc
├── 06-data-layer.md            # Storage, persistence, IPC payload schemas
├── 07-llm-integration.md       # Provider clients, prompt assembly, streaming
├── 08-pdf-ocr-pipeline.md      # pdfium FFI, OCR flow, file ingestion
├── 09-ui-components.md         # Reusable components, design tokens
├── 10..18-domain-*.md          # 9 RESERVED slots — exact names DERIVED from gitnexus clusters in Phase 2
├── 50-source-mapping.md        # 245 rows. Mechanical floor. Phase 6 verifies content.
├── 80-build-and-tooling.md     # Cargo, Vite, Tauri config, scripts
├── 90-risks-gaps.md            # Open questions, unanalyzed areas, gitnexus failures (if any)
└── 99-summary.md               # Executive summary; written last
```

Domain slots 03..09 above are *current best-guess* names. After Phase 2 emits gitnexus clusters, **domain cuts may be re-named or merged**; the mapping (cluster → final filename) is recorded in `00-overview.md §Domain Mapping`.

---

## 3. Methodology

### 3.1 The `_template.md` schema (Phase 1.5 emits this verbatim)

```markdown
# {Domain Name}

## Purpose
{1 paragraph: what this domain does, why it exists, what bounds it.}

## Public Interface
{Exported symbols, Tauri commands, IPC channels, HTTP routes, React props.
Format: `symbol — signature — file:line — short purpose`.}

## Internal Risk
{MANDATORY. Verbatim code quotes for each of:
 - `unsafe` blocks (Rust)
 - `.unwrap()` / `.expect()` chains (Rust)
 - `panic!` / `unreachable!` / `todo!` (Rust)
 - `Mutex::lock` / `RwLock::write` acquisition + drop discipline (Rust)
 - FFI loads, `extern "C"`, dlopen-style (Rust → pdfium et al.)
 - Result swallow: `let _ = expr;`, empty `catch {}`, `console.error` w/o rethrow, `as any`, `as unknown as` (TS)
Each quote: ```rust path:line ... ```. If a category has zero hits, write "None observed in this domain."}

## Cross-refs
{Links to other domain docs that share state, types, or call paths.
Links to specific rows of 50-source-mapping.md.}

## Evidence
{Bulleted list of file:line citations supporting every claim above.
Every claim in Purpose/Interface/Risk must trace to an Evidence entry.}
```

### 3.2 Read budget per file

- **Tauri commands & FFI (Rust):** read fully, no skim.
- **React entry points & state stores:** read fully.
- **Reusable components:** read fully if <200 LOC, otherwise read signature + render tree + side-effects.
- **Generated/vendored:** mark with `[generated]` or `[vendored]` tag, no deep read.
- **Risk Surface scan (every Rust + TS file touched):** run

```bash
grep -nE '\bunsafe\b|\.unwrap\(\)|\.expect\(|panic!|Mutex::|RwLock::' src-tauri/src/**
grep -nE 'as any|as unknown as|catch\s*\(\s*\)|let\s+_\s*=' src/**
```

Every relevant hit quoted into Internal Risk.

### 3.3 gitnexus contract & fallback

- **Primary path:** `gitnexus analyze /tmp/llm_wiki_inspect/` (writes to `/tmp/llm_wiki_inspect/.gitnexus/`, isolated from axhub's index). Use `gitnexus_query`, `gitnexus_context`, `gitnexus_impact` to derive clusters and shape domain cuts.
- **Fallback (Phase 2 explicit):** if `gitnexus analyze` fails, record verbatim in `90-risks-gaps.md §Tooling Gaps`, then derive domain cuts from `find` + directory structure (`src/components/*`, `src/pages/*`, `src-tauri/src/*`, `extension/*`). Plan continues; no hard dependency on gitnexus.

### 3.4 Tone & language

Plan + executor instructions: English. Deliverable file bodies (00..99): Korean prose acceptable where natural; code quotes and symbol names stay original. No emoji.

---

## 4. Execution Phases

### Phase 0 — Prerequisites

1. Verify `/tmp/llm_wiki_inspect/.git` exists and HEAD == `1434e08`. **If missing**, `git clone https://github.com/nashsu/llm_wiki.git /tmp/llm_wiki_inspect && cd /tmp/llm_wiki_inspect && git checkout 1434e08`.
2. Recompute denominator: `find /tmp/llm_wiki_inspect -type f -not -path '*/.git/*' -not -path '*/node_modules/*' -not -path '*/target/*' -not -path '*/dist/*' -not -path '*/build/*' | wc -l`. Locked at **245**. Update + note delta in `90-risks-gaps.md` if different.
3. Run `gitnexus analyze /tmp/llm_wiki_inspect/`. On success, capture cluster output. On failure, write error to `90-risks-gaps.md §Tooling Gaps`.
4. Verify Rust risk-site count: `grep -rnE '\bunsafe\b|\.unwrap\(\)|panic!|Mutex::|RwLock::|\.expect\(' /tmp/llm_wiki_inspect/src-tauri/src | wc -l`. Locked at **91**.

**Acceptance**
- [ ] Clone exists at `1434e08`.
- [ ] Denominator recorded in `50-source-mapping.md` header.
- [ ] gitnexus state recorded.
- [ ] Risk-site count recorded.

### Phase 1 — Skeleton + TOC stub

1. Create `.plan/llm-wiki-analysis/`.
2. Emit empty stubs for all 22 files with title + one-line description.
3. **`00-overview.md` is TOC-FIRST**: lists every other file with one-line "what's inside" link. Domain Mapping section says "TBD — Phase 2."

**Acceptance**
- [ ] All 22 files exist and are non-empty.
- [ ] `00-overview.md` links to every other file.

### Phase 1.5 — Emit `_template.md`

1. Write `_template.md` with verbatim 5-section schema from §3.1.
2. Header comment: "All domain docs MUST instantiate every section. Empty sections require explicit 'None observed' justification."

**Acceptance**
- [ ] `_template.md` exists with all 5 sections.

### Phase 2 — Cluster-derived domain cut

1. gitnexus success: pull cluster output. Identify 7–10 functional clusters.
2. Map clusters → domain doc filenames. Write into `00-overview.md §Domain Mapping`. Rename domain doc files (03..09) if cluster cut differs from best-guess.
3. gitnexus failure: derive cuts from directory structure + entry-point analysis. Same output: mapping in `00-overview.md`.

**Acceptance**
- [ ] `00-overview.md §Domain Mapping` lists 7–10 clusters with assigned domain doc filenames.
- [ ] No domain doc filename unmapped.

### Phase 3 — Tech stack & architecture (01, 02, 80)

1. `01-tech-stack.md`: pin every version from `package.json`, `Cargo.toml`, `tauri.conf.json`, `manifest.json`, `.tool-versions`.
2. `02-architecture.md`: process boundaries (renderer ↔ Tauri core ↔ Chrome ext), data flow diagram (text-based), IPC topology.
3. `80-build-and-tooling.md`: Vite config, Cargo profile, Tauri bundle config, scripts, CI.

**Acceptance**
- [ ] Versions pinned with file:line citations.
- [ ] Process boundary diagram present.
- [ ] Build pipeline reproducible from this doc alone.

### Phase 4 — Domain docs (template-instantiated, primary risk pass)

1. **`04-backend-rust.md` first** — primary risk surface. Quote every one of 91 Rust risk sites into `Internal Risk`. Cover `panic_guard`, `clip_server`, FFI to pdfium, Mutex/RwLock acquisition + drop discipline.
2. **`05-extension.md` second** — Chrome MV3 trust boundary, message passing, content-script injection points.
3. Then `03-frontend.md`, `06-data-layer.md`, `07-llm-integration.md`, `08-pdf-ocr-pipeline.md`, `09-ui-components.md` in parallel where independent.
4. Each doc instantiates ALL 5 template sections.

**Acceptance** (per domain doc)
- [ ] All 5 sections present (`grep -c '^## '` ≥ 5).
- [ ] `Internal Risk` non-empty OR explicit "None observed" with justification.
- [ ] `Evidence` cites ≥1 `path:line` per claim.
- [ ] `Cross-refs` links to ≥1 other domain doc + ≥3 rows of `50-source-mapping.md`.

### Phase 5 — Source mapping floor (`50-source-mapping.md`)

1. Header locks denominator (245) + file inventory by extension.
2. One row per file: `| path | purpose (≥80 chars) | backlink |`.
3. `backlink` = `#anchor` link to domain doc section OR literal tag `[leaf-utility]`. Generated/vendored files get `[generated]` or `[vendored]`.

**Acceptance**
- [ ] Row count == 245 (or updated denominator).
- [ ] No row has empty purpose.
- [ ] No row missing backlink AND lacking `[leaf-utility]`/`[generated]`/`[vendored]` tag.

### Phase 6 — Mechanical content verification

8 checks, all MUST pass (run from `.plan/llm-wiki-analysis/`):

1. **Row count:** `awk -F'|' 'NR>2 && NF>=4' 50-source-mapping.md | wc -l` equals locked denominator.
2. **Purpose length:** `awk -F'|' 'NR>2 && length($3)<80 {print NR": "$0}' 50-source-mapping.md` returns zero lines.
3. **Backlink-or-tag:** `awk -F'|' 'NR>2 && $4 !~ /#|leaf-utility|generated|vendored/ {print NR": "$0}' 50-source-mapping.md` returns zero lines.
4. **Path coverage:** for every path in mapping, `test -f "/tmp/llm_wiki_inspect/$path"` succeeds.
5. **Reverse coverage:** `comm -23 <(find /tmp/llm_wiki_inspect ... | sort) <(awk -F'|' 'NR>2 {gsub(/ /,"",$2); print "/tmp/llm_wiki_inspect/"$2}' 50-source-mapping.md | sort)` returns zero lines.
6. **Template instantiation:** for each domain doc, `grep -c '^## \(Purpose\|Public Interface\|Internal Risk\|Cross-refs\|Evidence\)$' $doc` equals 5.
7. **Risk surface non-trivial in 04-backend-rust.md:** `grep -c '\`\`\`rust' 04-backend-rust.md` ≥ 10.
8. **Cross-ref density:** every domain doc has ≥3 links to mapping anchors AND ≥1 link to another domain.

**Acceptance**
- [ ] All 8 checks pass. Failures block Phase 7.

### Phase 7 — Finalize

1. `00-overview.md`: stub → final TOC + reading order recommendation. Lock Domain Mapping.
2. `90-risks-gaps.md`: every gap (gitnexus failures, partial reads, suspicious "None observed", ambiguous Tauri boundaries).
3. `99-summary.md`: 5 sections — System Purpose / Architecture in One Page / Top 5 Risks (cite path:line) / What's Solid / What I'd Verify Before Trusting.

**Acceptance**
- [ ] `00-overview.md` no longer marked stub.
- [ ] `90-risks-gaps.md` has ≥1 entry per major risk category.
- [ ] `99-summary.md` cites specific `path:line` evidence in Top 5 Risks.

---

## 5. RALPLAN-DR Summary

### Principles (5)
1. **Internal Risk Surface analyzed.** Public-exports-only is rejected; we audit unsafe/unwrap/panic/locks/FFI/swallow points.
2. **Every domain doc fills the 5-section template.** Empty sections require "None observed" justification.
3. **Phase 6 verifies content, not paths.** 8 mechanical checks.
4. **Domain cuts derive from clusters, not intuition.**
5. **Exhaustive floor + semantic ceiling.** Floor = 245 rows. Ceiling = 5-section template.

### Decision Drivers (top 3)
1. User intent: "1부터 10까지 빠짐없이" — exhaustive, including failure modes.
2. Mechanical auditability — Critic flagged path-string match as too weak.
3. Drift across parallel agents — template required, not optional.

### Options
- **Option A (chosen):** Hybrid graph-assisted + template-driven + content-verified. gitnexus when available, find-fallback when not. Domain cuts from clusters. 5-section template enforced. Phase 6 mechanical content checks.
- **Option B (rejected):** Pure find-based, public-exports-only. Faster but doesn't satisfy "빠짐없이" — internal risk goes unaudited (misses 91 Rust risk sites).
- **Option C (rejected):** Single monolithic doc. Eliminates navigation tax but kills parallelism + makes Phase 6 unverifiable.

**Why A:** B fails user's stated intent; C fails executability. A is the only option satisfying both floor (245 files) and ceiling (template per domain) under mechanical verification.

---

## 6. ADR

- **Decision:** Template-driven domain docs with cluster-derived cuts and content-verified Phase 6.
- **Drivers:** User's "빠짐없이" intent; Architect's risk-surface objection; Critic's mechanical-gate requirement.
- **Alternatives considered:** Public-exports-only (rejected — misses 91 internal risk sites); a priori 9 fixed domains (rejected — drifts from actual code structure); path-string-only verification (rejected — accepts lazy stubs).
- **Why chosen:** Only option meeting both floor (245 files mapped) and ceiling (5-section template per domain) under enforceable mechanical gates.
- **Consequences:** Heavier Phase 6 verification machinery; higher per-domain-doc cost (Internal Risk requires reading files, not just symbol tables); less risk of lazy stubs slipping through; total executor effort ~1.4× a public-exports plan.
- **Follow-ups:** If `gitnexus analyze` fails on this repo, capture failure mode in `90-risks-gaps.md` for future tooling work; if denominator drifts from 245, lock new number in Phase 0 before Phase 6; revisit Check #7 threshold (≥10 rust quotes in 04-backend-rust.md) if 04 turns out skewed; specify "None observed" justification format hint at execution time.

---

## 7. Recommended Execution Path

**`/team`** — fans 8 mechanical Phase 6 gates cleanly to verifier. explore + executor + verifier triad fits 22-file scope. Lighter than ralph for this read-heavy analysis workload.

Alternate: `/ralph` if persistent loop preferred — but Phase 6 gates already give natural stopping criteria.

User invokes one of:
- `/oh-my-claudecode:team` with this PLAN.md as input
- `/oh-my-claudecode:ralph` with this PLAN.md as input
