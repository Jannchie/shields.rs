#![doc = r#"
# shields

A Rust library for generating SVG badges, inspired by [shields.io](https://shields.io/).

This crate provides flexible APIs for creating customizable status badges for CI, version, downloads, and more, supporting multiple styles (flat, plastic, social, for-the-badge, etc.).

## Features

- Generate SVG badge strings with custom label, message, color, logo, and links.
- Multiple badge styles: flat, flat-square, plastic, social, for-the-badge.
- Accurate text width calculation using font width tables embedded at compile time.
- Builder pattern and parameter struct APIs.
- Color normalization and aliasing (e.g., "critical" → red).
- No runtime file I/O required for badge generation.

### Example

```rust
use shields::{BadgeStyle, BadgeParams, render_badge_svg};

let params = BadgeParams {
    style: BadgeStyle::Flat,
    label: Some("build"),
    message: Some("passing"),
    label_color: Some("green"),
    message_color: Some("brightgreen"),
    link: Some("https://ci.example.com"),
    extra_link: None,
    logo: None,
    logo_color: None,
};
let svg = render_badge_svg(&params);
assert!(svg.contains("passing"));
```

Or use the builder API:

```rust
use shields::{BadgeStyle};
use shields::builder::Badge;

let svg = Badge::style(BadgeStyle::Plastic)
    .label("version")
    .message("1.0.0")
    .logo("github")
    .build();
assert!(svg.contains("version"));
```

See [`BadgeParams`](crate::BadgeParams), [`BadgeStyle`](crate::BadgeStyle), and [`BadgeBuilder`](crate::builder::BadgeBuilder) for details.

"#]
use askama::Template;
use std::borrow::Cow;
use std::str::FromStr;
pub mod builder;
pub mod measurer;
mod xml_escape;
use base64::Engine;
use color_util::to_svg_color;
use csscolorparser::Color;
use serde::Deserialize;

/// Font width tables generated at build time from `assets/fonts/*.json`.
mod font_tables {
    // Generated width data contains literals that happen to look like math constants
    #![allow(clippy::approx_constant)]
    include!(concat!(env!("OUT_DIR"), "/font_tables.rs"));
}

/// SVG rendering template context, fields must correspond to variables in badge_svg_template_askama.svg
#[derive(Template)]
#[template(path = "flat_badge_template.min.svg", escape = "svg")]
struct FlatBadgeSvgTemplateContext<'a> {
    logo_width: u32,
    total_width: i32,
    id_suffix: &'a str,
    badge_height: i32,
    accessible_text: &'a str,
    left_width: i32,
    right_width: i32,
    label_color: &'a str,
    message_color: &'a str,
    font_family: &'a str,
    font_size_scaled: i32,

    label: &'a str,
    label_x: f32,
    label_width_scaled: i32,
    label_text_color: &'a str,
    label_shadow_color: &'a str,

    message: &'a str,
    message_x: f32,
    message_shadow_color: &'a str,
    message_text_color: &'a str,
    message_width_scaled: i32,

    link: &'a str,
    extra_link: &'a str,

    logo: &'a str,
    rect_offset: i32,

    message_link_x: i32,
}
/// flat-square SVG rendering template context
#[derive(Template)]
#[template(path = "flat_square_badge_template.min.svg", escape = "svg")]
struct FlatSquareBadgeSvgTemplateContext<'a> {
    logo_width: u32,
    total_width: i32,
    badge_height: i32,
    accessible_text: &'a str,
    left_width: i32,
    right_width: i32,
    label_color: &'a str,
    message_color: &'a str,
    font_family: &'a str,
    font_size_scaled: i32,

    label: &'a str,
    label_x: f32,
    label_width_scaled: i32,
    label_text_color: &'a str,

    message: &'a str,
    message_x: f32,
    message_text_color: &'a str,
    message_width_scaled: i32,

    link: &'a str,
    extra_link: &'a str,
    logo: &'a str,
    rect_offset: i32,

    message_link_x: i32,
}
/// plastic SVG rendering template context
#[derive(Template)]
#[template(path = "plastic_badge_template.min.svg", escape = "svg")]
struct PlasticBadgeSvgTemplateContext<'a> {
    logo_width: u32,
    total_width: i32,
    id_suffix: &'a str,
    accessible_text: &'a str,
    left_width: i32,
    right_width: i32,
    // gradient
    label: &'a str,
    label_x: f32,
    label_text_length: i32,
    label_text_color: &'a str,
    label_shadow_color: &'a str,
    message: &'a str,
    message_x: f32,
    message_text_length: i32,
    message_text_color: &'a str,
    message_shadow_color: &'a str,
    label_color: &'a str,
    message_color: &'a str,

    link: &'a str,
    extra_link: &'a str,

    logo: &'a str,
    rect_offset: i32,

    message_link_x: i32,
}

/// social SVG rendering template context
#[derive(Template)]
#[template(path = "social_badge_template.min.svg", escape = "svg")]
struct SocialBadgeSvgTemplateContext<'a> {
    logo_width: u32,
    total_width: i32,
    id_suffix: &'a str,
    total_height: i32,
    internal_height: u32,
    accessible_text: &'a str,
    label_rect_width: i32,
    message_bubble_main_x: f32,
    message_rect_width: u32,
    message_bubble_notch_x: i32,
    label_text_x: f32,
    label_text_length: u32,
    label: &'a str,
    message_text_x: f32,
    message_text_length: u32,
    message: &'a str,

    link: &'a str,
    extra_link: &'a str,

    logo: &'a str,
}

/// for-the-badge SVG rendering template context
#[derive(Template)]
#[template(path = "for_the_badge_template.min.svg", escape = "svg")]
struct ForTheBadgeSvgTemplateContext<'a> {
    logo_width: u32,
    // SVG dimensions (upstream keeps fractional widths for this style)
    total_width: f64,

    // Accessibility
    accessible_text: &'a str,

    // Layout dimensions
    has_label_rect: bool,
    left_width: f64,
    right_width: f64,

    // Colors
    label_color: &'a str,
    message_color: &'a str,

    // Font settings
    font_family: &'a str,
    font_size: i32,

    // Label (left side)
    label: &'a str,
    label_x: f64,
    label_width_scaled: f64,
    label_text_color: &'a str,

    // Message (right side)
    message: &'a str,
    message_x: f64,
    message_text_color: &'a str,
    message_width_scaled: f64,

    // Links
    link: &'a str,
    extra_link: &'a str,

    // Logo
    logo: &'a str,
    logo_x: f64,
}

// --- Color processing utility module ---
// Supports standardization and SVG output of named colors, aliases, hex, and CSS color inputs

mod color_util {
    use csscolorparser::Color;
    use std::borrow::Cow;
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

    // 3/6 digit hex validation
    pub fn is_valid_hex(s: &str) -> bool {
        let s = s.trim_start_matches('#');
        let len = s.len();
        (len == 3 || len == 6) && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Outputs an SVG-compatible color: named colors and aliases become their hex value,
    /// hex strings are normalized to a leading `#`, other valid CSS colors pass through
    /// lowercased. Returns `None` for invalid input.
    pub fn to_svg_color(color: &str) -> Option<Cow<'_, str>> {
        let color = color.trim();
        if color.is_empty() {
            return None;
        }
        // Callers pass an already-lowercase color most of the time (`#4c1`, `blue`),
        // and named colors resolve to static hex, so the common paths never allocate.
        let lower = if color.bytes().any(|b| b.is_ascii_uppercase()) {
            Cow::Owned(color.to_ascii_lowercase())
        } else {
            Cow::Borrowed(color)
        };
        if let Some(hex) = named_color_hex(&lower) {
            return Some(Cow::Borrowed(hex));
        }
        if let Some(alias) = alias_target(&lower) {
            return named_color_hex(alias).map(Cow::Borrowed);
        }
        if is_valid_hex(&lower) {
            return Some(if lower.starts_with('#') {
                lower
            } else {
                Cow::Owned(format!("#{lower}"))
            });
        }
        if Color::from_str(&lower).is_ok() {
            return Some(lower);
        }
        None
    }
}
/// Font width calculation trait, to be implemented and injected by the main project
pub trait FontMetrics {
    /// Supports font-family fallback
    fn get_text_width_px(&self, text: &str, font_family: &str) -> f32;
}

/// Font enumeration for supported fonts
#[derive(Eq, PartialEq, Hash, Clone, Copy, Debug)]
pub enum Font {
    /// Verdana 11px Normal
    VerdanaNormal11,
    /// Helvetica 11px Bold
    HelveticaBold11,
    /// Verdana 10px Normal
    VerdanaNormal10,
    /// Verdana 10px Bold
    VerdanaBold10,
}

