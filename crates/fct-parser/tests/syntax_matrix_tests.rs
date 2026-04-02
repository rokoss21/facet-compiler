use fct_parser::parse_document;

#[test]
fn facet_forms_matrix_parses() {
    let cases = [
        (
            "meta",
            r#"
@meta
  version: "1.0"
  "x.acme.build_id": "abc"
"#,
        ),
        (
            "context",
            r#"
@context
  budget: 32000
  defaults: { priority: 500, min: 0, grow: 0, shrink: 0 }
"#,
        ),
        (
            "vars_and_var_types",
            r#"
@vars
  name: "Alice" |> trim()

@var_types
  name: "string"
"#,
        ),
        (
            "messages",
            r#"
@system(when=true)
  tools: [$WeatherAPI]
  content: "System"

@user
  content: [{ type: "text", text: "Hello" }]

@assistant
  content: "Ack"
"#,
        ),
        (
            "policy",
            r#"
@policy
  allow: [{ op: "message_emit", name: "user#1" }]
"#,
        ),
        (
            "interface",
            r#"
@interface WeatherAPI
  fn get_current(city: string) -> string (effect="read")
"#,
        ),
        (
            "test",
            r#"
@test(name="basic")
  input:
    query: "hello"
  assert:
    - "canonical contains hello"
"#,
        ),
        (
            "import_plus_block",
            r#"
@import "./module.facet"

@user
  content: "x"
"#,
        ),
    ];

    for (name, src) in cases {
        let doc = parse_document(src).unwrap_or_else(|e| panic!("case '{}' failed: {}", name, e));
        assert!(!doc.blocks.is_empty(), "case '{}' produced empty AST", name);
    }
}

#[test]
fn pipeline_shapes_matrix_parses() {
    let cases = [
        (
            "varref_path_multistep",
            r#"
@vars
  out: $doc.path |> trim() |> replace("a", "b")
"#,
        ),
        (
            "literal_with_named_args",
            r#"
@vars
  out: "a,b,c" |> split(",") |> json(indent=2)
"#,
        ),
        (
            "input_directive_pipeline",
            r#"
@vars
  query: @input(type="string") |> trim()
"#,
        ),
        (
            "inline_list_pipeline",
            r#"
@vars
  out: ["a", "b"] |> ensure_list()
"#,
        ),
        (
            "inline_map_pipeline",
            r#"
@vars
  out: { a: 1, b: 2 } |> json(indent=0)
"#,
        ),
    ];

    for (name, src) in cases {
        parse_document(src).unwrap_or_else(|e| panic!("pipeline case '{}' failed: {}", name, e));
    }
}
