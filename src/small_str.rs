//! A string that is borrowed when it can be, and kept on the stack when short.
//!
//! Recasing colors (`#FFF` → `#fff`) and badge text (`build` → `BUILD`) used to
//! allocate a `String` per call. Inputs are almost always short ASCII, so an
//! inline buffer covers them without touching the allocator.

use std::ops::Deref;

/// `N` is the inline capacity: 64 bytes covers badge text, colors use less so
/// the values they travel in stay small.
pub(crate) enum SmallStr<'a, const N: usize = 64> {
    Borrowed(&'a str),
    Static(&'static str),
    /// Always holds valid UTF-8 in `buf[..len]`.
    Inline {
        buf: [u8; N],
        len: u8,
    },
    Owned(String),
}

impl<'a, const N: usize> SmallStr<'a, N> {
    /// Copies `s` with its ASCII letters recased; other bytes are untouched,
    /// so the result stays valid UTF-8.
    fn recase_ascii(s: &str, upper: bool) -> Self {
        if s.len() > N {
            let mut owned = s.to_owned();
            if upper {
                owned.make_ascii_uppercase();
            } else {
                owned.make_ascii_lowercase();
            }
            return SmallStr::Owned(owned);
        }
        let mut buf = [0u8; N];
        let dst = &mut buf[..s.len()];
        dst.copy_from_slice(s.as_bytes());
        if upper {
            dst.make_ascii_uppercase();
        } else {
            dst.make_ascii_lowercase();
        }
        SmallStr::Inline {
            buf,
            len: s.len() as u8,
        }
    }

    /// `s.to_ascii_lowercase()`, borrowing when nothing changes.
    pub fn ascii_lowercase(s: &'a str) -> Self {
        if s.bytes().any(|b| b.is_ascii_uppercase()) {
            Self::recase_ascii(s, false)
        } else {
            SmallStr::Borrowed(s)
        }
    }

    /// `s.to_uppercase()`, borrowing when nothing changes.
    pub fn uppercase(s: &'a str) -> Self {
        if !s.is_ascii() {
            // Full Unicode case mapping can change the length (`ß` → `SS`).
            return SmallStr::Owned(s.to_uppercase());
        }
        if s.bytes().any(|b| b.is_ascii_lowercase()) {
            Self::recase_ascii(s, true)
        } else {
            SmallStr::Borrowed(s)
        }
    }

    /// `s` with its first character uppercased and the rest untouched.
    pub fn capitalize_first(s: &'a str) -> Self {
        let Some(first) = s.chars().next() else {
            return SmallStr::Borrowed(s);
        };
        if first.is_ascii() {
            if !first.is_ascii_lowercase() {
                return SmallStr::Borrowed(s);
            }
            if s.len() <= N {
                let mut buf = [0u8; N];
                buf[..s.len()].copy_from_slice(s.as_bytes());
                buf[0] = buf[0].to_ascii_uppercase();
                return SmallStr::Inline {
                    buf,
                    len: s.len() as u8,
                };
            }
        }
        let mut out: String = first.to_uppercase().collect();
        out.push_str(&s[first.len_utf8()..]);
        SmallStr::Owned(out)
    }

    /// `"#"` followed by `s`, for a short ASCII `s`.
    pub fn hash_prefixed(s: &str) -> Self {
        debug_assert!(s.is_ascii() && s.len() < N);
        let mut buf = [0u8; N];
        buf[0] = b'#';
        buf[1..=s.len()].copy_from_slice(s.as_bytes());
        SmallStr::Inline {
            buf,
            len: s.len() as u8 + 1,
        }
    }
}

impl<const N: usize> Deref for SmallStr<'_, N> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        match self {
            SmallStr::Borrowed(s) => s,
            SmallStr::Static(s) => s,
            // Constructors only ever store valid UTF-8 here.
            SmallStr::Inline { buf, len } => {
                std::str::from_utf8(&buf[..*len as usize]).unwrap_or_default()
            }
            SmallStr::Owned(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recasing_matches_std() {
        let long = "x".repeat(70);
        for s in [
            "",
            "abc",
            "ABC",
            "aBc",
            "#FFF",
            "école",
            "Straße",
            "ǅ",
            "ﬁ",
            "1st",
            long.as_str(),
            "测试abc",
        ] {
            assert_eq!(
                &*SmallStr::<64>::ascii_lowercase(s),
                s.to_ascii_lowercase(),
                "{s:?}"
            );
            assert_eq!(
                &*SmallStr::<4>::ascii_lowercase(s),
                s.to_ascii_lowercase(),
                "{s:?}"
            );
            assert_eq!(&*SmallStr::<64>::uppercase(s), s.to_uppercase(), "{s:?}");
            assert_eq!(&*SmallStr::<4>::uppercase(s), s.to_uppercase(), "{s:?}");
            let mut cap: String = s.chars().take(1).flat_map(char::to_uppercase).collect();
            cap.push_str(
                s.get(s.chars().next().map_or(0, char::len_utf8)..)
                    .unwrap_or(""),
            );
            assert_eq!(&*SmallStr::<64>::capitalize_first(s), cap, "{s:?}");
            assert_eq!(&*SmallStr::<4>::capitalize_first(s), cap, "{s:?}");
        }
        assert_eq!(&*SmallStr::<8>::hash_prefixed("4c1"), "#4c1");
    }
}
