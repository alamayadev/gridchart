use super::palette::{Preset, chart_style_family};
use crate::Rgba;

/// The type ladder for a chart's titles and labels, in chart-space points (tenths).
///
/// Every rung is derived from a single permille scale so the preset families keep one ratio
/// signature: [`TypeScale::DEFAULT`] is `rungs(1000)` = `(14.0, 10.0, 9.0)` bit-exactly, and the
/// deltas only scale that ladder (D-03).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeScale {
    /// Chart title.
    pub title: f32,
    /// Axis titles.
    pub axis_title: f32,
    /// Tick and category labels.
    pub tick: f32,
}

impl TypeScale {
    /// The pre-phase ladder: 14.0 / 10.0 / 9.0.
    pub const DEFAULT: Self = Self::rungs(1000);

    /// Scale the default ladder by `permille / 1000`, truncating in tenths.
    #[inline]
    pub const fn rungs(permille: u16) -> Self {
        const fn t(base: u32, permille: u16) -> f32 {
            let tenths = base * permille as u32 / 1000;
            tenths as f32 / 10.0
        }
        Self { title: t(140, permille), axis_title: t(100, permille), tick: t(90, permille) }
    }
}

/// Surface and ink a chart draws with, resolved per style id (D-01, D-02).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartStyle {
    /// Background of the whole chart, drawn as `prims[0]`.
    pub card: Rgba,
    /// Hairline around [`Self::card`].
    pub card_border: Rgba,
    /// Titles, axis titles, legend text and data labels drawn on the card.
    pub text: Rgba,
    /// Gridlines: colour and weight.
    pub grid: (Rgba, f32),
    /// Axis lines and ticks.
    pub axis: Rgba,
    /// Candle bodies on stock charts.
    pub stock: Rgba,
    /// True when [`Self::card`] is dark, so ink on it is light.
    pub dark: bool,
    /// Reserved plot-area fill; equals [`Self::card`] (no plan in phase 3 paints it).
    pub plot: Rgba,
    /// The solid baseline: colour and weight, drawn after the gridlines.
    pub baseline: (Rgba, f32),
    /// Ring around a marker hull: the card colour on every preset, so it never halos.
    pub marker_ring: (Rgba, f32),
    /// Seam between slices/wedges: the card colour on every preset.
    pub slice_seam: (Rgba, f32),
    /// The type ladder for this preset.
    pub type_scale: TypeScale,
}

const fn style(
    card: Rgba, card_border: Rgba, text: Rgba, grid: (Rgba, f32), axis: Rgba, stock: Rgba, dark: bool,
    baseline: (Rgba, f32), marker_ring_w: f32, slice_seam_w: f32, type_scale: TypeScale,
) -> ChartStyle {
    ChartStyle {
        card,
        card_border,
        text,
        grid,
        axis,
        stock,
        dark,
        plot: card,
        baseline,
        marker_ring: (card, marker_ring_w),
        slice_seam: (card, slice_seam_w),
        type_scale,
    }
}

impl Default for ChartStyle {
    fn default() -> Self {
        DEFAULT
    }
}

const DEFAULT: ChartStyle = style(
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xD9, 0xD9, 0xD9, 0xFF],
    [0x59, 0x59, 0x59, 0xFF],
    ([0xD9, 0xD9, 0xD9, 0xFF], 0.75),
    [0xBF, 0xBF, 0xBF, 0xFF],
    [0x40, 0x40, 0x40, 0xFF],
    false,
    ([0xA6, 0xA6, 0xA6, 0xFF], 1.25),
    0.75,
    1.5,
    TypeScale::DEFAULT,
);

const VIBRANT: ChartStyle = style(
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xD9, 0xD9, 0xD9, 0xFF],
    [0x59, 0x59, 0x59, 0xFF],
    ([0xD9, 0xD9, 0xD9, 0xFF], 0.75),
    [0xBF, 0xBF, 0xBF, 0xFF],
    [0x40, 0x40, 0x40, 0xFF],
    false,
    ([0xA6, 0xA6, 0xA6, 0xFF], 1.25),
    0.75,
    1.5,
    TypeScale::rungs(920),
);

