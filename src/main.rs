//! wiky — Wikipedia CLI
//!
//! Author:   Hadi Cahyadi <cumulus13@gmail.com>
//! Homepage: https://github.com/cumulus13/wiky

use std::process;

use clap::{ArgAction, Args, Parser, Subcommand};
use colored::control::set_override as set_color_override;
use indicatif::{ProgressBar, ProgressStyle};
use minus::Pager;

use wiky::{Config, Renderer, WikiClient, WikiError};

// ─── CLI ─────────────────────────────────────────────────────────────────────

/// 📖 wiky — Beautiful Wikipedia in your terminal.
///
/// Supports hex colors, emoji, Markdown rendering, and customizable themes.
#[derive(Parser)]
#[command(
    name    = "wiky",
    version = env!("CARGO_PKG_VERSION"),
    author  = "Hadi Cahyadi <cumulus13@gmail.com>",
    about   = "📖 Beautiful Wikipedia in your terminal",
    long_about = "\
wiky searches and displays Wikipedia articles with colors, emoji and Markdown.\n\
\n\
EXAMPLES:\n\
  wiky search \"Rust programming language\"\n\
  wiky get \"Rust (programming language)\"\n\
  wiky get \"Linux kernel\" --section History\n\
  wiky summary \"Python (programming language)\"\n\
  wiky i \"coldplay\"               # interactive: search → pick → read\n\
  wiky open \"Eiffel Tower\"\n\
  wiky --pager get \"History of the Internet\"\n\
  wiky --theme nord --lang de search \"Berlin\"\n\
  wiky config show\n\
  wiky config set theme dracula\n\
  wiky config set custom_theme.title \"#FF6600\"\n\
  wiky themes\n\
",
    propagate_version = true,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Wikipedia language code, e.g. en, id, de, ja (overrides config).
    #[arg(short = 'l', long, global = true, env = "wiky_LANG")]
    lang: Option<String>,

    /// Color theme: dark | light | solarized | nord | dracula | custom.
    #[arg(short = 't', long, global = true, env = "wiky_THEME")]
    theme: Option<String>,

    /// Terminal column width override (0 = auto-detect).
    #[arg(short = 'w', long, global = true)]
    width: Option<u16>,

    /// Disable color output.
    #[arg(long, global = true, env = "NO_COLOR", action = ArgAction::SetTrue)]
    no_color: bool,

    /// Disable emoji decorations.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    no_emoji: bool,

    /// Page long output (cross-platform, works on Linux/macOS/Windows).
    #[arg(short = 'p', long, global = true, action = ArgAction::SetTrue)]
    pager: bool,

    /// Plain text output: no colors, no emoji.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    raw: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// 🔍 Search Wikipedia for a query.
    Search(SearchCmd),
    /// 📄 Fetch and display a full Wikipedia article.
    Get(GetCmd),
    /// 💡 Show only the introduction/summary of an article.
    Summary(SummaryCmd),
    /// 🌐 Open an article in your default web browser.
    Open(OpenCmd),
    /// 🏷️  Show Wikipedia categories for an article.
    Categories(CategoriesCmd),
    /// ⚙️  Manage wiky configuration.
    Config(ConfigCmd),
    /// 🎨 List available color themes.
    Themes,
    /// 🔎 Interactive: search, pick a result, then read it.
    #[command(name = "i")]
    Interactive(SearchCmd),
}

#[derive(Args)]
struct SearchCmd {
    /// Search query.
    query: String,
    /// Maximum number of results.
    #[arg(short = 'n', long, default_value = "10")]
    results: u8,
}

#[derive(Args)]
struct GetCmd {
    /// Exact Wikipedia article title.
    title: String,
    /// Filter to sections whose heading contains this keyword.
    #[arg(short, long)]
    section: Option<String>,
}

#[derive(Args)]
struct SummaryCmd {
    /// Article title.
    title: String,
}

#[derive(Args)]
struct OpenCmd {
    /// Article title.
    title: String,
}

#[derive(Args)]
struct CategoriesCmd {
    /// Article title.
    title: String,
}

#[derive(Args)]
struct ConfigCmd {
    #[command(subcommand)]
    action: ConfigAction,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration and config file path.
    Show,
    /// Set a key: `wiky config set <key> <value>`.
    Set { key: String, value: String },
    /// Reset all settings to defaults.
    Reset,
    /// Print the config file path.
    Path,
}

