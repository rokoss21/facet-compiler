// ============================================================================
// TEST REPORTER - JSON Test Reports
// ============================================================================
// Generates comprehensive test reports in JSON format

use serde::{Deserialize, Serialize};
use crate::test_runner::{AssertionResult, TestResult};
// use serde_json::Value as JsonValue;
use std::time::SystemTime;

// ============================================================================
// REPORT STRUCTURES
// ============================================================================

/// Complete test suite report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteReport {
    /// Report metadata
    pub metadata: ReportMetadata,
    /// Test results
    pub tests: Vec<TestReportEntry>,
    /// Summary statistics
    pub summary: TestSummary,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// Report generation timestamp
    pub timestamp: String,
    /// FACET version
    pub facet_version: String,
    /// Test runner version
    pub runner_version: String,
    /// Environment info
    pub environment: Option<String>,
}

/// Individual test report entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestReportEntry {
    /// Test name
    pub name: String,
    /// Test status
    pub status: TestStatus,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Assertions
    pub assertions: Vec<AssertionReport>,
    /// Telemetry data
    pub telemetry: TelemetryReport,
    /// Rendered output (optional)
    pub output: Option<String>,
    /// Error message if test failed
    pub error: Option<String>,
}

/// Test status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

/// Assertion report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionReport {
    /// Assertion type
    pub kind: String,
    /// Whether assertion passed
    pub passed: bool,
    /// Assertion message
    pub message: String,
    /// Actual value obtained
    pub actual_value: Option<String>,
    /// Expected value (if applicable)
    pub expected_value: Option<String>,
}

/// Telemetry report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryReport {
    /// Tokens used
    pub tokens_used: usize,
    /// Estimated cost in USD
    pub estimated_cost: f64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Gas consumed
    pub gas_consumed: usize,
    /// Variables computed
    pub variables_computed: usize,
}

/// Test summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    /// Total number of tests
    pub total: usize,
    /// Number of passed tests
    pub passed: usize,
    /// Number of failed tests
    pub failed: usize,
    /// Number of skipped tests
    pub skipped: usize,
    /// Number of tests with errors
    pub errors: usize,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Total tokens used
    pub total_tokens: usize,
    /// Total estimated cost
    pub total_cost: f64,
}

// ============================================================================
// TEST REPORTER
// ============================================================================

/// Test reporter for generating reports
pub struct TestReporter {
    /// Report format
    pub format: ReportFormat,
}

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    Json,
    JsonPretty,
    JUnit,
}

impl TestReporter {
    /// Create new test reporter
    pub fn new(format: ReportFormat) -> Self {
        Self { format }
    }

    /// Generate report from test results
    pub fn generate_report(&self, results: &[TestResult]) -> Result<String, serde_json::Error> {
        let report = self.build_report(results);

        match self.format {
            ReportFormat::Json => serde_json::to_string(&report),
            ReportFormat::JsonPretty => serde_json::to_string_pretty(&report),
            ReportFormat::JUnit => self.generate_junit_report(&report),
        }
    }

    /// Build test suite report
    fn build_report(&self, results: &[TestResult]) -> TestSuiteReport {
        let metadata = ReportMetadata {
            timestamp: format_timestamp(SystemTime::now()),
            facet_version: env!("CARGO_PKG_VERSION").to_string(),
            runner_version: "1.0.0".to_string(),
            environment: std::env::var("FACET_ENV").ok(),
        };

        let tests: Vec<TestReportEntry> = results
            .iter()
            .map(|result| self.convert_test_result(result))
            .collect();

        let summary = self.compute_summary(&tests);

        TestSuiteReport {
            metadata,
            tests,
            summary,
        }
    }

    /// Convert TestResult to TestReportEntry
    fn convert_test_result(&self, result: &TestResult) -> TestReportEntry {
        let status = if result.passed {
            TestStatus::Passed
        } else if result.error.is_some() {
            TestStatus::Error
        } else {
            TestStatus::Failed
        };

        let assertions: Vec<AssertionReport> = result
            .assertions
            .iter()
            .map(|a| self.convert_assertion(a))
            .collect();

        let telemetry = TelemetryReport {
            tokens_used: result.telemetry.tokens_used,
            estimated_cost: result.telemetry.estimated_cost,
            execution_time_ms: result.telemetry.execution_time_ms,
            gas_consumed: result.telemetry.gas_consumed,
            variables_computed: result.telemetry.variables_computed,
        };

        TestReportEntry {
            name: result.name.clone(),
            status,
            duration_ms: result.telemetry.execution_time_ms,
            assertions,
            telemetry,
            output: result.rendered_output.clone(),
            error: result.error.clone(),
        }
    }

    /// Convert AssertionResult to AssertionReport
    fn convert_assertion(&self, assertion: &AssertionResult) -> AssertionReport {
        AssertionReport {
            kind: format!("{:?}", assertion.assertion.kind),
            passed: assertion.passed,
            message: assertion.message.clone(),
            actual_value: assertion.actual_value.clone(),
            expected_value: None, // Could extract from assertion.kind if needed
        }
    }

