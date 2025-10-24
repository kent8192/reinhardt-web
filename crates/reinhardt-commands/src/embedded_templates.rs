//! Embedded template files using rust-embed
//!
//! This module embeds all template files into the binary at compile time.

use rust_embed::RustEmbed;

/// Embedded template directory
#[derive(RustEmbed)]
#[folder = "templates/"]
pub struct TemplateAssets;
