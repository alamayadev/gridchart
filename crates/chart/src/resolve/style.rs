//! Resolved chart style tokens: the surface and ink one chart draws with.
//!
//! The default values are exactly the light colours the chart has always drawn with, so a chart on
//! the default preset renders byte-identically to before this module existed. A preset whose ramps
//! are built for a dark card ([`Preset::Dark`]) flips the whole surface, because light ink on a
//! white card is unreadable (D-01, D-04).

use super::palette::{Preset, chart_style_family};
use crate::Rgba;

/// Surface and ink a chart draws with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChartStyle {
    /// Background of the whole chart, drawn as `prims[0]`.
    pub card: Rgba,
    /// Hairline around [`Self::card`].
    pub card_border: Rgba,
    /// Titles, axis titles, legend text and data labels drawn on the card.
    pub text: Rgba,
    /// Gridlines.
    pub grid: Rgba,
    /// Axis lines and ticks.
    pub axis: Rgba,
    /// Candle bodies on stock charts.
    pub stock: Rgba,
    /// True when [`Self::card`] is dark, so ink on it is light.
    pub dark: bool,
}

impl Default for ChartStyle {
    fn default() -> Self {
        Self {
            card: [0xFF, 0xFF, 0xFF, 0xFF],
            card_border: [0xD9, 0xD9, 0xD9, 0xFF],
            text: [0x59, 0x59, 0x59, 0xFF],
            grid: [0xD9, 0xD9, 0xD9, 0xFF],
            axis: [0xBF, 0xBF, 0xBF, 0xFF],
            stock: [0x40, 0x40, 0x40, 0xFF],
            dark: false,
        }
    }
}

/// Deep neutral card with light ink, for the presets built to sit on it.
pub const DARK: ChartStyle = ChartStyle {
    card: [0x1B, 0x22, 0x2E, 0xFF],
    card_border: [0x3A, 0x44, 0x54, 0xFF],
    text: [0xE6, 0xE9, 0xEE, 0xFF],
    grid: [0x33, 0x3B, 0x47, 0xFF],
    axis: [0x59, 0x63, 0x72, 0xFF],
    stock: [0xC8, 0xCD, 0xD4, 0xFF],
    dark: true,
};

/// The resolved tokens for a style ID, on the same ring as
/// [`chart_style_preset`](super::palette::chart_style_preset).
#[inline]
pub fn chart_style(id: u32) -> ChartStyle {
    match chart_style_family(id) {
        Preset::Dark => DARK,
        _ => ChartStyle::default(),
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
        assert_eq!(s.grid, [0xD9, 0xD9, 0xD9, 0xFF]);
        assert_eq!(s.axis, [0xBF, 0xBF, 0xBF, 0xFF]);
        assert_eq!(s.stock, [0x40, 0x40, 0x40, 0xFF]);
        assert!(!s.dark);
        assert_eq!(chart_style(1), s, "1 is the same polished default as 0");
    }

    #[test]
    fn only_the_dark_family_flips_the_surface() {
        for id in [4u32, 10, 22, 46] {
            assert_eq!(chart_style(id), DARK, "id {id} is the dark preset");
        }
        for id in [0u32, 1, 2, 3, 5, 6, 7, 21, 47, 48] {
            assert_eq!(chart_style(id), ChartStyle::default(), "id {id} stays light");
        }
    }

    #[test]
    fn above_range_falls_back_to_the_default_surface() {
        assert_eq!(chart_style(49), ChartStyle::default());
        assert_eq!(chart_style(u32::MAX), ChartStyle::default());
    }

    /// Ink must stay readable on the card it is drawn on: a dark preset that kept the light-theme
    /// ink would render as an unreadable block.
    #[test]
    fn every_style_id_has_readable_ink_on_its_card() {
        let luma = |c: Rgba| 0.299 * f64::from(c[0]) + 0.587 * f64::from(c[1]) + 0.114 * f64::from(c[2]);
        for id in 0..=48u32 {
            let s = chart_style(id);
            assert!((luma(s.text) - luma(s.card)).abs() >= 80.0, "id {id}: text on card is unreadable");
            assert!((luma(s.axis) - luma(s.card)).abs() >= 30.0, "id {id}: axis on card is unreadable");
        }
    }
}
