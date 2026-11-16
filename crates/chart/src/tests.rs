use std::sync::Arc;

use gridcraft_core::{CellRef, Value};
use gridcraft_model::{Anchor, Chart, ChartKind, Color, LegendPos, Series, SparklineKind, Style, Workbook};

use crate::*;

mod budget;
mod data_table;

const ALL_KINDS: [ChartKind; 26] = [
    ChartKind::ColumnClustered,
    ChartKind::ColumnStacked,
    ChartKind::ColumnStacked100,
    ChartKind::BarClustered,
    ChartKind::BarStacked,
    ChartKind::BarStacked100,
    ChartKind::Line,
    ChartKind::LineMarkers,
    ChartKind::LineStacked,
    ChartKind::Pie,
    ChartKind::Doughnut,
    ChartKind::Area,
    ChartKind::AreaStacked,
    ChartKind::AreaStacked100,
    ChartKind::Scatter,
    ChartKind::ScatterLines,
    ChartKind::Bubble,
    ChartKind::Radar,
    ChartKind::Histogram,
    ChartKind::Waterfall,
    ChartKind::Funnel,
    ChartKind::Treemap,
    ChartKind::Sunburst,
    ChartKind::BoxWhisker,
    ChartKind::Stock,
    ChartKind::Combo,
];

fn chart(kind: ChartKind) -> Chart {
    Chart {
        id: 1,
        kind,
        anchor: Anchor::default(),
        title: Some("Quarterly Sales".into()),
        series: vec![],
        legend: LegendPos::Bottom,
        data_labels: true,
        data_table: None,
        gridlines: true,
        style: 0,
        hole_size: 75,
        first_slice_angle: 0,
        x_title: Some("Quarter".into()),
        y_title: Some("Revenue".into()),
        source: None,
        by_rows: false,
        pivot: None,
    }
}

fn series(name: &str, values: &[f64], color: Rgba, kind: ChartKind) -> SeriesData {
    SeriesData {
        name: name.into(),
        values: values.iter().map(|v| Some(*v)).collect(),
        x: Some(values.iter().enumerate().map(|(i, _)| Some(i as f64 * 1.5 + 2.0)).collect()),
        sizes: Some(values.iter().map(|v| Some(v.abs() + 1.0)).collect()),
        color,
        kind,
        secondary: false,
        smooth: false,
        trendlines: Vec::new(),
        number_format: "$#,##0".into(),
    }
}

fn sample(kind: ChartKind) -> ChartData {
    let cats = ["Q1", "Q2", "Q3", "Q4", "Q5", "Q6"];
    let k2 = if kind == ChartKind::Combo { ChartKind::Line } else { kind };
    let k1 = if kind == ChartKind::Combo { ChartKind::ColumnClustered } else { kind };
    let mut b = series("East", &[38000.0, 30000.0, 45000.0, 61870.0, 52000.0, 41000.0], [200, 80, 40, 255], k2);
    b.secondary = kind == ChartKind::Combo;
    ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: cats.iter().map(ToString::to_string).collect(),
        series: vec![
            series("North", &[12000.0, -4000.0, 23000.0, 31000.0, 18000.0, 27000.0], [20, 96, 130, 255], k1),
            b,
            series("South", &[9000.0, 11000.0, 8000.0, 15000.0, 21000.0, 17000.0], [25, 107, 36, 255], k2),
        ],
    }
}

/// A rect prim flattened for grouping: x, y, w, h, per-corner radii.
type BarRect = (f32, f32, f32, f32, [f32; 4]);

fn coords(p: &Prim) -> Vec<f32> {
    match p {
        Prim::Rect { x, y, w, h, .. } => vec![*x, *y, x + w, y + h],
        Prim::Line { pts, width, .. } => pts.iter().flat_map(|p| [p[0], p[1]]).chain([*width]).collect(),
        Prim::Polygon { pts, .. } => pts.iter().flat_map(|p| [p[0], p[1]]).collect(),
        Prim::Wedge { cx, cy, r_outer, r_inner, a0, a1, .. } => vec![*cx, *cy, *r_outer, *r_inner, *a0, *a1],
        Prim::Circle { cx, cy, r, .. } => vec![*cx, *cy, *r],
        Prim::Text { x, y, size, rotation, .. } => vec![*x, *y, *size, *rotation],
    }
}

fn assert_sane(prims: &[Prim], w: f32, h: f32) {
    let eps = 0.01;
    for p in prims {
        assert!(coords(p).iter().all(|v| v.is_finite()), "non-finite: {p:?}");
        let inside = |x: f32, y: f32| x >= -eps && x <= w + eps && y >= -eps && y <= h + eps;
        match p {
            Prim::Rect { x, y, w: rw, h: rh, .. } => assert!(inside(*x, *y) && inside(x + rw, y + rh), "{p:?}"),
            Prim::Line { pts, .. } | Prim::Polygon { pts, .. } => assert!(pts.iter().all(|q| inside(q[0], q[1])), "{p:?}"),
            Prim::Wedge { cx, cy, r_outer, .. } => assert!(inside(cx - r_outer, cy - r_outer) && inside(cx + r_outer, cy + r_outer), "{p:?}"),
            Prim::Circle { cx, cy, r, .. } => assert!(inside(cx - r, cy - r) && inside(cx + r, cy + r), "{p:?}"),
            Prim::Text { x, y, .. } => assert!(inside(*x, *y), "{p:?}"),
        }
    }
}

fn texts(prims: &[Prim]) -> Vec<&str> {
    prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn every_kind_renders_sanely() {
    let m = ApproxMeasure;
    for kind in ALL_KINDS {
        for legend in [LegendPos::Bottom, LegendPos::Top, LegendPos::Left, LegendPos::Right] {
            let mut c = chart(kind);
            c.legend = legend;
            let d = sample(kind);
            let (w, h) = (640.0, 400.0);
            let prims = render(&c, &d, w, h, &m);
            assert!(prims.len() > 3, "{kind:?}: too few prims");
            assert_sane(&prims, w, h);
            let t = texts(&prims);
            assert!(t.contains(&"Quarterly Sales"), "{kind:?}: no title");
            let entries = legend_entries(&c, &d);
            for (name, _, _) in &entries {
                assert!(t.contains(&name.as_str()), "{kind:?} {legend:?}: legend entry {name} missing");
            }
            let n_shapes = prims.iter().filter(|p| !matches!(p, Prim::Text { .. })).count();
            assert!(n_shapes > 2, "{kind:?}: nothing drawn");
        }
    }
}

#[test]
fn doughnut_draws_each_series_as_a_ring() {
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    c.data_labels = false;
    c.hole_size = 25;
    c.first_slice_angle = 0;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["A", "B", "C"].iter().map(|s| s.to_string()).collect(),
        series: vec![
            series("Inner", &[1.0, 2.0, 3.0], [20, 96, 130, 255], ChartKind::Doughnut),
            series("Middle", &[3.0, 2.0, 1.0], [221, 132, 82, 255], ChartKind::Doughnut),
            series("Outer", &[1.0, 1.0, 1.0], [89, 161, 79, 255], ChartKind::Doughnut),
        ],
    };
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let wedges: Vec<(f32, f32, f32)> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { r_outer, r_inner, a0, .. } => Some((*r_outer, *r_inner, *a0)),
            _ => None,
        })
        .collect();
    assert_eq!(wedges.len(), 9, "three rings of three slices");
    // Excel draws the first series as the innermost ring; the hole is 25% of the radius.
    assert!(wedges[0].0 < wedges[3].0 && wedges[3].0 < wedges[6].0, "{wedges:?}");
    assert!(wedges[0].1 > 0.0, "hole: {wedges:?}");
}

#[test]
fn doughnut_with_many_series_still_draws_the_outer_rings() {
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    c.data_labels = false;
    c.hole_size = 75;
    // The annulus is a few dozen pixels tall and there are far more series than pixels, so every
    // ring is thinner than 1 px. The chart used to draw nothing at all.
    let many = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["A", "B"].iter().map(|s| s.to_string()).collect(),
        series: (0..255).map(|i| series(&format!("S{i}"), &[1.0, 1.0], [20, 96, 130, 255], ChartKind::Doughnut)).collect(),
    };
    let single = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["A", "B"].iter().map(|s| s.to_string()).collect(),
        // Two series, so the title fallback (a lone series names the chart) does not shrink the plot.
        series: vec![
            series("Only", &[1.0, 1.0], [20, 96, 130, 255], ChartKind::Doughnut),
            series("Other", &[1.0, 1.0], [221, 132, 82, 255], ChartKind::Doughnut),
        ],
    };
    let (w, h) = (640.0, 400.0);
    let radii = |d: &ChartData| -> Vec<f32> {
        render(&c, d, w, h, &ApproxMeasure)
            .iter()
            .filter_map(|p| match p {
                Prim::Wedge { r_outer, .. } => Some(*r_outer),
                _ => None,
            })
            .collect()
    };
    let outer = radii(&many);
    assert!(!outer.is_empty(), "a sub-pixel ring must not suppress the whole chart");
    let mut distinct = outer.clone();
    distinct.sort_by(f32::total_cmp);
    distinct.dedup();
    assert!(distinct.len() > 8, "only {} rings drawn", distinct.len());
    assert!(distinct.len() < 255, "too many rings for the pixels available: {}", distinct.len());
    // The outermost ring still reaches the plot radius the single-ring chart uses.
    let r = radii(&single).into_iter().fold(0.0f32, f32::max);
    let drawn = distinct.last().copied().unwrap_or(0.0);
    assert!((drawn - r).abs() < 0.01, "outermost ring {drawn} of {r}");
}

#[test]
fn doughnut_one_pixel_rings_skip_the_separator_stroke() {
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    c.data_labels = false;
    c.hole_size = 75;
    // 255 series over a 46-pixel annulus: every drawn ring is one pixel tall, narrower than the
    // 1.5-pixel white separator, which would otherwise paint over both neighbouring rings.
    let thin = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["A", "B"].iter().map(|s| s.to_string()).collect(),
        series: (0..255).map(|i| series(&format!("S{i}"), &[1.0, 1.0], [20, 96, 130, 255], ChartKind::Doughnut)).collect(),
    };
    let strokes: Vec<Option<(Rgba, f32)>> = render(&c, &thin, 640.0, 400.0, &ApproxMeasure)
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { stroke, .. } => Some(*stroke),
            _ => None,
        })
        .collect();
    assert!(!strokes.is_empty(), "the chart must still draw its rings");
    assert!(strokes.iter().all(Option::is_none), "a 1.5px stroke covers a 1px band: {strokes:?}");

    // A ring with room for the separator keeps it.
    let thick = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["A", "B"].iter().map(|s| s.to_string()).collect(),
        series: vec![
            series("Inner", &[1.0, 1.0], [20, 96, 130, 255], ChartKind::Doughnut),
            series("Outer", &[1.0, 1.0], [221, 132, 82, 255], ChartKind::Doughnut),
        ],
    };
    let strokes: Vec<Option<(Rgba, f32)>> = render(&c, &thick, 640.0, 400.0, &ApproxMeasure)
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { stroke, .. } => Some(*stroke),
            _ => None,
        })
        .collect();
    assert!(!strokes.is_empty());
    assert!(strokes.iter().all(Option::is_some), "wide rings keep their separators: {strokes:?}");
}

#[test]
fn doughnut_wedge_budget_bounds_many_series_and_categories() {
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    c.data_labels = false;
    c.hole_size = 75;
    c.first_slice_angle = 0;
    // 20,000 categories across 80 series would be hundreds of thousands of wedges: the per-series
    // limit does not bound the product. The first ten categories of every ring are the largest.
    let cats = 20_000usize;
    let values: Vec<f64> = (0..cats).map(|j| if j < 10 { 1000.0 } else { 1.0 }).collect();
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: (0..cats).map(|j| format!("C{j}")).collect(),
        series: (0..80)
            .map(|i| SeriesData { name: format!("S{i}"), values: values.iter().map(|v| Some(*v)).collect(), ..SeriesData::default() })
            .collect(),
    };
    let wedges: Vec<(f32, f32)> = render(&c, &d, 640.0, 400.0, &ApproxMeasure)
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { r_outer, a0, a1, .. } => Some((*r_outer, *a1 - *a0)),
            _ => None,
        })
        .collect();
    assert!(!wedges.is_empty(), "the innermost ring must still be drawn");
    let mut radii: Vec<f32> = wedges.iter().map(|(r, _)| *r).collect();
    radii.sort_by(f32::total_cmp);
    radii.dedup();
    assert!(radii.len() > 1, "every drawable ring is capped, not dropped");
    // A slice crossing 12 o'clock is split into two primitives, so each ring may add one.
    let budget = crate::render::MAX_WEDGES + radii.len();
    assert!(wedges.len() <= budget, "{} wedges for {} rings", wedges.len(), radii.len());

    // The first (innermost) ring keeps its largest slices and still fills the circle.
    let inner: Vec<f32> = wedges.iter().filter(|(r, _)| *r == radii[0]).map(|(_, a)| *a).collect();
    assert!(inner.len() < cats, "the ring was not capped: {}", inner.len());
    assert_eq!(inner.iter().filter(|a| **a > 0.1).count(), 10, "largest slices went missing");
    let cover: f32 = inner.iter().sum();
    assert!((cover - std::f32::consts::TAU).abs() < 0.01, "first ring covers {cover}");
}

#[test]
fn doughnut_first_slice_angle_rotates_the_start() {
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    c.first_slice_angle = 90;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["A", "B"].iter().map(|s| s.to_string()).collect(),
        series: vec![series("Only", &[1.0, 1.0], [20, 96, 130, 255], ChartKind::Doughnut)],
    };
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let first = prims.iter().find_map(|p| match p {
        Prim::Wedge { a0, .. } => Some(*a0),
        _ => None,
    });
    assert!(first.is_some_and(|a| (a - std::f32::consts::FRAC_PI_2).abs() < 0.01), "{first:?}");
}

/// Every slice must occupy exactly its angular share, even when the first slice angle pushes the
/// start past 12 o'clock (the split at TAU used to put whole slices over their neighbours).
#[test]
fn rotated_wedges_cover_each_slice_once_without_overlap() {
    const TAU: f32 = std::f32::consts::TAU;
    for kind in [ChartKind::Doughnut, ChartKind::Pie] {
        for angle in [180u16, 270] {
            let mut c = chart(kind);
            c.title = None;
            c.legend = LegendPos::None;
            c.data_labels = false;
            c.first_slice_angle = angle;
            let d = ChartData {
                category_levels: vec![],
                palette: vec![[230, 126, 34, 255], [39, 174, 96, 255], [142, 68, 173, 255]],
                categories: ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect(),
                series: vec![series("Only", &[1.0; 4], [20, 96, 130, 255], kind)],
            };
            let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
            let wedges: Vec<(f32, f32)> = prims
                .iter()
                .filter_map(|p| match p {
                    Prim::Wedge { a0, a1, .. } => Some((*a0, *a1)),
                    _ => None,
                })
                .collect();
            assert!(wedges.len() >= 4, "{kind:?} at {angle}: {wedges:?}");
            for (a0, a1) in &wedges {
                assert!((0.0..TAU).contains(a0), "{kind:?} at {angle}: start {a0} outside [0, TAU)");
                assert!(*a1 > *a0 && *a1 <= TAU + 1e-4, "{kind:?} at {angle}: bad sweep {a0}..{a1}");
            }
            // Sample away from the boundaries: a correct four-slice chart covers every angle once.
            let steps = 1440;
            for k in 0..steps {
                let theta = (k as f32 + 0.5) * TAU / steps as f32;
                let cover = wedges.iter().filter(|(a0, a1)| theta >= *a0 && theta < *a1).count();
                assert_eq!(cover, 1, "{kind:?} at {angle}: {theta} covered {cover} times by {wedges:?}");
            }
        }
    }
}

/// A slice that crosses 12 o'clock is filled as two halves but outlined once: stroking each half
/// separately ran the white separator along the shared radial edge, straight through the slice.
#[test]
fn slice_crossing_twelve_has_no_internal_seam() {
    const TAU: f32 = std::f32::consts::TAU;
    let mut c = chart(ChartKind::Pie);
    c.title = None;
    c.legend = LegendPos::None;
    c.data_labels = false;
    c.first_slice_angle = 90;
    let a = [230, 126, 34, 255];
    let b = [39, 174, 96, 255];
    let d = ChartData {
        category_levels: vec![],
        palette: vec![a, b],
        categories: ["A", "B"].iter().map(|s| s.to_string()).collect(),
        series: vec![series("Only", &[1.0, 1.0], a, ChartKind::Pie)],
    };
    let (w, h) = (400u32, 400u32);
    let prims = render(&c, &d, w as f32, h as f32, &ApproxMeasure);
    // The second slice starts at 270° and wraps past 0°, so it is drawn as two unstroked halves.
    let mut halves = 0;
    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut r = 0.0;
    for p in &prims {
        if let Prim::Wedge { cx: x, cy: y, r_outer, a0, a1, fill, stroke, .. } = p
            && *fill == Some(Fill::solid(b))
        {
            assert!(stroke.is_none(), "a split half must not stroke its shared edge: {p:?}");
            assert!(*a0 == 0.0 || *a1 == TAU, "the crossing slice is split at 12 o'clock: {p:?}");
            halves += 1;
            (cx, cy, r) = (*x, *y, *r_outer);
        }
    }
    assert_eq!(halves, 2, "one slice crosses 12 o'clock");
    // Exactly one outline covers the whole crossing slice, drawn as line primitives.
    assert_eq!(prims.iter().filter(|p| matches!(p, Prim::Line { color, .. } if *color == [255, 255, 255, 255])).count(), 1, "{prims:?}");
    // No white pixel inside the slice along 12 o'clock, where the shared edge used to be stroked.
    let img = rasterize(&prims, w, h, [255, 255, 255, 255]);
    for t in [0.3f32, 0.5, 0.7] {
        assert_eq!(pixel(&img, w, cx as u32, (cy - r * t) as u32), b, "white seam at {t} of the slice");
    }
}

#[test]
fn doughnut_legend_names_categories_and_matches_every_ring() {
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.data_labels = false;
    c.legend = LegendPos::Right;
    c.first_slice_angle = 180;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![[230, 126, 34, 255], [39, 174, 96, 255], [142, 68, 173, 255]],
        categories: ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect(),
        series: vec![
            series("Outer", &[1.0; 4], [20, 96, 130, 255], ChartKind::Doughnut),
            series("Inner", &[1.0; 4], [221, 132, 82, 255], ChartKind::Doughnut),
        ],
    };
    let entries = legend_entries(&c, &d);
    assert_eq!(entries.iter().map(|(name, _, _)| name.as_str()).collect::<Vec<_>>(), ["A", "B", "C", "D"]);

    let start = f32::from(c.first_slice_angle) * std::f32::consts::TAU / 360.0;
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let mut rings = Vec::new();
    for p in &prims {
        let Prim::Wedge { r_outer, a0, a1, fill, .. } = p else { continue };
        rings.push(*r_outer);
        let rel = ((a0 + a1) / 2.0 - start).rem_euclid(std::f32::consts::TAU);
        let cat = ((rel / (std::f32::consts::TAU / 4.0)) as usize).min(3);
        assert_eq!(
            fill.and_then(|f| match f {
                Fill::Solid(c) => Some(c),
                Fill::Linear { .. } => None,
            }),
            Some(entries[cat].1),
            "ring wedge at {a0}..{a1} does not match legend entry {}",
            entries[cat].0
        );
    }
    rings.sort_by(f32::total_cmp);
    rings.dedup();
    assert_eq!(rings.len(), 2, "two series must draw two rings");
}

