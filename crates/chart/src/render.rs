//! Chart layout: title, legend, axes and every chart kind, drawn into [`Prim`]s.

use std::borrow::Cow;
use std::f32::consts::{PI, TAU};

use gridcraft_model::{Chart, ChartKind, LineDash, QuartileMethod, TrendlineKind};

use crate::axis::{Fmt, Scale, clean, nice_scale};
use crate::resolve::style::{ChartStyle, chart_style};
use crate::resolve::{MAX_POINTS, MAX_SERIES, MAX_TRENDLINES, points_per_series, ramp_color};
use crate::sanitize::{clamp_fill, clamp_radii, clamp_stroke, fin};
use crate::trend::{auto_name, fit_trendline, moving_average};
use crate::{ChartData, HAlign, Measure, Prim, Rgba, SeriesData, VAlign};
pub(crate) use crate::{Fill, Paint};

mod cartesian;
mod frame;
mod kinds;
mod legend;
mod lines;
pub(crate) mod shape;
mod table;
mod trend;

use cartesian::{cartesian, scatter};
use frame::{Bx, axis_titles, chart_title};
use kinds::{box_whisker, doughnut, funnel, histogram, pie, radar, stock, treemap, waterfall};
use legend::draw_legend;
pub use legend::{Swatch, legend_entries};
pub use trend::supports_trendline;

/// Ink readable on a *series fill* (`on`), independent of the card: white on dark fills, near-black
/// on light ones. The card's own ink lives in [`Ctx::style`].
const DARK_TEXT: Rgba = [0x26, 0x26, 0x26, 0xFF];
/// White ink/separator used by fixed visual decisions (notably the default preset's ring/seam anchor).
pub(crate) const WHITE: Rgba = [0xFF, 0xFF, 0xFF, 0xFF];
pub(crate) const UP: Rgba = [0x3E, 0x9B, 0x5F, 0xFF];
pub(crate) const DOWN: Rgba = [0xD6, 0x4F, 0x45, 0xFF];
/// Ring weight on a marker hull. The default preset's card-coloured ring (see
/// [`ChartStyle::marker_ring`](crate::ChartStyle::marker_ring)) anchors on this weight and on
/// [`WHITE`]; other presets resolve their own ring and seam through the style tokens.
pub(crate) const MARKER_RING_WEIGHT: f32 = 0.75;

const TITLE_SIZE: f32 = 14.0;
const LABEL_SIZE: f32 = 9.0;
/// Most categories drawn by category-axis charts.
const MAX_CATS: usize = 20_000;
/// Lines with more points than this are decimated (min/max per pixel column).
const MAX_LINE_PTS: usize = 2000;
/// Most markers per series (after de-duplicating by pixel).
const MAX_MARKERS: usize = 5000;
/// Data labels are skipped when a series has more points than this.
const MAX_LABELS: usize = 200;
/// Columns and bars narrower than this (in pixels) are merged per pixel column into one rectangle.
const MIN_BAR_PX: f32 = 0.25;
/// Bars thinner than this (in pixels) on either axis stay square: a rounded tip needs room.
const MIN_ROUNDED_BAR_PX: f32 = 3.0;
/// Corner radius of a bar tip, as a fraction of its shorter side (capped at half that side).
const BAR_RADIUS_FRACTION: f32 = 0.25;
/// Far-stop alpha of a bar's sheen ramp, as a fraction of the series colour's alpha.
const GRADIENT_SHEEN_ALPHA: f32 = 0.75;
/// A ramp shorter than this on both axes is not resolvable by any of the four backends (sub-pixel
/// at every zoom a chart is drawn at): it degrades to its start colour.
pub(crate) const MIN_GRADIENT_SPAN: f32 = 1e-4;
/// Most wedges a doughnut draws for the whole chart. The per-series and per-category limits do not
/// bound their product: 255 series of 20,000 categories would be millions of wedges. Every drawable
/// ring shares this budget, so the innermost ring keeps its largest slices and the outer rings are
/// capped instead of the chart dropping rings wholesale.
pub(crate) const MAX_WEDGES: usize = 2_048;

struct Ctx<'a> {
    out: Vec<Prim>,
    m: &'a dyn Measure,
    /// Resolved surface and ink for this chart's style preset.
    style: ChartStyle,
}

#[cfg(test)]
#[path = "render_perf_tests.rs"]
mod perf_tests;

