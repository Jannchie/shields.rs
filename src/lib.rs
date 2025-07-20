#![doc = r#"
# shields

A Rust library for generating SVG badges, inspired by [shields.io](https://shields.io/).

This crate provides flexible APIs for creating customizable status badges for CI, version, downloads, and more, supporting multiple styles (flat, plastic, social, for-the-badge, etc.).

## Features

- Generate SVG badge strings with custom label, message, color, logo, and links.
- Multiple badge styles: flat, flat-square, plastic, social, for-the-badge.
- Accurate text width calculation using embedded font width tables.
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
use askama::{Template, filters::capitalize};
use std::str::FromStr;
pub mod builder;
pub mod measurer;
use base64::Engine;
use color_util::to_svg_color;
use csscolorparser::Color;
use serde::Deserialize;

/// SVG rendering template context, fields must correspond to variables in badge_svg_template_askama.svg
#[derive(Template)]
#[template(path = "flat_badge_template.min.svg", escape = "none")]
struct FlatBadgeSvgTemplateContext<'a> {
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
#[template(path = "flat_square_badge_template.min.svg", escape = "none")]
struct FlatSquareBadgeSvgTemplateContext<'a> {
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
#[template(path = "plastic_badge_template.min.svg", escape = "none")]
struct PlasticBadgeSvgTemplateContext<'a> {
    total_width: i32,
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
#[template(path = "social_badge_template.min.svg", escape = "none")]
struct SocialBadgeSvgTemplateContext<'a> {
    total_width: i32,
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
#[template(path = "for_the_badge_template.min.svg", escape = "none")]
struct ForTheBadgeSvgTemplateContext<'a> {
    // SVG dimensions
    total_width: i32,

    // Accessibility
    accessible_text: &'a str,

    // Layout dimensions
    left_width: i32,
    right_width: i32,

    // Colors
    label_color: &'a str,
    message_color: &'a str,

    // Font settings
    font_family: &'a str,
    font_size: i32,

    // Label (left side)
    label: &'a str,
    label_x: f32,
    label_width_scaled: i32,
    label_text_color: &'a str,

    // Message (right side)
    message: &'a str,
    message_x: f32,
    message_text_color: &'a str,
    message_width_scaled: i32,

    // Links
    link: &'a str,
    extra_link: &'a str,

    // Logo
    logo: &'a str,
    logo_x: i32,
}

// --- Color processing utility module ---
// Supports standardization and SVG output of named colors, aliases, hex, and CSS color inputs

mod color_util {
    use csscolorparser::Color;
    use lru::LruCache;
    use once_cell::sync::Lazy;
    use std::collections::HashMap;
    use std::num::NonZeroUsize;
    use std::str::FromStr;
    use std::sync::Mutex;

    // Named color mapping
    pub static NAMED_COLORS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
        HashMap::from([
            ("brightgreen", "#4c1"),
            ("green", "#97ca00"),
            ("yellow", "#dfb317"),
            ("yellowgreen", "#a4a61d"),
            ("orange", "#fe7d37"),
            ("red", "#e05d44"),
            ("blue", "#007ec6"),
            ("grey", "#555"),
            ("lightgrey", "#9f9f9f"),
        ])
    });

    // Alias mapping
    pub static ALIASES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
        HashMap::from([
            ("gray", "grey"),
            ("lightgray", "lightgrey"),
            ("critical", "red"),
            ("important", "orange"),
            ("success", "brightgreen"),
            ("informational", "blue"),
            ("inactive", "lightgrey"),
        ])
    });

    // 3/6 digit hex validation
    pub fn is_valid_hex(s: &str) -> bool {
        let s = s.trim_start_matches('#');
        let len = s.len();
        (len == 3 || len == 6) && s.chars().all(|c| c.is_ascii_hexdigit())
    }

    // Simplified CSS color validation (supports rgb(a), hsl(a), common formats)
    pub fn is_css_color(s: &str) -> bool {
        Color::from_str(s).is_ok()
    }

    /// Standardizes color input, returning a string usable in SVG or None
    pub fn normalize_color(color: &str) -> Option<String> {
        static CACHE: Lazy<Mutex<LruCache<String, Option<String>>>> =
            Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(512).unwrap())));
        let color = color.trim();
        if color.is_empty() {
            return None;
        }
        let key = color.to_ascii_lowercase();
        // Check cache first
        if let Some(cached) = {
            let mut cache = CACHE.lock().unwrap();
            cache.get(&key).cloned()
        } {
            return cached;
        }
        // Allocate only if there are uppercase letters
        let lower = color.to_ascii_lowercase();
        let result = if NAMED_COLORS.contains_key(lower.as_str()) {
            Some(lower.to_string())
        } else if let Some(&alias) = ALIASES.get(lower.as_str()) {
            Some(alias.to_string())
        } else if is_valid_hex(lower.as_str()) {
            let hex = lower.trim_start_matches('#');
            Some(format!("#{}", hex))
        } else if is_css_color(lower.as_str()) {
            Some(lower.to_string())
        } else {
            None
        };
        let mut cache = CACHE.lock().unwrap();
        cache.put(key, result.clone());
        result
    }

    /// Outputs SVG-compatible color (hex string), prioritizing named colors and aliases, otherwise original
    pub fn to_svg_color(color: &str) -> Option<String> {
        static CACHE: Lazy<Mutex<LruCache<String, Option<String>>>> =
            Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(256).unwrap())));
        let key = color.to_ascii_lowercase();
        if let Some(cached) = {
            let mut cache = CACHE.lock().unwrap();
            cache.get(&key).cloned()
        } {
            return cached;
        }
        let normalized = normalize_color(color)?;
        let result = if let Some(&hex) = NAMED_COLORS.get(normalized.as_str()) {
            Some(hex.to_string())
        } else if let Some(&alias) = ALIASES.get(normalized.as_str()) {
            NAMED_COLORS.get(alias).map(|&h| h.to_string())
        } else {
            Some(normalized)
        };
        let mut cache = CACHE.lock().unwrap();
        cache.put(key, result.clone());
        result
    }
}
/// Font width calculation trait, to be implemented and injected by the main project
pub trait FontMetrics {
    /// Supports font-family fallback
    fn get_text_width_px(&self, text: &str, font_family: &str) -> f32;
}