/// Calculates the width of text in the given font (in pixels)
///
/// - Width tables are generated at compile time from the JSON sources; no runtime parsing or IO
/// - Can be directly used in scenarios like SVG badges
pub fn get_text_width(text: &str, font: Font) -> f64 {
    use crate::measurer::CharWidthMeasurer;
    use std::sync::LazyLock;

    static VERDANA_11_N: LazyLock<CharWidthMeasurer> =
        LazyLock::new(|| CharWidthMeasurer::from_sorted_static(&font_tables::VERDANA_11_NORMAL));
    static HELVETICA_11_B: LazyLock<CharWidthMeasurer> =
        LazyLock::new(|| CharWidthMeasurer::from_sorted_static(&font_tables::HELVETICA_11_BOLD));
    static VERDANA_10_N: LazyLock<CharWidthMeasurer> =
        LazyLock::new(|| CharWidthMeasurer::from_sorted_static(&font_tables::VERDANA_10_NORMAL));
    static VERDANA_10_B: LazyLock<CharWidthMeasurer> =
        LazyLock::new(|| CharWidthMeasurer::from_sorted_static(&font_tables::VERDANA_10_BOLD));

    match font {
        Font::VerdanaNormal11 => VERDANA_11_N.width_of(text, true),
        Font::HelveticaBold11 => HELVETICA_11_B.width_of(text, true),
        Font::VerdanaNormal10 => VERDANA_10_N.width_of(text, true),
        Font::VerdanaBold10 => VERDANA_10_B.width_of(text, true),
    }
}
macro_rules! round_up_to_odd_float {
    ($func:ident, $float:ty) => {
        fn $func(n: $float) -> u32 {
            let n_rounded = n.floor() as u32;
            if n_rounded % 2 == 0 {
                n_rounded + 1
            } else {
                n_rounded
            }
        }
    };
}

round_up_to_odd_float!(round_up_to_odd_f64, f64);
const BADGE_HEIGHT: u32 = 20;
const HORIZONTAL_PADDING: u32 = 5;
const FONT_FAMILY: &str = "Verdana,Geneva,DejaVu Sans,sans-serif";
const FONT_SIZE_SCALED: u32 = 110;
const FONT_SCALE_UP_FACTOR: u32 = 10;
/// Dynamically calculates foreground and shadow colors based on background color (equivalent to JS colorsForBackground)
///
/// - Input: hex color string (supports 3/6 digits, e.g. "#4c1", "#007ec6")
/// - Algorithm:
///   1. Parses hex to RGB
///   2. Calculates brightness = (0.299*R + 0.587*G + 0.114*B) / 255
///   3. If brightness ≤ 0.69, returns ("#fff", "#010101"), otherwise ("#333", "#ccc")
pub fn colors_for_background(hex: &str) -> (&'static str, &'static str) {
    // Remove leading #
    let hex = hex.trim_start_matches('#');
    // Expands a single hex digit to a full byte, e.g. 'c' -> 0xcc; invalid digits count as 0
    let expand_nibble = |c: u8| -> u8 {
        let v = match c {
            b'0'..=b'9' => c - b'0',
            b'a'..=b'f' => c - b'a' + 10,
            b'A'..=b'F' => c - b'A' + 10,
            _ => 0,
        };
        (v << 4) | v
    };
    // Parse RGB
    let (r, g, b) = match hex.len() {
        3 => {
            let bytes = hex.as_bytes();
            (
                expand_nibble(bytes[0]),
                expand_nibble(bytes[1]),
                expand_nibble(bytes[2]),
            )
        }
        6 => (
            u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
            u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
            u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
        ),
        _ => (0, 0, 0), // Invalid input, return black
    };
    // W3C recommended brightness formula
    let brightness = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) / 255.0;
    if brightness <= 0.69 {
        ("#fff", "#010101")
    } else {
        ("#333", "#ccc")
    }
}
pub(crate) fn preferred_width_of(text: &str, font: Font) -> u32 {
    round_up_to_odd_f64(get_text_width(text, font))
}

/// Foreground/shadow pair for `color`, falling back to `fallback` when it does not parse.
///
/// Colors reaching here have already been normalized by `to_svg_color`, so they are
/// usually plain hex. `colors_for_background` expands 3-digit hex to the same bytes
/// `to_css_hex` would produce, so hex inputs can skip the CSS parse entirely.
fn colors_for_color_or(color: &str, fallback: &str) -> (&'static str, &'static str) {
    if color_util::is_valid_hex(color) {
        return colors_for_background(color);
    }
    let hex = Color::from_str(color)
        .unwrap_or_else(|_| Color::from_str(fallback).unwrap())
        .to_css_hex();
    colors_for_background(&hex)
}

/// Capitalizes the first character and lowercases the rest (matches askama's `capitalize`).
fn capitalize(s: &str) -> String {
    match s.chars().next() {
        Some(first) => {
            let mut out: String = first.to_uppercase().collect();
            out.push_str(&s[first.len_utf8()..].to_lowercase());
            out
        }
        None => String::new(),
    }
}

/// Shared horizontal layout for the flat, flat-square and plastic styles.
/// These styles differ only in chrome (gradients, shadows), not in geometry.
struct FlatLayout<'a> {
    accessible_text: String,
    label: &'a str,
    left_width: i32,
    right_width: i32,
    total_width: i32,
    label_x: f32,
    label_width_scaled: i32,
    message_x: f32,
    message_width_scaled: i32,
    rect_offset: i32,
    message_link_x: i32,
    label_text_color: &'static str,
    label_shadow_color: &'static str,
    message_text_color: &'static str,
    message_shadow_color: &'static str,
}

#[allow(clippy::too_many_arguments)]
fn compute_flat_layout<'a>(
    label: Option<&'a str>,
    message: &str,
    label_color: &str,
    message_color: &str,
    has_label_color: bool,
    has_logo: bool,
    total_logo_width: u32,
    extra_link_not_empty_str: bool,
    extra_link: &str,
) -> FlatLayout<'a> {
    let accessible_text = create_accessible_text(label, message);
    let has_label_content = label.is_some() && !label.unwrap().is_empty();
    let has_label = has_label_content || has_label_color;
    let label_margin = total_logo_width + 1;

    let label_width = if has_label && label.is_some() {
        preferred_width_of(label.unwrap_or_default(), Font::VerdanaNormal11)
    } else {
        0
    };

    let mut left_width = if has_label {
        (label_width + 2 * HORIZONTAL_PADDING + total_logo_width) as i32
    } else {
        0
    };

    if has_label && label.is_some() && label.unwrap().is_empty() {
        left_width -= 1;
    }
    let message_width = preferred_width_of(message, Font::VerdanaNormal11);

    let offset = if label.is_none() && has_logo {
        -3i32
    } else {
        0
    };

    let left_width = left_width + offset;
    let mut message_margin: i32 = left_width - if message.is_empty() { 0 } else { 1 };
    if !has_label {
        if has_logo {
            message_margin += (total_logo_width + HORIZONTAL_PADDING) as i32;
        } else {
            message_margin += 1;
        }
    }

    let mut right_width = (message_width + 2 * HORIZONTAL_PADDING) as i32;
    if has_logo && !has_label {
        right_width += total_logo_width as i32
            + if !message.is_empty() {
                (HORIZONTAL_PADDING - 1) as i32
            } else {
                0i32
            };
    }

    let label_x = 10.0
        * (label_margin as f32 + (0.5 * label_width as f32) + HORIZONTAL_PADDING as f32)
        + offset as f32;
    let label_width_scaled = (label_width * 10) as i32;
    let total_width = left_width + right_width;

    let right_width = right_width + if !has_label_color { offset } else { 0 };
    let (label_text_color, label_shadow_color) = colors_for_color_or(label_color, "#555");
    let (message_text_color, message_shadow_color) = colors_for_color_or(message_color, "#007ec6");
    let rect_offset = if has_logo { 19 } else { 0 };

    let message_link_x = if has_logo && !has_label && extra_link_not_empty_str {
        total_logo_width as i32 + HORIZONTAL_PADDING as i32
    } else {
        left_width
    };

    let has_extra_link = !extra_link.is_empty();
    let message_x =
        10.0 * (message_margin as f32 + (0.5 * message_width as f32) + HORIZONTAL_PADDING as f32);
    let message_link_x = message_link_x
        + if !has_label && has_extra_link {
            offset
        } else {
            0
        };
    let message_width_scaled = (message_width * 10) as i32;
    let left_width = left_width.max(0);

    FlatLayout {
        accessible_text,
        label: label.unwrap_or(""),
        left_width,
        right_width,
        total_width,
        label_x,
        label_width_scaled,
        message_x,
        message_width_scaled,
        rect_offset,
        message_link_x,
        label_text_color,
        label_shadow_color,
        message_text_color,
        message_shadow_color,
    }
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
/// Badge style variants supported by the shields crate.
///
/// - `Flat`: Modern flat style (default).
/// - `FlatSquare`: Flat with square edges.
/// - `Plastic`: Classic plastic style.
/// - `Social`: Social badge style (e.g., GitHub social).
/// - `ForTheBadge`: All-caps, bold, attention-grabbing style.
///
/// ## Example
/// ```rust
/// use shields::BadgeStyle;
/// let style = BadgeStyle::Plastic;
/// ```
pub enum BadgeStyle {
    /// Flat style, which is modern and minimalistic.
    #[default]
    Flat,
    /// Flat style, which is modern and minimalistic, but with square edges.
    FlatSquare,
    /// Plastic style, which has a glossy look.
    Plastic,
    /// Social badge style, typically used for GitHub or other social media badges.
    Social,
    /// For-the-badge style, which is bold and all-caps.
    ForTheBadge,
}

