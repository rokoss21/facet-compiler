//! FACET v2.0 Renderer
//!
//! Converts Token Box Model allocation results into canonical JSON format
//! suitable for LLM providers.

use chrono::Utc;
use fct_ast::FacetDocument;
use fct_engine::AllocationResult;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during rendering
#[derive(Error, Debug)]
pub enum RenderError {
    #[error("Invalid section type: {0}")]
    InvalidSectionType(String),

    #[error("Missing required field: {0}")]
    MissingRequiredField(String),

    #[error("JSON serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Value conversion error: {0}")]
    ConversionError(String),
}

/// Main renderer for converting FACET documents to canonical JSON
pub struct Renderer {
    // Future configuration options can go here
}

impl Renderer {
    /// Create a new renderer instance
    pub fn new() -> Self {
        Self {}
    }

    /// Render allocation result to canonical JSON payload
    pub fn render(
        &self,
        document: &FacetDocument,
        allocation: &AllocationResult,
    ) -> Result<CanonicalPayload, RenderError> {
        let created_at = Utc::now().to_rfc3339();
        let mut doc_name = "facet_document".to_string();
        // Try to extract name/version from @meta attributes if available
        for node in &document.blocks {
            if let fct_ast::FacetNode::Meta(meta) = node {
                if let Some(fct_ast::ValueNode::String(name)) = meta.attributes.get("name") {
                    doc_name = name.clone();
                }
            }
        }

        let mut payload = CanonicalPayload {
            metadata: Metadata {
                name: doc_name,
                version: "2.0".to_string(),
                created_at,
                total_tokens: allocation.total_size,
                budget: allocation.budget,
                overflow: allocation.overflow,
            },
            system: Vec::new(),
            tools: Vec::new(),
            examples: Vec::new(),
            history: Vec::new(),
            user: Vec::new(),
            assistant: Vec::new(),
        };

        // Process allocated sections in canonical order
        for section_result in &allocation.sections {
            let section = &section_result.section;

            match section.id.as_str() {
                "system" => {
                    if section_result.final_size > 0 {
                        payload.system.push(ContentBlock {
                            role: "system".to_string(),
                            content: render_value_node(&section.content)?,
                            tokens: section_result.final_size,
                        });
                    }
                }
                "tools" => {
                    // Handle tool definitions from @interface blocks
                    if section_result.final_size > 0 {
                        // This would be populated from @interface blocks in the document
                        // For now, we'll implement basic structure
                    }
                }
                "user" => {
                    if section_result.final_size > 0 {
                        payload.user.push(ContentBlock {
                            role: "user".to_string(),
                            content: render_value_node(&section.content)?,
                            tokens: section_result.final_size,
                        });
                    }
                }
                "assistant" => {
                    if section_result.final_size > 0 {
                        payload.assistant.push(ContentBlock {
                            role: "assistant".to_string(),
                            content: render_value_node(&section.content)?,
                            tokens: section_result.final_size,
                        });
                    }
                }
                // Handle other section types as needed
                _ => {
                    // For unknown sections, we could either skip or add to user content
                    if section_result.final_size > 0 {
                        payload.user.push(ContentBlock {
                            role: "user".to_string(),
                            content: render_value_node(&section.content)?,
                            tokens: section_result.final_size,
                        });
                    }
                }
            }
        }

        // Extract tool definitions from @interface blocks
        payload.tools = extract_tools(document)?;

        Ok(payload)
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Canonical JSON payload structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CanonicalPayload {
    /// Metadata about the render
    pub metadata: Metadata,

    /// System messages and instructions
    pub system: Vec<ContentBlock>,

    /// Tool/function definitions
    pub tools: Vec<ToolDefinition>,

    /// Example interactions
    pub examples: Vec<Example>,

    /// Conversation history
    pub history: Vec<ContentBlock>,

    /// User input/content
    pub user: Vec<ContentBlock>,

    /// Assistant instructions/responses
    pub assistant: Vec<ContentBlock>,
}

/// Metadata about the canonical payload
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    /// Name of the FACET document
    pub name: String,

    /// FACET version
    pub version: String,

    /// Creation timestamp
    pub created_at: String,

    /// Total tokens used
    pub total_tokens: usize,

    /// Token budget that was allocated
    pub budget: usize,

    /// Overflow amount (0 if within budget)
    pub overflow: usize,
}

/// A content block with role and message
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentBlock {
    /// Role (system, user, assistant)
    pub role: String,

    /// Content of the message
    pub content: Content,

    /// Number of tokens allocated
    pub tokens: usize,
}

/// Content that can be text or multimodal
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Content {
    /// Simple text content
    Text(String),

    /// Multimodal content
    Multimodal(Vec<MultimodalItem>),
}

/// Multimodal content item
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MultimodalItem {
    /// Type of content (text, image, audio, etc.)
    #[serde(rename = "type")]
    pub item_type: String,

    /// The actual content
    pub content: serde_json::Value,

    /// Optional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Tool definition following OpenAI function calling format
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolDefinition {
    /// Type of tool (always "function" for now)
    #[serde(rename = "type")]
    pub tool_type: String,

    /// Function definition
    pub function: FunctionDefinition,
}

/// Function definition for tool calling
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FunctionDefinition {
    /// Name of the function
    pub name: String,

    /// Description of what the function does
    pub description: String,

    /// Parameters schema
    pub parameters: ParametersSchema,
}

/// Parameters schema for functions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParametersSchema {
    /// Type (always "object")
    #[serde(rename = "type")]
    pub schema_type: String,

    /// Property definitions
    pub properties: serde_json::Map<String, serde_json::Value>,

    /// Required parameters
    pub required: Vec<String>,
}

/// Example interaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Example {
    /// User input in example
    pub user: Content,

