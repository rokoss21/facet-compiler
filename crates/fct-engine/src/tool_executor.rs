// ============================================================================
// TOOL EXECUTION ENGINE
// ============================================================================
// Runtime tool calling system for FACET interfaces
// Supports multiple LLM providers: OpenAI, Anthropic, Llama

use crate::errors::{EngineError, EngineResult};
use crate::r_dag::ExecutionGuardDecision;
use fct_ast::{OrderedMap, ScalarValue, ValueNode, FACET_VERSION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Tool definition in FACET
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name (unique identifier)
    pub name: String,
    /// Tool description for LLM
    pub description: String,
    /// Input schema (JSON Schema format)
    pub input_schema: serde_json::Value,
    /// Optional output schema
    pub output_schema: Option<serde_json::Value>,
}

/// Tool invocation request
#[derive(Debug, Clone)]
pub struct ToolInvocation {
    /// Tool name to invoke
    pub tool_name: String,
    /// Arguments passed to the tool
    pub arguments: HashMap<String, ValueNode>,
    /// Optional invocation ID for tracking
    pub invocation_id: Option<String>,
}

/// Tool invocation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool name that was invoked
    pub tool_name: String,
    /// Result value
    pub result: ValueNode,
    /// Optional error message if tool failed
    pub error: Option<String>,
    /// Invocation ID if provided
    pub invocation_id: Option<String>,
}

/// Tool execution handler function type
pub type ToolHandler = Box<dyn Fn(&ToolInvocation) -> EngineResult<ValueNode> + Send + Sync>;

#[derive(Debug, Clone)]
struct ToolPolicyDecision {
    allowed: bool,
    policy_rule_id: Option<String>,
    error_code: Option<String>,
}

// ============================================================================
// TOOL EXECUTOR
// ============================================================================

