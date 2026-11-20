<!-- craft-house-rules:start core source=dde09ea31e2e457f5b4653a6e76f7cdf55944bdf sha256=540e6cea75d12d0710c3dca89658328e81e508ecef102446ca5bb95b7c0b92bb -->
## House rules

These apply in every Craft repository. A later rule may be stricter, not weaker. The source is `standards/house-rules-core.md` in `storytold/craft-contrib`. Propose changes there, not in a copy.

1. **A person is accountable.** Whoever runs an agent answers for what it opens and for its review comments.
2. **Check, then claim.** Before writing code, read the issue comments, search pull requests for `#N` and the keywords, and check recent commits on `main`. Comment on the issue first. Release a claim you will not finish.
3. **Verify, don't guess.** Open a definition before using it. Confirm the cause in the code, even when someone names a location.
4. **Keep fixes narrow and tested.** Fix what the issue describes, where the bug is. Add a test that fails without the fix.
5. **Review the diff before opening a pull request.** Identifiers exist as used, the test fails without the fix, nothing unrelated changed, and the description is true.
6. **Look for a better pull request.** If another fixes the same thing, close yours when it is better or equal, and say why.
7. **Report only what ran.** Say what you tested and what you could not run. Without a local build or test, say so and rely on CI before marking ready.
8. **`main` changes through a pull request.** Do not push to it. Branch protection enforces this. There is no exception.
9. **Stop and ask only about** licensing outside the allowlist, publishing, releases, a new remote, secrets, spending money, destructive git, and anything else hard to undo. Otherwise keep working. `plan/` is a gitignored notebook, not rules. Without it, do not ask for it and do not stop.
10. **Keep secrets and personal details out** of the repository, logs and requests.
11. **CI stays shared.** Workflows call `storytold/craft-common-actions`. Do not change CI in a feature or fix pull request. If CI cannot cover a change, open an issue there.
12. **Guards say why.** A check that looks removable says what it prevents. Read that before changing it.
13. **Instructions must be readable by everyone.** No notes about one machine, and no links to files a reader cannot open. `craftrules` is retired. Do not link to it.
14. **Keep pull requests moving.** Concurrent pull requests are fine; unanswered actionable feedback is not. Respond in 7 days, a reminder at 14, closure eligible at 30, from when an actionable request waits on the author. Suggestions and unconfirmed bot findings do not start the clock. A fix, explanation or agreed plan resets it; unrelated activity does not. Waiting on a reviewer or maintainer pauses it and does not count against the author. A maintainer decides closure; it is never automatic. Reuse search results, do not poll, and stop at a rate limit.
15. **No agent attribution.** A pull request does not say an agent wrote it. A repository rule may not require that.
<!-- craft-house-rules:end -->
<!-- craft-house-rules:start rust source=dde09ea31e2e457f5b4653a6e76f7cdf55944bdf sha256=387e0ccb4b49e567f60951d7d6fff7305054c89f43ad812b9993dc7c38528013 -->
## Rust Craft repositories

These apply to native Craft apps and to shared Rust libraries. Select them by repository type. A repository that merely contains some Rust does not take this block. Rules later in a repository's file may be stricter, but not weaker. The source is `standards/house-rules-rust.md` in [storytold/craft-contrib](https://github.com/storytold/craft-contrib).

