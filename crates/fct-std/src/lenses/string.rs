// ============================================================================
// STRING LENSES
// ============================================================================

use crate::{Lens, LensContext, LensError, LensResult, LensSignature, TrustLevel};
use fct_ast::{ScalarValue, ValueNode};
use std::collections::HashMap;

/// trim() - Remove whitespace from both ends of a string
pub struct TrimLens;

impl Lens for TrimLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::String(s) => Ok(ValueNode::String(s.trim().to_string())),
            other => Err(LensError::TypeMismatch {
                expected: "string".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "trim".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// lowercase() - Convert string to lowercase
pub struct LowercaseLens;

impl Lens for LowercaseLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::String(s) => Ok(ValueNode::String(s.to_lowercase())),
            other => Err(LensError::TypeMismatch {
                expected: "string".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "lowercase".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// uppercase() - Convert string to uppercase
pub struct UppercaseLens;

impl Lens for UppercaseLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::String(s) => Ok(ValueNode::String(s.to_uppercase())),
            other => Err(LensError::TypeMismatch {
                expected: "string".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "uppercase".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// split(separator) - Split string by delimiter
pub struct SplitLens;

impl Lens for SplitLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
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

        let separator = if let Some(ValueNode::String(sep)) = args.first() {
            sep.clone()
        } else {
            return Err(LensError::ArgumentError {
                message: "split() requires a separator argument".to_string(),
            });
        };

        let parts: Vec<ValueNode> = input_str
            .split(&separator)
            .map(|s| ValueNode::String(s.to_string()))
            .collect();

        Ok(ValueNode::List(parts))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "split".to_string(),
            input_type: "string".to_string(),
            output_type: "list<string>".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// replace(pattern, replacement) - Replace pattern in string
pub struct ReplaceLens;

impl Lens for ReplaceLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
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

        if args.len() < 2 {
            return Err(LensError::ArgumentError {
                message: "replace() requires pattern and replacement arguments".to_string(),
            });
        }

        let pattern = match &args[0] {
            ValueNode::String(s) => s,
            _ => {
                return Err(LensError::ArgumentError {
                    message: "pattern must be a string".to_string(),
                })
            }
        };

        let replacement = match &args[1] {
            ValueNode::String(s) => s,
            _ => {
                return Err(LensError::ArgumentError {
                    message: "replacement must be a string".to_string(),
                })
            }
        };

        Ok(ValueNode::String(input_str.replace(pattern, replacement)))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "replace".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// indent(size) - Add indentation to each line
pub struct IndentLens;

impl Lens for IndentLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let text = match input {
            ValueNode::String(s) => s,
            _ => {
                return Err(LensError::TypeMismatch {
                    expected: "string".to_string(),
                    got: format!("{:?}", input),
                })
            }
        };

        // Get indent level (default 2)
        let indent_size = if let Some(ValueNode::Scalar(ScalarValue::Int(n))) = args.first() {
            *n as usize
        } else {
            2
        };

        let indent = " ".repeat(indent_size);
        let indented = text
            .lines()
            .map(|line| format!("{}{}", indent, line))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(ValueNode::String(indented))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "indent".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// capitalize() - Capitalize the first letter of a string
pub struct CapitalizeLens;

impl Lens for CapitalizeLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::String(s) => {
                if s.is_empty() {
                    Ok(ValueNode::String(s))
                } else {
                    let mut chars = s.chars();
                    let first = chars.next().unwrap().to_uppercase().to_string();
                    let rest: String = chars.collect();
                    Ok(ValueNode::String(format!("{}{}", first, rest)))
                }
            }
            other => Err(LensError::TypeMismatch {
                expected: "string".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "capitalize".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// reverse() - Reverse the characters in a string
pub struct ReverseLens;

impl Lens for ReverseLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::String(s) => {
                let reversed: String = s.chars().rev().collect();
                Ok(ValueNode::String(reversed))
            }
            other => Err(LensError::TypeMismatch {
                expected: "string".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "reverse".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// substring(start, end) - Extract a substring from a string
/// start: starting index (inclusive)
/// end: ending index (exclusive), optional - if not provided, extracts until end of string
pub struct SubstringLens;

impl Lens for SubstringLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
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

        if args.is_empty() {
            return Err(LensError::ArgumentError {
                message: "substring() requires at least a start index argument".to_string(),
            });
        }

        let start = match &args[0] {
            ValueNode::Scalar(ScalarValue::Int(n)) => {
                let val = *n as usize;
                if val > input_str.len() {
                    return Err(LensError::ArgumentError {
                        message: format!("start index {} exceeds string length {}", val, input_str.len()),
                    });
                }
                val
            }
            _ => {
                return Err(LensError::ArgumentError {
                    message: "start index must be an integer".to_string(),
                })
            }
        };

        let end = if args.len() > 1 {
            match &args[1] {
                ValueNode::Scalar(ScalarValue::Int(n)) => {
                    let val = *n as usize;
                    if val > input_str.len() {
                        return Err(LensError::ArgumentError {
                            message: format!("end index {} exceeds string length {}", val, input_str.len()),
                        });
                    }
                    if val < start {
                        return Err(LensError::ArgumentError {
                            message: format!("end index {} is less than start index {}", val, start),
                        });
                    }
                    val
                }
                _ => {
                    return Err(LensError::ArgumentError {
                        message: "end index must be an integer".to_string(),
                    })
                }
            }
        } else {
            input_str.len()
        };

        // Use char indices for proper UTF-8 handling
        let chars: Vec<char> = input_str.chars().collect();
        let substring: String = chars[start..end].iter().collect();

        Ok(ValueNode::String(substring))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "substring".to_string(),
            input_type: "string".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}
