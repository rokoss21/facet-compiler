// ============================================================================
// PROVIDER-SPECIFIC TOOL SCHEMAS
// ============================================================================
// Schema converters for different LLM providers:
// - OpenAI (GPT-4, GPT-3.5)
// - Anthropic (Claude)
// - Llama (Meta Llama models)

use crate::tool_executor::ToolDefinition;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// ============================================================================
// PROVIDER ENUM
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    OpenAI,
    Anthropic,
    Llama,
}

// ============================================================================
// OPENAI FORMAT
// ============================================================================

/// OpenAI function calling schema
/// Reference: https://platform.openai.com/docs/guides/function-calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameters schema (JSON Schema)
    pub parameters: JsonValue,
}

impl From<&ToolDefinition> for OpenAIFunction {
    fn from(tool: &ToolDefinition) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        }
    }
}

/// OpenAI tools format (newer API)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    /// Type is always "function"
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition
    pub function: OpenAIFunction,
}

impl From<&ToolDefinition> for OpenAITool {
    fn from(tool: &ToolDefinition) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: OpenAIFunction::from(tool),
        }
    }
}

// ============================================================================
// ANTHROPIC FORMAT
// ============================================================================

/// Anthropic (Claude) tool schema
/// Reference: https://docs.anthropic.com/claude/docs/tool-use
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: JsonValue,
}

impl From<&ToolDefinition> for AnthropicTool {
    fn from(tool: &ToolDefinition) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        }
    }
}

// ============================================================================
// LLAMA FORMAT
// ============================================================================

/// Llama function calling schema (compatible with Llama 3.1+)
/// Uses a simplified format similar to OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaFunction {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameters (JSON Schema)
    pub parameters: JsonValue,
}

impl From<&ToolDefinition> for LlamaFunction {
    fn from(tool: &ToolDefinition) -> Self {
        Self {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        }
    }
}

/// Llama tool wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaTool {
    /// Type is always "function"
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function definition
    pub function: LlamaFunction,
}

impl From<&ToolDefinition> for LlamaTool {
    fn from(tool: &ToolDefinition) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: LlamaFunction::from(tool),
        }
    }
}

// ============================================================================
// SCHEMA CONVERTER
// ============================================================================

/// Schema converter for different providers
pub struct SchemaConverter;

impl SchemaConverter {
    /// Convert tool definition to provider-specific format
    pub fn convert_tool(tool: &ToolDefinition, provider: Provider) -> JsonValue {
        match provider {
            Provider::OpenAI => {
                let openai_tool = OpenAITool::from(tool);
                serde_json::to_value(openai_tool).unwrap_or(JsonValue::Null)
            }
            Provider::Anthropic => {
                let anthropic_tool = AnthropicTool::from(tool);
                serde_json::to_value(anthropic_tool).unwrap_or(JsonValue::Null)
            }
            Provider::Llama => {
                let llama_tool = LlamaTool::from(tool);
                serde_json::to_value(llama_tool).unwrap_or(JsonValue::Null)
            }
        }
    }

    /// Convert multiple tools to provider-specific format
    pub fn convert_tools(tools: &[ToolDefinition], provider: Provider) -> Vec<JsonValue> {
        tools
            .iter()
            .map(|tool| Self::convert_tool(tool, provider))
            .collect()
    }

    /// Convert tool definition to provider-specific JSON string
    pub fn convert_tool_to_json(
        tool: &ToolDefinition,
        provider: Provider,
    ) -> Result<String, serde_json::Error> {
        let json_value = Self::convert_tool(tool, provider);
        serde_json::to_string_pretty(&json_value)
    }