/// The separator between slices, tiles and bars is the preset's own card at the preset's seam
/// weight — not a hardcoded white that outlines every slice on the dark family. The histogram
/// separator keeps its narrow 0.75 weight: a 1.5 stroke would trim three pixels off a hairline bar.
#[test]
fn slice_seams_are_the_card_at_the_preset_weight() {
    for style in [0u32, 4] {
        let tokens = chart_style(style);
        assert_eq!(tokens.slice_seam.0, tokens.card, "style {style}: the seam is the card");
        assert!((1.0..=2.5).contains(&tokens.slice_seam.1), "style {style}: seam weight {}", tokens.slice_seam.1);
        let seam = Some(tokens.slice_seam);
        for kind in [ChartKind::Pie, ChartKind::Doughnut] {
            let mut c = chart(kind);
            c.style = style;
            let mut d = sample(kind);
            d.series.truncate(1);
            let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
            let wedges: Vec<_> = p.iter().filter(|q| matches!(q, Prim::Wedge { .. })).collect();
            assert!(wedges.len() >= 3, "{kind:?} draws slices");
            for q in &wedges {
                let Prim::Wedge { stroke, .. } = q else { continue };
                assert_eq!(*stroke, seam, "style {style} {kind:?}: every slice carries the card seam");
            }
        }
        for (kind, want) in [(ChartKind::Treemap, seam), (ChartKind::Histogram, Some((tokens.slice_seam.0, 0.75)))] {
            let mut c = chart(kind);
            c.style = style;
            c.legend = LegendPos::None;
            let p = render(&c, &sample(kind), 640.0, 400.0, &ApproxMeasure);
            let tiles: Vec<_> = p
                .iter()
                .filter_map(|q| match q {
                    Prim::Rect { fill, stroke, .. } if *fill != Some(Fill::solid(tokens.card)) => Some(*stroke),
                    _ => None,
                })
                .collect();
            assert!(tiles.len() >= 3, "{kind:?} draws tiles");
            for stroke in tiles {
                assert_eq!(stroke, want, "style {style} {kind:?}: non-card rects carry the seam");
            }
        }
    }
}

/// One pie's slices: nine wide ones and one hairline. The wide ones label inside exactly as
/// before; the hairline gets a three-point leader and a label anchored outside the rim.
#[test]
fn pie_labels_that_do_not_fit_inside_step_outside_with_a_leader() {
    let palette: Vec<Rgba> = (0..10).map(|j| [(30 + j as u8 * 20), 96, 130, 255]).collect();
    let d = ChartData {
        category_levels: vec![],
        palette: palette.clone(),
        categories: (0..10).map(|j| format!("C{j}")).collect(),
        series: vec![series("Only", [100.0; 9].iter().chain([&0.4]).copied().collect::<Vec<f64>>().as_slice(), palette[0], ChartKind::Pie)],
    };
    let mut c = chart(ChartKind::Pie);
    c.title = None;
    c.legend = LegendPos::None;
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let (cx, cy, r) = prims
        .iter()
        .find_map(|p| match p {
            Prim::Wedge { cx, cy, r_outer, .. } => Some((*cx, *cy, *r_outer)),
            _ => None,
        })
        .expect("a wedge");
    assert_eq!(prims.iter().filter(|p| matches!(p, Prim::Wedge { .. })).count(), 10, "all ten slices draw");
    let ink = chart_style(0).text;
    let leaders: Vec<&Prim> = prims
        .iter()
        .filter(|p| matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == ink && (*width - 1.0).abs() < 1e-6))
        .collect();
    assert_eq!(leaders.len(), 1, "exactly one three-point leader in text ink: {leaders:?}");
    let Prim::Line { pts: lpts, .. } = leaders[0] else { panic!("leader") };
    let dist = ((lpts[0][0] - cx).powi(2) + (lpts[0][1] - cy).powi(2)).sqrt();
    assert!((dist - (r + 3.0)).abs() < 0.01, "the leader starts on the rim at outer + 3.0: {dist} vs {}", r + 3.0);
    let anchored: Vec<&Prim> = prims
        .iter()
        .filter(|p| matches!(p, Prim::Text { x, y, size, .. } if *size == 9.0 && (*x - lpts[2][0]).abs() < 1e-3 && (*y - lpts[2][1]).abs() < 1e-3))
        .collect();
    assert_eq!(anchored.len(), 1, "exactly one label sits on the leader's end");
    let mut anchors: Vec<(f32, f32)> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { x, y, size, .. } if *size == 9.0 => Some((*x, *y)),
            _ => None,
        })
        .collect();
    anchors.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let before = anchors.len();
    anchors.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6);
    assert_eq!(anchors.len(), before, "no label emitted twice");
    // The same hairline on the dark preset carries the dark family's text ink, not the inside
    // contrast ink `on(fill)` would pick.
    let mut c4 = c;
    c4.style = 4;
    let prims4 = render(&c4, &d, 640.0, 400.0, &ApproxMeasure);
    let outside = prims4
        .iter()
        .find(|p| matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == chart_style(4).text && (*width - 1.0).abs() < 1e-6))
        .expect("the dark leader");
    let Prim::Line { pts: l4, .. } = outside else { panic!("leader") };
    let label4 = prims4
        .iter()
        .find_map(|p| match p {
            Prim::Text { x, y, color, size, .. } if *size == 9.0 && (*x - l4[2][0]).abs() < 1e-3 && (*y - l4[2][1]).abs() < 1e-3 => Some(*color),
            _ => None,
        })
        .expect("the dark outside label");
    assert_eq!(label4, chart_style(4).text, "the outside label uses the preset's text ink");
}

/// Every label that already fits inside keeps its exact anchor, alignment and contrast ink: the
/// fallback only adds a path for labels that were dropped before.
#[test]
fn pie_labels_keep_their_inside_placement() {
    let c = chart(ChartKind::Pie);
    let d = sample(ChartKind::Pie);
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    assert!(
        prims.iter().all(
            |p| !matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == chart_style(0).text && (*width - 1.0).abs() < 1e-6)
        ),
        "no leader on a pie whose labels all fit"
    );
    let ink = |fill: Rgba| -> Rgba {
        let luma = 0.299 * fill[0] as f32 + 0.587 * fill[1] as f32 + 0.114 * fill[2] as f32;
        if luma < 150.0 { [0xFF; 4] } else { [0x26, 0x26, 0x26, 0xFF] }
    };
    for p in &prims {
        if let Prim::Wedge { a0, a1, fill, .. } = p {
            let mid = (a0 + a1) / 2.0;
            let fill = match fill {
                Some(Fill::Solid(c)) => *c,
                _ => continue,
            };
            let (cx, cy, r) = prims
                .iter()
                .find_map(|q| match q {
                    Prim::Wedge { cx, cy, r_outer, .. } => Some((*cx, *cy, *r_outer)),
                    _ => None,
                })
                .expect("a wedge");
            let rr = r * 0.65;
            let (want_x, want_y) = (cx + rr * mid.sin(), cy - rr * mid.cos());
            let hit = prims.iter().any(|q| {
                matches!(q, Prim::Text { x, y, size, color, align, .. } if *size == 9.0
                && (*x - want_x).abs() < 1e-3 && (*y - want_y).abs() < 1e-3
                && *color == ink(fill) && *align == HAlign::Center)
            });
            assert!(hit, "slice at {a0}..{a1} keeps its inside anchor");
        }
    }
}

/// A single-ring doughnut is the one doughnut shape allowed to step outside: its thin slices get
/// the same three-point leader a pie gets, anchored on that ring's own outer radius.
#[test]
fn doughnut_single_ring_labels_small_slices_outside() {
    let d = ChartData {
        category_levels: vec![],
        palette: (0..10).map(|j| [(30 + j as u8 * 20), 96, 130, 255]).collect(),
        categories: (0..10).map(|j| format!("C{j}")).collect(),
        series: vec![series(
            "Only",
            [100.0; 9].iter().chain([&0.4]).copied().collect::<Vec<f64>>().as_slice(),
            [20, 96, 130, 255],
            ChartKind::Doughnut,
        )],
    };
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    c.hole_size = 75;
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let (cx, cy, outer) = prims
        .iter()
        .find_map(|p| match p {
            Prim::Wedge { cx, cy, r_outer, .. } => Some((*cx, *cy, *r_outer)),
            _ => None,
        })
        .expect("a wedge");
    let ink = chart_style(0).text;
    let leaders: Vec<&Prim> = prims
        .iter()
        .filter(|p| matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == ink && (*width - 1.0).abs() < 1e-6))
        .collect();
    assert_eq!(leaders.len(), 1, "one leader for the hairline slice");
    let Prim::Line { pts, .. } = leaders[0] else { panic!("leader") };
    let dist = ((pts[0][0] - cx).powi(2) + (pts[0][1] - cy).powi(2)).sqrt();
    assert!((dist - (outer + 3.0)).abs() < 0.01, "the leader starts on the ring's outer radius: {dist} vs {}", outer + 3.0);
    let anchored = prims
        .iter()
        .filter(|p| matches!(p, Prim::Text { x, y, size, .. } if *size == 9.0 && (*x - pts[2][0]).abs() < 1e-3 && (*y - pts[2][1]).abs() < 1e-3))
        .count();
    assert_eq!(anchored, 1, "the label sits on the leader's end");
}

/// A multi-ring doughnut never emits a leader: outside text would collide across rings. Every
/// label it does emit stays within the drawn rings' radial extent.
#[test]
fn doughnut_multiple_rings_keep_every_label_inside() {
    let d = ChartData {
        category_levels: vec![],
        palette: (0..6).map(|j| [(30 + j as u8 * 30), 96, 130, 255]).collect(),
        categories: (0..6).map(|j| format!("C{j}")).collect(),
        series: vec![
            series("A", &[1.0; 6], [20, 96, 130, 255], ChartKind::Doughnut),
            series("B", &[1.0; 6], [221, 132, 82, 255], ChartKind::Doughnut),
        ],
    };
    let mut c = chart(ChartKind::Doughnut);
    c.title = None;
    c.legend = LegendPos::None;
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let ink = chart_style(0).text;
    assert!(
        prims.iter().all(|p| !matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == ink && (*width - 1.0).abs() < 1e-6)),
        "no leader on a two-ring doughnut"
    );
    let (cx, cy) = prims
        .iter()
        .find_map(|p| match p {
            Prim::Wedge { cx, cy, .. } => Some((*cx, *cy)),
            _ => None,
        })
        .expect("a wedge");
    let ri = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { r_inner, .. } => Some(*r_inner),
            _ => None,
        })
        .fold(f32::INFINITY, f32::min);
    let ro = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { r_outer, .. } => Some(*r_outer),
            _ => None,
        })
        .fold(f32::NEG_INFINITY, f32::max);
    let labels: Vec<(f32, f32)> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { x, y, size, .. } if *size == 9.0 => Some((*x, *y)),
            _ => None,
        })
        .collect();
    assert!(!labels.is_empty(), "the inside gate still emits labels");
    for (x, y) in labels {
        let dist = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
        assert!(dist >= ri - 1e-3 && dist <= ro + 1e-3, "label at {dist} outside the rings [{ri}, {ro}]");
    }
}

/// A zero-value slice has no sweep, so it draws no wedge, no leader and no label.
#[test]
fn a_zero_value_slice_gets_neither_label_nor_leader() {
    let vals = [10.0, 0.0, 20.0, 30.0];
    let d = ChartData {
        category_levels: vec![],
        palette: (0..4).map(|j| [(30 + j as u8 * 40), 96, 130, 255]).collect(),
        categories: (0..4).map(|j| format!("C{j}")).collect(),
        series: vec![series("Only", &vals, [20, 96, 130, 255], ChartKind::Pie)],
    };
    let mut c = chart(ChartKind::Pie);
    c.title = None;
    c.legend = LegendPos::None;
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    assert_eq!(prims.iter().filter(|p| matches!(p, Prim::Wedge { .. })).count(), 3, "three wedges for four values");
    let ink = chart_style(0).text;
    assert!(
        prims.iter().all(|p| !matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == ink && (*width - 1.0).abs() < 1e-6)),
        "no leader for a zero slice"
    );
    let texts: Vec<&String> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, size, .. } if *size == 9.0 => Some(text),
            _ => None,
        })
        .collect();
    // The series number format is "$#,##0" (sample()'s), so a zero label would read "$0".
    assert_eq!(texts.iter().filter(|t| t.as_str() == "$0").count(), 0, "no label for the zero slice: {texts:?}");
}

/// A measure wider than `ApproxMeasure` — the UI and SVG measures are not the test measure — must
/// not push a label or a leader outside the card, even in a small chart.
struct WideMeasure;

impl Measure for WideMeasure {
    fn text_width(&self, s: &str, size: f32, _bold: bool) -> f32 {
        s.chars().count() as f32 * size * 1.15
    }
}

#[test]
fn outside_labels_and_leaders_stay_inside_a_small_chart() {
    let d = ChartData {
        category_levels: vec![],
        palette: (0..12).map(|j| [(20 + j as u8 * 17), 96, 130, 255]).collect(),
        categories: (0..12).map(|j| format!("Slice {j}")).collect(),
        series: vec![series("Only", [100.0; 11].iter().chain([&0.4]).copied().collect::<Vec<f64>>().as_slice(), [20, 96, 130, 255], ChartKind::Pie)],
    };
    let mut c = chart(ChartKind::Pie);
    c.title = None;
    c.legend = LegendPos::None;
    let (w, h) = (320.0f32, 220.0f32);
    let prims = render(&c, &d, w, h, &WideMeasure);
    let ink = chart_style(0).text;
    let mut leaders = 0;
    for p in &prims {
        if let Prim::Line { pts, color, width, .. } = p {
            if *color == ink && (*width - 1.0).abs() < 1e-6 {
                assert_eq!(pts.len(), 3, "a leader is exactly three points");
                leaders += 1;
            }
            for q in pts {
                assert!(q[0] >= -0.01 && q[0] <= w + 0.01 && q[1] >= -0.01 && q[1] <= h + 0.01, "line point {q:?} escapes the box");
            }
        }
        if let Prim::Text { x, y, text, .. } = p {
            assert!(*x >= -0.01 && *x <= w + 0.01 && *y >= -0.01 && *y <= h + 0.01, "label anchor ({x}, {y}) escapes the box");
            assert!(!text.is_empty(), "no empty label text");
        }
    }
    assert!(leaders > 0, "the hairline slice still gets its leader under the wide measure");
}

#[test]
fn outside_label_placement_is_deterministic() {
    let d = ChartData {
        category_levels: vec![],
        palette: (0..12).map(|j| [(20 + j as u8 * 17), 96, 130, 255]).collect(),
        categories: (0..12).map(|j| format!("Slice {j}")).collect(),
        series: vec![series("Only", [100.0; 11].iter().chain([&0.4]).copied().collect::<Vec<f64>>().as_slice(), [20, 96, 130, 255], ChartKind::Pie)],
    };
    let mut c = chart(ChartKind::Pie);
    c.title = None;
    c.legend = LegendPos::None;
    let a = render(&c, &d, 320.0, 220.0, &WideMeasure);
    let b = render(&c, &d, 320.0, 220.0, &WideMeasure);
    assert_eq!(format!("{a:?}"), format!("{b:?}"), "two renders of the same chart are byte-identical");
}

#[test]
fn legend_counts() {
    let d = sample(ChartKind::ColumnClustered);
    assert_eq!(legend_entries(&chart(ChartKind::ColumnClustered), &d).len(), 3);
    assert_eq!(legend_entries(&chart(ChartKind::Pie), &d).len(), 6);
    assert_eq!(legend_entries(&chart(ChartKind::Waterfall), &d).len(), 3);
    assert_eq!(legend_entries(&chart(ChartKind::Histogram), &d).len(), 0);
    let mut c = chart(ChartKind::Line);
    c.legend = LegendPos::None;
    assert!(legend_entries(&c, &d).is_empty());
}

#[test]
fn nice_axis() {
    let s = nice_scale(0.0, 61870.0, true);
    assert_eq!((s.min, s.max, s.step), (0.0, 70000.0, 10000.0));
    let s = nice_scale(0.0, 100.0, true);
    assert_eq!((s.min, s.max, s.step), (0.0, 120.0, 20.0));
    let s = nice_scale(-4000.0, 31000.0, true);
    assert!(s.min < -4000.0 && s.min >= -10000.0 && s.max >= 31000.0);
    // All zero.
    let s = nice_scale(0.0, 0.0, true);
    assert_eq!((s.min, s.max), (0.0, 1.0));
    // Far from zero: no forced zero.
    let s = nice_scale(50.0, 55.0, false);
    assert!(s.min >= 45.0 && s.max <= 60.0, "{s:?}");
    // Close to zero: zero included.
    assert_eq!(nice_scale(10.0, 90.0, false).min, 0.0);
    // Negative only.
    let s = nice_scale(-80.0, -10.0, false);
    assert_eq!(s.max, 0.0);
    assert!(s.min <= -80.0);
    for (lo, hi) in [(0.0, 1e-300), (-1e308, 1e308), (f64::NAN, 5.0), (0.001, 0.0013), (3.0, 3.0), (-7.0, -7.0), (1e15, 1e15 + 1.0)] {
        let s = nice_scale(lo, hi, false);
        assert!(s.min.is_finite() && s.max.is_finite() && s.step.is_finite() && s.max > s.min && s.step > 0.0, "{lo} {hi} {s:?}");
        let n = s.ticks().len();
        assert!((2..=51).contains(&n));
    }
    // Ticks count in 4..=10 for typical data.
    for hi in [1.0, 7.0, 13.0, 99.0, 250.0, 1234.0, 61870.0, 999999.0] {
        let n = nice_scale(0.0, hi, true).ticks().len();
        assert!((4..=10).contains(&n), "{hi}: {n}");
    }
}

#[test]
fn axis_labels_use_number_format() {
    assert_eq!(format_number(40000.0, "$#,##0", Some(10000.0)), "$40,000");
    assert_eq!(format_number(0.30000000000000004, "General", Some(0.1)), "0.3");
    assert_eq!(format_number(1e-17, "General", Some(0.2)), "0");
    let c = chart(ChartKind::ColumnClustered);
    let prims = render(&c, &sample(ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let t = texts(&prims);
    assert!(t.contains(&"$70,000"), "{t:?}");
    assert!(t.contains(&"$40,000"));
}

#[test]
fn resolve_from_workbook() {
    let mut wb = Workbook::new();
    let cur = wb.styles.intern(Style { num_fmt: gridcraft_model::NumFmt(Arc::from("$#,##0")), ..Style::default() });
    {
        let s = wb.sheet_mut(0).unwrap();
        s.name = "Sales".into();
        s.set_value(CellRef::new(0, 1), Value::from("Revenue"));
        for (i, (q, v)) in [("Q1", 100.0), ("Q2", 250.0), ("Q3", 175.0)].iter().enumerate() {
            let r = i as u32 + 1;
            s.set_value(CellRef::new(r, 0), Value::from(*q));
            s.set_value(CellRef::new(r, 1), Value::Number(*v));
            s.set_style(CellRef::new(r, 1), cur);
        }
        s.set_value(CellRef::new(2, 2), Value::Number(5.0));
    }
    let mut c = chart(ChartKind::ColumnClustered);
    c.series = vec![
        Series {
            name: Some("Sales!$B$1".into()),
            categories: Some("Sales!$A$2:$A$4".into()),
            values: "Sales!$B$2:$B$4".into(),
            bubble_sizes: None,
            color: None,
            secondary: false,
            kind: None,
            quartiles: Default::default(),
            smooth: false,
            trendlines: Vec::new(),
        },
        Series {
            name: None,
            categories: None,
            values: "=Sales!$C$2:$C$4".into(),
            bubble_sizes: None,
            color: Some(Color::rgb(1, 2, 3)),
            secondary: true,
            kind: None,
            quartiles: Default::default(),
            smooth: false,
            trendlines: Vec::new(),
        },
        Series {
            name: Some("Literal".into()),
            categories: None,
            values: "{1,2,3,4}".into(),
            bubble_sizes: None,
            color: None,
            secondary: false,
            kind: None,
            quartiles: Default::default(),
            smooth: false,
            trendlines: Vec::new(),
        },
    ];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.categories, vec!["Q1", "Q2", "Q3"]);
    assert_eq!(d.series.len(), 3);
    let s0 = &d.series[0];
    assert_eq!(s0.name, "Revenue");
    assert_eq!(s0.values, vec![Some(100.0), Some(250.0), Some(175.0)]);
    assert_eq!(s0.number_format, "$#,##0");
    assert_eq!(s0.color, series_color(&c, 0));
    assert_eq!(s0.color, chart_style_preset(c.style)[0]);
    // D-01: the workbook theme's accents never reach a chart fill, however loud they are.
    let mut themed = wb.clone();
    themed.theme.colors[4] = 0x4F6D8A;
    assert_eq!(resolve(&themed, 0, &c).series[0].color, s0.color);
    let s1 = &d.series[1];
    assert_eq!(s1.name, "Series2");
    assert_eq!(s1.values, vec![None, Some(5.0), None]);
    assert_eq!(s1.color, [1, 2, 3, 255]);
    assert!(s1.secondary);
    assert_eq!(d.series[2].name, "Literal");
    assert_eq!(d.series[2].values.len(), 4);

    // Default categories.
    c.series.remove(0);
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.categories, vec!["1", "2", "3", "4"]);

    // Combo kinds and scatter X values.
    let mut c = chart(ChartKind::Combo);
    c.series = vec![
        Series {
            name: None,
            categories: None,
            values: "Sales!B2:B4".into(),
            bubble_sizes: None,
            color: None,
            secondary: false,
            kind: None,
            quartiles: Default::default(),
            smooth: false,
            trendlines: Vec::new(),
        },
        Series {
            name: None,
            categories: None,
            values: "Sales!B2:B4".into(),
            bubble_sizes: None,
            color: None,
            secondary: true,
            kind: None,
            quartiles: Default::default(),
            smooth: false,
            trendlines: Vec::new(),
        },
    ];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.series[0].kind, ChartKind::ColumnClustered);
    assert_eq!(d.series[1].kind, ChartKind::Line);
    let mut c = chart(ChartKind::Bubble);
    c.series = vec![Series {
        name: None,
        categories: Some("Sales!B2:B4".into()),
        values: "Sales!B2:B4".into(),
        bubble_sizes: Some("{1,2,3}".into()),
        color: None,
        secondary: false,
        kind: None,
        quartiles: Default::default(),
        smooth: false,
        trendlines: Vec::new(),
    }];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.series[0].x.as_ref().map(|x| x.len()), Some(3));
    assert_eq!(d.series[0].sizes, Some(vec![Some(1.0), Some(2.0), Some(3.0)]));

    // Bad formulas never panic.
    let mut c = chart(ChartKind::Line);
    c.series = vec![Series {
        name: Some("=Nope!A1".into()),
        categories: Some("((".into()),
        values: "Missing!A1:A3".into(),
        bubble_sizes: Some("".into()),
        color: None,
        secondary: false,
        kind: None,
        quartiles: Default::default(),
        smooth: false,
        trendlines: Vec::new(),
    }];
    let d = resolve(&wb, 7, &c);
    assert_eq!(d.series.len(), 1);
    let _ = render(&c, &d, 300.0, 200.0, &ApproxMeasure);
}