/// Returns the default message color hex string (`#007ec6`).
pub fn default_message_color() -> &'static str {
    "#007ec6"
}

/// Returns the default label color hex string (`#555`).
pub fn default_label_color() -> &'static str {
    "#555"
}

#[derive(Deserialize, Debug)]
/// Parameters for generating a badge SVG.
///
/// This struct is used to configure all aspects of a badge, including style, label, message, colors, links, and logo.
///
/// # Fields
/// - `style`: Badge style variant (see [`BadgeStyle`]).
/// - `label`: Optional label text (left side).
/// - `message`: Optional message text (right side).
/// - `label_color`: Optional label background color (hex, name, or alias).
/// - `message_color`: Optional message background color (hex, name, or alias).
/// - `link`: Optional main link URL.
/// - `extra_link`: Optional secondary link URL.
/// - `logo`: Optional logo name or SVG data.
/// - `logo_color`: Optional logo color.
///
/// ## Example
/// ```rust
/// use shields::{BadgeParams, BadgeStyle, render_badge_svg};
/// let params = BadgeParams {
///     style: BadgeStyle::Flat,
///     label: Some("build"),
///     message: Some("passing"),
///     label_color: Some("green"),
///     message_color: Some("brightgreen"),
///     link: Some("https://ci.example.com"),
///     extra_link: None,
///     logo: None,
///     logo_color: None,
/// };
/// let svg = render_badge_svg(&params);
/// assert!(svg.contains("passing"));
/// ```
pub struct BadgeParams<'a> {
    #[serde(default)]
    /// Badge style variant (default is `Flat`).
    pub style: BadgeStyle,
    /// Optional label text (left side).
    pub label: Option<&'a str>,
    /// Optional message text (right side).
    pub message: Option<&'a str>,
    /// Optional label color, defaults to `#555` (dark gray).
    pub label_color: Option<&'a str>,
    /// Optional message color, defaults to `#007ec6` (blue).
    pub message_color: Option<&'a str>,
    /// Optional main link, used for linking the badge to a URL.
    pub link: Option<&'a str>,
    /// Optional secondary link, used for social badges or additional information.
    pub extra_link: Option<&'a str>,
    /// Optional logo name (e.g., "github", "rust") or SVG data.
    pub logo: Option<&'a str>,
    /// Optional logo color, defaults to `#000000` for social badges, otherwise `whitesmoke`.
    pub logo_color: Option<&'a str>,
}

/// Owned variant of [`BadgeParams`], for callers that cannot borrow —
/// typically deserializing from an HTTP query string or JSON body.
///
/// ## Example
/// ```rust
/// use shields::{BadgeParamsOwned, BadgeStyle};
/// let params: BadgeParamsOwned = serde_json::from_str(
///     r#"{"style":"flat","label":"build","message":"passing"}"#,
/// ).unwrap();
/// let svg = params.render();
/// assert!(svg.contains("passing"));
/// ```
#[derive(Deserialize, Debug, Clone, Default)]
pub struct BadgeParamsOwned {
    #[serde(default)]
    /// Badge style variant (default is `Flat`).
    pub style: BadgeStyle,
    /// Optional label text (left side).
    pub label: Option<String>,
    /// Optional message text (right side).
    pub message: Option<String>,
    /// Optional label color, defaults to `#555` (dark gray).
    pub label_color: Option<String>,
    /// Optional message color, defaults to `#007ec6` (blue).
    pub message_color: Option<String>,
    /// Optional main link, used for linking the badge to a URL.
    pub link: Option<String>,
    /// Optional secondary link, used for social badges or additional information.
    pub extra_link: Option<String>,
    /// Optional logo name (e.g., "github", "rust") or SVG data.
    pub logo: Option<String>,
    /// Optional logo color, defaults to `#000000` for social badges, otherwise `whitesmoke`.
    pub logo_color: Option<String>,
}

impl BadgeParamsOwned {
    /// Borrows these owned parameters as a [`BadgeParams`].
    pub fn as_params(&self) -> BadgeParams<'_> {
        BadgeParams {
            style: self.style,
            label: self.label.as_deref(),
            message: self.message.as_deref(),
            label_color: self.label_color.as_deref(),
            message_color: self.message_color.as_deref(),
            link: self.link.as_deref(),
            extra_link: self.extra_link.as_deref(),
            logo: self.logo.as_deref(),
            logo_color: self.logo_color.as_deref(),
        }
    }

    /// Renders the badge SVG (see [`render_badge_svg`]).
    pub fn render(&self) -> String {
        render_badge_svg(&self.as_params())
    }
}

/// Additional rendering options that extend [`BadgeParams`] without breaking
/// its literal-construction API.
///
/// Construct with [`RenderOptions::default`] and set fields through the
/// builder-style methods:
///
/// ```rust
/// use shields::RenderOptions;
/// let opts = RenderOptions::default().id_suffix("badge1");
/// ```
#[derive(Debug, Default, Clone)]
#[non_exhaustive]
pub struct RenderOptions<'a> {
    /// Suffix appended to every SVG element id (`#s`, `#r`, `#llink`, ...).
    ///
    /// SVGs embedded inline in the same HTML page share one id namespace, so
    /// two badges both defining `id="s"` corrupt each other's gradients. Give
    /// each badge a unique suffix to avoid collisions. Only `[A-Za-z0-9_-]`
    /// characters are used; anything else is stripped.
    pub id_suffix: &'a str,

    /// Width of the rendered logo in pixels (default 14).
    ///
    /// Mirrors badge-maker's `logoWidth` option. The logo height stays 14;
    /// widen this for logos with a wide aspect ratio so they are not squeezed.
    pub logo_width: Option<u32>,
}

impl<'a> RenderOptions<'a> {
    /// Sets the id suffix (see the field documentation).
    pub fn id_suffix(mut self, id_suffix: &'a str) -> Self {
        self.id_suffix = id_suffix;
        self
    }

    /// Sets the rendered logo width in pixels (see the field documentation).
    pub fn logo_width(mut self, logo_width: u32) -> Self {
        self.logo_width = Some(logo_width);
        self
    }
}

/// Rejects `href` values whose scheme executes script when the badge is opened
/// or embedded as a document.
///
/// XML escaping keeps a link inside its attribute, but `javascript:alert(1)`
/// needs no special character to fire — the scheme itself is the payload. Only
/// the script-bearing schemes are refused; everything else (absolute URLs,
/// relative paths, fragments, `mailto:`, …) passes through untouched.
fn is_safe_link(link: &str) -> bool {
    // Browsers ignore leading whitespace and C0 controls before the scheme, and
    // match it case-insensitively, so strip and fold before comparing.
    let trimmed = link.trim_matches(|c: char| c.is_whitespace() || (c as u32) < 0x20);
    let Some(colon) = trimmed.find(':') else {
        // No scheme at all: a relative path or fragment, which cannot execute.
        return true;
    };
    let scheme = &trimmed[..colon];
    // A '/', '?' or '#' before the colon means it was never a scheme
    // ("/a:b" is a path), so the value is relative and safe.
    if scheme.contains(['/', '?', '#']) {
        return true;
    }
    // Browsers also skip embedded tabs/newlines inside the scheme ("java\tscript:").
    let scheme: String = scheme
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    !matches!(scheme.as_str(), "javascript" | "vbscript" | "data")
}

/// Strips characters outside `[A-Za-z0-9_-]`.
///
/// The suffix is not only interpolated into `id="…"` but also into the
/// `url(#…)` references pointing at it, where escaping would break the link
/// rather than protect it. Restricting the character set keeps both sides
/// valid without relying on the escaper.
fn sanitize_id_suffix(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}

/// Generate an SVG badge string from [`BadgeParams`].
///
/// # Arguments
/// * `params` - Badge parameters (see [`BadgeParams`]).
///
/// # Returns
/// SVG string representing the badge.
///
/// ## Example
/// ```rust
/// use shields::{BadgeParams, BadgeStyle, render_badge_svg};
/// let params = BadgeParams {
///     style: BadgeStyle::Flat,
///     label: Some("build"),
///     message: Some("passing"),
///     label_color: Some("green"),
///     message_color: Some("brightgreen"),
///     link: Some("https://ci.example.com"),
///     extra_link: None,
///     logo: None,
///     logo_color: None,
/// };
/// let svg = render_badge_svg(&params);
/// assert!(svg.contains("passing"));
/// ```
pub fn render_badge_svg(params: &BadgeParams) -> String {
    render_badge_svg_with(params, &RenderOptions::default())
}

