use fct_parser::parse_document;
use fct_validator::TypeChecker;

fn validate(source: &str) -> Result<(), String> {
    let doc = parse_document(source)?;
    let mut checker = TypeChecker::new();
    checker.validate(&doc).map_err(|e| e.to_string())
}

fn payment_interface() -> &'static str {
    r#"
@interface PaymentAPI
  fn charge(amount: int) -> string (effect="payment")
"#
}

#[test]
fn policy_rejects_unknown_top_level_key() {
    let source = r#"
@policy
  unknown: true
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_non_list_allow() {
    let source = r#"
@policy
  allow: { op: "tool_call", name: "WeatherAPI.get_current" }
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_invalid_op() {
    let source = r#"
@policy
  allow: [{ op: "tooling", name: "WeatherAPI.get_current" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_requires_name_for_tool_call() {
    let source = r#"
@policy
  allow: [{ op: "tool_call" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_invalid_name_wildcard() {
    let source = r#"
@policy
  allow: [{ op: "tool_call", name: "Pay*ment.charge" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_whitespace_in_name() {
    let source = r#"
@policy
  allow: [{ op: "tool_call", name: "PaymentAPI.charge now" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_empty_all_condition() {
    let source = format!(
        r#"
{}
@policy
  allow: [{{ op: "tool_call", name: "PaymentAPI.charge", when: {{ all: [] }} }}]
"#,
        payment_interface()
    );
    let err = validate(&source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_unknown_condition_variable() {
    let source = format!(
        r#"
{}
@policy
  allow: [{{ op: "tool_call", name: "PaymentAPI.charge", when: $missing }}]
"#,
        payment_interface()
    );
    let err = validate(&source).unwrap_err();
    assert!(err.contains("F401"), "expected F401, got: {}", err);
}

#[test]
fn policy_rejects_non_boolean_condition_variable() {
    let source = format!(
        r#"
{}
@vars
  allow_tools: "yes"

@policy
  allow: [{{ op: "tool_call", name: "PaymentAPI.charge", when: $allow_tools }}]
"#,
        payment_interface()
    );
    let err = validate(&source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {}", err);
}

#[test]
fn policy_accepts_valid_rule_and_condition() {
    let source = format!(
        r#"
{}
@vars
  allow_tools: true

@policy
  allow: [{{ op: "tool_call", name: "PaymentAPI.charge", when: $allow_tools }}]
"#,
        payment_interface()
    );
    let result = validate(&source);
    assert!(result.is_ok(), "expected policy to be valid, got: {:?}", result);
}

#[test]
fn policy_rejects_unknown_interface_in_tool_name() {
    let source = r#"
@policy
  allow: [{ op: "tool_call", name: "MissingAPI.charge" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_unknown_function_in_tool_name() {
    let source = r#"
@interface PaymentAPI
  fn charge(amount: int) -> string (effect="payment")

@policy
  allow: [{ op: "tool_call", name: "PaymentAPI.refund" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_unknown_defaults_operation_key() {
    let source = r#"
@policy
  defaults: { tool_execute: "deny" }
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_rejects_invalid_defaults_value() {
    let source = r#"
@policy
  defaults: { tool_call: "block" }
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_accepts_valid_defaults_values() {
    let source = r#"
@policy
  defaults: { tool_call: "deny", message_emit: true, lens_call: false, tool_expose: "allow" }
"#;
    let result = validate(source);
    assert!(result.is_ok(), "expected valid defaults map, got: {:?}", result);
}

#[test]
fn policy_rejects_unknown_lens_name_in_lens_call_rule() {
    let source = r#"
@policy
  allow: [{ op: "lens_call", name: "missing_lens" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn policy_accepts_known_lens_name_and_prefix_matcher() {
    let source = r#"
@policy
  allow: [{ op: "lens_call", name: "trim" }, { op: "lens_call", name: "tr.*" }]
"#;
    let result = validate(source);
    assert!(result.is_ok(), "expected valid lens matchers, got: {:?}", result);
}

#[test]
fn policy_accepts_known_message_emit_ids() {
    let source = r#"
@system
  content: "s"

@user
  id: "u.custom"
  content: "u"

@policy
  allow: [{ op: "message_emit", name: "system#1" }, { op: "message_emit", name: "u.custom" }]
"#;
    let result = validate(source);
    assert!(
        result.is_ok(),
        "expected known message_emit ids to validate, got: {:?}",
        result
    );
}

#[test]
fn policy_rejects_unknown_message_emit_id() {
    let source = r#"
@system
  content: "s"

@policy
  allow: [{ op: "message_emit", name: "user#1" }]
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}
