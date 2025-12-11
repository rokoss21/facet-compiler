// ============================================================================
// TEST RUNNER - @test blocks execution engine
// ============================================================================

use crate::errors::EngineResult;
use crate::{ExecutionContext, RDagEngine, Section, TokenBoxModel};
use fct_ast::{
    Assertion, AssertionKind, BodyNode, FacetDocument, FacetNode, MockDefinition,
    ScalarValue, TestBlock, ValueNode
};
use fct_std::LensRegistry;
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

/// Test execution telemetry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestTelemetry {
    pub tokens_used: usize,
    pub estimated_cost: f64,
    pub execution_time_ms: u64,
    pub gas_consumed: usize,
    pub variables_computed: usize,
}

/// Result of a single assertion
#[derive(Debug, Clone)]
pub struct AssertionResult {
    pub assertion: Assertion,
    pub passed: bool,
    pub message: String,
    pub actual_value: Option<String>,
}

/// Complete test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub assertions: Vec<AssertionResult>,
    pub telemetry: TestTelemetry,
    pub rendered_output: Option<String>,
    pub error: Option<String>,
}

/// Mock registry for test execution
#[derive(Debug, Default)]
pub struct MockRegistry {
    pub interface_mocks: HashMap<String, ValueNode>,
    pub lens_mocks: HashMap<String, ValueNode>,
}

/// Test execution context
pub struct TestContext {
    pub execution_ctx: ExecutionContext,
    pub mock_registry: MockRegistry,
    pub telemetry: TestTelemetry,
}

/// Test runner engine
pub struct TestRunner {
    pub gas_limit: usize,
    pub token_budget: usize,
}

impl TestRunner {
    /// Create new test runner with resource limits
    pub fn new(gas_limit: usize, token_budget: usize) -> Self {
        Self {
            gas_limit,
            token_budget,
        }
    }

