# 📖 wiky

> Beautiful Wikipedia in your terminal — with hex colors, emoji, Markdown rendering, and full theme customisation.

[![Crates.io](https://img.shields.io/crates/v/wiky)](https://crates.io/crates/wiky)
[![docs.rs](https://img.shields.io/docsrs/wiky)](https://docs.rs/wiky)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

---

## ✨ Features

| Feature | Detail |
|---|---|
| 🎨 **Hex color theming** | Full 24-bit truecolor — every UI element configurable |
| 😀 **Emoji decorations** | Headings, bullets, links and results all get emoji prefixes |
| 📝 **Markdown rendering** | Article headings, lists, bold, italic, code rendered beautifully |
| 🌍 **Multi-language** | Any Wikipedia language (`en`, `id`, `de`, `ja`, …) |
| ⚙️  **TOML config file** | Persistent per-user settings, auto-placed by OS convention |
| 📄 **Pager support** | Pipe long articles through `less` automatically |
| 🌐 **Open in browser** | Jump straight to the Wikipedia page |
| 📦 **Library + binary** | Use as a Rust library or standalone CLI tool |
| 🔄 **Interactive mode** | Search → pick a result → read, all in one command |

---

## 📥 Installation

### From crates.io

```bash
cargo install wiky
```

### From source

```bash
git clone https://github.com/cumulus13/wiky
cd wiky
cargo install --path .
```

---

## 🚀 Usage

### Search

```bash
wiky search "Rust programming language"
wiky search "black hole" -n 5        # limit to 5 results
```

### Read an article

```bash
wiky get "Rust (programming language)"
wiky get "Linux kernel" --section "History"
```

### Quick summary

```bash
wiky summary "Python (programming language)"
```

### Interactive mode (search → pick → read)

```bash
wiky i "quantum computing"
```

### Open in browser

```bash
wiky open "Eiffel Tower"
```

### Categories

```bash
wiky categories "Albert Einstein"
```

### Global flags

```bash
wiky --lang id search "Bahasa Jawa"   # Indonesian Wikipedia
wiky --theme nord get "Tokyo"          # Nord color theme
wiky --no-color search "moon"          # plain output
wiky --no-emoji get "Mars"             # no emoji
wiky --pager get "History of the Internet"  # pipe through less
wiky --width 120 get "Climate change"  # set terminal width
wiky --raw search "test"               # raw plain text
```

---

## ⚙️ Configuration

### View / edit config

```bash
wiky config show          # show current settings
wiky config path          # print config file location
wiky config set lang id   # switch to Indonesian Wikipedia
wiky config set theme nord
wiky config set pager true
wiky config set results_count 5
wiky config reset         # restore defaults
```

### Config file (TOML)

The config file is automatically created at:

| Platform | Path |
|---|---|
| Linux | `~/.config/wiky/config.toml` |
| macOS | `~/Library/Application Support/wiky/config.toml` |
| Windows | `%APPDATA%\wiky\config.toml` |

```toml
language       = "en"        # Wikipedia language code
theme          = "dark"      # dark | light | solarized | nord | dracula | custom
width          = 0           # 0 = auto-detect terminal width
pager          = false       # pipe through $PAGER / less
results_count  = 10
open_urls      = false
show_image_alt = false

# Custom theme — used when theme = "custom"
[custom_theme]
title          = "#61AFEF"
heading        = "#E5C07B"
subheading     = "#98C379"
body           = "#ABB2BF"
link           = "#56B6C2"
bold           = "#E06C75"
italic         = "#C678DD"
code           = "#282C34"
result_title   = "#61AFEF"
result_snippet = "#ABB2BF"
result_index   = "#5C6370"
separator      = "#3E4451"
error          = "#E06C75"
success        = "#98C379"
dim            = "#5C6370"
```

### Set custom theme colors inline

```bash
wiky config set custom_theme.title "#FF6600"
wiky config set theme custom
```

---

## 🎨 Themes

```bash
wiky themes    # list all themes
```

| Name | Description |
|---|---|
| `dark`      | One Dark (default) |
| `light`     | Clean light terminal |
| `solarized` | Solarized Dark |
| `nord`      | Nord |
| `dracula`   | Dracula |
| `custom`    | Your own hex colors from `[custom_theme]` |

---

## 📚 Library Usage

Add to `Cargo.toml`:

```toml
[dependencies]
wiky = "0.1"
tokio = { version = "1", features = ["full"] }
```

### Basic example

```rust
use wiky::{WikiClient, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let client = WikiClient::new(config)?;

    // Search
    let results = client.search("Rust programming language", 5).await?;
    for r in &results {
        println!("{}: {}", r.title, r.snippet);
    }

    // Full article
    let article = client.fetch_article("Rust (programming language)").await?;
    println!("Title: {}", article.title);
    println!("URL:   {}", article.url);
    println!("{}", &article.content[..500]);

    Ok(())
}
```

### Custom theme

```rust
use wiky::{Config, Renderer, Theme};

let mut config = Config::default();
config.theme = "custom".into();
config.custom_theme.title   = "#FF6600".into();
config.custom_theme.heading = "#00BFFF".into();

let theme    = config.active_theme();
let renderer = Renderer::new(theme, 100, true);
renderer.print_message("Hello from wiky! 📖");
```

### Hex color utilities

```rust
use wiky::theme::{hex_to_rgb, colorize_hex};

let (r, g, b) = hex_to_rgb("#61AFEF").unwrap();    // (97, 175, 239)
let colored   = colorize_hex("Hello!", "#E06C75");  // red-ish truecolor string
println!("{colored}");
```

---

## 🏗️ Architecture

```
wiky/
├── src/
│   ├── lib.rs      — public API, re-exports
│   ├── main.rs     — CLI binary (clap subcommands)
│   ├── client.rs   — Wikipedia API client (async reqwest)
│   ├── config.rs   — TOML config via confy
│   ├── render.rs   — terminal renderer (colors + emoji + wrapping)
│   ├── theme.rs    — hex color parsing, Theme struct, built-ins
│   └── error.rs    — WikiError enum (thiserror)
└── tests/
    └── integration_tests.rs
```

---

## 🛠️ Development

```bash
# Build
cargo build

# Run
cargo run -- search "Linux kernel"

# Tests
cargo test

# Lint
cargo clippy -- -D warnings

# Docs
cargo doc --open

# Release build
cargo build --release
```

---

## 📜 License

MIT © 2024 Hadi Cahyadi — [cumulus13@gmail.com](mailto:cumulus13@gmail.com)

---

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)

---

## 🔗 Links

- **Homepage / Source:** https://github.com/cumulus13/wiky
- **Wikipedia API:** https://www.mediawiki.org/wiki/API:Main_page
- **confy** (config crate): https://crates.io/crates/confy
- **colored** (terminal colors): https://crates.io/crates/colored