const MINIMAL: ChartStyle = style(
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xD9, 0xD9, 0xD9, 0xFF],
    [0x59, 0x59, 0x59, 0xFF],
    ([0xD9, 0xD9, 0xD9, 0xFF], 0.55),
    [0xBF, 0xBF, 0xBF, 0xFF],
    [0x40, 0x40, 0x40, 0xFF],
    false,
    ([0xA6, 0xA6, 0xA6, 0xFF], 1.25),
    0.60,
    1.0,
    TypeScale::rungs(1080),
);

/// Deep neutral card with light ink, for the presets built to sit on it.
pub const DARK: ChartStyle = style(
    [0x1B, 0x22, 0x2E, 0xFF],
    [0x3A, 0x44, 0x54, 0xFF],
    [0xE6, 0xE9, 0xEE, 0xFF],
    ([0x33, 0x3B, 0x47, 0xD9], 0.90),
    [0x59, 0x63, 0x72, 0xFF],
    [0xC8, 0xCD, 0xD4, 0xFF],
    true,
    ([0x8A, 0x94, 0xA4, 0xFF], 1.25),
    1.00,
    2.0,
    TypeScale::DEFAULT,
);

const PASTEL: ChartStyle = style(
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xD9, 0xD9, 0xD9, 0xFF],
    [0x59, 0x59, 0x59, 0xFF],
    ([0xD9, 0xD9, 0xD9, 0xE6], 0.70),
    [0xBF, 0xBF, 0xBF, 0xFF],
    [0x40, 0x40, 0x40, 0xFF],
    false,
    ([0xA6, 0xA6, 0xA6, 0xFF], 1.25),
    0.85,
    1.5,
    TypeScale::rungs(960),
);

const TINTED: ChartStyle = style(
    [0xFF, 0xFF, 0xFF, 0xFF],
    [0xD9, 0xD9, 0xD9, 0xFF],
    [0x59, 0x59, 0x59, 0xFF],
    ([0xD9, 0xD9, 0xD9, 0xE6], 0.75),
    [0xBF, 0xBF, 0xBF, 0xFF],
    [0x40, 0x40, 0x40, 0xFF],
    false,
    ([0xA6, 0xA6, 0xA6, 0xFF], 1.25),
    0.85,
    1.5,
    TypeScale::DEFAULT,
);

