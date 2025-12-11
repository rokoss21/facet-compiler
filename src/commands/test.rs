//! # Test Command
//!
//! This module implements the test command for the FACET compiler.
//! The test command runs @test blocks in FACET documents.

use anyhow::{Result, Context};
use console::{style, Emoji};
use tracing::{info, error, debug};
use std::fs;
use std::path::Path;
use std::time::Instant;
use regex::Regex;

// Import FACET crates
use fct_parser::parse_document;
use fct_engine::{TestReporter, ReportFormat};

const TEST_EMOJI: Emoji = Emoji("🧪 ", "");
const PASS_EMOJI: Emoji = Emoji("✅ ", "");
const FAIL_EMOJI: Emoji = Emoji("❌ ", "");
const SKIP_EMOJI: Emoji = Emoji("⏭️ ", "");

/// Test command handler
pub fn execute_test(
    input: std::path::PathBuf,
    filter: Option<String>,
    output: String,
    budget: usize,
    gas_limit: usize,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        eprintln!("{}", style("Rate limit exceeded. Please wait before running another command.").red());
        std::process::exit(1);
    }

    let start_time = Instant::now();
    info!("Running tests for file: {:?}", input);

    // Validate input file exists
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {:?}", input));
    }

    // Read and parse the FACET document
    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read file: {:?}", input))?;

    let document = parse_document(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse FACET document {:?}: {}", input, e))?;

    // Extract test blocks from the document
    let test_blocks: Vec<_> = document.blocks
        .iter()
        .filter_map(|block| match block {
            fct_ast::FacetNode::Test(test_block) => Some(test_block),
            _ => None,
        })
        .collect();

    if test_blocks.is_empty() {
        println!("{}", style("No @test blocks found in document").yellow());
        return Ok(());
    }

    // Apply filter if provided
    let filtered_tests = if let Some(filter_pattern) = &filter {
        let filter_regex = match Regex::new(filter_pattern) {
            Ok(re) => re,
            Err(e) => {
                eprintln!("{} Invalid filter pattern '{}': {}",
                    style("Error:").red(), filter_pattern, e);
                return Err(anyhow::anyhow!("Invalid filter pattern"));
            }
        };

        test_blocks
            .into_iter()
            .filter(|test| filter_regex.is_match(&test.name))
            .collect()
    } else {
        test_blocks
    };

    if filtered_tests.is_empty() {
        println!("{}", style("No tests match the specified filter").yellow());
        return Ok(());
    }

    println!("{} Running {} test(s) from {:?}", TEST_EMOJI, filtered_tests.len(), input);
    if let Some(filter_pattern) = &filter {
        println!("{} Filter: {}", style("Filter:").blue(), filter_pattern);
    }
    println!();

    // Create test runner with resource limits
    let test_runner = fct_engine::TestRunner::new(gas_limit, budget);

    // Run all tests
    let mut test_results = Vec::new();
    let mut passed_count = 0;
    let mut failed_count = 0;

    for test_block in &filtered_tests {
        let test_start = Instant::now();
        debug!("Running test: {}", test_block.name);

        match test_runner.run_test(&document, test_block) {
            Ok(result) => {
                let test_duration = test_start.elapsed();

                if result.passed {
                    passed_count += 1;
                    if output == "verbose" {
                        println!("{} {} ({})", PASS_EMOJI, test_block.name,
                            style(format!("{:.2}ms", test_duration.as_millis())).dim());
                    }
                } else {
                    failed_count += 1;
                    if output != "json" {
                        println!("{} {} ({})", FAIL_EMOJI, test_block.name,
                            style(format!("{:.2}ms", test_duration.as_millis())).dim());

                        // Print assertion failures
                        for assertion in &result.assertions {
                            if !assertion.passed {
                                println!("  {} {}", style("✗").red(), assertion.message);
                                if output == "verbose" {
                                    if let Some(actual) = &assertion.actual_value {
                                        println!("    {} Actual: {}", style("│").dim(), actual);
                                    }
                                }
                            }
                        }
                    }
                }

                test_results.push(result);
            }
            Err(e) => {
                failed_count += 1;
                error!("Test '{}' failed with error: {}", test_block.name, e);

                if output != "json" {
                    println!("{} {} ({})", FAIL_EMOJI, test_block.name,
                        style(format!("{:.2}ms", test_start.elapsed().as_millis())).dim());
                    println!("  {} {}", style("✗").red(), e);
                }

                // Create a failed test result
                let failed_result = fct_engine::TestResult {
                    name: test_block.name.clone(),
                    passed: false,
                    assertions: vec![],
                    error: Some(e.to_string()),
                    rendered_output: None,
                    telemetry: fct_engine::TestTelemetry {
                        tokens_used: 0,
                        estimated_cost: 0.0,
                        execution_time_ms: test_start.elapsed().as_millis() as u64,
                        gas_consumed: 0,
                        variables_computed: 0,
                    },
                };
                test_results.push(failed_result);
            }
        }
    }

    // Generate output in the requested format
    let total_duration = start_time.elapsed();

    match output.as_str() {
        "json" => generate_json_output(&test_results, &input, total_duration)?,
        "junit" => generate_junit_output(&test_results, &input, total_duration)?,
        _ => generate_summary_output(&test_results, passed_count, failed_count, &input, total_duration),
    }

    // Exit with error code if any tests failed
    if failed_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

/// Generate JSON output using TestReporter
fn generate_json_output(
    test_results: &[fct_engine::TestResult],
    input_file: &Path,
    total_duration: std::time::Duration
) -> Result<()> {
    let reporter = TestReporter {
        format: ReportFormat::Json,
    };

    let report = reporter.generate_report(test_results)?;
    println!("{}", report);
    Ok(())
}

/// Generate JUnit XML output
fn generate_junit_output(
    test_results: &[fct_engine::TestResult],
    input_file: &Path,
    total_duration: std::time::Duration
) -> Result<()> {
    let reporter = TestReporter {
        format: ReportFormat::JUnit,
    };

    let report = reporter.generate_report(test_results)?;
    println!("{}", report);
    Ok(())
}

/// Generate human-readable summary output
fn generate_summary_output(
    test_results: &[fct_engine::TestResult],
    passed_count: usize,
    failed_count: usize,
    input_file: &Path,
    total_duration: std::time::Duration
) {
    println!();
    println!("{}", style("─".repeat(50)).dim());

    // Overall summary
    let total_count = passed_count + failed_count;
    let status = if failed_count == 0 {
        style(format!("PASSED ({} passed)", passed_count)).green()
    } else {
        style(format!("FAILED ({} passed, {} failed)", passed_count, failed_count)).red()
    };

    println!("{} {} in {:.2}s",
        TEST_EMOJI,
        status,
        total_duration.as_secs_f64()
    );

    // Telemetry summary
    let total_tokens: usize = test_results.iter().map(|r| r.telemetry.tokens_used).sum();
    let total_cost: f64 = test_results.iter().map(|r| r.telemetry.estimated_cost).sum();
    let total_gas: usize = test_results.iter().map(|r| r.telemetry.gas_consumed).sum();

    if total_tokens > 0 {
        println!("{} Tokens: {}, Cost: ${:.6}, Gas: {}",
            style("Telemetry:").blue(),
            total_tokens,
            total_cost,
            total_gas
        );
    }

    println!("{} {}", style("File:").blue(), input_file.display());

    // Failed test details
    if failed_count > 0 {
        println!();
        println!("{}", style("Failed Tests:").red());
        for result in test_results.iter().filter(|r| !r.passed) {
            println!("  {} {}", FAIL_EMOJI, result.name);
            for assertion in &result.assertions {
                if !assertion.passed {
                    println!("    ✗ {}", assertion.message);
                }
            }
            if let Some(error) = &result.error {
                println!("    ✗ {}", error);
            }
        }
    }
}