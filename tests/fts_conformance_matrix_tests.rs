use fct_parser::parse_document;
use fct_validator::TypeChecker;

fn validate(source: &str) -> Result<(), String> {
    let doc = parse_document(source)?;
    let mut checker = TypeChecker::new();
    checker.validate(&doc).map_err(|e| e.to_string())
}

#[test]
fn fts_positive_conformance_matrix() {
    let cases = [
        (
            "primitive_exact",
            r#"
@var_types
  n: "int"

@vars
  n: 7
"#,
        ),
        (
            "constraints_min_max_pattern_enum",
            r#"
@var_types
  code: { type: "string", pattern: "^[A-Z]+$", enum: ["OK", "FAIL"] }
  score: { type: "int", min: 0, max: 10 }

@vars
  code: "OK"
  score: 8
"#,
        ),
        (
            "struct_with_extra_fields",
            r#"
@var_types
  user: "struct { name: string, age: int }"

@vars
  user: { name: "Ada", age: 37, city: "Minsk" }
"#,
        ),
        (
            "nested_composites",
            r#"
@var_types
  rows: "list<map<string, int>>"

@vars
  rows: [{ a: 1 }, { b: 2 }]
"#,
        ),
        (
            "union_optional",
            r#"
@var_types
  maybe_name: "string | null"

@vars
  maybe_name: null
"#,
        ),
        (
            "multimodal_embedding",
            r#"
@var_types
  vec: "embedding<size=3>"

@vars
  vec: [0.1, 0.2, 0.3]
"#,
        ),
        (
            "multimodal_image_constraints",
            r#"
@var_types
  img: "image(format=jpeg, max_dim=1024)"

@vars
  img: { kind: "image", format: "jpeg", shape: { width: 900, height: 700 } }
"#,
        ),
    ];

    for (name, source) in cases {
        assert!(validate(source).is_ok(), "case failed: {name}");
    }
}

#[test]
fn fts_negative_conformance_matrix() {
    let cases = [
        (
            "primitive_mismatch",
            r#"
@var_types
  n: "float"

@vars
  n: 1
"#,
            "F451",
        ),
        (
            "enum_violation",
            r#"
@var_types
  state: { type: "string", enum: ["open", "closed"] }

@vars
  state: "pending"
"#,
            "F452",
        ),
        (
            "range_violation",
            r#"
@var_types
  score: { type: "int", min: 0, max: 5 }

@vars
  score: 9
"#,
            "F452",
        ),
        (
            "struct_missing_field",
            r#"
@var_types
  user: "struct { name: string, age: int }"

@vars
  user: { name: "Ada" }
"#,
            "F451",
        ),
        (
            "list_element_mismatch",
            r#"
@var_types
  nums: "list<int>"

@vars
  nums: [1, "x", 2]
"#,
            "F451",
        ),
        (
            "map_value_mismatch",
            r#"
@var_types
  scores: "map<string, int>"

@vars
  scores: { alice: 1, bob: "bad" }
"#,
            "F451",
        ),
        (
            "embedding_size_mismatch",
            r#"
@var_types
  vec: "embedding<size=4>"

@vars
  vec: [0.1, 0.2, 0.3]
"#,
            "F451",
        ),
        (
            "image_constraint_mismatch",
            r#"
@var_types
  img: "image(format=jpeg, max_dim=512)"

@vars
  img: { kind: "image", format: "jpeg", shape: { width: 800, height: 400 } }
"#,
            "F451",
        ),
        (
            "invalid_type_expression",
            r#"
@vars
  x: @input(type="list<int")
"#,
            "F452",
        ),
        (
            "invalid_multimodal_format",
            r#"
@vars
  img: @input(type="image(format=gif)")
"#,
            "F452",
        ),
    ];

    for (name, source, expected_code) in cases {
        let err = validate(source).expect_err(&format!("case should fail: {name}"));
        assert!(
            err.contains(expected_code),
            "case {name} expected {expected_code}, got: {err}"
        );
    }
}
