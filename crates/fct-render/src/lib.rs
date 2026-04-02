//! FACET v2.1.3 Renderer
//!
//! Converts Token Box Model allocation results into canonical JSON format
//! suitable for LLM providers.

use fct_ast::{
    BodyNode, FacetDocument, FacetNode, OrderedMap, ScalarValue, ValueNode, FACET_VERSION,
    POLICY_VERSION,
};
use fct_engine::AllocationResult;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
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

    #[error("F455: Guard undecidable for operation: {name}")]
    GuardUndecidable { name: String },
}

#[derive(Debug, Clone, Default)]
pub struct RenderContext {
    pub document_hash: Option<String>,
    pub policy_hash: Option<String>,
    pub profile: Option<String>,
    pub mode: Option<String>,
    pub host_profile_id: Option<String>,
    pub budget_units: Option<usize>,
    pub target_provider_id: Option<String>,
    pub computed_vars: Option<HashMap<String, ValueNode>>,
}

#[derive(Debug, Clone)]
pub struct RenderOutput {
    pub payload: CanonicalPayload,
    pub guard_decisions: Vec<GuardDecision>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GuardDecision {
    pub seq: usize,
    pub op: String,
    pub name: String,
    pub effect_class: Option<String>,
    pub mode: String,
    pub decision: String,
    pub policy_rule_id: Option<String>,
    pub input_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
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
        self.render_with_context(document, allocation, RenderContext::default())
    }

    pub fn render_with_context(
        &self,
        document: &FacetDocument,
        allocation: &AllocationResult,
        context: RenderContext,
    ) -> Result<CanonicalPayload, RenderError> {
        let output = self.render_with_trace(document, allocation, context)?;
        Ok(output.payload)
    }

    pub fn render_with_trace(
        &self,
        document: &FacetDocument,
        allocation: &AllocationResult,
        context: RenderContext,
    ) -> Result<RenderOutput, RenderError> {
        let document_hash = match context.document_hash {
            Some(hash) => hash,
            None => fallback_document_hash(document)?,
        };
        let mode = context.mode.unwrap_or_else(|| "exec".to_string());
        let profile = context.profile.unwrap_or_else(|| "hypervisor".to_string());
        let host_profile_id = context
            .host_profile_id
            .unwrap_or_else(|| "local.default.v1".to_string());
        let budget_units = context.budget_units.unwrap_or(allocation.budget);
        let target_provider_id = context
            .target_provider_id
            .unwrap_or_else(|| "unknown-provider".to_string());
        let computed_vars = context.computed_vars;

        let policy_hash = if let Some(hash) = context.policy_hash {
            Some(hash)
        } else {
            compute_policy_hash(document)?
        };

        let mut payload = CanonicalPayload {
            metadata: Metadata {
                facet_version: FACET_VERSION.to_string(),
                profile,
                mode: mode.clone(),
                host_profile_id: host_profile_id.clone(),
                policy_version: POLICY_VERSION.to_string(),
                document_hash,
                policy_hash,
                budget_units,
                target_provider_id,
            },
            tools: Vec::new(),
            messages: Vec::new(),
        };

        let mut system_messages: Vec<CanonicalMessage> = Vec::new();
        let mut user_messages: Vec<CanonicalMessage> = Vec::new();
        let mut assistant_messages: Vec<CanonicalMessage> = Vec::new();
        let effective_policy = collect_effective_policy(document);
        let mut message_guard_decisions: Vec<GuardDecision> = Vec::new();

        // Process allocated sections in canonical order
        for section_result in &allocation.sections {
            let section = &section_result.section;

            match section_role_for_section(section) {
                Some("system") => {
                    if section_result.final_size > 0 {
                        let guard = evaluate_message_emit(
                            effective_policy.as_ref(),
                            &section.id,
                            &mode,
                            computed_vars.as_ref(),
                        )?;
                        let input_hash =
                            message_emit_input_hash(&section.id, "system", &host_profile_id)?;
                        message_guard_decisions.push(GuardDecision {
                            seq: 0,
                            op: "message_emit".to_string(),
                            name: section.id.clone(),
                            effect_class: None,
                            mode: mode.clone(),
                            decision: if guard.allowed {
                                "allowed".to_string()
                            } else {
                                "denied".to_string()
                            },
                            policy_rule_id: guard.policy_rule_id,
                            input_hash,
                            error_code: guard.error_code.clone(),
                        });
                        if guard.error_code.as_deref() == Some("F455") {
                            return Err(RenderError::GuardUndecidable {
                                name: section.id.clone(),
                            });
                        }
                        if !guard.allowed {
                            continue;
                        }

                        let content = render_value_node(&section.content)?;
                        system_messages.push(CanonicalMessage {
                            role: "system".to_string(),
                            content,
                        });
                    }
                }
                Some("tools") => {
                    // Handle tool definitions from @interface blocks
                    if section_result.final_size > 0 {
                        // This would be populated from @interface blocks in the document
                        // For now, we'll implement basic structure
                    }
                }
                Some("user") => {
                    if section_result.final_size > 0 {
                        let guard = evaluate_message_emit(
                            effective_policy.as_ref(),
                            &section.id,
                            &mode,
                            computed_vars.as_ref(),
                        )?;
                        let input_hash =
                            message_emit_input_hash(&section.id, "user", &host_profile_id)?;
                        message_guard_decisions.push(GuardDecision {
                            seq: 0,
                            op: "message_emit".to_string(),
                            name: section.id.clone(),
                            effect_class: None,
                            mode: mode.clone(),
                            decision: if guard.allowed {
                                "allowed".to_string()
                            } else {
                                "denied".to_string()
                            },
                            policy_rule_id: guard.policy_rule_id,
                            input_hash,
                            error_code: guard.error_code.clone(),
                        });
                        if guard.error_code.as_deref() == Some("F455") {
                            return Err(RenderError::GuardUndecidable {
                                name: section.id.clone(),
                            });
                        }
                        if !guard.allowed {
                            continue;
                        }

                        let content = render_value_node(&section.content)?;
                        user_messages.push(CanonicalMessage {
                            role: "user".to_string(),
                            content,
                        });
                    }
                }
                Some("assistant") => {
                    if section_result.final_size > 0 {
                        let guard = evaluate_message_emit(
                            effective_policy.as_ref(),
                            &section.id,
                            &mode,
                            computed_vars.as_ref(),
                        )?;
                        let input_hash =
                            message_emit_input_hash(&section.id, "assistant", &host_profile_id)?;
                        message_guard_decisions.push(GuardDecision {
                            seq: 0,
                            op: "message_emit".to_string(),
                            name: section.id.clone(),
                            effect_class: None,
                            mode: mode.clone(),
                            decision: if guard.allowed {
                                "allowed".to_string()
                            } else {
                                "denied".to_string()
                            },
                            policy_rule_id: guard.policy_rule_id,
                            input_hash,
                            error_code: guard.error_code.clone(),
                        });
                        if guard.error_code.as_deref() == Some("F455") {
                            return Err(RenderError::GuardUndecidable {
                                name: section.id.clone(),
                            });
                        }
                        if !guard.allowed {
                            continue;
                        }

                        let content = render_value_node(&section.content)?;
                        assistant_messages.push(CanonicalMessage {
                            role: "assistant".to_string(),
                            content,
                        });
                    }
                }
                _ => {
                    // Unknown sections are treated as user-role content.
                    if section_result.final_size > 0 {
                        let guard = evaluate_message_emit(
                            effective_policy.as_ref(),
                            &section.id,
                            &mode,
                            computed_vars.as_ref(),
                        )?;
                        let input_hash =
                            message_emit_input_hash(&section.id, "user", &host_profile_id)?;
                        message_guard_decisions.push(GuardDecision {
                            seq: 0,
                            op: "message_emit".to_string(),
                            name: section.id.clone(),
                            effect_class: None,
                            mode: mode.clone(),
                            decision: if guard.allowed {
                                "allowed".to_string()
                            } else {
                                "denied".to_string()
                            },
                            policy_rule_id: guard.policy_rule_id,
                            input_hash,
                            error_code: guard.error_code.clone(),
                        });
                        if guard.error_code.as_deref() == Some("F455") {
                            return Err(RenderError::GuardUndecidable {
                                name: section.id.clone(),
                            });
                        }
                        if !guard.allowed {
                            continue;
                        }

                        let content = render_value_node(&section.content)?;
                        user_messages.push(CanonicalMessage {
                            role: "user".to_string(),
                            content,
                        });
                    }
                }
            }
        }

        // v2.1.3 canonical role ordering: system -> user -> assistant.
        payload.messages.extend(system_messages);
        payload.messages.extend(user_messages);
        payload.messages.extend(assistant_messages);

        // Extract tool definitions from @interface blocks
        let (tools, mut tool_guard_decisions) =
            extract_tools_with_guard(document, &mode, &host_profile_id, computed_vars.as_ref())?;
        payload.tools = tools;
        let mut guard_decisions = Vec::new();
        guard_decisions.append(&mut message_guard_decisions);
        guard_decisions.append(&mut tool_guard_decisions);
        for (idx, decision) in guard_decisions.iter_mut().enumerate() {
            decision.seq = idx + 1;
        }

        Ok(RenderOutput {
            payload,
            guard_decisions,
        })
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

    /// Tool/function definitions
    pub tools: Vec<ToolDefinition>,

    /// Canonical ordered messages (v2.1.3 model)
    pub messages: Vec<CanonicalMessage>,
}

/// Metadata about the canonical payload
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Metadata {
    /// FACET version (normative key in v2.1.3)
    pub facet_version: String,

    /// Active profile
    pub profile: String,

    /// Active execution mode
    pub mode: String,

    /// Host profile identifier
    pub host_profile_id: String,

    /// Policy DSL/guard semantic version
    pub policy_version: String,

    /// SHA-256 of the normalized source form (provided by caller or fallback hash)
    pub document_hash: String,

    /// Effective policy hash if policy is present
    pub policy_hash: Option<String>,

    /// Effective budget in FACET Units
    pub budget_units: usize,

    /// Target provider identifier
    pub target_provider_id: String,
}

fn fallback_document_hash(document: &FacetDocument) -> Result<String, RenderError> {
    let bytes = serde_json::to_vec(document)?;
    let hash = Sha256::digest(bytes);
    Ok(format!("{:x}", hash))
}

fn compute_policy_hash(document: &FacetDocument) -> Result<Option<String>, RenderError> {
    let effective_policy = collect_effective_policy(document);
    let Some(policy_map) = effective_policy else {
        return Ok(None);
    };

    let policy_json = ordered_map_to_json(&policy_map)?;
    let envelope = serde_json::json!({
        "policy_version": POLICY_VERSION,
        "policy": policy_json,
    });
    let canonical = canonicalize_json(&envelope)?;
    let hash = Sha256::digest(canonical.as_bytes());
    Ok(Some(format!("sha256:{:x}", hash)))
}

/// Compute v2.1.3 `policy_hash` for a resolved document.
pub fn policy_hash_for_document(document: &FacetDocument) -> Result<Option<String>, RenderError> {
    compute_policy_hash(document)
}

/// Return effective merged `@policy` as JSON (or `None` when absent).
pub fn effective_policy_json_for_document(
    document: &FacetDocument,
) -> Result<Option<serde_json::Value>, RenderError> {
    let Some(policy_map) = collect_effective_policy(document) else {
        return Ok(None);
    };
    Ok(Some(ordered_map_to_json(&policy_map)?))
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
                        let merged =
                            merge_policy_value(&kv.key, existing.clone(), kv.value.clone());
                        *existing = merged;
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

fn ordered_map_to_json(
    map: &OrderedMap<String, ValueNode>,
) -> Result<serde_json::Value, RenderError> {
    let mut out = serde_json::Map::new();
    for (k, v) in map {
        out.insert(k.clone(), value_node_to_json(v)?);
    }
    Ok(serde_json::Value::Object(out))
}

fn value_node_to_json(value: &ValueNode) -> Result<serde_json::Value, RenderError> {
    match value {
        ValueNode::Scalar(ScalarValue::Int(v)) => Ok(serde_json::json!(v)),
        ValueNode::Scalar(ScalarValue::Float(v)) => Ok(serde_json::json!(v)),
        ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(serde_json::json!(v)),
        ValueNode::Scalar(ScalarValue::Null) => Ok(serde_json::Value::Null),
        ValueNode::String(v) => Ok(serde_json::json!(v)),
        ValueNode::Variable(v) => Ok(serde_json::json!(format!("${v}"))),
        ValueNode::Directive(d) => serde_json::to_value(d).map_err(RenderError::SerializationError),
        ValueNode::Pipeline(p) => serde_json::to_value(p).map_err(RenderError::SerializationError),
        ValueNode::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(value_node_to_json(item)?);
            }
            Ok(serde_json::Value::Array(out))
        }
        ValueNode::Map(map) => ordered_map_to_json(map),
    }
}