/// The resolved tokens for a style ID, on the same ring as
/// [`chart_style_preset`](super::palette::chart_style_preset).
#[inline]
pub fn chart_style(id: u32) -> ChartStyle {
    match chart_style_family(id) {
        Preset::Default => DEFAULT,
        Preset::Vibrant => VIBRANT,
        Preset::Minimal => MINIMAL,
        Preset::Dark => DARK,
        Preset::Pastel => PASTEL,
        Preset::Tinted => TINTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tokens_are_the_colours_charts_always_drew_with() {
        let s = chart_style(0);
        assert_eq!(s, ChartStyle::default());
        assert_eq!(s.card, [0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(s.card_border, [0xD9, 0xD9, 0xD9, 0xFF]);
        assert_eq!(s.text, [0x59, 0x59, 0x59, 0xFF]);
        assert_eq!(s.grid.0, [0xD9, 0xD9, 0xD9, 0xFF]);
        assert_eq!(s.grid.1.to_bits(), 0.75f32.to_bits());
        assert_eq!(s.axis, [0xBF, 0xBF, 0xBF, 0xFF]);
        assert_eq!(s.stock, [0x40, 0x40, 0x40, 0xFF]);
        assert!(!s.dark);
        assert_eq!(s.plot, s.card);
        assert_eq!(s.baseline.0, [0xA6, 0xA6, 0xA6, 0xFF]);
        assert_eq!(s.baseline.1.to_bits(), 1.25f32.to_bits());
        assert_eq!(chart_style(1), s, "1 is the same polished default as 0");
    }

    #[test]
    fn only_the_dark_family_flips_the_surface() {
        for id in [4u32, 10, 22, 46] {
            let s = chart_style(id);
            assert!(s.dark, "id {id} is the dark preset");
            assert_eq!(s.card, DARK.card, "id {id} uses the dark card");
        }
        for id in [0u32, 1, 2, 3, 5, 6, 7, 21, 47, 48] {
            assert!(!chart_style(id).dark, "id {id} stays light");
        }
    }

    #[test]
    fn above_range_falls_back_to_the_default_surface() {
        assert_eq!(chart_style(49), ChartStyle::default());
        assert_eq!(chart_style(u32::MAX), ChartStyle::default());
    }

    /// Every preset family resolves to its own authored tokens, and no two are identical.
    #[test]
    fn every_preset_family_resolves_to_its_own_tokens() {
        let ids = [0u32, 2, 3, 4, 5, 6];
        let styles: Vec<ChartStyle> = ids.iter().map(|&id| chart_style(id)).collect();
        assert_eq!(chart_style(0), DEFAULT);
        assert_eq!(chart_style(2), VIBRANT);
        assert_eq!(chart_style(3), MINIMAL);
        assert_eq!(chart_style(4), DARK);
        assert_eq!(chart_style(5), PASTEL);
        assert_eq!(chart_style(6), TINTED);
        for (i, a) in styles.iter().enumerate() {
            for (j, b) in styles.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "preset ids {:?} and {:?} must differ", ids[i], ids[j]);
                }
            }
        }
        assert_eq!(chart_style(0).marker_ring, (crate::render::WHITE, crate::render::MARKER_RING_WEIGHT));
        assert_eq!(chart_style(0).slice_seam, (crate::render::WHITE, 1.5));
    }

    /// The default preset keeps today's type rungs bit-exactly.
    #[test]
    fn the_default_preset_keeps_todays_type_rungs() {
        let t = chart_style(0).type_scale;
        assert_eq!(t, TypeScale::DEFAULT);
        assert_eq!(t.title.to_bits(), 14.0f32.to_bits());
        assert_eq!(t.axis_title.to_bits(), 10.0f32.to_bits());
        assert_eq!(t.tick.to_bits(), 9.0f32.to_bits());
    }

    /// A preset ladder scales title, axis title and tick together.
    #[test]
    fn a_preset_ladder_scales_title_axis_title_and_tick_together() {
        let t = TypeScale::rungs(920);
        assert_eq!(t.title.to_bits(), 12.8f32.to_bits());
        assert_eq!(t.axis_title.to_bits(), 9.2f32.to_bits());
        assert_eq!(t.tick.to_bits(), 8.2f32.to_bits());
        assert_eq!(chart_style(2).type_scale, t);
        let m = TypeScale::rungs(1080);
        assert_eq!(m.title.to_bits(), 15.1f32.to_bits());
        assert_eq!(m.axis_title.to_bits(), 10.8f32.to_bits());
        assert_eq!(m.tick.to_bits(), 9.7f32.to_bits());
    }

    /// The ring and seam are the card colour on every preset (no dark halo).
    #[test]
    fn the_ring_and_seam_are_the_card_on_every_preset() {
        for id in 0..=48u32 {
            let s = chart_style(id);
            assert_eq!(s.marker_ring.0, s.card, "id {id}: ring must be the card");
            assert_eq!(s.slice_seam.0, s.card, "id {id}: seam must be the card");
            assert_eq!(s.plot, s.card, "id {id}: plot is reserved as the card");
        }
    }

    /// Ink must stay readable on the card it is drawn on, and grid/baseline must separate from it.
    #[test]
    fn every_style_id_has_readable_ink_on_its_card() {
        let luma = |c: Rgba| 0.299 * f64::from(c[0]) + 0.587 * f64::from(c[1]) + 0.114 * f64::from(c[2]);
        for id in 0..=48u32 {
            let s = chart_style(id);
            assert!((luma(s.text) - luma(s.card)).abs() >= 80.0, "id {id}: text on card is unreadable");
            assert!((luma(s.axis) - luma(s.card)).abs() >= 30.0, "id {id}: axis on card is unreadable");
            assert!((luma(s.baseline.0) - luma(s.card)).abs() >= 30.0, "id {id}: baseline on card is unreadable");
            assert!((luma(s.grid.0) - luma(s.card)).abs() >= 16.0, "id {id}: grid must separate from card");
        }
    }
}