impl Ctx<'_> {
    fn tw(&self, s: &str, size: f32) -> f32 {
        let w = self.m.text_width(s, size, false);
        if w.is_finite() { w.max(0.0) } else { 0.0 }
    }
    /// `s` shortened with an ellipsis to fit `maxw`; empty when not even one character fits.
    ///
    /// Binary search over the number of kept characters (at most 400 are tried), about nine
    /// `Measure` calls instead of one per character. It assumes the width of `prefix + "…"` does not
    /// shrink as the prefix grows, which holds for per-character advance sums (`ApproxMeasure`, the
    /// PDF metrics) and for the UI's text layout up to kerning of a few hundredths of a point; with
    /// such a non-monotone measure the result may differ from the old linear scan by a character.
    /// A label with a line break is not monotone (the ellipsis moves to a new line and the widest
    /// line can get shorter), so it takes the linear scan, longest prefix first.
    fn fit(&self, s: &str, size: f32, maxw: f32) -> String {
        if self.tw(s, size) <= maxw {
            return s.to_string();
        }
        // Byte offset where each of the first 400 characters starts; a prefix of `k` characters
        // (k < number of characters, so the offset exists) ends at `starts[k]`.
        let starts: Vec<usize> = s.char_indices().map(|(i, _)| i).take(400).collect();
        let top = starts.len().saturating_sub(1);
        let fits = |k: usize| -> bool {
            let end = starts.get(k).copied().unwrap_or(s.len());
            let mut t = String::with_capacity(end + 3);
            t.push_str(s.get(..end).unwrap_or(""));
            t.push('…');
            self.tw(&t, size) <= maxw
        };
        if s.contains('\n') {
            let k = (1..=top).rev().find(|&k| fits(k));
            return k.map_or_else(String::new, |k| format!("{}…", s.get(..starts.get(k).copied().unwrap_or(s.len())).unwrap_or("")));
        }
        let (mut lo, mut hi) = (0usize, top);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        if lo == 0 { String::new() } else { format!("{}…", s.get(..starts.get(lo).copied().unwrap_or(s.len())).unwrap_or("")) }
    }
    fn text(&mut self, x: f32, y: f32, s: &str, size: f32, color: Rgba, (align, valign): (HAlign, VAlign)) {
        if s.is_empty() {
            return;
        }
        self.out.push(Prim::Text { x, y, text: s.to_string(), size, color, bold: false, align, valign, rotation: 0.0 });
    }
    /// Push a rectangle with an explicit paint and per-corner radii.
    fn rect_paint(&mut self, r: Bx, fill: Paint, stroke: Option<(Rgba, f32)>, radii: [f32; 4]) {
        self.out.push(Prim::Rect { x: r.x, y: r.y, w: r.w, h: r.h, fill, stroke, radii });
    }
    fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, fill: Rgba, stroke: Option<(Rgba, f32)>) {
        self.rect_paint(Bx { x, y, w, h }, Some(Fill::solid(fill)), stroke, [0.0; 4]);
    }
    /// A two-stop sheen along a vertical value axis: full c on the baseline at y0, fading to
    /// GRADIENT_SHEEN_ALPHA of its alpha at the value end y1, both at x.
    fn gradient_axis_v(&self, c: Rgba, x: f32, y0: f32, y1: f32) -> Fill {
        Fill::Linear { from: c, to: sheen(c), start: [x, y0], end: [x, y1] }
    }
    /// A two-stop sheen along a horizontal value axis: full c on the baseline at x0, fading at
    /// the value end x1, both at y.
    fn gradient_axis_h(&self, c: Rgba, y: f32, x0: f32, x1: f32) -> Fill {
        Fill::Linear { from: c, to: sheen(c), start: [x0, y], end: [x1, y] }
    }
    fn line(&mut self, pts: Vec<[f32; 2]>, color: Rgba, width: f32) {
        if pts.len() >= 2 {
            self.out.push(Prim::Line { pts, color, width, dash: false });
        }
    }
    fn hline(&mut self, x0: f32, x1: f32, y: f32, color: Rgba, width: f32) {
        self.line(vec![[x0, y], [x1, y]], color, width);
    }
    fn vline(&mut self, x: f32, y0: f32, y1: f32, color: Rgba, width: f32) {
        self.line(vec![[x, y0], [x, y1]], color, width);
    }
}

/// The faded far stop of a sheen: same hue, alpha at GRADIENT_SHEEN_ALPHA.
fn sheen(c: Rgba) -> Rgba {
    [c[0], c[1], c[2], (c[3] as f32 * GRADIENT_SHEEN_ALPHA).round().clamp(0.0, 255.0) as u8]
}

fn sanitize_vals(v: &[Option<f64>]) -> Vec<Option<f64>> {
    v.iter().take(MAX_POINTS).map(|x| x.filter(|n| n.is_finite()).map(clean)).collect()
}

