//! Charts as inline SVG for Save as Web Page: the chart crate's drawing primitives, text included.

use std::fmt::Write as _;

use gridcraft_chart::{Fill, HAlign, Prim, Rgba, VAlign};
use gridcraft_model::{Chart, Workbook};

use crate::io::esc;

/// Widest or tallest chart written; larger anchors are clamped.
const MAX_SIDE: f32 = 16_384.0;

/// Most bytes of SVG written for the charts of one web page together. A chart past what is
/// left makes the export fail rather than grow without bound or be dropped without a word.
pub(crate) const MAX_PAGE_SVG_BYTES: usize = 64 * 1024 * 1024;

/// Most distinct linear gradients written into one `<defs>`. A shape whose ramp was not collected
/// keeps its solid `from` colour rather than referencing a definition that was never written.
const MAX_SVG_GRADIENTS: usize = 64;

struct HelveticaMeasure;

impl gridcraft_chart::Measure for HelveticaMeasure {
    fn text_width(&self, text: &str, size: f32, bold: bool) -> f32 {
        gridcraft_pdf::text_width(if bold { gridcraft_pdf::Font::HelveticaBold } else { gridcraft_pdf::Font::Helvetica }, size, text)
    }
}

/// `chart` of sheet `sheet` as an `<svg>` element placed by `style`, `None` for a degenerate size,
/// or an error for an element longer than `budget` bytes.
pub(crate) fn chart_svg(wb: &Workbook, sheet: usize, chart: &Chart, style: &str, budget: usize) -> crate::Result<Option<String>> {
    let (w, h) = (chart.anchor.width.min(MAX_SIDE), chart.anchor.height.min(MAX_SIDE));
    if !(w >= 1.0 && h >= 1.0) {
        return Ok(None);
    }
    let data = gridcraft_chart::resolve(wb, sheet, chart);
    let prims = gridcraft_chart::render(chart, &data, w, h, &HelveticaMeasure);
    match prims_svg_within(&prims, w, h, style, budget) {
        Some(svg) => Ok(Some(svg)),
        None => Err(crate::EngineError::Other(format!(
            "the charts would make the web page over {} MiB. Save as .xlsx or PDF instead.",
            MAX_PAGE_SVG_BYTES / (1024 * 1024)
        ))),
    }
}

/// The whole element for a scene, unbudgeted: the chart crate's parity fixture is legible to the
/// engine's own backend tests through this.
#[cfg(test)]
pub(crate) fn prims_svg(prims: &[Prim], w: f32, h: f32, style: &str) -> String {
    prims_svg_within(prims, w, h, style, usize::MAX).unwrap_or_default()
}

