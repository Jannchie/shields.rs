//! WebAssembly bindings for the `shields` badge rendering engine.
//!
//! The core crate is reused unchanged; this layer only marshals a JS object
//! into [`shields::BadgeParams`] and returns the rendered SVG string.
//!
//! Options are not deserialized with serde, which took several times longer
//! than rendering the badge. One JS call checks every field's type and reports
//! which are present; only those are then read, each as a single string handed
//! across the boundary.

use shields::{BadgeParams, BadgeStyle, RenderOptions, render_badge_svg_with};
use wasm_bindgen::prelude::*;

/// Option fields in the order of the bits [`present_fields`] reports.
const FIELDS: [&str; 11] = [
    "style",
    "label",
    "message",
    "labelColor",
    "messageColor",
    "link",
    "extraLink",
    "logo",
    "logoColor",
    "idSuffix",
    "logoWidth",
];

#[wasm_bindgen(inline_js = r#"
const STRINGS = ['style', 'label', 'message', 'labelColor', 'messageColor',
                 'link', 'extraLink', 'logo', 'logoColor', 'idSuffix'];
// Bit i is set when field i is present; -1 - i when field i has the wrong
// type. null counts as absent, as it did with serde.
export function present_fields(o) {
  let mask = 0;
  for (let i = 0; i < STRINGS.length; i++) {
    const v = o[STRINGS[i]];
    if (v === undefined || v === null) continue;
    if (typeof v !== 'string') return -1 - i;
    mask |= 1 << i;
  }
  const w = o.logoWidth;
  if (w !== undefined && w !== null) {
    if (!Number.isInteger(w) || w < 0 || w > 0xffffffff) return -1 - STRINGS.length;
    mask |= 1 << STRINGS.length;
  }
  return mask;
}
"#)]
extern "C" {
    fn present_fields(options: &BadgeOptions) -> i32;
}

#[wasm_bindgen]
extern "C" {
    /// The options object, a plain JS object with every field optional. Unknown
    /// keys are ignored.
    #[wasm_bindgen(typescript_type = "BadgeOptions")]
    pub type BadgeOptions;

    // Only called for fields `present_fields` found to hold a string (or, for
    // `logoWidth`, an integer), so they cannot throw and need no `catch` —
    // whose try/catch wrapper cost more than the read itself.
    #[wasm_bindgen(method, getter)]
    fn style(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter)]
    fn label(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter)]
    fn message(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter, js_name = labelColor)]
    fn label_color(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter, js_name = messageColor)]
    fn message_color(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter)]
    fn link(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter, js_name = extraLink)]
    fn extra_link(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter)]
    fn logo(this: &BadgeOptions) -> String;
    #[wasm_bindgen(method, getter, js_name = logoColor)]
    fn logo_color(this: &BadgeOptions) -> String;
    /// Suffix appended to every SVG element id to avoid collisions when
    /// multiple badges are inlined on one page.
    #[wasm_bindgen(method, getter, js_name = idSuffix)]
    fn id_suffix(this: &BadgeOptions) -> String;
    /// Rendered logo width in pixels (default 14).
    #[wasm_bindgen(method, getter, js_name = logoWidth)]
    fn logo_width(this: &BadgeOptions) -> f64;
}

#[wasm_bindgen(typescript_custom_section)]
const BADGE_OPTIONS: &str = r#"
export interface BadgeOptions {
  style?: "flat" | "flat-square" | "plastic" | "social" | "for-the-badge";
  label?: string;
  message?: string;
  labelColor?: string;
  messageColor?: string;
  link?: string;
  extraLink?: string;
  logo?: string;
  logoColor?: string;
  idSuffix?: string;
  logoWidth?: number;
}
"#;

fn parse_style(style: Option<&str>) -> Result<BadgeStyle, JsError> {
    // The kebab-case spellings `BadgeStyle` deserializes from.
    Ok(match style {
        None => BadgeStyle::default(),
        Some("flat") => BadgeStyle::Flat,
        Some("flat-square") => BadgeStyle::FlatSquare,
        Some("plastic") => BadgeStyle::Plastic,
        Some("social") => BadgeStyle::Social,
        Some("for-the-badge") => BadgeStyle::ForTheBadge,
        Some(other) => {
            return Err(JsError::new(&format!(
                "invalid badge options: unknown style `{other}`, expected one of \
                 `flat`, `flat-square`, `plastic`, `social`, `for-the-badge`"
            )));
        }
    })
}

/// Render a badge to an SVG string.
///
/// `options` is a plain JS object, e.g.
/// `{ style: "flat", label: "build", message: "passing", messageColor: "brightgreen" }`.
#[wasm_bindgen(js_name = renderBadge)]
pub fn render_badge(options: &BadgeOptions) -> Result<String, JsError> {
    if !options.is_object() {
        return Err(JsError::new("invalid badge options: expected an object"));
    }
    let present = present_fields(options);
    if present < 0 {
        let field = FIELDS[(-1 - present) as usize];
        let expected = if field == "logoWidth" {
            "a non-negative integer"
        } else {
            "a string"
        };
        return Err(JsError::new(&format!(
            "invalid badge options: `{field}` must be {expected}"
        )));
    }
    // `i` is the field's index in `FIELDS`.
    let has = |i: usize| present & (1 << i) != 0;
    let read = |i: usize, get: fn(&BadgeOptions) -> String| has(i).then(|| get(options));

    let style = read(0, BadgeOptions::style);
    let label = read(1, BadgeOptions::label);
    let message = read(2, BadgeOptions::message);
    let label_color = read(3, BadgeOptions::label_color);
    let message_color = read(4, BadgeOptions::message_color);
    let link = read(5, BadgeOptions::link);
    let extra_link = read(6, BadgeOptions::extra_link);
    let logo = read(7, BadgeOptions::logo);
    let logo_color = read(8, BadgeOptions::logo_color);
    let id_suffix = read(9, BadgeOptions::id_suffix);
    let logo_width = has(10).then(|| options.logo_width() as u32);

    let params = BadgeParams {
        style: parse_style(style.as_deref())?,
        label: label.as_deref(),
        message: message.as_deref(),
        label_color: label_color.as_deref(),
        message_color: message_color.as_deref(),
        link: link.as_deref(),
        extra_link: extra_link.as_deref(),
        logo: logo.as_deref(),
        logo_color: logo_color.as_deref(),
    };

    let mut render_opts = RenderOptions::default();
    if let Some(suffix) = id_suffix.as_deref() {
        render_opts = render_opts.id_suffix(suffix);
    }
    if let Some(width) = logo_width {
        render_opts = render_opts.logo_width(width);
    }
    Ok(render_badge_svg_with(&params, &render_opts))
}