fn canonicalize_json(value: &serde_json::Value) -> Result<String, RenderError> {
    serde_json_canonicalizer::to_string(value).map_err(RenderError::SerializationError)
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

/// Canonical message entry (provider-agnostic)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CanonicalMessage {
    pub role: String,
    pub content: Content,
}

/// Content that can be text or multimodal
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Content {
    /// Simple text content
    Text(String),

    /// Normative FACET content items
    Items(Vec<ContentItem>),
}

/// Canonical content item
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { asset: serde_json::Value },
    #[serde(rename = "audio")]
    Audio { asset: serde_json::Value },
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
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                let map = match item {
                    fct_ast::ValueNode::Map(map) => map,
                    _ => {
                        return Err(RenderError::ConversionError(
                            "Content list items must be maps with `type`".to_string(),
                        ))
                    }
                };

                let item_type = match map.get("type") {
                    Some(fct_ast::ValueNode::String(t)) => t.as_str(),
                    _ => {
                        return Err(RenderError::ConversionError(
                            "Content item requires string `type`".to_string(),
                        ))
                    }
                };

                match item_type {
                    "text" => {
                        let text = match map.get("text") {
                            Some(fct_ast::ValueNode::String(t)) => t.clone(),
                            _ => {
                                return Err(RenderError::ConversionError(
                                    "Text content item requires string `text`".to_string(),
                                ))
                            }
                        };
                        out.push(ContentItem::Text { text });
                    }
                    "image" => {
                        let asset = match map.get("asset") {
                            Some(fct_ast::ValueNode::Map(asset)) => ordered_map_to_json(asset)?,
                            _ => {
                                return Err(RenderError::ConversionError(
                                    "Image content item requires map `asset`".to_string(),
                                ))
                            }
                        };
                        out.push(ContentItem::Image { asset });
                    }
                    "audio" => {
                        let asset = match map.get("asset") {
                            Some(fct_ast::ValueNode::Map(asset)) => ordered_map_to_json(asset)?,
                            _ => {
                                return Err(RenderError::ConversionError(
                                    "Audio content item requires map `asset`".to_string(),
                                ))
                            }
                        };
                        out.push(ContentItem::Audio { asset });
                    }
                    other => {
                        return Err(RenderError::ConversionError(format!(
                            "Unsupported content item type `{}`",
                            other
                        )))
                    }
                }
            }
            Ok(Content::Items(out))
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