#[test]
fn palette_beyond_six_series() {
    let c = chart(ChartKind::ColumnClustered);
    let p = default_palette(&c, 14);
    assert_eq!(p.len(), 14);
    assert_ne!(p[0], p[6]);
    assert_ne!(p[6], p[12]);
}

/// The style contract, end to end: a preset ID must reach the primitives — the card, the ink on
/// it and the series ramp — so all four backends, which only ever read `Prim`s, follow it.
#[test]
fn style_preset_reaches_the_primitives() {
    let wb = Workbook::new();
    let mut c = chart(ChartKind::ColumnClustered);
    c.series = vec![Series {
        name: Some("S1".into()),
        categories: None,
        values: "{5,3,4}".into(),
        bubble_sizes: None,
        color: None,
        secondary: false,
        kind: None,
        quartiles: Default::default(),
        smooth: false,
        trendlines: Vec::new(),
    }];
    let mut fills = |style: u32| -> (Rgba, Rgba, Vec<Rgba>) {
        c.style = style;
        let d = resolve(&wb, 0, &c);
        let prims = render(&c, &d, 480.0, 288.0, &ApproxMeasure);
        let card = match prims.first() {
            Some(Prim::Rect { fill: Some(f), .. }) => f.start(),
            other => panic!("prims[0] should be the card, got {other:?}"),
        };
        let ink = prims
            .iter()
            .find_map(|p| match p {
                Prim::Text { color, .. } => Some(*color),
                _ => None,
            })
            .expect("the chart draws a title");
        (card, ink, d.palette.clone())
    };

    // The default is the light card the chart has always drawn, unchanged.
    let (card0, ink0, _) = fills(0);
    assert_eq!(card0, [0xFF, 0xFF, 0xFF, 0xFF]);
    assert_eq!(card0, chart_style(0).card);
    assert_eq!(ink0, chart_style(0).text);
    assert_eq!(fills(1), fills(0), "1 is the same polished default as 0");

    // The dark preset paints its own surface with light ink.
    let (card_dark, ink_dark, _) = fills(4);
    assert_eq!(card_dark, chart_style(4).card);
    assert_ne!(card_dark, card0);
    assert_eq!(ink_dark, chart_style(4).text);
    assert!(card_dark[0] < 0x60 && ink_dark[0] > 0xC0, "dark card needs light ink: {card_dark:?} / {ink_dark:?}");

    // Every preset must actually own a different ramp, or the style ID means nothing.
    let ramps: Vec<Vec<Rgba>> = [0u32, 2, 3, 4, 5, 6].iter().map(|s| fills(*s).2).collect();
    for (a, ra) in ramps.iter().enumerate() {
        for (b, rb) in ramps.iter().enumerate().skip(a + 1) {
            assert_ne!(ra, rb, "presets {a} and {b} resolve to the same ramp");
        }
    }

    // An out-of-range ID falls back to the default instead of panicking.
    assert_eq!(fills(49), fills(0));
    assert_eq!(fills(u32::MAX), fills(0));
}

#[test]
fn scatter_with_text_x_values_numbers_the_points() {
    let mut wb = Workbook::new();
    let s = wb.sheet_mut(0).unwrap();
    for (i, (label, v)) in [("North", 3.0), ("South", 5.0), ("East", 4.0)].iter().enumerate() {
        s.set_value(CellRef::new(i as u32, 0), Value::from(*label));
        s.set_value(CellRef::new(i as u32, 1), Value::Number(*v));
        s.set_value(CellRef::new(i as u32, 2), Value::Number(i as f64 * 10.0));
    }
    for kind in [ChartKind::Scatter, ChartKind::ScatterLines, ChartKind::Bubble] {
        let mut c = chart(kind);
        c.series = vec![Series { categories: Some("Sheet1!$A$1:$A$3".into()), values: "Sheet1!$B$1:$B$3".into(), ..Default::default() }];
        let d = resolve(&wb, 0, &c);
        assert_eq!(d.series[0].x, None, "{kind:?}");
        let prims = render(&c, &d, 400.0, 300.0, &ApproxMeasure);
        let empty =
            render(&c, &ChartData { category_levels: vec![], palette: vec![], categories: vec![], series: vec![] }, 400.0, 300.0, &ApproxMeasure);
        assert!(prims.len() > empty.len() + 2, "{kind:?} draws its points");
        // Numeric X values are kept.
        c.series[0].categories = Some("Sheet1!$C$1:$C$3".into());
        assert_eq!(resolve(&wb, 0, &c).series[0].x, Some(vec![Some(0.0), Some(10.0), Some(20.0)]));
    }
}

fn pixel(img: &[u8], w: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * w + x) * 4) as usize;
    [img[i], img[i + 1], img[i + 2], img[i + 3]]
}

#[test]
fn rasterize_bar_center_has_bar_color() {
    let mut c = chart(ChartKind::ColumnClustered);
    c.data_labels = false;
    let d = sample(ChartKind::ColumnClustered);
    let (w, h) = (640u32, 400u32);
    let prims = render(&c, &d, w as f32, h as f32, &ApproxMeasure);
    let img = rasterize(&prims, w, h, [255, 255, 255, 255]);
    assert_eq!(img.len(), (w * h * 4) as usize);
    // Find the bars of the East series (tallest) and check their centres.
    let east = d.series[1].color;
    let bars: Vec<_> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { x, y, w, h, fill: Some(f), .. } if f.start() == east && *w > 10.0 => Some((*x, *y, *w, *h)),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 6);
    for (x, y, bw, bh) in bars {
        let px = pixel(&img, w, (x + bw / 2.0) as u32, (y + bh / 2.0) as u32);
        // A bar is a two-stop sheen since plan 02-01: the ramp's `from` stop is the series colour
        // at the baseline and its `to` stop keeps that hue at 75% alpha, so the bar centre
        // composites the series colour lightened towards the white background.
        assert!(px[0] >= east[0] && px[1] >= east[1] && px[2] >= east[2], "the bar centre keeps the series hue: {px:?}");
        assert_eq!(px[3], 255);
    }
    // Primitive-level checks.
    let img = rasterize(&[Prim::Circle { cx: 10.0, cy: 10.0, r: 5.0, fill: Some([255, 0, 0, 255]), stroke: None }], 20, 20, [0, 0, 0, 0]);
    assert_eq!(pixel(&img, 20, 10, 10), [255, 0, 0, 255]);
    assert_eq!(pixel(&img, 20, 0, 0)[3], 0);
    let wedge = Prim::Wedge {
        cx: 50.0,
        cy: 50.0,
        r_outer: 40.0,
        r_inner: 20.0,
        a0: 0.0,
        a1: std::f32::consts::FRAC_PI_2,
        fill: Some(Fill::solid([0, 0, 255, 255])),
        stroke: None,
    };
    let img = rasterize(&[wedge], 100, 100, [255, 255, 255, 255]);
    assert_eq!(pixel(&img, 100, 79, 30), [0, 0, 255, 255]); // upper-right quadrant ring
    assert_eq!(pixel(&img, 100, 50, 50), [255, 255, 255, 255]); // hole
    assert_eq!(pixel(&img, 100, 25, 70), [255, 255, 255, 255]); // other quadrant
    assert!(rasterize(&[], 0, 10, [0; 4]).is_empty());
    assert!(rasterize(&[], 100_000, 100_000, [0; 4]).is_empty());
}

#[test]
fn rasterize_does_not_panic_on_negative_rect_extents() {
    let red = [255, 0, 0, 255];
    for (w, h) in [(-10.0, 10.0), (10.0, -10.0), (-10.0, -10.0), (f32::NAN, f32::NAN)] {
        let rect = Prim::Rect { x: 5.0, y: 5.0, w, h, fill: Some(Fill::solid(red)), stroke: Some((red, 2.0)), radii: [0.0; 4] };
        let img = rasterize(&[rect], 20, 20, [0, 0, 0, 0]);
        assert_eq!(img.len(), 20 * 20 * 4);
    }
}

#[test]
fn pie_starts_at_twelve_clockwise() {
    let c = chart(ChartKind::Pie);
    let d = sample(ChartKind::Pie);
    let prims = render(&c, &d, 400.0, 400.0, &ApproxMeasure);
    let wedges: Vec<_> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { a0, a1, r_inner, stroke, .. } => Some((*a0, *a1, *r_inner, *stroke)),
            _ => None,
        })
        .collect();
    assert_eq!(wedges.len(), 6);
    assert_eq!(wedges[0].0, 0.0);
    assert!(wedges.windows(2).all(|w| w[1].0 >= w[0].1 - 1e-4));
    assert!((wedges[5].1 - std::f32::consts::TAU).abs() < 1e-3);
    assert!(wedges.iter().all(|w| w.2 == 0.0 && w.3.map(|s| s.0) == Some([255; 4])));
    let mut c = chart(ChartKind::Doughnut);
    c.hole_size = 50;
    let mut d = sample(ChartKind::Doughnut);
    d.series.truncate(1);
    let prims = render(&c, &d, 400.0, 400.0, &ApproxMeasure);
    let ok = prims.iter().any(|p| matches!(p, Prim::Wedge { r_outer, r_inner, .. } if (r_inner / r_outer - 0.5).abs() < 1e-3));
    assert!(ok);
}

#[test]
fn negative_values_put_axis_below_zero() {
    let c = chart(ChartKind::ColumnClustered);
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec![],
        series: vec![series("N", &[-5.0, 3.0, -2.0], [1, 2, 3, 255], ChartKind::ColumnClustered)],
    };
    let prims = render(&c, &d, 400.0, 300.0, &ApproxMeasure);
    let t = texts(&prims);
    assert!(t.iter().any(|s| s.starts_with('-') || s.starts_with("($") || s.starts_with("-$")), "{t:?}");
    assert!(t.contains(&"1") && t.contains(&"3"), "default categories: {t:?}");
}

#[test]
fn edge_cases_never_panic() {
    let m = ApproxMeasure;
    let empty = ChartData::default();
    let zeros = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec!["a".into(), "b".into()],
        series: vec![series("z", &[0.0, 0.0], [0, 0, 0, 255], ChartKind::Line)],
    };
    let weird = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec![String::new(); 3],
        series: vec![SeriesData {
            name: "w".repeat(500),
            values: vec![Some(f64::NAN), Some(f64::INFINITY), None, Some(-1e308), Some(1e308)],
            x: Some(vec![Some(f64::NAN)]),
            sizes: Some(vec![Some(-1.0), Some(0.0)]),
            ..SeriesData::default()
        }],
    };
    for kind in ALL_KINDS {
        let c = chart(kind);
        for d in [&empty, &zeros, &weird] {
            for (w, h) in [(0.0, 0.0), (-5.0, 100.0), (f32::NAN, 10.0), (1.0, 1.0), (20.0, 15.0), (640.0, 400.0), (5000.0, 30.0), (1e6, 1e6)] {
                let p = render(&c, d, w, h, &m);
                if !(w > 0.0 && h > 0.0) || w.is_nan() {
                    assert!(p.is_empty());
                } else {
                    assert_sane(&p, w, h);
                }
            }
        }
    }
}

#[test]
fn thousands_of_points_are_decimated() {
    let n = 50_000;
    let vals: Vec<f64> = (0..n).map(|i| ((i as f64) * 0.01).sin() * 100.0 + i as f64 * 0.001).collect();
    let mut c = chart(ChartKind::Line);
    c.data_labels = true;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec![],
        series: vec![series("big", &vals, [9, 9, 9, 255], ChartKind::Line)],
    };
    let prims = render(&c, &d, 800.0, 400.0, &ApproxMeasure);
    let max_line = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Line { pts, .. } => Some(pts.len()),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    assert!(max_line > 10 && max_line <= 4 * 2000, "{max_line}");
    assert!(prims.len() < 10_000);
    for kind in [ChartKind::Scatter, ChartKind::Histogram, ChartKind::AreaStacked, ChartKind::BoxWhisker] {
        let p = render(&chart(kind), &d, 800.0, 400.0, &ApproxMeasure);
        assert_sane(&p, 800.0, 400.0);
    }
}

#[test]
fn area_prims_stay_inside_the_line_budget() {
    let n = 3_000;
    let mk = |name: &str, off: f64, color: Rgba| {
        let vals: Vec<f64> = (0..n).map(|i| ((i as f64) * 0.02 + off).sin() * 100.0 + 200.0).collect();
        series(name, &vals, color, ChartKind::Area)
    };
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec![],
        series: vec![mk("a", 0.0, [9, 9, 9, 255]), mk("b", 1.0, [99, 99, 99, 255]), mk("c", 2.0, [199, 199, 199, 255])],
    };
    let mut c = chart(ChartKind::Area);
    c.data_labels = false;
    let p = render(&c, &d, 800.0, 400.0, &ApproxMeasure);
    let ramps = p.iter().filter(|q| matches!(q, Prim::Polygon { fill: Some(Fill::Linear { .. }), .. })).count();
    let edges = p.iter().filter(|q| matches!(q, Prim::Line { width, dash: false, .. } if *width == 2.25)).count();
    assert_eq!(ramps, 3, "one gradient fill per series");
    assert_eq!(edges, 3, "one top-edge line per series");
    for q in &p {
        match q {
            // decimate_area's budget is per edge (each edge ≤ MAX_LINE_PTS * 2 = 4,000 columns),
            // so a whole polygon carries at most both edges: 8,000 points.
            Prim::Line { pts, .. } => assert!(pts.len() <= 4_000, "a Line beyond the budget: {}", pts.len()),
            Prim::Polygon { pts, .. } => assert!(pts.len() <= 8_000, "an area polygon beyond the per-edge budget: {}", pts.len()),
            _ => {}
        }
        assert!(coords(q).iter().all(|v| v.is_finite()), "no non-finite coordinate in {q:?}");
    }
}

#[test]
fn smooth_line_adds_interpolated_points_and_keeps_endpoints() {
    let m = ApproxMeasure;
    let cats = ["a", "b", "c", "d"].map(String::from).to_vec();
    let line_pts = |smooth: bool, m: &ApproxMeasure| -> usize {
        let mut s = series("L", &[1.0, 5.0, 2.0, 6.0], [10, 20, 30, 255], ChartKind::Line);
        s.smooth = smooth;
        let d = ChartData { category_levels: vec![], palette: vec![], categories: cats.clone(), series: vec![s] };
        render(&chart(ChartKind::Line), &d, 600.0, 400.0, m)
            .iter()
            .filter_map(|p| match p {
                Prim::Line { pts, .. } => Some(pts.len()),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    };
    let straight = line_pts(false, &m);
    let curved = line_pts(true, &m);
    assert_eq!(straight, 4, "straight polyline keeps one point per datum");
    assert!(curved > straight, "smooth line interpolates extra points: {curved} vs {straight}");
    // Rendering either way must stay finite and inside the canvas.
    let mut s = series("L", &[1.0, 5.0, 2.0, 6.0], [10, 20, 30, 255], ChartKind::Line);
    s.smooth = true;
    let d = ChartData { category_levels: vec![], palette: vec![], categories: cats, series: vec![s] };
    assert_sane(&render(&chart(ChartKind::Line), &d, 600.0, 400.0, &m), 600.0, 400.0);
}

#[test]
fn smooth_line_splits_at_gaps() {
    let m = ApproxMeasure;
    let mut s = series("L", &[1.0, 5.0, 2.0, 6.0], [10, 20, 30, 255], ChartKind::Line);
    s.smooth = true;
    s.values[1] = None; // gap between two data points
    let d = ChartData { category_levels: vec![], palette: vec![], categories: vec!["a".into(), "b".into(), "c".into(), "d".into()], series: vec![s] };
    let prims = render(&chart(ChartKind::Line), &d, 600.0, 400.0, &m);
    // Two separate runs (before/after the gap) → at least two line primitives.
    let lines = prims.iter().filter(|p| matches!(p, Prim::Line { .. })).count();
    assert!(lines >= 2, "a gap must split the smoothed line into runs, got {lines}");
    assert_sane(&prims, 600.0, 400.0);
}

/// Small deterministic PRNG (xorshift) for the fuzz test.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f(&mut self) -> f64 {
        (self.next() % 1_000_000) as f64 / 1_000_000.0
    }
}

