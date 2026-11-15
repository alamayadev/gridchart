//! Marker hulls: the ringed geometric markers a series can draw.

/// A marker hull. Production draws `marker_shape(series_index)`, the four-hull cycle below;
/// [`all_shapes`] keeps all five covered by a unit test, and `DownTriangle` is reserved for the
/// per-series configuration path tracked as `STYL-06`.
// Only the cycle's four hulls are reachable until the chart model can name a marker hull per
// series; `DownTriangle` stays covered by `lines::marker_tests::every_shape_costs_one_ringed_primitive_per_point`.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkerShape {
    Circle,
    Square,
    Diamond,
    UpTriangle,
    DownTriangle,
}

/// The hull cycle production markers draw, derived from the series index, never stored.
pub(crate) const MARKER_CYCLE: [MarkerShape; 4] = [MarkerShape::Circle, MarkerShape::Square, MarkerShape::Diamond, MarkerShape::UpTriangle];

/// The hull series `index` draws; the modulo, not a slice index, keeps a hostile index safe.
pub(crate) fn marker_shape(index: usize) -> MarkerShape {
    MARKER_CYCLE[index % 4]
}

/// The geometry of one marker: a circle, or a convex hull of at most four points, so a marker
/// costs one primitive and one point-budget slot (the price `MAX_MARKERS` counts).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MarkerGeometry {
    Circle { r: f32 },
    Polygon(Vec<[f32; 2]>),
}

/// Geometry of `shape` centred on `(cx, cy)` with circumradius `r`. Every hull is inscribed in
/// that circle, so the shapes read at the same visual weight.
pub(crate) fn geometry(shape: MarkerShape, cx: f32, cy: f32, r: f32) -> MarkerGeometry {
    // Half-side of the inscribed square, and the horizontal half-width of the triangles.
    let sq = r * std::f32::consts::FRAC_1_SQRT_2;
    let tri = r * 0.866_025_4;
    let pts = match shape {
        MarkerShape::Circle => return MarkerGeometry::Circle { r },
        MarkerShape::Square => vec![[cx - sq, cy - sq], [cx + sq, cy - sq], [cx + sq, cy + sq], [cx - sq, cy + sq]],
        MarkerShape::Diamond => vec![[cx, cy - r], [cx + r, cy], [cx, cy + r], [cx - r, cy]],
        MarkerShape::UpTriangle => vec![[cx, cy - r], [cx + tri, cy + r / 2.0], [cx - tri, cy + r / 2.0]],
        MarkerShape::DownTriangle => vec![[cx, cy + r], [cx + tri, cy - r / 2.0], [cx - tri, cy - r / 2.0]],
    };
    MarkerGeometry::Polygon(pts)
}

/// Every hull, so the shapes the wire format cannot select yet stay covered.
#[cfg(test)]
pub(crate) fn all_shapes() -> [MarkerShape; 5] {
    [MarkerShape::Circle, MarkerShape::Square, MarkerShape::Diamond, MarkerShape::UpTriangle, MarkerShape::DownTriangle]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cycle_is_four_hulls_and_repeats_by_series_index() {
        assert_eq!(MARKER_CYCLE, [MarkerShape::Circle, MarkerShape::Square, MarkerShape::Diamond, MarkerShape::UpTriangle]);
        let cycle: Vec<MarkerShape> = (0..12).map(marker_shape).collect();
        let triple: Vec<MarkerShape> = MARKER_CYCLE.to_vec().into_iter().cycle().take(12).collect();
        assert_eq!(cycle, triple, "series indices cycle the four hulls three times over 0..12");
        assert!(!MARKER_CYCLE.contains(&MarkerShape::DownTriangle), "DownTriangle is reserved for STYL-06");
        assert_eq!(marker_shape(usize::MAX), MARKER_CYCLE[usize::MAX % 4], "the modulo keeps a hostile index safe");
    }
}