// ─── Entry point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Load config (never fatal — fall back to defaults)
    let mut config = Config::load().unwrap_or_else(|e| {
        eprintln!("warning: could not load config ({e}), using defaults");
        Config::default()
    });

    // CLI flags override config
    if let Some(l) = &cli.lang {
        config.language = l.clone();
    }
    if let Some(t) = &cli.theme {
        config.theme = t.clone();
    }
    if let Some(w) = cli.width {
        config.width = w;
    }
    if cli.pager {
        config.pager = true;
    }

    let no_color = cli.no_color || cli.raw;
    let no_emoji = cli.no_emoji || cli.raw;
    if no_color {
        set_color_override(false);
    }

    let theme = config.active_theme();
    let width = config.effective_width();
    let renderer = Renderer::new(theme, width, !no_emoji);

    let result = dispatch(cli.command, &mut config, &renderer).await;

    if let Err(e) = result {
        renderer.print_error(&e.to_string());
        process::exit(1);
    }
}

// ─── Pager helper ────────────────────────────────────────────────────────────

/// Feed a pre-rendered ANSI string into the `minus` cross-platform pager.
///
/// `minus` with `static_output` feature works on Linux, macOS and Windows —
/// it does NOT rely on `less` or any external binary. It renders the content
/// in a scrollable TUI inside the terminal process itself.
fn page_string(content: String) -> Result<(), WikiError> {
    let pager = Pager::new();
    pager
        .push_str(&content)
        .map_err(|e: minus::error::MinusError| WikiError::Other(e.to_string()))?;
    minus::page_all(pager).map_err(|e: minus::error::MinusError| WikiError::Other(e.to_string()))
}

// ─── Dispatch ────────────────────────────────────────────────────────────────

async fn dispatch(
    cmd: Commands,
    config: &mut Config,
    renderer: &Renderer,
) -> Result<(), WikiError> {
    match cmd {
        Commands::Search(c) => run_search(c, config, renderer).await,
        Commands::Get(c) => run_get(c, config, renderer).await,
        Commands::Summary(c) => run_summary(c, config, renderer).await,
        Commands::Open(c) => run_open(c, config, renderer).await,
        Commands::Categories(c) => run_categories(c, config, renderer).await,
        Commands::Config(c) => run_config(c, config, renderer).await,
        Commands::Themes => {
            renderer.render_themes_list();
            Ok(())
        }
        Commands::Interactive(c) => run_interactive(c, config, renderer).await,
    }
}

// ─── Command handlers ─────────────────────────────────────────────────────────

async fn run_search(cmd: SearchCmd, config: &Config, renderer: &Renderer) -> Result<(), WikiError> {
    let client = WikiClient::new(config.clone())?;
    let sp = make_spinner(&format!("Searching for \"{}\"…", cmd.query));
    let results = client.search(&cmd.query, cmd.results).await?;
    sp.finish_and_clear();

    if config.pager {
        page_string(renderer.search_to_string(&cmd.query, &results))?;
    } else {
        renderer.render_search_results(&cmd.query, &results);
    }
    Ok(())
}

async fn run_get(cmd: GetCmd, config: &Config, renderer: &Renderer) -> Result<(), WikiError> {
    let client = WikiClient::new(config.clone())?;
    let sp = make_spinner(&format!("Fetching \"{}\"…", cmd.title));
    let article = client.fetch_article(&cmd.title).await?;
    sp.finish_and_clear();

    let article = if let Some(kw) = &cmd.section {
        wiky::prelude::Article {
            content: filter_section(&article.content, kw),
            ..article
        }
    } else {
        article
    };

    fetch_and_display(article, &client, config, renderer).await
}

async fn run_summary(
    cmd: SummaryCmd,
    config: &Config,
    renderer: &Renderer,
) -> Result<(), WikiError> {
    let client = WikiClient::new(config.clone())?;
    let sp = make_spinner(&format!("Fetching summary for \"{}\"…", cmd.title));
    let article = client.fetch_summary(&cmd.title).await?;
    sp.finish_and_clear();

    if config.pager {
        page_string(renderer.summary_to_string(&article))?;
    } else {
        renderer.render_summary(&article);
    }
    Ok(())
}