1. **Never crash.** Non-test code does not panic and has no `unsafe`, except where the repository file names an exception. Return errors, treat input as hostile, and bound recursion and allocations. New production crate roots copy the `deny` attribute from an existing crate: `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::unimplemented`, `clippy::todo`, `clippy::unreachable`. Every crash fix comes with a test that panicked before the fix.
2. **Clean room.** Learn from observed behaviour and public specifications. Do not copy another product's code, assets, strings or artwork. Check a dependency's licence before adding it.
3. **Assets and fonts.** Every asset is original or suitably licensed, and is recorded in `ATTRIBUTION.md` in the same change. Font files live in [storytold/craft-fonts](https://github.com/storytold/craft-fonts), not in the app repository.
4. **Rust only for native Craft apps.** No handwritten JavaScript or TypeScript, and no Tauri, Electron or webview shell, in those apps. Shared tooling follows its documented technical scope and maintainer-approved exceptions. Craft PR Review's existing Tauri and TypeScript UI is one such exception. It does not relax this rule for native apps. Preserve any existing web build.
5. **Time goes through `chrono`.** Do not hand-roll calendar conversion. For dates beyond what `chrono` can represent, follow [standards/time.md](https://github.com/storytold/craft-contrib/blob/main/standards/time.md).
6. **Contributor credits.** Add or edit only your own person's entry in `contributors/people.toml`, and only after asking them. Never guess a name or hand-edit `contributors.json`.

## Licence allowlist

Dependencies and assets use SPDX identifiers. An expression is allowed when every identifier is in this list. A repository may name a shorter list. It may not add an identifier here by editing its own copy of these rules.

- `ISC`
- `MIT`
- `BSD-2-Clause`
- `BSD-3-Clause`
- `Apache-2.0`
- `OFL-1.1`
- `Ubuntu-font-1.0`
- `Bitstream-Vera`
- `CC0-1.0`
- `CC-BY-4.0`
- `CC-BY-SA-4.0`
- `LicenseRef-Public-Domain`

`LicenseRef-Storyteller-Trademark` is not in this list. It covers Storyteller brand marks only, where a repository already says so.
<!-- craft-house-rules:end -->

# GridCraft — instructions for agents

GridCraft is a clean-room, open-source, Rust-native spreadsheet targeting Microsoft Excel parity — and superiority (speed, openness, agent control). It runs natively on macOS, Windows, Linux and BSD, and on the web via WASM. Siblings with the same conventions: `../designcraft` (InDesign-class, the closest reference), `../photocraft` (Photoshop), `../vectorcraft` (Illustrator), `../pdfcraft` (Acrobat), `../filmcraft` (Premiere), `../lightcraft` (Lightroom), `../effectcraft` (After Effects).

## Start every session here
Read [`ROADMAP.md`](ROADMAP.md) and [`docs/gaps.md`](docs/gaps.md). `plan/` is an optional gitignored notebook (status, execution notes, `plan/excel/*.md`). Do not ask for it and do not stop when it is absent. Screenshots of Microsoft software stay in that notebook and are never committed.

## Assets and iconography — absolute rule
**No Microsoft, Adobe, Avid, Autodesk (or any other vendor's) iconography, images, artwork, fonts, templates, themes or sample files — ever.** This is the most important rule in this repository; breaking it is a serious failure.
- Every asset in the repo is original work by GridCraft contributors, or public domain / CC0, or open source / Creative Commons that allows redistribution (or licensed open source by the contributor who created it).
- Every asset file has a row in [`ATTRIBUTION.md`](ATTRIBUTION.md) (author, source, licence). `cargo xtask assets` enforces it. Adding an asset without a row is a bug.
- Prefer art generated in code: the UI icon set (`crates/ui-egui/src/icons.rs`), table and cell style palettes, themes and sample workbooks are drawn/defined in code and original.
- Never copy Office's icons, ribbon artwork, theme XML, table-style definitions, templates, clip art or wording beyond feature names. Excel feature *names* (menu labels, function names) are fine.
- Screenshots of Microsoft software are never committed (they live only in `plan/excel/screenshots/`).
- The ArtCraft brand files in `docs/brand/` are ArtCraft trademarks under `docs/brand/LICENSE-brand.txt`, the one exception.

## Never crash
People trust GridCraft with their numbers; a crash loses their work. **This outranks feature work.**
- **No panics in non-test code:** no `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!`, `unimplemented!`; no `unsafe` (`unsafe_code = "forbid"`).
- **Errors are `Result<T, E>`** through the crate's error type and `?`.
- **Input-derived numbers are hostile** (files, formulas, commands, MCP/control params): validate indexes and ranges before access, use checked access when validity is not established, use checked/saturating arithmetic, and cap input-sized allocations (huge ranges, `REPT`, `SEQUENCE`, whole-column references).
- **Bound recursion** (formula nesting, dependency chains, names referring to names).
- **Last-resort guard:** `Session::execute` catches escaped panics and restores the workbook. It's a safety net, not a licence.
- **Prove it:** every crash fix lands with a regression test (`tests::every_command_survives_empty_params` fuzzes every command; `crates/engine/tests/it/command_sequence_fuzz.rs` fuzzes random command sequences against undo/redo, XLSX round-trip and full-recalc invariants; soak it with `GRIDCRAFT_FUZZ_SEEDS`/`GRIDCRAFT_FUZZ_STEPS`, replay with `GRIDCRAFT_FUZZ_SEED`).
- Crates carry `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.

## Non-negotiables
- **Clean-room.** Microsoft Excel is installed on the dev machine and may be *observed* black-box: run it, use its UI with synthetic workbooks, take screenshots (by window id) stored only under `plan/excel/screenshots/`. Never read, disassemble or copy anything inside the Office bundle, never commit files produced by Excel, never copy GPL/AGPL/LGPL code (LibreOffice, Gnumeric…). Function behaviour comes from public documentation and observation.
- **Fonts live in [`storytold/craft-fonts`](https://github.com/storytold/craft-fonts)**, never in this repo. It is an optional build input (`CRAFT_FONTS_DIR`); without it the app uses system fonts and egui's defaults.
- **Test corpora** (real Excel-authored files) live in a separate corpus repo, fetched pinned and sha256-verified. Never commit large binary fixtures; build test workbooks in code.
- **Everything is a command.** User-visible behaviour = a command in `crates/engine/src/cmd/*` (id, label, ribbon path, shortcut, params doc, `enabled`, `run`) + tests. Ids follow Excel's ribbon (`home.bold`, `insert.chart`, `data.sortAscending`) plus primitives (`cell.set`, `selection.set`, `range.setValues`). UI-only commands live in `crates/ui-egui`. The control channel, CLI and MCP reach all of them.
- **Programmatic calls never open dialogs**: a command with empty params runs with defaults or returns an error; only menu-style invocation opens a dialog (`UiRequest::Dialog`).
- **Layering** is enforced by `cargo xtask layers`: `core`/`numfmt` (L0) → `formula`/`functions` (L1) → `model`/`calc`/`xlsx` (L2) → `chart` (L3) → `engine` (L5) → `ui-egui`/`mcp` (L6) → `apps/*` (L7). Nothing below L6 depends on egui/eframe/winit/rfd. **The UI crate is swappable.**
- **The UI is thin**: panels read engine state and act through `app.run(id, params)`. Colours come from `theme::Tokens`.
- **Rust only** (no handwritten JS/TS). **Never break wasm** (`cargo xtask wasm`).
- **Quality gates** before every commit: `cargo xtask ci` (fmt, clippy -D warnings, tests, assets, layers, wasm). One task id per commit (`M2.1: borders gallery`).

## Running and looking at the app
- `cargo run --release -p gridcraft -- --sample sales --control 7979` (sample workbook + control channel).
- Drive it: `gridcraft-cli send 7979 <method> '<json>'` (reads the token file the app printed on stderr), or JSON lines on `127.0.0.1:7979` whose first line is `{"id":0,"method":"auth","params":{"token":"…"}}` (token: `$XDG_RUNTIME_DIR/gridcraft/control-7979.token`, or your own `GRIDCRAFT_CONTROL_TOKEN`), e.g. `{"id":1,"method":"engine.execute","params":{"command":"cell.set","params":{"cell":"B2","input":"=SUM(A1:A9)"}}}` then `{"id":2,"method":"ui.screenshot","params":{"path":"/tmp/shot.png"}}`. Methods: `crates/ui-egui/src/control.rs`, docs: `docs/control-protocol.md`.
- **Offscreen UI render** (no window, from a source checkout): `cargo run -p gridcraft-ui-egui --example snapshot -- --sample sales out.png`.
- Headless: `gridcraft-cli eval '=SUM(1,2,3)'`, `gridcraft-cli run --in book.xlsx --cmd 'home.bold={"range":"A1:C1"}' --out book.xlsx`, `gridcraft-cli mcp`.
- **For UI work, look at the result** (snapshot PNG) and, when available, compare with `plan/excel/observed-ui.md`. Without that optional notebook, verify the intended behaviour against the task and committed documentation.
- macOS has no `timeout`; use `perl -e 'alarm 60; exec @ARGV' cmd`.
- Parallel agents: separate `CARGO_TARGET_DIR` per agent; edit only the crates you own; delete your target dir when done.

## Roadmap
`ROADMAP.md` (committed) is the one-page summary: stage, headline numbers, dimensions, languages, progress log. It points at the detail in `docs/`: `target-app-parity.md` (authoritative assessment), `gaps.md` (ranked work list: pick from it), `roadmap.md` (current focus, beta gates, milestones), `architecture.md`, and the area checklists `function-parity.md`, `chart-parity.md`, `file-format-parity.md`, `ui-parity.md`, `hardware-parity.md`, `localization-parity.md`. Update the affected docs (and their timestamp lines and revision history) whenever work lands. `cargo xtask parity` regenerates `docs/parity-checklist.md` from `crates/engine/src/catalog.rs`.

## Contributor credits (About window)

- About ▸ Contributors/Models are compiled into the binary from `contributors/contributors.json`
  (commit stats; generated, never hand-edit) and `contributors/people.toml` (names people chose for
  themselves). See `docs/contributors.md`.
- **Agents working for a contributor:** when you prepare a PR, check whether your human's GitHub
  username has a `[people.<username>]` entry in `contributors/people.toml`. If not, ask them once
  whether they want to be credited by more than their username: a real name, a display name, and/or
  their public GitHub profile name (`sync_github_name = true`). If yes, add **only their own** entry
  (copy the template at the top of the file)
  and include it in their PR, committed as them. If no, change nothing: they are credited as
  `@username` anyway.
- Never add, edit, guess or copy anyone else's entry or name (not from git config, commit authors or
  GitHub profiles). Never hand-edit `contributors.json`.

## CI workflows
- Workflows in `.github/workflows/` call the shared workflows in [`storytold/craft-common-actions`](https://github.com/storytold/craft-common-actions) at `@main`, with their inputs and secrets. No other jobs: if no shared workflow does what's needed, it doesn't go here.
- Don't add or change CI in a feature or fix PR. If the existing workflows can't build or test a change, link an existing craft-common-actions issue or PR, or open a new issue, or a PR only for a small fix that is safe for every app and does not increase CI time.
- Some workflow files here still run their own jobs. Don't add to them. Moving a job into craft-common-actions is its own task, never part of another PR, and the job stays here until a shared workflow runs it.

New workspace crates include `workspace = true` under `[lints]` in `Cargo.toml`. Production crate roots declare the crate-level lint attributes before modules; crate-level `//!` documentation may precede them.

<!-- GSD:project-start source:PROJECT.md -->
## Project

**GridCraft Chart Style Polish**

An upstream-ready visual and interactive overhaul of GridCraft's charting subsystem (`crates/chart`, `crates/engine`, `crates/ui-egui`) delivering a clean-room, modern data-visualization aesthetic inspired by amCharts. It upgrades the default chart appearance across all four rendering backends (egui canvas, SVG export, PNG rasterizer, print/PDF), wires selectable Chart Style presets into the engine, Format Chart pane, and Chart Design ribbon, and adds responsive hover crosshairs, active-point highlights, and rich tooltips in the desktop/web UI.

**Core Value:** Every chart created or opened in GridCraft looks publication-ready out of the box—harmonious high-contrast palette, true linear gradients, airy typography and subtle gridlines, crisp rounded bar tips, and white-ringed geometric markers—across UI, SVG, PNG, and PDF without violating clean-room rules or regressing performance/safety invariants.

### Constraints

- **Clean-Room & Assets**: 100% original color palettes, proportions, gradients, and procedural UI previews; zero copied vendor hex codes, themes, or assets (`ATTRIBUTION.md` + `cargo xtask assets`).
- **Never Crash**: `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]` and `#![forbid(unsafe_code)]` across all non-test code; hostile float/NaN/zero-size chart bounds handled safely.
- **Cross-Backend Parity**: Any new or extended `Prim` variant (gradients, directional rounding, marker polygons/strokes) must render cleanly in all 4 backends (egui, SVG, raster PNG, PDF) and compile on `wasm32-unknown-unknown`.
- **Everything Is a Command**: Style preset changes must execute via engine commands (`chart.set`) with undo/redo, headless CLI, and MCP compatibility; programmatic empty-param calls must never open dialogs.
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## Recommended Stack
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Rust (`edition = "2024"`) | `1.95` (MSRV) | Language & `std::f32` geometry, interpolation, and color math | Workspace standard (`Cargo.toml`). `#![forbid(unsafe_code)]` and `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, ...)]` require deterministic, checked float math (`clamp`, `is_finite`, `total_cmp`) with zero third-party `unsafe` SIMD/FFI surfaces. |
| `gridcraft-chart` (L3) | `0.19.1` (workspace) | Style tokens/presets, layout, `Prim` scene graph, geometry hit-testing, and built-in RGBA scanline rasterizer | Sits at L3 (`xtask/src/layers.rs`), depending only on `core` (L0), `numfmt` (L0), `model` (L2), `calc` (L2), and `serde`. Extending `Prim` here automatically feeds all 4 rendering backends while keeping chart layout 100% UI-toolkit-free and WASM-compatible. |
| `egui` / `epaint` (L6) | `0.37` (pinned `storytold/egui` rev `6d518fc7`) | Hardware-accelerated canvas painting (`Mesh`, `CornerRadius`, `PathShape`) and `Order::Tooltip` hover overlays | Already powers `gridcraft-ui-egui` on native (`wgpu`/`glow`) and web (`WebGPU`/`WebGL2`). `egui::Mesh::colored_vertex` natively interpolates vertex RGBA on the GPU for linear gradients (`chartview.rs` already builds `egui::Mesh` for concave areas and wedges), `egui::CornerRadius` supports per-corner `nw`/`ne`/`sw`/`se` radii, and `egui::Area` at `Order::Tooltip` renders non-interactive floating cards above clip rects. |
| `gridcraft-engine::chart_svg` (L5) | `0.19.1` (workspace) | Inline SVG 1.1 serialization (`<defs><linearGradient>`, `<path>`, `<rect>`) for Save as Web Page | Hand-written `std::fmt::Write` emitter (`crates/engine/src/chart_svg.rs`) enforcing `MAX_PAGE_SVG_BYTES` (`64 MiB`) and non-finite coordinate sanitization (`num()`). Direct `<linearGradient gradientUnits="userSpaceOnUse">` and arc `<path>` emission adds ~60 lines with zero XML/DOM crate overhead. |
| `gridcraft-pdf` (L0) + `engine::cmd::print` (L5) | `0.19.1` (workspace) | Vector PDF 1.7 chart output (cubic Bézier rounded corners, axial `/ShadingType 2` gradients, polygons) | `gridcraft-pdf` (`crates/pdf/src/lib.rs`) builds the PDF envelope with the shared `craft-pdf` writer and keeps the chart drawing primitives (cubic Bézier corner arcs, reusing `K = 0.552_284_8` from `Page::circle`, and PDF 1.7 axial shading dictionaries `/ShadingType 2`, `/FunctionType 2`) in the app, since the shared library owns serialization, not chart policy. That preserves crisp vector print output without rasterizing charts into multi-megabyte bitmaps. |
| `gridcraft-model` + `gridcraft-xlsx` (L2) | `0.19.1` (workspace) | Persistence of `Chart.style: u32` and OOXML `<c:style val="1..48"/>` round-trip | `Chart.style: u32` (`crates/model/src/features.rs:601`) and `<c:style val="..."/>` (`crates/xlsx/src/chart.rs:761`) already exist and round-trip cleanly. Preset IDs map directly onto this field with zero schema breakage. |
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde` / `serde_json` | `1` (`rc`, `float_roundtrip`) | Serialization of `Prim`, `Chart`, and `chart.set` command parameters | Required on any new types embedded in `Prim` (`Fill`, `Gradient`, `CornerRadii`) because `Prim` derives `Serialize, Deserialize` (`crates/chart/src/lib.rs:52`) and `chart.set` parses JSON command payloads in L5/L6/MCP. |
| `image` | `0.25` (`png`, `jpeg`, `default-features = false`) | Encoding RGBA8 buffers from `gridcraft_chart::rasterize` in engine/UI export paths | Use only in L5 (`gridcraft-engine`) and L6 (`gridcraft-ui-egui`) where already declared. Never add `image` to `gridcraft-chart` (L3), which outputs raw `Vec<u8>` RGBA8. |
| `egui_kittest` + `epaint_default_fonts` | `0.37` | Deterministic offscreen `wgpu` UI snapshot & interaction testing | Use in `crates/ui-egui/tests/it/` and `examples/snapshot.rs` to verify hover tooltips, crosshairs, ribbon style gallery previews, and gradient rendering visually. |
### Development Tools
| Tool | Purpose | Notes |
|------|---------|-------|
| `cargo xtask layers` | Enforces L0→L7 DAG and bans UI crates (`egui*`, `eframe`, `winit`, `rfd`, `wgpu*`) below L6 | Run whenever touching imports across `chart`, `pdf`, `engine`, or `ui-egui`. |
| `cargo xtask wasm` | Runs `cargo check --target wasm32-unknown-unknown --keep-going` across L0–L5 + `gridcraft-web` | Verifies no OS-only APIs (`std::time::Instant`, platform threads) leak into chart rendering or hover state. |
| `cargo xtask assets` | Verifies `ATTRIBUTION.md` coverage for any static files | Keep all palettes, style thumbnails, and icons 100% procedural in Rust code so zero binary/SVG assets are added. |
| `cargo xtask parity` | Regenerates `docs/parity-checklist.md` from `crates/engine/src/catalog.rs` | Run after wiring `chart.set` style preset support (`catalog.rs:301` already tracks `("chart.set", "Chart Design", "Chart Styles", "Chart Styles")`). |
| `cargo xtask ci` | Full gate (`i18n-report`, `fmt`, `clippy -D warnings`, `test`, `doctest`, `assets`, `layers`, `wasm`, `parity --check`, `keytips --check`) | Mandatory pre-merge verification gate (`AGENTS.md`). |
## Installation
# No new Cargo dependencies are permitted or needed.
# Verify existing workspace toolchain and WASM target:
# Verify architecture layering and WASM compilation:
## Architecture & Backend Stack Decisions
### 1. Primitive Representation in `gridcraft-chart` (L3) — Confidence: HIGH
- **`Fill` (`Copy` enum):** Replace bare `Rgba` on fillable primitives (`Rect`, `Polygon`, `Wedge`) with a `Copy` type:
- **Directional `radii: [f32; 4]` on `Prim::Rect`:** Store `[nw, ne, se, sw]` in chart points instead of scalar `radius: f32`. Positive vertical columns set `[r, r, 0.0, 0.0]`, negative vertical columns set `[0.0, 0.0, r, r]`, positive horizontal bars set `[0.0, r, r, 0.0]`, negative horizontal bars set `[r, 0.0, 0.0, r]`, and stacked middle segments set `[0.0; 4]` (rounding only the outward terminal segment).
- **Ringed Multi-Shape Markers:** Emit existing `Prim::Circle` (circle) and convex `Prim::Polygon` (square, diamond, upward/downward triangle) with `fill: Fill::Solid(marker_fill)` and `stroke: Some((card_bg_rgba, 1.75))`. No new `Prim` enum variant is needed because all 4 backends already stroke circles and closed polygons.
### 2. Backend 1 — `egui` Canvas (`crates/ui-egui/src/chartview.rs`, L6 + Web WASM) — Confidence: HIGH
- **Linear Gradients:** Build `egui::Mesh` (`TextureId::default()`, white UV `(0.0, 0.0)`) and assign each vertex color by projecting the vertex's chart-space `[x, y]` onto `(x0, y0) -> (x1, y1)` and linearly interpolating straight `Rgba` into `Color32::from_rgba_unmultiplied`. `chartview.rs` already uses `egui::Mesh` in `area_mesh` (line 175) and `Prim::Wedge` (line 92). Subdivide gradient polygons/rects along gradient stop axes so hardware Gouraud interpolation across `wgpu`/`WebGPU`/`WebGL2` matches exact linear interpolation.
- **Directional Rounded Corners:**
- **RTL Sheet Safety:** In `chartview.rs:408–414`, every chart is recorded as a `PaintedObject` (`quadrant.rs:29–35`), which translates all shapes in the chart as a rigid unit (`cs.shape.translate(...)`) rather than reflecting individual vertices. `Shape::Mesh` and `CornerRadius` inside charts remain intact on RTL sheets.
- **60fps Hover Crosshairs, Highlights & Tooltips:**
### 3. Backend 2 — Inline SVG (`crates/engine/src/chart_svg.rs`, L5 + WASM) — Confidence: HIGH
- **Linear Gradients:** Pre-pass or on-demand collect unique `Fill::LinearGradient` values in `prims_svg_within`, emit `<defs><linearGradient id="g0" gradientUnits="userSpaceOnUse" x1="..." y1="..." x2="..." y2="..."><stop offset="0" stop-color="#rrggbb" stop-opacity="..."/><stop offset="1" .../></linearGradient></defs>`, and emit `fill="url(#g0)"` on shapes. `gradientUnits="userSpaceOnUse"` uses exact chart coordinates, avoiding `objectBoundingBox` aspect-ratio skew on non-square area polygons.
- **Directional Rounded Corners:** SVG `<rect>` only supports uniform corner rounding (`rx`/`ry`). Keep `<rect ... rx="..."/>` when all 4 corners are equal (preserving compact output under `MAX_PAGE_SVG_BYTES = 64 MiB`), and emit `<path d="M... H... A... V... Z"/>` when `radii` differ per corner.
### 4. Backend 3 — Built-in RGBA Rasterizer (`crates/chart/src/raster.rs`, L3 + WASM) — Confidence: HIGH
- **Linear Gradients:** Extend `Canvas::fill` (`raster.rs:59`) to accept `Fill` instead of flat `Rgba`. At scanline blend (`raster.rs:152–157`), compute `t = (((px as f32 + 0.5 - x0) * dx + (py as f32 + 0.5 - y0) * dy) / len_sq).clamp(0.0, 1.0)` (falling back to `start` when `!len_sq.is_finite() || len_sq <= 1e-6`), interpolate straight `[r, g, b, a]`, and pass the resulting `Rgba` to existing `self.blend(px, py, color, cov)`.
- **Directional Rounded Corners:** Update `rounded_rect(x, y, w, h, radii: [f32; 4])` (`raster.rs:180`) to clamp each corner radius independently to `(w.min(h) / 2.0).max(0.0)` and emit either a single corner vertex `[cx, cy]` (when `r_i < 0.5`) or `arc(...)` (when `r_i >= 0.5`).
### 5. Backend 4 — Print & PDF Export (`crates/pdf/src/lib.rs` L0 + `crates/engine/src/cmd/print/draw.rs` L5) — Confidence: HIGH
- **Current Gap Identified:** `draw_prims` (`draw.rs:109`) currently ignores `radius` on `Prim::Rect` altogether and flattens `Rgba` alpha against white (`blend([u8; 4]) -> Rgb` at `draw.rs:15`).
- **Directional Rounded Corners in PDF:** Add `Page::rounded_rect(x, y, w, h, radii: [f32; 4], fill, stroke)` in `crates/pdf/src/lib.rs` using cubic Bézier quarter-circles (`K = 0.552_284_8`, already proven in `Page::circle` at `pdf/src/lib.rs:661`).
- **Linear Gradients in PDF:** Add axial shading support to `crates/pdf/src/lib.rs` (L0; the PDF envelope comes from the shared `craft-pdf` writer, the shading dictionary stays app-side):
## Alternatives Considered
| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Extend `gridcraft_chart::raster` (`raster.rs`, ~370 LOC) in-place | `tiny-skia` or `raqote` | Never in GridCraft L3: `tiny-skia` pulls `bytemuck`/SIMD crates and ~25k LOC to solve paths `raster.rs` already scan-converts in 368 safe lines with explicit `MAX_SCAN_WORK` DoS budgets. |
| Native PDF `/ShadingType 2` axial shading in `gridcraft-pdf` (L0) | Rasterize gradient shapes via `gridcraft_chart::rasterize` and embed with `Page::image_rgba` | Only as a fallback if radial/mesh shadings were ever required; for linear gradients, `/ShadingType 2` stays infinitely sharp at 400%+ print zoom and uses ~200 bytes instead of kilobytes of RLE pixel streams (`MAX_EMBEDDED_IMAGE_BYTES`). |
| `Copy` 2-stop `LinearGradient` in `Prim` | Arbitrary `Vec<(f32, Rgba)>` stops on every `Prim` | Only if multi-hue rainbow heatmaps are drawn as a single primitive; for amCharts-style area fades and bar sheens, 2 stops (or fixed `[Stop; 3]`) avoid thousands of heap allocations per chart and keep `ChartCache` byte accounting exact. |
| Immediate-mode overlay painting in `chartview.rs` + `egui::Area` (`Order::Tooltip`) | Storing hovered series/point inside `gridcraft_model::Chart` or `Session` | Never: mutating `Workbook` on hover clones `Arc<Workbook>`, invalidates `ChartCache`, pollutes undo/dirty state, and triggers full recalculation/layout every frame. |
| Per-vertex `egui::Mesh` gradients in `chartview.rs` | Uploading 1D gradient `egui::TextureHandle` strips per series | Only if >8 nonlinear color stops per shape are needed; per-vertex `Mesh` uses `TextureId::default()`, avoiding GPU texture binds, atlas pressure, and lifetime management across multi-window viewports. |
## What NOT to Use
| Avoid | Why | Use Instead |
|-------|-----|-------------|
| External Rust plotting crates (`plotters`, `charming`, `egui_plot`, `kuva`) | `egui_plot` violates L3 UI-toolkit isolation (`UI_CRATES` check in `xtask/src/layers.rs`); `charming` generates ECharts JS (violates Rust-only house rule); `plotters` duplicates L3 layout/text measurement and cannot emit `Prim` across GridCraft's 4 custom backends. | Extend `gridcraft-chart` (`crates/chart`) layout and `Prim` directly. |
| External SVG/XML/PDF writers (`svg`, `resvg`, `usvg`, `printpdf`, `krilla`, `lopdf`) | `gridcraft-pdf` keeps chart drawing policy in the app and delegates PDF envelope serialization to the shared `craft-pdf` writer; `chart_svg.rs` enforces a streaming `MAX_PAGE_SVG_BYTES` cap during string construction that DOM builders violate by allocating full trees upfront. | Direct formatting in `crates/engine/src/chart_svg.rs`; `Page` drawing methods plus the shared writer in `crates/pdf/src/lib.rs`. |
| Color-space crates (`palette`, `csscolorparser`, `oklab`) | Unnecessary dependency churn for fixed design-system palettes and linear RGBA stop interpolation; some pull proc-macros or traits unneeded in L3. | Const clean-room `Rgba` tables in `crates/chart/src/style.rs` (or `resolve.rs`) + ~15-line sRGB/linear lerp helper using `std::f32`. |
| `std::time::Instant` in L3/L5/L6 hover or transitions | Panics at runtime on `wasm32-unknown-unknown` in browsers. | Keep L3 deterministic and stateless; if UI-side hover fade is ever desired in L6, use `egui::Context::animate_bool_with_time` or `web-time = "1.1.0"` (already in `ui-egui/Cargo.toml`). |
| Copied amCharts / Excel palettes, CSS, SVGs, or theme XML | Strictly prohibited by `AGENTS.md` Clean-Room and Assets rules (`cargo xtask assets`). | Author 100% original hex palettes, spacing ratios, stroke widths, and procedural ribbon style gallery previews in Rust code. |
| `unwrap()`, `expect()`, `panic!()`, `unreachable!()`, `as` without finite/bounds checks | Denied at compile time by crate lints; hostile `NaN`/`Infinity`/negative widths from formulas or zoomed viewports can panic `f32::clamp(min, max)` if `min > max`. | Always guard `w.max(0.0)`, `is_finite()`, `f32::total_cmp`, and `checked_get` (matching `raster.rs:182` and `pdf/src/lib.rs:354`). |
## Stack Patterns by Variant
- Emit an explicit background `Prim::Rect` from `gridcraft_chart::render` (with card fill and optional subtle border/corner radius) rather than relying on `chartview.rs:411` (`Tokens::get(p.ctx()).grid_bg`) or PDF `blend()` assuming `[255, 255, 255]`.
- Because SVG, PNG raster, PDF print, and light-mode egui sheets must all render dark-preset charts with their intended dark card surface and light axis labels.
- Check `radii == [0.0; 4]` fast path first across all 4 backends.
- Because gridlines, plot frames, legend swatches, and tiny sparkline/histogram bars (`MIN_BAR_PX = 0.25`) stay sharp single-quad primitives without arc tessellation overhead (`render_perf_tests.rs`).
- Decimate polylines (`MAX_LINE_PTS = 2000`) and cap markers (`MAX_MARKERS = 5000`) in `crates/chart` as today, and restrict hover hit-testing to O(log N) binary search on category/X coordinates or bounded spatial scan.
- Because pointer-move events fire at 60–240Hz in `ui-egui`; hover lookup must stay `< 0.2 ms` even when `ChartCache` is bypassed for volatile series (`chartview.rs:269`).
## Version Compatibility
| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `egui = "0.37"` (`storytold/egui` rev `6d518fc7`) | `eframe = "0.37"`, `epaint = "0.37"`, `egui_kittest = "0.37"` | Patched workspace-wide via `[patch.crates-io]` in root `Cargo.toml:110–124`. Use `egui::CornerRadius` (`nw`, `ne`, `sw`, `se`: `u8`), `StrokeKind::Inside`, and `egui::Tooltip` / `egui::Area` APIs from 0.37 (not legacy `egui::Rounding` from ≤0.30). |
| `gridcraft-chart` (`0.19.1`, L3) | `wasm32-unknown-unknown` + `gridcraft-web` (`0.19.1`, L7) | Verified by `cargo xtask wasm`; must remain free of `egui`, OS I/O, and unguarded `std::time::Instant`. |
| `Chart.style: u32` (`gridcraft-model`) | OOXML `<c:style val="1..48"/>` (`gridcraft-xlsx`) | `crates/xlsx/src/chart.rs:761` serializes `1..=48` into `<c:style>` and `0` as default/unset. Keep selectable style preset IDs within `1..=48` (with `0` resolving to the polished modern default) so XLSX round-trips preserve the user's chosen preset losslessly. |
## Sources
- `F:\GitHub\gridchart\Cargo.toml` — Verified workspace Rust edition (`2024`), MSRV (`1.95`), crate versions (`egui 0.37`, `serde 1`, `image 0.25`), lints (`unsafe_code = "forbid"`), and `storytold/egui` fork pin — **HIGH confidence**
- `F:\GitHub\gridchart\xtask\src\layers.rs` & `xtask\src\main.rs` — Verified L0–L7 dependency DAG (`pdf` L0, `model`/`xlsx` L2, `chart` L3, `engine` L5, `ui-egui` L6) and `wasm32-unknown-unknown` gate — **HIGH confidence**
- `F:\GitHub\gridchart\crates\chart\src\lib.rs` & `raster.rs` — Verified `Prim` schema, `SUB = 4` scanline coverage AA, `MAX_SCAN_WORK`, and `rounded_rect` implementation — **HIGH confidence**
- `F:\GitHub\gridchart\crates\ui-egui\src\chartview.rs`, `chartcache.rs`, `grid\quadrant.rs`, & `grid\show.rs` — Verified `paint_prims`, `area_mesh` (`egui::Mesh`), `CornerRadius` RTL mirroring, `PaintedObject`, `Weak<Workbook>` cache invariants, and `egui::Area` + `Order::Tooltip` popup pattern — **HIGH confidence**
- `F:\GitHub\gridchart\crates\engine\src\chart_svg.rs`, `crates\engine\src\cmd\print.rs`, `cmd\print\draw.rs`, & `crates\pdf\src\lib.rs` — Verified SVG byte budget (`64 MiB`), PDF `MAX_CHART_PRIMS` (`4M`), `MAX_SAVE_DEPTH` (`28`), cubic Bézier `Page::circle`, and current `draw_prims` `radius`/alpha gaps — **HIGH confidence**
- `F:\GitHub\gridchart\crates\model\src\features.rs` & `crates\xlsx\src\chart.rs` — Verified `Chart.style: u32` and `<c:style val="1..48"/>` round-trip range — **HIGH confidence**
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->

<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->

## Shared libraries

Before adding or changing file-format, codec, protocol, or reusable UI code, check `storytold/craft-libs` and its open PRs first; reuse or extend the shared library rather than duplicating it locally, keep app-specific policy and model mapping in the app, and preserve behavior, security/resource limits, attribution, and regression coverage during cutovers.