#[test]
fn fuzz_random_data_and_sizes() {
    let mut r = Rng(0x9E37_79B9_7F4A_7C15);
    let m = ApproxMeasure;
    for iter in 0..400 {
        let kind = ALL_KINDS[(r.next() % ALL_KINDS.len() as u64) as usize];
        let nser = (r.next() % 6) as usize;
        let npts = (r.next() % 40) as usize;
        let mag = 10f64.powi((r.next() % 30) as i32 - 10);
        let val = |r: &mut Rng| -> Option<f64> {
            match r.next() % 12 {
                0 => None,
                1 => Some(f64::NAN),
                2 => Some(0.0),
                3 => Some(-r.f() * mag),
                _ => Some(r.f() * mag),
            }
        };
        let series: Vec<SeriesData> = (0..nser)
            .map(|i| SeriesData {
                name: format!("S{i}"),
                values: (0..npts).map(|_| val(&mut r)).collect(),
                x: if r.next().is_multiple_of(2) { Some((0..npts).map(|_| val(&mut r)).collect()) } else { None },
                sizes: if r.next().is_multiple_of(2) { Some((0..npts).map(|_| val(&mut r)).collect()) } else { None },
                color: [(r.next() % 256) as u8, 100, 50, 255],
                kind: ALL_KINDS[(r.next() % ALL_KINDS.len() as u64) as usize],
                secondary: r.next().is_multiple_of(3),
                smooth: r.next().is_multiple_of(4),
                trendlines: (0..r.next() % 3)
                    .map(|_| TrendlineData {
                        spec: Trendline {
                            kind: [
                                TrendlineKind::Linear,
                                TrendlineKind::Exponential,
                                TrendlineKind::Logarithmic,
                                TrendlineKind::Polynomial((r.next() % 8) as u8),
                                TrendlineKind::Power,
                                TrendlineKind::MovingAverage((r.next() % 6) as u32),
                            ][(r.next() % 6) as usize],
                            forward: [0.0, 1.5, -2.0, f64::NAN, 1e300][(r.next() % 5) as usize],
                            backward: [0.0, 0.5, f64::INFINITY][(r.next() % 3) as usize],
                            intercept: [None, Some(0.0), Some(-5.0), Some(f64::NAN)][(r.next() % 4) as usize],
                            display_equation: r.next().is_multiple_of(2),
                            display_r_squared: r.next().is_multiple_of(2),
                            width: [None, Some(0.0), Some(f32::NAN), Some(3.0)][(r.next() % 4) as usize],
                            ..Trendline::default()
                        },
                        color: [0, 0, 0, 255],
                    })
                    .collect(),
                number_format: ["General", "$#,##0", "0.00%", "yyyy-mm-dd", "#,##0.0;[Red]-#,##0.0", "@"][(r.next() % 6) as usize].into(),
            })
            .collect();
        let ncat = (r.next() % 50) as usize;
        let d = ChartData {
            category_levels: vec![],
            palette: vec![],
            categories: (0..ncat).map(|i| "c".repeat(i % 7) + &i.to_string()).collect(),
            series,
        };
        let mut c = chart(kind);
        c.legend = [LegendPos::None, LegendPos::Bottom, LegendPos::Top, LegendPos::Left, LegendPos::Right][(r.next() % 5) as usize];
        c.data_labels = r.next().is_multiple_of(2);
        c.gridlines = r.next().is_multiple_of(2);
        if r.next().is_multiple_of(3) {
            c.title = None;
        }
        let w = [0.0, -3.0, 1.0, 12.0, 80.0, 320.0, 1200.0][(r.next() % 7) as usize] as f32 + r.f() as f32 * 10.0;
        let h = [0.0, 2.0, 30.0, 200.0, 900.0][(r.next() % 5) as usize] as f32 + r.f() as f32 * 10.0;
        let p = render(&c, &d, w, h, &m);
        if w >= 1.0 && h >= 1.0 {
            assert_sane(&p, w, h);
        }
        if iter % 20 == 0 && w >= 1.0 && h >= 1.0 && w < 2000.0 && h < 2000.0 {
            let img = rasterize(&p, w as u32, h as u32, [255; 4]);
            assert_eq!(img.len(), (w as u32 * h as u32 * 4) as usize);
        }
        let vals: Vec<Option<f64>> = (0..npts).map(|_| val(&mut r)).collect();
        for k in [SparklineKind::Line, SparklineKind::Column, SparklineKind::WinLoss] {
            let sp = render_sparkline(k, &vals, [0, 0, 0, 255], r.next().is_multiple_of(2), w, h);
            if w >= 1.0 && h >= 1.0 {
                assert_sane(&sp, w.max(1.0), h.max(1.0));
            }
        }
    }
}

#[test]
fn sparklines() {
    let v = [Some(1.0), Some(-2.0), None, Some(4.0), Some(0.0), Some(3.0)];
    let line = render_sparkline(SparklineKind::Line, &v, [1, 2, 3, 255], true, 60.0, 16.0);
    assert_eq!(line.iter().filter(|p| matches!(p, Prim::Line { .. })).count(), 2);
    assert_eq!(line.iter().filter(|p| matches!(p, Prim::Circle { .. })).count(), 5);
    assert_sane(&line, 60.0, 16.0);
    let col = render_sparkline(SparklineKind::Column, &v, [1, 2, 3, 255], false, 60.0, 16.0);
    assert_eq!(col.len(), 5);
    assert_sane(&col, 60.0, 16.0);
    let wl = render_sparkline(SparklineKind::WinLoss, &v, [1, 2, 3, 255], false, 60.0, 16.0);
    assert_eq!(wl.len(), 4);
    assert!(render_sparkline(SparklineKind::Line, &v, [0; 4], false, 0.0, 10.0).is_empty());
    assert!(render_sparkline(SparklineKind::Line, &[], [0; 4], false, 10.0, 10.0).is_empty());
}

// ---------------------------------------------------------------- trendlines

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + b.abs())
}

fn fit(kind: TrendlineKind, pts: &[(f64, f64)]) -> crate::trend::TrendFit {
    crate::trend::fit_trendline(&Trendline { kind, ..Trendline::default() }, pts).expect("fits")
}

#[test]
fn linear_trendline_matches_least_squares() {
    // Slope 0.6, intercept 2.2, R² = 1 - 2.4 / 6.
    let f = fit(TrendlineKind::Linear, &[(1.0, 2.0), (2.0, 4.0), (3.0, 5.0), (4.0, 4.0), (5.0, 5.0)]);
    assert!(close(f.coef[0], 2.2) && close(f.coef[1], 0.6), "{:?}", f.coef);
    assert!(close(f.r2.unwrap(), 0.6));
    assert_eq!(f.equation(), "y = 0.6x + 2.2");
    assert_eq!(f.r_squared(), "R² = 0.6");
    assert!(close(f.eval(10.0).unwrap(), 8.2));
    let down = fit(TrendlineKind::Linear, &[(0.0, 1.0), (1.0, -1.0), (2.0, -3.0)]);
    assert_eq!(down.equation(), "y = -2x + 1");
}

#[test]
fn set_intercept_fits_through_it() {
    // Through the origin: slope Σxy / Σx² = 30 / 14; R² against the uncentred total.
    let pts = [(1.0, 2.0), (2.0, 5.0), (3.0, 6.0)];
    let f = crate::trend::fit_trendline(&Trendline { intercept: Some(0.0), ..Trendline::default() }, &pts).unwrap();
    assert!(close(f.coef[0], 0.0) && close(f.coef[1], 15.0 / 7.0), "{:?}", f.coef);
    let ss_res: f64 = pts.iter().map(|(x, y)| (y - 15.0 / 7.0 * x).powi(2)).sum();
    assert!(close(f.r2.unwrap(), 1.0 - ss_res / 65.0));
    let q = crate::trend::fit_trendline(
        &Trendline { kind: TrendlineKind::Polynomial(2), intercept: Some(2.0), ..Trendline::default() },
        &[(1.0, 0.0), (2.0, 0.0), (3.0, 2.0), (4.0, 6.0)],
    )
    .unwrap();
    assert!(close(q.coef[0], 2.0) && close(q.coef[1], -3.0) && close(q.coef[2], 1.0), "{:?}", q.coef);
}

#[test]
fn transformed_trendlines_recover_their_curves() {
    let xs: [f64; 6] = [0.5, 1.0, 2.0, 3.0, 4.5, 6.0];
    let e = fit(TrendlineKind::Exponential, &xs.map(|x| (x, 3.0 * (0.5 * x).exp())));
    assert!(close(e.coef[0].exp(), 3.0) && close(e.coef[1], 0.5));
    assert_eq!(e.equation(), "y = 3e^(0.5x)");
    let l = fit(TrendlineKind::Logarithmic, &xs.map(|x| (x, 2.0 * x.ln() + 1.0)));
    assert!(close(l.coef[1], 2.0) && close(l.coef[0], 1.0));
    assert_eq!(l.equation(), "y = 2ln(x) + 1");
    let p = fit(TrendlineKind::Power, &xs.map(|x| (x, 4.0 * x.powf(1.5))));
    assert!(close(p.coef[0].exp(), 4.0) && close(p.coef[1], 1.5));
    assert_eq!(p.equation(), "y = 4x^1.5");
    assert!(close(p.r2.unwrap(), 1.0));
    assert_eq!(p.eval(-1.0), None, "power is undefined left of zero");
    // Points the transform can't take are skipped, as Excel documents.
    let skip = fit(TrendlineKind::Exponential, &[(1.0, 2.0f64.exp()), (2.0, 0.0), (3.0, -4.0), (4.0, 8.0f64.exp())]);
    assert!(close(skip.coef[1], 2.0) && close(skip.coef[0], 0.0), "{:?}", skip.coef);
    assert!(
        crate::trend::fit_trendline(&Trendline { kind: TrendlineKind::Power, ..Trendline::default() }, &[(-1.0, 1.0), (0.0, 2.0), (3.0, 3.0)])
            .is_none()
    );
}

#[test]
fn polynomial_trendlines_are_stable() {
    let q = fit(TrendlineKind::Polynomial(2), &[0.0, 1.0, 2.0, 3.0, 4.0].map(|x| (x, x * x - 3.0 * x + 2.0)));
    assert!(close(q.coef[0], 2.0) && close(q.coef[1], -3.0) && close(q.coef[2], 1.0), "{:?}", q.coef);
    assert_eq!(q.equation(), "y = x² - 3x + 2");
    // Year-valued x would make the raw normal equations hopelessly ill-conditioned.
    let curve = |x: f64| {
        let t = x - 2000.0;
        0.001 * t.powi(6) - 0.02 * t.powi(5) + 0.1 * t.powi(3) - t + 7.0
    };
    let years: Vec<(f64, f64)> = (2001..=2012).map(|x| (f64::from(x), curve(f64::from(x)))).collect();
    let six = fit(TrendlineKind::Polynomial(6), &years);
    for x in [2001.0, 2006.5, 2012.0, 2014.0] {
        let (got, want) = (six.eval(x).unwrap(), curve(x));
        assert!((got - want).abs() <= 1e-6 * (1.0 + want.abs()), "{x}: {got} vs {want}");
    }
    assert!(six.r2.unwrap() > 0.999_999);
}

#[test]
fn degenerate_trendlines_fit_nothing() {
    let lin = |pts: &[(f64, f64)]| crate::trend::fit_trendline(&Trendline::default(), pts);
    assert!(lin(&[]).is_none());
    assert!(lin(&[(1.0, 1.0)]).is_none());
    assert!(lin(&[(2.0, 1.0), (2.0, 5.0), (2.0, 3.0)]).is_none(), "all x equal is singular");
    assert!(lin(&[(f64::NAN, 1.0), (1.0, f64::INFINITY), (2.0, 3.0)]).is_none());
    assert!(
        crate::trend::fit_trendline(&Trendline { kind: TrendlineKind::Polynomial(3), ..Trendline::default() }, &[(1.0, 1.0), (2.0, 4.0), (3.0, 9.0)])
            .is_none()
    );
    assert!(
        crate::trend::fit_trendline(&Trendline { kind: TrendlineKind::MovingAverage(2), ..Trendline::default() }, &[(1.0, 1.0), (2.0, 2.0)])
            .is_none()
    );
    let flat = lin(&[(1.0, 4.0), (2.0, 4.0), (3.0, 4.0)]).unwrap();
    assert_eq!((flat.equation(), flat.r2), ("y = 4".to_string(), None));
    assert!(lin(&[(0.0, 1e300), (1.0, -1e300), (2.0, 1e300)]).is_none_or(|f| f.coef.iter().all(|c| c.is_finite())));
}

#[test]
fn moving_average_and_labels() {
    let pts = [(1.0, 1.0), (2.0, 2.0), (3.0, 6.0), (4.0, f64::NAN), (5.0, 4.0)];
    assert_eq!(crate::trend::moving_average(&pts, 2), vec![(2.0, 1.5), (3.0, 4.0), (5.0, 5.0)]);
    assert_eq!(crate::trend::moving_average(&pts, 3), vec![(3.0, 3.0), (5.0, 4.0)]);
    assert!(crate::trend::moving_average(&pts, 9).is_empty());
    assert_eq!(crate::trend::num_text(0.000012345), "1.2345E-05");
    assert_eq!(crate::trend::num_text(-0.00001), "-1E-05");
    assert_eq!(crate::trend::num_text(1234.56789), "1234.5679");
    assert_eq!(crate::trend::auto_name(TrendlineKind::MovingAverage(3), "Sales"), "3 per. Mov. Avg. (Sales)");
    assert_eq!(crate::trend::auto_name(TrendlineKind::Exponential, "Sales"), "Expon. (Sales)");
}

fn lines_of(prims: &[Prim], color: Rgba) -> Vec<&Vec<[f32; 2]>> {
    prims
        .iter()
        .filter_map(|p| match p {
            Prim::Line { pts, color: c, .. } if *c == color => Some(pts),
            _ => None,
        })
        .collect()
}

#[test]
fn line_chart_draws_trendline_forecast_and_labels() {
    let m = ApproxMeasure;
    let tc: Rgba = [1, 2, 3, 255];
    let mut s = series("Sales", &[9.0, 7.0, 5.0, 3.0], [10, 20, 30, 255], ChartKind::Line);
    s.trendlines =
        vec![TrendlineData { spec: Trendline { forward: 1.0, display_equation: true, display_r_squared: true, ..Trendline::default() }, color: tc }];
    let d =
        ChartData { category_levels: vec![], palette: vec![], categories: ["a", "b", "c", "d"].map(String::from).to_vec(), series: vec![s.clone()] };
    let mut c = chart(ChartKind::Line);
    c.legend = LegendPos::None; // keeps legend swatches out of the line counts
    let prims = render(&c, &d, 600.0, 400.0, &m);
    assert_sane(&prims, 600.0, 400.0);
    let data = lines_of(&prims, [10, 20, 30, 255]);
    let trend = lines_of(&prims, tc);
    assert_eq!((data.len(), trend.len()), (1, 1));
    let (d0, dn) = (data[0][0], data[0][3]);
    let (t0, tn) = (trend[0][0], *trend[0].last().unwrap());
    // Exact data: the trendline starts on the first point and runs through the last,
    // then on one more category (the forecast slot).
    assert!((t0[0] - d0[0]).abs() < 0.01 && (t0[1] - d0[1]).abs() < 0.01, "{t0:?} vs {d0:?}");
    let step = (dn[0] - d0[0]) / 3.0;
    assert!((tn[0] - (dn[0] + step)).abs() < 0.01, "forecast end {tn:?}, last point {dn:?}");
    assert!(tn[1] > dn[1], "keeps falling past the data");
    let t = texts(&prims);
    assert!(t.contains(&"y = -2x + 11") && t.contains(&"R² = 1"), "{t:?}");
    assert!(legend_entries(&chart(ChartKind::Line), &d).iter().any(|(n, col, sw)| n == "Linear (Sales)" && *col == tc && *sw == Swatch::Line));
    // Not on chart kinds Excel doesn't offer trendlines on.
    let pie = render(&chart(ChartKind::Pie), &d, 600.0, 400.0, &m);
    assert!(lines_of(&pie, tc).is_empty());
    // One point: nothing to fit, nothing drawn.
    s.values.truncate(1);
    let one = ChartData { category_levels: vec![], palette: vec![], categories: vec!["a".into()], series: vec![s] };
    assert!(lines_of(&render(&c, &one, 600.0, 400.0, &m), tc).is_empty());
}

#[test]
fn scatter_trendlines_use_x_values_and_stay_in_the_plot() {
    let m = ApproxMeasure;
    let xs = [1.0, 2.0, 4.0, 8.0];
    let mut s = series("P", &xs.map(|x| 4.0 * x * x), [10, 20, 30, 255], ChartKind::Scatter);
    s.x = Some(xs.iter().map(|x| Some(*x)).collect());
    let colors: [Rgba; 3] = [[1, 0, 0, 255], [2, 0, 0, 255], [3, 0, 0, 255]];
    s.trendlines = vec![
        TrendlineData {
            spec: Trendline { kind: TrendlineKind::Power, backward: 0.5, display_equation: true, ..Trendline::default() },
            color: colors[0],
        },
        // A steep exponential leaves the plot box and is clipped to it.
        TrendlineData { spec: Trendline { kind: TrendlineKind::Exponential, forward: 30.0, ..Trendline::default() }, color: colors[1] },
        TrendlineData { spec: Trendline { kind: TrendlineKind::MovingAverage(2), ..Trendline::default() }, color: colors[2] },
    ];
    let d = ChartData { category_levels: vec![], palette: vec![], categories: vec![], series: vec![s] };
    let mut c = chart(ChartKind::Scatter);
    c.legend = LegendPos::None;
    let prims = render(&c, &d, 600.0, 400.0, &m);
    assert_sane(&prims, 600.0, 400.0);
    for c in colors {
        assert!(!lines_of(&prims, c).is_empty(), "{c:?} not drawn");
    }
    assert_eq!(lines_of(&prims, colors[2])[0].len(), 3, "moving average: one point per full window");
    assert!(texts(&prims).contains(&"y = 4x^2"));
}

#[test]
fn treemap_and_histogram_shapes() {
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec!["a".into(), "b".into(), "c".into(), "d".into()],
        series: vec![series("t", &[6.0, 6.0, 4.0, 3.0], [10, 20, 30, 255], ChartKind::Treemap)],
    };
    let mut c = chart(ChartKind::Treemap);
    c.title = None;
    c.legend = LegendPos::None;
    let prims = render(&c, &d, 600.0, 400.0, &ApproxMeasure);
    let rects: Vec<f32> = prims
        .iter()
        .skip(1)
        .filter_map(|p| match p {
            Prim::Rect { w, h, .. } => Some(w * h),
            _ => None,
        })
        .collect();
    assert_eq!(rects.len(), 4);
    // Areas proportional to values.
    assert!((rects[0] / rects[3] - 2.0).abs() < 0.05, "{rects:?}");

    let vals: Vec<f64> = (0..100).map(|i| i as f64).collect();
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec![],
        series: vec![series("h", &vals, [10, 20, 30, 255], ChartKind::Histogram)],
    };
    let prims = render(&chart(ChartKind::Histogram), &d, 600.0, 400.0, &ApproxMeasure);
    let bins = prims.iter().filter(|p| matches!(p, Prim::Rect { fill: Some(f), .. } if f.start() == [10, 20, 30, 255])).count();
    assert_eq!(bins, 10);
}

#[test]
fn histogram_boundary_values_match_interval_labels() {
    let heights = |vals: &[f64]| -> Vec<f32> {
        let d = ChartData {
            category_levels: vec![],
            palette: vec![],
            categories: vec![],
            series: vec![series("h", vals, [10, 20, 30, 255], ChartKind::Histogram)],
        };
        let prims = render(&chart(ChartKind::Histogram), &d, 600.0, 400.0, &ApproxMeasure);
        prims
            .iter()
            .filter_map(|p| {
                if let Prim::Rect { h, fill: Some(f), .. } = p
                    && f.start() == [10, 20, 30, 255]
                {
                    Some(*h)
                } else {
                    None
                }
            })
            .collect()
    };
    // Labels are "[0, 7.5]" and "(7.5, 15.0]": 7.5 belongs to the first bar, so the counts are 2 and 1.
    let h = heights(&[0.0, 7.5, 15.0]);
    assert_eq!(h.len(), 2);
    assert!((h[0] / h[1] - 2.0).abs() < 0.01, "{h:?}");
    let h = heights(&[-15.0, -7.5, 0.0]);
    assert!((h[0] / h[1] - 2.0).abs() < 0.01, "{h:?}");
    // A value off the boundary still goes right: [0, 7.5] holds 1, (7.5, 15.0] holds 2.
    let h = heights(&[0.0, 7.75, 15.0]);
    assert!((h[1] / h[0] - 2.0).abs() < 0.01, "{h:?}");
    // Five values, three bins [0,5] (5,10] (10,15]: counts 3, 1, 1.
    let h = heights(&[0.0, 5.0, 5.0, 10.0, 15.0]);
    assert_eq!(h.len(), 3);
    assert!((h[0] / h[1] - 3.0).abs() < 0.01 && (h[1] - h[2]).abs() < 0.01, "{h:?}");
}