/// Font enumeration for supported fonts
#[derive(Eq, PartialEq, Hash, Clone, Debug)]
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

/// Calculates the width of text in Verdana 11px (in pixels)
///
/// - Only the text needs to be passed in, the width table is loaded and reused internally
/// - Efficient lazy initialization to avoid repeated IO
/// - Can be directly used in scenarios like SVG badges
pub fn get_text_width(text: &str, font: Font) -> f64 {
    use crate::measurer::CharWidthMeasurer;
    use once_cell::sync::Lazy;

    // 在编译时直接将 JSON 文件内容作为字符串嵌入
    const VERDANA_11_N_JSON_DATA: &str = include_str!("../assets/fonts/verdana-11px-normal.json");
    const HELVETICA_11_B_JSON_DATA: &str = include_str!("../assets/fonts/helvetica-11px-bold.json");
    const VERDANA_10_N_JSON_DATA: &str = include_str!("../assets/fonts/verdana-10px-normal.json");
    const VERDANA_10_B_JSON_DATA: &str = include_str!("../assets/fonts/verdana-10px-bold.json");
    static VERDANA_11_N_WIDTH_TABLE: Lazy<CharWidthMeasurer> = Lazy::new(|| {
        // 从嵌入的字符串加载数据，而不是从文件系统
        CharWidthMeasurer::load_from_str(VERDANA_11_N_JSON_DATA)
            .expect("Unable to parse Verdana 11px width table")
    });

    static HELVETICA_11_B_WIDTH_TABLE: Lazy<CharWidthMeasurer> = Lazy::new(|| {
        // 从嵌入的字符串加载数据
        CharWidthMeasurer::load_from_str(HELVETICA_11_B_JSON_DATA)
            .expect("Unable to parse Helvetica Bold width table")
    });
    static VERDANA_10_N_WIDTH_TABLE: Lazy<CharWidthMeasurer> = Lazy::new(|| {
        CharWidthMeasurer::load_from_str(VERDANA_10_N_JSON_DATA)
            .expect("Unable to parse Verdana 10px width table")
    });

    static VERDANA_10_B_WIDTH_TABLE: Lazy<CharWidthMeasurer> = Lazy::new(|| {
        CharWidthMeasurer::load_from_str(VERDANA_10_B_JSON_DATA)
            .expect("Unable to parse Verdana 10px Bold width table")
    });

    match font {
        Font::VerdanaNormal11 => VERDANA_11_N_WIDTH_TABLE.width_of(text, true),
        Font::HelveticaBold11 => HELVETICA_11_B_WIDTH_TABLE.width_of(text, true),
        Font::VerdanaNormal10 => VERDANA_10_N_WIDTH_TABLE.width_of(text, true),
        Font::VerdanaBold10 => VERDANA_10_B_WIDTH_TABLE.width_of(text, true),
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
    // Parse RGB
    let (r, g, b) = match hex.len() {
        3 => (
            {
                let c = hex.as_bytes()[0];
                let v = match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    _ => 0,
                };
                (v << 4) | v
            },
            {
                let c = hex.as_bytes()[1];
                let v = match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    _ => 0,
                };
                (v << 4) | v
            },
            {
                let c = hex.as_bytes()[2];
                let v = match c {
                    b'0'..=b'9' => c - b'0',
                    b'a'..=b'f' => c - b'a' + 10,
                    b'A'..=b'F' => c - b'A' + 10,
                    _ => 0,
                };
                (v << 4) | v
            },
        ),
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
    use lru::LruCache;
    use once_cell::sync::Lazy;
    use std::num::NonZeroUsize;
    use std::sync::Mutex;

    // Create a cache that includes font information in the key
    static WIDTH_CACHE: Lazy<Mutex<LruCache<(String, Font), u32>>> =
        Lazy::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(1024).unwrap())));

    let cache_key = (text.to_string(), font.clone());

    {
        let mut cache = WIDTH_CACHE.lock().unwrap();
        if let Some(&cached) = cache.get(&cache_key) {
            return cached;
        }
    }

    let width = get_text_width(text, font);
    let rounded = round_up_to_odd_f64(width);

    if text.len() <= 1024 {
        let mut cache = WIDTH_CACHE.lock().unwrap();
        cache.put(cache_key, rounded);
    }

    rounded
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
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

