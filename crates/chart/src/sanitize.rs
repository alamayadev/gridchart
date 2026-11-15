//! One central pass: after this module runs, every primitive a backend receives has finite
//! geometry, a resolvable gradient axis and radii already inside the box.

use crate::render::MIN_GRADIENT_SPAN;
use crate::{Fill, Paint, Rgba};

/// A finite coordinate, radius, angle or stroke width.
pub(crate) fn fin(v: f32) -> bool {
    v.is_finite()
}

/// A stroke survives only with a finite positive width, capped so one stroke cannot dominate a page
/// (a zero-weight ring would vanish instead of degrading, so it is dropped outright).
pub(crate) fn clamp_stroke(s: Option<(Rgba, f32)>) -> Option<(Rgba, f32)> {
    s.filter(|(_, w)| fin(*w) && *w > 0.0).map(|(c, w)| (c, w.min(50.0)))
}

/// D-03: a ramp a backend cannot resolve degrades to its start colour, never to a dropped shape.
pub(crate) fn clamp_fill(f: Paint) -> Paint {
    let Some(Fill::Linear { from, to, start, end }) = f else { return f };
    if !(fin(start[0]) && fin(start[1]) && fin(end[0]) && fin(end[1])) {
        return Some(Fill::solid(from));
    }
    if (end[0] - start[0]).abs() < MIN_GRADIENT_SPAN && (end[1] - start[1]).abs() < MIN_GRADIENT_SPAN {
        return Some(Fill::solid(from));
    }
    Some(Fill::Linear { from, to, start, end })
}

/// D-03: radii a backend cannot honour degrade to square corners, never to a dropped shape. The
/// bound is half the short side, computed before the clamp so `f32::clamp` never sees a NaN bound;
/// a radii array with any non-finite corner is hostile as a whole and goes fully sharp.
pub(crate) fn clamp_radii(radii: [f32; 4], w: f32, h: f32) -> [f32; 4] {
    if !radii.iter().all(|r| fin(*r)) {
        return [0.0; 4];
    }
    let bound = (w.min(h) / 2.0).max(0.0);
    radii.map(|r| r.clamp(0.0, bound))
}
