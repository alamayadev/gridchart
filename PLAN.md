# Phase PLAN: GridCraft Chart Style Polish

## Objective
Extend `ChartStyle` with `corner_radii` field to enable directional rounded corners on chart elements, updating all render backends and adding style preset support through engine commands.

## Progress

### Completed (Step 1)
- Added `corner_radii: [f32; 4]` field to `ChartStyle` struct in `crates/chart/src/resolve/style.rs:60`
- Updated `style()` constructor to accept and pass `corner_radii` parameter
- Updated all 6 style presets (DEFAULT, VIBRANT, MINIMAL, DARK, PASTEL, TINTED) to initialize `corner_radii` to `[0.0; 4]`
- Verified compilation (`cargo check -p gridcraft-chart`) and tests (`cargo test -p gridcraft-chart`: 158 passed)

### Remaining Work

#### Step 2: Update Render Backends
1. **PNG Rasterizer** (`crates/chart/src/raster.rs`)
   - Verify `rounded_rect()` already supports per-corner radii (already implemented at line 233)
   - Test with corner radii variations

2. **egui Canvas** (`crates/ui-egui/src/chartview.rs`)
   - Verify mesh rendering with per-corner radii for Rect primitives

3. **SVG Export** (`crates/engine/src/chart_svg.rs`)
   - Review SVG path generation for rounded rectangles with non-uniform radii
   - SVG `<rect>` only supports uniform radii; `<path>` needed for non-uniform

4. **PDF Print** (`crates/engine/src/cmd/print/draw.rs`)
   - Already supports rounded rect with per-corner radii via `pg.rounded_rect()`
   - Verify gradient fills with rounded corners work correctly

#### Step 3: Add Engine Command Support
- `crates/engine/src/cmd/insert.rs:884` - `chart_set()` already handles `style` parameter (0..48)
- Update tests to verify style presets work with new field

#### Step 4: Verification
- Run `cargo xtask ci` to verify all gates pass
- Run snapshot tests for each backend
- Check WASM compatibility with `cargo xtask wasm`

## Verification
- [x] Compilation passes for all workspace crates
- [x] `cargo test -p gridcraft-chart` passes (158 tests)
- [x] `cargo test -p gridcraft-engine` passes (973 tests)
- [x] WASM compatibility check passes (`cargo xtask wasm` - ok)
- [ ] `cargo xtask ci` passes (fmt, clippy, tests, wasm, layers, parity)
- [ ] Snapshot tests pass for egui, SVG, PNG, PDF backends