impl Default for BadgeStyle {
    /// Returns the default badge style (`Flat`).
    fn default() -> Self {
        BadgeStyle::Flat
    }
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
    let logo_color = to_svg_color(logo_color).unwrap_or(default_logo_color.to_string());
    let icon_svg = match logo {
        Some(logo) => {
            let logo = logo.trim();
            if logo.is_empty() {
                ""
            } else if logo.starts_with("<svg") {
                logo
            } else {
                // let logo_color = logo_color.unwrap_or("#555");
                // let icon = to_svg_color(logo_color).unwrap_or("#555".to_string());
                let icon = logo;
                let svg = simpleicons::Icon::get_svg(icon);
                svg.unwrap_or_default()
            }
        }
        None => "",
    };
    // 如果 logo 为 <svg 开头，则需要获取 base64 编码
    let logo = if icon_svg.starts_with("<svg") {
        // 只检查 <svg> 标签内是否有 fill 属性，且 logo_color 不为空，则添加 fill 属性
        let svg_tag_end = icon_svg.find('>').unwrap_or(0);
        let svg_tag = &icon_svg[..svg_tag_end];
        let has_fill_in_svg_tag = svg_tag.contains("fill=");
        let logo_svg = if !has_fill_in_svg_tag && !logo_color.is_empty() {
            icon_svg.replace("<svg", format!("<svg fill=\"{}\"", logo_color).as_str())
        } else {
            icon_svg.to_string()
        };
        let base64_logo = base64::engine::general_purpose::STANDARD.encode(logo_svg);
        format!("data:image/svg+xml;base64,{}", base64_logo)
    } else {
        icon_svg.to_string()
    };
    let has_logo = !logo.is_empty();
    let logo_width = 14;
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
    let message_color = to_svg_color(message_color).unwrap_or("#007ec6".to_string());

    let label_color = match (
        label.unwrap_or("").is_empty(),
        label_color.unwrap_or("").is_empty(),
    ) {
        (true, true) if has_logo => "#555",
        (true, true) => message_color.as_str(),
        (_, _) => label_color.unwrap_or(default_label_color()),
    };

    let binding = to_svg_color(label_color).unwrap_or("#555".to_string());
    let label_color = binding.as_str();

