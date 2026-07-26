//! XML escaper for the badge templates.
//!
//! Every `{{ … }}` in `templates/*.svg` lands either in an attribute value or
//! in element content, so nothing may reach the output unescaped. Askama's
//! built-in HTML escaper would do the job, but it emits numeric references
//! (`&#38;`) whereas shields.io emits named ones (`&amp;`); `tests/svg_compare`
//! diffs our output against upstream byte for byte, so we match upstream.
//!
//! Escaping deliberately happens at render time, not on the way in: text width
//! is measured from the original string, and pre-escaping `&` into `&amp;`
//! would make the badge five characters wide where upstream has one.
//!
//! Escaping is the default in the templates and `|safe` the exception, so that
//! forgetting it on a new interpolation costs a few nanoseconds rather than
//! opening a hole. `|safe` is used for two kinds of value only: numbers, whose
//! `Display` cannot produce a special character, and the logo data URI, which
//! this crate base64-encodes itself. The logo carries most of the output bytes,
//! so exempting it is what keeps the scan off the hot path.

use std::fmt::{self, Write};

/// Escapes the five characters XML gives special meaning, using the same named
/// references shields.io emits.
#[derive(Debug, Clone, Copy, Default)]
pub struct Xml;

/// Replacement for each byte, or `""` when the byte passes through.
///
/// The hot input is a multi-kilobyte base64 logo URI with nothing to escape, so
/// the scan must stay a table lookup per byte rather than a chain of compares.
static REPLACEMENTS: [&str; 256] = {
    let mut table = [""; 256];
    table[b'&' as usize] = "&amp;";
    table[b'<' as usize] = "&lt;";
    table[b'>' as usize] = "&gt;";
    table[b'"' as usize] = "&quot;";
    table[b'\'' as usize] = "&apos;";
    table
};

impl askama::filters::Escaper for Xml {
    fn write_escaped_str<W: Write>(&self, mut dest: W, string: &str) -> fmt::Result {
        // All five are ASCII, and no byte of a multi-byte UTF-8 sequence is
        // ASCII, so scanning bytes cannot split a character; every `i` and
        // `i + 1` below is a char boundary.
        let mut last = 0;
        for (i, &byte) in string.as_bytes().iter().enumerate() {
            let replacement = REPLACEMENTS[byte as usize];
            if replacement.is_empty() {
                continue;
            }
            dest.write_str(&string[last..i])?;
            dest.write_str(replacement)?;
            last = i + 1;
        }
        // Badge text almost never contains any of them, so the common case is
        // this single write of the whole string.
        dest.write_str(&string[last..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use askama::filters::Escaper;

    fn esc(s: &str) -> String {
        let mut out = String::new();
        Xml.write_escaped_str(&mut out, s).unwrap();
        out
    }

    #[test]
    fn escapes_the_five_xml_characters() {
        assert_eq!(esc("&<>\"'"), "&amp;&lt;&gt;&quot;&apos;");
    }

    #[test]
    fn leaves_ordinary_text_untouched() {
        for s in ["", "passing", "1.2.3", "a b/c-d_e", "构建通过", "✓ ok"] {
            assert_eq!(esc(s), s);
        }
    }

    #[test]
    fn preserves_text_around_escapes() {
        assert_eq!(esc("AT&T"), "AT&amp;T");
        assert_eq!(esc("&lead"), "&amp;lead");
        assert_eq!(esc("trail&"), "trail&amp;");
        assert_eq!(esc("a&&b"), "a&amp;&amp;b");
    }

    #[test]
    fn splits_on_char_boundaries_around_multibyte_text() {
        assert_eq!(esc("构建&通过"), "构建&amp;通过");
    }
}
