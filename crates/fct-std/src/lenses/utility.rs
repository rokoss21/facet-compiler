// ============================================================================
// UTILITY LENSES
// ============================================================================

use crate::{Lens, LensContext, LensError, LensResult, LensSignature, TrustLevel};
use fct_ast::{ScalarValue, ValueNode};
use std::collections::HashMap;

/// default(value) - Return input if not null, else return default
pub struct DefaultLens;

impl Lens for DefaultLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::Scalar(ScalarValue::Null) => {
                if let Some(default_val) = args.first() {
                    Ok(default_val.clone())
                } else {
                    Err(LensError::ArgumentError {
                        message: "default() requires a default value argument".to_string(),
                    })
                }
            }
            other => Ok(other),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "default".to_string(),
            input_type: "any".to_string(),
            output_type: "any".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// json(indent) - Format value as JSON
pub struct JsonLens;

impl Lens for JsonLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        // Get indent size (default None for compact)
        let indent = if let Some(ValueNode::Scalar(ScalarValue::Int(n))) = args.first() {
            Some(*n as usize)
        } else {
            None
        };

        let json_str = if let Some(indent_size) = indent {
            serde_json::to_string_pretty(&input)
                .map_err(|e| LensError::ExecutionError {
                    message: format!("JSON serialization failed: {}", e),
                })?
                .lines()
                .map(|line| " ".repeat(indent_size) + line)
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            serde_json::to_string(&input).map_err(|e| LensError::ExecutionError {
                message: format!("JSON serialization failed: {}", e),
            })?
        };

        Ok(ValueNode::String(json_str))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "json".to_string(),
            input_type: "any".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// json_parse() - Parse JSON string into structured data
pub struct JsonParseLens;

impl JsonParseLens {
    fn json_value_to_value_node(value: serde_json::Value) -> ValueNode {
        match value {
            serde_json::Value::Null => ValueNode::Scalar(ScalarValue::Null),
            serde_json::Value::Bool(b) => ValueNode::Scalar(ScalarValue::Bool(b)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    ValueNode::Scalar(ScalarValue::Int(i))
                } else if let Some(f) = n.as_f64() {
                    ValueNode::Scalar(ScalarValue::Float(f))
                } else {
                    ValueNode::Scalar(ScalarValue::Null)
                }
            }
            serde_json::Value::String(s) => ValueNode::String(s),
            serde_json::Value::Array(arr) => {
                let items: Vec<ValueNode> = arr
                    .into_iter()
                    .map(Self::json_value_to_value_node)
                    .collect();
                ValueNode::List(items)
            }
            serde_json::Value::Object(obj) => {
                let map: HashMap<String, ValueNode> = obj
                    .into_iter()
                    .map(|(k, v)| (k, Self::json_value_to_value_node(v)))
                    .collect();
                ValueNode::Map(map)
            }
        }
    }
}

impl Lens for JsonParseLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let json_str = match input {
            ValueNode::String(s) => s,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "string".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        let json_value: serde_json::Value =
            serde_json::from_str(&json_str).map_err(|e| LensError::ExecutionError {
                message: format!("JSON parsing failed: {}", e),
            })?;

        Ok(Self::json_value_to_value_node(json_value))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "json_parse".to_string(),
            input_type: "string".to_string(),
            output_type: "any".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// url_encode() - Encode string for URL
pub struct UrlEncodeLens;

impl Lens for UrlEncodeLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let input_str = match input {
            ValueNode::String(s) => s,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "string".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        // URL encode using percent encoding
        let encoded = urlencoding::encode(&input_str).into_owned();
        Ok(ValueNode::String(encoded))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "url_encode".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// url_decode() - Decode URL-encoded string
pub struct UrlDecodeLens;

impl Lens for UrlDecodeLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let input_str = match input {
            ValueNode::String(s) => s,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "string".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        let decoded = urlencoding::decode(&input_str)
            .map_err(|e| LensError::ExecutionError {
                message: format!("URL decoding failed: {}", e),
            })?
            .into_owned();

        Ok(ValueNode::String(decoded))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "url_decode".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// hash(algorithm) - Generate hash of input string
/// Supported algorithms: "md5", "sha256" (default), "sha512"
pub struct HashLens;

impl Lens for HashLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        use sha2::{Digest, Sha256, Sha512};

        let input_str = match input {
            ValueNode::String(s) => s,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "string".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        // Get hash algorithm (default "sha256")
        let algorithm = if let Some(ValueNode::String(algo)) = args.first() {
            algo.as_str()
        } else {
            "sha256"
        };

        let hash_hex = match algorithm {
            "md5" => {
                let digest = md5::compute(input_str.as_bytes());
                format!("{:x}", digest)
            }
            "sha256" => {
                let mut hasher = Sha256::new();
                hasher.update(input_str.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            "sha512" => {
                let mut hasher = Sha512::new();
                hasher.update(input_str.as_bytes());
                format!("{:x}", hasher.finalize())
            }
            _ => {
                return Err(LensError::ArgumentError {
                    message: format!(
                        "Unsupported hash algorithm: {}. Supported: md5, sha256, sha512",
                        algorithm
                    ),
                })
            }
        };

        Ok(ValueNode::String(hash_hex))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "hash".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// template(**kwargs) - Simple template rendering with variable substitution
/// Supports {{variable}} syntax
pub struct TemplateLens;

impl Lens for TemplateLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let template_str = match input {
            ValueNode::String(s) => s,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "string".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        let mut result = template_str.clone();

        // Replace {{variable}} patterns with kwargs values
        for (key, value) in kwargs.iter() {
            let placeholder = format!("{{{{{}}}}}", key);
            let replacement = match value {
                ValueNode::String(s) => s.clone(),
                ValueNode::Scalar(ScalarValue::Int(n)) => n.to_string(),
                ValueNode::Scalar(ScalarValue::Float(f)) => f.to_string(),
                ValueNode::Scalar(ScalarValue::Bool(b)) => b.to_string(),
                ValueNode::Scalar(ScalarValue::Null) => "null".to_string(),
                _ => format!("{:?}", value), // Fallback for complex types
            };
            result = result.replace(&placeholder, &replacement);
        }

        Ok(ValueNode::String(result))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "template".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}