    let message_color = message_color.as_str();
    let message = message.unwrap_or("");
    let link = link.unwrap_or("");
    let extra_link_not_empty_str = extra_link.is_none() || !extra_link.unwrap().is_empty();
    let extra_link = extra_link.unwrap_or("");
    let logo = logo.as_str();
    match style {
        BadgeStyle::Flat => {
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

            if has_label && label.is_some() {
                let label = label.unwrap();
                if label.is_empty() {
                    left_width -= 1;
                }
            }
            let message_width = preferred_width_of(message, Font::VerdanaNormal11);

            let offset = if label.is_none() && has_logo {
                -3i32
            } else {
                0
            };

            let left_width = left_width + offset as i32;
            let mut message_margin: i32 =
                left_width as i32 - if message.is_empty() { 0 } else { 1 };
            if !has_label {
                if has_logo {
                    message_margin += (total_logo_width + HORIZONTAL_PADDING) as i32
                } else {
                    message_margin += 1
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
            let label_width_scaled = label_width * 10;
            let total_width = left_width + right_width as i32;

            let right_width = right_width + if !has_label_color { offset } else { 0 };
            let hex_label_color = Color::from_str(label_color)
                .unwrap_or(Color::from_str("#555").unwrap())
                .to_css_hex();
            let hex_label_color = hex_label_color.as_str();
            let hex_message_color = Color::from_str(message_color)
                .unwrap_or(Color::from_str("#007ec6").unwrap())
                .to_css_hex();
            let hex_message_color = hex_message_color.as_str();
            let (label_text_color, label_shadow_color) = colors_for_background(hex_label_color);
            let (message_text_color, message_shadow_color) =
                colors_for_background(hex_message_color);
            let rect_offset = if has_logo { 19 } else { 0 };

            let message_link_x = if has_logo && !has_label && extra_link_not_empty_str {
                total_logo_width as i32 + HORIZONTAL_PADDING as i32
            } else {
                left_width
            };

            let has_extra_link = !extra_link.is_empty();
            let message_x = 10.0
                * (message_margin as f32
                    + (0.5 * message_width as f32)
                    + HORIZONTAL_PADDING as f32);
            let message_link_x = message_link_x
                + if !has_label && has_extra_link {
                    offset
                } else {
                    0
                } as i32;
            let message_width_scaled = message_width * 10;
            let left_width = if left_width < 0 { 0 } else { left_width };
            FlatBadgeSvgTemplateContext {
                font_family: FONT_FAMILY,

                accessible_text: accessible_text.as_str(),
                badge_height: BADGE_HEIGHT as i32,

                left_width: left_width as i32,
                right_width: right_width as i32,
                total_width: total_width as i32,

                label_color,
                message_color,

                font_size_scaled: FONT_SIZE_SCALED as i32,

                label: label.unwrap_or(""),
                label_x,
                label_width_scaled: label_width_scaled as i32,
                label_text_color,
                label_shadow_color,

                message_x,
                message_shadow_color,
                message_text_color,
                message_width_scaled: message_width_scaled as i32,
                message,

                link,
                extra_link,
                logo,

                rect_offset,
                message_link_x,
            }
            .render()
            .unwrap_or_else(|e| format!("<!-- Askama render error: {} -->", e))
        }
        BadgeStyle::FlatSquare => {
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

            if has_label && label.is_some() {
                let label = label.unwrap();
                if label.is_empty() {
                    left_width -= 1;
                }
            }
            let message_width = preferred_width_of(message, Font::VerdanaNormal11);

            let offset = if label.is_none() && has_logo {
                -3i32
            } else {
                0
            };

            let left_width = left_width + offset as i32;
            let mut message_margin: i32 =
                left_width as i32 - if message.is_empty() { 0 } else { 1 };
            if !has_label {
                if has_logo {
                    message_margin += (total_logo_width + HORIZONTAL_PADDING) as i32
                } else {
                    message_margin += 1
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
            let label_width_scaled = label_width * 10;
            let total_width = left_width + right_width as i32;

            let right_width = right_width + if !has_label_color { offset } else { 0 };
            let hex_label_color = Color::from_str(label_color)
                .unwrap_or(Color::from_str("#555").unwrap())
                .to_css_hex();
            let hex_label_color = hex_label_color.as_str();
            let hex_message_color = Color::from_str(message_color)
                .unwrap_or(Color::from_str("#007ec6").unwrap())
                .to_css_hex();
            let hex_message_color = hex_message_color.as_str();
            let (label_text_color, _) = colors_for_background(hex_label_color);
            let (message_text_color, _) = colors_for_background(hex_message_color);
            let rect_offset = if has_logo { 19 } else { 0 };

            let message_link_x = if has_logo && !has_label && extra_link_not_empty_str {
                total_logo_width as i32 + HORIZONTAL_PADDING as i32
            } else {
                left_width
            };

            let has_extra_link = !extra_link.is_empty();
            let message_x = 10.0
                * (message_margin as f32
                    + (0.5 * message_width as f32)
                    + HORIZONTAL_PADDING as f32);
            let message_link_x = message_link_x
                + if !has_label && has_extra_link {
                    offset
                } else {
                    0
                } as i32;
            let message_width_scaled = message_width * 10;
            let left_width = if left_width < 0 { 0 } else { left_width };
            FlatSquareBadgeSvgTemplateContext {
                font_family: FONT_FAMILY,
                accessible_text: accessible_text.as_str(),
                badge_height: BADGE_HEIGHT as i32,
                left_width,
                right_width,
                total_width,
                label_color,
                message_color,
                font_size_scaled: FONT_SIZE_SCALED as i32,
                label: label.unwrap_or(""),
                label_x,
                label_width_scaled: label_width_scaled as i32,
                label_text_color,
                message_x,
                message_text_color,
                message_width_scaled: message_width_scaled as i32,
                message,
                link,
                extra_link,
                logo,
                rect_offset,
                message_link_x,
            }
            .render()
            .unwrap_or_else(|e| format!("<!-- Askama render error: {} -->", e))
        }
        BadgeStyle::Plastic => {
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

            if has_label && label.is_some() {
                let label = label.unwrap();
                if label.is_empty() {
                    left_width -= 1;
                }
            }
            let message_width = preferred_width_of(message, Font::VerdanaNormal11);

            let offset = if label.is_none() && has_logo {
                -3i32
            } else {
                0
            };

            let left_width = left_width + offset as i32;
            let mut message_margin: i32 =
                left_width as i32 - if message.is_empty() { 0 } else { 1 };
            if !has_label {
                if has_logo {
                    message_margin += (total_logo_width + HORIZONTAL_PADDING) as i32;
                } else {
                    message_margin += 1
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
            let label_width_scaled = label_width * 10;
            let total_width = left_width + right_width as i32;

            let right_width = right_width + if !has_label_color { offset } else { 0 };
            let hex_label_color = Color::from_str(label_color)
                .unwrap_or(Color::from_str("#555").unwrap())
                .to_css_hex();
            let hex_label_color = hex_label_color.as_str();
            let hex_message_color = Color::from_str(message_color)
                .unwrap_or(Color::from_str("#007ec6").unwrap())
                .to_css_hex();
            let hex_message_color = hex_message_color.as_str();
            let (label_text_color, label_shadow_color) = colors_for_background(hex_label_color);
            let (message_text_color, message_shadow_color) =
                colors_for_background(hex_message_color);
            let rect_offset = if has_logo { 19 } else { 0 };

            let message_link_x = if has_logo && !has_label && extra_link_not_empty_str {
                total_logo_width as i32 + HORIZONTAL_PADDING as i32
            } else {
                left_width
            };

            let has_extra_link = !extra_link.is_empty();
            let message_x = 10.0
                * (message_margin as f32
                    + (0.5 * message_width as f32)
                    + HORIZONTAL_PADDING as f32);
            let message_link_x = message_link_x
                + if !has_label && has_extra_link {
                    offset
                } else {
                    0
                } as i32;
            let message_width_scaled = message_width * 10;
            let left_width = if left_width < 0 { 0 } else { left_width };
            PlasticBadgeSvgTemplateContext {
                total_width,
                left_width,
                right_width,
                accessible_text: accessible_text.as_str(),
                label: label.unwrap_or(""),
                label_x,
                label_text_length: label_width_scaled as i32,
                label_text_color,
                label_shadow_color,
                message,
                message_x,
                message_text_length: message_width_scaled as i32,
                message_text_color,
                message_shadow_color,
                label_color,
                message_color,
                link,
                extra_link,
                logo,
                rect_offset,
                message_link_x,
            }
            .render()
            .unwrap_or_else(|e| format!("<!-- Askama render error: {} -->", e))
        }
        BadgeStyle::Social => {
            let label_is_none = label.is_none();

            let offset = if label_is_none && has_logo {
                -3i32
            } else {
                0i32
            };

            let label = label.unwrap_or("");
            let label = capitalize(label).unwrap().to_string();
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

            let total_width = left_width + right_width as i32;

            SocialBadgeSvgTemplateContext {
                total_width,
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
            }
            .render()
            .unwrap_or_else(|e| format!("<!-- Askama render error: {} -->", e))
        }
        BadgeStyle::ForTheBadge => {
            // label to uppercase
            let label = label.unwrap_or("").to_uppercase();
            let accessible_text = create_accessible_text(Some(label.as_str()), message);
            let message = message.to_uppercase();
            let font_size = 10;
            let letter_spacing = 1.25;
            let logo_text_gutter = 6i32;
            let logo_margin = 9i32;
            let logo_width = logo_width as i32;
            let label_text_width = if !label.is_empty() {
                (get_text_width(&label, Font::VerdanaNormal10)
                    + letter_spacing * label.len() as f64) as i32
            } else {
                0
            };
            let message_text_width = if !message.is_empty() {
                (get_text_width(&message, Font::VerdanaBold10)
                    + letter_spacing * message.len() as f64) as i32
            } else {
                0
            };
            let has_label = !label.is_empty();
            let no_text = !has_label && message.is_empty();
            let need_label_rect = has_label || (!logo.is_empty() && !label_color.is_empty());
            let gutter = if no_text {
                logo_text_gutter - logo_margin
            } else {
                logo_text_gutter
            };
            let text_margin = 12;

            // Logo positioning
            let (logo_min_x, label_text_min_x) = if !logo.is_empty() {
                (logo_margin, logo_margin + logo_width + gutter)
            } else {
                (0, text_margin)
            };

            // Handle label and message rectangles
            let (label_rect_width, message_text_min_x, message_rect_width) = if need_label_rect {
                if has_label {
                    (
                        label_text_min_x + label_text_width + text_margin,
                        label_text_min_x + label_text_width + text_margin + text_margin,
                        2 * text_margin + message_text_width,
                    )
                } else {
                    (
                        2 * logo_margin + logo_width,
                        2 * logo_margin + logo_width + text_margin,
                        2 * text_margin + message_text_width,
                    )
                }
            } else if !logo.is_empty() {
                (
                    0,
                    text_margin + logo_width + gutter,
                    2 * text_margin + logo_width + gutter + message_text_width,
                )
            } else {
                (0, text_margin, 2 * text_margin + message_text_width)
            };
            let left_width = label_rect_width;
            let right_width = message_rect_width;
            let total_width = left_width + right_width;

            let hex_label_color = Color::from_str(label_color)
                .unwrap_or(Color::from_str("#555").unwrap())
                .to_css_hex();
            let hex_label_color = hex_label_color.as_str();
            let hex_message_color = Color::from_str(message_color)
                .unwrap_or(Color::from_str("#007ec6").unwrap())
                .to_css_hex();
            let hex_message_color = hex_message_color.as_str();

            let message_mid_x = message_text_min_x as f32 + 0.5 * message_text_width as f32;
            let label_mid_x = label_text_min_x as f32 + 0.5 * label_text_width as f32;

            let (label_text_color, _) = colors_for_background(hex_label_color);
            let (message_text_color, _) = colors_for_background(hex_message_color);

            ForTheBadgeSvgTemplateContext {
                total_width,
                accessible_text: accessible_text.as_str(),
                left_width: label_rect_width,
                right_width: message_rect_width,
                label_color,
                message_color,
                font_family: FONT_FAMILY,
                font_size: font_size * FONT_SCALE_UP_FACTOR as i32,
                label: label.as_str(),
                label_x: label_mid_x * FONT_SCALE_UP_FACTOR as f32,
                label_width_scaled: label_text_width * FONT_SCALE_UP_FACTOR as i32,
                label_text_color,
                message: message.as_str(),
                message_x: message_mid_x * FONT_SCALE_UP_FACTOR as f32,
                message_text_color,
                message_width_scaled: message_text_width * FONT_SCALE_UP_FACTOR as i32,
                link,
                extra_link,
                logo,
                logo_x: logo_min_x,
            }
            .render()
            .unwrap_or_else(|e| format!("<!-- Askama render error: {} -->", e))
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
        println!("{}", svg);
        let expected = r##"<svg xmlns="http://www.w3.org/2000/svg" width="160" height="28"><g shape-rendering="crispEdges"><rect width="102" height="28" fill="#555"/><rect x="102" width="58" height="28" fill="#fff"/></g><g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" text-rendering="geometricPrecision" font-size="100"><image x="9" y="7" width="14" height="14" href="data:image/svg+xml;base64,PHN2ZyBmaWxsPSIjMDA3ZWM2IiByb2xlPSJpbWciIHZpZXdCb3g9IjAgMCAyNCAyNCIgeG1sbnM9Imh0dHA6Ly93d3cudzMub3JnLzIwMDAvc3ZnIj48dGl0bGU+UnVzdDwvdGl0bGU+PHBhdGggZD0iTTIzLjgzNDYgMTEuNzAzM2wtMS4wMDczLS42MjM2YTEzLjcyNjggMTMuNzI2OCAwIDAwLS4wMjgzLS4yOTM2bC44NjU2LS44MDY5YS4zNDgzLjM0ODMgMCAwMC0uMTE1NC0uNTc4bC0xLjEwNjYtLjQxNGE4LjQ5NTggOC40OTU4IDAgMDAtLjA4Ny0uMjg1NmwuNjkwNC0uOTU4N2EuMzQ2Mi4zNDYyIDAgMDAtLjIyNTctLjU0NDZsLTEuMTY2My0uMTg5NGE5LjM1NzQgOS4zNTc0IDAgMDAtLjE0MDctLjI2MjJsLjQ5LTEuMDc2MWEuMzQzNy4zNDM3IDAgMDAtLjAyNzQtLjMzNjEuMzQ4Ni4zNDg2IDAgMDAtLjMwMDYtLjE1NGwtMS4xODQ1LjA0MTZhNi43NDQ0IDYuNzQ0NCAwIDAwLS4xODczLS4yMjY4bC4yNzIzLTEuMTUzYS4zNDcyLjM0NzIgMCAwMC0uNDE3LS40MTcybC0xLjE1MzIuMjcyNGExNC4wMTgzIDE0LjAxODMgMCAwMC0uMjI3OC0uMTg3M2wuMDQxNS0xLjE4NDVhLjM0NDIuMzQ0MiAwIDAwLS40OS0uMzI4bC0xLjA3Ni40OTFjLS4wODcyLS4wNDc2LS4xNzQyLS4wOTUyLS4yNjIzLS4xNDA3bC0uMTkwMy0xLjE2NzNBLjM0ODMuMzQ4MyAwIDAwMTYuMjU2Ljk1NWwtLjk1OTcuNjkwNWE4LjQ4NjcgOC40ODY3IDAgMDAtLjI4NTUtLjA4NmwtLjQxNC0xLjEwNjZhLjM0ODMuMzQ4MyAwIDAwLS41NzgxLS4xMTU0bC0uODA2OS44NjY2YTkuMjkzNiA5LjI5MzYgMCAwMC0uMjkzNi0uMDI4NEwxMi4yOTQ2LjE2ODNhLjM0NjIuMzQ2MiAwIDAwLS41ODkyIDBsLS42MjM2IDEuMDA3M2ExMy43MzgzIDEzLjczODMgMCAwMC0uMjkzNi4wMjg0TDkuOTgwMy4zMzc0YS4zNDYyLjM0NjIgMCAwMC0uNTc4LjExNTRsLS40MTQxIDEuMTA2NWMtLjA5NjIuMDI3NC0uMTkwMy4wNTY3LS4yODU1LjA4Nkw3Ljc0NC45NTVhLjM0ODMuMzQ4MyAwIDAwLS41NDQ3LjIyNThMNy4wMDkgMi4zNDhhOS4zNTc0IDkuMzU3NCAwIDAwLS4yNjIyLjE0MDdsLTEuMDc2Mi0uNDkxYS4zNDYyLjM0NjIgMCAwMC0uNDkuMzI4bC4wNDE2IDEuMTg0NWE3Ljk4MjYgNy45ODI2IDAgMDAtLjIyNzguMTg3M0wzLjg0MTMgMy40MjVhLjM0NzIuMzQ3MiAwIDAwLS40MTcxLjQxNzFsLjI3MTMgMS4xNTMxYy0uMDYyOC4wNzUtLjEyNTUuMTUwOS0uMTg2My4yMjY4bC0xLjE4NDUtLjA0MTVhLjM0NjIuMzQ2MiAwIDAwLS4zMjguNDlsLjQ5MSAxLjA3NjFhOS4xNjcgOS4xNjcgMCAwMC0uMTQwNy4yNjIybC0xLjE2NjIuMTg5NGEuMzQ4My4zNDgzIDAgMDAtLjIyNTguNTQ0NmwuNjkwNC45NTg3YTEzLjMwMyAxMy4zMDMgMCAwMC0uMDg3LjI4NTVsLTEuMTA2NS40MTRhLjM0ODMuMzQ4MyAwIDAwLS4xMTU1LjU3ODFsLjg2NTYuODA3YTkuMjkzNiA5LjI5MzYgMCAwMC0uMDI4My4yOTM1bC0xLjAwNzMuNjIzNmEuMzQ0Mi4zNDQyIDAgMDAwIC41ODkybDEuMDA3My42MjM2Yy4wMDguMDk4Mi4wMTgyLjE5NjQuMDI4My4yOTM2bC0uODY1Ni44MDc5YS4zNDYyLjM0NjIgMCAwMC4xMTU1LjU3OGwxLjEwNjUuNDE0MWMuMDI3My4wOTYyLjA1NjcuMTkxNC4wODcuMjg1NWwtLjY5MDQuOTU4N2EuMzQ1Mi4zNDUyIDAgMDAuMjI2OC41NDQ3bDEuMTY2Mi4xODkzYy4wNDU2LjA4OC4wOTIyLjE3NTEuMTQwOC4yNjIybC0uNDkxIDEuMDc2MmEuMzQ2Mi4zNDYyIDAgMDAuMzI4LjQ5bDEuMTgzNC0uMDQxNWMuMDYxOC4wNzY5LjEyMzUuMTUyOC4xODczLjIyNzdsLS4yNzEzIDEuMTU0MWEuMzQ2Mi4zNDYyIDAgMDAuNDE3MS40MTYxbDEuMTUzLS4yNzEzYy4wNzUuMDYzOC4xNTEuMTI1NS4yMjc5LjE4NjNsLS4wNDE1IDEuMTg0NWEuMzQ0Mi4zNDQyIDAgMDAuNDkuMzI3bDEuMDc2MS0uNDljLjA4Ny4wNDg2LjE3NDEuMDk1MS4yNjIyLjE0MDdsLjE5MDMgMS4xNjYyYS4zNDgzLjM0ODMgMCAwMC41NDQ3LjIyNjhsLjk1ODctLjY5MDRhOS4yOTkgOS4yOTkgMCAwMC4yODU1LjA4N2wuNDE0IDEuMTA2NmEuMzQ1Mi4zNDUyIDAgMDAuNTc4MS4xMTU0bC44MDc5LS44NjU2Yy4wOTcyLjAxMTEuMTk1NC4wMjAzLjI5MzYuMDI5NGwuNjIzNiAxLjAwNzNhLjM0NzIuMzQ3MiAwIDAwLjU4OTIgMGwuNjIzNi0xLjAwNzNjLjA5ODItLjAwOTEuMTk2NC0uMDE4My4yOTM2LS4wMjk0bC44MDY5Ljg2NTZhLjM0ODMuMzQ4MyAwIDAwLjU3OC0uMTE1NGwuNDE0MS0xLjEwNjZhOC40NjI2IDguNDYyNiAwIDAwLjI4NTUtLjA4N2wuOTU4Ny42OTA0YS4zNDUyLjM0NTIgMCAwMC41NDQ3LS4yMjY4bC4xOTAzLTEuMTY2MmMuMDg4LS4wNDU2LjE3NTEtLjA5MzEuMjYyMi0uMTQwN2wxLjA3NjIuNDlhLjM0NzIuMzQ3MiAwIDAwLjQ5LS4zMjdsLS4wNDE1LTEuMTg0NWE2LjcyNjcgNi43MjY3IDAgMDAuMjI2Ny0uMTg2M2wxLjE1MzEuMjcxM2EuMzQ3Mi4zNDcyIDAgMDAuNDE3MS0uNDE2bC0uMjcxMy0xLjE1NDJjLjA2MjgtLjA3NDkuMTI1NS0uMTUwOC4xODYzLS4yMjc4bDEuMTg0NS4wNDE1YS4zNDQyLjM0NDIgMCAwMC4zMjgtLjQ5bC0uNDktMS4wNzZjLjA0NzUtLjA4NzIuMDk1MS0uMTc0Mi4xNDA3LS4yNjIzbDEuMTY2Mi0uMTg5M2EuMzQ4My4zNDgzIDAgMDAuMjI1OC0uNTQ0N2wtLjY5MDQtLjk1ODcuMDg3LS4yODU1IDEuMTA2Ni0uNDE0YS4zNDYyLjM0NjIgMCAwMC4xMTU0LS41NzgxbC0uODY1Ni0uODA3OWMuMDEwMS0uMDk3Mi4wMjAyLS4xOTU0LjAyODMtLjI5MzZsMS4wMDczLS42MjM2YS4zNDQyLjM0NDIgMCAwMDAtLjU4OTJ6bS02Ljc0MTMgOC4zNTUxYS43MTM4LjcxMzggMCAwMS4yOTg2LTEuMzk2LjcxNC43MTQgMCAxMS0uMjk5NyAxLjM5NnptLS4zNDIyLTIuMzE0MmEuNjQ5LjY0OSAwIDAwLS43NzE1LjVsLS4zNTczIDEuNjY4NWMtMS4xMDM1LjUwMS0yLjMyODUuNzc5NS0zLjYxOTMuNzc5NWE4LjczNjggOC43MzY4IDAgMDEtMy42OTUxLS44MTRsLS4zNTc0LTEuNjY4NGEuNjQ4LjY0OCAwIDAwLS43NzE0LS40OTlsLTEuNDczLjMxNThhOC43MjE2IDguNzIxNiAwIDAxLS43NjEzLS44OThoNy4xNjc2Yy4wODEgMCAuMTM1Ni0uMDE0MS4xMzU2LS4wODh2LTIuNTM2YzAtLjA3NC0uMDUzNi0uMDg4MS0uMTM1Ni0uMDg4MWgtMi4wOTY2di0xLjYwNzdoMi4yNjc3Yy4yMDY1IDAgMS4xMDY1LjA1ODcgMS4zOTQgMS4yMDg4LjA5MDEuMzUzMy4yODc1IDEuNTA0NC40MjMyIDEuODcyOS4xMzQ2LjQxMy42ODMzIDEuMjM4MSAxLjI2ODUgMS4yMzgxaDMuNTcxNmEuNzQ5Mi43NDkyIDAgMDAuMTI5Ni0uMDEzMSA4Ljc4NzQgOC43ODc0IDAgMDEtLjgxMTkuOTUyNnpNNi44MzY5IDIwLjAyNGEuNzE0LjcxNCAwIDExLS4yOTk3LTEuMzk2LjcxNC43MTQgMCAwMS4yOTk3IDEuMzk2ek00LjExNzcgOC45OTcyYS43MTM3LjcxMzcgMCAxMS0xLjMwNC41NzkxLjcxMzcuNzEzNyAwIDAxMS4zMDQtLjU3OXptLS44MzUyIDEuOTgxM2wxLjUzNDctLjY4MjRhLjY1LjY1IDAgMDAuMzMtLjg1ODVsLS4zMTU4LS43MTQ3aDEuMjQzMnY1LjYwMjVIMy41NjY5YTguNzc1MyA4Ljc3NTMgMCAwMS0uMjgzNC0zLjM0OHptNi43MzQzLS41NDM3VjguNzgzNmgyLjk2MDFjLjE1MyAwIDEuMDc5Mi4xNzcyIDEuMDc5Mi44Njk3IDAgLjU3NS0uNzEwNy43ODE1LTEuMjk0OC43ODE1em0xMC43NTc0IDEuNDg2MmMwIC4yMTg3LS4wMDguNDM2My0uMDI0My42NTFoLS45Yy0uMDkgMC0uMTI2NS4wNTg2LS4xMjY1LjE0Nzd2LjQxM2MwIC45NzMtLjU0ODcgMS4xODQ2LTEuMDI5NiAxLjIzODItLjQ1NzYuMDUxNy0uOTY0OC0uMTkxMy0xLjAyNzUtLjQ3MTctLjI3MDQtMS41MTg2LS43MTk4LTEuODQzNi0xLjQzMDUtMi40MDM0Ljg4MTctLjU1OTkgMS43OTktMS4zODYgMS43OTktMi40OTE1IDAtMS4xOTM2LS44MTktMS45NDU4LTEuMzc2OS0yLjMxNTMtLjc4MjUtLjUxNjMtMS42NDkxLS42MTk1LTEuODgzLS42MTk1SDUuNDY4MmE4Ljc2NTEgOC43NjUxIDAgMDE0LjkwNy0yLjc2OTlsMS4wOTc0IDEuMTUxYS42NDguNjQ4IDAgMDAuOTE4Mi4wMjEzbDEuMjI3LTEuMTc0M2E4Ljc3NTMgOC43NzUzIDAgMDE2LjAwNDQgNC4yNzYybC0uODQwMyAxLjg5ODJhLjY1Mi42NTIgMCAwMC4zMy44NTg1bDEuNjE3OC43MTg4Yy4wMjgzLjI4NzUuMDQyNS41NzcuMDQyNS44NzE3em0tOS4zMDA2LTkuNTk5M2EuNzEyOC43MTI4IDAgMTEuOTg0IDEuMDMxNi43MTM3LjcxMzcgMCAwMS0uOTg0LTEuMDMxNnptOC4zMzg5IDYuNzFhLjcxMDcuNzEwNyAwIDAxLjkzOTUtLjM2MjUuNzEzNy43MTM3IDAgMTEtLjk0MDUuMzYzNXoiLz48L3N2Zz4="/><a target="_blank" href="https://google.com"><rect width="102" height="28" fill="rgba(0,0,0,0)"/><text transform="scale(.1)" x="595" y="175" textLength="610" fill="#fff">BUILDING</text></a><a target="_blank" href="https://example.com"><rect width="58" height="28" x="102" fill="rgba(0,0,0,0)"/><text transform="scale(.1)" x="1310" y="175" textLength="340" fill="#333" font-weight="bold">PASS</text></a></g></svg>"##;
        std::fs::write("badge.svg", &svg).unwrap();
        std::fs::write("badge_expected.svg", expected).unwrap();
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
        println!("{:?}", c);

        // 解析HEX
        let c = Color::from_str("#ff0080").unwrap();
        println!("{:?}", c);

        // 解析RGBA
        let c = Color::from_str("rgba(255,255,0,0.75)").unwrap();
        println!("{:?}", c);

        // 解析HSL
        let c = Color::from_str("hsl(120, 100%, 50%)").unwrap();
        println!("{:?}", c);

        let c = Color::from_str("notexists").is_err();
        println!("{:?}", c);
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
}