#[test]
fn union_series_references() {
    let mut wb = Workbook::new();
    {
        let s = wb.sheet_mut(0).unwrap();
        for (r, (l, v)) in [("a", 1.0), ("b", 2.0), ("Total", 3.0), ("d", 4.0)].iter().enumerate() {
            s.set_value(CellRef::new(r as u32, 0), Value::from(*l));
            s.set_value(CellRef::new(r as u32, 1), Value::Number(*v));
        }
    }
    let mut c = chart(ChartKind::ColumnClustered);
    c.series = vec![Series {
        name: Some("\"Sum of X\"".into()),
        categories: Some("(Sheet1!$A$1:$A$2,Sheet1!$A$4)".into()),
        values: "(Sheet1!$B$1:$B$2,Sheet1!$B$4)".into(),
        ..Series::default()
    }];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.series[0].values, vec![Some(1.0), Some(2.0), Some(4.0)]);
    assert_eq!(d.series[0].name, "Sum of X");
    assert_eq!(d.categories, vec!["a", "b", "d"]);
    // Deeply nested parentheses from a file are bounded, not a crash.
    c.series[0].values = format!("{}Sheet1!$B$1,Sheet1!$B$2{}", "(".repeat(5000), ")".repeat(5000));
    let _ = resolve(&wb, 0, &c);
}

#[test]
fn box_and_stock_charts_honour_the_gridlines_flag() {
    // Horizontal full-width lines in the gridline colour are the value-axis gridlines.
    let grid: Rgba = [217, 217, 217, 255];
    let is_grid = |p: &Prim| match p {
        Prim::Line { color, pts, .. } => *color == grid && pts.len() == 2 && pts[0][1] == pts[1][1] && pts[1][0] - pts[0][0] > 200.0,
        _ => false,
    };
    let grids = |prims: &[Prim]| prims.iter().filter(|p| is_grid(p)).count();
    for kind in [ChartKind::BoxWhisker, ChartKind::Stock, ChartKind::ColumnClustered] {
        let mut d = sample(kind);
        d.series.push(series("Close", &[10000.0, 12000.0, 9000.0, 14000.0, 20000.0, 16000.0], [90, 90, 90, 255], kind));
        let mut c = chart(kind);
        let on = render(&c, &d, 500.0, 350.0, &ApproxMeasure);
        c.gridlines = false;
        let off = render(&c, &d, 500.0, 350.0, &ApproxMeasure);
        assert!(grids(&on) > 0, "{kind:?}: gridlines on must draw lines");
        assert_eq!(grids(&off), 0, "{kind:?}: gridlines off must hide them");
    }
}

#[test]
fn funnel_value_labels_follow_the_data_labels_flag() {
    let data = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec!["a".into(), "b".into(), "c".into()],
        series: vec![series("S", &[11111.0, 22222.0, 33333.0], [20, 96, 130, 255], ChartKind::Funnel)],
    };
    let mut c = chart(ChartKind::Funnel);
    c.data_labels = false;
    let off = render(&c, &data, 500.0, 350.0, &ApproxMeasure);
    c.data_labels = true;
    let on = render(&c, &data, 500.0, 350.0, &ApproxMeasure);
    let values = ["$11,111", "$22,222", "$33,333"];
    for v in values {
        assert!(texts(&on).contains(&v), "{v} missing with labels on");
        assert!(!texts(&off).contains(&v), "{v} drawn with labels off");
    }
    for cat in ["a", "b", "c"] {
        assert!(texts(&off).contains(&cat));
        assert!(texts(&on).contains(&cat));
    }
}

#[test]
fn pie_points_follow_the_chart_preset() {
    let mut wb = Workbook::new();
    wb.theme.colors[4] = 0x4F6D8A;
    wb.theme.colors[5] = 0x8AA1B8;
    let mut c = chart(ChartKind::Pie);
    c.series = vec![Series {
        name: None,
        categories: None,
        values: "{5,3}".into(),
        bubble_sizes: None,
        color: None,
        secondary: false,
        kind: None,
        quartiles: Default::default(),
        smooth: false,
        trendlines: Vec::new(),
    }];
    let d = resolve(&wb, 0, &c);
    // D-01: the chart's preset owns the ramp, so the loud theme accents above must not leak in.
    let ramp = chart_style_preset(c.style);
    let expected = vec![ramp[0], ramp[1]];
    assert_eq!(series_color(&c, 0), ramp[0]);
    assert_ne!(ramp[0], [0x4F, 0x6D, 0x8A, 0xFF]);
    assert_ne!(ramp[1], [0x8A, 0xA1, 0xB8, 0xFF]);
    let legend: Vec<Rgba> = legend_entries(&c, &d).iter().map(|e| e.1).collect();
    assert_eq!(legend, expected);
    let fills: Vec<Rgba> = render(&c, &d, 480.0, 288.0, &ApproxMeasure)
        .iter()
        .filter_map(|p| match p {
            Prim::Wedge { fill, .. } => fill.and_then(|f| match f {
                Fill::Solid(c) => Some(c),
                Fill::Linear { .. } => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(fills, expected);
    for kind in [ChartKind::Doughnut, ChartKind::Treemap, ChartKind::Sunburst] {
        c.kind = kind;
        let legend: Vec<Rgba> = legend_entries(&c, &d).iter().map(|e| e.1).collect();
        assert_eq!(legend, expected, "{kind:?}");
    }
    // Past the 36-colour cycle the points keep following the chart's preset (D-02).
    c.kind = ChartKind::Pie;
    c.series[0].values = format!("{{{}}}", ["1"; 40].join(","));
    let d = resolve(&wb, 0, &c);
    let legend: Vec<Rgba> = legend_entries(&c, &d).iter().map(|e| e.1).collect();
    assert_eq!(legend.len(), 40);
    assert_eq!(legend.get(37), Some(&series_color(&c, 37)));
    // A different workbook theme resolves to the very same colours.
    let mut other = wb.clone();
    other.theme.colors[4] = 0x112233;
    let d2 = resolve(&other, 0, &c);
    let legend2: Vec<Rgba> = legend_entries(&c, &d2).iter().map(|e| e.1).collect();
    assert_eq!(legend2, legend, "the workbook theme must not change chart colours");
}

/// A 200-category pie whose labels cannot fit inside: every one of them steps outside with a
/// three-point leader, the leaders never outnumber `MAX_LABELS`, and none of it moves a slice.
#[test]
fn pie_leaders_are_bounded_and_never_move_a_slice() {
    let n = 200;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: (0..n).map(|j| format!("S{j}")).collect(),
        series: vec![series("Only", &vec![100.0; n], [20, 96, 130, 255], ChartKind::Pie)],
    };
    let mut on = chart(ChartKind::Pie);
    on.title = None;
    on.legend = LegendPos::None;
    let mut off = on.clone();
    off.data_labels = false;
    let p_on = render(&on, &d, 480.0, 288.0, &ApproxMeasure);
    let p_off = render(&off, &d, 480.0, 288.0, &ApproxMeasure);
    let ink = chart_style(0).text;
    let leaders: Vec<&Prim> = p_on
        .iter()
        .filter(|p| matches!(p, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == ink && (*width - 1.0).abs() < 1e-6))
        .collect();
    assert!(!leaders.is_empty() && leaders.len() <= 200, "{} leaders stays within MAX_LABELS", leaders.len());
    for p in &leaders {
        let Prim::Line { pts, .. } = p else { continue };
        assert!(pts.iter().all(|q| q[0].is_finite() && q[1].is_finite()), "leader points stay finite");
    }
    // MAX_WEDGES + 3 * MAX_LABELS + 64: wedges, leaders, labels and the card, with headroom.
    assert!(p_on.len() < 2048 + 3 * 200 + 64, "{} prims stays inside the budget", p_on.len());
    // Labels and leaders never perturb a slice angle or fill: the wedge lists are identical.
    let wedges = |p: &Vec<Prim>| -> Vec<Prim> { p.iter().filter(|q| matches!(q, Prim::Wedge { .. })).cloned().collect() };
    assert_eq!(wedges(&p_on), wedges(&p_off), "the wedge list is identical with labels on and off");
}

/// Past `MAX_LABELS` the gate fails honestly: a 400-category pie draws its wedges but emits no
/// label and no leader at all, rather than silently truncating the set.
#[test]
fn leaders_are_capped_by_max_labels() {
    let n = 400;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: (0..n).map(|j| format!("S{j}")).collect(),
        series: vec![series("Only", &vec![100.0; n], [20, 96, 130, 255], ChartKind::Pie)],
    };
    let mut c = chart(ChartKind::Pie);
    // A single-series chart auto-titles from the series name when `title` is None; suppress it so
    // the count below proves there is no data label and no leader.
    c.title = Some(String::new());
    c.legend = LegendPos::None;
    let p = render(&c, &d, 480.0, 288.0, &ApproxMeasure);
    let wedges = p.iter().filter(|q| matches!(q, Prim::Wedge { .. })).count();
    // A slice that crosses 12 o'clock is drawn as two wedges, so the count can only exceed n.
    assert!(wedges >= 400, "the wedges themselves are all drawn: {wedges}");
    let ink = chart_style(0).text;
    assert!(
        p.iter().all(|q| !matches!(q, Prim::Line { pts, color, width, .. } if pts.len() == 3 && *color == ink && (*width - 1.0).abs() < 1e-6)),
        "no leader past MAX_LABELS"
    );
    assert_eq!(texts(&p).len(), 0, "no label past MAX_LABELS");
}
#[test]
fn area_charts_draw_source_value_labels_only_when_enabled() {
    for kind in [ChartKind::Area, ChartKind::AreaStacked, ChartKind::AreaStacked100] {
        let d = sample(kind);
        let mut c = chart(kind);
        c.data_labels = false;
        let off = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
        c.data_labels = true;
        let on = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
        let count = |p: &[Prim], v: &str| texts(p).iter().filter(|t| **t == v).count();
        // One label per point: 3 series x 6 categories.
        assert_eq!(texts(&on).len(), texts(&off).len() + 18, "{kind:?}");
        for v in ["$12,000", "$38,000", "$9,000", "$27,000"] {
            assert_eq!(count(&on, v), count(&off, v) + 1, "{kind:?}: {v}");
        }
        // Stacked areas label the source values, not cumulative totals (12,000 + 38,000).
        assert_eq!(count(&on, "$50,000"), count(&off, "$50,000"), "{kind:?}: cumulative total shown");
        let polys = |p: &[Prim]| p.iter().filter(|p| matches!(p, Prim::Polygon { .. })).cloned().collect::<Vec<_>>();
        assert_eq!(polys(&on), polys(&off));
        // The first series' labels are drawn after every area, so the next area can't cover them.
        let last_poly = on.iter().rposition(|p| matches!(p, Prim::Polygon { .. }));
        let first_label = on.iter().position(|p| matches!(p, Prim::Text { text, .. } if text == "$12,000"));
        assert!(first_label > last_poly, "{kind:?}: label drawn under a later area");
    }
}

#[test]
fn drawing_and_resolving_agree_on_the_combo_plot_kind() {
    // Resolved series carry `effective_kind`; drawing re-derives the kind from it. Both paths
    // must give the same kind for every chart kind, series kind (or none) and series index.
    let series_kinds = std::iter::once(None).chain(ALL_KINDS.iter().map(|k| Some(*k)));
    for ck in ALL_KINDS {
        for sk in series_kinds.clone() {
            for i in 0..3 {
                let mut c = chart(ck);
                c.series = (0..3).map(|j| Series { kind: if j == i { sk } else { None }, ..Default::default() }).collect();
                let from_chart = series_plot_kind(&c, i);
                let resolved = crate::resolve::effective_kind(ck, sk, i);
                assert_eq!(crate::resolve::plot_kind(ck, Some(resolved), i), from_chart, "{ck:?} {sk:?} {i}");
                if ck != ChartKind::Combo {
                    assert_eq!(from_chart, ck);
                } else {
                    assert_ne!(from_chart, ChartKind::Combo);
                }
            }
        }
    }
}

#[test]
fn ticks_of_a_scale_off_the_step_grid_start_at_its_min() {
    let s = Scale { min: 3.0, max: 7.0, step: 4.0 };
    assert_eq!(s.ticks(), vec![3.0, 7.0]);
    // Aligned scales still snap float error away.
    let s = Scale { min: 0.0, max: 1.0, step: 0.1 };
    assert!((s.ticks()[3] - 0.3).abs() < 1e-12 && s.ticks()[0] == 0.0);
}

#[test]
fn hundred_percent_stacked_areas_fill_each_category_to_its_total() {
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["Q1", "Q2", "Q3", "Q4"].iter().map(|s| s.to_string()).collect(),
        series: vec![
            series("North", &[10.0, 30.0, 0.0, 5.0], [20, 96, 130, 255], ChartKind::AreaStacked100),
            series("South", &[30.0, 10.0, 0.0, 15.0], [25, 107, 36, 255], ChartKind::AreaStacked100),
        ],
    };
    let mut c = chart(ChartKind::AreaStacked100);
    c.data_labels = false;
    let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let t = texts(&p);
    assert!(t.contains(&"100%") && t.contains(&"0%"), "{t:?}");
    assert!(!t.contains(&"40"), "{t:?}");
    let polys: Vec<&Vec<[f32; 2]>> = p
        .iter()
        .filter_map(|p| match p {
            Prim::Polygon { pts, .. } => Some(pts),
            _ => None,
        })
        .collect();
    let (Some(north), Some(south)) = (polys.first(), polys.get(1)) else { panic!("two areas expected: {polys:?}") };
    // Each polygon is its top edge left to right, then its base right to left.
    let top = |poly: &[[f32; 2]], j: usize| poly.get(j).map(|p| p[1]).unwrap_or(f32::NAN);
    let base = |poly: &[[f32; 2]], j: usize| poly.get(poly.len() - 1 - j).map(|p| p[1]).unwrap_or(f32::NAN);
    let (y0, y100) = (base(north, 0), top(south, 0));
    assert!(y0 > y100, "{y0} {y100}");
    let frac = |y: f32| (y0 - y) / (y0 - y100);
    for (j, share) in [(0, 0.25), (1, 0.75), (3, 0.25)] {
        assert!((frac(top(north, j)) - share).abs() < 1e-3, "Q{}: North reaches {}", j + 1, frac(top(north, j)));
        assert!((frac(top(south, j)) - 1.0).abs() < 1e-3, "Q{}: South reaches {}", j + 1, frac(top(south, j)));
    }
    // A category whose total is zero stays on the baseline.
    assert!(frac(top(south, 2)).abs() < 1e-3, "Q3: South reaches {}", frac(top(south, 2)));
}

#[test]
fn area_under_fill_ramps_from_the_series_colour_to_the_card() {
    let style = crate::resolve::style::chart_style(0);
    for kind in [ChartKind::Area, ChartKind::AreaStacked] {
        let d = sample(kind);
        let c = chart(kind);
        let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
        // One ramped polygon per series, in series order.
        let fills: Vec<&Prim> = p.iter().filter(|p| matches!(p, Prim::Polygon { fill: Some(Fill::Linear { .. }), .. })).collect();
        assert_eq!(fills.len(), d.series.len(), "{kind:?}: one ramped polygon per series");
        for (i, prim) in fills.iter().enumerate() {
            let s = &d.series[i];
            let Prim::Polygon { pts, fill: Some(Fill::Linear { from, to, start, end }), .. } = prim else {
                panic!("{kind:?}: layer {i} is not a ramp");
            };
            assert_eq!(*from, s.color, "{kind:?}: layer {i} starts at the series colour");
            assert_eq!(*to, crate::render::mix(s.color, style.card, crate::render::AREA_UNDERFILL_FADE), "{kind:?}: layer {i} fades toward the card");
            assert_eq!(start[0], end[0], "{kind:?}: layer {i} ramp is vertical");
            let (x_min, x_max) = (pts.iter().map(|q| q[0]).fold(f32::INFINITY, f32::min), pts.iter().map(|q| q[0]).fold(f32::NEG_INFINITY, f32::max));
            assert!(start[0] > x_min && start[0] < x_max, "{kind:?}: layer {i} ramp sits inside its own x extent");
            assert!(start[1] < end[1], "{kind:?}: layer {i} ramps top to bottom");
            assert_eq!(from[3], 255, "{kind:?}: layer {i} keeps full alpha at the top");
            assert_eq!(to[3], 255, "{kind:?}: layer {i} keeps full alpha at the base");
            // The ramp spans exactly this layer's own band: the top edge's highest point and the
            // base edge's lowest point. The polygon is its top edge forward then its base edge
            // reversed (6 categories: no decimation), so the halves split the point list.
            let half = pts.len() / 2;
            let top_min = pts.iter().take(half).map(|q| q[1]).fold(f32::INFINITY, f32::min);
            let base_max = pts.iter().skip(half).map(|q| q[1]).fold(f32::NEG_INFINITY, f32::max);
            assert!((start[1] - top_min).abs() < 1e-3, "{kind:?}: layer {i} starts at its own top edge: {} {}", start[1], top_min);
            assert!((end[1] - base_max).abs() < 1e-3, "{kind:?}: layer {i} ends at its own base edge: {} {}", end[1], base_max);
        }
    }
    // With strictly positive values every stacked layer sits above the previous one, so each
    // layer's base (and its ramp's end) moves strictly up the plot, layer by layer.
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["Q1", "Q2", "Q3", "Q4"].iter().map(|s| s.to_string()).collect(),
        series: vec![
            series("A", &[10.0, 20.0, 30.0, 40.0], [20, 96, 130, 255], ChartKind::AreaStacked),
            series("B", &[30.0, 10.0, 20.0, 5.0], [200, 80, 40, 255], ChartKind::AreaStacked),
            series("C", &[5.0, 15.0, 10.0, 20.0], [25, 107, 36, 255], ChartKind::AreaStacked),
        ],
    };
    let c = chart(ChartKind::AreaStacked);
    let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let ends: Vec<f32> = p
        .iter()
        .filter_map(|p| match p {
            Prim::Polygon { fill: Some(Fill::Linear { end, .. }), .. } => Some(end[1]),
            _ => None,
        })
        .collect();
    assert_eq!(ends.len(), 3);
    assert!(ends[0] > ends[1] && ends[1] > ends[2], "each layer fades over its own band: {ends:?}");
    // A degenerate layer — a band with no height of its own (a first series of zeros sits flat on
    // the baseline) — stays solid, never a degenerate ramp.
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: ["Q1", "Q2", "Q3", "Q4"].iter().map(|s| s.to_string()).collect(),
        series: vec![series("Z", &[0.0, 0.0, 0.0, 0.0], [25, 107, 36, 255], ChartKind::AreaStacked)],
    };
    let c = chart(ChartKind::AreaStacked);
    let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let solid = p.iter().find(|p| matches!(p, Prim::Polygon { fill: Some(Fill::Solid(s)), .. } if *s == [25, 107, 36, 255]));
    assert!(solid.is_some(), "a zero-height band resolves to a solid fill, not a degenerate ramp");
}

#[test]
fn area_top_edge_traces_its_gradient_at_the_line_weight() {
    let d = sample(ChartKind::Area);
    let mut c = chart(ChartKind::Area);
    c.data_labels = true;
    let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    // The fill polygon's first half is its top edge forward (6 categories: no decimation), so the
    // stroke must be exactly those points, at the line families' pinned weight, right after it.
    let polys: Vec<usize> =
        p.iter().enumerate().filter(|(_, q)| matches!(q, Prim::Polygon { fill: Some(Fill::Linear { .. }), .. })).map(|(k, _)| k).collect();
    assert_eq!(polys.len(), d.series.len(), "one area polygon per series");
    let label_idx: Vec<usize> =
        p.iter().enumerate().filter(|(_, q)| matches!(q, Prim::Text { text, .. } if text.starts_with('$'))).map(|(k, _)| k).collect();
    // Axis ticks share the "$#,##0" format but paint before the series; the area value labels are
    // the batch the drain emits after every stroke, so they are the ones past the last top edge.
    let max_edge = p.iter().rposition(|q| matches!(q, Prim::Line { width, dash: false, .. } if *width == 2.25)).unwrap_or(0);
    let label_idx: Vec<usize> = label_idx.into_iter().filter(|&k| k > max_edge).collect();
    assert!(!label_idx.is_empty(), "value labels are on for the ordering half of the contract");
    let first_label = label_idx.first().copied().unwrap_or(p.len());
    let mut traced = 0;
    for &k in &polys {
        let Prim::Polygon { pts, .. } = &p[k] else { continue };
        let n = pts.len() / 2;
        let top: Vec<[f32; 2]> = pts[..n].to_vec();
        let line = p[k + 1..first_label].iter().find_map(|q| match q {
            Prim::Line { pts, color, width, dash: false } if *width == 2.25 && pts.to_vec() == top => Some(*color),
            _ => None,
        });
        match line {
            Some(color) => {
                assert!(d.series.iter().any(|s| s.color == color), "the edge keeps its series colour");
                traced += 1;
            }
            None => panic!("no 2.25 pt top edge traces polygon at prim {k}"),
        }
    }
    let edges = p.iter().filter(|q| matches!(q, Prim::Line { width, dash: false, .. } if *width == 2.25)).count();
    assert_eq!(edges, traced, "exactly one top edge per series: {edges}");
    assert!(label_idx.iter().all(|&t| t > max_edge), "every area label paints after every top edge");
}