    /// Assistant response in example
    pub assistant: Content,
}

/// Render a ValueNode to Content
fn render_value_node(value: &fct_ast::ValueNode) -> Result<Content, RenderError> {
    match value {
        fct_ast::ValueNode::String(s) => Ok(Content::Text(s.clone())),
        fct_ast::ValueNode::Scalar(s) => Ok(Content::Text(scalar_to_string(s))),
        fct_ast::ValueNode::List(items) => {
            let multimodal_items = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let content_str = match render_value_node(item) {
                        Ok(Content::Text(s)) => s,
                        Ok(content) => format!("{:?}", content), // Fallback for non-text content
                        Err(e) => format!("Error: {}", e),
                    };
                    MultimodalItem {
                        item_type: "text".to_string(),
                        content: serde_json::Value::String(content_str),
                        metadata: Some(serde_json::json!({
                            "index": i,
                            "source": "facet_list"
                        })),
                    }
                })
                .collect();
            Ok(Content::Multimodal(multimodal_items))
        }
        fct_ast::ValueNode::Map(map) => {
            // Convert map to JSON string for now
            let json_str = serde_json::to_string(map)
                .map_err(|e| RenderError::ConversionError(e.to_string()))?;
            Ok(Content::Text(json_str))
        }
        fct_ast::ValueNode::Variable(_) => {
            // Variables should be resolved by the engine before rendering
            Err(RenderError::ConversionError(
                "Variable found in render stage - should be resolved".to_string(),
            ))
        }
        fct_ast::ValueNode::Pipeline(_) => {
            // Pipelines should be executed by the engine before rendering
            Err(RenderError::ConversionError(
                "Pipeline found in render stage - should be executed".to_string(),
            ))
        }
        fct_ast::ValueNode::Directive(directive) => {
            // For directives, serialize to JSON format
            let directive_json = serde_json::to_string(directive)
                .map_err(|e| RenderError::ConversionError(e.to_string()))?;
            Ok(Content::Text(directive_json))
        }
    }
}

/// Convert scalar value to string
fn scalar_to_string(scalar: &fct_ast::ScalarValue) -> String {
    match scalar {
        fct_ast::ScalarValue::Int(i) => i.to_string(),
        fct_ast::ScalarValue::Float(f) => f.to_string(),
        fct_ast::ScalarValue::Bool(b) => b.to_string(),
        fct_ast::ScalarValue::Null => "null".to_string(),
    }
}

