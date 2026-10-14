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
pub mod builder;
mod color;
mod logo;
pub mod measurer;
mod out;
mod render;
mod small_str;
use serde::Deserialize;

pub use color::colors_for_background;

/// Font width tables generated at build time from `assets/fonts/*.json`.
mod font_tables {
    // Generated width data contains literals that happen to look like math constants
    #![allow(clippy::approx_constant)]
    include!(concat!(env!("OUT_DIR"), "/font_tables.rs"));
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
    let measurer = match font {
        Font::VerdanaNormal11 => &font_tables::VERDANA_11_NORMAL,
        Font::HelveticaBold11 => &font_tables::HELVETICA_11_BOLD,
        Font::VerdanaNormal10 => &font_tables::VERDANA_10_NORMAL,
        Font::VerdanaBold10 => &font_tables::VERDANA_10_BOLD,
    };
    measurer.width_of(text, true)
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

#[derive(Deserialize, Debug, Clone, Default)]
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
    let mut svg = String::new();
    render::render(params, options, &mut svg);
    svg
}

/// Appends the badge SVG to `out` instead of returning a new `String`.
///
/// Rendering into a buffer the caller reuses (clearing it between badges)
/// saves the one allocation a render otherwise makes, which matters when
/// rendering at high volume.
///
/// ## Example
/// ```rust
/// use shields::{BadgeParams, RenderOptions, render_badge_svg_into};
/// let mut buf = String::new();
/// for message in ["passing", "failing"] {
///     buf.clear();
///     let params = BadgeParams { label: Some("build"), message: Some(message), ..Default::default() };
///     render_badge_svg_into(&params, &RenderOptions::default(), &mut buf);
///     assert!(buf.contains(message));
/// }
/// ```
pub fn render_badge_svg_into(params: &BadgeParams, options: &RenderOptions, out: &mut String) {
    render::render(params, options, out);
}

/// Fallible variant of [`render_badge_svg`].
///
/// Rendering writes straight into a `String` and cannot fail, so this always
/// returns `Ok`; it remains for callers written against the template-based
/// renderer, which could.
pub fn try_render_badge_svg(params: &BadgeParams) -> Result<String, RenderError> {
    try_render_badge_svg_with(params, &RenderOptions::default())
}

/// [`try_render_badge_svg`] with additional [`RenderOptions`].
pub fn try_render_badge_svg_with(
    params: &BadgeParams,
    options: &RenderOptions,
) -> Result<String, RenderError> {
    Ok(render_badge_svg_with(params, options))
}

/// Error type of [`try_render_badge_svg`]. Rendering can no longer fail, so
/// no value of it is ever produced.
#[derive(Debug)]
pub struct RenderError(String);

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "badge template rendering failed: {}", self.0)
    }
}

impl std::error::Error for RenderError {}

#[cfg(test)]
mod tests {
    use csscolorparser::Color;
    use pretty_assertions::assert_eq;
    use std::str::FromStr;

    use super::*;
    use crate::render::{FONT_FAMILY, is_safe_link};
    use base64::Engine;

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

    #[test]
    fn test_social_label_keeps_its_case() {
        // Only the social style capitalizes, and only the first character; the
        // rest reaching the badge lowercased would also narrow it.
        let svg = render(BadgeStyle::Social, "AT&T", "PASSING", None);
        assert!(svg.contains("AT&amp;T"), "{svg}");
        assert!(svg.contains("PASSING"), "{svg}");
    }
}