#[test]
fn area_top_edge_breaks_at_gaps_but_the_fill_stays_one_polygon() {
    let mut d = sample(ChartKind::Area);
    d.series.truncate(1);
    d.series[0].values[2] = None; // a gap in the middle
    let mut c = chart(ChartKind::Area);
    c.data_labels = false;
    let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let polys = p.iter().filter(|q| matches!(q, Prim::Polygon { .. })).count();
    assert_eq!(polys, 1, "the fill stays one polygon across the gap");
    let edges: Vec<&Prim> = p.iter().filter(|q| matches!(q, Prim::Line { width, dash: false, .. } if *width == 2.25)).collect();
    assert_eq!(edges.len(), 2, "the stroke breaks at the gap: one run either side, got {}", edges.len());
    for e in &edges {
        let Prim::Line { pts, .. } = e else { continue };
        assert!(pts.len() >= 2, "draw_runs drops single-point runs: {}", pts.len());
        assert!(pts.iter().flatten().all(|v| v.is_finite()), "no non-finite stroke coordinate");
    }
    // A single-category area (the widening path) fills its widened polygon and never strokes: a
    // one-point run is below draw_runs' two-point floor.
    let mut d1 = sample(ChartKind::Area);
    d1.series.truncate(1);
    d1.series[0].values = vec![Some(25000.0)];
    d1.categories.truncate(1);
    let mut c1 = chart(ChartKind::Area);
    c1.data_labels = false;
    let p1 = render(&c1, &d1, 640.0, 400.0, &ApproxMeasure);
    assert!(p1.iter().any(|q| matches!(q, Prim::Polygon { .. })), "the one-category widening still fills");
    assert!(
        !p1.iter().any(|q| matches!(q, Prim::Line { width, dash: false, .. } if *width == 2.25)),
        "a single-point top edge is dropped, not stroked"
    );
}

#[test]
fn the_area_ramp_rasterises_from_the_series_colour_to_the_card() {
    let d = sample(ChartKind::Area);
    let mut c = chart(ChartKind::Area);
    c.data_labels = false;
    let p = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    // The topmost area layer is painted last, so both probes see its own ramp and nothing above.
    let (from, to, pts) = p
        .iter()
        .filter_map(|q| match q {
            Prim::Polygon { pts, fill: Some(Fill::Linear { from, to, .. }), .. } => Some((*from, *to, pts)),
            _ => None,
        })
        .next_back()
        .expect("an area ramp");
    // Probe down this layer's own tallest column: its top edge point is a category centre, so the
    // pixel 4 pt below it is this layer's fill at t≈0, and 4 pt above the baseline is t≈1.
    let half = pts.len() / 2;
    let top_pt = pts[..half].iter().copied().min_by(|a, b| a[1].total_cmp(&b[1])).expect("a top edge point");
    let (px, py) = (top_pt[0], top_pt[1]);
    let base_y = pts[half..].iter().map(|q| q[1]).fold(f32::NEG_INFINITY, f32::max);
    let (w, h) = (640u32, 400u32);
    let img = rasterize(&p, w, h, [255, 255, 255, 255]);
    let top_px = pixel(&img, w, px as u32, (py + 4.0) as u32);
    let base_px = pixel(&img, w, px as u32, (base_y - 4.0) as u32);
    let dist = |a: [u8; 4], b: Rgba| a.iter().zip(b).map(|(x, y)| (*x as i32 - y as i32).abs()).sum::<i32>() as f32;
    assert!(dist(top_px, from) < dist(top_px, to), "near its top edge the layer reads as its series colour: {top_px:?} vs {from:?}/{to:?}");
    assert!(dist(base_px, to) < dist(base_px, from), "near its base the layer reads toward the card mix: {base_px:?} vs {from:?}/{to:?}");
}

#[test]
fn two_level_categories_resolve_and_draw() {
    let years = ["2023", "", "", "2024", "", ""];
    let months = ["Oct", "Nov", "Dec", "Jan", "Feb", "Mar"];
    let mut wb = Workbook::new();
    {
        let s = wb.sheet_mut(0).unwrap();
        s.name = "Sales".into();
        for (i, (y, m)) in years.iter().zip(months).enumerate() {
            // Year beside month beside the values, points down the rows…
            let r = i as u32 + 1;
            if !y.is_empty() {
                s.set_value(CellRef::new(r, 0), Value::from(*y));
                s.set_value(CellRef::new(10, i as u32), Value::from(*y));
            }
            s.set_value(CellRef::new(r, 1), Value::from(m));
            s.set_value(CellRef::new(r, 2), Value::Number(10.0 * (i as f64 + 1.0)));
            // …and the same table in rows, points along the columns.
            s.set_value(CellRef::new(11, i as u32), Value::from(m));
            s.set_value(CellRef::new(12, i as u32), Value::Number(10.0 * (i as f64 + 1.0)));
        }
    }
    let mut c = chart(ChartKind::Line);
    c.series = vec![Series { categories: Some("Sales!$A$2:$B$7".into()), values: "Sales!$C$2:$C$7".into(), ..Default::default() }];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.categories, months);
    assert_eq!(d.category_levels, vec![years.map(String::from).to_vec()]);

    let texts = |prims: &[Prim]| -> Vec<(String, f32, f32)> {
        prims.iter().filter_map(|p| if let Prim::Text { text, x, y, .. } = p { Some((text.clone(), *x, *y)) } else { None }).collect()
    };
    let prims = render(&c, &d, 600.0, 300.0, &ApproxMeasure);
    assert_sane(&prims, 600.0, 300.0);
    let t = texts(&prims);
    let at = |t: &[(String, f32, f32)], s: &str| {
        t.iter().find(|(x, _, _)| x == s).map(|(_, x, y)| (*x, *y)).unwrap_or_else(|| panic!("no {s:?} in {t:?}"))
    };
    // Six evenly spaced month labels across the plot, not twelve slots for a two-column range.
    let xs: Vec<f32> = months.iter().map(|m| at(&t, m).0).collect();
    let step = xs[1] - xs[0];
    assert!(step > 40.0 && xs.windows(2).all(|w| (w[1] - w[0] - step).abs() < 0.5), "{xs:?}");
    // The years sit in a row below the months, each under its own group.
    let (x23, y23) = at(&t, "2023");
    let (x24, y24) = at(&t, "2024");
    assert!(y23 > at(&t, "Oct").1 && (y23 - y24).abs() < 0.01, "{t:?}");
    assert!(x23 < at(&t, "Dec").0 && x24 > at(&t, "Dec").0 && x24 < at(&t, "Mar").0, "{t:?}");

    // Bars: the levels stand in a column left of the labels.
    c.kind = ChartKind::BarClustered;
    let prims = render(&c, &d, 600.0, 300.0, &ApproxMeasure);
    assert_sane(&prims, 600.0, 300.0);
    let t = texts(&prims);
    let (x23, y23) = at(&t, "2023");
    let (x_oct, y_oct) = at(&t, "Oct");
    // Categories run up the axis: the group's name sits left of its labels, between Oct and Dec.
    assert!(x23 < x_oct && y23 < y_oct && y23 > at(&t, "Dec").1, "{t:?}");
    let (x24, y24) = at(&t, "2024");
    assert!((x23 - x24).abs() < 0.01 && y24 < y23, "same column, 2024 above: {t:?}");

    // A row of values: the levels are rows.
    c.series[0].categories = Some("Sales!$A$11:$F$12".into());
    c.series[0].values = "Sales!$A$13:$F$13".into();
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.categories, months);
    assert_eq!(d.category_levels, vec![years.map(String::from).to_vec()]);

    // One column of categories: a plain axis, as before.
    c.series[0].categories = Some("Sales!$B$2:$B$7".into());
    c.series[0].values = "Sales!$C$2:$C$7".into();
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.categories, months);
    assert!(d.category_levels.is_empty());
}

#[test]
fn a_huge_trendline_forecast_does_not_collapse_the_columns() {
    let m = ApproxMeasure;
    let tc: Rgba = [1, 2, 3, 255];
    let col: Rgba = [10, 20, 30, 255];
    let mut s = series("Sales", &[1.0, 3.0, 2.0, 5.0, 4.0], col, ChartKind::ColumnClustered);
    s.trendlines = vec![TrendlineData { spec: Trendline { forward: 1e6, backward: 1e6, ..Trendline::default() }, color: tc }];
    let d = ChartData { palette: vec![], category_levels: vec![], categories: ["a", "b", "c", "d", "e"].map(String::from).to_vec(), series: vec![s] };
    let mut c = chart(ChartKind::ColumnClustered);
    c.legend = LegendPos::None;
    let prims = render(&c, &d, 600.0, 400.0, &m);
    assert_sane(&prims, 600.0, 400.0);
    let widths: Vec<f32> =
        prims.iter().filter_map(|p| if let Prim::Rect { w, fill: Some(f), .. } = p { (f.start() == col).then_some(*w) } else { None }).collect();
    assert_eq!(widths.len(), 5);
    assert!(widths.iter().all(|w| *w > 4.0), "columns stay visible: {widths:?}");
    let trend = lines_of(&prims, tc);
    assert_eq!(trend.len(), 1);
    assert!(trend[0].len() >= 20, "the trendline is sampled over the plot, not the forecast: {}", trend[0].len());
}

#[test]
fn box_whisker_draws_each_series_quartile_method() {
    use gridcraft_model::QuartileMethod;
    let mut c = chart(ChartKind::BoxWhisker);
    c.legend = LegendPos::None;
    c.series = vec![Series::default(), Series { quartiles: QuartileMethod::Inclusive, ..Default::default() }];
    let color = [12, 34, 56, 255];
    let d = ChartData {
        series: vec![series("A", &[1., 2., 3., 4., 5., 6.], color, c.kind), series("B", &[1., 2., 3., 4., 5., 6.], color, c.kind)],
        ..Default::default()
    };
    let prims = render(&c, &d, 640., 400., &ApproxMeasure);
    let heights: Vec<f32> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { h, fill, .. } if fill.as_ref().map(Fill::start) == Some(color) => Some(*h),
            _ => None,
        })
        .collect();
    assert_eq!(heights.len(), 2);
    // The boxes span 3.5 and 2.5 data units on the same value axis.
    assert!((heights[0] / heights[1] - 1.4).abs() < 0.001);
}

#[test]
fn a_series_named_by_a_name_on_a_sheet_with_an_apostrophe_resolves() {
    // #2012: `'Bob''s'!Title` kept the doubled quote, so the sheet wasn't found and the series
    // was named by the literal text.
    let mut wb = Workbook::new();
    {
        let s = wb.sheet_mut(0).unwrap();
        s.name = "Bob's".into();
        s.set_value(CellRef::new(0, 0), Value::from("Takings"));
        s.set_value(CellRef::new(1, 0), Value::Number(4.0));
        s.set_value(CellRef::new(2, 0), Value::Number(9.0));
    }
    wb.names.push(gridcraft_model::DefinedName {
        name: "Title".into(),
        scope: Some(0),
        formula: "'Bob''s'!$A$1".into(),
        comment: String::new(),
        hidden: false,
    });
    let mut c = chart(ChartKind::ColumnClustered);
    c.series = vec![Series {
        name: Some("'Bob''s'!Title".into()),
        categories: None,
        values: "'Bob''s'!$A$2:$A$3".into(),
        bubble_sizes: None,
        color: None,
        secondary: false,
        kind: None,
        quartiles: Default::default(),
        smooth: false,
        trendlines: Vec::new(),
    }];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.series.first().map(|s| s.name.as_str()), Some("Takings"));
}

#[test]
fn a_series_named_by_a_name_on_a_sheet_with_a_bang_resolves() {
    // `'a!b'!Title`: the sheet name holds a `!`, so the evaluator must split the parsed name at
    // the last one, not the first.
    let mut wb = Workbook::new();
    {
        let s = wb.sheet_mut(0).unwrap();
        s.name = "a!b".into();
        s.set_value(CellRef::new(0, 0), Value::from("Takings"));
        s.set_value(CellRef::new(1, 0), Value::Number(4.0));
        s.set_value(CellRef::new(2, 0), Value::Number(9.0));
    }
    wb.names.push(gridcraft_model::DefinedName {
        name: "Title".into(),
        scope: Some(0),
        formula: "'a!b'!$A$1".into(),
        comment: String::new(),
        hidden: false,
    });
    let mut c = chart(ChartKind::ColumnClustered);
    c.series = vec![Series {
        name: Some("'a!b'!Title".into()),
        categories: None,
        values: "'a!b'!$A$2:$A$3".into(),
        bubble_sizes: None,
        color: None,
        secondary: false,
        kind: None,
        quartiles: Default::default(),
        smooth: false,
        trendlines: Vec::new(),
    }];
    let d = resolve(&wb, 0, &c);
    assert_eq!(d.series.first().map(|s| s.name.as_str()), Some("Takings"));
}

#[test]
fn volatile_series_formulas_are_detected() {
    let wb = Workbook::new();
    let with = |values: &str, name: Option<&str>| {
        let mut c = chart(ChartKind::Line);
        c.series.push(Series {
            name: name.map(Into::into),
            categories: None,
            values: values.into(),
            bubble_sizes: None,
            color: None,
            secondary: false,
            kind: None,
            smooth: false,
            quartiles: Default::default(),
            trendlines: vec![],
        });
        c
    };
    assert!(has_volatile_series(&wb, 0, &with("=NOW()", None)));
    assert!(has_volatile_series(&wb, 0, &with("=SUM(A1:A3)*rand ()", None)));
    assert!(has_volatile_series(&wb, 0, &with("Sheet1!$B$2:$B$9", Some("=TODAY()"))));
    assert!(has_volatile_series(&wb, 0, &with("=RANDARRAY(5)", None)));
    assert!(has_volatile_series(&wb, 0, &with("=_xlfn.RANDARRAY(5)", None)));
    assert!(!has_volatile_series(&wb, 0, &with("Sheet1!$B$2:$B$9", Some("Sheet1!$B$1"))));
    assert!(!has_volatile_series(&wb, 0, &with("=MYRAND(A1)", None)));
    assert!(!has_volatile_series(&wb, 0, &with("=Sheet1!$NOW", None)));
    assert!(!has_volatile_series(&wb, 0, &chart(ChartKind::Line)));
}

#[test]
fn volatile_defined_names_are_followed() {
    let with = |values: &str| {
        let mut c = chart(ChartKind::Line);
        c.series.push(Series {
            name: None,
            categories: None,
            values: values.into(),
            bubble_sizes: None,
            color: None,
            secondary: false,
            kind: None,
            smooth: false,
            quartiles: Default::default(),
            trendlines: vec![],
        });
        c
    };
    let name = |name: &str, scope: Option<usize>, formula: &str| gridcraft_model::DefinedName {
        name: name.into(),
        scope,
        formula: formula.into(),
        comment: String::new(),
        hidden: false,
    };
    let mut wb = Workbook::new();
    wb.sheet_mut(0).unwrap().name = "Bob's".into();
    wb.names.push(name("FreshClock", None, "NOW()"));
    wb.names.push(name("Later", None, "FreshClock+1"));
    wb.names.push(name("Dice", Some(0), "RANDBETWEEN(1,6)"));
    wb.names.push(name("Rates", None, "'Bob''s'!$A$1:$A$3"));
    wb.names.push(name("Loop", None, "Loop+1"));
    wb.names.push(name("Spin", None, "Spin+NOW()"));
    wb.names.push(name("Roll", None, "LAMBDA(n,n*RAND())"));
    // A workbook-scoped name, a name used inside another, and a sheet-scoped one, bare or qualified.
    assert!(has_volatile_series(&wb, 0, &with("=FreshClock")));
    assert!(has_volatile_series(&wb, 0, &with("=SUM(Later, 1)")));
    assert!(has_volatile_series(&wb, 0, &with("=Dice*2")));
    assert!(has_volatile_series(&wb, 0, &with("='Bob''s'!Dice")));
    assert!(has_volatile_series(&wb, 0, &with("=Roll(3)")));
    // A self-referencing name ends (it evaluates to an error, not a changing value); one that
    // also calls NOW is volatile.
    assert!(!has_volatile_series(&wb, 0, &with("=Loop")));
    assert!(has_volatile_series(&wb, 0, &with("=Spin")));
    // Many series over the same stable name stay cacheable: each name is followed once.
    let mut many = with("=Rates");
    let first = many.series[0].clone();
    many.series.extend(std::iter::repeat_n(first, 100));
    assert!(!has_volatile_series(&wb, 0, &many));
    // Names nested deeper than the limit are not known to be stable.
    for i in 0..10 {
        wb.names.push(name(&format!("Deep{i}"), None, &format!("Deep{}", i + 1)));
    }
    assert!(has_volatile_series(&wb, 0, &with("=Deep0")));
    // A name that refers to cells, a name inside a string and a name of another scope are stable.
    assert!(!has_volatile_series(&wb, 0, &with("=Rates")));
    assert!(!has_volatile_series(&wb, 0, &with("=\"FreshClock\"")));
    // Dice belongs to sheet 0: seen from another sheet it isn't defined.
    wb.sheets.push(Arc::new(gridcraft_model::Sheet::new("Other")));
    assert!(!has_volatile_series(&wb, 1, &with("=Dice")));
    assert!(has_volatile_series(&wb, 1, &with("=FreshClock")));
}

// ---------------------------------------------------------------- Fill / radii payload (02-01-01)

#[test]
fn payload_is_copy_copy_cheap_and_serde_round_trips() {
    // Prim was 48 bytes before the payload change; a cache line of headroom is the budget.
    assert!(std::mem::size_of::<Prim>() <= 112, "Prim grew to {}", std::mem::size_of::<Prim>());
    // Fill is Copy and needs no heap: one colour, or two stops plus an axis. The payload is 24
    // bytes, so with a discriminant and 4-byte alignment the enum is 28.
    assert!(std::mem::size_of::<Fill>() <= 28, "Fill grew to {}", std::mem::size_of::<Fill>());
    let ramp = Fill::Linear { from: [1, 2, 3, 255], to: [4, 5, 6, 128], start: [0.0, 10.0], end: [0.0, 0.0] };
    let copied = ramp;
    assert_eq!(copied, ramp, "Fill is Copy");
    let json = serde_json::to_string(&ramp).expect("Fill serializes");
    let back: Fill = serde_json::from_str(&json).expect("Fill deserializes");
    assert_eq!(back, ramp, "a ramp survives a round trip: {json}");
}

#[test]
fn a_solid_fill_answers_its_start_colour() {
    let a = [9, 8, 7, 200];
    assert_eq!(Fill::Solid(a).start(), a);
    assert_eq!(Fill::Linear { from: a, to: [1, 2, 3, 4], start: [0.0, 0.0], end: [10.0, 0.0] }.start(), a);
    assert_eq!(Fill::Linear { from: a, to: a, start: [0.0, 0.0], end: [10.0, 0.0] }.start(), a);
}

