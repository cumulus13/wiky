//! Wikipedia API client.
//!
//! Provides async methods for searching Wikipedia and fetching full articles.
//! Uses the [MediaWiki Action API](https://www.mediawiki.org/wiki/API:Main_page).
//!
//! ## API strategy
//!
//! All requests use `formatversion=2` which returns `pages` as a **JSON array**
//! (not a numeric-keyed map). `action=query&prop=extracts&explaintext=1` returns
//! clean plain text — no HTML parsing needed.

use reqwest::Client;
use serde::Deserialize;

use crate::config::Config;
use crate::error::WikiError;

/// A Wikipedia search result.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub snippet: String,
    pub wordcount: u32,
    pub pageid: u64,
}

/// A full Wikipedia article.
#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub content: String,
    pub url: String,
    pub lang: String,
    pub pageid: u64,
    pub categories: Vec<String>,
    pub summary: String,
}

// ─── Internal API shapes (formatversion=2) ────────────────────────────────────
//
// formatversion=1 (default):  {"query": {"pages": {"12345": {...}}}}
// formatversion=2:            {"query": {"pages": [{"pageid": 12345, ...}]}}
//
// We use formatversion=2 throughout.

#[derive(Deserialize, Debug)]
struct ApiQuery<T> {
    query: Option<T>,
    #[serde(default)]
    error: Option<ApiError>,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    info: String,
}

// ── Search ────────────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct SearchBody {
    search: Vec<SearchItem>,
}

#[derive(Deserialize, Debug)]
struct SearchItem {
    title: String,
    snippet: String,
    #[serde(default)]
    wordcount: u32,
    pageid: u64,
}

// ── Extracts (formatversion=2 → pages is an ARRAY) ────────────────────────────

#[derive(Deserialize, Debug)]
struct ExtractBody {
    pages: Vec<ExtractPage>,
}

#[derive(Deserialize, Debug)]
struct ExtractPage {
    #[serde(default)]
    pageid: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    extract: String,
    #[serde(default)]
    categories: Vec<CategoryItem>,
    /// Present (as `true`) when the page does not exist.
    #[serde(default)]
    missing: bool,
}

#[derive(Deserialize, Debug)]
struct CategoryItem {
    title: String,
}

// ─── Article helpers ─────────────────────────────────────────────────────────

impl Article {
    /// Returns true if this is a Wikipedia disambiguation page.
    ///
    /// Detected by either:
    /// - A category containing "disambiguation"
    /// - The title ending with "(disambiguation)"
    /// - The content starting with the canonical disambiguation phrase
    pub fn is_disambiguation(&self) -> bool {
        self.categories
            .iter()
            .any(|c| c.to_lowercase().contains("disambiguation"))
            || self.title.to_lowercase().ends_with("(disambiguation)")
            || self.content.trim_start().starts_with(&self.title)
                && self.content.contains("may refer to")
    }

    /// Parse disambiguation options from plain-text content.
    ///
    /// Returns a list of  pairs. The label is the
    /// full description line; the article title is the first part before
    /// the first comma or dash, cleaned up for use as a Wikipedia title.
    /// Parse disambiguation options from plain-text content.
    ///
    /// This is a fallback — prefer [`WikiClient::fetch_disambiguation`] which
    /// uses wikitext and gets exact article titles from `[[wiki links]]`.
    pub fn disambiguation_options(&self) -> Vec<DisambigOption> {
        let page_title = self.title.trim();
        self.content
            .lines()
            .map(str::trim)
            .filter(|l| {
                !l.is_empty()
                    && *l != page_title
                    && !l.ends_with("may refer to:")
                    && !l.ends_with("may refer to")
                    && !l.starts_with('#')
                    && !l.starts_with("==")
            })
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, ", ").collect();
                let (title, description) =
                    if parts[0].trim().eq_ignore_ascii_case(page_title) && parts.len() > 1 {
                        // Same name — description is the differentiator, use it as title hint
                        (
                            format!(
                                "{} {}",
                                page_title,
                                parts[1]
                                    .split_whitespace()
                                    .take(4)
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            ),
                            parts[1].to_string(),
                        )
                    } else {
                        (
                            parts[0].trim().to_string(),
                            parts.get(1).copied().unwrap_or("").to_string(),
                        )
                    };
                DisambigOption {
                    label: line.to_string(),
                    title,
                    description,
                }
            })
            .collect()
    }
}

/// A single option on a disambiguation page.
#[derive(Debug, Clone)]
pub struct DisambigOption {
    /// Full description line from the disambiguation page.
    pub label: String,
    /// Best-guess Wikipedia article title to fetch for this option.
    pub title: String,
    /// The description part (after the title in the label), used for display.
    pub description: String,
}

// ─── Client ───────────────────────────────────────────────────────────────────