/// Extract tool definitions from @interface blocks in the document
fn extract_tools(document: &FacetDocument) -> Result<Vec<ToolDefinition>, RenderError> {
    let mut tools = Vec::new();

    // Find all @interface blocks
    for node in &document.blocks {
        if let fct_ast::FacetNode::Interface(interface_block) = node {
            // Convert FACET interface to OpenAI function format
            for function in &interface_block.functions {
                let mut properties = serde_json::Map::new();
                let mut required = Vec::new();

                for param in &function.params {
                    // Convert FACET parameter to JSON schema property
                    let param_schema = type_node_to_json_schema(&param.type_node)?;
                    properties.insert(param.name.clone(), param_schema);

                    // For now, assume all parameters are required
                    // In a real implementation, we'd handle optional parameters
                    required.push(param.name.clone());
                }

                let tool = ToolDefinition {
                    tool_type: "function".to_string(),
                    function: FunctionDefinition {
                        name: function.name.clone(),
                        description: format!("Function: {}", function.name),
                        parameters: ParametersSchema {
                            schema_type: "object".to_string(),
                            properties,
                            required,
                        },
                    },
                };
                tools.push(tool);
            }
        }
    }

    Ok(tools)
}

/// Convert FACET TypeNode to JSON schema
fn type_node_to_json_schema(
    type_node: &fct_ast::TypeNode,
) -> Result<serde_json::Value, RenderError> {
    match type_node {
        fct_ast::TypeNode::Primitive(primitive) => {
            let schema = match primitive.as_str() {
                "String" => Ok(serde_json::json!({"type": "string"})),
                "Int" => Ok(serde_json::json!({"type": "integer"})),
                "Float" => Ok(serde_json::json!({"type": "number"})),
                "Bool" => Ok(serde_json::json!({"type": "boolean"})),
                "Null" => Ok(serde_json::json!({"type": "null"})),
                _ => Ok(serde_json::json!({"type": "string"})), // Default to string
            };
            schema
        }
        // Complex types not supported in rendering yet
        fct_ast::TypeNode::Struct(_) | fct_ast::TypeNode::List(_) |
        fct_ast::TypeNode::Map(_) | fct_ast::TypeNode::Union(_) |
        fct_ast::TypeNode::Image { .. } | fct_ast::TypeNode::Audio { .. } |
        fct_ast::TypeNode::Embedding { .. } => {
            // For now, render complex types as their string representation
            Ok(serde_json::json!({"type": "complex", "description": format!("{:?}", type_node)}))
        }
    }
}

/// Convert canonical payload to JSON string
pub fn to_json_string(payload: &CanonicalPayload) -> Result<String, RenderError> {
    serde_json::to_string_pretty(payload).map_err(RenderError::SerializationError)
}

/// Convert canonical payload to compact JSON string
pub fn to_json_compact(payload: &CanonicalPayload) -> Result<String, RenderError> {
    serde_json::to_string(payload).map_err(RenderError::SerializationError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fct_ast::{Span, ValueNode};
    use fct_engine::{AllocatedSection, AllocationResult, Section};

    #[test]
    fn test_simple_render() {
        let renderer = Renderer::new();

        // Create a simple test document
        let document = FacetDocument {
            blocks: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        // Create test allocation result
        let allocation = AllocationResult {
            sections: vec![AllocatedSection {
                final_size: 50,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section: Section::new(
                    "system".to_string(),
                    ValueNode::String("You are a helpful assistant".to_string()),
                    50,
                ),
            }],
            total_size: 50,
            budget: 100,
            overflow: 0,
        };

        let result = renderer.render(&document, &allocation).unwrap();

        assert_eq!(result.metadata.name, "facet_document");
        assert_eq!(result.system.len(), 1);
        assert_eq!(result.system[0].role, "system");
        assert_eq!(result.system[0].tokens, 50);
    }

    #[test]
    fn test_json_serialization() {
        let payload = CanonicalPayload {
            metadata: Metadata {
                name: "test".to_string(),
                version: "2.0".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                total_tokens: 100,
                budget: 150,
                overflow: 0,
            },
            system: vec![],
            tools: vec![],
            examples: vec![],
            history: vec![],
            user: vec![],
            assistant: vec![],
        };

        let json = to_json_string(&payload).unwrap();

        // Should be valid JSON
        let parsed: CanonicalPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.metadata.name, "test");
    }
}
