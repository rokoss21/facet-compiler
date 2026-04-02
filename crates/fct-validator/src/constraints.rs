//! # Type Constraints
//!
//! This module contains type constraint definitions and validation logic
//! for the FACET Type System (FTS).

use crate::errors::ValidationError;
use crate::types::PrimitiveType;
use fct_ast::{ScalarValue, ValueNode};
use regex::Regex;

/// Type constraints for FACET types.
///
/// Constraints allow fine-tuned validation of values against specific
/// requirements like ranges, patterns, and enumerated values.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeConstraints {
    /// Minimum value for numeric types (int, float)
    pub min: Option<f64>,

    /// Maximum value for numeric types (int, float)
    pub max: Option<f64>,

    /// Regex pattern for string validation
    pub pattern: Option<String>,

    /// List of allowed values for enum-like validation
    pub enum_values: Option<Vec<ValueNode>>,
}

impl TypeConstraints {
    /// Create new empty constraints
    pub fn new() -> Self {
        Self {
            min: None,
            max: None,
            pattern: None,
            enum_values: None,
        }
    }

    /// Validate an integer value against constraints
    pub fn validate_int(&self, value: i64) -> Result<(), ValidationError> {
        self.validate_enum_value(&ValueNode::Scalar(ScalarValue::Int(value)))?;

        if let Some(min) = self.min {
            if (value as f64) < min {
                return Err(ValidationError::ConstraintViolation {
                    constraint: format!("min >= {}", min),
                    value: value.to_string(),
                });
            }
        }

        if let Some(max) = self.max {
            if (value as f64) > max {
                return Err(ValidationError::ConstraintViolation {
                    constraint: format!("max <= {}", max),
                    value: value.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate a float value against constraints
    pub fn validate_float(&self, value: f64) -> Result<(), ValidationError> {
        self.validate_enum_value(&ValueNode::Scalar(ScalarValue::Float(value)))?;

        if let Some(min) = self.min {
            if value < min {
                return Err(ValidationError::ConstraintViolation {
                    constraint: format!("min >= {}", min),
                    value: value.to_string(),
                });
            }
        }

        if let Some(max) = self.max {
            if value > max {
                return Err(ValidationError::ConstraintViolation {
                    constraint: format!("max <= {}", max),
                    value: value.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate a string value against constraints
    pub fn validate_string(&self, value: &str) -> Result<(), ValidationError> {
        self.validate_enum_value(&ValueNode::String(value.to_string()))?;

        // Check pattern
        if let Some(ref pattern_str) = self.pattern {
            match Regex::new(pattern_str) {
                Ok(regex) => {
                    if !regex.is_match(value) {
                        return Err(ValidationError::ConstraintViolation {
                            constraint: format!("pattern '{}'", pattern_str),
                            value: value.to_string(),
                        });
                    }
                }
                Err(_) => {
                    // Invalid regex pattern - treat as constraint violation
                    return Err(ValidationError::ConstraintViolation {
                        constraint: format!("valid regex pattern (invalid: '{}')", pattern_str),
                        value: value.to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate a bool value against enum constraints.
    pub fn validate_bool(&self, value: bool) -> Result<(), ValidationError> {
        self.validate_enum_value(&ValueNode::Scalar(ScalarValue::Bool(value)))
    }

    /// Validate null against enum constraints.
    pub fn validate_null(&self) -> Result<(), ValidationError> {
        self.validate_enum_value(&ValueNode::Scalar(ScalarValue::Null))
    }

    fn validate_enum_value(&self, value: &ValueNode) -> Result<(), ValidationError> {
        if let Some(ref enum_vals) = self.enum_values {
            if !enum_vals.iter().any(|v| v == value) {
                return Err(ValidationError::ConstraintViolation {
                    constraint: format!("one of {:?}", enum_vals),
                    value: format!("{:?}", value),
                });
            }
        }
        Ok(())
    }

    /// Check if constraints are applicable to a specific primitive type
    pub fn is_applicable_to(&self, primitive_type: &PrimitiveType) -> bool {
        match primitive_type {
            PrimitiveType::Int | PrimitiveType::Float | PrimitiveType::Number => {
                self.min.is_some() || self.max.is_some() || self.enum_values.is_some()
            }
            PrimitiveType::String => {
                self.pattern.is_some() || self.enum_values.is_some()
            }
            PrimitiveType::Bool | PrimitiveType::Boolean | PrimitiveType::Null => {
                self.enum_values.is_some()
            }
            PrimitiveType::Any => false,
        }
    }
}

impl Default for TypeConstraints {
    fn default() -> Self {
        Self::new()
    }
}
