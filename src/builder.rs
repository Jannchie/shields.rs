//! Badge builder module for shields crate.
//!
//! Provides a builder-pattern API for constructing SVG badges with a fluent, ergonomic interface.
//! This module is ideal for users who want to configure badges step-by-step or with method chaining.
//!
//! ## Example
//!
//! ```rust
//! use shields::{BadgeStyle};
//! use shields::builder::Badge;
//!
//! let svg = Badge::style(BadgeStyle::Flat)
//!     .label("build")
//!     .message("passing")
//!     .label_color("green")
//!     .message_color("brightgreen")
//!     .logo("github")
//!     .build();
//! assert!(svg.contains("passing"));
//! ```
//!
//! See [`BadgeBuilder`] and [`Badge`] for details.
use crate::{
    BadgeParams, BadgeStyle, RenderOptions, default_label_color, default_message_color,
    render_badge_svg_with,
};

/// Builder for constructing SVG badges with a fluent API.
///
/// Use [`Badge::style`] to create a new builder, then chain methods to set label, message, colors, logo, and links.
/// Call [`build`](BadgeBuilder::build) to generate the SVG string.
///
/// ## Example
///
/// ```rust
/// use shields::{BadgeStyle};
/// use shields::builder::Badge;
///
/// let svg = Badge::style(BadgeStyle::Flat)
///     .label("build")
///     .message("passing")
///     .label_color("green")
///     .message_color("brightgreen")
///     .logo("github")
///     .build();
/// assert!(svg.contains("passing"));
/// ```
pub struct BadgeBuilder<'a> {
    style: BadgeStyle,
    label: Option<&'a str>,
    message: Option<&'a str>,
    label_color: Option<&'a str>,
    message_color: Option<&'a str>,
    logo: Option<&'a str>,
    logo_color: Option<&'a str>,
    link: Option<&'a str>,
    extra_link: Option<&'a str>,
    id_suffix: Option<&'a str>,
}

impl<'a> BadgeBuilder<'a> {
    /// Creates a new badge builder with the specified style.
    ///
    /// This is usually called via [`Badge::style`].
    ///
    /// # Arguments
    /// * `style` - The badge style to use.
    fn new(style: BadgeStyle) -> Self {
        Self {
            style,
            label: None,
            message: None,
            label_color: None,
            message_color: None,
            logo: None,
            logo_color: None,
            link: None,
            extra_link: None,
            id_suffix: None,
        }
    }

    /// Sets the label text (left side).
    ///
    /// # Arguments
    /// * `label` - The label text.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    ///
    /// ## Example
    /// ```
    /// use shields::{BadgeStyle};
    /// use shields::builder::Badge;
    ///
    /// let mut builder = Badge::style(BadgeStyle::Flat);
    /// builder.label("build");
    /// ```
    pub fn label(&mut self, label: &'a str) -> &mut Self {
        self.label = Some(label);
        self
    }

