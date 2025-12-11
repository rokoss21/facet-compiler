// ============================================================================
// TOOL EXECUTION ENGINE
// ============================================================================
// Runtime tool calling system for FACET interfaces
// Supports multiple LLM providers: OpenAI, Anthropic, Llama

use crate::errors::{EngineError, EngineResult};
use fct_ast::ValueNode;
use serde::{Deserialize, Serialize};
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
        let _tool = self.tools.get(&invocation.tool_name).ok_or_else(|| {
            EngineError::ExecutionError {
                message: format!("Tool '{}' not found", invocation.tool_name),
            }
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
            ScalarValue::Float(f) => {
                serde_json::Number::from_f64(*f)
                    .map(serde_json::Value::Number)
                    .ok_or_else(|| EngineError::ExecutionError {
                        message: format!("Invalid float value: {}", f),
                    })
            }
        },
        ValueNode::String(s) => Ok(serde_json::Value::String(s.clone())),
        ValueNode::List(items) => {
            let json_items: Result<Vec<_>, _> =
                items.iter().map(value_node_to_json).collect();
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
    use fct_ast::ScalarValue;

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
}