/// Tool execution engine
/// Manages tool registration, validation, and execution
pub struct ToolExecutor {
    /// Registered tools
    tools: HashMap<String, ToolDefinition>,
    /// Tool handlers (runtime implementations)
    handlers: HashMap<String, ToolHandler>,
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            handlers: HashMap::new(),
        }
    }

    /// Register a new tool with its definition
    pub fn register_tool(&mut self, tool: ToolDefinition) -> EngineResult<()> {
        if self.tools.contains_key(&tool.name) {
            return Err(EngineError::ExecutionError {
                message: format!("Tool '{}' is already registered", tool.name),
            });
        }

        self.tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// Register a tool handler (runtime implementation)
    pub fn register_handler<F>(&mut self, tool_name: String, handler: F) -> EngineResult<()>
    where
        F: Fn(&ToolInvocation) -> EngineResult<ValueNode> + Send + Sync + 'static,
    {
        if !self.tools.contains_key(&tool_name) {
            return Err(EngineError::ExecutionError {
                message: format!("Tool '{}' is not registered", tool_name),
            });
        }

        self.handlers.insert(tool_name, Box::new(handler));
        Ok(())
    }

    /// Get tool definition by name
    pub fn get_tool(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name)
    }

    /// List all registered tools
    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Validate tool invocation arguments against schema
    pub fn validate_invocation(&self, invocation: &ToolInvocation) -> EngineResult<()> {
        let _tool =
            self.tools
                .get(&invocation.tool_name)
                .ok_or_else(|| EngineError::ExecutionError {
                    message: format!("Tool '{}' not found", invocation.tool_name),
                })?;

        // TODO: Implement JSON Schema validation
        // For now, just check if tool exists
        if !self.handlers.contains_key(&invocation.tool_name) {
            return Err(EngineError::ExecutionError {
                message: format!("No handler registered for tool '{}'", invocation.tool_name),
            });
        }

        Ok(())
    }

    /// Evaluate policy guard for a tool call and produce a guard decision event.
    pub fn evaluate_tool_call_guard(
        &self,
        invocation: &ToolInvocation,
        policy: Option<&OrderedMap<String, ValueNode>>,
        computed_vars: Option<&HashMap<String, ValueNode>>,
        mode: &str,
        host_profile_id: &str,
        effect_class: Option<&str>,
    ) -> EngineResult<ExecutionGuardDecision> {
        let decision =
            evaluate_tool_call(policy, &invocation.tool_name, effect_class, computed_vars);
        let input_hash = tool_call_input_hash(
            &invocation.tool_name,
            &invocation.arguments,
            host_profile_id,
        )?;

        Ok(ExecutionGuardDecision {
            seq: 0,
            op: "tool_call".to_string(),
            name: invocation.tool_name.clone(),
            effect_class: effect_class.map(|s| s.to_string()),
            mode: mode.to_string(),
            decision: if decision.allowed {
                "allowed".to_string()
            } else {
                "denied".to_string()
            },
            policy_rule_id: decision.policy_rule_id,
            input_hash,
            error_code: decision.error_code,
        })
    }

    /// Execute a tool invocation with fail-closed guard semantics.
    pub fn execute_with_guard(
        &self,
        invocation: ToolInvocation,
        policy: Option<&OrderedMap<String, ValueNode>>,
        computed_vars: Option<&HashMap<String, ValueNode>>,
        mode: &str,
        host_profile_id: &str,
        effect_class: Option<&str>,
    ) -> EngineResult<(ToolResult, ExecutionGuardDecision)> {
        let decision = self.evaluate_tool_call_guard(
            &invocation,
            policy,
            computed_vars,
            mode,
            host_profile_id,
            effect_class,
        )?;

        if decision.error_code.as_deref() == Some("F455") {
            return Err(EngineError::GuardUndecidable {
                name: invocation.tool_name.clone(),
            });
        }
        if decision.decision == "denied" {
            return Err(EngineError::PolicyDenied {
                name: invocation.tool_name.clone(),
            });
        }

        let result = self.execute(invocation)?;
        Ok((result, decision))
    }

    /// Execute a tool invocation
    pub fn execute(&self, invocation: ToolInvocation) -> EngineResult<ToolResult> {
        // Validate invocation
        self.validate_invocation(&invocation)?;

        // Get handler
        let handler = self.handlers.get(&invocation.tool_name).ok_or_else(|| {
            EngineError::ExecutionError {
                message: format!("No handler for tool '{}'", invocation.tool_name),
            }
        })?;

        // Execute handler
        match handler(&invocation) {
            Ok(result) => Ok(ToolResult {
                tool_name: invocation.tool_name.clone(),
                result,
                error: None,
                invocation_id: invocation.invocation_id.clone(),
            }),
            Err(e) => Ok(ToolResult {
                tool_name: invocation.tool_name.clone(),
                result: ValueNode::Scalar(fct_ast::ScalarValue::Null),
                error: Some(e.to_string()),
                invocation_id: invocation.invocation_id.clone(),
            }),
        }
    }

    /// Execute multiple tool invocations in sequence
    pub fn execute_batch(&self, invocations: Vec<ToolInvocation>) -> Vec<ToolResult> {
        invocations
            .into_iter()
            .map(|inv| {
                let tool_name = inv.tool_name.clone();
                let invocation_id = inv.invocation_id.clone();

                self.execute(inv).unwrap_or_else(|e| ToolResult {
                    tool_name,
                    result: ValueNode::Scalar(fct_ast::ScalarValue::Null),
                    error: Some(e.to_string()),
                    invocation_id,
                })
            })
            .collect()
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new()
    }
}

fn evaluate_tool_call(
    policy: Option<&OrderedMap<String, ValueNode>>,
    tool_name: &str,
    effect_class: Option<&str>,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> ToolPolicyDecision {
    let Some(policy_map) = policy else {
        return ToolPolicyDecision {
            allowed: false,
            policy_rule_id: None,
            error_code: Some("F454".to_string()),
        };
    };

    if let Some(deny_rules) = policy_map.get("deny").and_then(as_rule_list) {
        for rule in deny_rules {
            match rule_matches_tool_call(rule, tool_name, effect_class, computed_vars) {
                RuleMatch::Matched(rule_id) => {
                    return ToolPolicyDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F454".to_string()),
                    }
                }
                RuleMatch::Undecidable(rule_id) => {
                    return ToolPolicyDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F455".to_string()),
                    }
                }
                RuleMatch::NoMatch => {}
            }
        }
    }

    if let Some(allow_rules) = policy_map.get("allow").and_then(as_rule_list) {
        for rule in allow_rules {
            match rule_matches_tool_call(rule, tool_name, effect_class, computed_vars) {
                RuleMatch::Matched(rule_id) => {
                    return ToolPolicyDecision {
                        allowed: true,
                        policy_rule_id: rule_id,
                        error_code: None,
                    }
                }
                RuleMatch::Undecidable(rule_id) => {
                    return ToolPolicyDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F455".to_string()),
                    }
                }
                RuleMatch::NoMatch => {}
            }
        }
    }

    default_guard_decision(policy_map, "tool_call", false)
}

