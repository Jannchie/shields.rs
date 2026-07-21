//! WebAssembly bindings for the `shields` badge rendering engine.
//!
//! The core crate is reused unchanged; this layer only marshals a JS object
//! into [`shields::BadgeParams`] and returns the rendered SVG string.

use serde::Deserialize;
use shields::{BadgeParams, BadgeStyle, RenderOptions, try_render_badge_svg_with};
use wasm_bindgen::prelude::*;

/// Options accepted by [`render_badge`], mirroring `BadgeParams` with a
/// JS-idiomatic camelCase shape. Every field is optional. (Unknown keys are
/// silently ignored — serde-wasm-bindgen does not honor `deny_unknown_fields`.)
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct BadgeOptions {
    /// Badge style; accepts the kebab-case spellings (`flat`, `flat-square`,
    /// `plastic`, `social`, `for-the-badge`) owned by [`BadgeStyle`].
    #[serde(default)]
    style: BadgeStyle,
    label: Option<String>,
    message: Option<String>,
    label_color: Option<String>,
    message_color: Option<String>,
    link: Option<String>,
    extra_link: Option<String>,
    logo: Option<String>,
    logo_color: Option<String>,
    /// Suffix appended to every SVG element id to avoid collisions when
    /// multiple badges are inlined on one page.
    id_suffix: Option<String>,
    /// Rendered logo width in pixels (default 14).
    logo_width: Option<u32>,
}

/// Render a badge to an SVG string.
///
/// `options` is a plain JS object, e.g.
/// `{ style: "flat", label: "build", message: "passing", messageColor: "brightgreen" }`.
#[wasm_bindgen(js_name = renderBadge)]
pub fn render_badge(options: JsValue) -> Result<String, JsError> {
    let opts: BadgeOptions = serde_wasm_bindgen::from_value(options)
        .map_err(|e| JsError::new(&format!("invalid badge options: {e}")))?;

    let params = BadgeParams {
        style: opts.style,
        label: opts.label.as_deref(),
        message: opts.message.as_deref(),
        label_color: opts.label_color.as_deref(),
        message_color: opts.message_color.as_deref(),
        link: opts.link.as_deref(),
        extra_link: opts.extra_link.as_deref(),
        logo: opts.logo.as_deref(),
        logo_color: opts.logo_color.as_deref(),
    };

    let mut render_opts = RenderOptions::default();
    if let Some(suffix) = opts.id_suffix.as_deref() {
        render_opts = render_opts.id_suffix(suffix);
    }
    if let Some(width) = opts.logo_width {
        render_opts = render_opts.logo_width(width);
    }

    try_render_badge_svg_with(&params, &render_opts)
        .map_err(|e| JsError::new(&format!("badge render failed: {e}")))
}
