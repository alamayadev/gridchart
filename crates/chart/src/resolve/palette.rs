//! Clean-room, original chart preset palettes. No vendor hexes; no imports.
//!
//! Palette contract (gridcraft-chart style system): each preset is exactly six base colours.
//! `series_color`-style tinting (`CYCLE_TINTS`) still applies for series beyond six, per D-02.
//! `PALETTE_CYCLE = 36` is unchanged.

use crate::Rgba;

/// Canonical polished default palette: warm, high-contrast, publication-grade blues/greys.
pub const PALETTE_DEFAULT: [Rgba; 6] = [
    [0x1F, 0x6F, 0xD9, 0xFF],
    [0x59, 0xB8, 0x57, 0xFF],
    [0xE8, 0x7B, 0x38, 0xFF],
    [0x8E, 0x44, 0xAD, 0xFF],
    [0x33, 0x85, 0xA8, 0xFF],
    [0xD1, 0x6A, 0x6A, 0xFF],
];

/// Vibrant: saturated primaries + deep neutrals, maximally punchy.
pub const PALETTE_VIBRANT: [Rgba; 6] = [
    [0xF2, 0x38, 0x38, 0xFF],
    [0xF2, 0xB5, 0x17, 0xFF],
    [0x1E, 0x8B, 0x4A, 0xFF],
    [0x2E, 0x6F, 0xB8, 0xFF],
    [0x9B, 0x59, 0xB6, 0xFF],
    [0xE9, 0x6C, 0xAA, 0xFF],
];

/// Minimal: monochrome greys, one accent for emphasis, airiest contrast.
pub const PALETTE_MINIMAL: [Rgba; 6] = [
    [0x66, 0x66, 0x66, 0xFF],
    [0x8C, 0x8C, 0x8C, 0xFF],
    [0xB3, 0xB3, 0xB3, 0xFF],
    [0x3D, 0x5A, 0x80, 0xFF],
    [0xA7, 0xC4, 0xDE, 0xFF],
    [0xD9, 0xD9, 0xD9, 0xFF],
];

/// Dark: deep ramps built to read on a dark card.
pub const PALETTE_DARK: [Rgba; 6] = [
    [0x64, 0xC7, 0xF7, 0xFF],
    [0x84, 0xD4, 0x99, 0xFF],
    [0xFF, 0xB9, 0x6F, 0xFF],
    [0xDD, 0x9F, 0xDE, 0xFF],
    [0x9B, 0xD4, 0xEA, 0xFF],
    [0xFF, 0x9F, 0xB0, 0xFF],
];

/// Pastel: low-saturation tints, no colour strain at small sizes.
pub const PALETTE_PASTEL: [Rgba; 6] = [
    [0xA8, 0xD8, 0xEA, 0xFF],
    [0xB9, 0xF2, 0xC3, 0xFF],
    [0xFF, 0xD8, 0xA8, 0xFF],
    [0xE8, 0xBB, 0xE8, 0xFF],
    [0xFF, 0xE8, 0xB6, 0xFF],
    [0xD7, 0xB5, 0xDE, 0xFF],
];

/// Tinted: one hue (blue family), monochromatic ramp — single-axis emphasis.
pub const PALETTE_TINTED: [Rgba; 6] = [
    [0x2E, 0x75, 0xB6, 0xFF],
    [0x4A, 0x90, 0xD9, 0xFF],
    [0x6C, 0xA8, 0xE8, 0xFF],
    [0x94, 0xC0, 0xF2, 0xFF],
    [0xBB, 0xD8, 0xFA, 0xFF],
    [0xDF, 0xEE, 0xFB, 0xFF],
];

/// The canonical polished default (same table the UI shows).
pub const PALETTE_DEFAULT_TABLE: [Rgba; 6] = PALETTE_DEFAULT;

/// The preset family a style ID selects. One ring, one source of truth: both the series ramp
/// ([`chart_style_preset`]) and the surface tokens ([`super::style::chart_style`]) read it, so a
/// chart's card and its colours always come from the same preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    Default,
    Vibrant,
    Minimal,
    Dark,
    Pastel,
    Tinted,
}