    /// Sets the message text (right side).
    ///
    /// # Arguments
    /// * `message` - The message text.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn message(&mut self, message: &'a str) -> &mut Self {
        self.message = Some(message);
        self
    }

    /// Sets the label background color.
    ///
    /// # Arguments
    /// * `color` - Color string (hex, name, or alias).
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn label_color(&mut self, color: &'a str) -> &mut Self {
        self.label_color = Some(color);
        self
    }

    /// Sets the message background color.
    ///
    /// # Arguments
    /// * `color` - Color string (hex, name, or alias).
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn message_color(&mut self, color: &'a str) -> &mut Self {
        self.message_color = Some(color);
        self
    }

    /// Sets the logo (name or SVG data).
    ///
    /// # Arguments
    /// * `logo` - Logo name or SVG data.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn logo(&mut self, logo: &'a str) -> &mut Self {
        self.logo = Some(logo);
        self
    }

    /// Sets the logo color.
    ///
    /// # Arguments
    /// * `color` - Logo color string.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn logo_color(&mut self, color: &'a str) -> &mut Self {
        self.logo_color = Some(color);
        self
    }

    /// Sets the main link URL.
    ///
    /// # Arguments
    /// * `link` - Main link URL.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn link(&mut self, link: &'a str) -> &mut Self {
        self.link = Some(link);
        self
    }

    /// Sets the extra (secondary) link URL.
    ///
    /// # Arguments
    /// * `link` - Extra link URL.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn extra_link(&mut self, link: &'a str) -> &mut Self {
        self.extra_link = Some(link);
        self
    }

    /// Sets a suffix appended to every SVG element id.
    ///
    /// SVGs embedded inline in the same HTML page share one id namespace, so
    /// two badges both defining `id="s"` corrupt each other's gradients. Give
    /// each badge a unique suffix to avoid collisions. Only `[A-Za-z0-9_-]`
    /// characters are kept.
    ///
    /// # Arguments
    /// * `id_suffix` - The suffix, e.g. `"badge1"`.
    ///
    /// # Returns
    /// Mutable reference to self for chaining.
    pub fn id_suffix(&mut self, id_suffix: &'a str) -> &mut Self {
        self.id_suffix = Some(id_suffix);
        self
    }

    /// Builds and returns the SVG badge string.
    ///
    /// # Returns
    /// SVG string representing the badge.
    ///
    /// ## Example
    /// ```
    /// use shields::{BadgeStyle};
    /// use shields::builder::Badge;
    ///
    /// let svg = Badge::style(BadgeStyle::Flat)
    ///     .label("build")
    ///     .message("passing")
    ///     .build();
    /// assert!(svg.contains("passing"));
    /// ```
    pub fn build(&self) -> String {
        let (label_color, message_color) = if self.style == BadgeStyle::Social {
            (None, Some(""))
        } else {
            (
                Some(self.label_color.unwrap_or(default_label_color())),
                Some(self.message_color.unwrap_or(default_message_color())),
            )
        };

        render_badge_svg_with(
            &BadgeParams {
                style: self.style,
                label: self.label,
                message: self.message,
                label_color,
                message_color,
                logo: self.logo,
                logo_color: self.logo_color,
                link: self.link,
                extra_link: self.extra_link,
            },
            &RenderOptions::default().id_suffix(self.id_suffix.unwrap_or("")),
        )
    }
}

/// Entry point for badge builder API.
///
/// This struct acts as a namespace for the builder pattern.
/// Use [`Badge::style`] to start building a badge.
///
/// ## Example
/// ```rust
/// use shields::{BadgeStyle};
/// use shields::builder::Badge;
///
/// let svg = Badge::style(BadgeStyle::Flat)
///     .label("build")
///     .message("passing")
///     .build();
/// assert!(svg.contains("passing"));
/// ```
pub struct Badge;

impl Badge {
    /// Creates a new [`BadgeBuilder`] with the specified style.
    ///
    /// # Arguments
    /// * `style` - The badge style to use.
    ///
    /// # Returns
    /// A [`BadgeBuilder`] for further configuration.
    ///
    /// ## Example
    /// ```
    /// use shields::{BadgeStyle};
    /// use shields::builder::Badge;
    ///
    /// let builder = Badge::style(BadgeStyle::Flat);
    /// ```
    pub fn style(style: BadgeStyle) -> BadgeBuilder<'static> {
        BadgeBuilder::new(style)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_flat_badge() {
        let mut builder = Badge::style(BadgeStyle::Flat);
        let badge = builder
            .label("test1")
            .message("test2")
            .label_color("#000000")
            .message_color("#ffffff")
            .build();

        assert!(badge.contains("test1"));
        assert!(badge.contains("test2"));
    }

    #[test]
    fn test_optimized_chaining() {
        let badge = Badge::style(BadgeStyle::Plastic)
            .label("version")
            .message("1.0.0")
            .logo("github")
            .build();

        assert!(badge.contains("version"));
        assert!(badge.contains("1.0.0"));
    }

    #[test]
    fn test_alternative_chaining() {
        let badge = {
            let mut b = Badge::style(BadgeStyle::Flat);
            b.label("hello").message("world");
            b.build()
        };
        assert!(badge.contains("hello"));
        assert!(badge.contains("world"));
    }
    #[test]
    fn test_configuring_step_by_step() {
        let mut badge = Badge::style(BadgeStyle::Flat);
        badge.label("no chaining");
        badge.message("test");
        badge.label_color("#000");
        badge.message_color("#fff");
        let resp = badge.build();
        assert!(resp.contains("no chaining"));
        assert!(resp.contains("test"));
    }
}
