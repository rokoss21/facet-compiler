//! # Build Command
//!
//! This module implements the build command for the FACET compiler.
//! The build command parses, resolves, validates, and compiles FACET documents.

use anyhow::{Context, Result};
use console::style;
use fct_parser::parse_document;
use fct_resolver::{Resolver, ResolverConfig};
use fct_validator::TypeChecker;
use std::fs;
use std::path::PathBuf;
use tracing::info;

// Icon constants
const GEAR: console::Emoji = console::Emoji("⚙️ ", "[BUILD] ");
const INFO: console::Emoji = console::Emoji("ℹ️ ", "[INFO] ");

/// Build command handler
pub fn execute_build(
    input: PathBuf,
    verbose: bool,
    _no_progress: bool,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        eprintln!(
            "{}",
            style("Rate limit exceeded. Please wait before running another command.").red()
        );
        std::process::exit(1);
    }

    info!("Building FACET document: {:?}", input);
    println!("{} Building {:?}", GEAR, input);

    if verbose {
        println!("{} Verbose mode enabled", INFO);
    }

    let source = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read input file: {:?}", input))?;

    let parsed = parse_document(&source).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let base_dir = input
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(std::env::current_dir()?);
    let mut resolver = Resolver::new(ResolverConfig {
        allowed_roots: vec![base_dir.clone()],
        base_dir,
    });
    let resolved = resolver
        .resolve(parsed)
        .map_err(|e| anyhow::anyhow!("Resolution error: {}", e))?;

    let mut checker = TypeChecker::new();
    checker
        .validate(&resolved)
        .map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;

    let block_count = resolved.blocks.len();
    println!(
        "{} Parsed + resolved + validated ({} block(s))",
        INFO, block_count
    );
    println!("{}", style("✓ Build completed successfully!").green());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use governor::{Quota, RateLimiter};
    use nonzero_ext::nonzero;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn execute_build_succeeds_for_valid_file() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-build-ok-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");

        let input_path = test_dir.join("input.facet");
        fs::write(
            &input_path,
            "@system\n  content: \"You are helpful.\"\n@user\n  content: \"hello\"\n",
        )
        .expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        execute_build(input_path, false, true, &limiter).expect("build should succeed");

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execute_build_fails_for_invalid_syntax() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-build-err-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");

        let input_path = test_dir.join("input.facet");
        fs::write(&input_path, "@system\n content: \"bad indent\"\n").expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        let err = execute_build(input_path, false, true, &limiter).unwrap_err();
        assert!(err.to_string().contains("Parse error"));

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execute_build_rejects_import_outside_allowed_root_with_f601() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root_dir = std::env::temp_dir().join(format!("facet-build-sandbox-root-{}", nonce));
        let outside_dir = std::env::temp_dir().join(format!("facet-build-sandbox-out-{}", nonce));
        fs::create_dir_all(&root_dir).expect("create root dir");
        fs::create_dir_all(&outside_dir).expect("create outside dir");

        let outside_path = outside_dir.join("outside.facet");
        fs::write(&outside_path, "@vars\n  x: \"outside\"\n").expect("write outside file");

        let input_path = root_dir.join("input.facet");
        fs::write(
            &input_path,
            format!("@import \"{}\"\n", outside_path.display()),
        )
        .expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        let err = execute_build(input_path, false, true, &limiter)
            .expect_err("build must reject import outside allowed root");
        let text = err.to_string();
        assert!(text.contains("Resolution error"), "unexpected error: {text}");
        assert!(text.contains("F601"), "expected F601, got: {text}");

        let _ = fs::remove_dir_all(root_dir);
        let _ = fs::remove_dir_all(outside_dir);
    }
}
