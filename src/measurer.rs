//! Font character width measurer for SVG badge rendering.
//!
//! This module provides [`CharWidthMeasurer`], a utility for loading and consuming font width tables
//! (from JSON or string), and for calculating the width of strings in a given font. It is equivalent
//! to the JS CharWidthTableConsumer used in shields.io, and is used internally for accurate badge layout.
//!
//! # Typical Usage
//!
//! ```rust
//! use shields::measurer::CharWidthMeasurer;
//! let data = vec![(65, 90, 10.0), (97, 122, 8.0)]; // A-Z width 10, a-z width 8
//! let measurer = CharWidthMeasurer::from_data(data);
//! let width = measurer.width_of("Hello", true);
//! assert!(width > 0.0);
//! ```
//!
//! See [`CharWidthMeasurer`] for details.

use std::borrow::Cow;
use std::fs;
use std::io::{self};

/// Measures character widths for a given font, for use in SVG badge layout.
///
/// This struct loads a font width table (from data, JSON file, or string) and provides methods
/// to look up the width of individual characters or entire strings. Widths are stored as sorted
/// `(lower, upper, width)` code point ranges and resolved with a binary search, mirroring the
/// upstream shields.io implementation.
///
/// ## Example
/// ```rust
/// use shields::measurer::CharWidthMeasurer;
/// let data = vec![(65, 90, 10.0), (97, 122, 8.0)];
/// let measurer = CharWidthMeasurer::from_data(data);
/// let width = measurer.width_of("Hello", true);
/// assert!(width > 0.0);
/// ```
pub struct CharWidthMeasurer {
    /// Sorted, non-overlapping (lower, upper, width) code point ranges
    ranges: Cow<'static, [(u32, u32, f64)]>,
    /// Direct lookup fast path for ASCII code points; NaN marks "not in table"
    ascii: [f64; 128],
    /// Width of character 'm'
    pub em_width: f64,
}

impl CharWidthMeasurer {
    /// Returns true if the given character code is a control character (ASCII 0-31 or 127).
    ///
    /// # Arguments
    /// * `char_code` - Unicode code point.
    ///
    /// # Returns
    /// `true` if control character, else `false`.
    pub fn is_control_char(char_code: u32) -> bool {
        char_code <= 31 || char_code == 127
    }

    fn build(ranges: Cow<'static, [(u32, u32, f64)]>) -> Self {
        let mut measurer = CharWidthMeasurer {
            ranges,
            ascii: [f64::NAN; 128],
            em_width: 0.0,
        };
        for code in 0..128u32 {
            if let Some(width) = measurer.lookup_range(code) {
                measurer.ascii[code as usize] = width;
            }
        }
        measurer.em_width = measurer.width_of("m", true);
        measurer
    }

    /// Binary search over the sorted range table.
    fn lookup_range(&self, char_code: u32) -> Option<f64> {
        let idx = self
            .ranges
            .partition_point(|&(lower, _, _)| lower <= char_code);
        let &(_, upper, width) = self.ranges.get(idx.checked_sub(1)?)?;
        (char_code <= upper).then_some(width)
    }

    /// Creates a new measurer from a vector of (lower, upper, width) tuples.
    ///
    /// Each tuple defines a range of character codes and their width.
    ///
    /// # Arguments
    /// * `data` - Vector of (lower, upper, width) tuples.
    ///
    /// # Returns
    /// A new [`CharWidthMeasurer`].
    ///
    /// ## Example
    /// ```
    /// use shields::measurer::CharWidthMeasurer;
    /// let data = vec![(65, 90, 10.0), (97, 122, 8.0)];
    /// let measurer = CharWidthMeasurer::from_data(data);
    /// ```
    pub fn from_data(data: Vec<(u32, u32, f64)>) -> Self {
        let mut ranges: Vec<(u32, u32, f64)> = Vec::with_capacity(data.len());
        for new in data {
            Self::overwrite(&mut ranges, new);
        }
        Self::build(Cow::Owned(ranges))
    }