async fn run_open(cmd: OpenCmd, config: &Config, renderer: &Renderer) -> Result<(), WikiError> {
    let client = WikiClient::new(config.clone())?;
    let url = client.article_url(&cmd.title);
    renderer.print_message(&format!("Opening: {url}"));
    open::that(&url).map_err(|e| WikiError::Other(e.to_string()))?;
    renderer.print_success("Opened in browser.");
    Ok(())
}

async fn run_categories(
    cmd: CategoriesCmd,
    config: &Config,
    renderer: &Renderer,
) -> Result<(), WikiError> {
    let client = WikiClient::new(config.clone())?;
    let sp = make_spinner(&format!("Fetching categories for \"{}\"…", cmd.title));
    let article = client.fetch_article(&cmd.title).await?;
    sp.finish_and_clear();

    let theme = config.active_theme();
    println!();
    println!(
        "{}",
        theme.title(format!("🏷️  Categories — {}", article.title))
    );
    println!("{}", theme.separator("─".repeat(60)));
    if article.categories.is_empty() {
        println!("{}", renderer.dim_str("  (none found)"));
    } else {
        for cat in &article.categories {
            println!("  {} {}", theme.result_index("•"), theme.body(cat));
        }
    }
    println!();
    Ok(())
}

async fn run_interactive(
    cmd: SearchCmd,
    config: &Config,
    renderer: &Renderer,
) -> Result<(), WikiError> {
    let client = WikiClient::new(config.clone())?;
    let sp = make_spinner(&format!("Searching for \"{}\"…", cmd.query));
    let results = client.search(&cmd.query, cmd.results).await?;
    sp.finish_and_clear();

    if results.is_empty() {
        renderer.print_error(&format!("No results for \"{}\"", cmd.query));
        return Ok(());
    }

    renderer.render_search_results(&cmd.query, &results);

    eprint!("  Enter result number to read (or 0 to quit): ");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let choice: usize = input.trim().parse().unwrap_or(0);
    if choice == 0 || choice > results.len() {
        return Ok(());
    }

    let title = &results[choice - 1].title;
    let sp = make_spinner(&format!("Fetching \"{}\"…", title));
    let article = client.fetch_article(title).await?;
    sp.finish_and_clear();

    fetch_and_display(article, &client, config, renderer).await
}