/// Resolve the preset family for a style ID.
///
/// `0`, `1` and `2` map to the canonical polished default. Pre-preset GridCraft wrote
/// `style = 2` into every imported chart (its old import default), so legacy workbooks
/// must reopen on the default, not silently flip palettes.
/// `3..=48` map into the preset ring: the six families repeated every six IDs.
/// `49` and above fall back to the canonical polished default.
#[inline]
pub fn chart_style_family(id: u32) -> Preset {
    match id {
        0..=2 => Preset::Default,
        49.. => Preset::Default,
        _ => match (id - 1) % 6 {
            1 => Preset::Vibrant,
            2 => Preset::Minimal,
            3 => Preset::Dark,
            4 => Preset::Pastel,
            5 => Preset::Tinted,
            _ => Preset::Default,
        },
    }
}

/// Resolve the palette table for a style ID.
#[inline]
pub fn chart_style_preset(id: u32) -> &'static [Rgba; 6] {
    match chart_style_family(id) {
        Preset::Default => &PALETTE_DEFAULT_TABLE,
        Preset::Vibrant => &PALETTE_VIBRANT,
        Preset::Minimal => &PALETTE_MINIMAL,
        Preset::Dark => &PALETTE_DARK,
        Preset::Pastel => &PALETTE_PASTEL,
        Preset::Tinted => &PALETTE_TINTED,
    }
}

/// The total number of selectable preset style IDs.
#[inline]
pub fn chart_style_total() -> u32 {
    48
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_total_is_48() {
        assert_eq!(chart_style_total(), 48);
    }

    #[test]
    fn zero_one_and_two_are_the_canonical_default() {
        assert!(chart_style_preset(0) == &PALETTE_DEFAULT_TABLE);
        assert!(chart_style_preset(1) == &PALETTE_DEFAULT_TABLE);
        // Legacy: pre-preset GridCraft persisted style = 2 as its import default.
        // It must reopen on the default, never silently flip palettes (WR-01).
        assert!(chart_style_preset(2) == &PALETTE_DEFAULT_TABLE);
        assert_eq!(chart_style_family(2), Preset::Default);
    }

    #[test]
    fn valid_preset_range_maps_to_a_table() {
        let r1 = chart_style_preset(1);
        let r6 = chart_style_preset(6);
        let r48 = chart_style_preset(48);
        // non-empty: a valid table has 6 elements and is not all-zero
        assert_eq!(r1.len(), 6);
        assert!(r1[0][0] != 0 || r1[0][1] != 0);
        assert_eq!(r6.len(), 6);
        assert_eq!(r48.len(), 6);
    }

    #[test]
    fn above_range_falls_back_not_panics() {
        assert!(chart_style_preset(49) == &PALETTE_DEFAULT_TABLE);
        assert!(chart_style_preset(u32::MAX) == &PALETTE_DEFAULT_TABLE);
    }

    #[test]
    fn ring_repeats_every_six() {
        // ID `1`, `2` = DEFAULT (2 is the legacy import default);
        // `3..=8` = MINIMAL, DARK, PASTEL, TINTED, DEFAULT, VIBRANT; `9..` repeats.
        assert_eq!(chart_style_preset(1), chart_style_preset(7));
        assert_eq!(chart_style_preset(6), chart_style_preset(12));
        assert_eq!(chart_style_preset(48), chart_style_preset(42));
    }

    #[test]
    fn family_matches_the_table_it_resolves_to() {
        let table = |p: Preset| match p {
            Preset::Default => &PALETTE_DEFAULT_TABLE,
            Preset::Vibrant => &PALETTE_VIBRANT,
            Preset::Minimal => &PALETTE_MINIMAL,
            Preset::Dark => &PALETTE_DARK,
            Preset::Pastel => &PALETTE_PASTEL,
            Preset::Tinted => &PALETTE_TINTED,
        };
        for id in 0..=60u32 {
            assert_eq!(chart_style_preset(id), table(chart_style_family(id)), "id {id}");
        }
        assert_eq!(chart_style_family(4), Preset::Dark);
        assert_eq!(chart_style_family(10), Preset::Dark);
        assert_eq!(chart_style_family(46), Preset::Dark);
        assert_eq!(chart_style_family(49), Preset::Default);
        assert_eq!(chart_style_family(u32::MAX), Preset::Default);
    }
}
