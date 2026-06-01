//! Integration tests for wiky.

use wiky::client::strip_html;
use wiky::config::Config;
use wiky::theme::{hex_to_rgb, Theme};

// ─── Theme tests ───────────────────────────────────────────────────────────────

#[test]
fn all_builtin_themes_parse() {
    for name in &["dark", "light", "solarized", "nord", "dracula"] {
        let theme = Theme::by_name(name);
        assert!(theme.is_some(), "Theme '{name}' should exist");
        let theme = theme.unwrap();
        // Each color should be a valid hex
        for color in [
            &theme.title,
            &theme.heading,
            &theme.body,
            &theme.link,
            &theme.error,
            &theme.success,
        ] {
            assert!(
                hex_to_rgb(color).is_ok(),
                "Color '{color}' in theme '{name}' should be valid hex"
            );
        }
    }
}

#[test]
fn custom_theme_unknown_name_falls_back() {
    let theme = Theme::by_name("does_not_exist");
    assert!(theme.is_none());
}

// ─── Config tests ──────────────────────────────────────────────────────────────

#[test]
fn config_default_lang_is_en() {
    let c = Config::default();
    assert_eq!(c.language, "en");
}

#[test]
fn config_api_url_format() {
    let mut c = Config::default();
    c.language = "fr".into();
    assert!(c.api_url().starts_with("https://fr.wikipedia.org"));
}

#[test]
fn config_active_theme_dark() {
    let mut c = Config::default();
    c.theme = "dark".into();
    let t = c.active_theme();
    assert_eq!(t.title, "#61AFEF");
}

#[test]
fn config_active_theme_custom() {
    let mut c = Config::default();
    c.theme = "custom".into();
    c.custom_theme.title = "#FF0000".into();
    let t = c.active_theme();
    assert_eq!(t.title, "#FF0000");
}

// ─── Client utility tests ──────────────────────────────────────────────────────

#[test]
fn strip_html_removes_tags() {
    assert_eq!(strip_html("<b>bold</b> text"), "bold text");
    assert_eq!(strip_html("<a href='x'>link</a>"), "link");
}

#[test]
fn strip_html_decodes_entities() {
    assert_eq!(strip_html("AT&amp;T"), "AT&T");
    assert_eq!(strip_html("&lt;tag&gt;"), "<tag>");
    assert_eq!(strip_html("&nbsp;"), " ");
}

// ─── Hex color tests ──────────────────────────────────────────────────────────

#[test]
fn hex_parse_uppercase() {
    assert_eq!(hex_to_rgb("#FFFFFF").unwrap(), (255, 255, 255));
}

#[test]
fn hex_parse_lowercase() {
    assert_eq!(hex_to_rgb("#000000").unwrap(), (0, 0, 0));
}

#[test]
fn hex_parse_shorthand() {
    assert_eq!(hex_to_rgb("#fff").unwrap(), (255, 255, 255));
    assert_eq!(hex_to_rgb("#000").unwrap(), (0, 0, 0));
}

#[test]
fn hex_parse_without_hash() {
    assert_eq!(hex_to_rgb("61AFEF").unwrap(), (97, 175, 239));
}

#[test]
fn hex_parse_invalid() {
    assert!(hex_to_rgb("#ZZZZZZ").is_err());
    assert!(hex_to_rgb("#12345").is_err());
    assert!(hex_to_rgb("").is_err());
}
