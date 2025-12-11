// ============================================================================
// LIST LENSES
// ============================================================================

use crate::{Lens, LensContext, LensError, LensResult, LensSignature, TrustLevel};
use fct_ast::{ScalarValue, ValueNode};
use std::collections::HashMap;

/// map(operation) - Transform list elements
pub struct MapLens;

impl Lens for MapLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let list = match input {
            ValueNode::List(items) => items,
            _ => {
                return Err(LensError::TypeMismatch {
                    expected: "list".to_string(),
                    got: format!("{:?}", input),
                })
            }
        };

        // Get map operation from args
        let operation = args.first().ok_or_else(|| LensError::ArgumentError {
            message: "Map requires an operation argument".to_string(),
        })?;

        let mut mapped_items = Vec::new();

        for item in list {
            match operation {
                ValueNode::Variable(_var_name) => {
                    // Simple variable substitution - for now just return the item
                    // In full implementation, this would support more complex operations
                    mapped_items.push(item.clone());
                }
                ValueNode::String(op) => {
                    // String-based operations
                    match op.as_str() {
                        "to_string" => {
                            mapped_items.push(ValueNode::String(format!("{:?}", item)));
                        }
                        _ => {
                            return Err(LensError::ArgumentError {
                                message: format!("Unknown map operation: {}", op),
                            });
                        }
                    }
                }
                _ => {
                    return Err(LensError::ArgumentError {
                        message: "Map operation must be variable reference or string".to_string(),
                    });
                }
            }
        }

        Ok(ValueNode::List(mapped_items))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "map".to_string(),
            input_type: "list".to_string(),
            output_type: "list".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// filter(condition) - Filter list elements
pub struct FilterLens;

impl Lens for FilterLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let list = match input {
            ValueNode::List(items) => items,
            _ => {
                return Err(LensError::TypeMismatch {
                    expected: "list".to_string(),
                    got: format!("{:?}", input),
                })
            }
        };

        // Get filter condition from args
        let condition = args.first().ok_or_else(|| LensError::ArgumentError {
            message: "Filter requires a condition argument".to_string(),
        })?;

        let filtered_items: Vec<ValueNode> = list
            .iter()
            .filter(|item| {
                // Basic filtering - non-null, non-empty values
                match condition {
                    ValueNode::String(cond) => match cond.as_str() {
                        "non_null" => !matches!(item, ValueNode::Scalar(ScalarValue::Null)),
                        "non_empty" => match item {
                            ValueNode::String(s) => !s.is_empty(),
                            ValueNode::List(l) => !l.is_empty(),
                            ValueNode::Map(m) => !m.is_empty(),
                            _ => true,
                        },
                        _ => true,
                    },
                    _ => true, // If condition is unclear, keep all items
                }
            })
            .cloned()
            .collect();

        Ok(ValueNode::List(filtered_items))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "filter".to_string(),
            input_type: "list".to_string(),
            output_type: "list".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// sort_by(key, order) - Sort list elements
pub struct SortByLens;

impl Lens for SortByLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let mut list = match input {
            ValueNode::List(items) => items,
            _ => {
                return Err(LensError::TypeMismatch {
                    expected: "list".to_string(),
                    got: format!("{:?}", input),
                })
            }
        };

        // Check if we should sort descending
        let descending = if let Some(ValueNode::String(order)) = args.get(1) {
            order.as_str() == "desc"
        } else {
            false
        };

        // Simple sort by string representation
        list.sort_by(|a, b| {
            let a_str = format!("{:?}", a);
            let b_str = format!("{:?}", b);

            if descending {
                b_str.cmp(&a_str)
            } else {
                a_str.cmp(&b_str)
            }
        });

        Ok(ValueNode::List(list))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "sort_by".to_string(),
            input_type: "list".to_string(),
            output_type: "list".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// ensure_list() - Ensure value is a list (wrap single values)
pub struct EnsureListLens;

impl Lens for EnsureListLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::List(_) => Ok(input),       // Already a list
            _ => Ok(ValueNode::List(vec![input])), // Wrap single value
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "ensure_list".to_string(),
            input_type: "any".to_string(),
            output_type: "list".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// first() - Get the first element of a list
pub struct FirstLens;