/// The `<svg>` element, or `None` as soon as it grows past `budget` bytes.
fn prims_svg_within(prims: &[Prim], w: f32, h: f32, style: &str, budget: usize) -> Option<String> {
    let mut out = String::new();
    let _ = write!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" font-family=\"Helvetica,Arial,sans-serif\" style=\"{style}\">"
    );
    let grads = gradients(prims);
    defs(&mut out, &grads);
    for p in prims {
        if out.len() > budget {
            return None;
        }
        match p {
            Prim::Rect { x, y, w, h, fill, stroke, radii } => {
                let r = clamped_radii(*w, *h, *radii);
                let grad = fill.as_ref().and_then(|g| gradient_id(&grads, g));
                // Four equal corners keep the compact `<rect rx>`; SVG has no per-corner radius,
                // so anything else becomes a `<path>` of four arcs.
                if r[0] == r[1] && r[1] == r[2] && r[2] == r[3] {
                    let _ = write!(out, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"", num(*x), num(*y), num(*w), num(*h));
                    if r[0] > 0.0 {
                        let _ = write!(out, " rx=\"{}\"", num(r[0]));
                    }
                } else {
                    let _ = write!(out, "<path d=\"{}\"", rounded_path(*x, *y, *w, *h, r));
                }
                paint(&mut out, *fill, grad, *stroke);
                out.push_str("/>");
            }
            Prim::Line { pts, color, width, dash } => {
                let _ = write!(out, "<polyline points=\"{}\"", points(pts));
                paint(&mut out, None, None, Some((*color, *width)));
                if *dash {
                    out.push_str(" stroke-dasharray=\"3 2\"");
                }
                out.push_str("/>");
            }
            Prim::Polygon { pts, fill, stroke } => {
                let _ = write!(out, "<polygon points=\"{}\"", points(pts));
                paint(&mut out, *fill, None, *stroke);
                out.push_str("/>");
            }
            Prim::Wedge { cx, cy, r_outer, r_inner, a0, a1, fill, stroke } => {
                let _ = write!(out, "<path d=\"{}\"", wedge_path(*cx, *cy, *r_outer, *r_inner, *a0, *a1));
                paint(&mut out, *fill, None, *stroke);
                out.push_str("/>");
            }
            Prim::Circle { cx, cy, r, fill, stroke } => {
                let _ = write!(out, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"", num(*cx), num(*cy), num(*r));
                paint(&mut out, fill.map(Fill::solid), None, *stroke);
                out.push_str("/>");
            }
            Prim::Text { x, y, text, size, color, bold, align, valign, rotation } => {
                // Baseline offsets as in the PDF export.
                let dy = match valign {
                    VAlign::Top => size * 0.78,
                    VAlign::Middle => size * 0.35,
                    VAlign::Bottom => -size * 0.21,
                };
                let anchor = match align {
                    HAlign::Left => "start",
                    HAlign::Center => "middle",
                    HAlign::Right => "end",
                };
                let _ = write!(out, "<text x=\"{}\" y=\"{}\" font-size=\"{}\" text-anchor=\"{anchor}\"", num(*x), num(y + dy), num(*size));
                if *bold {
                    out.push_str(" font-weight=\"bold\"");
                }
                if rotation.is_finite() && *rotation != 0.0 {
                    let _ = write!(out, " transform=\"rotate({} {} {})\"", num(rotation.to_degrees()), num(*x), num(*y));
                }
                let _ = write!(out, " fill=\"{}\"{}>{}</text>", hex(*color), opacity("fill", *color), esc(text));
            }
        }
    }
    out.push_str("</svg>");
    (out.len() <= budget).then_some(out)
}

/// Fill and stroke attributes of an open shape element; the caller closes it. `grad` is the index
/// of the written `<linearGradient>` when the fill is one of the collected ramps.
fn paint(out: &mut String, fill: Option<Fill>, grad: Option<usize>, stroke: Option<(Rgba, f32)>) {
    match (fill, grad) {
        (Some(_), Some(i)) => {
            let _ = write!(out, " fill=\"url(#g{i})\"");
        }
        (Some(c), None) => {
            let _ = write!(out, " fill=\"{}\"{}", hex(c.start()), opacity("fill", c.start()));
        }
        (None, _) => out.push_str(" fill=\"none\""),
    }
    if let Some((c, w)) = stroke {
        let _ = write!(out, " stroke=\"{}\" stroke-width=\"{}\"{}", hex(c), num(w), opacity("stroke", c));
    }
}

/// The distinct ramps used by the scene, in first-use order, up to [`MAX_SVG_GRADIENTS`].
/// Shortest ramp axis that is still a direction. Below it the shape paints its solid "from" stop,
/// so every backend degrades a degenerate ramp the same way; the chart crate flattens these centrally
/// in sanitize.rs and this is the backstop for direct callers (the rasteriser's Ramp::of uses the
/// same floor). An SVG userSpaceOnUse gradient with a zero-length axis would otherwise paint its last
/// stop and leave the four backends disagreeing.
const MIN_GRADIENT_SPAN: f32 = 1.0e-4;

/// Whether a ramp has an axis at all (finite, and longer than MIN_GRADIENT_SPAN).
fn directional(fill: &Fill) -> bool {
    let Fill::Linear { start, end, .. } = *fill else {
        return false;
    };
    let d = [end[0] - start[0], end[1] - start[1]];
    let len_sq = d[0] * d[0] + d[1] * d[1];
    len_sq.is_finite() && len_sq > MIN_GRADIENT_SPAN
}

fn gradients(prims: &[Prim]) -> Vec<Fill> {
    let mut out: Vec<Fill> = Vec::new();
    for p in prims {
        let Prim::Rect { fill: Some(f @ Fill::Linear { .. }), .. } = p else {
            continue;
        };
        if out.len() >= MAX_SVG_GRADIENTS {
            break;
        }
        if directional(f) && !out.contains(f) {
            out.push(*f);
        }
    }
    out
}

/// The index of a collected ramp, `None` when it was not collected (past the cap), in which case
/// the shape paints its solid `from` colour instead of a dangling `url(#..)`.
fn gradient_id(grads: &[Fill], fill: &Fill) -> Option<usize> {
    grads.iter().position(|g| g == fill)
}