/// Generate an SVG badge string from [`BadgeParams`] plus [`RenderOptions`].
///
/// ## Example
/// ```rust
/// use shields::{BadgeParams, BadgeStyle, RenderOptions, render_badge_svg_with};
/// let params = BadgeParams {
///     style: BadgeStyle::Flat,
///     label: Some("build"),
///     message: Some("passing"),
///     label_color: None,
///     message_color: None,
///     link: None,
///     extra_link: None,
///     logo: None,
///     logo_color: None,
/// };
/// let svg = render_badge_svg_with(&params, &RenderOptions::default().id_suffix("b1"));
/// assert!(svg.contains(r##"id="sb1""##));
/// ```
pub fn render_badge_svg_with(params: &BadgeParams, options: &RenderOptions) -> String {
    render_badge_svg_impl(params, options)
        .unwrap_or_else(|e| format!("<!-- Askama render error: {e} -->"))
}

/// Generate an SVG badge string, returning an error instead of an HTML
/// comment when template rendering fails.
///
/// [`render_badge_svg`] silently embeds failures as `<!-- Askama render
/// error -->` comments; use this variant when the caller needs to react.
pub fn try_render_badge_svg(params: &BadgeParams) -> Result<String, RenderError> {
    try_render_badge_svg_with(params, &RenderOptions::default())
}

/// [`try_render_badge_svg`] with additional [`RenderOptions`].
pub fn try_render_badge_svg_with(
    params: &BadgeParams,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    render_badge_svg_impl(params, options).map_err(|e| RenderError(e.to_string()))
}

/// Error returned by [`try_render_badge_svg`] when template rendering fails.
#[derive(Debug)]
pub struct RenderError(String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "badge template rendering failed: {}", self.0)
    }
}

impl std::error::Error for RenderError {}

/// Renders `ctx`, reserving room for the logo up front.
///
/// `Template::render` sizes its buffer from `SIZE_HINT`, which only covers the template's
/// literal text. A badge carrying a base64 logo runs several KB past that and would
/// reallocate mid-render.
fn render_reserving<T: Template>(ctx: &T, logo_len: usize) -> Result<String, askama::Error> {
    let mut buf = String::with_capacity(T::SIZE_HINT + logo_len);
    ctx.render_into(&mut buf)?;
    Ok(buf)
}

/// Resolves `logo` (a Simple Icons slug or a raw `<svg>` string) to the `href` value
/// the templates embed: a base64 data URI, or an empty string when nothing resolves.
fn build_logo_data_uri(logo: &str, logo_color: &str) -> String {
    let icon_svg: &str = if logo.starts_with("<svg") {
        logo
    } else {
        #[cfg(feature = "simple-icons")]
        {
            simpleicons::Icon::get_svg(logo).unwrap_or_default()
        }
        // Without the simple-icons feature, named logos resolve to nothing
        #[cfg(not(feature = "simple-icons"))]
        {
            ""
        }
    };
    if !icon_svg.starts_with("<svg") {
        return icon_svg.to_string();
    }

    // Only inject fill when the <svg> tag does not already carry one
    let svg_tag_end = icon_svg.find('>').unwrap_or(0);
    let has_fill_in_svg_tag = icon_svg[..svg_tag_end].contains("fill=");
    let logo_svg = if !has_fill_in_svg_tag && !logo_color.is_empty() {
        Cow::Owned(icon_svg.replace("<svg", format!("<svg fill=\"{logo_color}\"").as_str()))
    } else {
        Cow::Borrowed(icon_svg)
    };

    const PREFIX: &str = "data:image/svg+xml;base64,";
    let mut uri = String::with_capacity(PREFIX.len() + logo_svg.len().div_ceil(3) * 4);
    uri.push_str(PREFIX);
    base64::engine::general_purpose::STANDARD.encode_string(logo_svg.as_ref(), &mut uri);
    uri
}

/// Resolving a logo means a Simple Icons lookup, a fill rewrite and a base64 encode of
/// a few KB — roughly 70% of a logo badge's render time, and fully determined by
/// `(logo, logo_color)`. A small per-thread cache keeps it off the hot path; being
/// thread-local, it costs no lock and does not serialize concurrent rendering.
mod logo_cache {
    use std::cell::RefCell;
    use std::rc::Rc;

    const CAPACITY: usize = 16;

    thread_local! {
        /// Least-recently-used first, so eviction pops the front.
        static CACHE: RefCell<Vec<(String, String, Rc<str>)>> = const { RefCell::new(Vec::new()) };
    }

    pub fn get_or_insert(logo: &str, color: &str, build: fn(&str, &str) -> String) -> Rc<str> {
        CACHE.with_borrow_mut(|entries| {
            if let Some(i) = entries.iter().position(|(l, c, _)| l == logo && c == color) {
                let entry = entries.remove(i);
                let uri = Rc::clone(&entry.2);
                entries.push(entry);
                return uri;
            }
            let uri: Rc<str> = Rc::from(build(logo, color));
            if entries.len() == CAPACITY {
                entries.remove(0);
            }
            entries.push((logo.to_owned(), color.to_owned(), Rc::clone(&uri)));
            uri
        })
    }
}

