//! Logo resolution: a Simple Icons slug or raw `<svg>` in, the base64 data URI
//! the badges embed out.

use crate::color::SvgColor;
use base64::Engine;
use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

/// Resolves `logo` (a Simple Icons slug or a raw `<svg>` string) to the `href` value
/// the templates embed: a base64 data URI, or an empty string when nothing resolves.
fn build_data_uri(logo: &str, logo_color: &str) -> String {
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
/// a few KB. A small per-thread cache keeps it off the hot path; being thread-local, it
/// costs no lock and does not serialize concurrent rendering.
const CAPACITY: usize = 16;

/// What a data URI is a function of. The color is kept as the caller passed it, before
/// normalization, so a hit skips the color parse too; `social` picks the default
/// color an absent or invalid one falls back to.
struct Key {
    logo: String,
    color: Option<String>,
    social: bool,
}

impl Key {
    fn is(&self, logo: &str, color: Option<&str>, social: bool) -> bool {
        self.social == social && self.logo == logo && self.color.as_deref() == color
    }
}

thread_local! {
    /// Least-recently-used first, so eviction pops the front.
    static CACHE: RefCell<Vec<(Key, Rc<str>)>> = const { RefCell::new(Vec::new()) };
}

/// The data URI for `logo` in `color` (as given by the caller), from the cache when
/// possible.
pub(crate) fn data_uri(logo: &str, color: Option<&str>, social: bool) -> Rc<str> {
    CACHE.with_borrow_mut(|entries| {
        // Servers tend to render the same logo repeatedly, so check the most
        // recent entry first; a hit there needs no reordering.
        if let Some((_, uri)) = entries.last().filter(|(k, _)| k.is(logo, color, social)) {
            return Rc::clone(uri);
        }
        if let Some(i) = entries.iter().position(|(k, _)| k.is(logo, color, social)) {
            let entry = entries.remove(i);
            let uri = Rc::clone(&entry.1);
            entries.push(entry);
            return uri;
        }
        // Both defaults are already normalized, so they skip the parse.
        let default = if social { "#000000" } else { "whitesmoke" };
        let normalized = color.and_then(SvgColor::parse);
        let normalized = normalized.as_ref().map_or(default, |c| &*c.value);
        let uri: Rc<str> = Rc::from(build_data_uri(logo, normalized));
        if entries.len() == CAPACITY {
            entries.remove(0);
        }
        let key = Key {
            logo: logo.to_owned(),
            color: color.map(str::to_owned),
            social,
        };
        entries.push((key, Rc::clone(&uri)));
        uri
    })
}
