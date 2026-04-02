// ============================================================================
// TEST RUNNER - @test blocks execution engine
// ============================================================================

use crate::errors::{EngineError, EngineResult};
use crate::{
    count_facet_units_in_value, derive_message_section_id, value_node_to_json, ExecutionContext,
    RDagEngine, Section, TokenBoxModel, ToolDefinition, ToolExecutor, ToolInvocation,
};
use fct_ast::{
    Assertion, AssertionKind, BodyNode, FacetBlock, FacetDocument, FacetNode, KeyValueNode,
    MockDefinition, OrderedMap, PipelineNode, ScalarValue, TestBlock, ValueNode, FACET_VERSION,
};
use fct_std::{LensContext, LensRegistry, TrustLevel};
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
    pub canonical_output: Option<String>,
    pub execution_output: Option<String>,
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
    pub canonical: Option<serde_json::Value>,
    pub execution: Option<serde_json::Value>,
}

/// Test runner engine
pub struct TestRunner {
    pub gas_limit: usize,
    pub token_budget: usize,
    pub mode: crate::ExecutionMode,
}

impl TestRunner {
    /// Create new test runner with resource limits
    pub fn new(gas_limit: usize, token_budget: usize) -> Self {
        Self::new_with_mode(gas_limit, token_budget, crate::ExecutionMode::Exec)
    }