    /// Convert tools to formatted JSON for provider
    pub fn convert_tools_to_json(
        tools: &[ToolDefinition],
        provider: Provider,
    ) -> Result<String, serde_json::Error> {
        let json_values = Self::convert_tools(tools, provider);
        serde_json::to_string_pretty(&json_values)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Create a basic JSON Schema for a tool parameter
pub fn create_string_param(description: &str) -> JsonValue {
    serde_json::json!({
        "type": "string",
        "description": description
    })
}

/// Create a number parameter schema
pub fn create_number_param(description: &str) -> JsonValue {
    serde_json::json!({
        "type": "number",
        "description": description
    })
}

/// Create an object parameter schema
pub fn create_object_param(
    description: &str,
    properties: JsonValue,
    required: Vec<&str>,
) -> JsonValue {
    serde_json::json!({
        "type": "object",
        "description": description,
        "properties": properties,
        "required": required
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tool() -> ToolDefinition {
        ToolDefinition {
            name: "get_weather".to_string(),
            description: "Get the current weather for a location".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"]
                    }
                },
                "required": ["location"]
            }),
            output_schema: None,
        }
    }

    #[test]
    fn test_openai_conversion() {
        let tool = create_test_tool();
        let openai_tool = OpenAITool::from(&tool);

        assert_eq!(openai_tool.tool_type, "function");
        assert_eq!(openai_tool.function.name, "get_weather");
        assert_eq!(
            openai_tool.function.description,
            "Get the current weather for a location"
        );

        // Verify schema structure
        assert!(openai_tool.function.parameters.is_object());
    }

    #[test]
    fn test_anthropic_conversion() {
        let tool = create_test_tool();
        let anthropic_tool = AnthropicTool::from(&tool);

        assert_eq!(anthropic_tool.name, "get_weather");
        assert_eq!(
            anthropic_tool.description,
            "Get the current weather for a location"
        );
        assert!(anthropic_tool.input_schema.is_object());
    }

    #[test]
    fn test_llama_conversion() {
        let tool = create_test_tool();
        let llama_tool = LlamaTool::from(&tool);

        assert_eq!(llama_tool.tool_type, "function");
        assert_eq!(llama_tool.function.name, "get_weather");
        assert_eq!(
            llama_tool.function.description,
            "Get the current weather for a location"
        );
        assert!(llama_tool.function.parameters.is_object());
    }

    #[test]
    fn test_schema_converter() {
        let tool = create_test_tool();

        // Test OpenAI conversion
        let openai_json = SchemaConverter::convert_tool(&tool, Provider::OpenAI);
        assert!(openai_json.is_object());
        assert_eq!(openai_json["type"], "function");

        // Test Anthropic conversion
        let anthropic_json = SchemaConverter::convert_tool(&tool, Provider::Anthropic);
        assert!(anthropic_json.is_object());
        assert_eq!(anthropic_json["name"], "get_weather");

        // Test Llama conversion
        let llama_json = SchemaConverter::convert_tool(&tool, Provider::Llama);
        assert!(llama_json.is_object());
        assert_eq!(llama_json["type"], "function");
    }

    #[test]
    fn test_convert_multiple_tools() {
        let tools = vec![
            ToolDefinition {
                name: "tool1".to_string(),
                description: "First tool".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            },
            ToolDefinition {
                name: "tool2".to_string(),
                description: "Second tool".to_string(),
                input_schema: serde_json::json!({}),
                output_schema: None,
            },
        ];

        let converted = SchemaConverter::convert_tools(&tools, Provider::OpenAI);
        assert_eq!(converted.len(), 2);
    }

    #[test]
    fn test_json_string_conversion() {
        let tool = create_test_tool();

        // Test JSON string conversion
        let json_str = SchemaConverter::convert_tool_to_json(&tool, Provider::OpenAI).unwrap();
        assert!(json_str.contains("get_weather"));
        assert!(json_str.contains("function"));
    }

    #[test]
    fn test_helper_functions() {
        let string_param = create_string_param("A test string");
        assert_eq!(string_param["type"], "string");

        let number_param = create_number_param("A test number");
        assert_eq!(number_param["type"], "number");

        let object_param = create_object_param(
            "An object",
            serde_json::json!({
                "field": { "type": "string" }
            }),
            vec!["field"],
        );
        assert_eq!(object_param["type"], "object");
        assert!(object_param["required"].is_array());
    }
}
