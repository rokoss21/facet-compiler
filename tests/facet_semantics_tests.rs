use fct_parser::parse_document;
use fct_validator::TypeChecker;

fn validate(source: &str) -> Result<(), String> {
    let doc = parse_document(source)?;
    let mut checker = TypeChecker::new();
    checker.validate(&doc).map_err(|e| e.to_string())
}

#[test]
fn meta_rejects_non_atom_values() {
    let source = r#"
@vars
  x: "ok"

@meta
  bad: $x
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn meta_allows_namespaced_string_keys() {
    let source = r#"
@meta
  "x.acme.build_id": "abc123"
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn meta_allows_quoted_identifier_like_key() {
    let source = r#"
@meta
  "author": "Facet Playground"
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn context_requires_budget() {
    let source = r#"
@context
  defaults: { priority: 1 }
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn context_budget_must_be_non_negative_int() {
    let source = r#"
@context
  budget: -1
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn message_requires_content_field() {
    let source = r#"
@user
  priority: 10
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn when_attribute_must_be_bool_or_bool_var() {
    let source = r#"
@user(when="yes")
  content: "hello"
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn when_attribute_allows_bool_var() {
    let source = r#"
@vars
  show: true

@user(when=$show)
  content: "hello"
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_must_be_base_expression_in_vars() {
    let source = r#"
@vars
  query: { value: @input(type="string") }
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn input_requires_type_attr() {
    let source = r#"
@vars
  query: @input(default="x")
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn input_allows_pipeline_with_input_as_base() {
    let source = r#"
@vars
  query: @input(type="string") |> trim()
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_all_primitive_types() {
    let source = r#"
@vars
  s: @input(type="string")
  i: @input(type="int")
  f: @input(type="float")
  b: @input(type="bool")
  n: @input(type="null")
  a: @input(type="any")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_composite_list_type() {
    let source = r#"
@vars
  ids: @input(type="list<int>")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_composite_map_type() {
    let source = r#"
@vars
  scores: @input(type="map<string, int>")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_composite_struct_type() {
    let source = r#"
@vars
  user: @input(type="struct { name: string, age: int }")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_union_type() {
    let source = r#"
@vars
  maybe_name: @input(type="string | null")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_nested_composite_type_expression() {
    let source = r#"
@vars
  rows: @input(type="list<struct { id: int, tags: list<string> }>")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_multimodal_image_type() {
    let source = r#"
@vars
  img: @input(type="image")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_image_type_with_constraints() {
    let source = r#"
@vars
  img: @input(type="image(format=jpeg, max_dim=1024)")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_rejects_image_type_with_invalid_format() {
    let source = r#"
@vars
  img: @input(type="image(format=gif)")
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn input_allows_multimodal_audio_type() {
    let source = r#"
@vars
  clip: @input(type="audio")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_allows_embedding_type_with_size() {
    let source = r#"
@vars
  vec: @input(type="embedding<size=3>")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn input_rejects_embedding_type_with_non_positive_size() {
    let source = r#"
@vars
  vec: @input(type="embedding<size=0>")
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn var_types_multimodal_matches_input_declared_type() {
    let source = r#"
@var_types
  img: "image"

@vars
  img: @input(type="image")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_multimodal_image_constraint_accepts_narrower_input_type() {
    let source = r#"
@var_types
  img: "image"

@vars
  img: @input(type="image(format=jpeg, max_dim=512)")
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_multimodal_image_constraint_rejects_broader_input_type() {
    let source = r#"
@var_types
  img: "image(format=jpeg, max_dim=512)"

@vars
  img: @input(type="image")
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn var_types_enum_accepts_matching_string_literal() {
    let source = r#"
@var_types
  status: {
    type: "string",
    enum: ["open", "closed"]
  }

@vars
  status: "open"
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_enum_accepts_matching_int_literal() {
    let source = r#"
@var_types
  code: {
    type: "int",
    enum: [1, 2, 3]
  }

@vars
  code: 2
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_struct_accepts_matching_map_literal() {
    let source = r#"
@var_types
  user: "struct { name: string, age: int }"

@vars
  user: { name: "Ada", age: 42, city: "Minsk" }
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_struct_rejects_missing_required_field() {
    let source = r#"
@var_types
  user: "struct { name: string, age: int }"

@vars
  user: { name: "Ada" }
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn var_types_list_rejects_non_matching_item_type() {
    let source = r#"
@var_types
  nums: "list<int>"

@vars
  nums: [1, "oops", 3]
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn var_types_map_rejects_non_matching_value_type() {
    let source = r#"
@var_types
  scores: "map<string, int>"

@vars
  scores: { alice: 10, bob: "bad" }
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn var_types_union_accepts_member_value() {
    let source = r#"
@var_types
  maybe_name: "string | null"

@vars
  maybe_name: null
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_embedding_rejects_wrong_size_vector() {
    let source = r#"
@var_types
  vec: "embedding<size=3>"

@vars
  vec: [1, 2]
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn var_types_image_constraints_accept_valid_asset_literal() {
    let source = r#"
@var_types
  img: "image(format=jpeg, max_dim=1024)"

@vars
  img: { kind: "image", format: "jpeg", shape: { width: 800, height: 600 } }
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn var_types_image_constraints_reject_asset_over_max_dim() {
    let source = r#"
@var_types
  img: "image(format=jpeg, max_dim=512)"

@vars
  img: { kind: "image", format: "jpeg", shape: { width: 800, height: 400 } }
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn primitive_int_is_not_assignable_to_float_type() {
    let source = r#"
@var_types
  score: "float"

@vars
  score: 1
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn input_default_must_be_atom() {
    let source = r#"
@vars
  fallback: "x"
  query: @input(type="string", default=$fallback)
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn input_not_allowed_inside_lens_arguments() {
    let source = r#"
@vars
  query: @input(type="string") |> split(@input(type="string"))
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn input_not_allowed_outside_vars_value_position() {
    let source = r#"
@user
  content: @input(type="string")
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn string_keyed_map_outside_meta_is_rejected() {
    let source = r#"
@user
  content: { "x.acme.value": "v" }
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn quoted_identifier_key_outside_meta_is_rejected() {
    let source = r#"
@user
  "content": "hi"
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn multimodal_content_item_requires_asset_map() {
    let source = r#"
@user
  content: [{ type: "image", asset: "abc" }]
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn multimodal_image_asset_canonical_form_is_accepted() {
    let source = r#"
@user
  content: [{
    type: "image",
    asset: {
      kind: "image",
      format: "jpeg",
      digest: { algo: "sha256", value: "deadbeef" },
      shape: { width: 1024, height: 768 }
    }
  }]
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn multimodal_audio_asset_canonical_form_is_accepted() {
    let source = r#"
@user
  content: [{
    type: "audio",
    asset: {
      kind: "audio",
      format: "wav",
      digest: { algo: "sha256", value: "feedface" },
      shape: { duration: 3.2 }
    }
  }]
"#;

    assert!(validate(source).is_ok());
}

#[test]
fn multimodal_image_asset_rejects_non_sha256_digest_algo() {
    let source = r#"
@user
  content: [{
    type: "image",
    asset: {
      kind: "image",
      format: "png",
      digest: { algo: "md5", value: "abc" },
      shape: { width: 10, height: 10 }
    }
  }]
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn multimodal_audio_asset_rejects_invalid_format() {
    let source = r#"
@user
  content: [{
    type: "audio",
    asset: {
      kind: "audio",
      format: "flac",
      digest: { algo: "sha256", value: "abc" },
      shape: { duration: 1.0 }
    }
  }]
"#;

    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {err}");
}