    /// Discover all @test blocks in a document
    pub fn discover_tests<'a>(&self, doc: &'a FacetDocument) -> Vec<&'a TestBlock> {
        doc.blocks
            .iter()
            .filter_map(|block| {
                if let FacetNode::Test(test_block) = block {
                    Some(test_block)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Run a single test
    pub fn run_test(&self, doc: &FacetDocument, test: &TestBlock) -> EngineResult<TestResult> {
        let start_time = Instant::now();
        
        // Create isolated test context
        let mut test_ctx = self.create_test_context(test)?;
        
        // Apply variable overrides
        self.apply_var_overrides(&mut test_ctx, &test.vars)?;
        
        // Apply mocks
        self.apply_mocks(&mut test_ctx, &test.mocks)?;
        
        // Execute the full pipeline
        let rendered_output = match self.execute_pipeline(doc, &mut test_ctx) {
            Ok(output) => Some(output),
            Err(e) => {
                return Ok(TestResult {
                    name: test.name.clone(),
                    passed: false,
                    assertions: Vec::new(),
                    telemetry: test_ctx.telemetry,
                    rendered_output: None,
                    error: Some(e.to_string()),
                });
            }
        };
        
        // Update telemetry
        test_ctx.telemetry.execution_time_ms = start_time.elapsed().as_millis() as u64;
        
        // Evaluate assertions
        let assertions = self.evaluate_assertions(
            rendered_output.as_deref().unwrap_or(""),
            &test_ctx,
            &test.assertions,
        );
        
        // Check if all assertions passed
        let passed = assertions.iter().all(|a| a.passed);
        
        Ok(TestResult {
            name: test.name.clone(),
            passed,
            assertions,
            telemetry: test_ctx.telemetry,
            rendered_output,
            error: None,
        })
    }

    /// Run all tests in a document
    pub fn run_all(&self, doc: &FacetDocument) -> Vec<TestResult> {
        let tests = self.discover_tests(doc);
        tests
            .into_iter()
            .map(|test| self.run_test(doc, test))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|e| {
                vec![TestResult {
                    name: "TestRunner Error".to_string(),
                    passed: false,
                    assertions: Vec::new(),
                    telemetry: TestTelemetry {
                        tokens_used: 0,
                        estimated_cost: 0.0,
                        execution_time_ms: 0,
                        gas_consumed: 0,
                        variables_computed: 0,
                    },
                    rendered_output: None,
                    error: Some(e.to_string()),
                }]
            })
    }

    /// Create isolated test context
    fn create_test_context(&self, _test: &TestBlock) -> EngineResult<TestContext> {
        Ok(TestContext {
            execution_ctx: ExecutionContext::new(self.gas_limit),
            mock_registry: MockRegistry::default(),
            telemetry: TestTelemetry {
                tokens_used: 0,
                estimated_cost: 0.0,
                execution_time_ms: 0,
                gas_consumed: 0,
                variables_computed: 0,
            },
        })
    }

    /// Apply variable overrides from test vars section
    fn apply_var_overrides(
        &self,
        ctx: &mut TestContext,
        vars: &HashMap<String, ValueNode>,
    ) -> EngineResult<()> {
        for (name, value) in vars {
            ctx.execution_ctx.set_variable(name.clone(), value.clone());
        }
        Ok(())
    }

    /// Apply mocks for interfaces and lenses
    fn apply_mocks(&self, ctx: &mut TestContext, mocks: &[MockDefinition]) -> EngineResult<()> {
        for mock in mocks {
            if mock.target.contains('.') {
                // Interface mock (e.g., "WeatherAPI.get_current")
                ctx.mock_registry.interface_mocks.insert(mock.target.clone(), mock.return_value.clone());
            } else {
                // Lens mock
                ctx.mock_registry.lens_mocks.insert(mock.target.clone(), mock.return_value.clone());
            }
        }
        Ok(())
    }

    /// Execute the full FACET pipeline
    fn execute_pipeline(&self, doc: &FacetDocument, ctx: &mut TestContext) -> EngineResult<String> {
        // Build and validate R-DAG
        let mut engine = RDagEngine::new();
        engine.build(doc)?;
        engine.validate()?;
        
        // Execute R-DAG
        engine.execute(&mut ctx.execution_ctx)?;
        
        // Update telemetry
        ctx.telemetry.gas_consumed = ctx.execution_ctx.gas.consumed;
        ctx.telemetry.variables_computed = ctx.execution_ctx.variables.len();
        
        // Build sections for Token Box Model
        let mut sections = Vec::new();
        for node in &doc.blocks {
            if let Some((id, block)) = match node {
                FacetNode::System(b) => Some(("system", b)),
                FacetNode::User(b) => Some(("user", b)),
                FacetNode::Assistant(b) => Some(("assistant", b)),
                _ => None,
            } {
                let content_value = block_to_value(block);
                let base_size = serde_json::to_string(&content_value)
                    .map(|s| s.len())
                    .unwrap_or_default();
                sections.push(
                    Section::new(id.to_string(), content_value, base_size)
                        .with_priority(100)
                        .with_limits(0, 0.0, 0.5),
                );
            }
        }
        
        // Fallback section if none collected
        if sections.is_empty() {
            sections.push(
                Section::new(
                    "vars".to_string(),
                    ValueNode::Map(ctx.execution_ctx.variables.clone()),
                    serde_json::to_string(&ctx.execution_ctx.variables)
                        .map(|s| s.len())
                        .unwrap_or(0),
                )
                .with_priority(200)
                .with_limits(0, 0.0, 0.5),
            );
        }
        
        // Allocate tokens
        let model = TokenBoxModel::new(self.token_budget);
        let allocation = model.allocate(sections, &ctx.execution_ctx.lens_registry)?;
        
        // Update token telemetry
        ctx.telemetry.tokens_used = allocation.total_size;
        ctx.telemetry.estimated_cost = estimate_cost(&allocation);
        
        // For testing, just return a simple JSON representation
        let output = json!({
            "blocks": doc.blocks.len(),
            "total_tokens": allocation.total_size,
            "estimated_cost": estimate_cost(&allocation)
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }

    /// Evaluate all assertions
    pub fn evaluate_assertions(
        &self,
        output: &str,
        ctx: &TestContext,
        assertions: &[Assertion],
    ) -> Vec<AssertionResult> {
        assertions
            .iter()
            .map(|assertion| {
                let result = match &assertion.kind {
                    AssertionKind::Contains { target, text } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: target_value.contains(text),
                            message: format!(
                                "Expected '{}' to contain '{}'",
                                target_value, text
                            ),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::NotContains { target, text } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: !target_value.contains(text),
                            message: format!(
                                "Expected '{}' to NOT contain '{}'",
                                target_value, text
                            ),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::Equals { target, expected } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        let expected_str = value_to_string(expected);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: target_value == expected_str,
                            message: format!(
                                "Expected '{}' to equal '{}'",
                                target_value, expected_str
                            ),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::NotEquals { target, expected } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        let expected_str = value_to_string(expected);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: target_value != expected_str,
                            message: format!(
                                "Expected '{}' to NOT equal '{}'",
                                target_value, expected_str
                            ),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::LessThan { field, value } => {
                        let field_value = self.get_field_value(field, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: field_value < *value,
                            message: format!(
                                "Expected {} ({}) to be less than {}",
                                field, field_value, value
                            ),
                            actual_value: Some(field_value.to_string()),
                        }
                    }
                    AssertionKind::GreaterThan { field, value } => {
                        let field_value = self.get_field_value(field, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: field_value > *value,
                            message: format!(
                                "Expected {} ({}) to be greater than {}",
                                field, field_value, value
                            ),
                            actual_value: Some(field_value.to_string()),
                        }
                    }
                    AssertionKind::Sentiment { target, expected } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        let sentiment = analyze_sentiment(&target_value);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: sentiment == *expected,
                            message: format!(
                                "Expected sentiment '{}' but got '{}'",
                                expected, sentiment
                            ),
                            actual_value: Some(sentiment),
                        }
                    }
                    AssertionKind::Matches { target, pattern } => {
                        let target_value = self.get_target_value(target, output, ctx);

                        let regex = match regex::Regex::new(pattern) {
                            Ok(re) => re,
                            Err(e) => {
                                // Return a failed assertion if regex is invalid
                                return AssertionResult {
                                    assertion: assertion.clone(),
                                    passed: false,
                                    message: format!(
                                        "Invalid regex pattern '{}': {}",
                                        pattern, e
                                    ),
                                    actual_value: Some(target_value),
                                };
                            }
                        };

                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: regex.is_match(&target_value),
                            message: format!(
                                "Expected '{}' to match pattern '{}'",
                                target_value, pattern
                            ),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::NotMatches { target, pattern } => {
                        let target_value = self.get_target_value(target, output, ctx);

                        let regex = match regex::Regex::new(pattern) {
                            Ok(re) => re,
                            Err(e) => {
                                // Return a failed assertion if regex is invalid
                                return AssertionResult {
                                    assertion: assertion.clone(),
                                    passed: false,
                                    message: format!(
                                        "Invalid regex pattern '{}': {}",
                                        pattern, e
                                    ),
                                    actual_value: Some(target_value),
                                };
                            }
                        };

                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: !regex.is_match(&target_value),
                            message: format!(
                                "Expected '{}' to NOT match pattern '{}'",
                                target_value, pattern
                            ),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::True { target } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: is_truthy(&target_value),
                            message: format!("Expected '{}' to be true", target_value),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::False { target } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: !is_truthy(&target_value),
                            message: format!("Expected '{}' to be false", target_value),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::Null { target } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: target_value == "null",
                            message: format!("Expected '{}' to be null", target_value),
                            actual_value: Some(target_value),
                        }
                    }
                    AssertionKind::NotNull { target } => {
                        let target_value = self.get_target_value(target, output, ctx);
                        AssertionResult {
                            assertion: assertion.clone(),
                            passed: target_value != "null",
                            message: format!("Expected '{}' to not be null", target_value),
                            actual_value: Some(target_value),
                        }
                    }
                };
                result
            })
            .collect()
    }

    /// Get value for assertion target (e.g., "output", "telemetry.cost")
    fn get_target_value(&self, target: &str, output: &str, ctx: &TestContext) -> String {
        match target {
            "output" => output.to_string(),
            s if s.starts_with("telemetry.") => {
                let field = &s[10..]; // Remove "telemetry."
                match field {
                    "tokens" => ctx.telemetry.tokens_used.to_string(),
                    "cost" => format!("{:.4}", ctx.telemetry.estimated_cost),
                    "time" => format!("{}ms", ctx.telemetry.execution_time_ms),
                    "gas" => ctx.telemetry.gas_consumed.to_string(),
                    _ => "unknown telemetry field".to_string(),
                }
            }
            _ => format!("unknown target: {}", target),
        }
    }

    /// Get numeric field value from telemetry
    fn get_field_value(&self, field: &str, ctx: &TestContext) -> f64 {
        match field {
            "cost" => ctx.telemetry.estimated_cost,
            "tokens" => ctx.telemetry.tokens_used as f64,
            "time" => ctx.telemetry.execution_time_ms as f64,
            "gas" => ctx.telemetry.gas_consumed as f64,
            _ => 0.0,
        }
    }
}

/// Convert FacetBlock to ValueNode (from main.rs)
fn block_to_value(block: &fct_ast::FacetBlock) -> ValueNode {
    let mut map = std::collections::HashMap::new();
    let mut list_items = Vec::new();

    for body in &block.body {
        match body {
            BodyNode::KeyValue(kv) => {
                map.insert(kv.key.clone(), kv.value.clone());
            }
            BodyNode::ListItem(item) => {
                list_items.push(item.value.clone());
            }
        }
    }

    if !map.is_empty() && list_items.is_empty() {
        ValueNode::Map(map)
    } else if map.is_empty() && !list_items.is_empty() {
        ValueNode::List(list_items)
    } else {
        // Mixed content: wrap list under "__items"
        map.insert("__items".to_string(), ValueNode::List(list_items));
        ValueNode::Map(map)
    }
}

/// Convert ValueNode to string representation
fn value_to_string(value: &ValueNode) -> String {
    match value {
        ValueNode::String(s) => s.clone(),
        ValueNode::Scalar(ScalarValue::Bool(b)) => b.to_string(),
        ValueNode::Scalar(s) => format!("{:?}", s),
        ValueNode::List(items) => format!("[{}]", 
            items.iter().map(value_to_string).collect::<Vec<_>>().join(", ")
        ),
        ValueNode::Map(map) => {
            let pairs: Vec<_> = map.iter()
                .map(|(k, v)| format!("{}: {}", k, value_to_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        _ => "unknown".to_string(),
    }
}

/// Estimate cost based on token usage (simplified)
fn estimate_cost(allocation: &crate::AllocationResult) -> f64 {
    // Rough estimate: $0.001 per 1K tokens
    allocation.total_size as f64 * 0.000001
}

/// Simple sentiment analysis (placeholder)
fn analyze_sentiment(text: &str) -> String {
    // Very basic sentiment detection
    let positive_words = ["good", "great", "helpful", "excellent", "positive", "thanks"];
    let negative_words = ["bad", "terrible", "unhelpful", "negative", "error", "fail"];
    
    let text_lower = text.to_lowercase();
    let positive_count = positive_words.iter()
        .filter(|word| text_lower.contains(*word))
        .count();
    let negative_count = negative_words.iter()
        .filter(|word| text_lower.contains(*word))
        .count();
    
    if positive_count > negative_count {
        "positive".to_string()
    } else if negative_count > positive_count {
        "negative".to_string()
    } else {
        "neutral".to_string()
    }
}

/// Check if a string value is truthy
fn is_truthy(value: &str) -> bool {
    !value.is_empty() && value != "false" && value != "0" && value != "null"
}