pub(crate) fn sanitize(d: &ChartData) -> ChartData {
    ChartData {
        palette: d.palette.iter().take(MAX_POINTS).copied().collect(),
        categories: d.categories.iter().take(MAX_POINTS).cloned().collect(),
        category_levels: d.category_levels.iter().take(3).map(|l| l.iter().take(MAX_POINTS).cloned().collect()).collect(),
        series: d
            .series
            .iter()
            .take(MAX_SERIES)
            .map(|s| SeriesData {
                name: s.name.clone(),
                values: sanitize_vals(&s.values),
                x: s.x.as_deref().map(sanitize_vals),
                sizes: s.sizes.as_deref().map(sanitize_vals),
                color: s.color,
                kind: s.kind,
                secondary: s.secondary,
                smooth: s.smooth,
                trendlines: s.trendlines.iter().take(MAX_TRENDLINES).cloned().collect(),
                number_format: s.number_format.clone(),
            })
            .collect(),
    }
}

/// Whether `v` survives `sanitize_vals` unchanged.
fn is_clean(v: &Option<f64>) -> bool {
    v.is_none_or(|n| n.is_finite() && clean(n) == n)
}

/// Whether [`sanitize`] and [`apply_budget`] would change `d`.
fn needs_sanitizing(d: &ChartData) -> bool {
    let cap = points_per_series(d.series.len());
    let long = |n: usize| n > cap;
    let mut trendlines = 0usize;
    d.palette.len() > MAX_POINTS
        || long(d.categories.len())
        || d.category_levels.len() > 3
        || d.category_levels.iter().any(|l| l.len() > MAX_POINTS)
        || d.series.len() > MAX_SERIES
        || d.series.iter().any(|s| {
            trendlines += s.trendlines.len();
            long(s.values.len())
                || !s.values.iter().all(is_clean)
                || s.x.as_ref().is_some_and(|x| long(x.len()) || !x.iter().all(is_clean))
                || s.sizes.as_ref().is_some_and(|x| long(x.len()) || !x.iter().all(is_clean))
                || s.trendlines.len() > MAX_TRENDLINES
        })
        || trendlines > MAX_TRENDLINES
}

/// Enforces the chart's budget of points (`MAX_TOTAL_POINTS` = series x points: every series and
/// the category list keep their first `MAX_TOTAL_POINTS / series` points) and of trendlines
/// (`MAX_TRENDLINES` per chart, in series order).
fn apply_budget(mut d: ChartData) -> ChartData {
    let cap = points_per_series(d.series.len());
    d.categories.truncate(cap);
    let mut trendlines = MAX_TRENDLINES;
    for s in &mut d.series {
        s.values.truncate(cap);
        for v in [&mut s.x, &mut s.sizes].into_iter().flatten() {
            v.truncate(cap);
        }
        s.trendlines.truncate(trendlines);
        trendlines = trendlines.saturating_sub(s.trendlines.len());
    }
    d
}

/// The data `render` draws: `d` itself when it is already clean and within budget (the usual
/// case: no copy), else a cleaned, truncated copy.
pub(crate) fn sanitized(d: &ChartData) -> Cow<'_, ChartData> {
    if needs_sanitizing(d) { Cow::Owned(apply_budget(sanitize(d))) } else { Cow::Borrowed(d) }
}

/// Lays out and draws a chart into a `w`Ã—`h` box at the origin. Empty for a degenerate box.
pub fn render(chart: &Chart, data: &ChartData, w: f32, h: f32, m: &dyn Measure) -> Vec<Prim> {
    if !(w.is_finite() && h.is_finite()) || w < 1.0 || h < 1.0 || w > 1e7 || h > 1e7 {
        return vec![];
    }
    let data = sanitized(data);
    let style = chart_style(chart.style);
    let mut ctx = Ctx { out: Vec::new(), m, style };
    // The card is an explicit primitive (prims[0]), so a dark preset paints its own surface here
    // rather than relying on whatever background the host happens to draw behind the chart.
    ctx.out.push(Prim::Rect { x: 0.0, y: 0.0, w, h, fill: Some(Fill::solid(style.card)), stroke: Some((style.card_border, 1.0)), radii: [0.0; 4] });
    let pad = (w.min(h) * 0.04).clamp(2.0, 10.0);
    let mut area = Bx { x: pad, y: pad, w: w - 2.0 * pad, h: h - 2.0 * pad };
    if let Some(t) = chart_title(chart, &data) {
        let size = TITLE_SIZE.min(h * 0.12).max(4.0);
        let t = ctx.fit(&t, size, area.w);
        ctx.text(w / 2.0, area.y, &t, size, ctx.style.text, (HAlign::Center, VAlign::Top));
        let used = size * 1.3 + 4.0;
        area.y += used;
        area.h -= used;
    }
    let entries = legend_entries(chart, &data);
    if !entries.is_empty() && area.ok() {
        area = draw_legend(&mut ctx, &entries, chart.legend, area);
    }
    if area.ok() && !data.series.is_empty() {
        draw_kind(&mut ctx, chart, &data, area);
    }
    finalize(ctx.out, w, h)
}