    /// Creates a new measurer borrowing a static table of sorted (lower, upper, width) ranges.
    ///
    /// The ranges must be sorted by their lower bound and non-overlapping. Unlike
    /// [`from_data`](Self::from_data), this performs no allocation or copying, which makes it
    /// suitable for tables generated at compile time.
    pub fn from_sorted_static(ranges: &'static [(u32, u32, f64)]) -> Self {
        debug_assert!(ranges.windows(2).all(|w| w[0].1 < w[1].0));
        Self::build(Cow::Borrowed(ranges))
    }

    /// Inserts `new` into the sorted, non-overlapping range list, overwriting any
    /// overlapped portion of earlier ranges (later data wins, matching the previous
    /// per-code-point overwrite semantics).
    fn overwrite(ranges: &mut Vec<(u32, u32, f64)>, new: (u32, u32, f64)) {
        let (new_lower, new_upper, _) = new;
        if new_lower > new_upper {
            return;
        }
        // Overlapping window: ranges with upper >= new_lower and lower <= new_upper
        let start = ranges.partition_point(|&(_, upper, _)| upper < new_lower);
        let end = ranges.partition_point(|&(lower, _, _)| lower <= new_upper);
        let mut replacement = Vec::with_capacity(3);
        if start < end {
            let (first_lower, _, first_width) = ranges[start];
            if first_lower < new_lower {
                replacement.push((first_lower, new_lower - 1, first_width));
            }
        }
        replacement.push(new);
        if start < end {
            let (_, last_upper, last_width) = ranges[end - 1];
            if last_upper > new_upper {
                replacement.push((new_upper + 1, last_upper, last_width));
            }
        }
        ranges.splice(start..end, replacement);
    }

    fn parse_json(data: &str) -> io::Result<Vec<(u32, u32, f64)>> {
        serde_json::from_str(data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
    }

    /// Loads a measurer from a JSON file (synchronously).
    ///
    /// # Arguments
    /// * `path` - Path to the JSON file.
    ///
    /// # Returns
    /// `Ok(CharWidthMeasurer)` if successful, or an `io::Error`.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load_sync(path: &str) -> io::Result<Self> {
        let json_str = fs::read_to_string(path)?;
        Ok(Self::from_data(Self::parse_json(&json_str)?))
    }

    /// Loads a measurer from a JSON string.
    ///
    /// # Arguments
    /// * `data` - JSON string.
    ///
    /// # Returns
    /// `Ok(CharWidthMeasurer)` if successful, or an `io::Error`.
    ///
    /// # Errors
    /// Returns an error if the string cannot be parsed.
    pub fn load_from_str(data: &str) -> io::Result<Self> {
        Ok(Self::from_data(Self::parse_json(data)?))
    }

    /// Looks up the width of a single character code.
    ///
    /// Control characters have width 0. Returns `None` if not found.
    ///
    /// # Arguments
    /// * `char_code` - Unicode code point.
    ///
    /// # Returns
    /// Some(width) if found, or None.
    ///
    /// ## Example
    /// ```
    /// use shields::measurer::CharWidthMeasurer;
    /// let data = vec![(65, 90, 10.0)];
    /// let measurer = CharWidthMeasurer::from_data(data);
    /// assert_eq!(measurer.width_of_char_code(65), Some(10.0));
    /// ```
    pub fn width_of_char_code(&self, char_code: u32) -> Option<f64> {
        if Self::is_control_char(char_code) {
            return Some(0.0);
        }
        if char_code < 128 {
            let width = self.ascii[char_code as usize];
            return if width.is_nan() { None } else { Some(width) };
        }
        self.lookup_range(char_code)
    }