    /// Create new test runner with resource limits and execution mode
    pub fn new_with_mode(
        gas_limit: usize,
        token_budget: usize,
        mode: crate::ExecutionMode,
    ) -> Self {
        Self {
            gas_limit,
            token_budget,
            mode,
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

        // Build effective document with @test vars overrides.
        let effective_doc = match self.build_doc_with_var_overrides(doc, &test.vars) {
            Ok(effective_doc) => effective_doc,
            Err(e) => {
                return Ok(TestResult {
                    name: test.name.clone(),
                    passed: false,
                    assertions: Vec::new(),
                    telemetry: test_ctx.telemetry,
                    rendered_output: None,
                    canonical_output: None,
                    execution_output: build_execution_test_view(&test_ctx.execution_ctx)
                        .ok()
                        .map(|v| v.to_string()),
                    error: Some(e.to_string()),
                });
            }
        };

        // Apply runtime input overrides for @input(...) directives
        if let Err(e) = self.apply_input_overrides(&mut test_ctx, &test.input) {
            return Ok(TestResult {
                name: test.name.clone(),
                passed: false,
                assertions: Vec::new(),
                telemetry: test_ctx.telemetry,
                rendered_output: None,
                canonical_output: None,
                execution_output: build_execution_test_view(&test_ctx.execution_ctx)
                    .ok()
                    .map(|v| v.to_string()),
                error: Some(e.to_string()),
            });
        }

        // Apply mocks
        if let Err(e) = self.apply_mocks(&mut test_ctx, &test.mocks) {
            return Ok(TestResult {
                name: test.name.clone(),
                passed: false,
                assertions: Vec::new(),
                telemetry: test_ctx.telemetry,
                rendered_output: None,
                canonical_output: None,
                execution_output: build_execution_test_view(&test_ctx.execution_ctx)
                    .ok()
                    .map(|v| v.to_string()),
                error: Some(e.to_string()),
            });
        }

        // Execute the full pipeline
        let rendered_output = match self.execute_pipeline(&effective_doc, &mut test_ctx) {
            Ok(output) => Some(output),
            Err(e) => {
                return Ok(TestResult {
                    name: test.name.clone(),
                    passed: false,
                    assertions: Vec::new(),
                    telemetry: test_ctx.telemetry,
                    rendered_output: None,
                    canonical_output: None,
                    execution_output: build_execution_test_view(&test_ctx.execution_ctx)
                        .ok()
                        .map(|v| v.to_string()),
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
            canonical_output: test_ctx.canonical.as_ref().map(|v| v.to_string()),
            execution_output: test_ctx.execution.as_ref().map(|v| v.to_string()),
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
                    canonical_output: None,
                    execution_output: None,
                    error: Some(e.to_string()),
                }]
            })
    }

    /// Create isolated test context
    fn create_test_context(&self, _test: &TestBlock) -> EngineResult<TestContext> {
        Ok(TestContext {
            execution_ctx: ExecutionContext::new_with_mode(self.gas_limit, self.mode),
            mock_registry: MockRegistry::default(),
            telemetry: TestTelemetry {
                tokens_used: 0,
                estimated_cost: 0.0,
                execution_time_ms: 0,
                gas_consumed: 0,
                variables_computed: 0,
            },
            canonical: None,
            execution: None,
        })
    }

    /// Build an effective document by appending @test vars as the latest @vars override block.
    fn build_doc_with_var_overrides(
        &self,
        doc: &FacetDocument,
        vars: &OrderedMap<String, ValueNode>,
    ) -> EngineResult<FacetDocument> {
        if vars.is_empty() {
            return Ok(doc.clone());
        }

        let var_type_decls = collect_var_type_declarations(doc)?;
        for (name, value) in vars {
            if let Some(decl) = var_type_decls.get(name) {
                validate_var_override_value(name, value, decl)?;
            }
        }

        let mut effective = doc.clone();
        let mut body = Vec::with_capacity(vars.len());
        for (name, value) in vars {
            body.push(BodyNode::KeyValue(KeyValueNode {
                key: name.clone(),
                key_kind: Default::default(),
                value: value.clone(),
                span: doc.span.clone(),
            }));
        }
        effective.blocks.push(FacetNode::Vars(FacetBlock {
            name: "vars".to_string(),
            attributes: OrderedMap::new(),
            body,
            span: doc.span.clone(),
        }));
        Ok(effective)
    }

    fn apply_input_overrides(
        &self,
        ctx: &mut TestContext,
        input: &OrderedMap<String, ValueNode>,
    ) -> EngineResult<()> {
        for (name, value) in input {
            ctx.execution_ctx.set_input(name.clone(), value.clone());
        }
        Ok(())
    }

    /// Apply mocks for interfaces and lenses
    fn apply_mocks(&self, ctx: &mut TestContext, mocks: &[MockDefinition]) -> EngineResult<()> {
        for mock in mocks {
            if mock.target.contains('.') {
                // Interface mock (e.g., "WeatherAPI.get_current")
                ctx.mock_registry
                    .interface_mocks
                    .insert(mock.target.clone(), mock.return_value.clone());
            } else {
                // Lens mock
                ctx.mock_registry
                    .lens_mocks
                    .insert(mock.target.clone(), mock.return_value.clone());
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

        // Simulate @test interface mock calls through ToolExecutor guard path.
        self.execute_mocked_interface_calls(doc, ctx)?;

        // Update telemetry
        ctx.telemetry.gas_consumed = ctx.execution_ctx.gas.consumed;
        ctx.telemetry.variables_computed = ctx.execution_ctx.variables.len();

        // Build sections for Token Box Model
        let mut sections = doc_to_sections(
            doc,
            &ctx.execution_ctx.variables,
            &ctx.execution_ctx.lens_registry,
            ctx.execution_ctx.mode,
        )?;

        // Fallback section if none collected
        if sections.is_empty() {
            let vars_value = ValueNode::Map(
                ctx.execution_ctx
                    .variables
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
            let base_size = count_facet_units_in_value(&vars_value);
            sections.push(
                Section::new(
                    "vars".to_string(),
                    vars_value,
                    base_size,
                )
                .with_priority(200)
                .with_limits(0, 0.0, 0.5),
            );
        }

        // Allocate tokens
        let effective_budget = effective_layout_budget_from_doc(doc, self.token_budget);
        let model = TokenBoxModel::new(effective_budget);
        let allocation = model.allocate_with_mode(
            sections,
            &ctx.execution_ctx.lens_registry,
            ctx.execution_ctx.mode,
        )?;

        // Update token telemetry
        ctx.telemetry.tokens_used = allocation.total_size;
        ctx.telemetry.estimated_cost = estimate_cost(&allocation);

        // Build assertion contexts for `canonical` and `execution` targets.
        let canonical = build_canonical_test_view(doc, &ctx.execution_ctx)?;
        let execution = build_execution_test_view(&ctx.execution_ctx)?;
        ctx.canonical = Some(canonical);
        ctx.execution = Some(execution);

        // For testing, just return a simple JSON representation
        let output = json!({
            "blocks": doc.blocks.len(),
            "total_tokens": allocation.total_size,
            "estimated_cost": estimate_cost(&allocation)
        });

        Ok(serde_json::to_string_pretty(&output)?)
    }

    fn execute_mocked_interface_calls(
        &self,
        doc: &FacetDocument,
        ctx: &mut TestContext,
    ) -> EngineResult<()> {
        if ctx.mock_registry.interface_mocks.is_empty() {
            return Ok(());
        }

        let policy = collect_effective_policy(doc);
        let effect_by_tool = collect_interface_effects(doc);
        let mode = match ctx.execution_ctx.mode {
            crate::ExecutionMode::Pure => "pure",
            crate::ExecutionMode::Exec => "exec",
        };
        let host_profile_id = ctx.execution_ctx.host_profile_id.clone();

        for (target, return_value) in &ctx.mock_registry.interface_mocks {
            if ctx.execution_ctx.mode == crate::ExecutionMode::Pure {
                return Err(EngineError::LensExecutionFailed {
                    message: format!("Runtime I/O prohibited in pure mode: tool_call {}", target),
                });
            }

            let mut executor = ToolExecutor::new();
            executor.register_tool(ToolDefinition {
                name: target.clone(),
                description: "test mock tool".to_string(),
                input_schema: json!({"type":"object"}),
                output_schema: None,
            })?;

            let tool_name = target.clone();
            let mocked = return_value.clone();
            executor.register_handler(tool_name.clone(), move |_| Ok(mocked.clone()))?;

            let invocation = ToolInvocation {
                tool_name: target.clone(),
                arguments: HashMap::new(),
                invocation_id: None,
            };

            let decision = executor.evaluate_tool_call_guard(
                &invocation,
                policy.as_ref(),
                Some(&ctx.execution_ctx.variables),
                mode,
                &host_profile_id,
                effect_by_tool.get(target).map(String::as_str),
            )?;
            ctx.execution_ctx.record_guard_decision(decision.clone());

            if decision.error_code.as_deref() == Some("F455") {
                return Err(EngineError::GuardUndecidable {
                    name: target.clone(),
                });
            }
            if decision.decision == "denied" {
                return Err(EngineError::PolicyDenied {
                    name: target.clone(),
                });
            }

            let _ = executor.execute(invocation)?;
        }

        Ok(())
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
                            message: format!("Expected '{}' to contain '{}'", target_value, text),
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
                                    message: format!("Invalid regex pattern '{}': {}", pattern, e),
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
                                    message: format!("Invalid regex pattern '{}': {}", pattern, e),
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
        let root = json!({
            "output": output,
            "telemetry": {
                "tokens": ctx.telemetry.tokens_used,
                "cost": ctx.telemetry.estimated_cost,
                "time_ms": ctx.telemetry.execution_time_ms,
                "gas": ctx.telemetry.gas_consumed,
                "variables_computed": ctx.telemetry.variables_computed
            },
            "canonical": ctx.canonical.clone().unwrap_or(serde_json::Value::Null),
            "execution": ctx.execution.clone().unwrap_or(serde_json::Value::Null)
        });

        if let Some(value) = lookup_json_path(&root, target) {
            json_value_to_display(value)
        } else {
            format!("unknown target: {}", target)
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

#[derive(Debug, Clone)]
struct RuntimeVarTypeDecl {
    declared_type: String,
    min: Option<f64>,
    max: Option<f64>,
    pattern: Option<String>,
}

fn collect_var_type_declarations(
    doc: &FacetDocument,
) -> EngineResult<HashMap<String, RuntimeVarTypeDecl>> {
    let mut out = HashMap::new();
    for node in &doc.blocks {
        if let FacetNode::VarTypes(block) = node {
            for body in &block.body {
                let BodyNode::KeyValue(kv) = body else {
                    continue;
                };
                let decl = parse_runtime_var_type_decl(&kv.key, &kv.value)?;
                out.insert(kv.key.clone(), decl);
            }
        }
    }
    Ok(out)
}

fn parse_runtime_var_type_decl(
    var_name: &str,
    value: &ValueNode,
) -> EngineResult<RuntimeVarTypeDecl> {
    match value {
        ValueNode::String(type_name) => Ok(RuntimeVarTypeDecl {
            declared_type: normalize_runtime_type_name(var_name, type_name)?,
            min: None,
            max: None,
            pattern: None,
        }),
        ValueNode::Map(map) => {
            let type_name = map.get("type").ok_or_else(|| EngineError::ConstraintViolation {
                message: format!("@var_types.{var_name} is missing required 'type'"),
            })?;
            let ValueNode::String(type_name) = type_name else {
                return Err(EngineError::ConstraintViolation {
                    message: format!("@var_types.{var_name}.type must be a string"),
                });
            };

            let min = match map.get("min") {
                Some(ValueNode::Scalar(ScalarValue::Int(v))) => Some(*v as f64),
                Some(ValueNode::Scalar(ScalarValue::Float(v))) => Some(*v),
                Some(_) => {
                    return Err(EngineError::ConstraintViolation {
                        message: format!("@var_types.{var_name}.min must be int or float"),
                    });
                }
                None => None,
            };
            let max = match map.get("max") {
                Some(ValueNode::Scalar(ScalarValue::Int(v))) => Some(*v as f64),
                Some(ValueNode::Scalar(ScalarValue::Float(v))) => Some(*v),
                Some(_) => {
                    return Err(EngineError::ConstraintViolation {
                        message: format!("@var_types.{var_name}.max must be int or float"),
                    });
                }
                None => None,
            };
            let pattern = match map.get("pattern") {
                Some(ValueNode::String(v)) => Some(v.clone()),
                Some(_) => {
                    return Err(EngineError::ConstraintViolation {
                        message: format!("@var_types.{var_name}.pattern must be a string"),
                    });
                }
                None => None,
            };

            Ok(RuntimeVarTypeDecl {
                declared_type: normalize_runtime_type_name(var_name, type_name)?,
                min,
                max,
                pattern,
            })
        }
        _ => Err(EngineError::ConstraintViolation {
            message: format!(
                "@var_types.{var_name} must be a string type or map declaration"
            ),
        }),
    }
}

fn normalize_runtime_type_name(var_name: &str, ty: &str) -> EngineResult<String> {
    match ty {
        "any" | "string" | "int" | "float" | "bool" | "null" => Ok(ty.to_string()),
        _ => Err(EngineError::ConstraintViolation {
            message: format!(
                "@var_types.{var_name} uses unsupported runtime type '{ty}' for @test override checks"
            ),
        }),
    }
}

fn validate_var_override_value(
    var_name: &str,
    value: &ValueNode,
    decl: &RuntimeVarTypeDecl,
) -> EngineResult<()> {
    if !value_matches_runtime_type(value, &decl.declared_type) {
        return Err(EngineError::TypeMismatch {
            message: format!(
                "@test vars override '{}' does not satisfy declared type '{}'",
                var_name, decl.declared_type
            ),
        });
    }

    if let Some(min) = decl.min {
        let Some(actual) = as_number(value) else {
            return Err(EngineError::TypeMismatch {
                message: format!(
                    "@test vars override '{}' must be numeric for min constraint",
                    var_name
                ),
            });
        };
        if actual < min {
            return Err(EngineError::ConstraintViolation {
                message: format!(
                    "@test vars override '{}' violates min >= {}",
                    var_name, min
                ),
            });
        }
    }

    if let Some(max) = decl.max {
        let Some(actual) = as_number(value) else {
            return Err(EngineError::TypeMismatch {
                message: format!(
                    "@test vars override '{}' must be numeric for max constraint",
                    var_name
                ),
            });
        };
        if actual > max {
            return Err(EngineError::ConstraintViolation {
                message: format!(
                    "@test vars override '{}' violates max <= {}",
                    var_name, max
                ),
            });
        }
    }

    if let Some(pattern) = &decl.pattern {
        let ValueNode::String(actual) = value else {
            return Err(EngineError::TypeMismatch {
                message: format!(
                    "@test vars override '{}' must be string for pattern constraint",
                    var_name
                ),
            });
        };

        let regex = regex::Regex::new(pattern).map_err(|_| EngineError::ConstraintViolation {
            message: format!(
                "@var_types.{}.pattern must be a valid regex pattern",
                var_name
            ),
        })?;
        if !regex.is_match(actual) {
            return Err(EngineError::ConstraintViolation {
                message: format!(
                    "@test vars override '{}' does not match pattern '{}'",
                    var_name, pattern
                ),
            });
        }
    }

    Ok(())
}

fn as_number(value: &ValueNode) -> Option<f64> {
    match value {
        ValueNode::Scalar(ScalarValue::Int(v)) => Some(*v as f64),
        ValueNode::Scalar(ScalarValue::Float(v)) => Some(*v),
        _ => None,
    }
}

fn value_matches_runtime_type(value: &ValueNode, declared_type: &str) -> bool {
    match declared_type {
        "any" => true,
        "string" => matches!(value, ValueNode::String(_)),
        "int" => matches!(value, ValueNode::Scalar(ScalarValue::Int(_))),
        "float" => matches!(
            value,
            ValueNode::Scalar(ScalarValue::Float(_)) | ValueNode::Scalar(ScalarValue::Int(_))
        ),
        "bool" => matches!(value, ValueNode::Scalar(ScalarValue::Bool(_))),
        "null" => matches!(value, ValueNode::Scalar(ScalarValue::Null)),
        _ => false,
    }
}

#[derive(Debug, Clone, Copy)]
struct LayoutDefaults {
    priority: i32,
    min: usize,
    grow: f64,
    shrink: f64,
}

#[derive(Debug, Clone)]
struct SectionLayout {
    id: String,
    priority: i32,
    min: usize,
    grow: f64,
    shrink: f64,
    strategy: Option<PipelineNode>,
}

fn doc_to_sections(
    doc: &FacetDocument,
    computed_vars: &HashMap<String, ValueNode>,
    lens_registry: &LensRegistry,
    mode: crate::ExecutionMode,
) -> EngineResult<Vec<Section>> {
    let mut sections = Vec::new();
    let defaults = context_layout_defaults_from_doc(doc);
    let mut system_count = 0usize;
    let mut user_count = 0usize;
    let mut assistant_count = 0usize;

    for node in &doc.blocks {
        let (derived_id, block) = match node {
            FacetNode::System(b) => {
                system_count += 1;
                (derive_message_section_id("system", system_count), b)
            }
            FacetNode::User(b) => {
                user_count += 1;
                (derive_message_section_id("user", user_count), b)
            }
            FacetNode::Assistant(b) => {
                assistant_count += 1;
                (derive_message_section_id("assistant", assistant_count), b)
            }
            _ => continue,
        };

        if !message_block_enabled(block, computed_vars)? {
            continue;
        }

        let layout = resolve_section_layout(block, &defaults, &derived_id);
        let content = extract_message_content(block, computed_vars, lens_registry, mode)?;
        let base_size = count_facet_units_in_value(&content);
        let mut section = Section::new(layout.id, content, base_size)
            .with_priority(layout.priority)
            .with_limits(layout.min, layout.grow, layout.shrink);
        if let Some(strategy) = layout.strategy {
            section = section.with_strategy(strategy);
        }
        sections.push(section);
    }

    Ok(sections)
}

fn effective_layout_budget_from_doc(doc: &FacetDocument, fallback_budget: usize) -> usize {
    for block in &doc.blocks {
        let FacetNode::Context(ctx) = block else {
            continue;
        };
        for body in &ctx.body {
            let BodyNode::KeyValue(kv) = body else {
                continue;
            };
            if kv.key != "budget" {
                continue;
            }
            if let ValueNode::Scalar(ScalarValue::Int(v)) = &kv.value {
                if *v >= 0 {
                    return *v as usize;
                }
            }
        }
    }
    fallback_budget
}

fn context_layout_defaults_from_doc(doc: &FacetDocument) -> LayoutDefaults {
    let mut out = LayoutDefaults {
        priority: 500,
        min: 0,
        grow: 0.0,
        shrink: 0.0,
    };

    for block in &doc.blocks {
        let FacetNode::Context(ctx) = block else {
            continue;
        };
        for body in &ctx.body {
            let BodyNode::KeyValue(kv) = body else {
                continue;
            };
            if kv.key != "defaults" {
                continue;
            }
            let ValueNode::Map(defaults) = &kv.value else {
                continue;
            };
            if let Some(v) = layout_value_as_i32(defaults.get("priority")) {
                out.priority = v;
            }
            if let Some(v) = layout_value_as_usize(defaults.get("min")) {
                out.min = v;
            }
            if let Some(v) = layout_value_as_f64(defaults.get("grow")) {
                out.grow = v;
            }
            if let Some(v) = layout_value_as_f64(defaults.get("shrink")) {
                out.shrink = v;
            }
        }
    }

    out
}

fn resolve_section_layout(
    block: &fct_ast::FacetBlock,
    defaults: &LayoutDefaults,
    derived_id: &str,
) -> SectionLayout {
    let mut layout = SectionLayout {
        id: derived_id.to_string(),
        priority: defaults.priority,
        min: defaults.min,
        grow: defaults.grow,
        shrink: defaults.shrink,
        strategy: None,
    };

    for body in &block.body {
        let BodyNode::KeyValue(kv) = body else {
            continue;
        };
        match kv.key.as_str() {
            "id" => {
                if let ValueNode::String(v) = &kv.value {
                    layout.id = v.clone();
                }
            }
            "priority" => {
                if let Some(v) = layout_value_as_i32(Some(&kv.value)) {
                    layout.priority = v;
                }
            }
            "min" => {
                if let Some(v) = layout_value_as_usize(Some(&kv.value)) {
                    layout.min = v;
                }
            }
            "grow" => {
                if let Some(v) = layout_value_as_f64(Some(&kv.value)) {
                    layout.grow = v;
                }
            }
            "shrink" => {
                if let Some(v) = layout_value_as_f64(Some(&kv.value)) {
                    layout.shrink = v;
                }
            }
            "strategy" => {
                if let ValueNode::Pipeline(p) = &kv.value {
                    layout.strategy = Some(p.clone());
                }
            }
            _ => {}
        }
    }

    layout
}

fn layout_value_as_i32(value: Option<&ValueNode>) -> Option<i32> {
    match value {
        Some(ValueNode::Scalar(ScalarValue::Int(v))) => Some((*v).clamp(0, i32::MAX as i64) as i32),
        _ => None,
    }
}

fn layout_value_as_usize(value: Option<&ValueNode>) -> Option<usize> {
    match value {
        Some(ValueNode::Scalar(ScalarValue::Int(v))) if *v >= 0 => Some(*v as usize),
        _ => None,
    }
}

fn layout_value_as_f64(value: Option<&ValueNode>) -> Option<f64> {
    match value {
        Some(ValueNode::Scalar(ScalarValue::Int(v))) if *v >= 0 => Some(*v as f64),
        Some(ValueNode::Scalar(ScalarValue::Float(v))) if *v >= 0.0 => Some(*v),
        _ => None,
    }
}

/// Convert ValueNode to string representation
fn value_to_string(value: &ValueNode) -> String {
    match value {
        ValueNode::String(s) => s.clone(),
        ValueNode::Scalar(ScalarValue::Bool(b)) => b.to_string(),
        ValueNode::Scalar(s) => format!("{:?}", s),
        ValueNode::List(items) => format!(
            "[{}]",
            items
                .iter()
                .map(value_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ValueNode::Map(map) => {
            let pairs: Vec<_> = map
                .iter()
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
    let positive_words = [
        "good",
        "great",
        "helpful",
        "excellent",
        "positive",
        "thanks",
    ];
    let negative_words = ["bad", "terrible", "unhelpful", "negative", "error", "fail"];

    let text_lower = text.to_lowercase();
    let positive_count = positive_words
        .iter()
        .filter(|word| text_lower.contains(*word))
        .count();
    let negative_count = negative_words
        .iter()
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

fn build_canonical_test_view(
    doc: &FacetDocument,
    exec_ctx: &ExecutionContext,
) -> EngineResult<serde_json::Value> {
    let mut messages = Vec::new();
    let lens_registry = LensRegistry::new();

    for role in ["system", "user", "assistant"] {
        for node in &doc.blocks {
            let block = match (role, node) {
                ("system", FacetNode::System(block)) => Some(block),
                ("user", FacetNode::User(block)) => Some(block),
                ("assistant", FacetNode::Assistant(block)) => Some(block),
                _ => None,
            };
            let Some(block) = block else {
                continue;
            };
            if !message_block_enabled(block, &exec_ctx.variables)? {
                continue;
            }

            let content = extract_message_content(
                block,
                &exec_ctx.variables,
                &lens_registry,
                exec_ctx.mode,
            )?;
            messages.push(json!({
                "role": role,
                "content": value_node_to_json(&content)?
            }));
        }
    }

    let mode = match exec_ctx.mode {
        crate::ExecutionMode::Pure => "pure",
        crate::ExecutionMode::Exec => "exec",
    };

    Ok(json!({
        "metadata": {
            "facet_version": FACET_VERSION,
            "profile": "hypervisor",
            "mode": mode,
            "host_profile_id": exec_ctx.host_profile_id
        },
        "messages": messages
    }))
}

fn build_execution_test_view(ctx: &ExecutionContext) -> EngineResult<serde_json::Value> {
    let events: Vec<serde_json::Value> = ctx
        .guard_decisions
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;

    Ok(json!({
        "metadata": {
            "facet_version": FACET_VERSION,
            "host_profile_id": ctx.host_profile_id
        },
        "provenance": {
            "events": events
        }
    }))
}

fn extract_message_content(
    block: &fct_ast::FacetBlock,
    computed_vars: &HashMap<String, ValueNode>,
    lens_registry: &LensRegistry,
    mode: crate::ExecutionMode,
) -> EngineResult<ValueNode> {
    for body in &block.body {
        if let BodyNode::KeyValue(kv) = body {
            if kv.key == "content" {
                return resolve_message_value_for_test(&kv.value, computed_vars, lens_registry, mode);
            }
        }
    }
    Ok(ValueNode::String(String::new()))
}

fn message_block_enabled(
    block: &fct_ast::FacetBlock,
    computed_vars: &HashMap<String, ValueNode>,
) -> EngineResult<bool> {
    let attr_when = match block.attributes.get("when") {
        Some(v) => eval_when_atom_for_test(v, computed_vars)?,
        None => true,
    };

    let mut body_when = true;
    for body in &block.body {
        let BodyNode::KeyValue(kv) = body else {
            continue;
        };
        if kv.key == "when" {
            body_when = eval_when_atom_for_test(&kv.value, computed_vars)?;
        }
    }

    Ok(attr_when && body_when)
}

fn eval_when_atom_for_test(
    when_value: &ValueNode,
    computed_vars: &HashMap<String, ValueNode>,
) -> EngineResult<bool> {
    match when_value {
        ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(*v),
        ValueNode::Variable(var_ref) => match resolve_variable_ref_for_test(var_ref, computed_vars)? {
            ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(v),
            _ => Err(EngineError::TypeMismatch {
                message: "'when' must evaluate to bool".to_string(),
            }),
        },
        _ => Err(EngineError::TypeMismatch {
            message: "'when' must be bool or variable reference".to_string(),
        }),
    }
}

fn resolve_message_value_for_test(
    value: &ValueNode,
    computed_vars: &HashMap<String, ValueNode>,
    lens_registry: &LensRegistry,
    mode: crate::ExecutionMode,
) -> EngineResult<ValueNode> {
    match value {
        ValueNode::Variable(var_ref) => resolve_variable_ref_for_test(var_ref, computed_vars),
        ValueNode::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_message_value_for_test(
                    item,
                    computed_vars,
                    lens_registry,
                    mode,
                )?);
            }
            Ok(ValueNode::List(out))
        }
        ValueNode::Map(map) => {
            let mut out = OrderedMap::new();
            for (k, v) in map {
                out.insert(
                    k.clone(),
                    resolve_message_value_for_test(v, computed_vars, lens_registry, mode)?,
                );
            }
            Ok(ValueNode::Map(out))
        }
        ValueNode::Pipeline(pipeline) => {
            let mut current =
                resolve_message_value_for_test(&pipeline.initial, computed_vars, lens_registry, mode)?;
            let ctx = LensContext {
                variables: computed_vars.clone(),
            };

            for lens_call in &pipeline.lenses {
                let lens =
                    lens_registry
                        .get(&lens_call.name)
                        .ok_or_else(|| EngineError::UnknownLens {
                            name: lens_call.name.clone(),
                        })?;
                let signature = lens.signature();
                if signature.trust_level != TrustLevel::Pure {
                    return Err(EngineError::LensExecutionFailed {
                        message: format!(
                            "Message content lens '{}' must be Level-0 (pure)",
                            lens_call.name
                        ),
                    });
                }

                let mut resolved_args = Vec::with_capacity(lens_call.args.len());
                for arg in &lens_call.args {
                    resolved_args.push(resolve_message_value_for_test(
                        arg,
                        computed_vars,
                        lens_registry,
                        mode,
                    )?);
                }
                let mut resolved_kwargs = HashMap::with_capacity(lens_call.kwargs.len());
                for (k, v) in &lens_call.kwargs {
                    resolved_kwargs.insert(
                        k.clone(),
                        resolve_message_value_for_test(v, computed_vars, lens_registry, mode)?,
                    );
                }

                current = lens
                    .execute(current, resolved_args, resolved_kwargs, &ctx)
                    .map_err(|e| EngineError::LensExecutionFailed {
                        message: format!("Message content lens execution failed: {}", e),
                    })?;
            }

            Ok(current)
        }
        ValueNode::Directive(_) => Err(EngineError::ExecutionError {
            message: "Unresolved directive in message content".to_string(),
        }),
        _ => Ok(value.clone()),
    }
}

fn resolve_variable_ref_for_test(
    var_ref: &str,
    computed_vars: &HashMap<String, ValueNode>,
) -> EngineResult<ValueNode> {
    let mut parts = var_ref.split('.');
    let base = parts.next().unwrap_or(var_ref);
    let mut current =
        computed_vars
            .get(base)
            .cloned()
            .ok_or_else(|| EngineError::VariableNotFound {
                var: base.to_string(),
            })?;

    for segment in parts {
        if segment.chars().all(|c| c.is_ascii_digit()) {
            return Err(EngineError::InvalidVariablePath {
                path: var_ref.to_string(),
            });
        }

        current = match current {
            ValueNode::Map(map) => {
                map.get(segment)
                    .cloned()
                    .ok_or_else(|| EngineError::InvalidVariablePath {
                        path: var_ref.to_string(),
                    })?
            }
            _ => {
                return Err(EngineError::InvalidVariablePath {
                    path: var_ref.to_string(),
                });
            }
        };
    }

    Ok(current)
}

#[derive(Debug, Clone)]
enum JsonPathSegment {
    Key(String),
    Index(usize),
}

fn parse_json_path(path: &str) -> Option<Vec<JsonPathSegment>> {
    if path.is_empty() {
        return None;
    }

    let bytes = path.as_bytes();
    let mut i = 0usize;
    let mut segments = Vec::new();

    while i < bytes.len() {
        // Parse key segment if present.
        if bytes[i] != b'[' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'.' && bytes[i] != b'[' {
                i += 1;
            }
            if start == i {
                return None;
            }
            segments.push(JsonPathSegment::Key(path[start..i].to_string()));
        }

        // Parse one or more [index] suffixes.
        while i < bytes.len() && bytes[i] == b'[' {
            i += 1;
            let idx_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if idx_start == i || i >= bytes.len() || bytes[i] != b']' {
                return None;
            }
            let idx = path[idx_start..i].parse::<usize>().ok()?;
            segments.push(JsonPathSegment::Index(idx));
            i += 1;
        }

        if i < bytes.len() {
            if bytes[i] != b'.' {
                return None;
            }
            i += 1;
        }
    }

    Some(segments)
}

fn lookup_json_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let segments = parse_json_path(path)?;
    let mut current = root;
    for seg in segments {
        current = match seg {
            JsonPathSegment::Key(key) => current.get(&key)?,
            JsonPathSegment::Index(idx) => current.get(idx)?,
        };
    }
    Some(current)
}

fn json_value_to_display(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn collect_effective_policy(document: &FacetDocument) -> Option<OrderedMap<String, ValueNode>> {
    let mut effective: OrderedMap<String, ValueNode> = OrderedMap::new();
    let mut seen_policy = false;

    for node in &document.blocks {
        if let FacetNode::Policy(policy_block) = node {
            seen_policy = true;
            for body in &policy_block.body {
                if let BodyNode::KeyValue(kv) = body {
                    if let Some(existing) = effective.get_mut(&kv.key) {
                        *existing = merge_policy_value(&kv.key, existing.clone(), kv.value.clone());
                    } else {
                        effective.insert(kv.key.clone(), kv.value.clone());
                    }
                }
            }
        }
    }

    seen_policy.then_some(effective)
}

fn merge_policy_value(key: &str, old: ValueNode, new: ValueNode) -> ValueNode {
    match (old, new) {
        (ValueNode::Map(old_map), ValueNode::Map(new_map)) => {
            ValueNode::Map(merge_policy_maps(old_map, new_map))
        }
        (ValueNode::List(old_list), ValueNode::List(new_list))
            if key == "allow" || key == "deny" =>
        {
            ValueNode::List(merge_policy_lists(old_list, new_list))
        }
        (_, replacement) => replacement,
    }
}

fn merge_policy_maps(
    mut old_map: OrderedMap<String, ValueNode>,
    new_map: OrderedMap<String, ValueNode>,
) -> OrderedMap<String, ValueNode> {
    for (key, new_value) in new_map {
        if let Some(old_value) = old_map.get_mut(&key) {
            *old_value = merge_policy_value(&key, old_value.clone(), new_value);
        } else {
            old_map.insert(key, new_value);
        }
    }
    old_map
}

fn merge_policy_lists(old_items: Vec<ValueNode>, new_items: Vec<ValueNode>) -> Vec<ValueNode> {
    let mut merged = Vec::new();
    let mut id_index: OrderedMap<String, usize> = OrderedMap::new();

    for item in old_items {
        if let Some(id) = policy_rule_id(&item) {
            id_index.insert(id.to_string(), merged.len());
        }
        merged.push(item);
    }

    for item in new_items {
        if let Some(id) = policy_rule_id(&item) {
            if let Some(pos) = id_index.get(id).copied() {
                let old_item = merged[pos].clone();
                merged[pos] = merge_policy_value("", old_item, item);
            } else {
                id_index.insert(id.to_string(), merged.len());
                merged.push(item);
            }
        } else {
            merged.push(item);
        }
    }

    merged
}

fn policy_rule_id(item: &ValueNode) -> Option<&str> {
    match item {
        ValueNode::Map(map) => match map.get("id") {
            Some(ValueNode::String(id)) => Some(id.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn collect_interface_effects(document: &FacetDocument) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for node in &document.blocks {
        if let FacetNode::Interface(interface) = node {
            for func in &interface.functions {
                if let Some(effect) = &func.effect {
                    out.insert(format!("{}.{}", interface.name, func.name), effect.clone());
                }
            }
        }
    }
    out
}
