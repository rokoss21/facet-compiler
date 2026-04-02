//! # FACET v2.1.3 Compiler Main Entry Point
//!
//! This is the main entry point for the FACET compiler. It provides a CLI interface
//! for building, running, and testing FACET documents.

mod commands;

use clap::Parser;
use commands::{Cli, Commands, DefaultRateLimiter};
use console::style;
use governor::{Quota, RateLimiter};
use nonzero_ext::nonzero;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let cli = Cli::parse();

    // Setup logging
    setup_logging(&cli);

    // Setup rate limiting
    let rate_limiter = setup_rate_limiter();

    // Execute command
    match cli.command {
        Commands::Build { input } => {
            commands::build::execute_build(input, cli.verbose, cli.no_progress, &rate_limiter)
        }
        Commands::Inspect {
            input,
            ast,
            dag,
            layout,
            policy,
            budget,
            pure,
            exec,
        } => commands::inspect::execute_inspect(
            input,
            ast,
            dag,
            layout,
            policy,
            budget,
            pure,
            exec,
            &rate_limiter,
        ),
        Commands::Run {
            input,
            runtime_input,
            budget,
            context_budget,
            format,
            pure,
            exec,
        } => commands::run::execute_run(
            input,
            runtime_input,
            budget,
            context_budget,
            format,
            pure,
            exec,
            cli.no_progress,
            &rate_limiter,
        ),
        Commands::Test {
            input,
            filter,
            output,
            budget,
            gas_limit,
            pure,
            exec,
        } => commands::test::execute_test(
            input,
            filter,
            output,
            budget,
            gas_limit,
            pure,
            exec,
            &rate_limiter,
        ),
        Commands::Codegen {
            input,
            output,
            language,
            name,
        } => commands::codegen::execute_codegen(input, output, language, name, &rate_limiter),
    }
}

/// Setup logging configuration based on CLI flags
fn setup_logging(cli: &Cli) {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .finish();

    if cli.json_logs {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        subscriber.init();
    }

    if cli.verbose {
        tracing::subscriber::set_global_default(
            tracing_subscriber::fmt()
                .with_max_level(tracing::Level::DEBUG)
                .finish(),
        )
        .unwrap();
    }
}

/// Setup rate limiting for CLI commands
fn setup_rate_limiter() -> DefaultRateLimiter {
    RateLimiter::direct(
        Quota::per_second(nonzero!(10u32)), // Allow 10 requests per second
    )
}