    /// Compute summary statistics
    fn compute_summary(&self, tests: &[TestReportEntry]) -> TestSummary {
        let total = tests.len();
        let passed = tests.iter().filter(|t| t.status == TestStatus::Passed).count();
        let failed = tests.iter().filter(|t| t.status == TestStatus::Failed).count();
        let skipped = tests.iter().filter(|t| t.status == TestStatus::Skipped).count();
        let errors = tests.iter().filter(|t| t.status == TestStatus::Error).count();

        let total_duration_ms = tests.iter().map(|t| t.duration_ms).sum();
        let total_tokens = tests.iter().map(|t| t.telemetry.tokens_used).sum();
        let total_cost = tests.iter().map(|t| t.telemetry.estimated_cost).sum();

        TestSummary {
            total,
            passed,
            failed,
            skipped,
            errors,
            total_duration_ms,
            total_tokens,
            total_cost,
        }
    }

    /// Generate JUnit XML report
    fn generate_junit_report(&self, report: &TestSuiteReport) -> Result<String, serde_json::Error> {
        // For now, return JSON representation
        // Full JUnit XML implementation would require an XML library
        serde_json::to_string_pretty(&serde_json::json!({
            "testsuite": {
                "name": "FACET Tests",
                "tests": report.summary.total,
                "failures": report.summary.failed,
                "errors": report.summary.errors,
                "skipped": report.summary.skipped,
                "time": report.summary.total_duration_ms as f64 / 1000.0,
                "timestamp": report.metadata.timestamp,
                "testcases": report.tests.iter().map(|test| {
                    serde_json::json!({
                        "name": test.name,
                        "classname": "facet.test",
                        "time": test.duration_ms as f64 / 1000.0,
                        "status": format!("{:?}", test.status).to_lowercase(),
                    })
                }).collect::<Vec<_>>()
            }
        }))
    }
}

impl Default for TestReporter {
    fn default() -> Self {
        Self::new(ReportFormat::JsonPretty)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Format timestamp as ISO 8601
fn format_timestamp(time: SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => {
            // Simple ISO 8601 format (without timezone for now)
            let secs = duration.as_secs();
            let nanos = duration.subsec_nanos();
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                1970 + secs / 31557600, // Approximate year
                ((secs % 31557600) / 2629800) + 1, // Approximate month
                ((secs % 2629800) / 86400) + 1, // Approximate day
                (secs % 86400) / 3600, // Hours
                (secs % 3600) / 60, // Minutes
                secs % 60, // Seconds
                nanos / 1_000_000 // Milliseconds
            )
        }
        Err(_) => "1970-01-01T00:00:00.000Z".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_runner::{TestTelemetry, TestResult, AssertionResult};
    use fct_ast::{Assertion, AssertionKind, ValueNode};

    fn create_test_result(name: &str, passed: bool) -> TestResult {
        TestResult {
            name: name.to_string(),
            passed,
            assertions: vec![],
            telemetry: TestTelemetry {
                tokens_used: 100,
                estimated_cost: 0.001,
                execution_time_ms: 50,
                gas_consumed: 10,
                variables_computed: 5,
            },
            rendered_output: Some("test output".to_string()),
            error: if passed { None } else { Some("Test failed".to_string()) },
        }
    }

    #[test]
    fn test_generate_json_report() {
        let reporter = TestReporter::new(ReportFormat::JsonPretty);
        let results = vec![
            create_test_result("test1", true),
            create_test_result("test2", false),
        ];

        let report = reporter.generate_report(&results).unwrap();
        assert!(report.contains("test1"));
        assert!(report.contains("test2"));
        assert!(report.contains("summary"));
    }

    #[test]
    fn test_summary_computation() {
        let reporter = TestReporter::new(ReportFormat::Json);
        let results = vec![
            create_test_result("test1", true),
            create_test_result("test2", true),
            create_test_result("test3", false),
        ];

        let report = reporter.build_report(&results);

        assert_eq!(report.summary.total, 3);
        assert_eq!(report.summary.passed, 2);
        assert_eq!(report.summary.failed, 0);
        assert_eq!(report.summary.errors, 1);
        assert_eq!(report.summary.total_tokens, 300);
    }

    #[test]
    fn test_test_status_conversion() {
        let reporter = TestReporter::new(ReportFormat::Json);

        let passed_result = create_test_result("passed", true);
        let entry = reporter.convert_test_result(&passed_result);
        assert_eq!(entry.status, TestStatus::Passed);

        let failed_result = create_test_result("failed", false);
        let entry = reporter.convert_test_result(&failed_result);
        assert_eq!(entry.status, TestStatus::Error);
    }

    #[test]
    fn test_report_metadata() {
        let reporter = TestReporter::new(ReportFormat::Json);
        let report = reporter.build_report(&[]);

        assert!(!report.metadata.facet_version.is_empty());
        assert!(!report.metadata.timestamp.is_empty());
    }

    #[test]
    fn test_junit_format() {
        let reporter = TestReporter::new(ReportFormat::JUnit);
        let results = vec![create_test_result("test1", true)];

        let report = reporter.generate_report(&results).unwrap();
        assert!(report.contains("testsuite"));
        assert!(report.contains("testcases"));
    }
}
