//! Floating objects on the sheet: charts (rendered by `gridcraft-chart` into primitives that
//! we paint with egui), pictures, shapes, equations, slicers and form controls (`form_controls.rs`).

use std::sync::Arc;

use egui::epaint::{PathShape, PathStroke};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};
use gridcraft_chart::{Fill, HAlign, Prim, VAlign};
use gridcraft_engine::model::{Anchor, ObjKind, ObjRef, ShapeKind, Sheet, Workbook};

use crate::SheetApp;
use crate::grid::Geo;
use crate::theme::{self, Tokens};

fn rgba(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}
/// Shortest ramp axis that is still a direction. The chart layer degrades a shorter one before a
/// primitive is built; this is the local backstop, because `paint_prims` is public.
const MIN_RAMP_SPAN: f32 = 1.0e-4;

/// The one dash period every dashed chart gridline uses, in screen pixels at zoom `z`.
fn dash_px(z: f32) -> (f32, f32) {
    (gridcraft_chart::GRID_DASH.0 * z, gridcraft_chart::GRID_DASH.1 * z)
}

/// `[nw, ne, se, sw]` chart-space radii as egui takes them: scaled, rounded, and clamped to the
/// range `CornerRadius` can hold, with a hostile value treated as a square corner.
fn corner_radius(radii: [f32; 4], z: f32) -> egui::CornerRadius {
    let px = |v: f32| {
        let v = v * z;
        if v.is_finite() { v.round().clamp(0.0, 255.0) as u8 } else { 0 }
    };
    egui::CornerRadius { nw: px(radii[0]), ne: px(radii[1]), se: px(radii[2]), sw: px(radii[3]) }
}

/// One colour of the ramp. The ends are the stop colours exactly: an interpolation that lands on
/// `t = 1.0` need not reproduce the far stop to the last bit, so both ends return early (D-01).
fn ramp_colour(from: Color32, to: Color32, t: f32) -> Color32 {
    if t <= 0.0 {
        return from;
    }
    if t >= 1.0 {
        return to;
    }
    let (a, b) = (from.to_srgba_unmultiplied(), to.to_srgba_unmultiplied());
    let mix = |i: usize| (f32::from(a[i]) + (f32::from(b[i]) - f32::from(a[i])) * t).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(mix(0), mix(1), mix(2), mix(3))
}

/// The outline of a rounded rectangle, clockwise from the top-left corner: a corner whose radius
/// cannot fill a pixel stays a single point, the others become an arc of 4 to 8 segments. Each
/// radius is honoured on its own (02-02-04).
fn rounded_outline(r: Rect, radius: egui::CornerRadius) -> Vec<Pos2> {
    use std::f32::consts::{FRAC_PI_2, PI};

    let limit = if r.width().is_finite() && r.height().is_finite() { (r.width().min(r.height()) * 0.5).max(0.0) } else { 0.0 };
    let rad = |v: u8| f32::from(v).clamp(0.0, limit);
    let (nw, ne, se, sw) = (rad(radius.nw), rad(radius.ne), rad(radius.se), rad(radius.sw));
    let mut out = Vec::with_capacity(4 * 9);
    // Twelve o’clock to three for a top-right corner, and so on: the direction the chart
    // rasterizer and the PDF page trace the same corner.
    // Twelve o clock to three for a top-right corner, and so on: the direction the chart rasterizer
    // and the PDF page trace the same corner.
    corner_outline(&mut out, r.left() + nw, r.top() + nw, nw, 3.0 * FRAC_PI_2);
    corner_outline(&mut out, r.right() - ne, r.top() + ne, ne, 0.0);
    corner_outline(&mut out, r.right() - se, r.bottom() - se, se, FRAC_PI_2);
    corner_outline(&mut out, r.left() + sw, r.bottom() - sw, sw, PI);
    out
}

/// Pushes one corner: its single point under half a pixel, otherwise a quarter turn clockwise in
/// screen coordinates starting at `a0`.
fn corner_outline(out: &mut Vec<Pos2>, cx: f32, cy: f32, rad: f32, a0: f32) {
    if rad < 0.5 {
        out.push(pos2(cx, cy));
        return;
    }
    let segments = ((rad * 0.5).ceil() as usize).clamp(4, 8);
    for i in 0..=segments {
        let a = a0 + std::f32::consts::FRAC_PI_2 * (i as f32 / segments as f32);
        out.push(pos2(cx + rad * a.sin(), cy - rad * a.cos()));
    }
}

/// A rectangle filled with a linear ramp: a fan of triangles over its rounded outline, each vertex
/// coloured by its own projection onto the axis (the same formula the rasterizer uses, so all four
/// backends agree along the ramp). The hub is the rect centre, so a rounded tip keeps its arc.
fn ramp_mesh(r: Rect, radius: egui::CornerRadius, from: Color32, to: Color32, a: Pos2, b: Pos2) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let d = b - a;
    let len_sq = d.length_sq();
    let colour_at = |p: Pos2| {
        let off = p - a;
        let t = if len_sq > 0.0 && len_sq.is_finite() { ((off.x * d.x + off.y * d.y) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
        ramp_colour(from, to, t)
    };
    let outline = rounded_outline(r, radius);
    for &p in &outline {
        mesh.colored_vertex(p, colour_at(p));
    }
    let hub = outline.len() as u32;
    mesh.colored_vertex(r.center(), colour_at(r.center()));
    for i in 0..hub {
        mesh.add_triangle(hub, i, (i + 1) % hub);
    }
    mesh
}

/// A polygon filled with a linear ramp as vertical strips, with each vertex coloured by its
/// projection onto  -> b. Modelled on rea_mesh so concave and convex polygons share the same
/// fill geometry and solid fills can keep their existing fast paths.
fn ramp_strips(v: &[Pos2], from: Color32, to: Color32, a: Pos2, b: Pos2) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let n = v.len();
    let half = n / 2;
    if half < 1 {
        return mesh;
    }
    let d = b - a;
    let len_sq = d.length_sq();
    let colour_at = |p: Pos2| {
        let off = p - a;
        let t = if len_sq > 0.0 && len_sq.is_finite() { ((off.x * d.x + off.y * d.y) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
        ramp_colour(from, to, t)
    };
    for &p in v {
        mesh.colored_vertex(p, colour_at(p));
    }
    for i in 0..half.saturating_sub(1) {
        let (t0, t1) = (i as u32, (i + 1) as u32);
        let (b0, b1) = ((n - 1 - i) as u32, (n - 2 - i) as u32);
        mesh.add_triangle(t0, b0, t1);
        mesh.add_triangle(t1, b0, b1);
    }
    mesh
}

struct EguiMeasure<'a> {
    painter: &'a Painter,
    z: f32,
}

impl gridcraft_chart::Measure for EguiMeasure<'_> {
    fn text_width(&self, text: &str, size: f32, bold: bool) -> f32 {
        let font = if bold { theme::ui_bold(size * self.z) } else { theme::ui_font(size * self.z) };
        self.painter.layout_no_wrap(text.to_string(), font, Color32::BLACK).size().x / self.z.max(0.01)
    }
}