fn render_badge_svg_impl(
    params: &BadgeParams,
    options: &RenderOptions,
) -> Result<String, askama::Error> {
    let id_suffix = sanitize_id_suffix(options.id_suffix);
    let id_suffix = id_suffix.as_str();
    let BadgeParams {
        style,
        label,
        message,
        label_color,
        message_color,
        link,
        extra_link,
        logo,
        logo_color,
    } = params;
    let label = *label;
    let default_logo_color = if *style == BadgeStyle::Social {
        "#000000"
    } else {
        "whitesmoke"
    };

    let logo_color = logo_color.unwrap_or(default_logo_color);
    let logo_color = to_svg_color(logo_color).unwrap_or(Cow::Borrowed(default_logo_color));

    let logo_src = logo.map(str::trim).unwrap_or("");
    let logo_uri = (!logo_src.is_empty())
        .then(|| logo_cache::get_or_insert(logo_src, &logo_color, build_logo_data_uri));
    let logo = logo_uri.as_deref().unwrap_or("");
    let has_logo = !logo.is_empty();
    let logo_width = options.logo_width.unwrap_or(14);
    let mut logo_padding = 3;
    if label.is_some() && label.unwrap().is_empty() {
        logo_padding = 0;
    }

    let total_logo_width = if has_logo {
        logo_width + logo_padding
    } else {
        0
    };

    let has_label_color = !label_color.unwrap_or("").is_empty();
    let message_color = message_color.unwrap_or(default_message_color());
    let message_color = to_svg_color(message_color).unwrap_or(Cow::Borrowed("#007ec6"));

    let label_color = match (
        label.unwrap_or("").is_empty(),
        label_color.unwrap_or("").is_empty(),
    ) {
        (true, true) if has_logo => "#555",
        (true, true) => message_color.as_ref(),
        (_, _) => label_color.unwrap_or(default_label_color()),
    };

    let binding = to_svg_color(label_color).unwrap_or(Cow::Borrowed("#555"));
    let label_color = binding.as_ref();

    let message_color = message_color.as_ref();
    let message = message.unwrap_or("");
    // A rejected link is treated exactly like an absent one, so layout stays consistent.
    let link = link.filter(|l| is_safe_link(l));
    let extra_link = extra_link.filter(|l| is_safe_link(l));
    let link = link.unwrap_or("");
    let extra_link_not_empty_str = extra_link.is_none() || !extra_link.unwrap().is_empty();
    let extra_link = extra_link.unwrap_or("");
    match style {
        BadgeStyle::Flat => {
            let l = compute_flat_layout(
                label,
                message,
                label_color,
                message_color,
                has_label_color,
                has_logo,
                total_logo_width,
                extra_link_not_empty_str,
                extra_link,
            );
            let ctx = FlatBadgeSvgTemplateContext {
                logo_width,
                font_family: FONT_FAMILY,
                id_suffix,
                accessible_text: l.accessible_text.as_str(),
                badge_height: BADGE_HEIGHT as i32,
                left_width: l.left_width,
                right_width: l.right_width,
                total_width: l.total_width,
                label_color,
                message_color,
                font_size_scaled: FONT_SIZE_SCALED as i32,
                label: l.label,
                label_x: l.label_x,
                label_width_scaled: l.label_width_scaled,
                label_text_color: l.label_text_color,
                label_shadow_color: l.label_shadow_color,
                message_x: l.message_x,
                message_shadow_color: l.message_shadow_color,
                message_text_color: l.message_text_color,
                message_width_scaled: l.message_width_scaled,
                message,
                link,
                extra_link,
                logo,
                rect_offset: l.rect_offset,
                message_link_x: l.message_link_x,
            };
            render_reserving(&ctx, logo.len())
        }
        BadgeStyle::FlatSquare => {
            let l = compute_flat_layout(
                label,
                message,
                label_color,
                message_color,
                has_label_color,
                has_logo,
                total_logo_width,
                extra_link_not_empty_str,
                extra_link,
            );
            let ctx = FlatSquareBadgeSvgTemplateContext {
                logo_width,
                font_family: FONT_FAMILY,
                accessible_text: l.accessible_text.as_str(),
                badge_height: BADGE_HEIGHT as i32,
                left_width: l.left_width,
                right_width: l.right_width,
                total_width: l.total_width,
                label_color,
                message_color,
                font_size_scaled: FONT_SIZE_SCALED as i32,
                label: l.label,
                label_x: l.label_x,
                label_width_scaled: l.label_width_scaled,
                label_text_color: l.label_text_color,
                message_x: l.message_x,
                message_text_color: l.message_text_color,
                message_width_scaled: l.message_width_scaled,
                message,
                link,
                extra_link,
                logo,
                rect_offset: l.rect_offset,
                message_link_x: l.message_link_x,
            };
            render_reserving(&ctx, logo.len())
        }
        BadgeStyle::Plastic => {
            let l = compute_flat_layout(
                label,
                message,
                label_color,
                message_color,
                has_label_color,
                has_logo,
                total_logo_width,
                extra_link_not_empty_str,
                extra_link,
            );
            let ctx = PlasticBadgeSvgTemplateContext {
                logo_width,
                total_width: l.total_width,
                id_suffix,
                left_width: l.left_width,
                right_width: l.right_width,
                accessible_text: l.accessible_text.as_str(),
                label: l.label,
                label_x: l.label_x,
                label_text_length: l.label_width_scaled,
                label_text_color: l.label_text_color,
                label_shadow_color: l.label_shadow_color,
                message,
                message_x: l.message_x,
                message_text_length: l.message_width_scaled,
                message_text_color: l.message_text_color,
                message_shadow_color: l.message_shadow_color,
                label_color,
                message_color,
                link,
                extra_link,
                logo,
                rect_offset: l.rect_offset,
                message_link_x: l.message_link_x,
            };
            render_reserving(&ctx, logo.len())
        }
        BadgeStyle::Social => {
            let label_is_none = label.is_none();

            let offset = if label_is_none && has_logo {
                -3i32
            } else {
                0i32
            };

            let label = capitalize(label.unwrap_or(""));
            let label_str = label.as_str();
            let accessible_text = create_accessible_text(Some(label_str), message);
            let internal_height = 19;
            let label_horizontal_padding = 5;
            let message_horizontal_padding = 4;
            let horizontal_gutter = 6;

            let label_text_width = preferred_width_of(label_str, Font::HelveticaBold11);

            let label_rect_width =
                (label_text_width + total_logo_width + 2 * label_horizontal_padding) as i32
                    + offset;

            let message_text_width = preferred_width_of(message, Font::HelveticaBold11);

            let message_rect_width = message_text_width + 2 * message_horizontal_padding;
            let has_message = !message.is_empty();

            let message_bubble_main_x = label_rect_width as f32 + horizontal_gutter as f32 + 0.5;
            let message_bubble_notch_x = label_rect_width + horizontal_gutter;
            let label_text_x = FONT_SCALE_UP_FACTOR as f32
                * (total_logo_width as f32
                    + label_text_width as f32 / 2.0
                    + label_horizontal_padding as f32
                    + offset as f32);
            let message_text_x = FONT_SCALE_UP_FACTOR as f32
                * (label_rect_width as f32
                    + horizontal_gutter as f32
                    + message_rect_width as f32 / 2.0);
            let message_text_length = FONT_SCALE_UP_FACTOR * message_text_width;
            let label_text_length = FONT_SCALE_UP_FACTOR * label_text_width;

            let left_width = label_rect_width + 1;
            let right_width = if has_message {
                horizontal_gutter + message_rect_width as i32
            } else {
                0
            };

            let total_width = left_width + right_width;

            let ctx = SocialBadgeSvgTemplateContext {
                logo_width,
                total_width,
                id_suffix,
                total_height: BADGE_HEIGHT as i32,
                internal_height,
                accessible_text: accessible_text.as_str(),
                message_rect_width,
                message_bubble_main_x,
                message_bubble_notch_x,
                label_text_length,
                label: label_str,
                message,
                label_text_x,
                message_text_x,
                message_text_length,
                label_rect_width,
                link,
                extra_link,
                logo,
            };
            render_reserving(&ctx, logo.len())
        }
        BadgeStyle::ForTheBadge => {
            // for-the-badge is styled in all caps; convert before measuring widths
            let label = label.unwrap_or("").to_uppercase();
            let message = message.to_uppercase();
            let accessible_text = create_accessible_text(Some(label.as_str()), message.as_str());
            let font_size = 10;
            let letter_spacing = 1.25f64;
            let logo_text_gutter = 6.0f64;
            let logo_margin = 9.0f64;
            let logo_width = logo_width as f64;
            // Upstream truncates the font measurement (`anafanafo(...) | 0`) and adds
            // letter spacing per UTF-16 code unit, keeping fractional widths throughout.
            let label_text_width = if !label.is_empty() {
                get_text_width(&label, Font::VerdanaNormal10).trunc()
                    + letter_spacing * label.encode_utf16().count() as f64
            } else {
                0.0
            };
            let message_text_width = if !message.is_empty() {
                get_text_width(&message, Font::VerdanaBold10).trunc()
                    + letter_spacing * message.encode_utf16().count() as f64
            } else {
                0.0
            };
            let has_label = !label.is_empty();
            let no_text = !has_label && message.is_empty();
            // Upstream checks the caller-supplied labelColor, not the resolved
            // default that the shared preprocessing may have filled in.
            let need_label_rect = has_label || (!logo.is_empty() && has_label_color);
            let gutter = if no_text {
                logo_text_gutter - logo_margin
            } else {
                logo_text_gutter
            };
            let text_margin = 12.0f64;

            // Logo positioning
            let (logo_min_x, label_text_min_x) = if !logo.is_empty() {
                (logo_margin, logo_margin + logo_width + gutter)
            } else {
                (0.0, text_margin)
            };

            // Handle label and message rectangles
            let (label_rect_width, message_text_min_x, message_rect_width) = if need_label_rect {
                if has_label {
                    (
                        label_text_min_x + label_text_width + text_margin,
                        label_text_min_x + label_text_width + text_margin + text_margin,
                        2.0 * text_margin + message_text_width,
                    )
                } else {
                    (
                        2.0 * logo_margin + logo_width,
                        2.0 * logo_margin + logo_width + text_margin,
                        2.0 * text_margin + message_text_width,
                    )
                }
            } else if !logo.is_empty() {
                (
                    0.0,
                    text_margin + logo_width + gutter,
                    2.0 * text_margin + logo_width + gutter + message_text_width,
                )
            } else {
                (0.0, text_margin, 2.0 * text_margin + message_text_width)
            };
            let total_width = label_rect_width + message_rect_width;

            let message_mid_x = message_text_min_x + 0.5 * message_text_width;
            let label_mid_x = label_text_min_x + 0.5 * label_text_width;

            let (label_text_color, _) = colors_for_color_or(label_color, "#555");
            let (message_text_color, _) = colors_for_color_or(message_color, "#007ec6");

            let ctx = ForTheBadgeSvgTemplateContext {
                logo_width: logo_width as u32,
                total_width,
                accessible_text: accessible_text.as_str(),
                has_label_rect: need_label_rect,
                left_width: label_rect_width,
                right_width: message_rect_width,
                label_color,
                message_color,
                font_family: FONT_FAMILY,
                font_size: font_size * FONT_SCALE_UP_FACTOR as i32,
                label: label.as_str(),
                label_x: label_mid_x * FONT_SCALE_UP_FACTOR as f64,
                label_width_scaled: label_text_width * FONT_SCALE_UP_FACTOR as f64,
                label_text_color,
                message: message.as_str(),
                message_x: message_mid_x * FONT_SCALE_UP_FACTOR as f64,
                message_text_color,
                message_width_scaled: message_text_width * FONT_SCALE_UP_FACTOR as f64,
                link,
                extra_link,
                logo,
                logo_x: logo_min_x,
            };
            render_reserving(&ctx, logo.len())
        }
    }
}

fn create_accessible_text(label: Option<&str>, message: &str) -> String {
    let use_label = match label {
        Some(l) if !l.is_empty() => Some(l),
        _ => None,
    };
    let label_len = use_label.map_or(0, |l| l.len() + 2); // +2 for ": "
    let mut buf = String::with_capacity(label_len + message.len());
    if let Some(label) = use_label {
        buf.push_str(label);
        buf.push_str(": ");
    }
    buf.push_str(message);
    buf
}

#[cfg(test)]
mod tests {
    use csscolorparser::Color;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::*;
    #[test]
    fn test_svg() {
        // Test SVG rendering
        let params = BadgeParams {
            style: BadgeStyle::FlatSquare,
            label: Some("build"),
            message: Some("passing"),
            label_color: Some("#333"),
            message_color: Some("#4c1"),
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        };
        let svg = render_badge_svg(&params);
        assert!(!svg.is_empty(), "SVG rendering failed");
    }

