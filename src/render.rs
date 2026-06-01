//! Terminal renderer for Wikipedia articles.
//!
//! Renders article content with hex colors, emoji, and word-wrapping.
//! Supports writing to any [`std::io::Write`] target — stdout for normal
//! display, or a `Vec<u8>` buffer for pager capture.

use std::io::{self, Write};

use textwrap::Options;

use crate::client::{Article, DisambigOption, SearchResult};
use crate::config::Config;
use crate::theme::Theme;

// ─── Emoji constants ──────────────────────────────────────────────────────────

const EMOJI_TITLE: &str = "📖";
const EMOJI_HEADING: &str = "📌";
const EMOJI_SUBHEAD: &str = "🔹";
const EMOJI_BULLET: &str = "•";
const EMOJI_LINK: &str = "🔗";
const EMOJI_SEARCH: &str = "🔍";
const EMOJI_RESULT: &str = "📄";
const EMOJI_CATEGORY: &str = "🏷️";
const EMOJI_SEPARATOR: &str = "─";
const EMOJI_SUMMARY: &str = "💡";
const EMOJI_WARN: &str = "⚠️";
const EMOJI_OK: &str = "✅";

// ─── Renderer ─────────────────────────────────────────────────────────────────

/// Terminal renderer for Wikipedia content.
///
/// Renders to any [`Write`] target — use [`Renderer::stdout`] for normal
/// display, or [`Renderer::to_buf`] to capture ANSI-colored output into a
/// `Vec<u8>` (e.g. for piping into a pager).
///
/// # Example
/// ```rust,no_run
/// use wiky::{Config, Renderer};
///
/// let config  = Config::default();
/// let theme   = config.active_theme();
/// let renderer = Renderer::stdout(theme, 100, true);
/// renderer.print_message("Hello, wiky! ✨");
/// ```
pub struct Renderer {
    theme: Theme,
    width: u16,
    emoji: bool,
}

impl Renderer {
    /// Create a renderer that writes to stdout.
    pub fn stdout(theme: Theme, width: u16, emoji: bool) -> Self {
        Self {
            theme,
            width,
            emoji,
        }
    }