fn as_rule_list(value: &ValueNode) -> Option<&Vec<ValueNode>> {
    match value {
        ValueNode::List(items) => Some(items),
        _ => None,
    }
}

enum RuleMatch {
    Matched(Option<String>),
    Undecidable(Option<String>),
    NoMatch,
}

fn rule_matches_tool_call(
    rule: &ValueNode,
    tool_name: &str,
    effect_class: Option<&str>,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> RuleMatch {
    let ValueNode::Map(map) = rule else {
        return RuleMatch::NoMatch;
    };

    let Some(ValueNode::String(op)) = map.get("op") else {
        return RuleMatch::NoMatch;
    };
    if op != "tool_call" {
        return RuleMatch::NoMatch;
    }
    let rule_id = match map.get("id") {
        Some(ValueNode::String(id)) => Some(id.clone()),
        _ => None,
    };

    let name_match = match map.get("name") {
        Some(ValueNode::String(pattern)) => matcher_matches(pattern, tool_name),
        Some(_) => return RuleMatch::Undecidable(rule_id),
        None => true,
    };
    if !name_match {
        return RuleMatch::NoMatch;
    }

    if let Some(effect_matcher) = map.get("effect") {
        match effect_matcher {
            ValueNode::String(pattern) => {
                let Some(op_effect) = effect_class else {
                    return RuleMatch::NoMatch;
                };
                if !matcher_matches(pattern, op_effect) {
                    return RuleMatch::NoMatch;
                }
            }
            _ => return RuleMatch::Undecidable(rule_id),
        }
    }

    let when_eval = match map.get("when") {
        None => true,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return RuleMatch::Undecidable(rule_id),
        },
    };
    if !when_eval {
        return RuleMatch::NoMatch;
    }

    let unless_eval = match map.get("unless") {
        None => false,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return RuleMatch::Undecidable(rule_id),
        },
    };
    if unless_eval {
        return RuleMatch::NoMatch;
    }

    RuleMatch::Matched(rule_id)
}