    #[test]
    fn text_for_the_badge() {
        // Test ForTheBadge style rendering
        let params = BadgeParams {
            style: BadgeStyle::ForTheBadge,
            label: Some("building"),
            message: Some("pass"),
            label_color: Some("#555"),
            message_color: Some("#fff"),
            link: Some("https://google.com"),
            extra_link: Some("https://example.com"),
            logo: Some("rust"),
            logo_color: Some("blue"),
        };
        let svg = render_badge_svg(&params);
        let expected = r##"<svg xmlns="http://www.w3.org/2000/svg" width="160" height="28"><g shape-rendering="crispEdges"><rect width="102" height="28" fill="#555"/><rect x="102" width="58" height="28" fill="#fff"/></g><g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="100"><image x="9" y="7" width="14" height="14" href="data:image/svg+xml;base64,PHN2ZyBmaWxsPSIjMDA3ZWM2IiByb2xlPSJpbWciIHZpZXdCb3g9IjAgMCAyNCAyNCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48dGl0bGU+UnVzdDwvdGl0bGU+PHBhdGggZD0iTTIzLjgzNDYgMTEuNzAzM2wtMS4wMDczLS42MjM2YTEzLjcyNjggMTMuNzI2OCAwIDAwLS4wMjgzLS4yOTM2bC44NjU2LS44MDY5YS4zNDgzLjM0ODMgMCAwMC0uMTE1NC0uNTc4bC0xLjEwNjYtLjQxNGE4LjQ5NTggOC40OTU4IDAgMDAtLjA4Ny0uMjg1NmwuNjkwNC0uOTU4N2EuMzQ2Mi4zNDYyIDAgMDAtLjIyNTctLjU0NDZsLTEuMTY2My0uMTg5NGE5LjM1NzQgOS4zNTc0IDAgMDAtLjE0MDctLjI2MjJsLjQ5LTEuMDc2MWEuMzQzNy4zNDM3IDAgMDAtLjAyNzQtLjMzNjEuMzQ4Ni4zNDg2IDAgMDAtLjMwMDYtLjE1NGwtMS4xODQ1LjA0MTZhNi43NDQ0IDYuNzQ0NCAwIDAwLS4xODczLS4yMjY4bC4yNzIzLTEuMTUzYS4zNDcyLjM0NzIgMCAwMC0uNDE3LS40MTcybC0xLjE1MzIuMjcyNGExNC4wMTgzIDE0LjAxODMgMCAwMC0uMjI3OC0uMTg3M2wuMDQxNS0xLjE4NDVhLjM0NDIuMzQ0MiAwIDAwLS40OS0uMzI4bC0xLjA3Ni40OTFjLS4wODcyLS4wNDc2LS4xNzQyLS4wOTUyLS4yNjIzLS4xNDA3bC0uMTkwMy0xLjE2NzNBLjM0ODMuMzQ4MyAwIDAwMTYuMjU2Ljk1NWwtLjk1OTcuNjkwNWE4LjQ4NjcgOC40ODY3IDAgMDAtLjI4NTUtLjA4NmwtLjQxNC0xLjEwNjZhLjM0ODMuMzQ4MyAwIDAwLS41NzgxLS4xMTU0bC0uODA2OS44NjY2YTkuMjkzNiA5LjI5MzYgMCAwMC0uMjkzNi0uMDI4NEwxMi4yOTQ2LjE2ODNhLjM0NjIuMzQ2MiAwIDAwLS41ODkyIDBsLS42MjM2IDEuMDA3M2ExMy43MzgzIDEzLjczODMgMCAwMC0uMjkzNi4wMjg0TDkuOTgwMy4zMzc0YS4zNDYyLjM0NjIgMCAwMC0uNTc4LjExNTRsLS40MTQxIDEuMTA2NWMtLjA5NjIuMDI3NC0uMTkwMy4wNTY3LS4yODU1LjA4Nkw3Ljc0NC45NTVhLjM0ODMuMzQ4MyAwIDAwLS41NDQ3LjIyNThMNy4wMDkgMi4zNDhhOS4zNTc0IDkuMzU3NCAwIDAwLS4yNjIyLjE0MDdsLTEuMDc2Mi0uNDkxYS4zNDYyLjM0NjIgMCAwMC0uNDkuMzI4bC4wNDE2IDEuMTg0NWE3Ljk4MjYgNy45ODI2IDAgMDAtLjIyNzguMTg3M0wzLjg0MTMgMy40MjVhLjM0NzIuMzQ3MiAwIDAwLS40MTcxLjQxNzFsLjI3MTMgMS4xNTMxYy0uMDYyOC4wNzUtLjEyNTUuMTUwOS0uMTg2My4yMjY4bC0xLjE4NDUtLjA0MTVhLjM0NjIuMzQ2MiAwIDAwLS4zMjguNDlsLjQ5MSAxLjA3NjFhOS4xNjcgOS4xNjcgMCAwMC0uMTQwNy4yNjIybC0xLjE2NjIuMTg5NGEuMzQ4My4zNDgzIDAgMDAtLjIyNTguNTQ0NmwuNjkwNC45NTg3YTEzLjMwMyAxMy4zMDMgMCAwMC0uMDg3LjI4NTVsLTEuMTA2NS40MTRhLjM0ODMuMzQ4MyAwIDAwLS4xMTU1LjU3ODFsLjg2NTYuODA3YTkuMjkzNiA5LjI5MzYgMCAwMC0uMDI4My4yOTM1bC0xLjAwNzMuNjIzNmEuMzQ0Mi4zNDQyIDAgMDAwIC41ODkybDEuMDA3My42MjM2Yy4wMDguMDk4Mi4wMTgyLjE5NjQuMDI4My4yOTM2bC0uODY1Ni44MDc5YS4zNDYyLjM0NjIgMCAwMC4xMTU1LjU3OGwxLjEwNjUuNDE0MWMuMDI3My4wOTYyLjA1NjcuMTkxNC4wODcuMjg1NWwtLjY5MDQuOTU4N2EuMzQ1Mi4zNDUyIDAgMDAuMjI2OC41NDQ3bDEuMTY2Mi4xODkzYy4wNDU2LjA4OC4wOTIyLjE3NTEuMTQwOC4yNjIybC0uNDkxIDEuMDc2MmEuMzQ2Mi4zNDYyIDAgMDAuMzI4LjQ5bDEuMTgzNC0uMDQxNWMuMDYxOC4wNzY5LjEyMzUuMTUyOC4xODczLjIyNzdsLS4yNzEzIDEuMTU0MWEuMzQ2Mi4zNDYyIDAgMDAuNDE3MS40MTYxbDEuMTUzLS4yNzEzYy4wNzUuMDYzOC4xNTEuMTI1NS4yMjc5LjE4NjNsLS4wNDE1IDEuMTg0NWEuMzQ0Mi4zNDQyIDAgMDAuNDkuMzI3bDEuMDc2MS0uNDljLjA4Ny4wNDg2LjE3NDEuMDk1MS4yNjIyLjE0MDdsLjE5MDMgMS4xNjYyYS4zNDgzLjM0ODMgMCAwMC41NDQ3LjIyNjhsLjk1ODctLjY5MDRhOS4yOTkgOS4yOTkgMCAwMC4yODU1LjA4N2wuNDE0IDEuMTA2NmEuMzQ1Mi4zNDUyIDAgMDAuNTc4MS4xMTU0bC44MDc5LS44NjU2Yy4wOTcyLjAxMTEuMTk1NC4wMjAzLjI5MzYuMDI5NGwuNjIzNiAxLjAwNzNhLjM0NzIuMzQ3MiAwIDAwLjU4OTIgMGwuNjIzNi0xLjAwNzNjLjA5ODItLjAwOTEuMTk2NC0uMDE4My4yOTM2LS4wMjk0bC44MDY5Ljg2NTZhLjM0ODMuMzQ4MyAwIDAwLjU3OC0uMTE1NGwuNDE0MS0xLjEwNjZhOC40NjI2IDguNDYyNiAwIDAwLjI4NTUtLjA4N2wuOTU4Ny42OTA0YS4zNDUyLjM0NTIgMCAwMC41NDQ3LS4yMjY4bC4xOTAzLTEuMTY2MmMuMDg4LS4wNDU2LjE3NTEtLjA5MzEuMjYyMi0uMTQwN2wxLjA3NjIuNDlhLjM0NzIuMzQ3MiAwIDAwLjQ5LS4zMjdsLS4wNDE1LTEuMTg0NWE2LjcyNjcgNi43MjY3IDAgMDAuMjI2Ny0uMTg2M2wxLjE1MzEuMjcxM2EuMzQ3Mi4zNDcyIDAgMDAuNDE3MS0uNDE2bC0uMjcxMy0xLjE1NDJjLjA2MjgtLjA3NDkuMTI1NS0uMTUwOC4xODYzLS4yMjc4bDEuMTg0NS4wNDE1YS4zNDQyLjM0NDIgMCAwMC4zMjgtLjQ5bC0uNDktMS4wNzZjLjA0NzUtLjA4NzIuMDk1MS0uMTc0Mi4xNDA3LS4yNjIzbDEuMTY2Mi0uMTg5M2EuMzQ4My4zNDgzIDAgMDAuMjI1OC0uNTQ0N2wtLjY5MDQtLjk1ODcuMDg3LS4yODU1IDEuMTA2Ni0uNDE0YS4zNDYyLjM0NjIgMCAwMC4xMTU0LS41NzgxbC0uODY1Ni0uODA3OWMuMDEwMS0uMDk3Mi4wMjAyLS4xOTU0LjAyODMtLjI5MzZsMS4wMDczLS42MjM2YS4zNDQyLjM0NDIgMCAwMDAtLjU4OTJ6bS02Ljc0MTMgOC4zNTUxYS43MTM4LjcxMzggMCAwMS4yOTg2LTEuMzk2LjcxNC43MTQgMCAxMS0uMjk5NyAxLjM5NnptLS4zNDIyLTIuMzE0MmEuNjQ5LjY0OSAwIDAwLS43NzE1LjVsLS4zNTczIDEuNjY4NWMtMS4xMDM1LjUwMS0yLjMyODUuNzc5NS0zLjYxOTMuNzc5NWE4LjczNjggOC43MzY4IDAgMDEtMy42OTUxLS44MTRsLS4zNTc0LTEuNjY4NGEuNjQ4LjY0OCAwIDAwLS43NzE0LS40OTlsLTEuNDczLjMxNThhOC43MjE2IDguNzIxNiAwIDAxLS43NjEzLS44OThoNy4xNjc2Yy4wODEgMCAuMTM1Ni0uMDE0MS4xMzU2LS4wODh2LTIuNTM2YzAtLjA3NC0uMDUzNi0uMDg4MS0uMTM1Ni0uMDg4MWgtMi4wOTY2di0xLjYwNzdoMi4yNjc3Yy4yMDY1IDAgMS4xMDY1LjA1ODcgMS4zOTQgMS4yMDg4LjA5MDEuMzUzMy4yODc1IDEuNTA0NC40MjMyIDEuODcyOS4xMzQ2LjQxMy42ODMzIDEuMjM4MSAxLjI2ODUgMS4yMzgxaDMuNTcxNmEuNzQ5Mi43NDkyIDAgMDAuMTI5Ni0uMDEzMSA4Ljc4NzQgOC43ODc0IDAgMDEtLjgxMTkuOTUyNnpNNi44MzY5IDIwLjAyNGEuNzE0LjcxNCAwIDExLS4yOTk3LTEuMzk2LjcxNC43MTQgMCAwMS4yOTk3IDEuMzk2ek00LjExNzcgOC45OTcyYS43MTM3LjcxMzcgMCAxMS0xLjMwNC41NzkxLjcxMzcuNzEzNyAwIDAxMS4zMDQtLjU3OXptLS44MzUyIDEuOTgxM2wxLjUzNDctLjY4MjRhLjY1LjY1IDAgMDAuMzMtLjg1ODVsLS4zMTU4LS43MTQ3aDEuMjQzMnY1LjYwMjVIMy41NjY5YTguNzc1MyA4Ljc3NTMgMCAwMS0uMjgzNC0zLjM0OHptNi43MzQzLS41NDM3VjguNzgzNmgyLjk2MDFjLjE1MyAwIDEuMDc5Mi4xNzcyIDEuMDc5Mi44Njk3IDAgLjU3NS0uNzEwNy43ODE1LTEuMjk0OC43ODE1em0xMC43NTc0IDEuNDg2MmMwIC4yMTg3LS4wMDguNDM2My0uMDI0My42NTFoLS45Yy0uMDkgMC0uMTI2NS4wNTg2LS4xMjY1LjE0Nzd2LjQxM2MwIC45NzMtLjU0ODcgMS4xODQ2LTEuMDI5NiAxLjIzODItLjQ1NzYuMDUxNy0uOTY0OC0uMTkxMy0xLjAyNzUtLjQ3MTctLjI3MDQtMS41MTg2LS43MTk4LTEuODQzNi0xLjQzMDUtMi40MDM0Ljg4MTctLjU1OTkgMS43OTktMS4zODYgMS43OTktMi40OTE1IDAtMS4xOTM2LS44MTktMS45NDU4LTEuMzc2OS0yLjMxNTMtLjc4MjUtLjUxNjMtMS42NDkxLS42MTk1LTEuODgzLS42MTk1SDUuNDY4MmE4Ljc2NTEgOC43NjUxIDAgMDE0LjkwNy0yLjc2OTlsMS4wOTc0IDEuMTUxYS42NDguNjQ4IDAgMDAuOTE4Mi4wMjEzbDEuMjI3LTEuMTc0M2E4Ljc3NTMgOC43NzUzIDAgMDE2LjAwNDQgNC4yNzYybC0uODQwMyAxLjg5ODJhLjY1Mi42NTIgMCAwMC4zMy44NTg1bDEuNjE3OC43MTg4Yy4wMjgzLjI4NzUuMDQyNS41NzcuMDQyNS44NzE3em0tOS4zMDA2LTkuNTk5M2EuNzEyOC43MTI4IDAgMTEuOTg0IDEuMDMxNi43MTM3LjcxMzcgMCAwMS0uOTg0LTEuMDMxNnptOC4zMzg5IDYuNzFhLjcxMDcuNzEwNyAwIDAxLjkzOTUtLjM2MjUuNzEzNy43MTM3IDAgMTEtLjk0MDUuMzYzNXoiLz48L3N2Zz4="/><a target="_blank" href="https://google.com"><rect width="102" height="28" fill="rgba(0,0,0,0)"/><text transform="scale(.1)" x="595" y="175" textLength="610">BUILDING</text></a><a target="_blank" href="https://example.com"><rect width="58" height="28" x="102" fill="rgba(0,0,0,0)"/><text transform="scale(.1)" x="1310" y="175" textLength="340" font-weight="bold" fill="#333">PASS</text></a></g></svg>"##;
        assert_eq!(
            svg, expected,
            "SVG rendering for ForTheBadge did not match expected output"
        );
        assert!(!svg.is_empty(), "SVG rendering for ForTheBadge failed");
    }