fn section_role(id: &str) -> Option<&str> {
    if id == "system" || id.starts_with("system#") {
        Some("system")
    } else if id == "tools" || id.starts_with("tools#") {
        Some("tools")
    } else if id == "user" || id.starts_with("user#") {
        Some("user")
    } else if id == "assistant" || id.starts_with("assistant#") {
        Some("assistant")
    } else {
        None
    }
}

fn section_role_for_section(section: &fct_engine::Section) -> Option<&str> {
    if let Some(role) = section.role.as_deref() {
        return match role {
            "system" | "tools" | "user" | "assistant" => Some(role),
            _ => section_role(&section.id),
        };
    }
    section_role(&section.id)
}

/// Extract tool definitions from @interface blocks in the document
#[allow(dead_code)]
fn extract_tools(document: &FacetDocument) -> Result<Vec<ToolDefinition>, RenderError> {
    let (tools, _) = extract_tools_with_guard(document, "exec", "local.default.v1", None)?;
    Ok(tools)
}

fn extract_tools_with_guard(
    document: &FacetDocument,
    mode: &str,
    host_profile_id: &str,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<(Vec<ToolDefinition>, Vec<GuardDecision>), RenderError> {
    let mut tools = Vec::new();
    let mut guard_decisions = Vec::new();
    let mut seq = 1usize;
    let mut referenced_interfaces = HashSet::new();
    let mut has_tool_refs = false;
    let effective_policy = collect_effective_policy(document);

    for node in &document.blocks {
        if let FacetNode::System(system_block) = node {
            for body in &system_block.body {
                let BodyNode::KeyValue(kv) = body else {
                    continue;
                };
                if kv.key != "tools" {
                    continue;
                }

                let entries = match &kv.value {
                    ValueNode::List(items) => items,
                    _ => {
                        return Err(RenderError::ConversionError(
                            "@system.tools must be a list".to_string(),
                        ))
                    }
                };
                has_tool_refs = true;
                for item in entries {
                    match item {
                        ValueNode::Variable(name) => {
                            referenced_interfaces.insert(name.clone());
                        }
                        _ => {
                            return Err(RenderError::ConversionError(
                                "@system.tools items must be interface refs".to_string(),
                            ))
                        }
                    }
                }
            }
        }
    }

    // Find all @interface blocks in resolved source order.
    for node in &document.blocks {
        if let fct_ast::FacetNode::Interface(interface_block) = node {
            if has_tool_refs && !referenced_interfaces.contains(&interface_block.name) {
                continue;
            }
            // Convert FACET interface to OpenAI function format
            for function in &interface_block.functions {
                let canonical_name = format!("{}.{}", interface_block.name, function.name);
                let guard = evaluate_tool_expose(
                    effective_policy.as_ref(),
                    &canonical_name,
                    function.effect.as_deref(),
                    mode,
                    computed_vars,
                )?;
                let input_hash = tool_expose_input_hash(&interface_block.name, host_profile_id)?;
                guard_decisions.push(GuardDecision {
                    seq,
                    op: "tool_expose".to_string(),
                    name: canonical_name.clone(),
                    effect_class: function.effect.clone(),
                    mode: mode.to_string(),
                    decision: if guard.allowed {
                        "allowed".to_string()
                    } else {
                        "denied".to_string()
                    },
                    policy_rule_id: guard.policy_rule_id,
                    input_hash,
                    error_code: guard.error_code.clone(),
                });
                seq += 1;

                if let Some(code) = guard.error_code.as_deref() {
                    if code == "F455" {
                        return Err(RenderError::GuardUndecidable {
                            name: canonical_name,
                        });
                    }
                }

                if !guard.allowed {
                    continue;
                }

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

    Ok((tools, guard_decisions))
}

/// Convert FACET TypeNode to JSON schema
fn type_node_to_json_schema(
    type_node: &fct_ast::TypeNode,
) -> Result<serde_json::Value, RenderError> {
    match type_node {
        fct_ast::TypeNode::Primitive(primitive) => {
            match primitive.as_str() {
                "string" | "String" => Ok(serde_json::json!({"type": "string"})),
                "int" | "Int" => Ok(serde_json::json!({"type": "integer"})),
                "float" | "Float" => Ok(serde_json::json!({"type": "number"})),
                "bool" | "Bool" => Ok(serde_json::json!({"type": "boolean"})),
                "null" | "Null" => Ok(serde_json::json!({"type": "null"})),
                "any" | "Any" => Ok(serde_json::json!({})),
                _ => Ok(serde_json::json!({"type": "string"})), // fallback for legacy unknown primitives
            }
        }
        fct_ast::TypeNode::Struct(fields) => {
            let mut properties = serde_json::Map::new();
            let mut required = Vec::new();
            for (field_name, field_type) in fields {
                properties.insert(field_name.clone(), type_node_to_json_schema(field_type)?);
                required.push(field_name.clone());
            }
            Ok(serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            }))
        }
        fct_ast::TypeNode::List(item_type) => Ok(serde_json::json!({
            "type": "array",
            "items": type_node_to_json_schema(item_type)?
        })),
        fct_ast::TypeNode::Map(value_type) => Ok(serde_json::json!({
            "type": "object",
            "additionalProperties": type_node_to_json_schema(value_type)?
        })),
        fct_ast::TypeNode::Union(types) => {
            let mut one_of = Vec::new();
            for ty in types {
                one_of.push(type_node_to_json_schema(ty)?);
            }
            Ok(serde_json::json!({
                "oneOf": one_of
            }))
        }
        fct_ast::TypeNode::Embedding { size } => Ok(serde_json::json!({
            "type": "array",
            "items": { "type": "number" },
            "minItems": size,
            "maxItems": size
        })),
        // Appendix D does not define image/audio mapping; keep deterministic fallback.
        fct_ast::TypeNode::Image { .. } | fct_ast::TypeNode::Audio { .. } => {
            // For now, render complex types as their string representation
            Ok(serde_json::json!({"type": "complex", "description": format!("{:?}", type_node)}))
        }
    }
}

#[derive(Debug, Clone)]
struct ToolExposeDecision {
    allowed: bool,
    policy_rule_id: Option<String>,
    error_code: Option<String>,
}

fn evaluate_tool_expose(
    policy: Option<&OrderedMap<String, ValueNode>>,
    tool_name: &str,
    effect_class: Option<&str>,
    _mode: &str,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<ToolExposeDecision, RenderError> {
    let Some(policy_map) = policy else {
        return Ok(ToolExposeDecision {
            allowed: true,
            policy_rule_id: None,
            error_code: None,
        });
    };

    if let Some(deny_rules) = policy_map.get("deny").and_then(as_rule_list) {
        for rule in deny_rules {
            match rule_matches_tool_expose(rule, tool_name, effect_class, computed_vars)? {
                RuleMatch::Matched(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F454".to_string()),
                    })
                }
                RuleMatch::Undecidable(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F455".to_string()),
                    })
                }
                RuleMatch::NoMatch => {}
            }
        }
    }

    if let Some(allow_rules) = policy_map.get("allow").and_then(as_rule_list) {
        for rule in allow_rules {
            match rule_matches_tool_expose(rule, tool_name, effect_class, computed_vars)? {
                RuleMatch::Matched(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: true,
                        policy_rule_id: rule_id,
                        error_code: None,
                    })
                }
                RuleMatch::Undecidable(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F455".to_string()),
                    })
                }
                RuleMatch::NoMatch => {}
            }
        }
    }

    default_guard_decision(policy_map, "tool_expose", true)
}