#[test]
fn the_producer_gives_a_bar_rounded_outward_tips() {
    let prims = render(&chart(ChartKind::ColumnClustered), &sample(ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let up = prims.iter().any(|p| {
        matches!(p, Prim::Rect { radii, .. }
        if radii[0] > 0.0 && radii[1] > 0.0 && radii[2] == 0.0 && radii[3] == 0.0)
    });
    let down = prims.iter().any(|p| {
        matches!(p, Prim::Rect { radii, .. }
        if radii[2] > 0.0 && radii[3] > 0.0 && radii[0] == 0.0 && radii[1] == 0.0)
    });
    assert!(up, "a positive column rounds the two tips away from the baseline");
    assert!(down, "a negative column rounds the two tips below the baseline");
}

#[test]
fn gridlines_are_dashed_and_muted_and_the_baseline_is_solid_and_heavier() {
    let prims = render(&chart(ChartKind::ColumnClustered), &sample(ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let grid = chart_style(0).grid;
    let grid_lines: Vec<&Prim> = prims.iter().filter(|p| matches!(p, Prim::Line { color, .. } if *color == grid.0)).collect();
    assert!(!grid_lines.is_empty(), "default charts must emit gridlines");
    for p in grid_lines {
        match p {
            Prim::Line { dash, width, .. } => {
                assert!(*dash, "gridlines are dashed");
                assert!((*width - grid.1).abs() < 1e-6, "gridline width is the muted token");
            }
            _ => unreachable!("grid line"),
        }
    }
    let base = chart_style(0).baseline;
    let baseline = prims.iter().any(|p| {
        matches!(p, Prim::Line { color, width, dash, pts, .. }
            if *color == base.0 && (*width - base.1).abs() < 1e-6 && !*dash && pts.len() == 2
                && (pts[0][1] - pts[1][1]).abs() < 1e-3)
    });
    assert!(baseline, "a horizontal chart draws its zero/category baseline as a solid, heavier line");
}

#[test]
fn the_baseline_is_drawn_even_with_gridlines_off() {
    let mut c = chart(ChartKind::ColumnClustered);
    c.gridlines = false;
    let prims = render(&c, &sample(ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let grid = chart_style(0).grid.0;
    assert!(!prims.iter().any(|p| matches!(p, Prim::Line { color, .. } if *color == grid)), "gridlines off must hide gridlines");
    let base = chart_style(0).baseline;
    assert!(
        prims.iter().any(|p| matches!(p, Prim::Line { color, width, dash, pts, .. }
            if *color == base.0 && (*width - base.1).abs() < 1e-6 && !*dash && pts.len() == 2
                && (pts[0][1] - pts[1][1]).abs() < 1e-3)),
        "the baseline is still drawn with gridlines off"
    );
}

#[test]
fn radar_gridlines_are_dashed_too() {
    let prims = render(&chart(ChartKind::Radar), &sample(ChartKind::Radar), 640.0, 400.0, &ApproxMeasure);
    let grid = chart_style(0).grid.0;
    let grid_lines: Vec<&Prim> = prims.iter().filter(|p| matches!(p, Prim::Line { color, .. } if *color == grid)).collect();
    assert!(!grid_lines.is_empty(), "a radar chart draws grid rings/spokes");
    for p in grid_lines {
        assert!(matches!(p, Prim::Line { dash: true, .. }), "radar gridlines are dashed");
    }
}

#[test]
fn the_type_ladder_reaches_the_title_axis_titles_ticks_and_the_legend() {
    // Minimal (style 3) scales the whole ladder together: rungs(1080) = 15.1/10.8/9.7.
    let mut c = chart(ChartKind::ColumnClustered);
    c.style = 3;
    let ladder = chart_style(3).type_scale;
    let prims = render(&c, &sample(ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let sizes: Vec<(f32, &str, bool)> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, size, bold, .. } => Some((*size, text.as_str(), *bold)),
            _ => None,
        })
        .collect();
    let title = sizes.iter().find(|(s, t, _)| *s == ladder.title && t.contains("Quarterly Sales"));
    assert!(title.is_some(), "the title sits on the ladder's top rung: {sizes:?}");
    let axis = sizes.iter().filter(|(s, t, _)| *s == ladder.axis_title && (t.contains("Quarter") || t.contains("Revenue")));
    assert_eq!(axis.count(), 2, "both axis titles sit on the middle rung: {sizes:?}");
    let ticks = sizes
        .iter()
        .any(|(s, t, _)| *s == ladder.tick && t.trim().len() <= 4 && !t.contains("Sales") && !t.contains("Quarter") && !t.contains("Revenue"));
    assert!(ticks, "the tick labels sit on the bottom rung: {sizes:?}");
    // The legend entries are the series names, and they share the tick rung (9.7 on Minimal).
    let on_tick: Vec<&str> = sizes.iter().filter(|(s, _, _)| *s == ladder.tick).map(|(_, t, _)| *t).collect();
    for name in ["North", "East", "South"] {
        assert!(on_tick.iter().any(|t| *t == name), "the legend entry {name} sits on the tick rung: {sizes:?}");
    }
}

#[test]
fn a_short_chart_still_shrinks_its_title() {
    // At 320x100 the shrink rule caps the Minimal title (15.1) at 12% of the height: 12.0.
    let mut c = chart(ChartKind::ColumnClustered);
    c.style = 3;
    let prims = render(&c, &sample(ChartKind::ColumnClustered), 320.0, 100.0, &ApproxMeasure);
    let title = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { text, size, .. } if text.contains("Quarterly Sales") => Some(*size),
            _ => None,
        })
        .collect::<Vec<f32>>();
    assert_eq!(title, vec![12.0], "the title shrinks to h*0.12, never above the rung");
    let max = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Text { size, .. } => Some(*size),
            _ => None,
        })
        .fold(0.0_f32, f32::max);
    assert!((max - 12.0).abs() < 1e-6, "nothing outsizes the shrunken title: {max}");
}

// ---------------------------------------------------------------- sanitizer: ramps and radii (02-01-02)

/// A rect at the box origin with the given paint and radii, for the finalize pass.
fn rect_of(fill: Paint, radii: [f32; 4], w: f32, h: f32) -> Prim {
    Prim::Rect { x: 0.0, y: 0.0, w, h, fill, stroke: None, radii }
}

/// The single surviving rect of a one-primitive finalize pass in a `w`x`h` box.
fn only_rect(p: Prim, w: f32, h: f32) -> Prim {
    let mut out = crate::render::finalize(vec![p], w, h);
    assert_eq!(out.len(), 1, "bad payload degrades, only absent geometry is dropped (D-03)");
    out.pop().expect("one primitive")
}

fn radii_of(p: &Prim) -> [f32; 4] {
    match p {
        Prim::Rect { radii, .. } => *radii,
        other => panic!("expected a rect, got {other:?}"),
    }
}

fn fill_of(p: &Prim) -> Paint {
    match p {
        Prim::Rect { fill, .. } => *fill,
        other => panic!("expected a rect, got {other:?}"),
    }
}

#[test]
fn a_zero_length_gradient_degrades_to_its_start_colour() {
    let from = [200, 20, 10, 255];
    let ramp = Fill::Linear { from, to: [10, 20, 200, 128], start: [22.0, 22.0], end: [22.0, 22.0] };
    let out = only_rect(rect_of(Some(ramp), [0.0; 4], 60.0, 60.0), 100.0, 100.0);
    assert_eq!(fill_of(&out), Some(Fill::Solid(from)), "an unresolvable ramp degrades to its start colour");
}

#[test]
fn hostile_radii_become_sharp_corners_not_panics() {
    let out = only_rect(rect_of(None, [f32::NAN, f32::INFINITY, -2.0, 1e30], 40.0, 40.0), 100.0, 100.0);
    assert_eq!(radii_of(&out), [0.0; 4], "a hostile radii array degrades to square corners");
}

#[test]
fn radii_are_clamped_independently_to_half_the_short_side() {
    let wide = only_rect(rect_of(None, [50.0, 3.0, 1.0, 0.0], 100.0, 20.0), 100.0, 20.0);
    assert_eq!(radii_of(&wide), [10.0, 3.0, 1.0, 0.0], "each corner clamps on its own");
    let square = only_rect(rect_of(None, [6.0; 4], 9.0, 9.0), 9.0, 9.0);
    assert_eq!(radii_of(&square), [4.5; 4], "half the short side is the bound");
}

#[test]
fn precision_is_exact_at_the_endpoints_and_midpoint_rounds_deterministically() {
    let ramp = Fill::Linear { from: [7, 9, 11, 200], to: [3, 5, 7, 100], start: [0.0, 0.0], end: [10.0, 10.0] };
    assert_eq!(ramp.start(), [7, 9, 11, 200], "the start stop is exact, not rounded");
    let out = only_rect(rect_of(None, [4.55; 4], 9.10, 9.10), 9.10, 9.10);
    let r = radii_of(&out)[0];
    assert_eq!(r.to_bits(), 4.55f32.to_bits(), "a radius on the bound clamps exactly, got {r}");
}

/// 03-01-02: a `Polygon` and a `Wedge` carry paint and degrade exactly like a `Rect` — a
/// degenerate ramp becomes the solid `from` stop, never a dropped or hollow shape.
#[test]
fn a_polygon_and_a_wedge_carry_paint_and_degrade_like_a_rect() {
    let zero_span = Prim::Polygon {
        pts: vec![[10.0, 10.0], [60.0, 10.0], [60.0, 60.0], [10.0, 60.0]],
        fill: Some(Fill::Linear { from: [10, 20, 30, 255], to: [40, 50, 60, 128], start: [0.0, 0.0], end: [0.0, 0.0] }),
        stroke: None,
    };
    let out = crate::render::finalize(vec![zero_span], 100.0, 100.0);
    assert_eq!(out.len(), 1, "a bad payload degrades, only absent geometry is dropped");
    match &out[0] {
        Prim::Polygon { fill, .. } => {
            assert_eq!(*fill, Some(Fill::Solid([10, 20, 30, 255])), "a zero-span polygon ramp collapses to its start colour");
        }
        other => panic!("expected a polygon, got {other:?}"),
    }

    let nan_axis = Prim::Wedge {
        cx: 50.0,
        cy: 50.0,
        r_outer: 30.0,
        r_inner: 10.0,
        a0: 0.0,
        a1: 1.0,
        fill: Some(Fill::Linear { from: [7, 8, 9, 255], to: [1, 2, 3, 255], start: [f32::NAN, 0.0], end: [1.0, 1.0] }),
        stroke: None,
    };
    let out = crate::render::finalize(vec![nan_axis], 100.0, 100.0);
    assert_eq!(out.len(), 1, "a bad payload degrades, only absent geometry is dropped");
    match &out[0] {
        Prim::Wedge { fill, .. } => {
            assert_eq!(*fill, Some(Fill::Solid([7, 8, 9, 255])), "a non-finite wedge ramp collapses to its start colour");
        }
        other => panic!("expected a wedge, got {other:?}"),
    }

    let square = vec![[10.0, 10.0], [60.0, 10.0], [60.0, 60.0], [10.0, 60.0]];
    let solid = Prim::Polygon { pts: square.clone(), fill: Some(Fill::Solid([1, 2, 3, 255])), stroke: None };
    let out = crate::render::finalize(vec![solid], 100.0, 100.0);
    assert_eq!(
        out,
        vec![Prim::Polygon { pts: square.clone(), fill: Some(Fill::Solid([1, 2, 3, 255])), stroke: None }],
        "a solid polygon round-trips unchanged"
    );

    let outline = Prim::Polygon { pts: square.clone(), fill: None, stroke: Some(([0, 0, 0, 255], 1.0)) };
    let out = crate::render::finalize(vec![outline], 100.0, 100.0);
    match &out[0] {
        Prim::Polygon { fill, .. } => assert!(fill.is_none(), "an unfilled polygon keeps drawing only its stroke"),
        other => panic!("expected a polygon, got {other:?}"),
    }
}

// ---------------------------------------------------------------- bar tips, ramps and markers (02-01-03)

/// One series over one category per value, for the bar-shape tests.
fn solo(values: &[f64], kind: ChartKind) -> ChartData {
    ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: (0..values.len()).map(|i| format!("C{i}")).collect(),
        series: vec![series("S", values, [200, 80, 40, 255], kind)],
    }
}

/// The radii of every rect a chart emitted, in order.
fn rect_radii(prims: &[Prim]) -> Vec<[f32; 4]> {
    prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { radii, .. } => Some(*radii),
            _ => None,
        })
        .collect()
}

#[test]
fn bar_radii_point_outward_for_both_signs() {
    let mut c = chart(ChartKind::ColumnClustered);
    c.data_labels = false;
    let prims = render(&c, &solo(&[3.0, -3.0], ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    let up = radii.iter().find(|r| r[0] > 0.0).copied().expect("a positive column rounds away from the axis");
    assert!(up[0] == up[1] && up[2] == 0.0 && up[3] == 0.0, "top corners only, got {up:?}");
    let down = radii.iter().find(|r| r[2] > 0.0).copied().expect("a negative column rounds below the axis");
    assert!(down[2] == down[3] && down[0] == 0.0 && down[1] == 0.0, "bottom corners only, got {down:?}");

    let mut hb = chart(ChartKind::BarClustered);
    hb.data_labels = false;
    let prims = render(&hb, &solo(&[3.0, -3.0], ChartKind::BarClustered), 640.0, 400.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    let pos = radii.iter().find(|r| r[1] > 0.0).copied().expect("a positive bar rounds at its far end");
    assert!(pos[1] == pos[2] && pos[0] == 0.0 && pos[3] == 0.0, "right corners only, got {pos:?}");
    let neg = radii.iter().find(|r| r[0] > 0.0).copied().expect("a negative bar rounds at its far end");
    assert!(neg[0] == neg[3] && neg[1] == 0.0 && neg[2] == 0.0, "left corners only, got {neg:?}");
}

#[test]
fn a_stacked_middle_segment_stays_square() {
    let mut c = chart(ChartKind::ColumnStacked);
    c.data_labels = false;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec!["C0".into()],
        series: vec![
            series("A", &[10.0], [200, 80, 40, 255], ChartKind::ColumnStacked),
            series("B", &[20.0], [20, 96, 130, 255], ChartKind::ColumnStacked),
            series("C", &[30.0], [25, 107, 36, 255], ChartKind::ColumnStacked),
        ],
    };
    let prims = render(&c, &d, 640.0, 400.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    let rounded = radii.iter().filter(|r| r[0] > 0.0 || r[2] > 0.0).count();
    assert_eq!(rounded, 1, "only the segment at the far end of the stack rounds");
    assert!(radii.iter().filter(|r| r[0] == 0.0 && r[2] == 0.0).all(|r| *r == [0.0; 4]), "an interior segment is square");
}

#[test]
fn hairline_bars_stay_square() {
    let mut c = chart(ChartKind::ColumnClustered);
    c.data_labels = false;
    c.legend = LegendPos::None;
    let values: Vec<f64> = (0..400).map(|i| (i % 7) as f64 + 1.0).collect();
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: (0..values.len()).map(|i| format!("C{i}")).collect(),
        series: vec![series("S", &values, [200, 80, 40, 255], ChartKind::ColumnClustered)],
    };
    let prims = render(&c, &d, 120.0, 300.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    assert!(!radii.is_empty(), "the chart still draws");
    assert!(radii.iter().all(|r| *r == [0.0; 4]), "a merged hairline run has no roundable tip");
}

/// Criterion 1 proof (03-01-07): only the outward terminal of every bar rounds. The producer
/// implements this via `bar_radii`/`bar_radii_at` in `crates/chart/src/render/cartesian.rs`;
/// Phase 4's waterfall, funnel and treemap bars must reuse those helpers.
#[test]
fn criterion_one_rounds_only_the_outward_terminal_of_every_bar() {
    // 1. Clustered columns: positive rounds the top two corners, negative rounds the bottom two.
    let mut col = chart(ChartKind::ColumnClustered);
    col.data_labels = false;
    let prims = render(&col, &solo(&[3.0, -3.0], ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    let up = radii.iter().copied().find(|r| r[0] > 0.0 || r[1] > 0.0).expect("positive column: a rounded rect exists");
    assert!(up[0] == up[1] && up[0] > 0.0 && up[2] == 0.0 && up[3] == 0.0, "positive column: only top corners round, got {up:?}");
    let down = radii.iter().copied().find(|r| r[2] > 0.0 || r[3] > 0.0).expect("negative column: a rounded rect exists");
    assert!(down[2] == down[3] && down[2] > 0.0 && down[0] == 0.0 && down[1] == 0.0, "negative column: only bottom corners round, got {down:?}");

    // 2. Clustered bars: positive rounds the right two corners, negative rounds the left two.
    let mut bar = chart(ChartKind::BarClustered);
    bar.data_labels = false;
    let prims = render(&bar, &solo(&[3.0, -3.0], ChartKind::BarClustered), 640.0, 400.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    let pos = radii.iter().copied().find(|r| r[1] > 0.0 || r[2] > 0.0).expect("positive bar: a rounded rect exists");
    assert!(pos[1] == pos[2] && pos[1] > 0.0 && pos[0] == 0.0 && pos[3] == 0.0, "positive bar: only right corners round, got {pos:?}");
    let neg = radii.iter().copied().find(|r| r[0] > 0.0 || r[3] > 0.0).expect("negative bar: a rounded rect exists");
    assert!(neg[0] == neg[3] && neg[0] > 0.0 && neg[1] == 0.0 && neg[2] == 0.0, "negative bar: only left corners round, got {neg:?}");

    // 3. Mixed-sign stacked columns: each category has one positive terminal on top, one negative
    // terminal on the bottom, and every internal join square.
    let mut stacked = chart(ChartKind::ColumnStacked);
    stacked.data_labels = false;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec!["C0".into(), "C1".into()],
        series: vec![
            series("A", &[10.0, 10.0], [200, 80, 40, 255], ChartKind::ColumnStacked),
            series("B", &[20.0, 20.0], [20, 96, 130, 255], ChartKind::ColumnStacked),
            series("C", &[-15.0, -15.0], [25, 107, 36, 255], ChartKind::ColumnStacked),
        ],
    };
    let prims = render(&stacked, &d, 640.0, 400.0, &ApproxMeasure);
    let mut rects: Vec<(f32, f32, f32, f32, [f32; 4])> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { x, y, w, h, radii, fill: Some(_), .. } if *h > 0.5 && *w > 5.0 => Some((*x, *y, *w, *h, *radii)),
            _ => None,
        })
        .collect();
    rects.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut groups: Vec<Vec<BarRect>> = Vec::new();
    for r in rects {
        let tol = (r.2 * 0.6).max(4.0);
        if let Some(g) = groups.iter_mut().find(|g| (g[0].0 - r.0).abs() <= tol) {
            g.push(r);
        } else {
            groups.push(vec![r]);
        }
    }
    let mut bar_groups: Vec<Vec<BarRect>> = groups.into_iter().filter(|g| g.len() == 3).collect();
    bar_groups.sort_by(|a, b| a[0].0.total_cmp(&b[0].0));
    assert_eq!(bar_groups.len(), 2, "mixed stacked: expected two category columns with 3 segments, got {bar_groups:?}");
    for col_rects in &mut bar_groups {
        col_rects.sort_by(|a, b| a.1.total_cmp(&b.1));
        let top = col_rects.first().copied().expect("top segment");
        let bottom = col_rects.last().copied().expect("bottom segment");
        assert!(
            top.4[0] == top.4[1] && top.4[0] > 0.0 && top.4[2] == 0.0 && top.4[3] == 0.0,
            "mixed stacked column: positive top terminal rounds outward, got {:?}",
            top.4
        );
        assert!(
            bottom.4[2] == bottom.4[3] && bottom.4[2] > 0.0 && bottom.4[0] == 0.0 && bottom.4[1] == 0.0,
            "mixed stacked column: negative bottom terminal rounds outward, got {:?}",
            bottom.4
        );
        for mid in col_rects.iter().skip(1).take(col_rects.len().saturating_sub(2)) {
            assert_eq!(mid.4, [0.0; 4], "mixed stacked column: interior segment stays square, got {:?}", mid.4);
        }
        assert_eq!(
            col_rects.iter().filter(|r| r.4 == [0.0; 4]).count(),
            col_rects.len().saturating_sub(2),
            "mixed stacked column: only the two terminals round"
        );
    }

    // 4. Merged hairline clustered bars stay square (60x60 chart with 12 categories).
    let mut thin = chart(ChartKind::ColumnClustered);
    thin.data_labels = false;
    thin.legend = LegendPos::None;
    let values: Vec<f64> = (0..12).map(|i| (i % 7) as f64 + 1.0).collect();
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: (0..values.len()).map(|i| format!("C{i}")).collect(),
        series: vec![series("S", &values, [200, 80, 40, 255], ChartKind::ColumnClustered)],
    };
    let prims = render(&thin, &d, 60.0, 60.0, &ApproxMeasure);
    let radii = rect_radii(&prims);
    assert!(!radii.is_empty(), "hairline case: chart still draws bars");
    assert!(radii.iter().all(|r| *r == [0.0; 4]), "hairline case: merged hairline bars stay square, got {radii:?}");

    // 5. 100% stacked columns share the same two-terminal invariant.
    let mut stacked100 = chart(ChartKind::ColumnStacked100);
    stacked100.data_labels = false;
    let d = ChartData {
        category_levels: vec![],
        palette: vec![],
        categories: vec!["C0".into(), "C1".into()],
        series: vec![
            series("A", &[10.0, 10.0], [200, 80, 40, 255], ChartKind::ColumnStacked100),
            series("B", &[20.0, 20.0], [20, 96, 130, 255], ChartKind::ColumnStacked100),
            series("C", &[-15.0, -15.0], [25, 107, 36, 255], ChartKind::ColumnStacked100),
        ],
    };
    let prims = render(&stacked100, &d, 640.0, 400.0, &ApproxMeasure);
    let mut rects: Vec<(f32, f32, f32, f32, [f32; 4])> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { x, y, w, h, radii, fill: Some(_), .. } if *h > 0.5 && *w > 5.0 => Some((*x, *y, *w, *h, *radii)),
            _ => None,
        })
        .collect();
    rects.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let mut groups: Vec<Vec<BarRect>> = Vec::new();
    for r in rects {
        let tol = (r.2 * 0.6).max(4.0);
        if let Some(g) = groups.iter_mut().find(|g| (g[0].0 - r.0).abs() <= tol) {
            g.push(r);
        } else {
            groups.push(vec![r]);
        }
    }
    let mut bar_groups: Vec<Vec<BarRect>> = groups.into_iter().filter(|g| g.len() == 3).collect();
    bar_groups.sort_by(|a, b| a[0].0.total_cmp(&b[0].0));
    assert_eq!(bar_groups.len(), 2, "stacked100: expected two category columns with 3 segments, got {bar_groups:?}");
    for col_rects in &mut bar_groups {
        col_rects.sort_by(|a, b| a.1.total_cmp(&b.1));
        let top = col_rects.first().copied().expect("stacked100 top segment");
        let bottom = col_rects.last().copied().expect("stacked100 bottom segment");
        assert!(
            top.4[0] == top.4[1] && top.4[0] > 0.0 && top.4[2] == 0.0 && top.4[3] == 0.0,
            "stacked100 column: top terminal rounds outward, got {:?}",
            top.4
        );
        assert!(
            bottom.4[2] == bottom.4[3] && bottom.4[2] > 0.0 && bottom.4[0] == 0.0 && bottom.4[1] == 0.0,
            "stacked100 column: bottom terminal rounds outward, got {:?}",
            bottom.4
        );
        assert_eq!(
            col_rects.iter().filter(|r| r.4 == [0.0; 4]).count(),
            col_rects.len().saturating_sub(2),
            "stacked100 column: only the two terminals round"
        );
    }
}