/// Fetch and display an article, handling disambiguation pages transparently.
///
/// If the article is a disambiguation page, a numbered sub-menu is shown so
/// the user can pick which meaning they want. Recurses (via Box::pin) until a
/// real article is rendered or the user enters 0 to quit.
fn fetch_and_display<'a>(
    article: wiky::prelude::Article,
    client: &'a WikiClient,
    config: &'a Config,
    renderer: &'a Renderer,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), WikiError>> + 'a>> {
    Box::pin(async move {
        let mut current = article;
        // Loop so we never recurse infinitely even through chains of disambig pages
        for _ in 0..10 {
            if !current.is_disambiguation() {
                return if config.pager {
                    page_string(renderer.article_to_string(&current))
                } else {
                    renderer.render_article(&current);
                    Ok(())
                };
            }

            let opts = client
                .fetch_disambiguation(&current)
                .await
                .unwrap_or_else(|_| current.disambiguation_options());
            if opts.is_empty() {
                // Fallback: render as-is (unusual empty disambig page)
                return if config.pager {
                    page_string(renderer.article_to_string(&current))
                } else {
                    renderer.render_article(&current);
                    Ok(())
                };
            }

            renderer.render_disambiguation(&current, &opts);

            eprint!("  Pick a number (or 0 to quit): ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            let choice: usize = input.trim().parse().unwrap_or(0);
            if choice == 0 || choice > opts.len() {
                return Ok(());
            }

            let opt = &opts[choice - 1];
            let sp = make_spinner(&format!("Fetching \"{}\"…", opt.title));

            match client.fetch_article(&opt.title).await {
                Ok(next) => {
                    sp.finish_and_clear();
                    // If we got back the exact same page, fall back to search
                    if next.title == current.title {
                        sp.finish_and_clear();
                        renderer.print_message(&format!(
                            "Tip: try `wiky search \"{}\"`  to find the exact article.",
                            opt.description
                        ));
                        return Ok(());
                    }
                    current = next;
                }
                Err(_) => {
                    sp.finish_and_clear();
                    // Title guess failed — suggest a search instead
                    renderer.print_error(&format!(
                        "Could not find an article for \"{}\". \
Try: wiky search \"{}\", {}\",",
                        opt.title, current.title, opt.description
                    ));
                    return Ok(());
                }
            }
        }
        renderer.print_error("Too many disambiguation levels — giving up.");
        Ok(())
    })
}

async fn run_config(
    cmd: ConfigCmd,
    config: &mut Config,
    renderer: &Renderer,
) -> Result<(), WikiError> {
    match cmd.action {
        ConfigAction::Show => {
            renderer.render_config_info(config);
        }
        ConfigAction::Set { key, value } => {
            apply_config_set(config, &key, &value)?;
            config.save()?;
            renderer.print_success(&format!("Set {key} = {value}"));
        }
        ConfigAction::Reset => {
            *config = Config::default();
            config.save()?;
            renderer.print_success("Configuration reset to defaults.");
        }
        ConfigAction::Path => match Config::path() {
            Some(p) => println!("{}", p.display()),
            None => renderer.print_error("Could not determine config path."),
        },
    }
    Ok(())
}

// ─── Config helpers ───────────────────────────────────────────────────────────

fn apply_config_set(config: &mut Config, key: &str, value: &str) -> Result<(), WikiError> {
    match key {
        "language" | "lang" => {
            config.language = value.into();
        }
        "theme" => {
            config.theme = value.into();
        }
        "open_urls" => {
            config.open_urls = parse_bool(value)?;
        }
        "pager" => {
            config.pager = parse_bool(value)?;
        }
        "show_image_alt" => {
            config.show_image_alt = parse_bool(value)?;
        }
        "width" => {
            config.width = value
                .parse::<u16>()
                .map_err(|_| WikiError::Other(format!("'{value}' is not a valid width")))?;
        }
        "results_count" => {
            config.results_count = value
                .parse::<u8>()
                .map_err(|_| WikiError::Other(format!("'{value}' is not a valid count")))?;
        }
        k if k.starts_with("custom_theme.") => {
            let field = k.trim_start_matches("custom_theme.");
            apply_theme_field(&mut config.custom_theme, field, value)?;
        }
        _ => return Err(WikiError::Other(format!("Unknown config key: '{key}'"))),
    }
    Ok(())
}

fn apply_theme_field(
    theme: &mut wiky::Theme,
    field: &str,
    value: &str,
) -> Result<(), WikiError> {
    wiky::theme::hex_to_rgb(value)?; // validates hex first
    let v = value.to_string();
    match field {
        "title" => theme.title = v,
        "heading" => theme.heading = v,
        "subheading" => theme.subheading = v,
        "body" => theme.body = v,
        "link" => theme.link = v,
        "bold" => theme.bold = v,
        "italic" => theme.italic = v,
        "code" => theme.code = v,
        "result_title" => theme.result_title = v,
        "result_snippet" => theme.result_snippet = v,
        "result_index" => theme.result_index = v,
        "separator" => theme.separator = v,
        "error" => theme.error = v,
        "success" => theme.success = v,
        "dim" => theme.dim = v,
        _ => return Err(WikiError::Other(format!("Unknown theme field: '{field}'"))),
    }
    Ok(())
}

fn parse_bool(s: &str) -> Result<bool, WikiError> {
    match s.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(WikiError::Other(format!(
            "'{s}' is not a boolean (use true/false)"
        ))),
    }
}

// ─── Utilities ────────────────────────────────────────────────────────────────

fn filter_section(content: &str, keyword: &str) -> String {
    let kw = keyword.to_lowercase();
    let mut out = String::new();
    let mut in_m = false;
    let mut buf = String::new();

    for line in content.lines() {
        if line.starts_with('#') {
            if in_m && !buf.is_empty() {
                out.push_str(&buf);
                buf.clear();
            }
            in_m = line.to_lowercase().contains(&kw);
        }
        if in_m {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    if in_m && !buf.is_empty() {
        out.push_str(&buf);
    }
    if out.is_empty() {
        content.to_string()
    } else {
        out
    }
}

fn make_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}