impl Lens for FirstLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::List(items) => {
                items.first().cloned().ok_or_else(|| LensError::ExecutionError {
                    message: "Cannot get first element of empty list".to_string(),
                })
            }
            other => Err(LensError::TypeMismatch {
                expected: "list".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "first".to_string(),
            input_type: "list".to_string(),
            output_type: "any".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// last() - Get the last element of a list
pub struct LastLens;

impl Lens for LastLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::List(items) => {
                items.last().cloned().ok_or_else(|| LensError::ExecutionError {
                    message: "Cannot get last element of empty list".to_string(),
                })
            }
            other => Err(LensError::TypeMismatch {
                expected: "list".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "last".to_string(),
            input_type: "list".to_string(),
            output_type: "any".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// nth(index) - Get the nth element of a list (0-indexed)
pub struct NthLens;

impl Lens for NthLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let list = match input {
            ValueNode::List(items) => items,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "list".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        let index = match args.first() {
            Some(ValueNode::Scalar(ScalarValue::Int(n))) => *n as usize,
            _ => {
                return Err(LensError::ArgumentError {
                    message: "nth() requires an integer index argument".to_string(),
                })
            }
        };

        list.get(index).cloned().ok_or_else(|| LensError::ExecutionError {
            message: format!("Index {} out of bounds for list of length {}", index, list.len()),
        })
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "nth".to_string(),
            input_type: "list".to_string(),
            output_type: "any".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// slice(start, end) - Extract a slice from a list
/// start: starting index (inclusive)
/// end: ending index (exclusive), optional - if not provided, slices until end of list
pub struct SliceLens;

impl Lens for SliceLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let list = match input {
            ValueNode::List(items) => items,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "list".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        if args.is_empty() {
            return Err(LensError::ArgumentError {
                message: "slice() requires at least a start index argument".to_string(),
            });
        }

        let start = match &args[0] {
            ValueNode::Scalar(ScalarValue::Int(n)) => {
                let val = *n as usize;
                if val > list.len() {
                    return Err(LensError::ArgumentError {
                        message: format!("start index {} exceeds list length {}", val, list.len()),
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
                    if val > list.len() {
                        return Err(LensError::ArgumentError {
                            message: format!("end index {} exceeds list length {}", val, list.len()),
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
            list.len()
        };

        Ok(ValueNode::List(list[start..end].to_vec()))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "slice".to_string(),
            input_type: "list".to_string(),
            output_type: "list".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// length() - Get the length of a list
pub struct LengthLens;

impl Lens for LengthLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::List(items) => Ok(ValueNode::Scalar(ScalarValue::Int(items.len() as i64))),
            other => Err(LensError::TypeMismatch {
                expected: "list".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "length".to_string(),
            input_type: "list".to_string(),
            output_type: "int".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// unique() - Remove duplicate elements from a list
pub struct UniqueLens;

impl Lens for UniqueLens {
    fn execute(
        &self,
        input: ValueNode,
        _args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        match input {
            ValueNode::List(items) => {
                let mut unique_items = Vec::new();
                let mut seen = std::collections::HashSet::new();

                for item in items {
                    // Use debug representation as a simple way to compare items
                    let item_repr = format!("{:?}", item);
                    if seen.insert(item_repr) {
                        unique_items.push(item);
                    }
                }

                Ok(ValueNode::List(unique_items))
            }
            other => Err(LensError::TypeMismatch {
                expected: "list".to_string(),
                got: format!("{:?}", other),
            }),
        }
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "unique".to_string(),
            input_type: "list".to_string(),
            output_type: "list".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}

/// join(separator) - Join list elements into a string
pub struct JoinLens;

impl Lens for JoinLens {
    fn execute(
        &self,
        input: ValueNode,
        args: Vec<ValueNode>,
        _kwargs: HashMap<String, ValueNode>,
        _ctx: &LensContext,
    ) -> LensResult<ValueNode> {
        let list = match input {
            ValueNode::List(items) => items,
            other => {
                return Err(LensError::TypeMismatch {
                    expected: "list".to_string(),
                    got: format!("{:?}", other),
                })
            }
        };

        let separator = if let Some(ValueNode::String(sep)) = args.first() {
            sep.clone()
        } else {
            "".to_string() // Default to empty separator
        };

        let string_parts: Vec<String> = list
            .iter()
            .map(|item| match item {
                ValueNode::String(s) => s.clone(),
                ValueNode::Scalar(ScalarValue::Int(n)) => n.to_string(),
                ValueNode::Scalar(ScalarValue::Float(f)) => f.to_string(),
                ValueNode::Scalar(ScalarValue::Bool(b)) => b.to_string(),
                ValueNode::Scalar(ScalarValue::Null) => "null".to_string(),
                _ => format!("{:?}", item), // Fallback to debug representation
            })
            .collect();

        Ok(ValueNode::String(string_parts.join(&separator)))
    }

    fn signature(&self) -> LensSignature {
        LensSignature {
            name: "join".to_string(),
            input_type: "list".to_string(),
            output_type: "string".to_string(),
            trust_level: TrustLevel::Pure,
            deterministic: true,
        }
    }
}
