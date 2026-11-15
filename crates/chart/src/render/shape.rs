//! Marker hulls: the ringed geometric markers a series can draw.

/// A marker hull. The wire format cannot name one per series yet, so every production caller
/// passes [`MarkerShape::Circle`]; [`all_shapes`] keeps the other four covered by a unit test.
// Only `Circle` is reachable until the chart model can name a marker hull per series; the other four
// stay covered by `lines::marker_tests::every_shape_costs_one_ringed_primitive_per_point`.
#[cfg_attr(not(test), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarkerShape {
    Circle,
    Square,
    Diamond,
    UpTriangle,
    DownTriangle,
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