/// Async Wikipedia API client.
///
/// # Example
/// ```rust,no_run
/// use wiky::{WikiClient, Config};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let config = Config::load()?;
///     let client = WikiClient::new(config)?;
///     let results = client.search("Rust language", 5).await?;
///     println!("{} results", results.len());
///     Ok(())
/// }
/// ```
pub struct WikiClient {
    http: Client,
    pub(crate) config: Config,
}

impl WikiClient {
    /// Create a new `WikiClient`.
    pub fn new(config: Config) -> Result<Self, WikiError> {
        let http = Client::builder()
            .user_agent(concat!(
                "wiky/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/cumulus13/wiky)"
            ))
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { http, config })
    }

    /// Search Wikipedia for `query`, returning up to `limit` results.
    pub async fn search(&self, query: &str, limit: u8) -> Result<Vec<SearchResult>, WikiError> {
        let url = self.config.api_url();
        let lim = limit.to_string();
        let resp = self
            .http
            .get(&url)
            .query(&[
                ("action", "query"),
                ("list", "search"),
                ("srsearch", query),
                ("srlimit", &lim),
                ("srprop", "snippet|wordcount"),
                ("format", "json"),
                ("formatversion", "2"),
                ("utf8", "1"),
            ])
            .send()
            .await?;

        let body: ApiQuery<SearchBody> = resp.json().await?;

        if let Some(err) = body.error {
            return Err(WikiError::Api(err.info));
        }

        let results = body
            .query
            .map(|q| {
                q.search
                    .into_iter()
                    .map(|i| SearchResult {
                        title: i.title,
                        snippet: strip_html(&i.snippet),
                        wordcount: i.wordcount,
                        pageid: i.pageid,
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// Fetch a full article by title as plain text with section headings.
    pub async fn fetch_article(&self, title: &str) -> Result<Article, WikiError> {
        let url = self.config.api_url();
        let lang = self.config.language.clone();

        let resp = self
            .http
            .get(&url)
            .query(&[
                ("action", "query"),
                ("titles", title),
                ("prop", "extracts|categories"),
                ("explaintext", "1"),
                ("exsectionformat", "wiki"), // section headings as == H ==
                ("cllimit", "50"),
                ("format", "json"),
                ("formatversion", "2"), // pages is an ARRAY
                ("utf8", "1"),
                ("redirects", "1"), // follow redirects automatically
            ])
            .send()
            .await?;

        let body: ApiQuery<ExtractBody> = resp.json().await?;

        if let Some(err) = body.error {
            return Err(WikiError::Api(err.info));
        }

        let page = body
            .query
            .and_then(|q| q.pages.into_iter().next())
            .ok_or_else(|| WikiError::NotFound(title.to_string()))?;

        if page.missing {
            return Err(WikiError::NotFound(title.to_string()));
        }

        let categories: Vec<String> = page
            .categories
            .into_iter()
            .map(|c| c.title.trim_start_matches("Category:").to_string())
            .collect();

        let article_url = format!(
            "https://{}.wikipedia.org/wiki/{}",
            lang,
            urlencode(&page.title)
        );

        let content = wiki_sections_to_markdown(&page.extract);

        let summary: String = content
            .lines()
            .take_while(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .chars()
            .take(800)
            .collect();

        Ok(Article {
            title: page.title,
            content,
            url: article_url,
            lang,
            pageid: page.pageid,
            categories,
            summary,
        })
    }

    /// Fetch only the intro/summary of an article.
    pub async fn fetch_summary(&self, title: &str) -> Result<Article, WikiError> {
        let url = self.config.api_url();
        let lang = self.config.language.clone();

        let resp = self
            .http
            .get(&url)
            .query(&[
                ("action", "query"),
                ("titles", title),
                ("prop", "extracts"),
                ("exintro", "1"),
                ("explaintext", "1"),
                ("format", "json"),
                ("formatversion", "2"),
                ("utf8", "1"),
                ("redirects", "1"),
            ])
            .send()
            .await?;

        let body: ApiQuery<ExtractBody> = resp.json().await?;

        if let Some(err) = body.error {
            return Err(WikiError::Api(err.info));
        }

        let page = body
            .query
            .and_then(|q| q.pages.into_iter().next())
            .ok_or_else(|| WikiError::NotFound(title.to_string()))?;

        if page.missing {
            return Err(WikiError::NotFound(title.to_string()));
        }

        let article_url = format!(
            "https://{}.wikipedia.org/wiki/{}",
            lang,
            urlencode(&page.title)
        );

        let summary = page.extract.trim().to_string();

        Ok(Article {
            title: page.title.clone(),
            summary: summary.clone(),
            content: summary,
            url: article_url,
            lang,
            pageid: page.pageid,
            categories: vec![],
        })
    }

    /// Fetch article by page ID.
    pub async fn fetch_by_id(&self, pageid: u64) -> Result<Article, WikiError> {
        let url = self.config.api_url();

        #[derive(Deserialize)]
        struct IdBody {
            pages: Vec<IdPage>,
        }
        #[derive(Deserialize)]
        struct IdPage {
            title: String,
        }

        let resp = self
            .http
            .get(&url)
            .query(&[
                ("action", "query"),
                ("pageids", &pageid.to_string()),
                ("format", "json"),
                ("formatversion", "2"),
            ])
            .send()
            .await?;

        let body: ApiQuery<IdBody> = resp.json().await?;
        let title = body
            .query
            .and_then(|q| q.pages.into_iter().next())
            .map(|p| p.title)
            .ok_or_else(|| WikiError::NotFound(pageid.to_string()))?;

        self.fetch_article(&title).await
    }

    /// Fetch exact disambiguation options by parsing wikitext `[[links]]`.
    ///
    /// Wikipedia disambiguation pages use wikitext like:
    /// ```text
    /// *Michael Orlando, band member of [[Vampires Everywhere!]]
    /// *Michael Orlando, acting director of the [[National Counterintelligence and Security Center]]
    /// ```
    /// This extracts the `[[ArticleTitle]]` or `[[ArticleTitle|display]]` targets
    /// directly — giving us the exact Wikipedia page to navigate to.
    pub async fn fetch_disambiguation(
        &self,
        article: &Article,
    ) -> Result<Vec<DisambigOption>, WikiError> {
        let wikitext = self.fetch_wikitext(&article.title).await?;
        let opts = parse_wikitext_disambig(&wikitext, &article.title);
        // Fall back to plain-text parsing if wikitext gave nothing
        if opts.is_empty() {
            Ok(article.disambiguation_options())
        } else {
            Ok(opts)
        }
    }

    /// Fetch the raw wikitext of a page (used for disambiguation link extraction). (used for disambiguation link extraction).
    pub(crate) async fn fetch_wikitext(&self, title: &str) -> Result<String, WikiError> {
        #[derive(Deserialize, Debug)]
        struct RevBody {
            pages: Vec<RevPage>,
        }
        #[derive(Deserialize, Debug)]
        struct RevPage {
            #[serde(default)]
            revisions: Vec<Revision>,
        }
        #[derive(Deserialize, Debug)]
        struct Revision {
            #[serde(rename = "slots")]
            slots: Slots,
        }
        #[derive(Deserialize, Debug)]
        struct Slots {
            main: SlotContent,
        }
        #[derive(Deserialize, Debug)]
        struct SlotContent {
            #[serde(rename = "content", default)]
            content: String,
        }

        let resp = self
            .http
            .get(self.config.api_url())
            .query(&[
                ("action", "query"),
                ("titles", title),
                ("prop", "revisions"),
                ("rvprop", "content"),
                ("rvslots", "main"),
                ("format", "json"),
                ("formatversion", "2"),
                ("redirects", "1"),
            ])
            .send()
            .await?;

        let body: ApiQuery<RevBody> = resp.json().await?;
        Ok(body
            .query
            .and_then(|q| q.pages.into_iter().next())
            .and_then(|p| p.revisions.into_iter().next())
            .map(|r| r.slots.main.content)
            .unwrap_or_default())
    }

    /// Build the Wikipedia URL for a title (no network call).
    pub fn article_url(&self, title: &str) -> String {
        format!(
            "https://{}.wikipedia.org/wiki/{}",
            self.config.language,
            urlencode(title)
        )
    }
}

// ─── Text helpers ─────────────────────────────────────────────────────────────

/// Convert `== Section ==` wiki-style headings → `## Section` renderer markers.
pub fn wiki_sections_to_markdown(text: &str) -> String {
    use regex::Regex;
    // Process deepest first to avoid double-matching
    let re4 = Regex::new(r"(?m)^====\s*(.+?)\s*====$").unwrap();
    let re3 = Regex::new(r"(?m)^===\s*(.+?)\s*===$").unwrap();
    let re2 = Regex::new(r"(?m)^==\s*(.+?)\s*==$").unwrap();
    let re_blank = Regex::new(r"\n{3,}").unwrap();

    let t = re4.replace_all(text, "#### $1");
    let t = re3.replace_all(&t, "### $1");
    let t = re2.replace_all(&t, "## $1");
    re_blank.replace_all(&t, "\n\n").trim().to_string()
}

/// Strip HTML tags and decode entities from a string.
pub fn strip_html(html: &str) -> String {
    let re = regex::Regex::new(r"<[^>]+>").unwrap();
    let out = re.replace_all(html, "");
    decode_entities(&out)
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
        .replace("&hellip;", "…")
        .replace("&laquo;", "«")
        .replace("&raquo;", "»")
}

fn urlencode(title: &str) -> String {
    title.replace(' ', "_")
}

// ─── Wikitext disambiguation parser ──────────────────────────────────────────

/// Parse wikitext list items to extract `[[ArticleTitle]]` link targets.
///
/// Handles these wikitext patterns on bullet lines (`*`):
/// - `[[Target]]`                     → title = "Target"
/// - `[[Target|display text]]`        → title = "Target"
/// - `[[Target]], description`        → title = "Target"
///
/// The full line (minus the leading `*`) becomes the label, with `[[...]]`
/// brackets removed so it reads naturally.
pub fn parse_wikitext_disambig(wikitext: &str, page_title: &str) -> Vec<DisambigOption> {
    use regex::Regex;

    // Matches [[Target]] or [[Target|Label]] anywhere in a string
    let re_link = Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]*)?\]\]").unwrap();
    // Strip all wikitext markup for the label
    let re_strip = Regex::new(r"\[\[[^\]]*\]\]|\{\{[^}]*\}\}|'{2,}").unwrap();

    let mut opts = Vec::new();

    for line in wikitext.lines() {
        let line = line.trim();
        // Only list items (bullet points)
        if !line.starts_with('*') || line.starts_with("**") {
            continue;
        }
        let body = line.trim_start_matches('*').trim();

        // Skip meta-lines like "See also", template calls, etc.
        if body.starts_with("{{") || body.is_empty() {
            continue;
        }

        // Extract the first [[link]] target — this is the article to navigate to
        if let Some(cap) = re_link.captures(body) {
            let link_target = cap[1].trim().to_string();

            // Build a clean human-readable label (strip wikitext markup)
            let label = re_strip.replace_all(body, |c: &regex::Captures| {
                // For [[T|Display]] keep Display; for [[T]] keep T
                let inner = &c[0];
                let content = inner.trim_start_matches('[').trim_end_matches(']');
                if let Some(pipe) = content.find('|') {
                    content[pipe + 1..].to_string()
                } else {
                    content.to_string()
                }
            });
            let label = label.trim().to_string();

            // Description = everything after the first comma in the label
            let description = label.split_once(", ").map(|x| x.1).unwrap_or("").to_string();

            // Skip if the link just points back to the disambiguation page itself
            if link_target.eq_ignore_ascii_case(page_title) {
                continue;
            }

            opts.push(DisambigOption {
                label,
                title: link_target,
                description,
            });
        }
    }

    opts
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_html_tags_and_entities() {
        assert_eq!(strip_html("<b>hello</b> &amp; world"), "hello & world");
        assert_eq!(strip_html("&#039;quoted&#039;"), "'quoted'");
        assert_eq!(strip_html("AT&amp;T"), "AT&T");
    }

    #[test]
    fn parse_wikitext_disambig_extracts_links() {
        let wikitext = r#"
'''Michael Orlando''' may refer to:
*Michael Orlando, band member of [[Vampires Everywhere!]]
*Michael Orlando, acting director of the [[National Counterintelligence and Security Center]]
{{disambig}}
"#;
        let opts = parse_wikitext_disambig(wikitext, "Michael Orlando");
        assert_eq!(opts.len(), 2, "expected 2 options, got: {:?}", opts);
        assert_eq!(opts[0].title, "Vampires Everywhere!");
        assert_eq!(
            opts[1].title,
            "National Counterintelligence and Security Center"
        );
    }

    #[test]
    fn parse_wikitext_disambig_pipe_links() {
        let wikitext = "* [[Some Article|display text here]], extra info";
        let opts = parse_wikitext_disambig(wikitext, "Other");
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0].title, "Some Article");
    }

    #[test]
    fn parse_wikitext_disambig_skips_self_links() {
        let wikitext = "* [[Michael Orlando]] disambiguation
* [[Vampires Everywhere!]]";
        let opts = parse_wikitext_disambig(wikitext, "Michael Orlando");
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0].title, "Vampires Everywhere!");
    }

    #[test]
    fn wiki_h2_converted() {
        let out = wiki_sections_to_markdown("Intro\n\n== History ==\n\nText.");
        assert!(out.contains("## History"), "got: {out}");
        assert!(!out.contains("=="), "should not contain raw == : {out}");
    }

    #[test]
    fn wiki_h3_converted() {
        let out = wiki_sections_to_markdown("=== Sub ===\nContent");
        assert_eq!(out.trim(), "### Sub\nContent");
    }

    #[test]
    fn wiki_h4_not_double_matched() {
        let out = wiki_sections_to_markdown("==== Deep ====");
        assert_eq!(out.trim(), "#### Deep");
    }

    #[test]
    fn blank_lines_collapsed() {
        let out = wiki_sections_to_markdown("a\n\n\n\nb");
        assert!(out.contains("a\n\nb"), "got: {out}");
    }
}
