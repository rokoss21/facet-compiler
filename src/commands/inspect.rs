//! # Inspect Command
//!
//! This module implements the inspect command for the FACET compiler.
//! The inspect command shows the parsed AST structure of FACET documents.

use anyhow::Result;
use console::style;
use governor::RateLimiter;
use std::fs;
use tracing::warn;

/// Inspect command handler
pub fn execute_inspect(
    input: std::path::PathBuf,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        warn!("Rate limit exceeded for inspect command");
        eprintln!("{}", style("L Rate limit exceeded. Please wait before running another command.").red());
        std::process::exit(1);
    }

    let content = fs::read_to_string(&input)?;
    let doc = fct_parser::parse_document(&content)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    println!("{:#?}", doc);
    Ok(())
}