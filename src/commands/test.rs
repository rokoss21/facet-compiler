//! # Test Command
//!
//! This module implements the test command for the FACET compiler.
//! The test command runs @test blocks in FACET documents.

use anyhow::{Context, Result};
use console::{style, Emoji};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tracing::{debug, error, info};

// Import FACET crates
use fct_engine::{ReportFormat, TestReporter};
use fct_parser::parse_document;
use fct_resolver::{Resolver, ResolverConfig};
use fct_validator::TypeChecker;

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
    pure: bool,
    exec: bool,
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

    let start_time = Instant::now();
    info!("Running tests for file: {:?}", input);

    if pure && exec {
        return Err(anyhow::anyhow!("Use only one mode flag: --pure or --exec"));
    }

    let mode = if pure {
        fct_engine::ExecutionMode::Pure
    } else {
        fct_engine::ExecutionMode::Exec
    };

    // Validate input file exists
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {:?}", input));
    }

    // Read and parse the FACET document
    let content =
        fs::read_to_string(&input).with_context(|| format!("Failed to read file: {:?}", input))?;

    let parsed = parse_document(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse FACET document {:?}: {}", input, e))?;

    let base_dir = input
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(std::env::current_dir()?);
    let mut resolver = Resolver::new(ResolverConfig {
        allowed_roots: vec![base_dir.clone()],
        base_dir,
    });
    let document = resolver
        .resolve(parsed)
        .map_err(|e| anyhow::anyhow!("Resolution error: {}", e))?;

    let mut checker = TypeChecker::new();
    checker
        .validate(&document)
        .map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;

    // Extract test blocks from the document
    let test_blocks: Vec<_> = document
        .blocks
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
                eprintln!(
                    "{} Invalid filter pattern '{}': {}",
                    style("Error:").red(),
                    filter_pattern,
                    e
                );
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

    println!(
        "{} Running {} test(s) from {:?}",
        TEST_EMOJI,
        filtered_tests.len(),
        input
    );
    if let Some(filter_pattern) = &filter {
        println!("{} Filter: {}", style("Filter:").blue(), filter_pattern);
    }
    println!();

    // Create test runner with resource limits and execution mode
    let test_runner = fct_engine::TestRunner::new_with_mode(gas_limit, budget, mode);

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
                        println!(
                            "{} {} ({})",
                            PASS_EMOJI,
                            test_block.name,
                            style(format!("{:.2}ms", test_duration.as_millis())).dim()
                        );
                    }
                } else {
                    failed_count += 1;
                    if output != "json" {
                        println!(
                            "{} {} ({})",
                            FAIL_EMOJI,
                            test_block.name,
                            style(format!("{:.2}ms", test_duration.as_millis())).dim()
                        );

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
                    println!(
                        "{} {} ({})",
                        FAIL_EMOJI,
                        test_block.name,
                        style(format!("{:.2}ms", test_start.elapsed().as_millis())).dim()
                    );
                    println!("  {} {}", style("✗").red(), e);
                }

                // Create a failed test result
                let failed_result = fct_engine::TestResult {
                    name: test_block.name.clone(),
                    passed: false,
                    assertions: vec![],
                    error: Some(e.to_string()),
                    rendered_output: None,
                    canonical_output: None,
                    execution_output: None,
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
        _ => generate_summary_output(
            &test_results,
            passed_count,
            failed_count,
            &input,
            total_duration,
        ),
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
    total_duration: std::time::Duration,
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
    total_duration: std::time::Duration,
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
    total_duration: std::time::Duration,
) {
    println!();
    println!("{}", style("─".repeat(50)).dim());

    // Overall summary
    let total_count = passed_count + failed_count;
    let status = if failed_count == 0 {
        style(format!("PASSED ({} passed)", passed_count)).green()
    } else {
        style(format!(
            "FAILED ({} passed, {} failed)",
            passed_count, failed_count
        ))
        .red()
    };

    println!(
        "{} {} in {:.2}s",
        TEST_EMOJI,
        status,
        total_duration.as_secs_f64()
    );

    // Telemetry summary
    let total_tokens: usize = test_results.iter().map(|r| r.telemetry.tokens_used).sum();
    let total_cost: f64 = test_results
        .iter()
        .map(|r| r.telemetry.estimated_cost)
        .sum();
    let total_gas: usize = test_results.iter().map(|r| r.telemetry.gas_consumed).sum();

    if total_tokens > 0 {
        println!(
            "{} Tokens: {}, Cost: ${:.6}, Gas: {}",
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

#[cfg(test)]
mod tests {
    use super::*;
    use governor::{Quota, RateLimiter};
    use nonzero_ext::nonzero;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn execute_test_rejects_conflicting_mode_flags() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-test-flags-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");
        let input_path = test_dir.join("input.facet");
        fs::write(&input_path, "@system\n  content: \"hello\"\n").expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        let err = execute_test(
            input_path,
            None,
            "summary".to_string(),
            1024,
            2048,
            true,
            true,
            &limiter,
        )
        .unwrap_err();
        assert!(err.to_string().contains("Use only one mode flag"));

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execute_test_returns_ok_when_document_has_no_tests() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-test-empty-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");
        let input_path = test_dir.join("input.facet");
        fs::write(&input_path, "@system\n  content: \"hello\"\n").expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        execute_test(
            input_path,
            None,
            "summary".to_string(),
            1024,
            2048,
            false,
            true,
            &limiter,
        )
        .expect("should return ok for document without @test blocks");

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execute_test_validates_document_even_without_tests() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-test-validate-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");
        let input_path = test_dir.join("input.facet");
        fs::write(
            &input_path,
            r#"
@system
  foo: "bar"
"#,
        )
        .expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        let err = execute_test(
            input_path,
            None,
            "summary".to_string(),
            1024,
            2048,
            false,
            true,
            &limiter,
        )
        .expect_err("invalid document must fail validation before test discovery");
        assert!(err.to_string().contains("Validation error"));

        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn execute_test_runs_guarded_mock_in_exec_mode() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-test-guarded-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");
        let input_path = test_dir.join("input.facet");
        fs::write(
            &input_path,
            r#"
@interface WeatherAPI
  fn get_current(city: string) -> string (effect="read")

@policy
  allow: [{ op: "tool_call", name: "WeatherAPI.get_current", effect: "read" }]

@test(name="guarded-mock")
  mock:
    "WeatherAPI.get_current": "ok"
"#,
        )
        .expect("write input");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        execute_test(
            input_path,
            None,
            "summary".to_string(),
            1024,
            2048,
            false,
            true,
            &limiter,
        )
        .expect("execute_test should pass guarded mock flow in exec mode");

        let _ = fs::remove_dir_all(test_dir);
    }
}