fn evaluate_message_emit(
    policy: Option<&OrderedMap<String, ValueNode>>,
    message_id: &str,
    _mode: &str,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<ToolExposeDecision, RenderError> {
    let Some(policy_map) = policy else {
        return Ok(ToolExposeDecision {
            allowed: true,
            policy_rule_id: None,
            error_code: None,
        });
    };

    if let Some(deny_rules) = policy_map.get("deny").and_then(as_rule_list) {
        for rule in deny_rules {
            match rule_matches_message_emit(rule, message_id, computed_vars)? {
                RuleMatch::Matched(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F454".to_string()),
                    })
                }
                RuleMatch::Undecidable(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F455".to_string()),
                    })
                }
                RuleMatch::NoMatch => {}
            }
        }
    }

    if let Some(allow_rules) = policy_map.get("allow").and_then(as_rule_list) {
        for rule in allow_rules {
            match rule_matches_message_emit(rule, message_id, computed_vars)? {
                RuleMatch::Matched(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: true,
                        policy_rule_id: rule_id,
                        error_code: None,
                    })
                }
                RuleMatch::Undecidable(rule_id) => {
                    return Ok(ToolExposeDecision {
                        allowed: false,
                        policy_rule_id: rule_id,
                        error_code: Some("F455".to_string()),
                    })
                }
                RuleMatch::NoMatch => {}
            }
        }
    }

    default_guard_decision(policy_map, "message_emit", true)
}

fn as_rule_list(value: &ValueNode) -> Option<&Vec<ValueNode>> {
    match value {
        ValueNode::List(items) => Some(items),
        _ => None,
    }
}

fn default_guard_decision(
    policy_map: &OrderedMap<String, ValueNode>,
    op: &str,
    fallback_allow: bool,
) -> Result<ToolExposeDecision, RenderError> {
    if let Some(defaults_node) = policy_map.get("defaults") {
        let defaults_map = match defaults_node {
            ValueNode::Map(map) => map,
            _ => {
                return Ok(ToolExposeDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F455".to_string()),
                })
            }
        };

        if let Some(op_default) = defaults_map.get(op) {
            return match op_default {
                ValueNode::String(s) if s == "allow" => Ok(ToolExposeDecision {
                    allowed: true,
                    policy_rule_id: None,
                    error_code: None,
                }),
                ValueNode::String(s) if s == "deny" => Ok(ToolExposeDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F454".to_string()),
                }),
                ValueNode::Scalar(ScalarValue::Bool(true)) => Ok(ToolExposeDecision {
                    allowed: true,
                    policy_rule_id: None,
                    error_code: None,
                }),
                ValueNode::Scalar(ScalarValue::Bool(false)) => Ok(ToolExposeDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F454".to_string()),
                }),
                _ => Ok(ToolExposeDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F455".to_string()),
                }),
            };
        }
    }

    Ok(ToolExposeDecision {
        allowed: fallback_allow,
        policy_rule_id: None,
        error_code: if fallback_allow {
            None
        } else {
            Some("F454".to_string())
        },
    })
}

enum RuleMatch {
    Matched(Option<String>),
    Undecidable(Option<String>),
    NoMatch,
}

fn rule_matches_tool_expose(
    rule: &ValueNode,
    tool_name: &str,
    tool_effect_class: Option<&str>,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<RuleMatch, RenderError> {
    let ValueNode::Map(map) = rule else {
        return Ok(RuleMatch::NoMatch);
    };

    let Some(ValueNode::String(op)) = map.get("op") else {
        return Ok(RuleMatch::NoMatch);
    };
    if op != "tool_expose" {
        return Ok(RuleMatch::NoMatch);
    }
    let rule_id = match map.get("id") {
        Some(ValueNode::String(id)) => Some(id.clone()),
        _ => None,
    };

    let name_match = match map.get("name") {
        Some(ValueNode::String(pattern)) => matcher_matches(pattern, tool_name),
        Some(_) => return Ok(RuleMatch::Undecidable(rule_id)),
        None => true,
    };
    if !name_match {
        return Ok(RuleMatch::NoMatch);
    }

    if let Some(effect_matcher) = map.get("effect") {
        match effect_matcher {
            ValueNode::String(pattern) => {
                let Some(effect_class) = tool_effect_class else {
                    return Ok(RuleMatch::NoMatch);
                };
                if !matcher_matches(pattern, effect_class) {
                    return Ok(RuleMatch::NoMatch);
                }
            }
            _ => return Ok(RuleMatch::Undecidable(rule_id)),
        }
    }

    let when_eval = match map.get("when") {
        None => true,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return Ok(RuleMatch::Undecidable(rule_id)),
        },
    };

    if !when_eval {
        return Ok(RuleMatch::NoMatch);
    }

    let unless_eval = match map.get("unless") {
        None => false,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return Ok(RuleMatch::Undecidable(rule_id)),
        },
    };

    if unless_eval {
        return Ok(RuleMatch::NoMatch);
    }

    Ok(RuleMatch::Matched(rule_id))
}

fn rule_matches_message_emit(
    rule: &ValueNode,
    message_id: &str,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<RuleMatch, RenderError> {
    let ValueNode::Map(map) = rule else {
        return Ok(RuleMatch::NoMatch);
    };

    let Some(ValueNode::String(op)) = map.get("op") else {
        return Ok(RuleMatch::NoMatch);
    };
    if op != "message_emit" {
        return Ok(RuleMatch::NoMatch);
    }
    let rule_id = match map.get("id") {
        Some(ValueNode::String(id)) => Some(id.clone()),
        _ => None,
    };

    let name_match = match map.get("name") {
        Some(ValueNode::String(pattern)) => matcher_matches(pattern, message_id),
        Some(_) => return Ok(RuleMatch::Undecidable(rule_id)),
        None => true,
    };
    if !name_match {
        return Ok(RuleMatch::NoMatch);
    }

    // message_emit OpDesc currently has effect_class = null; if rule requires effect,
    // the conjunctive match fails (must not match).
    if let Some(effect_matcher) = map.get("effect") {
        match effect_matcher {
            ValueNode::String(_pattern) => return Ok(RuleMatch::NoMatch),
            _ => return Ok(RuleMatch::Undecidable(rule_id)),
        }
    }

    let when_eval = match map.get("when") {
        None => true,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return Ok(RuleMatch::Undecidable(rule_id)),
        },
    };

    if !when_eval {
        return Ok(RuleMatch::NoMatch);
    }

    let unless_eval = match map.get("unless") {
        None => false,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return Ok(RuleMatch::Undecidable(rule_id)),
        },
    };

    if unless_eval {
        return Ok(RuleMatch::NoMatch);
    }

    Ok(RuleMatch::Matched(rule_id))
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
                    // Short-circuit: stop at first false.
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
                    // Short-circuit: stop at first true.
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