    #[test]
    fn test_named_color() {
        let params = BadgeParams {
            style: BadgeStyle::FlatSquare,
            label: Some("status"),
            message: Some("ok"),
            label_color: Some("brightgreen"),
            message_color: Some("blue"),
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        };
        let svg = render_badge_svg(&params);
        assert!(
            svg.contains("fill=\"#4c1\""),
            "Named color brightgreen not correctly mapped"
        );
        assert!(
            svg.contains("fill=\"#007ec6\""),
            "Named color blue not correctly mapped"
        );
    }

    #[test]
    fn test_alias_color() {
        let params = BadgeParams {
            style: BadgeStyle::FlatSquare,
            label: Some("status"),
            message: Some("ok"),
            label_color: Some("gray"),
            message_color: Some("critical"),
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        };
        let svg = render_badge_svg(&params);
        assert!(
            svg.contains("fill=\"#555\""),
            "Alias gray not correctly mapped"
        );
        assert!(
            svg.contains("fill=\"#e05d44\""),
            "Alias critical not correctly mapped"
        );
    }

    #[test]
    fn test_hex_color() {
        let params = BadgeParams {
            style: BadgeStyle::FlatSquare,
            label: Some("hex"),
            message: Some("ok"),
            label_color: Some("#4c1"),
            message_color: Some("dfb317"),
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        };
        let svg = render_badge_svg(&params);
        assert!(
            svg.contains("fill=\"#4c1\""),
            "3-digit hex not correctly processed"
        );
        assert!(
            svg.contains("fill=\"#dfb317\""),
            "6-digit hex not correctly processed"
        );
    }