/// Paints `chart` (not necessarily on the sheet: the Recommended Charts preview) scaled to fit
/// `rect`.
pub fn paint_chart(p: &Painter, rect: Rect, wb: &Workbook, sheet: usize, chart: &gridcraft_engine::model::Chart) {
    let (w, h) = (chart.anchor.width.max(1.0), chart.anchor.height.max(1.0));
    let z = (rect.width() / w).min(rect.height() / h);
    if !(z.is_finite() && z > 0.0) {
        return;
    }
    let origin = rect.center() - vec2(w * z, h * z) / 2.0;
    let data = gridcraft_chart::resolve(wb, sheet, chart);
    let m = EguiMeasure { painter: p, z };
    let prims = gridcraft_chart::render(chart, &data, w, h, &m);
    paint_prims(&p.with_clip_rect(rect.intersect(p.clip_rect())), origin, &prims, z);
}

/// Paints chart primitives at `origin`, scaled by `z`.
pub fn paint_prims(p: &Painter, origin: Pos2, prims: &[Prim], z: f32) {
    let pt = |x: f32, y: f32| pos2(origin.x + x * z, origin.y + y * z);
    for prim in prims {
        match prim {
            Prim::Rect { x, y, w, h, fill, stroke, radii } => {
                let r = Rect::from_min_size(pt(*x, *y), vec2(w * z, h * z));
                let chart_radii = radii.map(|v| if v.is_finite() { v } else { 0.0 });
                let radius = corner_radius(chart_radii, z);
                match fill {
                    Some(Fill::Solid(c)) => {
                        p.rect_filled(r, radius, rgba(*c));
                    }
                    Some(f @ Fill::Linear { from, to, start, end }) => {
                        let (a, b) = (pt(start[0], start[1]), pt(end[0], end[1]));
                        let len_sq = (b - a).length_sq();
                        if len_sq.is_finite() && len_sq > MIN_RAMP_SPAN {
                            p.add(egui::Shape::mesh(ramp_mesh(r, radius, rgba(*from), rgba(*to), a, b)));
                        } else {
                            p.rect_filled(r, radius, rgba(f.start()));
                        }
                    }
                    None => {}
                }
                if let Some((c, wd)) = stroke {
                    p.rect_stroke(r, radius, Stroke::new(wd * z.max(0.5), rgba(*c)), StrokeKind::Inside);
                }
            }
            Prim::Line { pts, color, width, dash } => {
                let v: Vec<Pos2> = pts.iter().map(|q| pt(q[0], q[1])).collect();
                if v.len() < 2 {
                    continue;
                }
                if *dash {
                    let (on, off) = dash_px(z);
                    p.extend(egui::Shape::dashed_line(&v, Stroke::new(width * z.max(0.5), rgba(*color)), on, off));
                } else {
                    p.add(PathShape::line(v, PathStroke::new(width * z.max(0.5), rgba(*color))));
                }
            }
            Prim::Polygon { pts, fill, stroke } => {
                let v: Vec<Pos2> = pts.iter().map(|q| pt(q[0], q[1])).collect();
                if v.len() < 3 {
                    continue;
                }
                match fill {
                    Some(Fill::Linear { from, to, start, end }) => {
                        let (a, b) = (pt(start[0], start[1]), pt(end[0], end[1]));
                        let len_sq = (b - a).length_sq();
                        if len_sq.is_finite() && len_sq > MIN_RAMP_SPAN {
                            p.add(egui::Shape::mesh(ramp_strips(&v, rgba(*from), rgba(*to), a, b)));
                        } else {
                            let col = rgba(*from);
                            let mut mesh = egui::Mesh::default();
                            for &ptv in &v {
                                mesh.colored_vertex(ptv, col);
                            }
                            let n = v.len();
                            let half = n / 2;
                            for i in 0..half.saturating_sub(1) {
                                let (t0, t1) = (i as u32, (i + 1) as u32);
                                let (b0, b1) = ((n - 1 - i) as u32, (n - 2 - i) as u32);
                                mesh.add_triangle(t0, b0, t1);
                                mesh.add_triangle(t1, b0, b1);
                            }
                            p.add(egui::Shape::mesh(mesh));
                        }
                    }
                    Some(Fill::Solid(c)) => {
                        let col = rgba(*c);
                        if is_convex(&v) {
                            p.add(PathShape { points: v.clone(), closed: true, fill: col, stroke: PathStroke::NONE });
                        } else {
                            p.add(egui::Shape::mesh(area_mesh(&v, col)));
                        }
                    }
                    None => {}
                }
                if let Some((c, w)) = stroke {
                    p.add(PathShape::closed_line(v, PathStroke::new(w * z.max(0.5), rgba(*c))));
                }
            }
            Prim::Wedge { cx, cy, r_outer, r_inner, a0, a1, fill, stroke } => {
                let c = pt(*cx, *cy);
                let steps = (((a1 - a0).abs() / 0.05).ceil() as usize).clamp(2, 400);
                let at = |a: f32, r: f32| pos2(c.x + r * z * a.sin(), c.y - r * z * a.cos());
                let ramp = match fill {
                    Some(Fill::Linear { from, to, start, end }) => {
                        let (sa, sb) = (pt(start[0], start[1]), pt(end[0], end[1]));
                        let len_sq = (sb - sa).length_sq();
                        if len_sq.is_finite() && len_sq > MIN_RAMP_SPAN {
                            Some((rgba(*from), rgba(*to), sa, sb))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                let flat = match fill {
                    Some(Fill::Linear { from, .. }) if ramp.is_none() => rgba(*from),
                    Some(Fill::Solid(sc)) => rgba(*sc),
                    Some(Fill::Linear { from, .. }) => rgba(*from),
                    None => Color32::TRANSPARENT,
                };
                let mut mesh = egui::Mesh::default();
                for i in 0..=steps {
                    let ang = a0 + (a1 - a0) * i as f32 / steps as f32;
                    let outer = at(ang, *r_outer);
                    let inner = if *r_inner > 0.0 { at(ang, *r_inner) } else { c };
                    let (oc, ic) = if let Some((from, to, sa, sb)) = ramp {
                        let d = sb - sa;
                        let len_sq = d.length_sq();
                        let colour_at = |p: Pos2| {
                            let off = p - sa;
                            let t = if len_sq > 0.0 && len_sq.is_finite() { ((off.x * d.x + off.y * d.y) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
                            ramp_colour(from, to, t)
                        };
                        (colour_at(outer), colour_at(inner))
                    } else {
                        (flat, flat)
                    };
                    mesh.colored_vertex(outer, oc);
                    mesh.colored_vertex(inner, ic);
                }
                for i in 0..steps as u32 {
                    let bi = i * 2;
                    mesh.add_triangle(bi, bi + 1, bi + 2);
                    mesh.add_triangle(bi + 1, bi + 3, bi + 2);
                }
                p.add(egui::Shape::mesh(mesh));
                if let Some((sc, w)) = stroke {
                    let mut outline: Vec<Pos2> = (0..=steps).map(|i| at(a0 + (a1 - a0) * i as f32 / steps as f32, *r_outer)).collect();
                    if *r_inner > 0.0 {
                        outline.extend((0..=steps).rev().map(|i| at(a0 + (a1 - a0) * i as f32 / steps as f32, *r_inner)));
                    } else {
                        outline.push(c);
                    }
                    p.add(PathShape::closed_line(outline, PathStroke::new(w * z.max(0.5), rgba(*sc))));
                }
            }
            Prim::Circle { cx, cy, r, fill, stroke } => {
                if let Some(f) = fill {
                    p.circle_filled(pt(*cx, *cy), r * z, rgba(*f));
                }
                if let Some((c, w)) = stroke {
                    p.circle_stroke(pt(*cx, *cy), r * z, Stroke::new(w * z.max(0.5), rgba(*c)));
                }
            }
            Prim::Text { x, y, text, size, color, bold, align, valign, rotation } => {
                let font: FontId = if *bold { theme::ui_bold(size * z) } else { theme::ui_font(size * z) };
                let galley = p.layout_no_wrap(text.clone(), font, rgba(*color));
                let sz = galley.size();
                let ax = match align {
                    HAlign::Left => 0.0,
                    HAlign::Center => 0.5,
                    HAlign::Right => 1.0,
                };
                let ay = match valign {
                    VAlign::Top => 0.0,
                    VAlign::Middle => 0.5,
                    VAlign::Bottom => 1.0,
                };
                let anchor = pt(*x, *y);
                if *rotation == 0.0 {
                    p.galley(pos2(anchor.x - sz.x * ax, anchor.y - sz.y * ay), galley, rgba(*color));
                } else {
                    // Rotate about the anchor: offset the galley origin by the rotated alignment.
                    let (s, c) = rotation.sin_cos();
                    let off = vec2(-sz.x * ax, -sz.y * ay);
                    let rot = vec2(off.x * c - off.y * s, off.x * s + off.y * c);
                    let mut shape = egui::epaint::TextShape::new(anchor + rot, galley, rgba(*color));
                    shape.angle = *rotation;
                    p.add(shape);
                }
            }
        }
    }
}

fn is_convex(v: &[Pos2]) -> bool {
    let n = v.len();
    let mut sign = 0.0f32;
    for i in 0..n {
        let a = v[i];
        let b = v[(i + 1) % n];
        let c = v[(i + 2) % n];
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if cross.abs() > 1e-3 {
            if sign == 0.0 {
                sign = cross.signum();
            } else if cross.signum() != sign {
                return false;
            }
        }
    }
    true
}

/// Area charts produce "mountain" polygons whose bottom edge is a straight baseline: fill them
/// as vertical strips (exact for such shapes, acceptable for others).
fn area_mesh(v: &[Pos2], col: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let n = v.len();
    if n < 3 {
        return mesh;
    }
    let half = n / 2;
    for i in 0..half.saturating_sub(1) {
        let a = v[i];
        let b = v[i + 1];
        let c = v[n - 2 - i];
        let d = v[n - 1 - i];
        let base = mesh.vertices.len() as u32;
        for q in [a, b, c, d] {
            mesh.colored_vertex(q, col);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
    mesh
}

/// A shape's text with its formatting: size, bold/italic, typeface, colour, outline, horizontal
/// alignment and vertical anchor. Shapes other than text boxes default to white text centred in
/// the shape; text boxes to black text at the top left.
fn paint_shape_text(p: &Painter, wb: &Workbook, sp: &gridcraft_engine::model::Shape, r: Rect, z: f32) {
    use gridcraft_engine::model::{ShapeTextAlign, ShapeTextAnchor};
    let f = &sp.format;
    let word_art = sp.kind == ShapeKind::WordArt;
    let text_box = sp.kind == ShapeKind::TextBox;
    let align = f.align.unwrap_or(if text_box { ShapeTextAlign::Left } else { ShapeTextAlign::Center });
    let anchor = f.anchor.unwrap_or(if text_box { ShapeTextAnchor::Top } else { ShapeTextAnchor::Middle });
    let default_color = if text_box || word_art { Color32::BLACK } else { Color32::WHITE };
    let color = f.text_color.and_then(|c| c.resolve(&wb.theme)).map(theme::color32).unwrap_or(default_color);
    // Default 14 px (as before); a set size is in points.
    let size = f.font_size.map_or(14.0, |pt| pt * 96.0 / 72.0).clamp(1.0, 2000.0) * z;
    let family = theme::cell_family(f.font.as_deref().unwrap_or(""), f.bold, f.italic);
    let inset = 4.0;
    let width = (r.width() - 2.0 * inset).max(1.0);
    let mut job = egui::text::LayoutJob::simple(sp.text.clone(), FontId::new(size, family), color, width);
    job.halign = match align {
        ShapeTextAlign::Left => egui::Align::LEFT,
        ShapeTextAlign::Center => egui::Align::Center,
        ShapeTextAlign::Right => egui::Align::RIGHT,
    };
    let g = p.layout_job(job);
    let x = match align {
        ShapeTextAlign::Left => r.left() + inset,
        ShapeTextAlign::Center => r.center().x,
        ShapeTextAlign::Right => r.right() - inset,
    };
    let y = match anchor {
        ShapeTextAnchor::Top => r.top() + inset,
        ShapeTextAnchor::Middle => r.center().y - g.size().y / 2.0,
        ShapeTextAnchor::Bottom => r.bottom() - inset - g.size().y,
    };
    let pos = pos2(x, y);
    let clip = p.with_clip_rect(r.intersect(p.clip_rect()));
    // WordArt's drop shadow: the text again, offset down and right, under everything else.
    if let Some(sc) = f.text_shadow.and_then(|c| c.resolve(&wb.theme)).map(theme::color32) {
        let d = (size * 0.05).clamp(1.0, 24.0);
        clip.galley_with_override_text_color(pos + vec2(d, d), g.clone(), sc);
    }
    // egui has no stroked text: the outline is the text drawn in the outline colour, offset in
    // eight directions by the outline width, under the filled text.
    if let Some((c, w)) = f.text_outline
        && let Some(oc) = c.resolve(&wb.theme).map(theme::color32)
    {
        let d = (w * 96.0 / 72.0 * z / 2.0).clamp(0.5, 20.0);
        for (dx, dy) in [(-1.0, -1.0), (0.0, -1.0), (1.0, -1.0), (-1.0, 0.0), (1.0, 0.0), (-1.0, 1.0), (0.0, 1.0), (1.0, 1.0)] {
            clip.galley_with_override_text_color(pos + vec2(dx * d, dy * d), g.clone(), oc);
        }
    }
    clip.galley(pos, g, color);
}

/// Resolves and renders `ch` (chart `ci` of sheet `si`), at zoom `z`.
fn render_chart(p: &Painter, z: f32, wb: &Workbook, si: usize, ch: &gridcraft_engine::model::Chart) -> Vec<Prim> {
    #[cfg(test)]
    tests::RENDERS.with(|c| c.set(c.get() + 1));
    let data = gridcraft_chart::resolve(wb, si, ch);
    let m = EguiMeasure { painter: p, z };
    gridcraft_chart::render(ch, &data, ch.anchor.width, ch.anchor.height, &m)
}

/// The primitives of chart `ci` of sheet `si`: from the frame cache when the workbook, zoom and
/// screen scale are those it was rendered for, else rendered now (see [`crate::chartcache`]).
fn chart_prims(app: &SheetApp, p: &Painter, z: f32, wb: &Arc<Workbook>, si: usize, ci: usize, ch: &gridcraft_engine::model::Chart) -> Arc<[Prim]> {
    // Only the active document's own workbook is cached: anything else cannot be tied to a uid.
    let Some(d) = app.session.active().filter(|d| Arc::ptr_eq(&d.wb, wb)) else {
        return render_chart(p, z, wb, si, ch).into();
    };
    // A series that calls NOW or RAND, directly or through a defined name, must be evaluated
    // again on every paint.
    if gridcraft_chart::has_volatile_series(wb, si, ch) {
        return render_chart(p, z, wb, si, ch).into();
    }
    let mut cache = app.chart_cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.get(d.uid, wb, si, ci, (z, p.ctx().pixels_per_point()), || render_chart(p, z, wb, si, ch))
}

pub(crate) fn anchor_rect(geo: &Geo, sh: &Sheet, a: &Anchor) -> Rect {
    let x = geo.x(sh, a.cell.col) + a.dx * geo.z;
    let y = geo.y(sh, a.cell.row) + a.dy * geo.z;
    Rect::from_min_size(pos2(x, y), vec2(a.width * geo.z, a.height.max(1.0) * geo.z))
}

/// The shapes one floating object painted (indices into the painter's layer) and its logical
/// rectangle, so a right-to-left sheet can move it as a whole instead of mirroring it.
pub(crate) struct PaintedObject {
    pub shapes: std::ops::Range<usize>,
    pub rect: Rect,
}

/// Ctrl+6 (Options › Advanced › "For objects, show: Nothing"): objects are neither drawn nor
/// clickable. Placeholders, which Excel no longer offers, show the objects.
pub(crate) fn objects_hidden(wb: &Workbook) -> bool {
    wb.show_objects == gridcraft_engine::model::ShowObjects::None
}

pub(crate) fn paint_objects(app: &SheetApp, p: &Painter, geo: &Geo, wb: &Arc<Workbook>, si: usize, sh: &Sheet) -> Vec<PaintedObject> {
    let mut painted = Vec::new();
    if objects_hidden(wb) {
        return painted;
    }
    // Source outlines stay in cell space beneath every floating object.
    if let Some(ch) = sh.charts.iter().find(|ch| app.selected_chart() == Some(ch.id))
        && anchor_rect(geo, sh, &ch.anchor).intersects(p.clip_rect())
    {
        for s in &ch.series {
            if let Ok(gridcraft_engine::formula::Expr::Ref(rf)) = gridcraft_engine::formula::parse(&s.values)
                && rf.sheet_name().is_none_or(|n| n.eq_ignore_ascii_case(&sh.name))
            {
                let rr = geo.range_rect(sh, rf.range());
                let color = Tokens::get(p.ctx()).ref_color(0, true);
                for pane in geo.range_panes(rf.range()) {
                    let pp = p.with_clip_rect(pane.intersect(p.clip_rect()));
                    pp.rect_stroke(rr, 0.0, Stroke::new(1.5, color), StrokeKind::Inside);
                }
            }
        }
    }
    let mut done = |start: egui::layers::ShapeIdx, rect: Rect| painted.push(PaintedObject { shapes: start.0..p.add(egui::Shape::Noop).0, rect });
    // The grid paints the active workbook: its pictures are cached under it.
    let doc = app.session.active().map_or(0, |d| d.uid);
    for sp in &sh.shapes {
        let r = anchor_rect(geo, sh, &sp.anchor);
        if !r.intersects(p.clip_rect()) {
            continue;
        }
        let start = p.add(egui::Shape::Noop);
        let mut label = true;
        let fill = sp.fill.resolve(&wb.theme).map(theme::color32).unwrap_or(Color32::TRANSPARENT);
        let line =
            if sp.format.no_line { Color32::TRANSPARENT } else { sp.line.resolve(&wb.theme).map(theme::color32).unwrap_or(Color32::DARK_GRAY) };
        match sp.kind {
            // WordArt is only its text (no fill or outline of its own).
            ShapeKind::WordArt => {}
            ShapeKind::Rectangle | ShapeKind::TextBox => {
                p.rect_filled(r, 0.0, fill);
                p.rect_stroke(r, 0.0, Stroke::new(1.0, line), StrokeKind::Inside);
            }
            ShapeKind::RoundedRectangle => {
                p.rect_filled(r, 10.0 * geo.z, fill);
                p.rect_stroke(r, 10.0 * geo.z, Stroke::new(1.0, line), StrokeKind::Inside);
            }
            ShapeKind::Ellipse => {
                let pts: Vec<Pos2> = (0..64)
                    .map(|i| {
                        let a = i as f32 / 64.0 * std::f32::consts::TAU;
                        pos2(r.center().x + r.width() / 2.0 * a.cos(), r.center().y + r.height() / 2.0 * a.sin())
                    })
                    .collect();
                p.add(PathShape::convex_polygon(pts, fill, Stroke::new(1.0, line)));
            }
            ShapeKind::Triangle => {
                p.add(PathShape::convex_polygon(vec![pos2(r.center().x, r.top()), r.right_bottom(), r.left_bottom()], fill, Stroke::new(1.0, line)));
            }
            ShapeKind::Ink => {
                let pt = gridcraft_engine::cmd::draw::ink_width(sp);
                let w = pt * geo.z;
                let pts: Vec<Pos2> =
                    gridcraft_engine::cmd::draw::ink_points(sp).iter().map(|q| pos2(r.left() + q[0] * geo.z, r.top() + q[1] * geo.z)).collect();
                if pts.len() >= 2 {
                    let col = ink_color(line, pt);
                    p.add(PathShape::line(pts, PathStroke::new(w, col)));
                }
                label = false;
            }
            ShapeKind::Icon => {
                if let Some(icon) = crate::icons::from_name(&sp.text) {
                    let t = Tokens::get(p.ctx());
                    crate::icons::paint_on_sheet(p, r, icon, if fill == Color32::TRANSPARENT { t.cell_text } else { fill });
                }
                label = false;
            }
            ShapeKind::Equation => {
                crate::equation::paint_shape(p, r, sp, wb, geo.z);
                label = false;
            }
            ShapeKind::Line | ShapeKind::Arrow => {
                p.line_segment([r.left_top(), r.right_bottom()], Stroke::new(2.0, line));
                if sp.kind == ShapeKind::Arrow {
                    let d = (r.right_bottom() - r.left_top()).normalized();
                    let n = vec2(-d.y, d.x);
                    let tip = r.right_bottom();
                    p.add(PathShape::convex_polygon(vec![tip, tip - d * 10.0 + n * 5.0, tip - d * 10.0 - n * 5.0], line, Stroke::NONE));
                }
            }
        }
        if label && !sp.text.is_empty() && !app.editing_text_box(sp.id) {
            paint_shape_text(p, wb, sp, r, geo.z);
        }
        done(start, r);
    }
    for im in &sh.images {
        let r = anchor_rect(geo, sh, &im.anchor);
        if !r.intersects(p.clip_rect()) {
            continue;
        }
        let start = p.add(egui::Shape::Noop);
        if let Some(tex) = crate::floating_pictures::texture(p.ctx(), doc, im, r) {
            p.image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        } else {
            p.rect_filled(r, 0.0, Tokens::get(p.ctx()).sheet_track);
        }
        done(start, r);
    }
    for (ci, ch) in sh.charts.iter().enumerate() {
        let r = anchor_rect(geo, sh, &ch.anchor);
        if !r.intersects(p.clip_rect()) {
            continue;
        }
        let start = p.add(egui::Shape::Noop);
        // Soft shadow + white card.
        p.rect_filled(r.translate(vec2(0.0, 1.5)), 2.0, Color32::from_black_alpha(18));
        p.rect_filled(r, 0.0, Tokens::get(p.ctx()).grid_bg);
        let prims = chart_prims(app, p, geo.z, wb, si, ci, ch);
        paint_prims(&p.with_clip_rect(r.intersect(p.clip_rect())), r.min, &prims, geo.z);
        done(start, r);
    }
    paint_slicers(p, geo, wb, si, sh);
    // Form controls sit above every other object.
    let hover = p.ctx().input(|i| i.pointer.hover_pos()).map(|h| geo.mirror_pos(h)).filter(|h| geo.in_cells(*h));
    for c in &sh.controls {
        let r = anchor_rect(geo, sh, &c.anchor);
        // A slider's value shows beside it while it is dragged.
        if !r.expand(24.0).intersects(p.clip_rect()) {
            continue;
        }
        let start = p.add(egui::Shape::Noop);
        crate::form_controls::paint(p, r, c, geo.z, crate::form_controls::look(app, geo, c, r, hover));
        done(start, r);
    }
    // The selected object's frame and handles go over every object, and move with it on a
    // right-to-left sheet.
    if let Some(r) = app.selected_object.and_then(|o| object_rect(geo, sh, o))
        && r.intersects(p.clip_rect())
    {
        let start = p.add(egui::Shape::Noop);
        selection_frame(p, r);
        done(start, r);
    }
    painted
}

/// Geometry of a slicer's parts within its screen rect.
pub struct SlicerLayout {
    pub header: Rect,
    /// Clear-filter button in the header.
    pub clear: Rect,
    /// One rect per value tile, in order.
    pub tiles: Vec<Rect>,
}

pub fn slicer_layout(rect: Rect, z: f32, n: usize, columns: u32) -> SlicerLayout {
    let header_h = 26.0 * z;
    let pad = 6.0 * z;
    let gap = 3.0 * z;
    let tile_h = 22.0 * z;
    let header = Rect::from_min_size(rect.min, vec2(rect.width(), header_h));
    let cb = 16.0 * z;
    let clear = Rect::from_min_size(pos2(header.right() - pad - cb, header.top() + (header_h - cb) / 2.0), vec2(cb, cb));
    let cols = columns.max(1) as f32;
    let inner_left = rect.left() + pad;
    let inner_top = header.bottom() + pad;
    let inner_w = (rect.width() - 2.0 * pad).max(1.0);
    let tile_w = ((inner_w - (cols - 1.0) * gap) / cols).max(1.0);
    let mut tiles = Vec::with_capacity(n);
    for i in 0..n {
        let col = (i as f32) % cols;
        let row = (i as f32 / cols).floor();
        let x = inner_left + col * (tile_w + gap);
        let y = inner_top + row * (tile_h + gap);
        tiles.push(Rect::from_min_size(pos2(x, y), vec2(tile_w, tile_h)));
    }
    SlicerLayout { header, clear, tiles }
}

fn paint_slicers(p: &Painter, geo: &Geo, wb: &Workbook, si: usize, sh: &Sheet) {
    let tk = theme::Tokens::get(p.ctx());
    for sl in &sh.slicers {
        let r = anchor_rect(geo, sh, &sl.anchor);
        if !r.intersects(p.clip_rect()) {
            continue;
        }
        let items = gridcraft_engine::cmd::slicer::slicer_items(wb, si, sl);
        let lay = slicer_layout(r, geo.z, items.len(), sl.columns);
        let radius = 4.0 * geo.z;
        // Card + soft shadow.
        p.rect_filled(r.translate(vec2(0.0, 1.5)), radius, Color32::from_black_alpha(18));
        p.rect_filled(r, radius, tk.window);
        p.rect_stroke(r, radius, Stroke::new(1.0, tk.separator), StrokeKind::Inside);
        // Header.
        p.rect_filled(lay.header, radius, tk.accent_soft);
        p.rect_filled(Rect::from_min_max(pos2(lay.header.left(), lay.header.bottom() - radius), lay.header.max), 0.0, tk.accent_soft);
        let cap = if sl.caption.is_empty() { sl.column.clone() } else { sl.caption.clone() };
        p.text(lay.header.left_center() + vec2(8.0 * geo.z, 0.0), Align2::LEFT_CENTER, cap, theme::ui_bold(12.5 * geo.z), tk.accent_dark);
        // Clear-filter button (tinted when a filter is active).
        let filtered = sl.selected.is_some();
        if let Some(icon) = crate::icons::from_name("filter") {
            crate::icons::paint(p, lay.clear, icon, if filtered { tk.accent } else { tk.text_dim });
        }
        // Value tiles.
        for ((val, on), tr) in items.iter().zip(&lay.tiles) {
            if !r.contains(tr.left_top()) {
                continue; // overflowed the card; skip (no scroll yet)
            }
            let (fill, fg) = if *on { (tk.accent, tk.on_accent) } else { (tk.hover, tk.text_dim) };
            p.rect_filled(*tr, 3.0 * geo.z, fill);
            let cp = p.with_clip_rect(tr.intersect(p.clip_rect()));
            cp.text(tr.left_center() + vec2(6.0 * geo.z, 0.0), Align2::LEFT_CENTER, val, theme::ui_font(12.0 * geo.z), fg);
        }
    }
}

/// What a click inside a slicer hit.
pub enum SlicerAction {
    Toggle(String),
    Clear,
}

/// A click on a slicer's value tile or clear button, if any. A click elsewhere in a slicer returns
/// `None` so the caller can treat it as a select/drag on the object.
pub fn slicer_click(geo: &Geo, sh: &Sheet, wb: &Workbook, si: usize, p: Pos2) -> Option<(u32, SlicerAction)> {
    if !geo.cells.contains(p) || objects_hidden(wb) {
        return None;
    }
    for sl in sh.slicers.iter().rev() {
        let r = anchor_rect(geo, sh, &sl.anchor);
        if !r.contains(p) {
            continue;
        }
        let items = gridcraft_engine::cmd::slicer::slicer_items(wb, si, sl);
        let lay = slicer_layout(r, geo.z, items.len(), sl.columns);
        if lay.clear.contains(p) {
            return Some((sl.id, SlicerAction::Clear));
        }
        for ((val, _on), tr) in items.iter().zip(&lay.tiles) {
            if r.contains(tr.left_top()) && tr.contains(p) {
                return Some((sl.id, SlicerAction::Toggle(val.clone())));
            }
        }
        return None; // inside this (topmost) slicer, but on its header/background → select/drag
    }
    None
}

fn selection_frame(p: &Painter, r: Rect) {
    let t = Tokens::get(p.ctx());
    p.rect_stroke(r, 0.0, Stroke::new(1.0, t.sheet_control_border), StrokeKind::Outside);
    for h in crate::objects::Handle::ALL {
        let c = h.pos(r);
        p.circle_filled(c, 4.0, t.sheet_control_bg);
        p.circle_stroke(c, 4.0, Stroke::new(1.0, t.sheet_control_border));
    }
}

/// Where object `o` sits on the sheet (logical coordinates), when it exists on `sh`.
pub fn object_rect(geo: &Geo, sh: &Sheet, o: ObjRef) -> Option<Rect> {
    sh.object_anchor(o).map(|a| anchor_rect(geo, sh, &a))
}

/// The resize handle of the selected object under `p`, with the object and its rect. Handles
/// reach a little outside their object, so they win over [`hit`], but like it only over the cell
/// area: a handle that lies under the headers, the formula bar or a scroll bar leaves the press
/// to them. Nothing while objects are hidden.
pub fn selected_handle(app: &SheetApp, geo: &Geo, sh: &Sheet, p: Pos2) -> Option<(ObjRef, Rect, crate::objects::Handle)> {
    if !geo.cells.contains(p) || app.session.active().is_some_and(|d| objects_hidden(&d.wb)) {
        return None;
    }
    let o = app.selected_object?;
    let r = object_rect(geo, sh, o)?;
    crate::objects::handle_at(r, p).map(|h| (o, r, h))
}

/// The topmost object under `p` and its rect: form controls paint on top, so they are hit first,
/// then slicers, charts, pictures and shapes (equations included), each from the top of its stack.
/// Nothing is hit outside the cell area or while objects are hidden.
pub fn hit(app: &SheetApp, geo: &Geo, sh: &Sheet, p: Pos2) -> Option<(ObjRef, Rect)> {
    if !geo.cells.contains(p) || app.session.active().is_some_and(|d| objects_hidden(&d.wb)) {
        return None;
    }
    for c in sh.controls.iter().rev() {
        let r = anchor_rect(geo, sh, &c.anchor);
        if r.contains(p) {
            return Some((ObjRef::new(ObjKind::Control, c.id), r));
        }
    }
    for sl in sh.slicers.iter().rev() {
        let r = anchor_rect(geo, sh, &sl.anchor);
        if r.contains(p) {
            return Some((ObjRef::new(ObjKind::Slicer, sl.id), r));
        }
    }
    for ch in sh.charts.iter().rev() {
        let r = anchor_rect(geo, sh, &ch.anchor);
        if r.expand(4.0).contains(p) {
            return Some((ObjRef::new(ObjKind::Chart, ch.id), r));
        }
    }
    for im in sh.images.iter().rev() {
        let r = anchor_rect(geo, sh, &im.anchor);
        if r.contains(p) {
            return Some((ObjRef::new(ObjKind::Image, im.id), r));
        }
    }
    for s in sh.shapes.iter().rev() {
        let r = anchor_rect(geo, sh, &s.anchor);
        if r.expand(3.0).contains(p) {
            let kind = if s.kind == ShapeKind::Equation { ObjKind::Equation } else { ObjKind::Shape };
            return Some((ObjRef::new(kind, s.id), r));
        }
    }
    None
}

/// The colour ink is painted in: a highlighter (a stroke over 8 pt wide) is translucent, by its
/// width in points, so the live pen stroke and the committed ink agree at every zoom.
pub(crate) fn ink_color(line: Color32, width_pt: f32) -> Color32 {
    if width_pt > 8.0 { line.gamma_multiply(0.45) } else { line }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::Cell;

    use gridcraft_chart::{Fill, Measure};

    use super::*;

    thread_local! {
        /// Charts resolved and rendered on this thread (a test runs on its own thread).
        pub(crate) static RENDERS: Cell<usize> = const { Cell::new(0) };
    }

    /// The production measure is not monotone for labels with a line break: egui lays the text out
    /// line by line, so ending a prefix with `\n…` can be narrower than the shorter `iii…`. The
    /// chart's label fitting must not assume otherwise (#1152).
    #[test]
    fn egui_measure_shrinks_when_a_prefix_ends_in_a_line_break() {
        let ctx = egui::Context::default();
        ctx.set_fonts(theme::pinned_font_definitions());
        let mut checked = false;
        // Fonts are ready from the second pass on.
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                let m = EguiMeasure { painter: ui.painter(), z: 1.0 };
                let (short, long) = (m.text_width("iii…", 11.0, false), m.text_width("iiii\n…", 11.0, false));
                checked = long < short;
            });
            out.textures_delta.clear();
        }
        assert!(checked);
    }

    /// STYL-04 (egui leg): the canvas paints the primitive's own colours — the resolved card and
    /// the preset ramp — and never re-derives them from the workbook theme.
    #[test]
    fn paint_prims_paints_the_resolved_card_and_ramp() {
        fn fills(out: &egui::FullOutput) -> Vec<Color32> {
            out.shapes
                .iter()
                .filter_map(|cs| match &cs.shape {
                    egui::Shape::Rect(r) => Some(r.fill),
                    _ => None,
                })
                .collect()
        }
        let mut seen = Vec::new();
        for id in [0u32, 4] {
            let style = gridcraft_chart::chart_style(id);
            let bar = gridcraft_chart::chart_style_preset(id)[0];
            let prims = vec![
                Prim::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 480.0,
                    h: 288.0,
                    fill: Some(Fill::solid(style.card)),
                    stroke: Some((style.card_border, 1.0)),
                    radii: [0.0; 4],
                },
                Prim::Rect { x: 40.0, y: 40.0, w: 60.0, h: 200.0, fill: Some(Fill::solid(bar)), stroke: None, radii: [0.0; 4] },
            ];
            let ctx = egui::Context::default();
            let mut out = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &prims, 1.0));
            let painted = fills(&out);
            out.textures_delta.clear();
            assert!(painted.contains(&rgba(style.card)), "style {id}: the card token reaches the painter");
            assert!(painted.contains(&rgba(bar)), "style {id}: the preset ramp reaches the painter");
            seen.push((rgba(style.card), rgba(bar)));
        }
        assert_ne!(seen[0], seen[1], "the dark preset paints a different surface and ramp");
    }

    /// All vertex colours of the meshes painted, in paint order. Each mesh appends its fan hub last,
    /// and the hub is not part of the outline, so the list starts and ends on the ramp stops.
    fn mesh_colours(out: &egui::FullOutput) -> Vec<Color32> {
        let mut colours = Vec::new();
        for cs in &out.shapes {
            if let egui::Shape::Mesh(m) = &cs.shape {
                let outline = m.vertices.len().saturating_sub(1);
                colours.extend(m.vertices.iter().take(outline).map(|v| v.color));
            }
        }
        colours
    }

    fn gradient_bar(fill: Fill) -> Vec<Prim> {
        vec![Prim::Rect { x: 0.0, y: 0.0, w: 20.0, h: 100.0, fill: Some(fill), stroke: None, radii: [0.0; 4] }]
    }

    /// D-01 (egui leg): the stop colours at both ends of the ramp are exact, not re-derived.
    #[test]
    fn a_gradient_bar_paints_a_mesh_with_the_exact_end_vertex_colours() {
        let (from, to) = ([200u8, 10, 10, 255], [10u8, 10, 200, 255]);
        let prims = gradient_bar(Fill::Linear { from, to, start: [0.0, 0.0], end: [0.0, 100.0] });
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &prims, 1.0));
        let colours = mesh_colours(&out);
        out.textures_delta.clear();
        // A fan over the outline interpolates a two-stop ramp exactly: every vertex takes the colour
        // its own projection gives, so the shared hub never bends the ramp away from the axis.
        assert!(colours.len() >= 4, "the fan outlines the bar, got {} vertices", colours.len());
        assert_eq!(colours.first(), Some(&rgba(from)), "the near stop is exact");
        assert_eq!(colours.last(), Some(&rgba(to)), "the far stop is exact");
        assert!(colours.iter().all(|c| *c == rgba(from) || *c == rgba(to) || (c.r() >= 10 && c.r() <= 200)), "no colour outside the ramp");
    }
    /// 02-02-04: a gradient bar keeps its rounded tips in the app. The mesh is a fan over the
    /// rounded outline, so every corner radius cuts its corner away and a vertex still sits on each
    /// arc, while the vertex colours keep the exact ramp stops.
    #[test]
    fn a_gradient_bar_keeps_every_corner_radius_in_the_mesh() {
        let (from, to) = ([200u8, 10, 10, 255], [10u8, 10, 200, 255]);
        let radii = [12.0, 0.0, 8.0, 4.0];
        let prims = vec![Prim::Rect {
            x: 0.0,
            y: 0.0,
            w: 40.0,
            h: 100.0,
            fill: Some(Fill::Linear { from, to, start: [20.0, 0.0], end: [20.0, 100.0] }),
            stroke: None,
            radii,
        }];
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &prims, 1.0));
        let verts: Vec<(Pos2, Color32)> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Mesh(m) => Some(m.vertices.iter().map(|v| (v.pos, v.color))),
                _ => None,
            })
            .flatten()
            .collect();
        out.textures_delta.clear();
        assert_eq!(corner_radius(radii, 1.0), egui::CornerRadius { nw: 12, ne: 0, se: 8, sw: 4 });

        let hub = verts.last().map(|(p, _)| *p);
        let outline: Vec<(Pos2, Color32)> = verts[..verts.len().saturating_sub(1)].to_vec();
        assert!(outline.len() > 8, "the outline is tessellated, got {} vertices", outline.len());

        // Each rounded corner: its centre and radius. A vertex of the cut-out square that the arc
        // removes would sit further than the radius from the centre, so none may.
        let arcs = [(pos2(12.0, 12.0), 12.0f32), (pos2(32.0, 92.0), 8.0), (pos2(4.0, 96.0), 4.0)];
        let inside = |p: Pos2, c: Pos2, rad: f32| (p.x - c.x).abs() <= rad && (p.y - c.y).abs() <= rad;
        for (c, rad) in arcs {
            let corner: Vec<Pos2> = outline.iter().map(|(p, _)| *p).filter(|p| inside(*p, c, rad)).collect();
            assert!(!corner.is_empty(), "corner at {c:?} with radius {rad} has no vertices");
            assert!(corner.iter().all(|p| p.distance(c) <= rad + 0.5), "corner at {c:?} keeps its square point");
            assert!(corner.iter().any(|p| (p.distance(c) - rad).abs() <= 0.5), "corner at {c:?} has no arc");
        }
        // The north-east corner takes no radius, so its square point stays: the radii are independent.
        assert!(outline.iter().any(|(p, _)| *p == pos2(40.0, 0.0)), "the square corner is still a point");
        assert!(hub.is_some(), "the fan has a hub");

        let at = |y: f32| outline.iter().filter(|(p, _)| (p.y - y).abs() < 1e-3).map(|(_, c)| *c).collect::<Vec<Color32>>();
        let (top, bottom) = (at(0.0), at(100.0));
        assert!(!top.is_empty() && !bottom.is_empty());
        assert!(top.iter().all(|c| *c == rgba(from)), "the near stop is exact at the ramp start");
        assert!(bottom.iter().all(|c| *c == rgba(to)), "the far stop is exact at the ramp end");
    }

    /// RTL sheets move a chart as a rigid object to a mirrored origin, so the vertex colours carry
    /// the ramp and never depend on where the chart sits on the sheet.
    #[test]
    fn a_gradient_inside_an_rtl_sheet_keeps_its_vertex_colours() {
        let (from, to) = ([200u8, 10, 10, 255], [10u8, 10, 200, 255]);
        let prims = gradient_bar(Fill::Linear { from, to, start: [0.0, 0.0], end: [0.0, 100.0] });
        let ctx = egui::Context::default();
        let mut lhs = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &prims, 1.0));
        let mut rhs = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(300.0, 50.0), &prims, 1.0));
        let (a, b) = (mesh_colours(&lhs), mesh_colours(&rhs));
        lhs.textures_delta.clear();
        rhs.textures_delta.clear();
        assert_eq!(a, b, "a rigid move changes no vertex colour");
        assert!(!a.is_empty());
    }

    /// The `[nw, ne, se, sw]` radii reach egui field for field, scaled by the zoom.
    #[test]
    fn per_corner_radii_map_to_corner_radius_fields() {
        let prims =
            vec![Prim::Rect { x: 0.0, y: 0.0, w: 40.0, h: 40.0, fill: Some(Fill::solid([1, 2, 3, 255])), stroke: None, radii: [4.0, 0.0, 8.0, 2.0] }];
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &prims, 2.0));
        let radii: Vec<egui::CornerRadius> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Rect(r) => Some(r.corner_radius),
                _ => None,
            })
            .collect();
        out.textures_delta.clear();
        assert_eq!(radii, vec![egui::CornerRadius { nw: 8, ne: 0, se: 16, sw: 4 }]);
    }

    /// PRIM-03 (egui leg): the shared in-code fixture reaches the canvas — the ramp's exact stops as
    /// mesh vertices, and the ring colour as the stroke of every marker hull.
    #[test]
    fn the_fixture_paints_gradient_stops_and_ringed_markers() {
        use gridcraft_chart::fixture;

        let prims = gridcraft_chart::parity_fixture();
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &prims, 1.0));
        let colours = mesh_colours(&out);
        // A filled path and its stroke are two shapes; a ring is a stroke with a width.
        let rings: Vec<Color32> = out
            .shapes
            .iter()
            .filter_map(|cs| match &cs.shape {
                egui::Shape::Circle(c) => Some((c.stroke.width, c.stroke.color)),
                egui::Shape::Path(p) => match p.stroke.color {
                    egui::epaint::ColorMode::Solid(c) => Some((p.stroke.width, c)),
                    egui::epaint::ColorMode::UV(_) => None,
                },
                _ => None,
            })
            .filter(|(w, _)| *w > 0.0)
            .map(|(_, c)| c)
            .collect();
        out.textures_delta.clear();
        assert_eq!(colours.first(), Some(&rgba(fixture::FROM)), "the near stop is exact");
        assert_eq!(colours.last(), Some(&rgba(fixture::TO)), "the far stop is exact");
        assert_eq!(rings.len(), 5, "one ring per marker hull, got {rings:?}");
        assert!(rings.iter().all(|c| *c == rgba(fixture::RING)), "every ring is the ring colour, got {rings:?}");
    }
    /// M3.6: a linear ramp on a polygon and a wedge reaches the egui mesh per vertex (solid shapes
    /// keep their flat fast path).
    #[test]
    fn a_gradient_polygon_and_wedge_keep_vertex_colours_in_the_egui_mesh() {
        use gridcraft_chart::Fill;

        let from = [200u8, 10, 10, 255];
        let to = [10u8, 10, 200, 255];
        let ramp = Fill::Linear { from, to, start: [0.0, 20.0], end: [40.0, 20.0] };

        let polygon = vec![Prim::Polygon {
            pts: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
            fill: Some(ramp),
            stroke: None,
        }];
        let wedge = vec![Prim::Wedge {
            cx: 20.0,
            cy: 20.0,
            r_outer: 20.0,
            r_inner: 8.0,
            a0: -std::f32::consts::FRAC_PI_2,
            a1: std::f32::consts::FRAC_PI_2,
            fill: Some(Fill::Linear { from, to, start: [0.0, 20.0], end: [40.0, 20.0] }),
            stroke: None,
        }];

        let ctx = egui::Context::default();
        let mut out_poly = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &polygon, 1.0));
        let poly_cols = mesh_colours(&out_poly);
        out_poly.textures_delta.clear();
        assert!(poly_cols.contains(&rgba(from)), "convex gradient polygon keeps the near stop");
        assert!(poly_cols.contains(&rgba(to)), "convex gradient polygon keeps the far stop");
        assert!(poly_cols.iter().any(|c| *c != rgba(from) && *c != rgba(to)) || poly_cols.iter().any(|c| *c != poly_cols[0]),
            "convex gradient polygon is not flat: {poly_cols:?}");

        let concave = vec![Prim::Polygon {
            pts: vec![[0.0, 0.0], [40.0, 0.0], [20.0, 20.0], [40.0, 40.0], [0.0, 40.0]],
            fill: Some(ramp),
            stroke: None,
        }];
        let mut out_concave = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &concave, 1.0));
        let concave_meshes = out_concave.shapes.iter().filter(|cs| matches!(cs.shape, egui::Shape::Mesh(_))).count();
        let concave_cols = mesh_colours(&out_concave);
        out_concave.textures_delta.clear();
        assert!(concave_meshes >= 1, "concave gradient polygon uses the mesh path");
        assert!(concave_cols.contains(&rgba(from)) && concave_cols.contains(&rgba(to)), "concave gradient polygon keeps both stops");

        let mut out_wedge = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &wedge, 1.0));
        let wedge_cols = mesh_colours(&out_wedge);
        out_wedge.textures_delta.clear();
        assert!(wedge_cols.contains(&rgba(from)) && wedge_cols.contains(&rgba(to)), "gradient wedge keeps both stops");
        assert!(wedge_cols.iter().any(|c| *c != wedge_cols[0]), "gradient wedge is not flat: {wedge_cols:?}");

        let degenerate = vec![Prim::Polygon {
            pts: vec![[0.0, 0.0], [40.0, 0.0], [40.0, 40.0], [0.0, 40.0]],
            fill: Some(Fill::Linear { from, to, start: [10.0, 10.0], end: [10.0, 10.0] }),
            stroke: None,
        }];
        let mut out_deg = ctx.run_ui(Default::default(), |ui| paint_prims(ui.painter(), pos2(0.0, 0.0), &degenerate, 1.0));
        let deg_cols = mesh_colours(&out_deg);
        out_deg.textures_delta.clear();
        assert!(!deg_cols.is_empty() && deg_cols.iter().all(|c| *c == rgba(from)), "a degenerate ramp paints flat in the near stop: {deg_cols:?}");
    }

    #[test]
    fn egui_dashes_use_the_one_dash_pattern() {
        assert_eq!(super::dash_px(2.0), (6.0, 4.0));
    }
}