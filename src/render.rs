//! Layout and SVG emission for every badge style.
//!
//! The emitters below write the markup shields.io produces, byte for byte;
//! `tests/svg_compare` diffs them against img.shields.io. They used to be askama
//! templates, but per-interpolation formatting cost more than everything else
//! in a render combined, and the markup has few enough branches to read fine
//! as straight-line code.

use crate::color::SvgColor;
use crate::out::Out;
use crate::small_str::SmallStr;
use crate::{BadgeParams, BadgeStyle, Font, RenderOptions, get_text_width, logo};
use std::borrow::Cow;

const HORIZONTAL_PADDING: u32 = 5;
pub(crate) const FONT_FAMILY: &str = "Verdana,Geneva,DejaVu Sans,sans-serif";
const FONT_SCALE_UP_FACTOR: u32 = 10;

fn round_up_to_odd(n: f64) -> u32 {
    let n_rounded = n.floor() as u32;
    if n_rounded % 2 == 0 {
        n_rounded + 1
    } else {
        n_rounded
    }
}

fn preferred_width_of(text: &str, font: Font) -> u32 {
    round_up_to_odd(get_text_width(text, font))
}

/// Rejects `href` values whose scheme executes script when the badge is opened
/// or embedded as a document.
///
/// XML escaping keeps a link inside its attribute, but `javascript:alert(1)`
/// needs no special character to fire — the scheme itself is the payload. Only
/// the script-bearing schemes are refused; everything else (absolute URLs,
/// relative paths, fragments, `mailto:`, …) passes through untouched.
pub(crate) fn is_safe_link(link: &str) -> bool {
    // Browsers ignore leading whitespace and C0 controls before the scheme, and
    // match it case-insensitively, so strip and fold before comparing.
    // Ordinary links start and end with a printable ASCII character, which
    // the trim would keep, so they skip its char-by-char scan.
    let printable = |b: Option<&u8>| b.is_some_and(|&b| b > b' ' && b.is_ascii());
    let trimmed = if printable(link.as_bytes().first()) && printable(link.as_bytes().last()) {
        link
    } else {
        link.trim_matches(|c: char| c.is_whitespace() || (c as u32) < 0x20)
    };
    let Some(colon) = trimmed.find(':') else {
        // No scheme at all: a relative path or fragment, which cannot execute.
        return true;
    };
    let scheme = &trimmed[..colon];
    // A '/', '?' or '#' before the colon means it was never a scheme
    // ("/a:b" is a path), so the value is relative and safe.
    if scheme.bytes().any(|b| matches!(b, b'/' | b'?' | b'#')) {
        return true;
    }
    const BLOCKED: [&str; 3] = ["javascript", "vbscript", "data"];
    // Common case: a plain ASCII scheme ("https"), compared without decoding.
    // On ASCII, `char::is_whitespace` is exactly `\t`..=`\r` and space.
    let bytes = scheme.as_bytes();
    if bytes
        .iter()
        .all(|&b| b.is_ascii() && !matches!(b, b'\t'..=b'\r' | b' '))
    {
        return !BLOCKED
            .iter()
            .any(|bad| bytes.eq_ignore_ascii_case(bad.as_bytes()));
    }
    // Browsers also skip embedded tabs/newlines inside the scheme ("java\tscript:").
    let folded = || {
        scheme
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| c.to_ascii_lowercase())
    };
    !BLOCKED.iter().any(|bad| folded().eq(bad.chars()))
}

/// Strips characters outside `[A-Za-z0-9_-]`.
///
/// The suffix is not only interpolated into `id="…"` but also into the
/// `url(#…)` references pointing at it, where escaping would break the link
/// rather than protect it. Restricting the character set keeps both sides
/// valid, and lets the emitters write it unescaped.
fn sanitize_id_suffix(raw: &str) -> Cow<'_, str> {
    let allowed = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if raw.chars().all(allowed) {
        Cow::Borrowed(raw)
    } else {
        Cow::Owned(raw.chars().filter(|&c| allowed(c)).collect())
    }
}

/// Inputs every style emits the same way.
struct Common<'a> {
    message: &'a str,
    link: &'a str,
    extra_link: &'a str,
    /// Base64 data URI, or empty.
    logo: &'a str,
    logo_width: u32,
    /// Already restricted to `[A-Za-z0-9_-]`.
    id_suffix: &'a str,
    label_color: &'a str,
    message_color: &'a str,
}