    #[test]
    fn test_css_color() {
        let params = BadgeParams {
            style: BadgeStyle::FlatSquare,
            label: Some("css"),
            message: Some("ok"),
            label_color: Some("rgb(0,128,0)"),
            message_color: Some("hsl(120,100%,25%)"),
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        };
        let svg = render_badge_svg(&params);
        assert!(
            svg.contains(r#"fill="rgb(0,128,0)""#),
            "CSS rgb color not correctly processed"
        );
        assert!(
            svg.contains(r#"fill="hsl(120,100%,25%)""#),
            "CSS hsl color not correctly processed"
        );
    }

    #[test]
    fn test_invalid_color_fallback() {
        let params = BadgeParams {
            style: BadgeStyle::FlatSquare,
            label: Some("bad"),
            message: Some("ok"),
            label_color: Some("notacolor"),
            message_color: Some(""),
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        };
        let svg = render_badge_svg(&params);
        assert!(
            svg.contains("fill=\"#555\""),
            "Invalid label_color did not fallback to default color"
        );
        assert!(
            svg.contains("fill=\"#007ec6\""),
            "Empty message_color did not fallback to default color"
        );
    }

    #[test]
    fn test_color() {
        // 解析名称
        let c = Color::from_str("red").unwrap();
        println!("{c:?}");

        // 解析HEX
        let c = Color::from_str("#ff0080").unwrap();
        println!("{c:?}");

        // 解析RGBA
        let c = Color::from_str("rgba(255,255,0,0.75)").unwrap();
        println!("{c:?}");

        // 解析HSL
        let c = Color::from_str("hsl(120, 100%, 50%)").unwrap();
        println!("{c:?}");

        let c = Color::from_str("notexists").is_err();
        println!("{c:?}");
    }

    #[test]
    fn test_id_suffix() {
        use crate::builder::Badge;
        for style in [BadgeStyle::Flat, BadgeStyle::Plastic] {
            let svg = render_badge_svg_with(
                &BadgeParams {
                    style,
                    label: Some("a"),
                    message: Some("b"),
                    label_color: None,
                    message_color: None,
                    link: None,
                    extra_link: None,
                    logo: None,
                    logo_color: None,
                },
                &RenderOptions::default().id_suffix("x1"),
            );
            assert!(svg.contains(r##"id="sx1""##), "{style:?}: {svg}");
            assert!(svg.contains(r##"url(#sx1)"##), "{style:?}");
            assert!(svg.contains(r##"id="rx1""##), "{style:?}");
            assert!(svg.contains(r##"url(#rx1)"##), "{style:?}");
            assert!(!svg.contains(r##"id="s" "##), "{style:?}");
        }

        let svg = Badge::style(BadgeStyle::Social)
            .label("a")
            .message("b")
            .id_suffix("x1")
            .build();
        for needle in [
            r##"id="ax1""##,
            r##"id="bx1""##,
            r##"id="llinkx1""##,
            r##"id="rlinkx1""##,
            r##"url(#ax1)"##,
            "a:hover #llinkx1{fill:url(#bx1);stroke:#ccc}a:hover #rlinkx1{fill:#4183c4}",
        ] {
            assert!(svg.contains(needle), "missing {needle} in {svg}");
        }

        // Unsafe characters are stripped, and the default is suffix-free
        let svg = render_badge_svg_with(
            &BadgeParams {
                style: BadgeStyle::Flat,
                label: Some("a"),
                message: Some("b"),
                label_color: None,
                message_color: None,
                link: None,
                extra_link: None,
                logo: None,
                logo_color: None,
            },
            &RenderOptions::default().id_suffix("x\"><script>1"),
        );
        assert!(svg.contains(r##"id="sxscript1""##));
        let default_svg = render_badge_svg(&BadgeParams {
            style: BadgeStyle::Flat,
            label: Some("a"),
            message: Some("b"),
            label_color: None,
            message_color: None,
            link: None,
            extra_link: None,
            logo: None,
            logo_color: None,
        });
        assert!(default_svg.contains(r##"id="s""##));
    }

    #[test]
    fn test_logo_width() {
        let params = BadgeParams {
            style: BadgeStyle::Flat,
            label: Some("build"),
            message: Some("passing"),
            label_color: None,
            message_color: None,
            link: None,
            extra_link: None,
            logo: Some("rust"),
            logo_color: None,
        };
        let default_svg = render_badge_svg(&params);
        let wide_svg = render_badge_svg_with(&params, &RenderOptions::default().logo_width(30));
        assert!(default_svg.contains(r#"width="14" height="14""#));
        assert!(wide_svg.contains(r#"width="30" height="14""#));

        let width_of = |svg: &str| -> u32 {
            let start = svg.find("width=\"").unwrap() + 7;
            let end = svg[start..].find('"').unwrap() + start;
            svg[start..end].parse().unwrap()
        };
        // totalLogoWidth = logoWidth + logoPadding, so +16px logo -> +16px badge
        assert_eq!(width_of(&wide_svg), width_of(&default_svg) + 16);
    }

    #[test]
    fn test_custom_svg_logo() {
        let custom_svg = "<svg width=\"377\" height=\"377\" viewBox=\"0 0 377 377\" xmlns=\"http://www.w3.org/2000/svg\">\
<circle cx=\"188.5\" cy=\"188.5\" r=\"172.5\" fill=\"#D9D9D9\" stroke=\"#1874A8\" stroke-width=\"32\"/>\
<circle cx=\"188.5\" cy=\"188.5\" r=\"172.5\" fill=\"#D9D9D9\" stroke=\"#1874A8\" stroke-width=\"32\"/>\
<path d=\"M289.352 113L307.016 140.904L223.944 189.416L307.016 237.032L288.712 265.832L189 203.88V175.208L289.352 113Z\" fill=\"#2E2E2E\"/>\
</svg>";

        let params = BadgeParams {
            style: BadgeStyle::Flat,
            label: Some("custom"),
            message: Some("logo"),
            label_color: Some("#333"),
            message_color: Some("#4c1"),
            link: None,
            extra_link: None,
            logo: Some(custom_svg),
            logo_color: Some("#1874A8"),
        };

        let svg = render_badge_svg(&params);
        // Test that the badge contains expected text
        assert!(svg.contains("custom"), "Badge should contain 'custom' text");
        assert!(svg.contains("logo"), "Badge should contain 'logo' text");

        // Test that SVG contains custom logo (base64 encoded)
        assert!(
            svg.contains("data:image/svg+xml;base64,"),
            "SVG should contain base64 encoded custom logo"
        );

        // Test that the logo color is applied to the custom SVG (in lowercase)
        let encoded_svg = base64::engine::general_purpose::STANDARD
            .encode(custom_svg.replace("<svg", &format!("<svg fill=\"{}\"", "#1874a8")));
        assert!(
            svg.contains(&encoded_svg),
            "SVG should contain custom logo with applied color"
        );

        assert!(!svg.is_empty(), "Generated SVG should not be empty");
    }

    const ALL_STYLES: [BadgeStyle; 5] = [
        BadgeStyle::Flat,
        BadgeStyle::FlatSquare,
        BadgeStyle::Plastic,
        BadgeStyle::Social,
        BadgeStyle::ForTheBadge,
    ];

    fn render(style: BadgeStyle, label: &str, message: &str, link: Option<&str>) -> String {
        render_badge_svg(&BadgeParams {
            style,
            label: Some(label),
            message: Some(message),
            label_color: None,
            message_color: None,
            link,
            extra_link: None,
            logo: None,
            logo_color: None,
        })
    }

    #[test]
    fn test_text_is_xml_escaped() {
        // `&`, `<` and `"` are ordinary badge text ("AT&T", "C++ <3"); rendering
        // them raw produced SVG that no XML parser would accept.
        for style in ALL_STYLES {
            // Social and for-the-badge recase the label, so match on the
            // case-insensitive form; what matters is that `&` and `<` are entities.
            let svg = render(style, "AT&T <3", "a\"b'c", None).to_lowercase();
            assert!(svg.contains("at&amp;t &lt;3"), "{style:?}: {svg}");
            assert!(svg.contains("a&quot;b&apos;c"), "{style:?}: {svg}");
            // No raw special character survives into the markup.
            assert!(!svg.contains("at&t"), "{style:?}: {svg}");
            assert!(!svg.contains("<3"), "{style:?}: {svg}");
        }
    }

    #[test]
    fn test_text_cannot_break_out_of_attribute_or_element() {
        for style in ALL_STYLES {
            let svg = render(
                style,
                "\" onload=\"PWN",
                "</text><script>x</script><text>",
                None,
            );
            assert!(!svg.contains("onload=\"PWN\""), "{style:?}: {svg}");
            assert!(!svg.contains("<script>"), "{style:?}: {svg}");
        }
    }

    #[test]
    fn test_link_scheme_is_filtered() {
        for link in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "  \t javascript:alert(1)",
            "java\tscript:alert(1)",
            "vbscript:msgbox(1)",
            "data:text/html,<script>x</script>",
        ] {
            assert!(!is_safe_link(link), "{link:?} should be rejected");
            for style in ALL_STYLES {
                let svg = render(style, "a", "b", Some(link));
                assert!(!svg.contains("href=\""), "{style:?} {link:?}: {svg}");
            }
        }

        // Ordinary links keep working, including relative ones with a colon.
        for link in [
            "https://example.com/x?a=1&b=2",
            "/relative/path",
            "#frag",
            "mailto:a@b.com",
            "/path:with:colon",
        ] {
            assert!(is_safe_link(link), "{link:?} should be allowed");
        }
        let svg = render(BadgeStyle::Flat, "a", "b", Some("https://e.com/?a=1&b=2"));
        assert!(svg.contains("href=\"https://e.com/?a=1&amp;b=2\""), "{svg}");
    }

    #[test]
    fn test_escaping_preserves_logo_and_font() {
        // The logo data URI is base64 (no escapable characters) and the font
        // stack has none either; escaping must leave both byte-identical.
        let svg = render_badge_svg(&BadgeParams {
            style: BadgeStyle::Flat,
            label: Some("a"),
            message: Some("b"),
            label_color: None,
            message_color: None,
            link: None,
            extra_link: None,
            logo: Some("rust"),
            logo_color: None,
        });
        assert!(svg.contains("href=\"data:image/svg+xml;base64,"), "{svg}");
        // Nothing was escaped at all: no entity appears anywhere in the output.
        assert!(!svg.contains('&'), "logo/font must not be altered: {svg}");
        assert!(
            svg.contains(&format!("font-family=\"{FONT_FAMILY}\"")),
            "{svg}"
        );
    }
}
