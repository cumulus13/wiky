//! User configuration for wiky.
//!
//! Config is stored as TOML using [`confy`], which automatically places the file
//! in the OS-appropriate location:
//!
//! | Platform | Path |
//! |----------|------|
//! | Linux    | `~/.config/wiky/config.toml` |
//! | macOS    | `~/Library/Application Support/wiky/config.toml` |
//! | Windows  | `%APPDATA%\wiky\config.toml` |
//!
//! # Example config file
//!
//! ```toml
//! language = "en"
//! theme = "dark"
//! width = 100
//! pager = true
//! results_count = 10
//! open_urls = false
//!
//! [custom_theme]
//! title      = "#61AFEF"
//! heading    = "#E5C07B"
//! subheading = "#98C379"
//! body       = "#ABB2BF"
//! link       = "#56B6C2"
//! bold       = "#E06C75"
//! italic     = "#C678DD"
//! code       = "#282C34"
//! result_title   = "#61AFEF"
//! result_snippet = "#ABB2BF"
//! result_index   = "#5C6370"
//! separator      = "#3E4451"
//! error          = "#E06C75"
//! success        = "#98C379"
//! dim            = "#5C6370"
//! ```

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::WikiError;
use crate::theme::Theme;

const APP_NAME: &str = "wiky";

/// Top-level user configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Wikipedia language code (e.g. `"en"`, `"id"`, `"de"`).
    pub language: String,

    /// Active theme name. Built-in options: `dark`, `light`, `solarized`,
    /// `nord`, `dracula`. Set to `"custom"` to use `[custom_theme]`.
    pub theme: String,

    /// Terminal output width in columns (0 = auto-detect).
    pub width: u16,

    /// Pipe output through a pager (`$PAGER` or `less`).
    pub pager: bool,

    /// Default number of search results to show.
    pub results_count: u8,

    /// When a single result is found, open URL in the browser automatically.
    pub open_urls: bool,

    /// Show images alt-text in output.
    pub show_image_alt: bool,

    /// Maximum article length in bytes (0 = unlimited).
    pub max_article_bytes: usize,

    /// Custom theme (used when `theme = "custom"`).
    pub custom_theme: Theme,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: "en".into(),
            theme: "dark".into(),
            width: 0,
            pager: false,
            results_count: 10,
            open_urls: false,
            show_image_alt: false,
            max_article_bytes: 0,
            custom_theme: Theme::dark(),
        }
    }
}

impl Config {
    /// Load configuration from disk, creating defaults if no file exists.
    ///
    /// # Errors
    /// Returns [`WikiError::Config`] if the file exists but cannot be parsed.
    pub fn load() -> Result<Self, WikiError> {
        let cfg: Self = confy::load(APP_NAME, None)?;
        Ok(cfg)
    }

    /// Save the current configuration to disk.
    ///
    /// # Errors
    /// Returns [`WikiError::Config`] on IO or serialisation failure.
    pub fn save(&self) -> Result<(), WikiError> {
        confy::store(APP_NAME, None, self)?;
        Ok(())
    }

    /// Return the filesystem path where config is stored.
    pub fn path() -> Option<PathBuf> {
        confy::get_configuration_file_path(APP_NAME, None).ok()
    }

    /// Resolve and return the active [`Theme`].
    ///
    /// If `self.theme` matches a built-in name, that theme is returned.
    /// If `self.theme == "custom"`, the `custom_theme` field is cloned.
    /// Otherwise falls back to the dark theme.
    pub fn active_theme(&self) -> Theme {
        if self.theme.eq_ignore_ascii_case("custom") {
            return self.custom_theme.clone();
        }
        Theme::by_name(&self.theme).unwrap_or_else(Theme::dark)
    }

    /// Return the Wikipedia API base URL for the configured language.
    pub fn api_url(&self) -> String {
        format!(
            "https://{}.wikipedia.org/w/api.php",
            self.language.to_lowercase()
        )
    }

    /// Effective terminal width: use `self.width` if > 0, else auto-detect.
    pub fn effective_width(&self) -> u16 {
        if self.width > 0 {
            self.width
        } else {
            terminal_width()
        }
    }
}

/// Detect the terminal width, defaulting to 80 if detection fails.
///
/// Checks (in order):
/// 1. `$COLUMNS` environment variable
/// 2. `terminal_size` crate (cross-platform, works on Windows/macOS/Linux)
/// 3. Hard-coded fallback of 80
pub fn terminal_width() -> u16 {
    // 1. Shell-provided override
    if let Ok(cols) = std::env::var("COLUMNS") {
        if let Ok(n) = cols.parse::<u16>() {
            if n > 0 {
                return n;
            }
        }
    }
    // 2. Cross-platform terminal size detection via terminal_size crate
    #[cfg(not(test))]
    if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
        if w > 0 {
            return w;
        }
    }
    // 3. Safe default
    80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let c = Config::default();
        assert_eq!(c.language, "en");
        assert_eq!(c.results_count, 10);
    }

    #[test]
    fn api_url_uses_language() {
        let mut c = Config::default();
        c.language = "id".into();
        assert_eq!(c.api_url(), "https://id.wikipedia.org/w/api.php");
    }

    #[test]
    fn active_theme_builtin() {
        let mut c = Config::default();
        c.theme = "nord".into();
        let t = c.active_theme();
        // Nord title should be the Nord blue
        assert_eq!(t.title, "#88C0D0");
    }

    #[test]
    fn active_theme_custom() {
        let mut c = Config::default();
        c.theme = "custom".into();
        c.custom_theme.title = "#AABBCC".into();
        assert_eq!(c.active_theme().title, "#AABBCC");
    }
}