/// `<defs>` with one `userSpaceOnUse` axial gradient per collected ramp, or nothing at all.
fn defs(out: &mut String, grads: &[Fill]) {
    if grads.is_empty() {
        return;
    }
    out.push_str("<defs>");
    for (i, g) in grads.iter().enumerate() {
        let Fill::Linear { from, to, start, end } = *g else {
            continue;
        };
        let _ = write!(
            out,
            "<linearGradient id=\"g{i}\" gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\">",
            num(start[0]),
            num(start[1]),
            num(end[0]),
            num(end[1])
        );
        for (offset, c) in [(0, from), (1, to)] {
            let _ = write!(out, "<stop offset=\"{offset}\" stop-color=\"{}\"{}", hex(c), opacity("stop", c));
            out.push_str("/>");
        }
        out.push_str("</linearGradient>");
    }
    out.push_str("</defs>");
}

/// Each radius clamped on its own to the half-extent, a non-finite one to zero.
fn clamped_radii(w: f32, h: f32, radii: [f32; 4]) -> [f32; 4] {
    let limit = (w.min(h) / 2.0).max(0.0);
    radii.map(|v| if v.is_finite() { v.clamp(0.0, limit) } else { 0.0 })
}

/// A rectangle with an independent radius per corner, as four arcs (clockwise from the top left).
fn rounded_path(x: f32, y: f32, w: f32, h: f32, r: [f32; 4]) -> String {
    let [nw, ne, se, sw] = r;
    let (x1, y1) = (x + w, y + h);
    let arc = |cx: f32, cy: f32, rad: f32| format!("A{r} {r} 0 0 1 {} {}", num(cx), num(cy), r = num(rad));
    format!(
        "M{} {} H{} {} V{} {} H{} {} V{} {} Z",
        num(x + nw),
        num(y),
        num(x1 - ne),
        arc(x1, y + ne, ne),
        num(y1 - se),
        arc(x1 - se, y1, se),
        num(x + sw),
        arc(x, y1 - sw, sw),
        num(y + nw),
        arc(x + nw, y, nw)
    )
}

/// A pie or doughnut sector. Angles run clockwise from 12 o'clock, like the chart crate's.
fn wedge_path(cx: f32, cy: f32, r_outer: f32, r_inner: f32, a0: f32, a1: f32) -> String {
    let span = (a1 - a0).clamp(-std::f32::consts::TAU + 0.001, std::f32::consts::TAU - 0.001);
    let a1 = a0 + span;
    let at = |a: f32, r: f32| format!("{} {}", num(cx + r * a.sin()), num(cy - r * a.cos()));
    let large = u8::from(span.abs() > std::f32::consts::PI);
    let sweep = u8::from(span >= 0.0);
    let mut d = format!("M{} A{r} {r} 0 {large} {sweep} {}", at(a0, r_outer), at(a1, r_outer), r = num(r_outer));
    if r_inner > 0.0 {
        let _ = write!(d, " L{} A{r} {r} 0 {large} {} {} Z", at(a1, r_inner), 1 - sweep, at(a0, r_inner), r = num(r_inner));
    } else {
        let _ = write!(d, " L{} {} Z", num(cx), num(cy));
    }
    d
}

fn points(pts: &[[f32; 2]]) -> String {
    pts.iter().map(|p| format!("{},{}", num(p[0]), num(p[1]))).collect::<Vec<_>>().join(" ")
}

fn hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn opacity(attr: &str, c: Rgba) -> String {
    if c[3] == 255 { String::new() } else { format!(" {attr}-opacity=\"{:.2}\"", f32::from(c[3]) / 255.0) }
}