impl Common<'_> {
    fn has_one_link(&self) -> bool {
        !self.link.is_empty() && self.extra_link.is_empty()
    }

    fn has_two_links(&self) -> bool {
        !self.link.is_empty() && !self.extra_link.is_empty()
    }
}

/// Writes the root `<svg>` up to and including the optional outer link, which
/// is identical across styles apart from the dimensions.
#[inline(always)]
fn open_svg(
    o: &mut Out,
    c: &Common,
    label: &str,
    message: &str,
    width: impl FnOnce(&mut Out),
    height: &str,
) {
    o.raw(r#"<svg xmlns="http://www.w3.org/2000/svg" width=""#);
    width(o);
    o.raw(r#"" height=""#).raw(height).raw("\"");
    // With two links each half is its own link, so the root carries no label.
    if !c.has_two_links() {
        o.raw(r#" role="img" aria-label=""#);
        accessible_text(o, label, message);
        o.raw(r#""><title>"#);
        accessible_text(o, label, message);
        o.raw("</title>");
    } else {
        o.raw(">");
    }
    if c.has_one_link() {
        o.raw(r#"<a target="_blank" href=""#)
            .text(c.link)
            .raw(r#"">"#);
    }
}

#[inline(always)]
fn close_svg(o: &mut Out, c: &Common) {
    o.raw("</g>");
    if c.has_one_link() {
        o.raw("</a>");
    }
    o.raw("</svg>");
}

/// `label: message`, or just `message` without a label.
#[inline(always)]
fn accessible_text(o: &mut Out, label: &str, message: &str) {
    if !label.is_empty() {
        o.text(label).raw(": ");
    }
    o.text(message);
}

/// ` fill="#333"` for dark text; light text inherits the group's white.
#[inline(always)]
fn text_fill(o: &mut Out, dark: bool) {
    if dark {
        o.raw(r##" fill="#333""##);
    }
}

#[inline(always)]
fn logo_image(o: &mut Out, c: &Common, x: impl FnOnce(&mut Out), y: &str) {
    if !c.logo.is_empty() {
        o.raw(r#"<image x=""#);
        x(o);
        o.raw(r#"" y=""#)
            .raw(y)
            .raw(r#"" width=""#)
            .uint(c.logo_width.into())
            .raw(r#"" height="14" href=""#)
            .raw(c.logo)
            .raw(r#""/>"#);
    }
}

/// Writes the SVG for `params` onto the end of `out`.
pub(crate) fn render(params: &BadgeParams, options: &RenderOptions, out: &mut String) {
    let id_suffix = sanitize_id_suffix(options.id_suffix);
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
    } = *params;
    let logo_src = logo.map(str::trim).unwrap_or("");
    let logo_uri = (!logo_src.is_empty())
        .then(|| logo::data_uri(logo_src, logo_color, style == BadgeStyle::Social));
    let logo = logo_uri.as_deref().unwrap_or("");
    let has_logo = !logo.is_empty();
    let logo_width = options.logo_width.unwrap_or(14);
    let logo_padding = if label == Some("") { 0 } else { 3 };
    let total_logo_width = if has_logo {
        logo_width + logo_padding
    } else {
        0
    };

    let has_label_color = !label_color.unwrap_or("").is_empty();
    let message_color = message_color
        .and_then(SvgColor::parse)
        .unwrap_or(SvgColor::fixed("#007ec6"));
    let label_color = match (
        label.unwrap_or("").is_empty(),
        label_color.unwrap_or("").is_empty(),
    ) {
        (true, true) if has_logo => "#555",
        (true, true) => &message_color.value,
        (_, _) => label_color.unwrap_or("#555"),
    };
    let label_color = SvgColor::parse(label_color).unwrap_or(SvgColor::fixed("#555"));

    let message = message.unwrap_or("");
    // A rejected link is treated exactly like an absent one, so layout stays consistent.
    let link = link.filter(|l| is_safe_link(l)).unwrap_or("");
    let extra_link = extra_link.filter(|l| is_safe_link(l));
    let extra_link_absent_or_set = extra_link.is_none_or(|l| !l.is_empty());
    let extra_link = extra_link.unwrap_or("");

    // Literal markup peaks at ~1.5KB (plastic and social with every optional
    // element); text appears up to five times (three <text> elements plus the
    // accessible name twice). Reserving once avoids growing — and copying —
    // the buffer mid-render, which a base64 logo would otherwise force.
    out.reserve(
        1800 + logo.len()
            + 5 * (label.map_or(0, str::len) + message.len())
            + 2 * (link.len() + extra_link.len())
            + 8 * id_suffix.len(),
    );
    let o = &mut Out(out);
    let c = Common {
        message,
        link,
        extra_link,
        logo,
        logo_width,
        id_suffix: &id_suffix,
        label_color: &label_color.value,
        message_color: &message_color.value,
    };
    match style {
        BadgeStyle::Flat | BadgeStyle::FlatSquare | BadgeStyle::Plastic => {
            let layout = FlatLayout::compute(
                label,
                message,
                &label_color,
                &message_color,
                has_label_color,
                has_logo,
                total_logo_width,
                extra_link_absent_or_set,
                !extra_link.is_empty(),
            );
            // A literal `kind` per arm lets each inlined copy of `emit_flat`
            // fold its per-style strings into constants, which `push_str`
            // then copies with a few moves instead of a `memcpy` call.
            let label = label.unwrap_or("");
            match style {
                BadgeStyle::Flat => emit_flat(o, FlatKind::Flat, &layout, label, &c),
                BadgeStyle::FlatSquare => emit_flat(o, FlatKind::Square, &layout, label, &c),
                _ => emit_flat(o, FlatKind::Plastic, &layout, label, &c),
            }
        }
        BadgeStyle::Social => emit_social(o, label, total_logo_width, &c),
        BadgeStyle::ForTheBadge => {
            emit_for_the_badge(o, label, has_label_color, &label_color, &message_color, &c)
        }
    }
}

/// Shared horizontal layout for the flat, flat-square and plastic styles.
/// These styles differ only in chrome (gradients, shadows), not in geometry.
///
/// Positions stay `f32` as in the original computation; they are always whole
/// numbers, which `Out::f32` writes as integers.
struct FlatLayout {
    left_width: i32,
    right_width: i32,
    total_width: i32,
    label_x: f32,
    label_width_scaled: i32,
    message_x: f32,
    message_width_scaled: i32,
    rect_offset: i32,
    message_link_x: i32,
    label_dark_text: bool,
    message_dark_text: bool,
}

impl FlatLayout {
    #[allow(clippy::too_many_arguments)]
    fn compute(
        label: Option<&str>,
        message: &str,
        label_color: &SvgColor,
        message_color: &SvgColor,
        has_label_color: bool,
        has_logo: bool,
        total_logo_width: u32,
        extra_link_absent_or_set: bool,
        has_extra_link: bool,
    ) -> Self {
        let has_label_content = label.is_some_and(|l| !l.is_empty());
        let has_label = has_label_content || has_label_color;
        let label_margin = total_logo_width + 1;

        let label_width = match label {
            Some(label) if has_label => preferred_width_of(label, Font::VerdanaNormal11),
            _ => 0,
        };

        let mut left_width = if has_label {
            (label_width + 2 * HORIZONTAL_PADDING + total_logo_width) as i32
        } else {
            0
        };
        if has_label && label == Some("") {
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
        let total_width = left_width + right_width;

        let right_width = right_width + if !has_label_color { offset } else { 0 };

        let message_link_x = if has_logo && !has_label && extra_link_absent_or_set {
            total_logo_width as i32 + HORIZONTAL_PADDING as i32
        } else {
            left_width
        } + if !has_label && has_extra_link {
            offset
        } else {
            0
        };
        let message_x = 10.0
            * (message_margin as f32 + (0.5 * message_width as f32) + HORIZONTAL_PADDING as f32);

        FlatLayout {
            left_width: left_width.max(0),
            right_width,
            total_width,
            label_x,
            label_width_scaled: (label_width * 10) as i32,
            message_x,
            message_width_scaled: (message_width * 10) as i32,
            rect_offset: if has_logo { 19 } else { 0 },
            message_link_x,
            label_dark_text: label_color.dark_text(),
            message_dark_text: message_color.dark_text(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum FlatKind {
    Flat,
    Square,
    Plastic,
}

/// One half of a flat-family badge: the text with its shadow, wrapped in its
/// own link when the badge has two.
struct FlatText<'a> {
    text: &'a str,
    href: &'a str,
    link_rect_width: i32,
    link_rect_x: i32,
    x: f32,
    length: i32,
    dark_text: bool,
}

#[inline(always)]
fn emit_flat(o: &mut Out, kind: FlatKind, l: &FlatLayout, label: &str, c: &Common) {
    let height = if kind == FlatKind::Plastic {
        "18"
    } else {
        "20"
    };
    open_svg(
        o,
        c,
        label,
        c.message,
        |o| _ = o.int(l.total_width.into()),
        height,
    );

    match kind {
        FlatKind::Flat | FlatKind::Plastic => {
            let (gradient, radius) = if kind == FlatKind::Flat {
                (
                    r##"<stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/>"##,
                    "3",
                )
            } else {
                (
                    r##"<stop offset="0" stop-color="#fff" stop-opacity=".7"/><stop offset=".1" stop-color="#aaa" stop-opacity=".1"/><stop offset=".9" stop-color="#000" stop-opacity=".3"/><stop offset="1" stop-color="#000" stop-opacity=".5"/>"##,
                    "4",
                )
            };
            o.raw(r#"<filter id="blur"><feGaussianBlur stdDeviation="16"/></filter><linearGradient id="s"#)
                .id(c.id_suffix)
                .raw(r#"" x2="0" y2="100%">"#)
                .raw(gradient)
                .raw(r#"</linearGradient><clipPath id="r"#)
                .id(c.id_suffix)
                .raw(r#""><rect width=""#)
                .int(l.total_width.into())
                .raw(r#"" height=""#)
                .raw(height)
                .raw(r#"" rx=""#)
                .raw(radius)
                .raw(r#""/></clipPath><g clip-path="url(#r"#)
                .id(c.id_suffix)
                .raw(r#")">"#);
            background_rects(o, l, c, height);
            o.raw(r#"<rect width=""#)
                .int(l.total_width.into())
                .raw(r#"" height=""#)
                .raw(height)
                .raw(r#"" fill="url(#s"#)
                .id(c.id_suffix)
                .raw(r#")"/></g>"#);
        }
        FlatKind::Square => {
            o.raw(r#"<g shape-rendering="crispEdges">"#);
            background_rects(o, l, c, height);
            o.raw("</g>");
        }
    }
    o.raw(r##"<g fill="#fff" text-anchor="middle" font-family=""##)
        .raw(FONT_FAMILY)
        .raw(r#"" text-rendering="geometricPrecision" font-size="110">"#);
    logo_image(
        o,
        c,
        |o| _ = o.raw("5"),
        if kind == FlatKind::Plastic { "2" } else { "3" },
    );

    if !label.is_empty() {
        let half = FlatText {
            text: label,
            href: c.link,
            link_rect_width: l.left_width,
            link_rect_x: l.rect_offset,
            x: l.label_x,
            length: l.label_width_scaled,
            dark_text: l.label_dark_text,
        };
        flat_text(o, kind, &half, c, height);
    }
    if !c.message.is_empty() {
        let half = FlatText {
            text: c.message,
            href: c.extra_link,
            link_rect_width: l.right_width,
            link_rect_x: l.message_link_x,
            x: l.message_x,
            length: l.message_width_scaled,
            dark_text: l.message_dark_text,
        };
        flat_text(o, kind, &half, c, height);
    }
    close_svg(o, c);
}

#[inline(always)]
fn background_rects(o: &mut Out, l: &FlatLayout, c: &Common, height: &str) {
    o.raw(r#"<rect width=""#)
        .int(l.left_width.into())
        .raw(r#"" height=""#)
        .raw(height)
        .raw(r#"" fill=""#)
        .text(c.label_color)
        .raw(r#""/><rect x=""#)
        .int(l.left_width.into())
        .raw(r#"" width=""#)
        .int(l.right_width.into())
        .raw(r#"" height=""#)
        .raw(height)
        .raw(r#"" fill=""#)
        .text(c.message_color)
        .raw(r#""/>"#);
}

#[inline(always)]
fn flat_text(o: &mut Out, kind: FlatKind, t: &FlatText, c: &Common, height: &str) {
    let two_links = c.has_two_links();
    if two_links {
        o.raw(r#"<a target="_blank" href=""#)
            .text(t.href)
            .raw(r#""><rect width=""#)
            .int(t.link_rect_width.into())
            .raw(r#"" x=""#)
            .int(t.link_rect_x.into())
            .raw(r#"" height=""#)
            .raw(height)
            .raw(r#"" fill="rgba(0,0,0,0)"/>"#);
    }
    if kind == FlatKind::Square {
        o.raw(r#"<text x=""#)
            .f32(t.x)
            .raw(r#"" y="140" textLength=""#)
            .int(t.length.into())
            .raw(r#"" transform="scale(.1)""#);
        text_fill(o, t.dark_text);
        o.raw(">").text(t.text).raw("</text>");
    } else {
        // Plastic sits 10 units (1px) higher on its 18px body.
        let (shadow_y, y) = if kind == FlatKind::Flat {
            ("150", "140")
        } else {
            ("140", "130")
        };
        o.raw(r#"<g transform="scale(.1)"><g aria-hidden="true" fill=""#);
        if t.dark_text {
            o.raw(r##"#ccc">"##);
        } else {
            o.raw(r##"#010101">"##);
        }
        for opacity in [
            r#"" fill-opacity=".8" filter="url(#blur)" textLength=""#,
            r#"" fill-opacity=".3" textLength=""#,
        ] {
            o.raw(r#"<text x=""#)
                .f32(t.x)
                .raw(r#"" y=""#)
                .raw(shadow_y)
                .raw(opacity)
                .int(t.length.into())
                .raw(r#"">"#)
                .text(t.text)
                .raw("</text>");
        }
        o.raw(r#"</g><text x=""#)
            .f32(t.x)
            .raw(r#"" y=""#)
            .raw(y)
            .raw(r#"" textLength=""#)
            .int(t.length.into())
            .raw("\"");
        text_fill(o, t.dark_text);
        o.raw(">").text(t.text).raw("</text></g>");
    }
    if two_links {
        o.raw("</a>");
    }
}

fn emit_social(o: &mut Out, label: Option<&str>, total_logo_width: u32, c: &Common) {
    let message = c.message;
    let offset = if label.is_none() && !c.logo.is_empty() {
        -3i32
    } else {
        0i32
    };

    let label: SmallStr = SmallStr::capitalize_first(label.unwrap_or(""));
    let internal_height = 19;
    let label_horizontal_padding = 5;
    let message_horizontal_padding = 4;
    let horizontal_gutter = 6;

    let label_text_width = preferred_width_of(&label, Font::HelveticaBold11);
    let label_rect_width =
        (label_text_width + total_logo_width + 2 * label_horizontal_padding) as i32 + offset;

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
        * (label_rect_width as f32 + horizontal_gutter as f32 + message_rect_width as f32 / 2.0);
    let message_text_length = FONT_SCALE_UP_FACTOR * message_text_width;
    let label_text_length = FONT_SCALE_UP_FACTOR * label_text_width;

    let left_width = label_rect_width + 1;
    let right_width = if has_message {
        horizontal_gutter + message_rect_width as i32
    } else {
        0
    };
    let total_width = left_width + right_width;

    let s = c.id_suffix;
    let two_links = c.has_two_links();
    let label_rect = |o: &mut Out| {
        o.raw(r#"<rect id="llink"#)
            .id(s)
            .raw(r##"" stroke="#d5d5d5" fill="url(#a"##)
            .id(s)
            .raw(r#")" x=".5" y=".5" width=""#)
            .int(label_rect_width.into())
            .raw(r#"" height=""#)
            .uint(internal_height)
            .raw(r#"" rx="2"/>"#);
    };

    open_svg(
        o,
        c,
        &label,
        message,
        |o| _ = o.int(total_width.into()),
        "20",
    );
    o.raw("<style>a:hover #llink")
        .id(s)
        .raw("{fill:url(#b")
        .id(s)
        .raw(");stroke:#ccc}a:hover #rlink")
        .id(s)
        .raw(r#"{fill:#4183c4}</style><linearGradient id="a"#)
        .id(s)
        .raw(r##"" x2="0" y2="100%"><stop offset="0" stop-color="#fcfcfc" stop-opacity="0"/><stop offset="1" stop-opacity=".1"/></linearGradient><linearGradient id="b"##)
        .id(s)
        .raw(r##"" x2="0" y2="100%"><stop offset="0" stop-color="#ccc" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient><g stroke="#d5d5d5"><rect stroke="none" fill="#fcfcfc" x=".5" y=".5" width=""##)
        .int(label_rect_width.into())
        .raw(r#"" height=""#)
        .uint(internal_height)
        .raw(r#"" rx="2"/>"#);
    if has_message {
        o.raw(r#"<rect x=""#)
            .f32(message_bubble_main_x)
            .raw(r#"" y=".5" width=""#)
            .uint(message_rect_width.into())
            .raw(r#"" height=""#)
            .uint(internal_height)
            .raw(r##"" rx="2" fill="#fafafa"/><rect x=""##)
            .int(message_bubble_notch_x.into())
            .raw(r##"" y="7.5" width=".5" height="5" stroke="#fafafa"/><path d="M"##)
            .f32(message_bubble_main_x)
            .raw(r##" 6.5 l-3 3v1 l3 3" fill="#fafafa"/>"##);
    }
    o.raw("</g>");
    logo_image(o, c, |o| _ = o.raw("5"), "3");
    if two_links {
        o.raw(r#"<g aria-hidden="false""#);
    } else {
        o.raw(r#"<g aria-hidden="true""#);
    }
    o.raw(r##" fill="#333" text-anchor="middle" font-family="Helvetica Neue,Helvetica,Arial,sans-serif" text-rendering="geometricPrecision" font-weight="700" font-size="110px" line-height="14px">"##);
    if two_links {
        o.raw(r#"<a target="_blank" href=""#)
            .text(c.link)
            .raw(r#"">"#);
    } else {
        label_rect(o);
    }
    social_text(o, None, label_text_x, label_text_length, &label);
    if two_links {
        label_rect(o);
        o.raw("</a>");
    }
    if has_message {
        if two_links {
            o.raw(r#"<a target="_blank" href=""#)
                .text(c.extra_link)
                .raw(r#""><rect width=""#)
                .uint((message_rect_width + 1).into())
                .raw(r#"" x=""#)
                .f32(message_bubble_main_x - 0.5)
                .raw(r#"" height="20" fill="rgba(0,0,0,0)"/>"#);
        }
        social_text(o, Some(s), message_text_x, message_text_length, message);
        if two_links {
            o.raw("</a>");
        }
    }
    close_svg(o, c);
}

/// A social half: white embossing behind the dark text. The message text
/// carries `id="rlink…"` for its hover style; the label text has no id.
fn social_text(o: &mut Out, rlink_id: Option<&str>, x: f32, length: u32, text: &str) {
    o.raw(r##"<text aria-hidden="true" x=""##)
        .f32(x)
        .raw(r##"" y="150" fill="#fff" transform="scale(.1)" textLength=""##)
        .uint(length.into())
        .raw(r#"">"#)
        .text(text)
        .raw("</text><text ");
    if let Some(suffix) = rlink_id {
        o.raw(r#"id="rlink"#).id(suffix).raw(r#"" "#);
    }
    o.raw(r#"x=""#)
        .f32(x)
        .raw(r#"" y="140" transform="scale(.1)" textLength=""#)
        .uint(length.into())
        .raw(r#"">"#)
        .text(text)
        .raw("</text>");
}

fn emit_for_the_badge(
    o: &mut Out,
    label: Option<&str>,
    has_label_color: bool,
    label_color: &SvgColor,
    message_color: &SvgColor,
    c: &Common,
) {
    // for-the-badge is styled in all caps; convert before measuring widths
    let label: SmallStr = SmallStr::uppercase(label.unwrap_or(""));
    let message: SmallStr = SmallStr::uppercase(c.message);
    let font_size = 10;
    let letter_spacing = 1.25f64;
    let logo_text_gutter = 6.0f64;
    let logo_margin = 9.0f64;
    let logo_width = c.logo_width as f64;
    let has_logo = !c.logo.is_empty();
    // Upstream truncates the font measurement (`anafanafo(...) | 0`) and adds
    // letter spacing per UTF-16 code unit, keeping fractional widths throughout.
    let spaced_width = |text: &str, font| {
        if text.is_empty() {
            0.0
        } else {
            get_text_width(text, font).trunc() + letter_spacing * utf16_len(text) as f64
        }
    };
    let label_text_width = spaced_width(&label, Font::VerdanaNormal10);
    let message_text_width = spaced_width(&message, Font::VerdanaBold10);
    let has_label = !label.is_empty();
    let no_text = !has_label && message.is_empty();
    // Upstream checks the caller-supplied labelColor, not the resolved
    // default that the shared preprocessing may have filled in.
    let need_label_rect = has_label || (has_logo && has_label_color);
    let gutter = if no_text {
        logo_text_gutter - logo_margin
    } else {
        logo_text_gutter
    };
    let text_margin = 12.0f64;

    let (logo_min_x, label_text_min_x) = if has_logo {
        (logo_margin, logo_margin + logo_width + gutter)
    } else {
        (0.0, text_margin)
    };

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
    } else if has_logo {
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
    let scale = FONT_SCALE_UP_FACTOR as f64;

    let label_dark_text = label_color.dark_text();
    let message_dark_text = message_color.dark_text();
    let two_links = c.has_two_links();

    open_svg(o, c, &label, &message, |o| _ = o.f64(total_width), "28");
    o.raw(r#"<g shape-rendering="crispEdges">"#);
    if need_label_rect {
        o.raw(r#"<rect width=""#)
            .f64(label_rect_width)
            .raw(r#"" height="28" fill=""#)
            .text(c.label_color)
            .raw(r#""/><rect x=""#)
            .f64(label_rect_width)
            .raw(r#"" width=""#);
    } else {
        o.raw(r#"<rect width=""#);
    }
    o.f64(message_rect_width)
        .raw(r#"" height="28" fill=""#)
        .text(c.message_color)
        .raw(r##""/></g><g fill="#fff" text-anchor="middle" font-family=""##)
        .raw(FONT_FAMILY)
        .raw(r#"" text-rendering="geometricPrecision" font-size=""#)
        .int((font_size * FONT_SCALE_UP_FACTOR as i32).into())
        .raw(r#"">"#);
    logo_image(o, c, |o| _ = o.f64(logo_min_x), "7");
    if has_label {
        if two_links {
            o.raw(r#"<a target="_blank" href=""#)
                .text(c.link)
                .raw(r#""><rect width=""#)
                .f64(label_rect_width)
                .raw(r#"" height="28" fill="rgba(0,0,0,0)"/>"#);
        }
        o.raw(r#"<text transform="scale(.1)" x=""#)
            .f64(label_mid_x * scale)
            .raw(r#"" y="175" textLength=""#)
            .f64(label_text_width * scale)
            .raw("\"");
        text_fill(o, label_dark_text);
        o.raw(">").text(&label).raw("</text>");
        if two_links {
            o.raw("</a>");
        }
    }
    // Upstream always renders the message text element, even when empty.
    if two_links {
        o.raw(r#"<a target="_blank" href=""#)
            .text(c.extra_link)
            .raw(r#""><rect width=""#)
            .f64(message_rect_width)
            .raw(r#"" height="28" x=""#)
            .f64(label_rect_width)
            .raw(r#"" fill="rgba(0,0,0,0)"/>"#);
    }
    o.raw(r#"<text transform="scale(.1)" x=""#)
        .f64(message_mid_x * scale)
        .raw(r#"" y="175" textLength=""#)
        .f64(message_text_width * scale)
        .raw(r#"" font-weight="bold""#);
    text_fill(o, message_dark_text);
    o.raw(">").text(&message).raw("</text>");
    if two_links {
        o.raw("</a>");
    }
    close_svg(o, c);
}

/// `s.encode_utf16().count()` without encoding: one unit per character, two
/// for those outside the BMP (4-byte UTF-8 sequences).
fn utf16_len(s: &str) -> usize {
    s.chars().count() + s.bytes().filter(|&b| b >= 0xF0).count()
}