#[test]
fn a_bar_ramp_runs_along_the_value_axis() {
    let mut c = chart(ChartKind::ColumnClustered);
    c.data_labels = false;
    let prims = render(&c, &solo(&[3.0, 5.0], ChartKind::ColumnClustered), 640.0, 400.0, &ApproxMeasure);
    let ramps: Vec<&Fill> = prims
        .iter()
        .filter_map(|p| match p {
            Prim::Rect { fill: Some(f @ Fill::Linear { .. }), .. } => Some(f),
            _ => None,
        })
        .collect();
    assert!(!ramps.is_empty(), "bars carry a ramp");
    for f in ramps {
        if let Fill::Linear { from, to, start, end } = f {
            assert_eq!(start[0], end[0], "a column's ramp is vertical");
            assert!(end[1] < start[1], "it runs from the baseline up to the value end");
            assert_eq!(from[3] - to[3], 64, "a 255-alpha series fades by GRADIENT_SHEEN_ALPHA");
        }
    }
}

#[test]
fn markers_cycle_the_four_hulls_by_series_index() {
    use crate::render::shape::{MarkerGeometry, geometry, marker_shape};
    let c = chart(ChartKind::LineMarkers);
    let prims = render(&c, &sample(ChartKind::LineMarkers), 640.0, 400.0, &ApproxMeasure);
    let markers: Vec<&Prim> =
        prims.iter().filter(|p| matches!(p, Prim::Circle { stroke: Some(_), .. } | Prim::Polygon { stroke: Some(_), .. })).collect();
    assert_eq!(markers.len(), 18, "one ringed marker per point across three series");
    let hull_of = |p: &Prim| -> (Vec<[f32; 2]>, f32) {
        match p {
            Prim::Circle { cx, cy, r, .. } => (vec![[*cx, *cy]], *r),
            Prim::Polygon { pts, .. } => {
                let cx = pts.iter().map(|q| q[0]).fold(f32::MIN, f32::max) / 2.0 + pts.iter().map(|q| q[0]).fold(f32::MAX, f32::min) / 2.0;
                let cy = pts.iter().map(|q| q[1]).fold(f32::MIN, f32::max) / 2.0 + pts.iter().map(|q| q[1]).fold(f32::MAX, f32::min) / 2.0;
                let r = pts.iter().map(|q| ((q[0] - cx).powi(2) + (q[1] - cy).powi(2)).sqrt()).fold(0.0f32, f32::max);
                (pts.clone(), r)
            }
            _ => (vec![], 0.0),
        }
    };
    let close = |a: &[f32; 2], b: &[f32; 2]| (a[0] - b[0]).abs() < 1e-3 && (a[1] - b[1]).abs() < 1e-3;
    for (i, p) in markers.iter().enumerate() {
        let series = i / 6;
        match geometry(marker_shape(series), 0.0, 0.0, 3.5) {
            MarkerGeometry::Circle { .. } => {
                let (centers, r) = hull_of(p);
                assert!(matches!(p, Prim::Circle { .. }), "series {series} draws circles, got {p:?}");
                assert!((r - 3.5).abs() < 1e-3, "series {series} circle radius 3.5, got {r}");
                assert_eq!(centers.len(), 1);
            }
            MarkerGeometry::Polygon(want) => {
                let (got, r) = hull_of(p);
                assert_eq!(got.len(), want.len(), "series {series} hull has {} points", want.len());
                assert!((r - 3.5).abs() < 1e-3, "series {series} hull circumradius 3.5, got {r}");
                let cx = got.iter().map(|q| q[0]).fold(f32::MIN, f32::max) / 2.0 + got.iter().map(|q| q[0]).fold(f32::MAX, f32::min) / 2.0;
                let cy = got.iter().map(|q| q[1]).fold(f32::MIN, f32::max) / 2.0 + got.iter().map(|q| q[1]).fold(f32::MAX, f32::min) / 2.0;
                let expect: Vec<[f32; 2]> = want.iter().map(|q| [q[0] + cx, q[1] + cy]).collect();
                assert!(got.iter().zip(&expect).all(|(a, b)| close(a, b)), "series {series} hull {got:?} vs {expect:?}");
            }
        }
    }
    // A fourth series draws the triangle hull, a fifth cycles back to the circle.
    let mut five = sample(ChartKind::LineMarkers);
    five.series.push(series("West", &[5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0], [90, 40, 120, 255], ChartKind::LineMarkers));
    five.series.push(series("North2", &[11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0], [40, 120, 90, 255], ChartKind::LineMarkers));
    let prims = render(&c, &five, 640.0, 400.0, &ApproxMeasure);
    let markers: Vec<&Prim> =
        prims.iter().filter(|p| matches!(p, Prim::Circle { stroke: Some(_), .. } | Prim::Polygon { stroke: Some(_), .. })).collect();
    assert_eq!(markers.len(), 30, "five series of six points");
    assert!(matches!(markers[18 + 5], Prim::Polygon { pts, .. } if pts.len() == 3), "series 3 draws the three-point triangle");
    assert!(matches!(markers[24 + 5], Prim::Circle { .. }), "series 4 cycles back to the circle");
}

#[test]
fn markers_ring_with_the_card_at_the_preset_weight() {
    let style_of = crate::resolve::style::chart_style;
    for style in [0u32, 4] {
        let ring = style_of(style).marker_ring;
        assert_eq!(ring.0, style_of(style).card, "the ring is the card colour on style {style}");
        assert!((0.5..=2.5).contains(&ring.1), "the ring weight stays hairline-thin on style {style}");
        for kind in [ChartKind::LineMarkers, ChartKind::Scatter, ChartKind::Bubble] {
            let mut c = chart(kind);
            c.style = style;
            let prims = render(&c, &sample(kind), 640.0, 400.0, &ApproxMeasure);
            let ringed: Vec<&Prim> =
                prims.iter().filter(|p| matches!(p, Prim::Circle { stroke: Some(_), .. } | Prim::Polygon { stroke: Some(_), .. })).collect();
            assert!(!ringed.is_empty(), "{kind:?} on style {style} draws ringed markers");
            for p in ringed {
                let stroke = match p {
                    Prim::Circle { stroke, .. } | Prim::Polygon { stroke, .. } => *stroke,
                    _ => None,
                };
                assert_eq!(stroke, Some(ring), "{kind:?} on style {style} rings with the card token");
                if kind == ChartKind::Bubble
                    && let Prim::Circle { r, fill, .. } = p
                {
                    assert!(*r > 3.5, "a bubble keeps its value-sized radius, got {r}");
                    assert_eq!(fill.map(|f| f[3]), Some(0xC0), "a bubble keeps its translucent fill");
                }
            }
        }
    }
}

#[test]
fn every_marker_shape_has_a_distinct_hull_and_the_same_ring() {
    use crate::render::shape::{MarkerGeometry, all_shapes, geometry};
    let mut geometries = Vec::new();
    let mut hulls: Vec<Vec<[f32; 2]>> = Vec::new();
    for shape in all_shapes() {
        let g = geometry(shape, 10.0, 20.0, 3.0);
        match &g {
            MarkerGeometry::Circle { r } => assert_eq!(*r, 3.0, "a circle keeps its radius"),
            MarkerGeometry::Polygon(pts) => {
                assert!((3..=4).contains(&pts.len()), "a hull is at most four points: {pts:?}");
                hulls.push(pts.clone());
            }
        }
        geometries.push(g);
    }
    assert_eq!(hulls.len(), 4, "four hulls plus the circle");
    for i in 0..hulls.len() {
        for j in i + 1..hulls.len() {
            assert_ne!(hulls[i], hulls[j], "hulls {i} and {j} are the same shape");
        }
    }
    for i in 0..geometries.len() {
        for j in i + 1..geometries.len() {
            assert_ne!(geometries[i], geometries[j], "shapes {i} and {j} are the same");
        }
    }
}

#[test]
fn legend_swatches_follow_the_series_marker_hull() {
    use crate::render::shape::{MarkerGeometry, geometry, marker_shape};
    let mut c = chart(ChartKind::LineMarkers);
    c.legend = LegendPos::Bottom;
    let prims = render(&c, &sample(ChartKind::LineMarkers), 640.0, 400.0, &ApproxMeasure);
    let swatches: Vec<&Prim> = prims.iter().filter(|p| matches!(p, Prim::Circle { stroke: None, .. } | Prim::Polygon { stroke: None, .. })).collect();
    assert_eq!(swatches.len(), 3, "one swatch per legend entry");
    for (i, p) in swatches.iter().enumerate() {
        let (cx, cy, r) = match p {
            Prim::Circle { cx, cy, r, .. } => (*cx, *cy, *r),
            Prim::Polygon { pts, .. } => {
                let cx = pts.iter().map(|q| q[0]).fold(f32::MIN, f32::max) / 2.0 + pts.iter().map(|q| q[0]).fold(f32::MAX, f32::min) / 2.0;
                let cy = pts.iter().map(|q| q[1]).fold(f32::MIN, f32::max) / 2.0 + pts.iter().map(|q| q[1]).fold(f32::MAX, f32::min) / 2.0;
                let r = pts.iter().map(|q| ((q[0] - cx).powi(2) + (q[1] - cy).powi(2)).sqrt()).fold(0.0f32, f32::max);
                (cx, cy, r)
            }
            _ => continue,
        };
        match geometry(marker_shape(i), cx, cy, r) {
            MarkerGeometry::Circle { .. } => {
                assert!(matches!(p, Prim::Circle { .. }), "entry {i} should be the series hull circle");
            }
            MarkerGeometry::Polygon(want) => {
                let got = match p {
                    Prim::Polygon { pts, .. } => pts.clone(),
                    _ => vec![],
                };
                assert_eq!(got.len(), 4, "entries 1 and 2 are four-point hulls");
                assert_eq!(got, want, "entry {i} is its series' own hull");
            }
        }
    }
}

// ---------------------------------------------------------------- central sanitizer (02-01-04)

#[test]
fn the_fixture_survives_hostile_numbers_without_panicking() {
    let prims = vec![
        Prim::Rect {
            x: 0.0,
            y: 0.0,
            w: 60.0,
            h: 40.0,
            fill: Some(Fill::Linear { from: [10, 20, 30, 255], to: [40, 50, 60, 128], start: [f32::NAN, f32::NAN], end: [60.0, 0.0] }),
            stroke: Some(([1, 2, 3, 255], 1.0)),
            radii: [f32::NAN, f32::INFINITY, -1.0, 1e30],
        },
        Prim::Rect { x: 10.0, y: 10.0, w: 0.0, h: 20.0, fill: Some(Fill::solid([5, 6, 7, 255])), stroke: None, radii: [0.0; 4] },
        Prim::Circle { cx: 20.0, cy: 20.0, r: 0.0, fill: Some([8, 9, 10, 255]), stroke: None },
        Prim::Text {
            x: 5.0,
            y: 5.0,
            text: String::new(),
            size: 10.0,
            color: [0, 0, 0, 255],
            bold: false,
            align: HAlign::Left,
            valign: VAlign::Top,
            rotation: 0.0,
        },
    ];
    let out = crate::render::finalize(prims, 100.0, 80.0);
    assert!(!out.is_empty(), "a shape is never dropped for a hostile fill or radius, only for having no geometry");
    for p in &out {
        if let Prim::Rect { w, h, radii, .. } = p {
            assert!(radii.iter().all(|r| r.is_finite()), "finite radii, got {radii:?}");
            assert!(radii.iter().all(|r| *r <= w.min(*h) / 2.0 + 1e-3), "radii inside the box, got {radii:?} for {w}x{h}");
        }
    }
}

// ------------------------------------------------------------------ the cross-backend fixture

/// PRIM-03: the one in-code fixture reads the same in the pixel backend — exact stop colours at both
/// ends of the ramp, the ring colour and the body colour of every marker hull, and a corner rounded
/// on one side only.
#[test]
fn the_fixture_round_trips_through_the_rasteriser() {
    let (w, h) = (fixture::WIDTH as u32, fixture::HEIGHT as u32);
    let px = rasterize(&parity_fixture(), w, h, fixture::BACKGROUND);
    assert_eq!(px.len(), (w * h * 4) as usize, "a full RGBA buffer");

    for (i, [ring, body]) in fixture::MARKER_PROBES.iter().enumerate() {
        assert_eq!(pixel(&px, w, ring[0], ring[1]), fixture::RING, "marker {i}: the ring band is the ring colour");
        assert_eq!(pixel(&px, w, body[0], body[1]), fixture::MARKER_FILL, "marker {i}: the marker body is its own colour");
    }

    let [from, to] = fixture::GRADIENT_PROBES;
    assert_eq!(pixel(&px, w, from[0], from[1]), fixture::FROM, "the ramp starts at its exact near stop");
    assert_eq!(pixel(&px, w, to[0], to[1]), fixture::TO, "the ramp ends at its exact far stop");

    let [rounded, square] = fixture::ASYMMETRIC_PROBES;
    assert_eq!(pixel(&px, w, rounded[0], rounded[1]), fixture::BACKGROUND, "the rounded corner is cut out, not painted");
    assert_eq!(pixel(&px, w, square[0], square[1]), fixture::ASYMMETRIC_FILL, "the square corner keeps the fill");
}

/// The fixture is the cross-backend probe: a later wave cannot silently re-import this plan's ring
/// tokens into it. Eight prims, the card first, the ring values pinned, five hulls at their centres.
#[test]
fn the_parity_fixture_keeps_its_eight_primitives_and_probe_colours() {
    let scene = parity_fixture();
    assert_eq!(scene.len(), 8, "card, gradient bar, asymmetric rect, five markers");
    assert!(
        matches!(scene.first(), Some(Prim::Rect { x: 0.0, y: 0.0, w: 132.0, h: 80.0, fill: Some(Fill::Solid(c)), radii, .. }) if *c == fixture::CARD && *radii == [0.0; 4]),
        "the card leads the scene"
    );
    assert_eq!(fixture::RING, [255, 255, 255, 255]);
    assert_eq!(fixture::RING_WEIGHT, 2.0);
    assert_eq!(fixture::MARKER_CENTRES, [20.0, 46.0, 72.0, 98.0, 124.0]);
    let centres: Vec<f32> = scene
        .iter()
        .filter_map(|q| match q {
            Prim::Circle { cx, .. } => Some(*cx),
            _ => None,
        })
        .collect();
    assert_eq!(centres, vec![fixture::MARKER_CENTRES[0]], "the circle hull is first of the five, at its pinned centre");
    let ringed = scene
        .iter()
        .filter(|q| matches!(q, Prim::Circle { stroke: Some((c, w)), .. } | Prim::Polygon { stroke: Some((c, w)), .. } if *c == fixture::RING && *w == fixture::RING_WEIGHT))
        .count();
    assert_eq!(ringed, 5, "every hull carries the pinned ring");
}

/// PRIM-03 / D-03: hostile stops, a non-finite axis and impossible radii degrade to a solid start
/// colour and square corners, and the central pass drops nothing that has geometry.
#[test]
fn the_fixture_survives_a_hostile_gradient_and_radii_set() {
    let (w, h) = (fixture::WIDTH as u32, fixture::HEIGHT as u32);
    let near = [200u8, 10, 10, 255];
    let flat = [90u8, 90, 200, 255];
    let mut scene = parity_fixture();
    let kept = scene.len();
    scene.extend([
        Prim::Rect {
            x: 160.0,
            y: 56.0,
            w: 40.0,
            h: 24.0,
            fill: Some(Fill::Linear { from: near, to: [10, 200, 10, 255], start: [f32::NAN; 2], end: [f32::NAN; 2] }),
            stroke: None,
            radii: [0.0; 4],
        },
        Prim::Rect {
            x: 160.0,
            y: 88.0,
            w: 40.0,
            h: 24.0,
            fill: Some(Fill::solid(flat)),
            stroke: Some(([255, 255, 255, 255], 1.0)),
            radii: [f32::NAN, -1.0, f32::INFINITY, 1e30],
        },
        Prim::Rect {
            x: 204.0,
            y: 56.0,
            w: 24.0,
            h: 24.0,
            fill: Some(Fill::Linear { from: [10, 10, 10, 255], to: [250, 250, 250, 255], start: [216.0, 68.0], end: [216.0, 68.0] }),
            stroke: None,
            radii: [0.0; 4],
        },
    ]);

    let finalised = crate::render::finalize(scene, fixture::WIDTH, fixture::HEIGHT);
    assert_eq!(finalised.len(), kept + 3, "only a primitive with no geometry is dropped");

    let px = rasterize(&finalised, w, h, fixture::BACKGROUND);
    assert_eq!(px.len(), (w * h * 4) as usize, "a full buffer, never an empty one");
    assert_eq!(pixel(&px, w, 161, 57), near, "a ramp with a non-finite axis degrades to its near stop");
    assert_eq!(pixel(&px, w, 161, 89), flat, "hostile radii degrade to square corners, not to a hole");
    assert_eq!(pixel(&px, w, 210, 60), [10, 10, 10, 255], "a zero-length ramp degrades to its near stop");
}
