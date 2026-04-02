use fct_parser::parse_document;
use fct_validator::{TypeChecker, ValidationProfile};

fn validate_with_profile(source: &str, profile: ValidationProfile) -> Result<(), String> {
    let doc = parse_document(source)?;
    let mut checker = TypeChecker::new().with_profile(profile);
    checker.validate(&doc).map_err(|e| e.to_string())
}

#[test]
fn core_profile_rejects_interface() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string
"#;

    let result = validate_with_profile(source, ValidationProfile::Core);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("F801"));
    assert!(error.contains("@interface"));
}

#[test]
fn core_profile_rejects_test_block() {
    let source = r#"
@test(name="basic")
  assert: []
"#;

    let result = validate_with_profile(source, ValidationProfile::Core);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("F801"));
    assert!(error.contains("@test"));
}

#[test]
fn core_profile_rejects_non_literal_vars() {
    let source = r#"
@vars
  a: "x"
  b: $a
"#;

    let result = validate_with_profile(source, ValidationProfile::Core);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("F801"));
    assert!(error.contains("@vars.b"));
}

#[test]
fn core_profile_accepts_literal_vars() {
    let source = r#"
@vars
  a: "x"
  b: 42
  c: {k: "v"}
  d: [1, 2, 3]
"#;

    let result = validate_with_profile(source, ValidationProfile::Core);
    assert!(result.is_ok(), "core should allow literal-only vars");
}

#[test]
fn core_profile_rejects_input_directive_in_vars() {
    let source = r#"
@vars
  q: @input(type="string")
"#;

    let result = validate_with_profile(source, ValidationProfile::Core);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("F801"));
    assert!(error.contains("@vars.q"));
}

#[test]
fn core_profile_rejects_pipeline_in_vars() {
    let source = r#"
@vars
  greeting: "Hello" |> trim()
"#;

    let result = validate_with_profile(source, ValidationProfile::Core);
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.contains("F801"));
    assert!(error.contains("@vars.greeting"));
}
