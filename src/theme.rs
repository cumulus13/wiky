//! Theme system for wiky.
//!
//! Provides hex-color parsing, built-in themes (dark/light/solarized/nord/custom),
//! and helpers to apply truecolor ANSI codes to terminal output — effectively
//! implementing the spirit of the `make_colors` crate requested.

use colored::{ColoredString, Colorize};
use serde::{Deserialize, Serialize};

use crate::error::WikiError;

// ─── Color helpers ────────────────────────────────────────────────────────────

/// Parse a CSS-style hex color string (`#RRGGBB` or `#RGB`) into `(r, g, b)`.
///
/// # Examples
/// ```
/// use wiky::theme::hex_to_rgb;
/// assert_eq!(hex_to_rgb("#ff6600").unwrap(), (255, 102, 0));
/// assert_eq!(hex_to_rgb("#f60").unwrap(),    (255, 102, 0));
/// ```
pub fn hex_to_rgb(hex: &str) -> Result<(u8, u8, u8), WikiError> {
    let hex = hex.trim().trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| WikiError::InvalidColor(format!("#{hex}")))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| WikiError::InvalidColor(format!("#{hex}")))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| WikiError::InvalidColor(format!("#{hex}")))?;
            Ok((r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                .map_err(|_| WikiError::InvalidColor(format!("#{hex}")))?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                .map_err(|_| WikiError::InvalidColor(format!("#{hex}")))?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                .map_err(|_| WikiError::InvalidColor(format!("#{hex}")))?;
            Ok((r, g, b))
        }
        _ => Err(WikiError::InvalidColor(format!("#{hex}"))),
    }
}

/// Apply a hex foreground color to a string using truecolor ANSI.
///
/// Falls back gracefully if the terminal doesn't support truecolor.
pub fn colorize_hex<S: AsRef<str>>(text: S, hex: &str) -> ColoredString {
    match hex_to_rgb(hex) {
        Ok((r, g, b)) => text.as_ref().truecolor(r, g, b),
        Err(_) => text.as_ref().normal(),
    }
}

/// Apply a hex background color to a string using truecolor ANSI.
pub fn bg_hex<S: AsRef<str>>(text: S, hex: &str) -> ColoredString {
    match hex_to_rgb(hex) {
        Ok((r, g, b)) => text.as_ref().on_truecolor(r, g, b),
        Err(_) => text.as_ref().normal(),
    }
}

// ─── Theme definition ─────────────────────────────────────────────────────────