    /// Calculates the width of a string.
    ///
    /// If `guess` is true, uses `em_width` for unknown characters; otherwise panics.
    ///
    /// # Arguments
    /// * `text` - The string to measure.
    /// * `guess` - Whether to guess width for unknown characters.
    ///
    /// # Returns
    /// Total width of the string.
    ///
    /// # Panics
    /// If `guess` is false and an unknown character is encountered.
    ///
    /// ## Example
    /// ```
    /// use shields::measurer::CharWidthMeasurer;
    /// let data = vec![(65, 90, 10.0)];
    /// let measurer = CharWidthMeasurer::from_data(data);
    /// let width = measurer.width_of("ABC", true);
    /// assert_eq!(width, 30.0);
    /// ```
    pub fn width_of(&self, text: &str, guess: bool) -> f64 {
        let mut total = 0.0;
        for ch in text.chars() {
            let code = ch as u32;
            match self.width_of_char_code(code) {
                Some(width) => total += width,
                None => {
                    if guess {
                        total += self.em_width;
                    } else {
                        panic!("No width available for character code {code} ({ch:?})");
                    }
                }
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_chars() {
        assert!(CharWidthMeasurer::is_control_char(0));
        assert!(CharWidthMeasurer::is_control_char(31));
        assert!(CharWidthMeasurer::is_control_char(127));
        assert!(!CharWidthMeasurer::is_control_char(32));
        assert!(!CharWidthMeasurer::is_control_char(128));
    }

    #[test]
    fn test_from_data() {
        let data = vec![(65, 90, 10.0), (97, 122, 8.0)]; // A-Z width 10, a-z width 8
        let measurer = CharWidthMeasurer::from_data(data);

        assert_eq!(measurer.width_of_char_code(65), Some(10.0)); // 'A'
        assert_eq!(measurer.width_of_char_code(90), Some(10.0)); // 'Z'
        assert_eq!(measurer.width_of_char_code(97), Some(8.0)); // 'a'
        assert_eq!(measurer.width_of_char_code(122), Some(8.0)); // 'z'
        assert_eq!(measurer.width_of_char_code(64), None); // '@'
    }

    #[test]
    fn test_from_unsorted_data() {
        let data = vec![(97, 122, 8.0), (65, 90, 10.0)];
        let measurer = CharWidthMeasurer::from_data(data);
        assert_eq!(measurer.width_of_char_code(65), Some(10.0));
        assert_eq!(measurer.width_of_char_code(97), Some(8.0));
        assert_eq!(measurer.width_of_char_code(123), None);
    }

    #[test]
    fn test_overlapping_ranges_later_wins() {
        // (109, 109) overlaps (97, 122): the later range must win for 'm',
        // while the rest of (97, 122) keeps its original width.
        let data = vec![(97, 122, 8.0), (109, 109, 16.0)];
        let measurer = CharWidthMeasurer::from_data(data);
        assert_eq!(measurer.width_of_char_code(109), Some(16.0)); // 'm'
        assert_eq!(measurer.width_of_char_code(108), Some(8.0)); // 'l'
        assert_eq!(measurer.width_of_char_code(110), Some(8.0)); // 'n'
    }

    #[test]
    fn test_from_sorted_static() {
        static RANGES: [(u32, u32, f64); 2] = [(65, 90, 10.0), (109, 109, 16.0)];
        let measurer = CharWidthMeasurer::from_sorted_static(&RANGES);
        assert_eq!(measurer.em_width, 16.0);
        assert_eq!(measurer.width_of_char_code(70), Some(10.0));
        assert_eq!(measurer.width_of_char_code(91), None);
    }

    #[test]
    fn test_width_of() {
        let data = vec![
            (65, 90, 10.0),   // A-Z width 10
            (97, 122, 8.0),   // a-z width 8
            (109, 109, 16.0), // Set width of 'm' to 16 for testing
        ];
        let measurer = CharWidthMeasurer::from_data(data);

        // Check if em_width is set correctly
        assert_eq!(measurer.em_width, 16.0);

        // Test string width calculation
        assert_eq!(measurer.width_of("ABC", true), 30.0);
        assert_eq!(measurer.width_of("abc", true), 24.0);
        assert_eq!(measurer.width_of("Am", true), 26.0);
    }

    #[test]
    #[should_panic(expected = "No width available for character code")]
    fn test_width_of_no_guess() {
        let data = vec![(65, 90, 10.0)];
        let measurer = CharWidthMeasurer::from_data(data);
        measurer.width_of("A測", false); // Should panic for unknown character '測'
    }
}
