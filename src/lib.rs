//! # wiky
//!
//! A beautiful, colorful Wikipedia CLI and library with full Markdown rendering,
//! emoji support, hex color theming, and a TOML-based config system.
//!
//! ## Library Usage
//!
//! ```rust,no_run
//! use wiky::{WikiClient, Config};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load()?;
//!     let client = WikiClient::new(config)?;
//!
//!     // Search Wikipedia
//!     let results = client.search("Rust programming language", 5).await?;
//!     for r in &results {
//!         println!("{}: {}", r.title, r.snippet);
//!     }
//!
//!     // Fetch and render a full article
//!     let article = client.fetch_article("Rust (programming language)").await?;
//!     println!("{}", article.title);
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod render;
pub mod theme;

pub use client::WikiClient;
pub use config::Config;
pub use error::WikiError;
pub use render::Renderer;
pub use theme::Theme;

/// Re-export core types for convenience.
pub mod prelude {
    pub use crate::client::{Article, DisambigOption, SearchResult, WikiClient};
    pub use crate::config::Config;
    pub use crate::error::WikiError;
    pub use crate::render::Renderer;
    pub use crate::theme::Theme;
}