/// A coordinate with two decimals; a non-finite value is 0 so the SVG stays well-formed.
fn num(v: f32) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_svg_past_its_byte_budget_is_left_out() {
        let rect = Prim::Rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0, fill: Some(Fill::solid([255, 0, 0, 255])), stroke: None, radii: [0.0; 4] };
        let prims = vec![rect; 10_000];
        let full = prims_svg(&prims, 100.0, 50.0, "");
        assert!(full.len() > 100_000);
        assert_eq!(prims_svg_within(&prims, 100.0, 50.0, "", full.len()), Some(full.clone()));
        assert_eq!(prims_svg_within(&prims, 100.0, 50.0, "", full.len() - 1), None);
        assert_eq!(prims_svg_within(&prims, 100.0, 50.0, "", 1_000), None);
    }

    #[test]
    fn every_primitive_becomes_an_svg_element() {
        let red = [255, 0, 0, 255];
        let prims = vec![
            Prim::Rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0, fill: Some(Fill::solid(red)), stroke: None, radii: [0.0; 4] },
            Prim::Line { pts: vec![[0.0, 0.0], [5.0, 5.5]], color: red, width: 1.0, dash: true },
            Prim::Polygon { pts: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]], fill: Some(Fill::solid(red)), stroke: Some(([0, 0, 0, 255], 1.0)) },
            Prim::Wedge { cx: 10.0, cy: 10.0, r_outer: 5.0, r_inner: 2.0, a0: 0.0, a1: 1.0, fill: Some(Fill::solid(red)), stroke: None },
            Prim::Circle { cx: 1.0, cy: 1.0, r: 2.0, fill: Some([0, 0, 255, 128]), stroke: None },
            Prim::Text {
                x: 5.0,
                y: 6.0,
                text: "A<B".into(),
                size: 10.0,
                color: red,
                bold: true,
                align: HAlign::Center,
                valign: VAlign::Top,
                rotation: -1.5,
            },
        ];
        let svg = prims_svg(&prims, 100.0, 50.0, "left:1px");
        for part in [
            "<rect x=\"1\" y=\"2\" width=\"3\" height=\"4\" fill=\"#ff0000\"/>",
            "<polyline points=\"0,0 5,5.5\" fill=\"none\" stroke=\"#ff0000\" stroke-width=\"1\" stroke-dasharray=\"3 2\"/>",
            "<polygon ",
            "<path d=\"M10 5 A5 5 0 0 1 ",
            "fill-opacity=\"0.50\"",
            "text-anchor=\"middle\"",
            "font-weight=\"bold\"",
            "transform=\"rotate(-85.94 5 6)\"",
            ">A&lt;B</text>",
        ] {
            assert!(svg.contains(part), "{part} in {svg}");
        }
        assert!(svg.starts_with("<svg ") && svg.ends_with("</svg>") && svg.contains("style=\"left:1px\""), "{svg}");
    }

    #[test]
    fn non_finite_numbers_do_not_break_the_markup() {
        let prims = [Prim::Circle { cx: f32::NAN, cy: f32::INFINITY, r: 1.0, fill: None, stroke: None }];
        assert!(prims_svg(&prims, 10.0, 10.0, "").contains("<circle cx=\"0\" cy=\"0\" r=\"1\" fill=\"none\"/>"));
    }

    /// A rect painted with a two-stop ramp: the SVG needs `<defs>` and a `url(#gN)` fill.
    fn gradient_rect(fill: Fill) -> Prim {
        Prim::Rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0, fill: Some(fill), stroke: None, radii: [0.0; 4] }
    }

    fn ramp() -> Fill {
        Fill::Linear { from: [200, 10, 10, 255], to: [10, 10, 200, 255], start: [0.0, 0.0], end: [0.0, 100.0] }
    }

    #[test]
    fn a_gradient_rect_emits_defs_and_a_url_fill() {
        let svg = prims_svg(&[gradient_rect(ramp())], 10.0, 10.0, "");
        for part in [
            "<defs><linearGradient id=\"g0\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0\" x2=\"0\" y2=\"100\">",
            "<stop offset=\"0\" stop-color=\"#c80a0a\"/>",
            "<stop offset=\"1\" stop-color=\"#0a0ac8\"/>",
            "</linearGradient></defs>",
            "fill=\"url(#g0)\"",
        ] {
            assert!(svg.contains(part), "{part} in {svg}");
        }
        let defs = svg.find("<defs>").unwrap_or(0);
        assert!(defs > 0 && defs < svg.find("url(#g0)").unwrap_or(0), "the defs come before the shapes: {svg}");
    }

    #[test]
    fn two_identical_gradients_share_one_def() {
        let svg = prims_svg(&[gradient_rect(ramp()), gradient_rect(ramp())], 10.0, 10.0, "");
        assert_eq!(svg.matches("<linearGradient").count(), 1, "{svg}");
        assert_eq!(svg.matches("fill=\"url(#g0)\"").count(), 2, "{svg}");
        assert!(!svg.contains("url(#g1)"), "{svg}");
    }

    #[test]
    fn defs_are_inside_the_byte_budget() {
        let prims = [gradient_rect(ramp())];
        let full = prims_svg(&prims, 10.0, 10.0, "");
        assert!(full.contains("<linearGradient"), "the defs are written: {full}");
        assert_eq!(prims_svg_within(&prims, 10.0, 10.0, "", full.len()), Some(full.clone()));
        assert_eq!(prims_svg_within(&prims, 10.0, 10.0, "", full.len() - 1), None, "the defs count against the budget");
    }

    #[test]
    fn asymmetric_corners_become_a_path_with_four_arcs() {
        let solid = Some(Fill::solid([1, 2, 3, 255]));
        let path = Prim::Rect { x: 0.0, y: 0.0, w: 20.0, h: 20.0, fill: solid, stroke: None, radii: [4.0, 0.0, 0.0, 0.0] };
        let svg = prims_svg(&[path], 20.0, 20.0, "");
        assert!(!svg.contains("<rect "), "{svg}");
        assert_eq!(svg.matches(" A").count(), 4, "{svg}");
        assert!(svg.contains("fill=\"#010203\""), "{svg}");
        let round = Prim::Rect { x: 0.0, y: 0.0, w: 20.0, h: 20.0, fill: solid, stroke: None, radii: [4.0; 4] };
        let svg = prims_svg(&[round], 20.0, 20.0, "");
        assert!(svg.contains("<rect x=\"0\" y=\"0\" width=\"20\" height=\"20\" rx=\"4\" fill=\"#010203\"/>"), "{svg}");
    }

    #[test]
    fn an_empty_scene_has_no_defs() {
        let svg = prims_svg(&[], 10.0, 10.0, "");
        assert!(svg.starts_with("<svg ") && !svg.contains("<defs"), "{svg}");
        let flat = prims_svg(&[gradient_rect(Fill::solid([1, 2, 3, 255]))], 10.0, 10.0, "");
        assert!(!flat.contains("<defs"), "a solid fill needs no defs: {flat}");
    }

    #[test]
    fn a_full_circle_wedge_keeps_a_drawable_arc() {
        let d = wedge_path(0.0, 0.0, 10.0, 0.0, 0.0, std::f32::consts::TAU);
        assert!(d.contains("A10 10 0 1 1 ") && d.ends_with("L0 0 Z"), "{d}");
    }

    /// D-03: degenerate rects are still drawable shapes. All-non-finite radii clamp to zero and
    /// take the compact square element; mixed hostile radii still clamp per corner, so the result is
    /// a closed path of real arcs. A non-finite extent or stroke width is written as 0 by num()
    /// instead of poisoning the markup, and the byte ceiling is unchanged.
    #[test]
    fn a_degenerate_primitive_is_still_emitted_as_a_shape() {
        let fill = Some(Fill::solid([90, 90, 200, 255]));
        let ring = Some(([255, 255, 255, 255], f32::NAN));
        let prims = [
            Prim::Rect {
                x: f32::NAN,
                y: 0.0,
                w: f32::INFINITY,
                h: 24.0,
                fill,
                stroke: ring,
                radii: [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::NAN],
            },
            Prim::Rect { x: 0.0, y: 0.0, w: 24.0, h: 24.0, fill, stroke: ring, radii: [f32::NAN, -1.0, f32::INFINITY, 1e30] },
        ];
        let svg = prims_svg(&prims, 64.0, 64.0, "");
        assert!(svg.contains("<rect "), "the all-non-finite radii take the square element: {svg}");
        assert!(!svg.contains(" rx="), "and its corners are square: {svg}");
        assert_eq!(svg.matches(" A").count(), 4, "the mixed radii still make four real arcs: {svg}");
        assert!(svg.contains("fill=\"#5a5ac8\""), "{svg}");
        assert_eq!(svg.matches("stroke=\"#ffffff\"").count(), 2, "both keep the ring: {svg}");
        assert!(svg.contains("stroke-width=\"0\""), "a non-finite width is 0, not NaN: {svg}");
        assert!(!svg.contains("NaN") && !svg.contains("inf"), "{svg}");
        // The byte ceiling is the same one for friendly and hostile scenes.
        let full = prims_svg(&prims, 64.0, 64.0, "");
        assert!(full.len() < MAX_PAGE_SVG_BYTES, "inside MAX_PAGE_SVG_BYTES");
        assert_eq!(prims_svg_within(&prims, 64.0, 64.0, "", full.len()), Some(full.clone()));
        assert_eq!(prims_svg_within(&prims, 64.0, 64.0, "", full.len() - 1), None);
    }
}
