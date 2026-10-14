//! Color normalization: named colors, aliases, hex and CSS colors in, an
//! SVG-ready color string and its foreground/shadow pair out.

use crate::small_str::SmallStr;
use csscolorparser::Color;
use std::str::FromStr;

/// shields.io named color palette
fn named_color_hex(name: &str) -> Option<&'static str> {
    Some(match name {
        "brightgreen" => "#4c1",
        "green" => "#97ca00",
        "yellow" => "#dfb317",
        "yellowgreen" => "#a4a61d",
        "orange" => "#fe7d37",
        "red" => "#e05d44",
        "blue" => "#007ec6",
        "grey" => "#555",
        "lightgrey" => "#9f9f9f",
        _ => return None,
    })
}

/// Aliases resolving to named colors
fn alias_target(name: &str) -> Option<&'static str> {
    Some(match name {
        "gray" => "grey",
        "lightgray" => "lightgrey",
        "critical" => "red",
        "important" => "orange",
        "success" => "brightgreen",
        "informational" => "blue",
        "inactive" => "lightgrey",
        _ => return None,
    })
}

/// 3/6 digit hex, with any number of leading `#`.
pub(crate) fn is_valid_hex(s: &str) -> bool {
    let s = s.trim_start_matches('#');
    let len = s.len();
    (len == 3 || len == 6) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A color normalized for SVG output.
pub(crate) struct SvgColor<'a> {
    /// The value written into `fill="…"`. Hex needs at most 7 bytes and CSS
    /// colors rarely pass 24; longer ones fall back to the heap.
    pub value: SmallStr<'a, 24>,
    /// RGBA known from parsing, so the text color does not parse the string a
    /// second time. `None` only for the crate's own fixed colors.
    rgba: Option<[u8; 4]>,
}

impl<'a> SvgColor<'a> {
    /// A hex color the crate itself supplies (defaults and fallbacks).
    pub fn fixed(hex: &'static str) -> Self {
        debug_assert!(is_valid_hex(hex));
        SvgColor {
            value: SmallStr::Static(hex),
            rgba: None,
        }
    }

    /// Named colors and aliases become their hex value, hex strings are
    /// normalized to a leading `#`, other valid CSS colors pass through
    /// lowercased. Returns `None` for invalid input.
    pub fn parse(color: &'a str) -> Option<Self> {
        let color = color.trim();
        if color.is_empty() {
            return None;
        }
        // Hex is by far the most common input. No named color or alias is
        // spelled with hex digits only, so recognizing it first changes nothing.
        if is_valid_hex(color) {
            let lower = SmallStr::ascii_lowercase(color);
            let value = if color.starts_with('#') {
                lower
            } else {
                SmallStr::hash_prefixed(&lower)
            };
            let [r, g, b] = hex_rgb(color);
            return Some(SvgColor {
                value,
                rgba: Some([r, g, b, 255]),
            });
        }
        let lower = SmallStr::ascii_lowercase(color);
        if let Some(hex) = named_color_hex(&lower) {
            return Some(Self::fixed(hex));
        }
        if let Some(alias) = alias_target(&lower) {
            return named_color_hex(alias).map(Self::fixed);
        }
        let parsed = Color::from_str(&lower).ok()?;
        Some(SvgColor {
            value: lower,
            rgba: Some(parsed.to_rgba8()),
        })
    }

    /// Whether text on this background is dark (`#333` on a `#ccc` shadow)
    /// rather than the default white (on `#010101`); see
    /// [`colors_for_background`].
    pub fn dark_text(&self) -> bool {
        match self.rgba {
            // `Color::to_css_hex` writes 8 digits when there is alpha, which
            // `colors_for_background` does not parse and treats as black.
            Some([_, _, _, a]) if a < 255 => false,
            Some([r, g, b, _]) => is_bright(r, g, b),
            None => is_bright_hex(&self.value),
        }
    }
}

/// Dynamically calculates foreground and shadow colors based on background color (equivalent to JS colorsForBackground)
///
/// - Input: hex color string (supports 3/6 digits, e.g. "#4c1", "#007ec6")
/// - Algorithm:
///   1. Parses hex to RGB
///   2. Calculates brightness = (0.299*R + 0.587*G + 0.114*B) / 255
///   3. If brightness ≤ 0.69, returns ("#fff", "#010101"), otherwise ("#333", "#ccc")
pub fn colors_for_background(hex: &str) -> (&'static str, &'static str) {
    if is_bright_hex(hex) {
        ("#333", "#ccc")
    } else {
        ("#fff", "#010101")
    }
}

fn is_bright_hex(hex: &str) -> bool {
    let [r, g, b] = hex_rgb(hex);
    is_bright(r, g, b)
}

/// RGB of a 3/6 digit hex color with any number of leading `#`; anything
/// else is black.
fn hex_rgb(hex: &str) -> [u8; 3] {
    match *hex.trim_start_matches('#').as_bytes() {
        [r, g, b] => [expand_nibble(r), expand_nibble(g), expand_nibble(b)],
        [r1, r2, g1, g2, b1, b2] => [byte(r1, r2), byte(g1, g2), byte(b1, b2)],
        _ => [0, 0, 0],
    }
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

/// Expands a single hex digit to a full byte, e.g. 'c' -> 0xcc; invalid digits count as 0
fn expand_nibble(c: u8) -> u8 {
    let v = hex_digit(c).unwrap_or(0);
    (v << 4) | v
}

/// Two hex digits as a byte, the way `u8::from_str_radix(.., 16)` reads them —
/// including its acceptance of a leading `+` — and 0 when that would fail.
fn byte(hi: u8, lo: u8) -> u8 {
    match (hi, hex_digit(hi), hex_digit(lo)) {
        (_, Some(h), Some(l)) => h << 4 | l,
        (b'+', None, Some(l)) => l,
        _ => 0,
    }
}

fn is_bright(r: u8, g: u8, b: u8) -> bool {
    // W3C recommended brightness formula
    let brightness = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
    brightness > 0.69
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_for_background_does_not_panic_on_multibyte_input() {
        // Six bytes but two characters: slicing it as hex used to panic.
        assert_eq!(colors_for_background("中文"), ("#fff", "#010101"));
        assert_eq!(colors_for_background("#中"), ("#fff", "#010101"));
    }

    #[test]
    fn byte_matches_from_str_radix() {
        for hi in 0..=255u8 {
            for lo in 0..=255u8 {
                let pair = [hi, lo];
                let Ok(s) = std::str::from_utf8(&pair) else {
                    continue;
                };
                assert_eq!(
                    byte(hi, lo),
                    u8::from_str_radix(s, 16).unwrap_or(0),
                    "{s:?}"
                );
            }
        }
    }
}