fn matcher_matches(pattern: &str, value: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix(".*") {
        value.starts_with(prefix)
    } else {
        pattern == value
    }
}

fn tool_expose_input_hash(
    interface_name: &str,
    host_profile_id: &str,
) -> Result<String, RenderError> {
    let input_obj = serde_json::json!({
        "interface": interface_name,
        "host_profile_id": host_profile_id,
        "facet_version": FACET_VERSION,
    });
    let canonical = canonicalize_json(&input_obj)?;
    let hash = Sha256::digest(canonical.as_bytes());
    Ok(format!("sha256:{:x}", hash))
}

fn message_emit_input_hash(
    message_id: &str,
    role: &str,
    host_profile_id: &str,
) -> Result<String, RenderError> {
    let input_obj = serde_json::json!({
        "message_id": message_id,
        "role": role,
        "host_profile_id": host_profile_id,
        "facet_version": FACET_VERSION,
    });
    let canonical = canonicalize_json(&input_obj)?;
    let hash = Sha256::digest(canonical.as_bytes());
    Ok(format!("sha256:{:x}", hash))
}

/// Convert canonical payload to JSON string
pub fn to_json_string(payload: &CanonicalPayload) -> Result<String, RenderError> {
    serde_json::to_string_pretty(payload).map_err(RenderError::SerializationError)
}