fn eval_policy_cond(
    cond: &ValueNode,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<bool, ()> {
    match cond {
        ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(*v),
        ValueNode::Variable(var_ref) => {
            let vars = computed_vars.ok_or(())?;
            let value = resolve_policy_var(var_ref, vars)?;
            match value {
                ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(*v),
                _ => Err(()),
            }
        }
        ValueNode::Map(map) => {
            if map.len() != 1 {
                return Err(());
            }

            let (op, arg) = map.iter().next().ok_or(())?;
            match op.as_str() {
                "not" => Ok(!eval_policy_cond(arg, computed_vars)?),
                "all" => {
                    let items = match arg {
                        ValueNode::List(items) if !items.is_empty() => items,
                        _ => return Err(()),
                    };
                    for item in items {
                        if !eval_policy_cond(item, computed_vars)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                "any" => {
                    let items = match arg {
                        ValueNode::List(items) if !items.is_empty() => items,
                        _ => return Err(()),
                    };
                    for item in items {
                        if eval_policy_cond(item, computed_vars)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

fn resolve_policy_var<'a>(
    var_ref: &str,
    vars: &'a HashMap<String, ValueNode>,
) -> Result<&'a ValueNode, ()> {
    let mut parts = var_ref.split('.');
    let base = parts.next().ok_or(())?;
    let mut current = vars.get(base).ok_or(())?;

    for seg in parts {
        if seg.chars().all(|c| c.is_ascii_digit()) {
            return Err(());
        }
        current = match current {
            ValueNode::Map(map) => map.get(seg).ok_or(())?,
            _ => return Err(()),
        };
    }

    Ok(current)
}

fn default_guard_decision(
    policy_map: &OrderedMap<String, ValueNode>,
    op: &str,
    fallback_allow: bool,
) -> ToolPolicyDecision {
    if let Some(defaults_node) = policy_map.get("defaults") {
        let defaults_map = match defaults_node {
            ValueNode::Map(map) => map,
            _ => {
                return ToolPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F455".to_string()),
                };
            }
        };

        if let Some(op_default) = defaults_map.get(op) {
            return match op_default {
                ValueNode::String(s) if s == "allow" => ToolPolicyDecision {
                    allowed: true,
                    policy_rule_id: None,
                    error_code: None,
                },
                ValueNode::String(s) if s == "deny" => ToolPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F454".to_string()),
                },
                ValueNode::Scalar(ScalarValue::Bool(true)) => ToolPolicyDecision {
                    allowed: true,
                    policy_rule_id: None,
                    error_code: None,
                },
                ValueNode::Scalar(ScalarValue::Bool(false)) => ToolPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F454".to_string()),
                },
                _ => ToolPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F455".to_string()),
                },
            };
        }
    }

    ToolPolicyDecision {
        allowed: fallback_allow,
        policy_rule_id: None,
        error_code: if fallback_allow {
            None
        } else {
            Some("F454".to_string())
        },
    }
}

fn matcher_matches(pattern: &str, value: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix(".*") {
        value.starts_with(prefix)
    } else {
        pattern == value
    }
}

fn canonicalize_json(value: &serde_json::Value) -> EngineResult<String> {
    Ok(serde_json_canonicalizer::to_string(value)?)
}

fn tool_call_input_hash(
    tool_name: &str,
    args: &HashMap<String, ValueNode>,
    host_profile_id: &str,
) -> EngineResult<String> {
    let (interface_name, fn_name) = match tool_name.split_once('.') {
        Some((iface, func)) => (iface, func),
        None => (tool_name, ""),
    };

    let input_obj = serde_json::json!({
        "interface": interface_name,
        "fn": fn_name,
        "args": value_node_map_to_json(args)?,
        "host_profile_id": host_profile_id,
        "facet_version": FACET_VERSION,
    });
    let canonical = canonicalize_json(&input_obj)?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical.as_bytes())))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert ValueNode HashMap to JSON Value for tool arguments
pub fn value_node_map_to_json(
    args: &HashMap<String, ValueNode>,
) -> EngineResult<serde_json::Value> {
    let mut json_map = serde_json::Map::new();

    for (key, value) in args {
        let json_value = value_node_to_json(value)?;
        json_map.insert(key.clone(), json_value);
    }

    Ok(serde_json::Value::Object(json_map))
}

/// Convert ValueNode to JSON Value
pub fn value_node_to_json(node: &ValueNode) -> EngineResult<serde_json::Value> {
    use fct_ast::ScalarValue;

    match node {
        ValueNode::Scalar(scalar) => match scalar {
            ScalarValue::Null => Ok(serde_json::Value::Null),
            ScalarValue::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            ScalarValue::Int(i) => Ok(serde_json::Value::Number((*i).into())),
            ScalarValue::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| EngineError::ExecutionError {
                    message: format!("Invalid float value: {}", f),
                }),
        },
        ValueNode::String(s) => Ok(serde_json::Value::String(s.clone())),
        ValueNode::List(items) => {
            let json_items: Result<Vec<_>, _> = items.iter().map(value_node_to_json).collect();
            Ok(serde_json::Value::Array(json_items?))
        }
        ValueNode::Map(map) => {
            let mut json_map = serde_json::Map::new();
            for (k, v) in map {
                json_map.insert(k.clone(), value_node_to_json(v)?);
            }
            Ok(serde_json::Value::Object(json_map))
        }
        _ => Err(EngineError::ExecutionError {
            message: format!("Unsupported ValueNode type for JSON conversion: {:?}", node),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fct_ast::{OrderedMap, ScalarValue};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn test_tool_registration() {
        let mut executor = ToolExecutor::new();

        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                }
            }),
            output_schema: None,
        };

        executor.register_tool(tool).unwrap();
        assert!(executor.get_tool("test_tool").is_some());
        assert_eq!(executor.list_tools().len(), 1);
    }

    #[test]
    fn test_tool_handler_registration() {
        let mut executor = ToolExecutor::new();

        let tool = ToolDefinition {
            name: "echo".to_string(),
            description: "Echo tool".to_string(),
            input_schema: serde_json::json!({}),
            output_schema: None,
        };

        executor.register_tool(tool).unwrap();

        // Register handler
        executor
            .register_handler("echo".to_string(), |inv| {
                Ok(ValueNode::String(format!("Echo: {:?}", inv.tool_name)))
            })
            .unwrap();

        // Execute
        let invocation = ToolInvocation {
            tool_name: "echo".to_string(),
            arguments: HashMap::new(),
            invocation_id: Some("test-1".to_string()),
        };

        let result = executor.execute(invocation).unwrap();
        assert_eq!(result.tool_name, "echo");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_value_node_to_json() {
        // Test scalar conversions
        assert_eq!(
            value_node_to_json(&ValueNode::Scalar(ScalarValue::Null)).unwrap(),
            serde_json::Value::Null
        );

        assert_eq!(
            value_node_to_json(&ValueNode::Scalar(ScalarValue::Bool(true))).unwrap(),
            serde_json::Value::Bool(true)
        );

        assert_eq!(
            value_node_to_json(&ValueNode::Scalar(ScalarValue::Int(42))).unwrap(),
            serde_json::Value::Number(42.into())
        );

        // Test string
        assert_eq!(
            value_node_to_json(&ValueNode::String("test".to_string())).unwrap(),
            serde_json::Value::String("test".to_string())
        );

        // Test list
        let list = ValueNode::List(vec![
            ValueNode::Scalar(ScalarValue::Int(1)),
            ValueNode::Scalar(ScalarValue::Int(2)),
        ]);
        assert_eq!(
            value_node_to_json(&list).unwrap(),
            serde_json::json!([1, 2])
        );
    }

    #[test]
    fn test_execute_batch() {
        let mut executor = ToolExecutor::new();

        // Register tools
        let tool1 = ToolDefinition {
            name: "add".to_string(),
            description: "Add two numbers".to_string(),
            input_schema: serde_json::json!({}),
            output_schema: None,
        };

        executor.register_tool(tool1).unwrap();
        executor
            .register_handler("add".to_string(), |_| {
                Ok(ValueNode::Scalar(ScalarValue::Int(10)))
            })
            .unwrap();

        // Execute batch
        let invocations = vec![
            ToolInvocation {
                tool_name: "add".to_string(),
                arguments: HashMap::new(),
                invocation_id: Some("1".to_string()),
            },
            ToolInvocation {
                tool_name: "add".to_string(),
                arguments: HashMap::new(),
                invocation_id: Some("2".to_string()),
            },
        ];

        let results = executor.execute_batch(invocations);
        assert_eq!(results.len(), 2);
        assert!(results[0].error.is_none());
        assert!(results[1].error.is_none());
    }

    #[test]
    fn test_tool_call_guard_default_denies_without_policy() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();
        executor
            .register_handler("WeatherAPI.get_current".to_string(), |_| {
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: Some("t1".to_string()),
        };

        let err = executor
            .execute_with_guard(
                invocation,
                None,
                None,
                "exec",
                "local.default.v1",
                Some("read"),
            )
            .unwrap_err();

        assert!(matches!(err, EngineError::PolicyDenied { .. }));
        assert!(err.to_string().contains("F454"));
    }

    #[test]
    fn test_tool_call_guard_allows_with_matching_rule() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();
        executor
            .register_handler("WeatherAPI.get_current".to_string(), |_| {
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let allow_rule = ValueNode::Map(OrderedMap::from([
            (
                "id".to_string(),
                ValueNode::String("allow-weather".to_string()),
            ),
            ("op".to_string(), ValueNode::String("tool_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get_current".to_string()),
            ),
            ("effect".to_string(), ValueNode::String("read".to_string())),
        ]));
        let policy = OrderedMap::from([("allow".to_string(), ValueNode::List(vec![allow_rule]))]);

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: Some("t2".to_string()),
        };

        let (result, decision) = executor
            .execute_with_guard(
                invocation,
                Some(&policy),
                None,
                "exec",
                "local.default.v1",
                Some("read"),
            )
            .unwrap();

        assert!(result.error.is_none());
        assert_eq!(decision.decision, "allowed");
        assert_eq!(decision.policy_rule_id.as_deref(), Some("allow-weather"));
        assert!(decision.error_code.is_none());

        let expected_input_obj = serde_json::json!({
            "interface": "WeatherAPI",
            "fn": "get_current",
            "args": serde_json::json!({}),
            "host_profile_id": "local.default.v1",
            "facet_version": FACET_VERSION,
        });
        let expected_hash = format!(
            "sha256:{:x}",
            Sha256::digest(canonicalize_json(&expected_input_obj).unwrap().as_bytes())
        );
        assert_eq!(decision.input_hash, expected_hash);
    }

    #[test]
    fn test_tool_call_guard_effect_is_conjunctive_filter() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();
        executor
            .register_handler("WeatherAPI.get_current".to_string(), |_| {
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let allow_rule = ValueNode::Map(OrderedMap::from([
            ("op".to_string(), ValueNode::String("tool_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get_current".to_string()),
            ),
            ("effect".to_string(), ValueNode::String("read".to_string())),
        ]));
        let policy = OrderedMap::from([("allow".to_string(), ValueNode::List(vec![allow_rule]))]);

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: None,
        };

        let err = executor
            .execute_with_guard(
                invocation,
                Some(&policy),
                None,
                "exec",
                "local.default.v1",
                None,
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::PolicyDenied { .. }));
    }

    #[test]
    fn test_tool_call_guard_short_circuit_any_avoids_undecidable_tail() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();
        executor
            .register_handler("WeatherAPI.get_current".to_string(), |_| {
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let allow_rule = ValueNode::Map(OrderedMap::from([
            ("id".to_string(), ValueNode::String("allow-any".to_string())),
            ("op".to_string(), ValueNode::String("tool_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get_current".to_string()),
            ),
            (
                "when".to_string(),
                ValueNode::Map(OrderedMap::from([(
                    "any".to_string(),
                    ValueNode::List(vec![
                        ValueNode::Scalar(ScalarValue::Bool(true)),
                        ValueNode::Variable("missing.flag".to_string()),
                    ]),
                )])),
            ),
        ]));
        let policy = OrderedMap::from([("allow".to_string(), ValueNode::List(vec![allow_rule]))]);

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: None,
        };

        let (_result, decision) = executor
            .execute_with_guard(
                invocation,
                Some(&policy),
                Some(&HashMap::new()),
                "exec",
                "local.default.v1",
                Some("read"),
            )
            .unwrap();
        assert_eq!(decision.decision, "allowed");
        assert_eq!(decision.policy_rule_id.as_deref(), Some("allow-any"));
    }

    #[test]
    fn test_tool_call_guard_undecidable_returns_f455() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();
        executor
            .register_handler("WeatherAPI.get_current".to_string(), |_| {
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let allow_rule = ValueNode::Map(OrderedMap::from([
            ("op".to_string(), ValueNode::String("tool_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get_current".to_string()),
            ),
            (
                "when".to_string(),
                ValueNode::Variable("missing.flag".to_string()),
            ),
        ]));
        let policy = OrderedMap::from([("allow".to_string(), ValueNode::List(vec![allow_rule]))]);

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: None,
        };

        let err = executor
            .execute_with_guard(
                invocation,
                Some(&policy),
                Some(&HashMap::new()),
                "exec",
                "local.default.v1",
                Some("read"),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::GuardUndecidable { .. }));
        assert!(err.to_string().contains("F455"));
    }

    #[test]
    fn test_tool_call_guard_denied_does_not_invoke_handler() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();

        let handler_calls = Arc::new(AtomicUsize::new(0));
        let handler_calls_clone = Arc::clone(&handler_calls);
        executor
            .register_handler("WeatherAPI.get_current".to_string(), move |_| {
                handler_calls_clone.fetch_add(1, Ordering::SeqCst);
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: Some("deny-before-call".to_string()),
        };

        let err = executor
            .execute_with_guard(
                invocation,
                None,
                None,
                "exec",
                "local.default.v1",
                Some("read"),
            )
            .unwrap_err();

        assert!(matches!(err, EngineError::PolicyDenied { .. }));
        assert_eq!(handler_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_tool_call_guard_undecidable_does_not_invoke_handler() {
        let mut executor = ToolExecutor::new();
        executor
            .register_tool(ToolDefinition {
                name: "WeatherAPI.get_current".to_string(),
                description: "weather".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            })
            .unwrap();

        let handler_calls = Arc::new(AtomicUsize::new(0));
        let handler_calls_clone = Arc::clone(&handler_calls);
        executor
            .register_handler("WeatherAPI.get_current".to_string(), move |_| {
                handler_calls_clone.fetch_add(1, Ordering::SeqCst);
                Ok(ValueNode::String("ok".to_string()))
            })
            .unwrap();

        let allow_rule = ValueNode::Map(OrderedMap::from([
            ("op".to_string(), ValueNode::String("tool_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get_current".to_string()),
            ),
            (
                "when".to_string(),
                ValueNode::Variable("missing.flag".to_string()),
            ),
        ]));
        let policy = OrderedMap::from([("allow".to_string(), ValueNode::List(vec![allow_rule]))]);

        let invocation = ToolInvocation {
            tool_name: "WeatherAPI.get_current".to_string(),
            arguments: HashMap::new(),
            invocation_id: Some("undecidable-before-call".to_string()),
        };

        let err = executor
            .execute_with_guard(
                invocation,
                Some(&policy),
                Some(&HashMap::new()),
                "exec",
                "local.default.v1",
                Some("read"),
            )
            .unwrap_err();

        assert!(matches!(err, EngineError::GuardUndecidable { .. }));
        assert_eq!(handler_calls.load(Ordering::SeqCst), 0);
    }
}
