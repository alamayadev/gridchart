//! One deterministic scene, in chart coordinates, that every rendering backend can replay.
//!
//! PRIM-03 / D-02: parity is asserted against a scene built in code, not against a golden image a
//! backend could silently re-baseline. The fixture carries the four shapes this phase changed - a
//! card, a two-stop gradient bar, a rectangle with one rounded corner, and one of each ringed marker
//! hull - with explicit numbers and no layout, workbook or text measurement, so all four backends
//! (egui canvas, inline SVG, the built-in rasterizer, print/PDF) read exactly the same input.
//!
//! Probe constants name pixels that lie wholly inside the feature they check, so a backend that
//! paints the feature paints that pixel exactly, with no anti-aliasing ambiguity.

use crate::render::shape::{MarkerGeometry, MarkerShape, geometry};
use crate::{Fill, Prim, Rgba};

/// Scene size in chart points. Small enough for the raster leg, large enough for every shape.
pub const WIDTH: f32 = 240.0;
/// Scene height in chart points.
pub const HEIGHT: f32 = 160.0;

/// The sheet colour a raster leg paints under the scene before rasterizing it: a chart sits on a
/// sheet, never on a hardcoded white.
pub const BACKGROUND: Rgba = [9, 12, 16, 255];
/// The card fill, so a missing fill is visibly not the backdrop.
pub const CARD: Rgba = [32, 40, 48, 255];
/// x, y, w, h of the card.
pub const CARD_RECT: [f32; 4] = [0.0, 0.0, 132.0, 80.0];

/// Near stop of the gradient bar.
pub const FROM: Rgba = [16, 80, 200, 255];
/// Far stop of the gradient bar.
pub const TO: Rgba = [220, 40, 80, 255];
/// x, y, w, h of the gradient bar.
pub const GRADIENT_RECT: [f32; 4] = [12.0, 8.0, 32.0, 40.0];
/// Ramp axis start: the centre of the bar's top pixel row, so a raster probe finds the near stop.
pub const GRADIENT_START: [f32; 2] = [28.0, 8.5];
/// Ramp axis end: the centre of the bar's bottom pixel row, so a raster probe finds the far stop.
pub const GRADIENT_END: [f32; 2] = [28.0, 47.5];

/// Fill of the single-rounded-corner rectangle.
pub const ASYMMETRIC_FILL: Rgba = [240, 200, 80, 255];
/// x, y, w, h of the single-rounded-corner rectangle.
pub const ASYMMETRIC_RECT: [f32; 4] = [152.0, 8.0, 40.0, 40.0];
/// [nw, ne, se, sw]: only the north-west corner is rounded, so a backend that applies one radius
/// to every corner is caught.
pub const ASYMMETRIC_RADII: [f32; 4] = [16.0, 0.0, 0.0, 0.0];

/// The marker body colour every hull is filled with.
pub const MARKER_FILL: Rgba = [16, 200, 140, 255];
/// The marker ring: white, as the canonical default preset draws it
/// (`ChartStyle::marker_ring == (render::WHITE, render::MARKER_RING_WEIGHT)`).
pub const RING: Rgba = [255, 255, 255, 255];
/// Ring width. Wider than the production ring weight (`crate::render::MARKER_RING_WEIGHT`, 0.75) on purpose: a 0.75-wide band
/// cannot fill a 1 px raster pixel, and this fixture is also read by the pixel backend, so the ring
/// has to be checkable there. The production weight stays covered by the marker tests in
/// crate::render::lines.
pub const RING_WEIGHT: f32 = 2.0;
/// Marker circumradius, shared by all five hulls.
pub const MARKER_RADIUS: f32 = 6.0;
/// Marker centre row.
pub const MARKER_Y: f32 = 56.0;
/// One centre per marker hull, in the order parity_fixture emits them: circle, square, diamond,
/// up triangle, down triangle.
pub const MARKER_CENTRES: [f32; 5] = [20.0, 46.0, 72.0, 98.0, 124.0];

/// Pixel probes, one pair per marker in MARKER_CENTRES order: [ring, body]. The ring probe lies
/// wholly inside that hull's ring band and the body probe wholly inside its fill, so a backend that
/// rounds the marker, rings it, or fills it differently paints a different pixel.
pub const MARKER_PROBES: [[[u32; 2]; 2]; 5] = [
    [[14, 56], [20, 56]],   // circle: the leftmost pixel is inside the annulus
    [[46, 51], [46, 56]],   // square: the row under the top edge is inside the band
    [[74, 52], [72, 56]],   // diamond: the upper-right edge band
    [[98, 58], [98, 56]],   // up triangle: the band under the base
    [[124, 52], [124, 56]], // down triangle: the band under the top edge
];

/// Pixel probes for the ramp: [from, to], one per stop.
pub const GRADIENT_PROBES: [[u32; 2]; 2] = [[28, 8], [28, 47]];
/// Pixel probes for the asymmetric rectangle: [rounded corner, square corner].
pub const ASYMMETRIC_PROBES: [[u32; 2]; 2] = [[154, 10], [190, 11]];

fn rect(x: f32, y: f32, w: f32, h: f32, fill: Fill, radii: [f32; 4]) -> Prim {
    Prim::Rect { x, y, w, h, fill: Some(fill), stroke: None, radii }
}

fn marker(shape: MarkerShape, cx: f32) -> Prim {
    match geometry(shape, cx, MARKER_Y, MARKER_RADIUS) {
        MarkerGeometry::Circle { r } => Prim::Circle { cx, cy: MARKER_Y, r, fill: Some(MARKER_FILL), stroke: Some((RING, RING_WEIGHT)) },
        MarkerGeometry::Polygon(pts) => Prim::Polygon { pts, fill: Some(Fill::solid(MARKER_FILL)), stroke: Some((RING, RING_WEIGHT)) },
    }
}

/// The scene: card, gradient bar, one rounded corner, and one of each ringed marker hull.
///
/// Deterministic and allocation-light (eight primitives), so it is cheap enough for every backend's
/// test suite to replay the same input.
pub fn parity_fixture() -> Vec<Prim> {
    let [cx, cy, cw, ch] = CARD_RECT;
    let [gx, gy, gw, gh] = GRADIENT_RECT;
    let [ax, ay, aw, ah] = ASYMMETRIC_RECT;
    let shapes = [MarkerShape::Circle, MarkerShape::Square, MarkerShape::Diamond, MarkerShape::UpTriangle, MarkerShape::DownTriangle];
    let mut out = vec![
        rect(cx, cy, cw, ch, Fill::solid(CARD), [0.0; 4]),
        rect(gx, gy, gw, gh, Fill::Linear { from: FROM, to: TO, start: GRADIENT_START, end: GRADIENT_END }, [0.0; 4]),
        rect(ax, ay, aw, ah, Fill::solid(ASYMMETRIC_FILL), ASYMMETRIC_RADII),
    ];
    out.extend(shapes.into_iter().zip(MARKER_CENTRES).map(|(s, x)| marker(s, x)));
    out
}
