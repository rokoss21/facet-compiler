use crate::error::{ParseResult, SpanInput};
use fct_ast::{
    Assertion, AssertionKind, MockDefinition, TestBlock, ValueNode, Span
};
use std::collections::HashMap;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while},
    character::complete::{char, multispace0, none_of},
    combinator::{map, opt, recognize},
    sequence::{delimited, preceded},
};

/// Parse assertion from key-value pairs in assert section
pub fn parse_assertion(assert_type: &str, assert_value: &ValueNode, span: &Span) -> Assertion {
    let kind = match assert_type {
        "contains" => {
            if let (Some(target), Some(text)) = extract_two_strings(assert_value) {
                AssertionKind::Contains { target, text }
            } else {
                AssertionKind::Contains { 
                    target: "output".to_string(), 
                    text: format!("{:?}", assert_value) 
                }
            }
        }
        "not_contains" => {
            if let (Some(target), Some(text)) = extract_two_strings(assert_value) {
                AssertionKind::NotContains { target, text }
            } else {
                AssertionKind::NotContains { 
                    target: "output".to_string(), 
                    text: format!("{:?}", assert_value) 
                }
            }
        }
        "equals" => {
            if let (Some(target), expected) = extract_target_and_value(assert_value) {
                AssertionKind::Equals { target, expected }
            } else {
                AssertionKind::Equals { 
                    target: "output".to_string(), 
                    expected: assert_value.clone()
                }
            }
        }
        "less_than" => {
            if let (Some(field), value) = extract_field_and_number(assert_value) {
                AssertionKind::LessThan { field, value }
            } else {
                AssertionKind::LessThan { 
                    field: "cost".to_string(), 
                    value: 0.01 
                }
            }
        }
        "greater_than" => {
            if let (Some(field), value) = extract_field_and_number(assert_value) {
                AssertionKind::GreaterThan { field, value }
            } else {
                AssertionKind::GreaterThan { 
                    field: "cost".to_string(), 
                    value: 0.01 
                }
            }
        }
        "sentiment" => {
            if let (Some(target), Some(expected)) = extract_two_strings(assert_value) {
                AssertionKind::Sentiment { target, expected }
            } else {
                AssertionKind::Sentiment {
                    target: "output".to_string(),
                    expected: "helpful".to_string()
                }
            }
        }
        "matches" => {
            if let (Some(target), Some(pattern)) = extract_two_strings(assert_value) {
                AssertionKind::Matches { target, pattern }
            } else {
                AssertionKind::Matches {
                    target: "output".to_string(),
                    pattern: ".*".to_string()
                }
            }
        }
        "true" => {
            let target = extract_single_string(assert_value).unwrap_or_else(|| "output".to_string());
            AssertionKind::True { target }
        }
        "false" => {
            let target = extract_single_string(assert_value).unwrap_or_else(|| "output".to_string());
            AssertionKind::False { target }
        }
        "null" => {
            let target = extract_single_string(assert_value).unwrap_or_else(|| "output".to_string());
            AssertionKind::Null { target }
        }
        "not_null" => {
            let target = extract_single_string(assert_value).unwrap_or_else(|| "output".to_string());
            AssertionKind::NotNull { target }
        }
        _ => AssertionKind::Contains { 
            target: "output".to_string(), 
            text: format!("unknown assertion: {}", assert_type) 
        },
    };
    
    Assertion {
        kind,
        span: span.clone(),
    }
}

/// Extract two string values from a ValueNode (for target/text pairs)
pub fn extract_two_strings(value: &ValueNode) -> (Option<String>, Option<String>) {
    match value {
        ValueNode::List(items) if items.len() >= 2 => {
            let target = extract_single_string(&items[0]);
            let text = extract_single_string(&items[1]);
            (target, text)
        }
        ValueNode::Map(map) if map.len() >= 2 => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            if keys.len() >= 2 {
                let target = extract_single_string(&map[keys[0]]);
                let text = extract_single_string(&map[keys[1]]);
                (target, text)
            } else {
                (None, None)
            }
        }
        _ => (None, None),
    }
}

/// Extract target string and expected value from a ValueNode
fn extract_target_and_value(value: &ValueNode) -> (Option<String>, ValueNode) {
    match value {
        ValueNode::List(items) if items.len() >= 2 => {
            let target = extract_single_string(&items[0]);
            let expected = items.get(1).cloned().unwrap_or(ValueNode::Scalar(fct_ast::ScalarValue::Null));
            (target, expected)
        }
        ValueNode::Map(map) if map.contains_key("target") => {
            let target = map.get("target").and_then(extract_single_string);
            let expected = map.get("expected").cloned().unwrap_or(ValueNode::Scalar(fct_ast::ScalarValue::Null));
            (target, expected)
        }
        _ => (None, value.clone()),
    }
}

/// Extract field name and numeric value from a ValueNode
fn extract_field_and_number(value: &ValueNode) -> (Option<String>, f64) {
    match value {
        ValueNode::List(items) if items.len() >= 2 => {
            let field = extract_single_string(&items[0]);
            let num = extract_number(&items[1]);
            (field, num)
        }
        ValueNode::Map(map) if map.len() >= 2 => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            if keys.len() >= 2 {
                let field = Some(keys[0].clone());
                let num = extract_number(&map[keys[1]]);
                (field, num)
            } else {
                (None, 0.0)
            }
        }
        _ => (None, 0.0),
    }
}

/// Extract a single string from a ValueNode
fn extract_single_string(value: &ValueNode) -> Option<String> {
    match value {
        ValueNode::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Extract a numeric value from a ValueNode
fn extract_number(value: &ValueNode) -> f64 {
    match value {
        ValueNode::Scalar(fct_ast::ScalarValue::Int(i)) => *i as f64,
        ValueNode::Scalar(fct_ast::ScalarValue::Float(f)) => *f,
        _ => 0.0,
    }
}