    /// Backwards-compatible constructor alias.
    pub fn new(theme: Theme, width: u16, emoji: bool) -> Self {
        Self::stdout(theme, width, emoji)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn w(&self) -> usize {
        self.width as usize
    }

    fn em<'a>(&self, icon: &'a str) -> &'a str {
        if self.emoji {
            icon
        } else {
            ""
        }
    }

    fn sep_line(&self) -> String {
        EMOJI_SEPARATOR.repeat(self.w().min(80))
    }

    fn wrap(&self, text: &str, indent: &str) -> Vec<String> {
        let opts = Options::new(self.w().saturating_sub(indent.len()))
            .initial_indent(indent)
            .subsequent_indent(indent);
        textwrap::wrap(text, opts)
            .into_iter()
            .map(|s| s.into_owned())
            .collect()
    }

    // ── Render to a String (ANSI-colored) ────────────────────────────────────

    /// Render a full article into a colored `String`.
    pub fn article_to_string(&self, article: &Article) -> String {
        let mut buf = Vec::new();
        self.write_article(&mut buf, article).unwrap_or(());
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Render search results into a colored `String`.
    pub fn search_to_string(&self, query: &str, results: &[SearchResult]) -> String {
        let mut buf = Vec::new();
        self.write_search_results(&mut buf, query, results)
            .unwrap_or(());
        String::from_utf8_lossy(&buf).into_owned()
    }

    /// Render a summary into a colored `String`.
    pub fn summary_to_string(&self, article: &Article) -> String {
        let mut buf = Vec::new();
        self.write_summary(&mut buf, article).unwrap_or(());
        String::from_utf8_lossy(&buf).into_owned()
    }

    // ── Public print API (writes to stdout) ───────────────────────────────────

    /// Render a full [`Article`] to stdout.
    pub fn render_article(&self, article: &Article) {
        let _ = self.write_article(&mut io::stdout(), article);
    }

    /// Render only the summary of an article to stdout.
    pub fn render_summary(&self, article: &Article) {
        let _ = self.write_summary(&mut io::stdout(), article);
    }

    /// Render a list of search results to stdout.
    pub fn render_search_results(&self, query: &str, results: &[SearchResult]) {
        let _ = self.write_search_results(&mut io::stdout(), query, results);
    }

    /// Print a plain message to stdout.
    pub fn print_message(&self, msg: &str) {
        println!("{}", self.theme.body(msg));
    }

    /// Return a dim-colored string (useful for callers that do their own printing).
    pub fn dim_str(&self, s: &str) -> String {
        self.theme.dim(s).to_string()
    }

    /// Print a success message to stdout.
    pub fn print_success(&self, msg: &str) {
        let line = format!("{} {}", self.em(EMOJI_OK), msg);
        println!("{}", self.theme.success(&line));
    }

    /// Print an error message to stderr.
    pub fn print_error(&self, msg: &str) {
        let line = format!("{} {}", self.em(EMOJI_WARN), msg);
        eprintln!("{}", self.theme.error(&line));
    }

    /// Render current config settings to stdout.
    pub fn render_config_info(&self, config: &Config) {
        println!();
        println!("{}", self.theme.title("⚙️  wiky Configuration"));
        println!("{}", self.theme.separator(self.sep_line()));
        let rows = [
            ("language", config.language.clone()),
            ("theme", config.theme.clone()),
            ("width", config.width.to_string()),
            ("pager", config.pager.to_string()),
            ("results_count", config.results_count.to_string()),
            ("open_urls", config.open_urls.to_string()),
        ];
        for (k, v) in &rows {
            println!("  {:<20} {}", self.theme.heading(k), self.theme.body(v));
        }
        if let Some(path) = Config::path() {
            println!();
            println!(
                "  {:<20} {}",
                self.theme.dim("config file"),
                self.theme.link(path.display().to_string().as_str())
            );
        }
        println!();
    }

    /// Render the list of available themes to stdout.
    pub fn render_themes_list(&self) {
        println!();
        println!("{}", self.theme.title("🎨  Available Themes"));
        println!("{}", self.theme.separator(self.sep_line()));
        let active = self.theme_name();
        for t in &["dark", "light", "solarized", "nord", "dracula", "custom"] {
            let marker = if *t == active { " ← active" } else { "" };
            println!(
                "  {} {}{}",
                self.em("🖌️"),
                self.theme.heading(t),
                self.theme.dim(marker)
            );
        }
        println!();
        println!(
            "{}",
            self.theme
                .dim("  Set theme: wiky config set theme <name>")
        );
        println!();
    }

    /// Render a disambiguation page options to stdout as a numbered list.
    pub fn render_disambiguation(&self, article: &Article, opts: &[DisambigOption]) {
        println!();
        println!(
            "{}",
            self.theme
                .title(format!("🔀  {} — disambiguation", article.title))
        );
        println!("{}", self.theme.separator(self.sep_line()));
        println!(
            "{}",
            self.theme
                .dim("  This page has multiple meanings. Choose one:")
        );
        println!();
        for (i, opt) in opts.iter().enumerate() {
            println!(
                "  {} {}",
                self.theme.result_index(format!("[{}]", i + 1)),
                self.theme.result_snippet(&opt.label)
            );
        }
        println!();
        println!(
            "  {} {}",
            self.theme.result_index("[0]"),
            self.theme.dim("quit / go back")
        );
        println!();
    }

    fn theme_name(&self) -> &str {
        match self.theme.title.as_str() {
            "#88C0D0" => "nord",
            "#268BD2" => "solarized",
            "#8BE9FD" => "dracula",
            "#005F87" => "light",
            _ => "dark",
        }
    }

    // ── Core write methods (generic over Write) ───────────────────────────────

    fn write_article(&self, w: &mut dyn Write, article: &Article) -> io::Result<()> {
        writeln!(w)?;
        let title_line = format!("{} {}", self.em(EMOJI_TITLE), article.title);
        writeln!(w, "{}", self.theme.title(&title_line))?;
        writeln!(w, "{}", self.theme.separator(self.sep_line()))?;

        let url_line = format!("{} {}", self.em(EMOJI_LINK), article.url);
        writeln!(w, "{}", self.theme.link(&url_line))?;
        writeln!(w)?;

        self.write_body(w, &article.content)?;

        if !article.categories.is_empty() {
            writeln!(w)?;
            writeln!(w, "{}", self.theme.separator(self.sep_line()))?;
            let cat_hdr = format!("{} Categories:", self.em(EMOJI_CATEGORY));
            writeln!(w, "{}", self.theme.dim(&cat_hdr))?;
            let cats: Vec<String> = article
                .categories
                .iter()
                .take(10)
                .map(|c| format!("  [{}]", c))
                .collect();
            writeln!(w, "{}", self.theme.dim(cats.join("  ")))?;
        }
        writeln!(w)?;
        Ok(())
    }

    fn write_summary(&self, w: &mut dyn Write, article: &Article) -> io::Result<()> {
        writeln!(w)?;
        let hdr = format!("{} {}", self.em(EMOJI_SUMMARY), article.title);
        writeln!(w, "{}", self.theme.title(&hdr))?;
        writeln!(w, "{}", self.theme.separator(self.sep_line()))?;
        let url_line = format!("{} {}", self.em(EMOJI_LINK), article.url);
        writeln!(w, "{}", self.theme.link(&url_line))?;
        writeln!(w)?;
        if article.summary.is_empty() {
            writeln!(w, "{}", self.theme.dim("(no summary available)"))?;
        } else {
            for line in self.wrap(&article.summary, "") {
                writeln!(w, "{}", self.theme.body(&line))?;
            }
        }
        writeln!(w)?;
        Ok(())
    }

    fn write_search_results(
        &self,
        w: &mut dyn Write,
        query: &str,
        results: &[SearchResult],
    ) -> io::Result<()> {
        writeln!(w)?;
        let hdr = format!(
            "{} Search: \"{}\"  ({} results)",
            self.em(EMOJI_SEARCH),
            query,
            results.len()
        );
        writeln!(w, "{}", self.theme.title(&hdr))?;
        writeln!(w, "{}", self.theme.separator(self.sep_line()))?;

        if results.is_empty() {
            let msg = format!("{} No results found for \"{}\"", self.em(EMOJI_WARN), query);
            writeln!(w, "{}", self.theme.error(&msg))?;
            return Ok(());
        }

        for (i, r) in results.iter().enumerate() {
            writeln!(w)?;
            let idx = format!("[{}]", i + 1);
            writeln!(
                w,
                "{} {} {}",
                self.theme.result_index(&idx),
                self.em(EMOJI_RESULT),
                self.theme.result_title(&r.title)
            )?;
            if r.wordcount > 0 {
                writeln!(
                    w,
                    "{}",
                    self.theme.dim(format!("    {} words", r.wordcount))
                )?;
            }
            if !r.snippet.is_empty() {
                let clean = r.snippet.replace('\n', " ");
                for line in self.wrap(&clean, "    ") {
                    writeln!(w, "{}", self.theme.result_snippet(&line))?;
                }
            }
        }

        writeln!(w)?;
        writeln!(w, "{}", self.theme.separator(self.sep_line()))?;
        writeln!(
            w,
            "{}",
            self.theme
                .dim("  tip: use `wiky get \"<title>\"` to read an article")
        )?;
        writeln!(w)?;
        Ok(())
    }

    fn write_body(&self, w: &mut dyn Write, text: &str) -> io::Result<()> {
        for line in text.lines() {
            let t = line.trim_end();
            if let Some(rest) = t.strip_prefix("#### ") {
                writeln!(w, "{}", self.theme.italic_text(format!("  {rest}")))?;
            } else if let Some(rest) = t.strip_prefix("### ") {
                writeln!(
                    w,
                    "  {}{}",
                    self.em(EMOJI_SUBHEAD),
                    self.theme.subheading(format!(" {rest}"))
                )?;
            } else if let Some(rest) = t.strip_prefix("## ") {
                writeln!(w)?;
                writeln!(
                    w,
                    "{}{}",
                    self.em(EMOJI_HEADING),
                    self.theme.heading(format!(" {rest}"))
                )?;
                writeln!(
                    w,
                    "{}",
                    self.theme.separator("─".repeat(rest.chars().count() + 2))
                )?;
            } else if let Some(rest) = t.strip_prefix("# ") {
                writeln!(w)?;
                writeln!(w, "{}", self.theme.title(rest))?;
            } else if t.starts_with("  •") {
                let content = t.trim_start_matches("  •").trim();
                let bullet = format!("  {} ", EMOJI_BULLET);
                for line in self.wrap(content, &bullet) {
                    writeln!(w, "{}", self.theme.body(&line))?;
                }
            } else if t.is_empty() {
                writeln!(w)?;
            } else {
                for wrapped in self.wrap(t, "") {
                    writeln!(w, "{}", self.theme.body(&wrapped))?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn renderer() -> Renderer {
        Renderer::new(Theme::dark(), 80, false)
    }

    #[test]
    fn article_to_string_contains_title() {
        let r = renderer();
        let article = Article {
            title: "Test Article".into(),
            content: "Some content.".into(),
            url: "https://en.wikipedia.org/wiki/Test".into(),
            lang: "en".into(),
            pageid: 1,
            categories: vec![],
            summary: "Some content.".into(),
        };
        let s = r.article_to_string(&article);
        assert!(s.contains("Test Article"), "got: {s}");
    }

    #[test]
    fn wrap_respects_width() {
        let r = Renderer::new(Theme::dark(), 40, true);
        let long = "This is a long line that should be wrapped at 40 columns by textwrap.";
        let lines = r.wrap(long, "");
        assert!(lines.len() > 1);
    }
}
