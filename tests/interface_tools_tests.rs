use fct_ast::{
    FacetDocument, FacetNode, FunctionSignature, InterfaceNode, Parameter, Span, TypeNode,
    ValueNode,
};
use fct_engine::{Section, TokenBoxModel};
use fct_parser::{compute_document_hash, parse_document};
use fct_render::{RenderContext, Renderer};
use fct_std::LensRegistry;
use fct_validator::TypeChecker;

fn validate(source: &str) -> Result<fct_ast::FacetDocument, String> {
    let doc = parse_document(source)?;
    let mut checker = TypeChecker::new();
    checker.validate(&doc).map_err(|e| e.to_string())?;
    Ok(doc)
}

#[test]
fn validator_rejects_unknown_system_tool_reference() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string (effect="read")

@system
  tools: [$MissingAPI]
  content: "x"
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn validator_rejects_duplicate_interface_names() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string (effect="read")

@interface WeatherAPI
  fn get2(city: string) -> string (effect="read")
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn validator_rejects_duplicate_function_names_within_interface() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string (effect="read")
  fn get(city: string) -> string (effect="read")
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn validator_rejects_missing_function_effect() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F456"), "expected F456, got: {}", err);
}

#[test]
fn validator_rejects_invalid_function_effect() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string (effect="unknown")
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F456"), "expected F456, got: {}", err);
}

#[test]
fn validator_accepts_namespaced_effect_class() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string (effect="x.acme.custom")
"#;
    let result = validate(source);
    assert!(
        result.is_ok(),
        "expected valid namespaced effect, got {:?}",
        result
    );
}

#[test]
fn validator_rejects_unmappable_interface_primitive_type() {
    let source = r#"
@interface WeatherAPI
  fn get(city: custom) -> string (effect="read")
"#;
    let err = validate(source).unwrap_err();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn validator_rejects_unmappable_interface_image_type() {
    let span = Span {
        start: 0,
        end: 0,
        line: 1,
        column: 1,
    };
    let doc = FacetDocument {
        blocks: vec![FacetNode::Interface(InterfaceNode {
            name: "VisionAPI".to_string(),
            functions: vec![FunctionSignature {
                name: "describe".to_string(),
                params: vec![Parameter {
                    name: "image".to_string(),
                    type_node: TypeNode::Image {
                        max_dim: Some(1024),
                        format: Some("jpeg".to_string()),
                    },
                    span: span.clone(),
                }],
                return_type: TypeNode::Primitive("string".to_string()),
                effect: Some("read".to_string()),
                span: span.clone(),
            }],
            span: span.clone(),
        })],
        span,
    };

    let mut checker = TypeChecker::new();
    let err = checker.validate(&doc).unwrap_err().to_string();
    assert!(err.contains("F452"), "expected F452, got: {}", err);
}

#[test]
fn renderer_emits_only_system_referenced_tools() {
    let source = r#"
@interface WeatherAPI
  fn get(city: string) -> string (effect="read")

@interface PaymentAPI
  fn charge(amount: int) -> string (effect="payment")

@system
  tools: [$PaymentAPI]
  content: "x"
"#;

    let doc = validate(source).expect("document should validate");

    let mut sections = Vec::new();
    for block in &doc.blocks {
        if let FacetNode::System(system) = block {
            let content = system
                .body
                .iter()
                .find_map(|entry| match entry {
                    fct_ast::BodyNode::KeyValue(kv) if kv.key == "content" => {
                        Some(kv.value.clone())
                    }
                    _ => None,
                })
                .unwrap_or(ValueNode::String("system".to_string()));

            sections.push(Section::new("system".to_string(), content, 10));
        }
    }

    let box_model = TokenBoxModel::new(1000);
    let allocation = box_model
        .allocate(sections, &LensRegistry::new())
        .expect("allocation should succeed");
    let payload = Renderer::new()
        .render_with_context(
            &doc,
            &allocation,
            RenderContext {
                document_hash: Some(compute_document_hash(source)),
                policy_hash: None,
                profile: Some("hypervisor".to_string()),
                mode: Some("exec".to_string()),
                host_profile_id: Some("local.default.v1".to_string()),
                budget_units: Some(1000),
                target_provider_id: Some("test-provider".to_string()),
                computed_vars: None,
            },
        )
        .expect("render should succeed");

    let names: Vec<String> = payload
        .tools
        .iter()
        .map(|t| t.function.name.clone())
        .collect();
    assert_eq!(names, vec!["charge".to_string()]);
}
