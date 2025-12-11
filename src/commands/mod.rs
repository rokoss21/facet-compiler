//! # CLI Commands Module
//!
//! This module contains all CLI command handlers for the FACET compiler.
//! Each command is implemented in its own module for better organization.

use clap::Parser;
use std::path::PathBuf;
use governor::{RateLimiter, state::InMemoryState, clock::DefaultClock, state::direct::NotKeyed};

pub type DefaultRateLimiter = RateLimiter<NotKeyed, InMemoryState, DefaultClock>;

pub mod build;
pub mod run;
pub mod test;
pub mod inspect;
pub mod codegen;

/// Main CLI structure using clap for argument parsing
#[derive(Parser)]
#[command(name = "fct")]
#[command(about = "FACET v2.0 Compiler - Deterministic AI Agent Compiler", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable progress bars
    #[arg(long)]
    pub no_progress: bool,

    /// Enable JSON logging
    #[arg(long)]
    pub json_logs: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands
#[derive(clap::Subcommand)]
pub enum Commands {
    /// Parse, resolve, and validate a FACET document
    Build {
        /// Input FACET file path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Inspect the parsed AST structure
    Inspect {
        /// Input FACET file path
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Run full pipeline: parse, resolve, validate, compute, and render
    Run {
        /// Input FACET file path
        #[arg(short, long)]
        input: PathBuf,

        /// Token budget for context window (default: 4096)
        #[arg(short, long, default_value_t = 4096)]
        budget: usize,

        /// Execution context budget for R-DAG (default: 10000)
        #[arg(short = 'c', long, default_value_t = 10000)]
        context_budget: usize,

        /// Output format: json or pretty
        #[arg(short, long, default_value = "json")]
        format: String,
    },

    /// Run @test blocks
    Test {
        /// Input FACET file path
        #[arg(short, long)]
        input: PathBuf,

        /// Filter tests by name pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Output format: summary, verbose, json
        #[arg(long, default_value = "summary")]
        output: String,

        /// Token budget for test execution
        #[arg(long, default_value_t = 4096)]
        budget: usize,

        /// Gas limit for test execution
        #[arg(long, default_value_t = 10000)]
        gas_limit: usize,
    },

    /// Generate SDK from FACET interfaces
    Codegen {
        /// Input FACET file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory for generated code
        #[arg(short, long)]
        output: PathBuf,

        /// Target language: typescript, python, rust
        #[arg(short, long, default_value = "typescript")]
        language: String,

        /// SDK name (default: derived from input file)
        #[arg(long)]
        name: Option<String>,
    },
}