/// Convert canonical payload to compact JSON string
pub fn to_json_compact(payload: &CanonicalPayload) -> Result<String, RenderError> {
    let value = serde_json::to_value(payload).map_err(RenderError::SerializationError)?;
    canonicalize_json(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fct_ast::{
        BodyNode, FacetBlock, FacetNode, FunctionSignature, InterfaceNode, KeyValueNode,
        OrderedMap, Parameter, ScalarValue, Span, TypeNode, ValueNode,
    };
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

        assert_eq!(result.metadata.facet_version, FACET_VERSION);
        assert_eq!(result.metadata.policy_version, POLICY_VERSION);
        assert_eq!(result.metadata.profile, "hypervisor");
        assert_eq!(result.metadata.mode, "exec");
        assert!(!result.metadata.document_hash.is_empty());
        assert!(result.metadata.policy_hash.is_none());
        assert_eq!(result.metadata.budget_units, 100);
        assert_eq!(result.metadata.target_provider_id, "unknown-provider");
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].role, "system");
        assert!(matches!(
            &result.messages[0].content,
            Content::Text(s) if s == "You are a helpful assistant"
        ));
    }

    #[test]
    fn test_render_accepts_role_hash_section_ids() {
        let renderer = Renderer::new();
        let document = FacetDocument {
            blocks: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let allocation = AllocationResult {
            sections: vec![
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "system#1".to_string(),
                        ValueNode::String("sys".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "user#1".to_string(),
                        ValueNode::String("usr".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "assistant#1".to_string(),
                        ValueNode::String("asst".to_string()),
                        10,
                    ),
                },
            ],
            total_size: 30,
            budget: 100,
            overflow: 0,
        };

        let result = renderer.render(&document, &allocation).unwrap();
        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.messages[0].role, "system");
        assert_eq!(result.messages[1].role, "user");
        assert_eq!(result.messages[2].role, "assistant");
    }

    #[test]
    fn test_render_uses_explicit_section_role_when_id_is_custom() {
        let renderer = Renderer::new();
        let document = FacetDocument {
            blocks: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let allocation = AllocationResult {
            sections: vec![
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "sys.long".to_string(),
                        ValueNode::String("sys".to_string()),
                        10,
                    )
                    .with_role("system"),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "user.critical".to_string(),
                        ValueNode::String("usr".to_string()),
                        10,
                    )
                    .with_role("user"),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "assistant.flex".to_string(),
                        ValueNode::String("asst".to_string()),
                        10,
                    )
                    .with_role("assistant"),
                },
            ],
            total_size: 30,
            budget: 100,
            overflow: 0,
        };

        let result = renderer.render(&document, &allocation).unwrap();
        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.messages[0].role, "system");
        assert_eq!(result.messages[1].role, "user");
        assert_eq!(result.messages[2].role, "assistant");
    }

    #[test]
    fn test_render_preserves_canonical_multimodal_content_items() {
        let renderer = Renderer::new();
        let document = FacetDocument {
            blocks: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let image_asset = OrderedMap::from([
            ("kind".to_string(), ValueNode::String("image".to_string())),
            ("format".to_string(), ValueNode::String("jpeg".to_string())),
            (
                "digest".to_string(),
                ValueNode::Map(OrderedMap::from([
                    ("algo".to_string(), ValueNode::String("sha256".to_string())),
                    (
                        "value".to_string(),
                        ValueNode::String("deadbeef".to_string()),
                    ),
                ])),
            ),
            (
                "shape".to_string(),
                ValueNode::Map(OrderedMap::from([
                    ("width".to_string(), ValueNode::Scalar(ScalarValue::Int(64))),
                    (
                        "height".to_string(),
                        ValueNode::Scalar(ScalarValue::Int(64)),
                    ),
                ])),
            ),
        ]);

        let audio_asset = OrderedMap::from([
            ("kind".to_string(), ValueNode::String("audio".to_string())),
            ("format".to_string(), ValueNode::String("wav".to_string())),
            (
                "digest".to_string(),
                ValueNode::Map(OrderedMap::from([
                    ("algo".to_string(), ValueNode::String("sha256".to_string())),
                    (
                        "value".to_string(),
                        ValueNode::String("cafebabe".to_string()),
                    ),
                ])),
            ),
            (
                "shape".to_string(),
                ValueNode::Map(OrderedMap::from([(
                    "duration".to_string(),
                    ValueNode::Scalar(ScalarValue::Float(1.5)),
                )])),
            ),
        ]);

        let list_content = ValueNode::List(vec![
            ValueNode::Map(OrderedMap::from([
                ("type".to_string(), ValueNode::String("text".to_string())),
                (
                    "text".to_string(),
                    ValueNode::String("hello multimodal".to_string()),
                ),
            ])),
            ValueNode::Map(OrderedMap::from([
                ("type".to_string(), ValueNode::String("image".to_string())),
                ("asset".to_string(), ValueNode::Map(image_asset)),
            ])),
            ValueNode::Map(OrderedMap::from([
                ("type".to_string(), ValueNode::String("audio".to_string())),
                ("asset".to_string(), ValueNode::Map(audio_asset)),
            ])),
        ]);

        let allocation = AllocationResult {
            sections: vec![AllocatedSection {
                final_size: 100,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section: Section::new("user#1".to_string(), list_content, 100),
            }],
            total_size: 100,
            budget: 200,
            overflow: 0,
        };

        let result = renderer.render(&document, &allocation).unwrap();
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].role, "user");

        let Content::Items(items) = &result.messages[0].content else {
            panic!("expected content items");
        };
        assert_eq!(items.len(), 3);
        assert_eq!(
            items[0],
            ContentItem::Text {
                text: "hello multimodal".to_string()
            }
        );
        match &items[1] {
            ContentItem::Image { asset } => {
                assert_eq!(asset.get("kind").and_then(|v| v.as_str()), Some("image"));
                assert_eq!(asset.get("format").and_then(|v| v.as_str()), Some("jpeg"));
            }
            _ => panic!("expected image item"),
        }
        match &items[2] {
            ContentItem::Audio { asset } => {
                assert_eq!(asset.get("kind").and_then(|v| v.as_str()), Some("audio"));
                assert_eq!(asset.get("format").and_then(|v| v.as_str()), Some("wav"));
            }
            _ => panic!("expected audio item"),
        }
    }

    #[test]
    fn test_type_node_to_json_schema_primitives() {
        let cases = vec![
            (
                TypeNode::Primitive("string".to_string()),
                serde_json::json!({"type": "string"}),
            ),
            (
                TypeNode::Primitive("int".to_string()),
                serde_json::json!({"type": "integer"}),
            ),
            (
                TypeNode::Primitive("float".to_string()),
                serde_json::json!({"type": "number"}),
            ),
            (
                TypeNode::Primitive("bool".to_string()),
                serde_json::json!({"type": "boolean"}),
            ),
            (
                TypeNode::Primitive("null".to_string()),
                serde_json::json!({"type": "null"}),
            ),
            (
                TypeNode::Primitive("any".to_string()),
                serde_json::json!({}),
            ),
        ];

        for (ty, expected) in cases {
            let schema = type_node_to_json_schema(&ty).expect("primitive mapping should succeed");
            assert_eq!(schema, expected);
        }
    }

    #[test]
    fn test_type_node_to_json_schema_struct_list_map_union_embedding() {
        let struct_type = TypeNode::Struct(OrderedMap::from([
            (
                "name".to_string(),
                TypeNode::Primitive("string".to_string()),
            ),
            (
                "score".to_string(),
                TypeNode::Union(vec![
                    TypeNode::Primitive("float".to_string()),
                    TypeNode::Primitive("null".to_string()),
                ]),
            ),
            (
                "tags".to_string(),
                TypeNode::List(Box::new(TypeNode::Primitive("string".to_string()))),
            ),
            (
                "weights".to_string(),
                TypeNode::Map(Box::new(TypeNode::Primitive("float".to_string()))),
            ),
            ("vec".to_string(), TypeNode::Embedding { size: 3 }),
        ]));

        let schema = type_node_to_json_schema(&struct_type).expect("struct mapping should succeed");
        let expected = serde_json::json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "score": { "oneOf": [ { "type": "number" }, { "type": "null" } ] },
                "tags": { "type": "array", "items": { "type": "string" } },
                "weights": { "type": "object", "additionalProperties": { "type": "number" } },
                "vec": {
                    "type": "array",
                    "items": { "type": "number" },
                    "minItems": 3,
                    "maxItems": 3
                }
            },
            "required": ["name", "score", "tags", "weights", "vec"],
            "additionalProperties": false
        });

        assert_eq!(schema, expected);
    }

    #[test]
    fn test_render_messages_preserve_within_role_order() {
        let renderer = Renderer::new();
        let document = FacetDocument {
            blocks: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let allocation = AllocationResult {
            sections: vec![
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "user#1".to_string(),
                        ValueNode::String("u1".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "system#1".to_string(),
                        ValueNode::String("s1".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "user#2".to_string(),
                        ValueNode::String("u2".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "assistant#1".to_string(),
                        ValueNode::String("a1".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 10,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "system#2".to_string(),
                        ValueNode::String("s2".to_string()),
                        10,
                    ),
                },
            ],
            total_size: 50,
            budget: 100,
            overflow: 0,
        };

        let result = renderer.render(&document, &allocation).unwrap();
        assert_eq!(result.messages.len(), 5);
        // Canonical role ordering: all system, then user, then assistant.
        assert_eq!(result.messages[0].role, "system");
        assert!(matches!(&result.messages[0].content, Content::Text(s) if s == "s1"));
        assert_eq!(result.messages[1].role, "system");
        assert!(matches!(&result.messages[1].content, Content::Text(s) if s == "s2"));
        assert_eq!(result.messages[2].role, "user");
        assert!(matches!(&result.messages[2].content, Content::Text(s) if s == "u1"));
        assert_eq!(result.messages[3].role, "user");
        assert!(matches!(&result.messages[3].content, Content::Text(s) if s == "u2"));
        assert_eq!(result.messages[4].role, "assistant");
        assert!(matches!(&result.messages[4].content, Content::Text(s) if s == "a1"));
    }

    #[test]
    fn test_render_preserves_order_when_sections_are_dropped() {
        let renderer = Renderer::new();
        let document = FacetDocument {
            blocks: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let allocation = AllocationResult {
            sections: vec![
                AllocatedSection {
                    final_size: 0,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: true,
                    section: Section::new(
                        "user#1".to_string(),
                        ValueNode::String("u1".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 12,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "system#1".to_string(),
                        ValueNode::String("s1".to_string()),
                        12,
                    ),
                },
                AllocatedSection {
                    final_size: 0,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: true,
                    section: Section::new(
                        "assistant#1".to_string(),
                        ValueNode::String("a1".to_string()),
                        10,
                    ),
                },
                AllocatedSection {
                    final_size: 8,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "user#2".to_string(),
                        ValueNode::String("u2".to_string()),
                        8,
                    ),
                },
                AllocatedSection {
                    final_size: 7,
                    was_compressed: false,
                    was_truncated: false,
                    was_dropped: false,
                    section: Section::new(
                        "assistant#2".to_string(),
                        ValueNode::String("a2".to_string()),
                        7,
                    ),
                },
            ],
            total_size: 27,
            budget: 27,
            overflow: 0,
        };

        let result = renderer.render(&document, &allocation).unwrap();
        assert_eq!(result.messages.len(), 3);
        assert_eq!(result.messages[0].role, "system");
        assert!(matches!(&result.messages[0].content, Content::Text(s) if s == "s1"));
        assert_eq!(result.messages[1].role, "user");
        assert!(matches!(&result.messages[1].content, Content::Text(s) if s == "u2"));
        assert_eq!(result.messages[2].role, "assistant");
        assert!(matches!(&result.messages[2].content, Content::Text(s) if s == "a2"));
    }

    #[test]
    fn test_json_serialization() {
        let payload = CanonicalPayload {
            metadata: Metadata {
                facet_version: FACET_VERSION.to_string(),
                profile: "hypervisor".to_string(),
                mode: "exec".to_string(),
                host_profile_id: "local.default.v1".to_string(),
                policy_version: POLICY_VERSION.to_string(),
                document_hash: "deadbeef".to_string(),
                policy_hash: None,
                budget_units: 150,
                target_provider_id: "test-provider".to_string(),
            },
            tools: vec![],
            messages: vec![],
        };

        let json = to_json_string(&payload).unwrap();

        // Should be valid JSON
        let parsed: CanonicalPayload = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.metadata.facet_version, FACET_VERSION);
        assert_eq!(parsed.metadata.target_provider_id, "test-provider");
    }

    #[test]
    fn test_compact_json_is_deterministic_and_key_sorted() {
        let payload = CanonicalPayload {
            metadata: Metadata {
                facet_version: FACET_VERSION.to_string(),
                profile: "hypervisor".to_string(),
                mode: "exec".to_string(),
                host_profile_id: "local.default.v1".to_string(),
                policy_version: POLICY_VERSION.to_string(),
                document_hash: "deadbeef".to_string(),
                policy_hash: None,
                budget_units: 256,
                target_provider_id: "test-provider".to_string(),
            },
            tools: vec![],
            messages: vec![],
        };

        let c1 = to_json_compact(&payload).unwrap();
        let c2 = to_json_compact(&payload).unwrap();
        assert_eq!(c1, c2);

        let idx_messages = c1.find("\"messages\"").unwrap();
        let idx_metadata = c1.find("\"metadata\"").unwrap();
        let idx_tools = c1.find("\"tools\"").unwrap();

        assert!(idx_messages < idx_metadata);
        assert!(idx_metadata < idx_tools);
        assert!(!c1.contains("\"assistant\""));
        assert!(!c1.contains("\"system\""));
        assert!(!c1.contains("\"examples\""));
        assert!(!c1.contains("\"history\""));
        assert!(!c1.contains("\"user\""));
    }

    #[test]
    fn test_canonicalize_json_uses_rfc8785_number_form() {
        let value = serde_json::json!({
            "b": 12e1,
            "a": "Hello!"
        });

        let canonical = canonicalize_json(&value).unwrap();
        assert_eq!(canonical, r#"{"a":"Hello!","b":120}"#);
    }

    #[test]
    fn test_policy_hash_is_computed_with_version_envelope() {
        let mut policy_map = OrderedMap::new();
        policy_map.insert("op".to_string(), ValueNode::String("tool_call".to_string()));
        policy_map.insert(
            "name".to_string(),
            ValueNode::String("WeatherAPI.get_current".to_string()),
        );

        let policy_block = FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "allow".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Map(policy_map)]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let document = FacetDocument {
            blocks: vec![FacetNode::Policy(policy_block)],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let hash = compute_policy_hash(&document).unwrap();
        assert!(hash.is_some());
        assert!(hash.unwrap().starts_with("sha256:"));
    }

    #[test]
    fn test_policy_hash_none_when_policy_absent() {
        let document = FacetDocument {
            blocks: vec![FacetNode::Meta(FacetBlock {
                name: "meta".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "name".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::String("test".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                })],
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };
        assert!(compute_policy_hash(&document).unwrap().is_none());
    }

    #[test]
    fn test_tool_expose_policy_deny_filters_tools() {
        let interface = FacetNode::Interface(InterfaceNode {
            name: "WeatherAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "get".to_string(),
                params: vec![Parameter {
                    name: "city".to_string(),
                    type_node: TypeNode::Primitive("string".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            }],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let system = FacetNode::System(FacetBlock {
            name: "system".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "tools".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Variable("WeatherAPI".to_string())]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let deny_rule = ValueNode::Map(OrderedMap::from([
            (
                "op".to_string(),
                ValueNode::String("tool_expose".to_string()),
            ),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get".to_string()),
            ),
        ]));
        let policy = FacetNode::Policy(FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "deny".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![deny_rule]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let document = FacetDocument {
            blocks: vec![interface, system, policy],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let (tools, decisions) =
            extract_tools_with_guard(&document, "exec", "local.default.v1", None).unwrap();
        assert!(tools.is_empty());
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "denied");
        assert_eq!(decisions[0].error_code.as_deref(), Some("F454"));
        let expected_input_obj = serde_json::json!({
            "interface": "WeatherAPI",
            "host_profile_id": "local.default.v1",
            "facet_version": FACET_VERSION,
        });
        let expected_hash = format!(
            "sha256:{:x}",
            Sha256::digest(canonicalize_json(&expected_input_obj).unwrap().as_bytes())
        );
        assert_eq!(decisions[0].input_hash, expected_hash);
    }

    #[test]
    fn test_tool_expose_policy_undecidable_returns_f455() {
        let interface = FacetNode::Interface(InterfaceNode {
            name: "WeatherAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "get".to_string(),
                params: vec![Parameter {
                    name: "city".to_string(),
                    type_node: TypeNode::Primitive("string".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            }],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let system = FacetNode::System(FacetBlock {
            name: "system".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "tools".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Variable("WeatherAPI".to_string())]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let invalid_rule = ValueNode::Map(OrderedMap::from([
            (
                "op".to_string(),
                ValueNode::String("tool_expose".to_string()),
            ),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get".to_string()),
            ),
            (
                "when".to_string(),
                ValueNode::String("not-bool".to_string()),
            ),
        ]));
        let policy = FacetNode::Policy(FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "deny".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![invalid_rule]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let document = FacetDocument {
            blocks: vec![interface, system, policy],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let err =
            extract_tools_with_guard(&document, "exec", "local.default.v1", None).unwrap_err();
        assert!(matches!(err, RenderError::GuardUndecidable { .. }));
        assert!(err.to_string().contains("F455"));
    }

    #[test]
    fn test_tool_expose_effect_field_is_conjunctive_filter() {
        let interface = FacetNode::Interface(InterfaceNode {
            name: "WeatherAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "get".to_string(),
                params: vec![Parameter {
                    name: "city".to_string(),
                    type_node: TypeNode::Primitive("string".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            }],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let system = FacetNode::System(FacetBlock {
            name: "system".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "tools".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Variable("WeatherAPI".to_string())]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        // Effect matcher is present but does not match the tool effect_class ("read").
        // Rule must not match due to conjunctive semantics.
        let deny_rule = ValueNode::Map(OrderedMap::from([
            (
                "op".to_string(),
                ValueNode::String("tool_expose".to_string()),
            ),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get".to_string()),
            ),
            (
                "effect".to_string(),
                ValueNode::String("payment".to_string()),
            ),
        ]));
        let policy = FacetNode::Policy(FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "deny".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![deny_rule]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let document = FacetDocument {
            blocks: vec![interface, system, policy],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let (tools, decisions) =
            extract_tools_with_guard(&document, "exec", "local.default.v1", None).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "allowed");
        assert_eq!(decisions[0].effect_class.as_deref(), Some("read"));
        assert!(decisions[0].error_code.is_none());
    }

    #[test]
    fn test_tool_expose_effect_matcher_denies_on_match() {
        let interface = FacetNode::Interface(InterfaceNode {
            name: "WeatherAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "get".to_string(),
                params: vec![Parameter {
                    name: "city".to_string(),
                    type_node: TypeNode::Primitive("string".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            }],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let system = FacetNode::System(FacetBlock {
            name: "system".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "tools".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Variable("WeatherAPI".to_string())]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let deny_rule = ValueNode::Map(OrderedMap::from([
            (
                "op".to_string(),
                ValueNode::String("tool_expose".to_string()),
            ),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get".to_string()),
            ),
            ("effect".to_string(), ValueNode::String("read".to_string())),
        ]));
        let policy = FacetNode::Policy(FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "deny".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![deny_rule]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let document = FacetDocument {
            blocks: vec![interface, system, policy],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let (tools, decisions) =
            extract_tools_with_guard(&document, "exec", "local.default.v1", None).unwrap();
        assert!(tools.is_empty());
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "denied");
        assert_eq!(decisions[0].effect_class.as_deref(), Some("read"));
        assert_eq!(decisions[0].error_code.as_deref(), Some("F454"));
    }

    #[test]
    fn test_policy_cond_all_short_circuit_avoids_undecidable_tail() {
        let interface = FacetNode::Interface(InterfaceNode {
            name: "WeatherAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "get".to_string(),
                params: vec![Parameter {
                    name: "city".to_string(),
                    type_node: TypeNode::Primitive("string".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            }],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let system = FacetNode::System(FacetBlock {
            name: "system".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "tools".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Variable("WeatherAPI".to_string())]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        // all: [false, $missing] must short-circuit on first false and not become F455.
        let mut when_map = OrderedMap::new();
        when_map.insert(
            "all".to_string(),
            ValueNode::List(vec![
                ValueNode::Scalar(ScalarValue::Bool(false)),
                ValueNode::Variable("missing.flag".to_string()),
            ]),
        );

        let deny_rule = ValueNode::Map(OrderedMap::from([
            (
                "op".to_string(),
                ValueNode::String("tool_expose".to_string()),
            ),
            (
                "name".to_string(),
                ValueNode::String("WeatherAPI.get".to_string()),
            ),
            ("when".to_string(), ValueNode::Map(when_map)),
        ]));
        let policy = FacetNode::Policy(FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "deny".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![deny_rule]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let document = FacetDocument {
            blocks: vec![interface, system, policy],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let (tools, decisions) =
            extract_tools_with_guard(&document, "exec", "local.default.v1", None).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "allowed");
        assert!(decisions[0].error_code.is_none());
    }

    #[test]
    fn test_message_emit_policy_deny_omits_message_and_records_decision() {
        let document = FacetDocument {
            blocks: vec![FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "deny".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::List(vec![ValueNode::Map(OrderedMap::from([
                        (
                            "op".to_string(),
                            ValueNode::String("message_emit".to_string()),
                        ),
                        ("name".to_string(), ValueNode::String("user#1".to_string())),
                    ]))]),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                })],
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let section = Section::new(
            "user#1".to_string(),
            ValueNode::String("hello".to_string()),
            5,
        );
        let allocation = AllocationResult {
            sections: vec![AllocatedSection {
                final_size: 5,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section,
            }],
            total_size: 5,
            budget: 100,
            overflow: 0,
        };

        let output = Renderer::new()
            .render_with_trace(&document, &allocation, RenderContext::default())
            .expect("render with trace should succeed");

        assert!(output.payload.messages.is_empty());
        assert_eq!(output.guard_decisions.len(), 1);
        assert_eq!(output.guard_decisions[0].op, "message_emit");
        assert_eq!(output.guard_decisions[0].name, "user#1");
        assert_eq!(output.guard_decisions[0].decision, "denied");
        assert_eq!(
            output.guard_decisions[0].error_code.as_deref(),
            Some("F454")
        );
        let expected_input_obj = serde_json::json!({
            "message_id": "user#1",
            "role": "user",
            "host_profile_id": "local.default.v1",
            "facet_version": FACET_VERSION,
        });
        let expected_hash = format!(
            "sha256:{:x}",
            Sha256::digest(canonicalize_json(&expected_input_obj).unwrap().as_bytes())
        );
        assert_eq!(output.guard_decisions[0].input_hash, expected_hash);
    }

    #[test]
    fn test_message_emit_policy_undecidable_returns_f455() {
        let document = FacetDocument {
            blocks: vec![FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "deny".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::List(vec![ValueNode::Map(OrderedMap::from([
                        (
                            "op".to_string(),
                            ValueNode::String("message_emit".to_string()),
                        ),
                        ("name".to_string(), ValueNode::String("user#1".to_string())),
                        (
                            "when".to_string(),
                            ValueNode::Variable("missing.flag".to_string()),
                        ),
                    ]))]),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                })],
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let section = Section::new(
            "user#1".to_string(),
            ValueNode::String("hello".to_string()),
            5,
        );
        let allocation = AllocationResult {
            sections: vec![AllocatedSection {
                final_size: 5,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section,
            }],
            total_size: 5,
            budget: 100,
            overflow: 0,
        };

        let err = Renderer::new()
            .render_with_trace(&document, &allocation, RenderContext::default())
            .unwrap_err();
        assert!(matches!(err, RenderError::GuardUndecidable { .. }));
        assert!(err.to_string().contains("F455"));
    }

    #[test]
    fn test_message_emit_effect_field_is_conjunctive_filter() {
        let deny_rule = ValueNode::Map(OrderedMap::from([
            (
                "op".to_string(),
                ValueNode::String("message_emit".to_string()),
            ),
            ("name".to_string(), ValueNode::String("user#1".to_string())),
            ("effect".to_string(), ValueNode::String("read".to_string())),
        ]));
        let document = FacetDocument {
            blocks: vec![FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "deny".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::List(vec![deny_rule]),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                })],
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let section = Section::new(
            "user#1".to_string(),
            ValueNode::String("hello".to_string()),
            5,
        );
        let allocation = AllocationResult {
            sections: vec![AllocatedSection {
                final_size: 5,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section,
            }],
            total_size: 5,
            budget: 100,
            overflow: 0,
        };

        let output = Renderer::new()
            .render_with_trace(&document, &allocation, RenderContext::default())
            .expect("render with trace should succeed");
        assert_eq!(output.payload.messages.len(), 1);
        assert_eq!(output.guard_decisions.len(), 1);
        assert_eq!(output.guard_decisions[0].op, "message_emit");
        assert_eq!(output.guard_decisions[0].decision, "allowed");
        assert!(output.guard_decisions[0].error_code.is_none());
    }

    #[test]
    fn test_tool_expose_defaults_deny_applies_when_no_rule_matches() {
        let interface = FacetNode::Interface(InterfaceNode {
            name: "WeatherAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "get".to_string(),
                params: vec![Parameter {
                    name: "city".to_string(),
                    type_node: TypeNode::Primitive("string".to_string()),
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            }],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let system = FacetNode::System(FacetBlock {
            name: "system".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "tools".to_string(),
                key_kind: Default::default(),
                value: ValueNode::List(vec![ValueNode::Variable("WeatherAPI".to_string())]),
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let defaults_map = ValueNode::Map(OrderedMap::from([(
            "tool_expose".to_string(),
            ValueNode::String("deny".to_string()),
        )]));
        let policy = FacetNode::Policy(FacetBlock {
            name: "policy".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "defaults".to_string(),
                key_kind: Default::default(),
                value: defaults_map,
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        });

        let document = FacetDocument {
            blocks: vec![interface, system, policy],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let (tools, decisions) =
            extract_tools_with_guard(&document, "exec", "local.default.v1", None).unwrap();
        assert!(tools.is_empty());
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].decision, "denied");
        assert_eq!(decisions[0].error_code.as_deref(), Some("F454"));
    }

    #[test]
    fn test_message_emit_defaults_deny_applies_when_no_rule_matches() {
        let defaults_map = ValueNode::Map(OrderedMap::from([(
            "message_emit".to_string(),
            ValueNode::String("deny".to_string()),
        )]));
        let document = FacetDocument {
            blocks: vec![FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "defaults".to_string(),
                    key_kind: Default::default(),
                    value: defaults_map,
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                })],
                span: Span {
                    start: 0,
                    end: 0,
                    line: 1,
                    column: 1,
                },
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let section = Section::new(
            "user#1".to_string(),
            ValueNode::String("hello".to_string()),
            5,
        );
        let allocation = AllocationResult {
            sections: vec![AllocatedSection {
                final_size: 5,
                was_compressed: false,
                was_truncated: false,
                was_dropped: false,
                section,
            }],
            total_size: 5,
            budget: 100,
            overflow: 0,
        };

        let output = Renderer::new()
            .render_with_trace(&document, &allocation, RenderContext::default())
            .expect("render with trace should succeed");
        assert!(output.payload.messages.is_empty());
        assert_eq!(output.guard_decisions.len(), 1);
        assert_eq!(output.guard_decisions[0].decision, "denied");
        assert_eq!(
            output.guard_decisions[0].error_code.as_deref(),
            Some("F454")
        );
    }
}
