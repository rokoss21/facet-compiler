use fct_ast::{FacetDocument, FacetNode, ValueNode};
use fct_engine::{ExecutionContext, RDagEngine, Section, TokenBoxModel};
use fct_parser::{compute_document_hash, parse_document};
use fct_render::{to_json_compact, CanonicalPayload, RenderContext, Renderer};
use fct_std::LensRegistry;
use fct_validator::TypeChecker;

fn doc_to_sections(doc: &FacetDocument) -> Vec<Section> {
    let mut sections = Vec::new();

    for block in &doc.blocks {
        let (id, content) = match block {
            FacetNode::System(_) => {
                ("system".to_string(), ValueNode::String("System block".to_string()))
            }
            FacetNode::User(_) => {
                ("user".to_string(), ValueNode::String("User block".to_string()))
            }
            FacetNode::Assistant(_) => {
                ("assistant".to_string(), ValueNode::String("Assistant block".to_string()))
            }
            _ => continue,
        };

        sections.push(Section::new(id, content, 100));
    }

    sections
}

fn render_once(source: &str) -> CanonicalPayload {
    let doc = parse_document(source).expect("parse");

    let mut checker = TypeChecker::new();
    checker.validate(&doc).expect("validate");

    let mut engine = RDagEngine::new();
    engine.build(&doc).expect("build DAG");
    engine.validate().expect("validate DAG");

    let mut ctx = ExecutionContext::new(10_000);
    engine.execute(&mut ctx).expect("execute DAG");

    let sections = doc_to_sections(&doc);
    let allocation = TokenBoxModel::new(4096)
        .allocate(sections, &LensRegistry::new())
        .expect("layout");

    let renderer = Renderer::new();
    renderer
        .render_with_context(
            &doc,
            &allocation,
            RenderContext {
                document_hash: Some(compute_document_hash(source)),
                policy_hash: None,
                profile: Some("hypervisor".to_string()),
                mode: Some("exec".to_string()),
                host_profile_id: Some("local.default.v1".to_string()),
                budget_units: Some(4096),
                target_provider_id: Some("generic-llm".to_string()),
                computed_vars: Some(ctx.variables.clone()),
            },
        )
        .expect("render")
}

#[test]
fn canonical_output_is_stable_across_repeated_runs() {
    let source = r#"
@meta
  version: "1.0"
@system
  content: "You are deterministic."
@user
  content: "Hello"
"#;

    let first = to_json_compact(&render_once(source)).expect("json");
    for _ in 0..4 {
        let next = to_json_compact(&render_once(source)).expect("json");
        assert_eq!(first, next);
    }
}

#[test]
fn canonical_output_has_no_legacy_v20_top_level_fields() {
    let source = r#"
@system
  content: "You are deterministic."
@user
  content: "Hello"
"#;

    let payload = render_once(source);
    let value: serde_json::Value =
        serde_json::from_str(&to_json_compact(&payload).expect("json")).expect("valid json");

    let obj = value.as_object().expect("top-level object");
    assert_eq!(obj.len(), 3, "expected only metadata/tools/messages");
    assert!(obj.contains_key("metadata"));
    assert!(obj.contains_key("tools"));
    assert!(obj.contains_key("messages"));

    assert!(!obj.contains_key("system_prompt"));
    assert!(!obj.contains_key("user_prompt"));
    assert!(!obj.contains_key("assistant_prompt"));
    assert!(!obj.contains_key("context_window"));
    assert!(!obj.contains_key("variables"));
}
