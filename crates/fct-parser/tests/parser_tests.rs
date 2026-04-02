use fct_ast::FacetNode;
use fct_parser::parse_document;

#[test]
fn test_parse_simple_system_block() {
    let source = r#"
@system
  role: "assistant"
  model: "gpt-4"
"#;

    let doc = parse_document(source).expect("Failed to parse document");
    assert_eq!(doc.blocks.len(), 1);

    match &doc.blocks[0] {
        FacetNode::System(block) => {
            assert_eq!(block.name, "system");
            assert!(block.body.len() >= 2);
        }
        _ => panic!("Expected System block"),
    }
}

#[test]
fn test_parse_variables() {
    let source = r#"
@vars
  name: "Alice"
  count: 42
  active: true
"#;
    let doc = parse_document(source).expect("Failed to parse vars");
    assert_eq!(doc.blocks.len(), 1);
}

#[test]
fn test_parse_pipeline() {
    let source = r#"
@vars
  processed: $input |> trim() |> uppercase()
"#;
    let _doc = parse_document(source).expect("Failed to parse pipeline");
    // Successful parse is the target here.
}
