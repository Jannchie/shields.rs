//! The writer every badge style emits through.
//!
//! Rendering used to go through askama, which formats every interpolation via
//! `core::fmt`. A badge has 30–50 of them, mostly small integers, and the
//! formatting machinery — not the layout math — dominated the render time.
//! Writing literals with `push_str` and numbers with the routines below keeps
//! each interpolation to a few instructions.

use std::fmt::Write as _;

/// Appends SVG fragments to a borrowed `String`.
pub(crate) struct Out<'a>(pub &'a mut String);

/// Two ASCII digits for every value in `0..100`.
static DIGIT_PAIRS: &[u8; 200] = b"\
    0001020304050607080910111213141516171819\
    2021222324252627282930313233343536373839\
    4041424344454647484950515253545556575859\
    6061626364656667686970717273747576777879\
    8081828384858687888990919293949596979899";

/// Fractional digits of `k / 8` for `k` in `0..8`.
static EIGHTHS: [&str; 8] = ["", ".125", ".25", ".375", ".5", ".625", ".75", ".875"];

/// Replacement for each byte, or `""` when the byte passes through.
///
/// Uses the named references shields.io emits (`&amp;`) rather than numeric
/// ones (`&#38;`): `tests/svg_compare` diffs the output against upstream byte
/// for byte.
static ESCAPES: [&str; 256] = {
    let mut table = [""; 256];
    table[b'&' as usize] = "&amp;";
    table[b'<' as usize] = "&lt;";
    table[b'>' as usize] = "&gt;";
    table[b'"' as usize] = "&quot;";
    table[b'\'' as usize] = "&apos;";
    table
};

impl Out<'_> {
    /// Appends `s` verbatim. Only for literals and values that cannot contain
    /// an XML special character (sanitized ids, base64 data URIs, fixed colors).
    #[inline(always)]
    pub fn raw(&mut self, s: &str) -> &mut Self {
        self.0.push_str(s);
        self
    }

    /// Appends an id suffix, which is usually empty. Already restricted to
    /// `[A-Za-z0-9_-]`, so it needs no escaping.
    #[inline(always)]
    pub fn id(&mut self, suffix: &str) -> &mut Self {
        if !suffix.is_empty() {
            self.0.push_str(suffix);
        }
        self
    }

    /// Appends `s` with the five XML special characters escaped.
    ///
    /// Every user-supplied string lands in an attribute value or in element
    /// content, so all of it goes through here. Escaping happens at output time
    /// rather than on the way in because text width is measured from the
    /// original string: pre-escaping `&` would measure five characters where
    /// upstream measures one.
    #[inline]
    pub fn text(&mut self, s: &str) -> &mut Self {
        // All five are ASCII, and no byte of a multi-byte UTF-8 sequence is
        // ASCII, so scanning bytes cannot split a character; every `i` and
        // `i + 1` below is a char boundary.
        let mut last = 0;
        for (i, &byte) in s.as_bytes().iter().enumerate() {
            let replacement = ESCAPES[byte as usize];
            if replacement.is_empty() {
                continue;
            }
            self.0.push_str(&s[last..i]);
            self.0.push_str(replacement);
            last = i + 1;
        }
        // Badge text almost never contains any of them, so the common case is
        // this single copy of the whole string.
        self.0.push_str(&s[last..]);
        self
    }

    #[inline]
    pub fn uint(&mut self, n: u64) -> &mut Self {
        // Badge numbers are almost all below 10 000. Pushing their digits one
        // by one stays inline, where a variable-length `push_str` would call
        // `memcpy` for two or three bytes.
        if n < 10_000 {
            let digit = |d: u64| char::from(b'0' + (d % 10) as u8);
            if n >= 1000 {
                self.0.push(digit(n / 1000));
            }
            if n >= 100 {
                self.0.push(digit(n / 100));
            }
            if n >= 10 {
                self.0.push(digit(n / 10));
            }
            self.0.push(digit(n));
            return self;
        }
        let mut n = n;
        let mut buf = [0u8; 20];
        let mut i = buf.len();
        while n >= 100 {
            let pair = (n % 100) as usize * 2;
            n /= 100;
            i -= 2;
            buf[i..i + 2].copy_from_slice(&DIGIT_PAIRS[pair..pair + 2]);
        }
        if n >= 10 {
            let pair = n as usize * 2;
            i -= 2;
            buf[i..i + 2].copy_from_slice(&DIGIT_PAIRS[pair..pair + 2]);
        } else {
            i -= 1;
            buf[i] = b'0' + n as u8;
        }
        // Only ASCII digits were written, so this never fails.
        self.0
            .push_str(std::str::from_utf8(&buf[i..]).unwrap_or_default());
        self
    }

    #[inline]
    pub fn int(&mut self, n: i64) -> &mut Self {
        if n < 0 {
            self.0.push('-');
        }
        self.uint(n.unsigned_abs())
    }

    /// Writes `v` exactly as `Display` would.
    ///
    /// Badge coordinates are whole numbers or multiples of 1/8 (the layout only
    /// ever halves odd widths and adds 1.25px letter spacing), so they take the
    /// integer path. The bounds keep the float's precision well below 1/8,
    /// where the shortest round-tripping decimal — what `Display` prints — is
    /// the exact value; anything else falls back to `Display` itself.
    #[inline]
    pub fn f32(&mut self, v: f32) -> &mut Self {
        // Integrality is tested by a round trip through `i64` rather than
        // `fract()`: without SSE4.1 (not in the x86-64 baseline) `fract` is a
        // libm call. Inside the bounds the cast is exact.
        if v.abs() < 16_777_216.0 && !is_negative_zero(v.into()) {
            let n = v as i64;
            if n as f32 == v {
                return self.int(n);
            }
            let k = (v * 8.0) as i64;
            if k as f32 == v * 8.0 && v.abs() < 32_768.0 {
                return self.eighths(k);
            }
        }
        self.display(v)
    }

    /// `f64` counterpart of [`f32`](Self::f32).
    #[inline]
    pub fn f64(&mut self, v: f64) -> &mut Self {
        if v.abs() < 9_007_199_254_740_992.0 && !is_negative_zero(v) {
            let n = v as i64;
            if n as f64 == v {
                return self.int(n);
            }
            let k = (v * 8.0) as i64;
            if k as f64 == v * 8.0 && v.abs() < 1_099_511_627_776.0 {
                return self.eighths(k);
            }
        }
        self.display(v)
    }

    /// Kept out of line so the fast paths above stay small enough to inline.
    #[cold]
    #[inline(never)]
    fn display(&mut self, v: impl std::fmt::Display) -> &mut Self {
        let _ = write!(self.0, "{v}");
        self
    }

    /// Writes `k / 8` for a `k` that is not a multiple of 8.
    fn eighths(&mut self, k: i64) -> &mut Self {
        if k < 0 {
            self.0.push('-');
        }
        let k = k.unsigned_abs();
        self.uint(k / 8);
        self.0.push_str(EIGHTHS[(k % 8) as usize]);
        self
    }
}

