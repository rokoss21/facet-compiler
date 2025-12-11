//! # Run Command
//!
//! This module implements the run command for the FACET compiler.
//! The run command executes the full pipeline: parse, resolve, validate, compute, and render.

use anyhow::Result;
use console::style;
use governor::RateLimiter;
use tracing::info;

/// Run command handler
pub fn execute_run(
    input: std::path::PathBuf,
    budget: usize,
    context_budget: usize,
    format: String,
    _no_progress: bool,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        eprintln!("{}", style("L Rate limit exceeded. Please wait before running another command.").red());
        std::process::exit(1);
    }

    info!("Starting full pipeline for file: {:?}", input);
    info!("Budget: {}, Context budget: {}", budget, context_budget);

    // TODO: Implement full pipeline execution
    // For now, just indicate that the command was processed
    println!("{}", style("🏃 Run command processed successfully!").green());
    println!("File: {:?}", input);
    println!("Budget: {}", budget);
    println!("Context budget: {}", context_budget);
    println!("Format: {}", format);

    Ok(())
}