/// A complete color theme for wiky terminal output.
///
/// All color values are hex strings (`#RRGGBB`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Article title color.
    pub title: String,
    /// Section heading color.
    pub heading: String,
    /// Sub-section heading color.
    pub subheading: String,
    /// Body text color.
    pub body: String,
    /// URL / link color.
    pub link: String,
    /// Bold / emphasis color.
    pub bold: String,
    /// Italic text color.
    pub italic: String,
    /// Code block color.
    pub code: String,
    /// Search-result title color.
    pub result_title: String,
    /// Search-result snippet color.
    pub result_snippet: String,
    /// Result index number color.
    pub result_index: String,
    /// Separator / rule color.
    pub separator: String,
    /// Error message color.
    pub error: String,
    /// Success / info message color.
    pub success: String,
    /// Dim / muted text color.
    pub dim: String,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    // ── Built-in themes ───────────────────────────────────────────────────────

    /// Classic dark-terminal theme (default).
    pub fn dark() -> Self {
        Self {
            title: "#61AFEF".into(),
            heading: "#E5C07B".into(),
            subheading: "#98C379".into(),
            body: "#ABB2BF".into(),
            link: "#56B6C2".into(),
            bold: "#E06C75".into(),
            italic: "#C678DD".into(),
            code: "#282C34".into(),
            result_title: "#61AFEF".into(),
            result_snippet: "#ABB2BF".into(),
            result_index: "#5C6370".into(),
            separator: "#3E4451".into(),
            error: "#E06C75".into(),
            success: "#98C379".into(),
            dim: "#5C6370".into(),
        }
    }

    /// Light-terminal theme.
    pub fn light() -> Self {
        Self {
            title: "#005F87".into(),
            heading: "#AF5F00".into(),
            subheading: "#008700".into(),
            body: "#1C1C1C".into(),
            link: "#0087AF".into(),
            bold: "#AF0000".into(),
            italic: "#875FAF".into(),
            code: "#EEEEEE".into(),
            result_title: "#005F87".into(),
            result_snippet: "#4E4E4E".into(),
            result_index: "#9E9E9E".into(),
            separator: "#BCBCBC".into(),
            error: "#AF0000".into(),
            success: "#008700".into(),
            dim: "#9E9E9E".into(),
        }
    }

    /// Solarized Dark theme.
    pub fn solarized() -> Self {
        Self {
            title: "#268BD2".into(),
            heading: "#B58900".into(),
            subheading: "#2AA198".into(),
            body: "#839496".into(),
            link: "#2AA198".into(),
            bold: "#DC322F".into(),
            italic: "#D33682".into(),
            code: "#073642".into(),
            result_title: "#268BD2".into(),
            result_snippet: "#657B83".into(),
            result_index: "#586E75".into(),
            separator: "#073642".into(),
            error: "#DC322F".into(),
            success: "#859900".into(),
            dim: "#586E75".into(),
        }
    }

    /// Nord theme.
    pub fn nord() -> Self {
        Self {
            title: "#88C0D0".into(),
            heading: "#EBCB8B".into(),
            subheading: "#A3BE8C".into(),
            body: "#D8DEE9".into(),
            link: "#81A1C1".into(),
            bold: "#BF616A".into(),
            italic: "#B48EAD".into(),
            code: "#2E3440".into(),
            result_title: "#88C0D0".into(),
            result_snippet: "#D8DEE9".into(),
            result_index: "#4C566A".into(),
            separator: "#3B4252".into(),
            error: "#BF616A".into(),
            success: "#A3BE8C".into(),
            dim: "#4C566A".into(),
        }
    }

    /// Dracula theme.
    pub fn dracula() -> Self {
        Self {
            title: "#8BE9FD".into(),
            heading: "#F1FA8C".into(),
            subheading: "#50FA7B".into(),
            body: "#F8F8F2".into(),
            link: "#8BE9FD".into(),
            bold: "#FF5555".into(),
            italic: "#FF79C6".into(),
            code: "#282A36".into(),
            result_title: "#BD93F9".into(),
            result_snippet: "#F8F8F2".into(),
            result_index: "#6272A4".into(),
            separator: "#44475A".into(),
            error: "#FF5555".into(),
            success: "#50FA7B".into(),
            dim: "#6272A4".into(),
        }
    }

    // ── Theme lookup ──────────────────────────────────────────────────────────

    /// Get a built-in theme by name (case-insensitive).
    ///
    /// Returns `None` if the name is not recognised — caller should fall
    /// back to the user-supplied custom theme from config.
    pub fn by_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "dark" => Some(Self::dark()),
            "light" => Some(Self::light()),
            "solarized" => Some(Self::solarized()),
            "nord" => Some(Self::nord()),
            "dracula" => Some(Self::dracula()),
            _ => None,
        }
    }

    // ── Colorize helpers ──────────────────────────────────────────────────────

    /// Colorize text using the theme's `title` color.
    pub fn title<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.title).bold()
    }

    /// Colorize text using the theme's `heading` color.
    pub fn heading<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.heading).bold()
    }

    /// Colorize text using the theme's `subheading` color.
    pub fn subheading<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.subheading)
    }

    /// Colorize text using the theme's `body` color.
    pub fn body<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.body)
    }

    /// Colorize text using the theme's `link` color.
    pub fn link<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.link).underline()
    }

    /// Colorize text using the theme's `bold` color.
    pub fn bold_text<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.bold).bold()
    }

    /// Colorize text using the theme's `italic` color.
    pub fn italic_text<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.italic).italic()
    }

    /// Colorize text using the theme's `code` color as background.
    pub fn code_text<S: AsRef<str>>(&self, s: S) -> ColoredString {
        // Use body-colored text on code background
        match (hex_to_rgb(&self.body), hex_to_rgb(&self.code)) {
            (Ok((fr, fg, fb)), Ok((br, bg, bb))) => {
                s.as_ref().truecolor(fr, fg, fb).on_truecolor(br, bg, bb)
            }
            _ => s.as_ref().normal(),
        }
    }

    /// Colorize text using the theme's `result_title` color.
    pub fn result_title<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.result_title).bold()
    }

    /// Colorize text using the theme's `result_snippet` color.
    pub fn result_snippet<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.result_snippet)
    }

    /// Colorize text using the theme's `result_index` color.
    pub fn result_index<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.result_index)
    }

    /// Colorize text using the theme's `separator` color.
    pub fn separator<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.separator)
    }

    /// Colorize text using the theme's `error` color.
    pub fn error<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.error).bold()
    }

    /// Colorize text using the theme's `success` color.
    pub fn success<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.success)
    }

    /// Colorize text using the theme's `dim` color.
    pub fn dim<S: AsRef<str>>(&self, s: S) -> ColoredString {
        colorize_hex(s, &self.dim)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_rgb_6_digit() {
        assert_eq!(hex_to_rgb("#ff6600").unwrap(), (255, 102, 0));
        assert_eq!(hex_to_rgb("ff6600").unwrap(), (255, 102, 0));
    }

    #[test]
    fn test_hex_to_rgb_3_digit() {
        assert_eq!(hex_to_rgb("#f60").unwrap(), (255, 102, 0));
    }

    #[test]
    fn test_hex_invalid() {
        assert!(hex_to_rgb("#gg0000").is_err());
        assert!(hex_to_rgb("#12345").is_err());
    }

    #[test]
    fn test_theme_by_name() {
        assert!(Theme::by_name("dark").is_some());
        assert!(Theme::by_name("NORD").is_some());
        assert!(Theme::by_name("unknown").is_none());
    }
}