/// `Display` prints `-0`, which the integer path would lose.
fn is_negative_zero(v: f64) -> bool {
    v == 0.0 && v.is_sign_negative()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with(f: impl FnOnce(&mut Out)) -> String {
        let mut s = String::new();
        f(&mut Out(&mut s));
        s
    }

    #[test]
    fn escapes_the_five_xml_characters() {
        assert_eq!(with(|o| _ = o.text("&<>\"'")), "&amp;&lt;&gt;&quot;&apos;");
    }

    #[test]
    fn leaves_ordinary_text_untouched() {
        for s in ["", "passing", "1.2.3", "a b/c-d_e", "构建通过", "✓ ok"] {
            assert_eq!(with(|o| _ = o.text(s)), s);
        }
    }

    #[test]
    fn preserves_text_around_escapes() {
        assert_eq!(with(|o| _ = o.text("AT&T")), "AT&amp;T");
        assert_eq!(with(|o| _ = o.text("&lead")), "&amp;lead");
        assert_eq!(with(|o| _ = o.text("trail&")), "trail&amp;");
        assert_eq!(with(|o| _ = o.text("a&&b")), "a&amp;&amp;b");
        assert_eq!(with(|o| _ = o.text("构建&通过")), "构建&amp;通过");
    }

    #[test]
    fn integers_match_display() {
        let mut values: Vec<i64> = (-1000..=1000).collect();
        let mut p = 1i64;
        while let Some(next) = p.checked_mul(10) {
            values.extend([p - 1, p, p + 1, -p, next - 1]);
            p = next;
        }
        values.extend([i64::MIN, i64::MAX, i64::MIN + 1]);
        for n in values {
            assert_eq!(with(|o| _ = o.int(n)), n.to_string());
        }
        assert_eq!(with(|o| _ = o.uint(u64::MAX)), u64::MAX.to_string());
    }

    #[test]
    fn floats_match_display() {
        let mut f64s = vec![
            0.0,
            -0.0,
            0.1,
            1.0 / 3.0,
            1e-7,
            1e21,
            1e300,
            f64::MAX,
            f64::MIN_POSITIVE,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            9_007_199_254_740_993.0,
            1_099_511_627_776.125,
        ];
        for k in -40_000i64..40_000 {
            f64s.push(k as f64 / 8.0);
            f64s.push(k as f64 * 1.25);
            f64s.push(k as f64 / 16.0);
            f64s.push(k as f64 * 997.0 / 8.0);
        }
        for p in 0..64 {
            let base = 2f64.powi(p);
            f64s.extend([base, base + 0.125, base - 0.125, base + 0.5, -base - 0.375]);
        }
        for v in f64s {
            assert_eq!(with(|o| _ = o.f64(v)), v.to_string(), "{v:?}");
            let v32 = v as f32;
            assert_eq!(with(|o| _ = o.f32(v32)), v32.to_string(), "{v32:?}");
        }
    }
}