fn draw_kind(ctx: &mut Ctx, chart: &Chart, data: &ChartData, area: Bx) {
    use ChartKind::*;
    match chart.kind {
        Pie => pie(ctx, chart, data, area, 0.0, false),
        Doughnut => doughnut(ctx, chart, data, area),
        Sunburst => pie(ctx, chart, data, area, 0.2, true),
        Treemap => treemap(ctx, chart, data, area),
        Funnel => funnel(ctx, chart, data, area),
        Radar => radar(ctx, chart, data, area),
        _ => {
            let area = axis_titles(ctx, chart, area);
            if !area.ok() {
                return;
            }
            match chart.kind {
                BarClustered | BarStacked | BarStacked100 => cartesian(ctx, chart, data, area, true),
                Scatter | ScatterLines | Bubble => scatter(ctx, chart, data, area),
                Histogram => histogram(ctx, chart, data, area),
                Waterfall => waterfall(ctx, chart, data, area),
                BoxWhisker => box_whisker(ctx, chart, data, area),
                Stock => stock(ctx, chart, data, area),
                _ => cartesian(ctx, chart, data, area, false),
            }
        }
    }
}

// ---------------------------------------------------------------- final safety pass

/// Drops non-finite primitives and clamps every coordinate into the `w`Ã—`h` box, then hands every
/// surviving primitive to [`crate::sanitize`] for its fill, radii and stroke.
pub(crate) fn finalize(prims: Vec<Prim>, w: f32, h: f32) -> Vec<Prim> {
    let cx = |v: f32| v.clamp(0.0, w);
    let cy = |v: f32| v.clamp(0.0, h);
    let pts_ok = |pts: &[[f32; 2]]| pts.iter().all(|p| fin(p[0]) && fin(p[1]));
    prims
        .into_iter()
        .filter_map(|p| match p {
            Prim::Rect { x, y, w: rw, h: rh, fill, stroke, radii } => {
                if !(fin(x) && fin(y) && fin(rw) && fin(rh)) {
                    return None;
                }
                let (x0, x1) = (cx(x.min(x + rw)), cx(x.max(x + rw)));
                let (y0, y1) = (cy(y.min(y + rh)), cy(y.max(y + rh)));
                if x1 - x0 <= 0.0 || y1 - y0 <= 0.0 {
                    return None;
                }
                let radii = clamp_radii(radii, x1 - x0, y1 - y0);
                let fill = clamp_fill(fill);
                Some(Prim::Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0, fill, stroke: clamp_stroke(stroke), radii })
            }
            Prim::Line { pts, color, width, dash } => {
                if !pts_ok(&pts) || pts.len() < 2 || !fin(width) {
                    return None;
                }
                let pts = pts.into_iter().map(|p| [cx(p[0]), cy(p[1])]).collect();
                Some(Prim::Line { pts, color, width: width.clamp(0.1, 50.0), dash })
            }
            Prim::Polygon { pts, fill, stroke } => {
                if !pts_ok(&pts) || pts.len() < 3 {
                    return None;
                }
                let pts = pts.into_iter().map(|p| [cx(p[0]), cy(p[1])]).collect();
                let fill = clamp_fill(fill);
                Some(Prim::Polygon { pts, fill, stroke: clamp_stroke(stroke) })
            }
            Prim::Wedge { cx: x, cy: y, r_outer, r_inner, a0, a1, fill, stroke } => {
                if ![x, y, r_outer, r_inner, a0, a1].iter().all(|v| fin(*v)) {
                    return None;
                }
                let (x, y) = (cx(x), cy(y));
                let maxr = x.min(w - x).min(y).min(h - y);
                let ro = r_outer.min(maxr);
                if ro <= 0.0 {
                    return None;
                }
                let fill = clamp_fill(fill);
                Some(Prim::Wedge { cx: x, cy: y, r_outer: ro, r_inner: r_inner.clamp(0.0, ro), a0, a1, fill, stroke: clamp_stroke(stroke) })
            }
            Prim::Circle { cx: x, cy: y, r, fill, stroke } => {
                if !(fin(x) && fin(y) && fin(r)) {
                    return None;
                }
                let (x, y) = (cx(x), cy(y));
                let r = r.min(x.min(w - x).min(y).min(h - y));
                if r <= 0.0 {
                    return None;
                }
                Some(Prim::Circle { cx: x, cy: y, r, fill, stroke: clamp_stroke(stroke) })
            }
            Prim::Text { x, y, text, size, color, bold, align, valign, rotation } => {
                if !(fin(x) && fin(y) && fin(size) && fin(rotation)) || text.is_empty() {
                    return None;
                }
                Some(Prim::Text { x: cx(x), y: cy(y), text, size: size.clamp(1.0, 200.0), color, bold, align, valign, rotation })
            }
        })
        .collect()
}

