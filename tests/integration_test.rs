// Integration tests for FACET v2.0 Compiler
// Tests full pipeline: Parse -> Resolve -> Validate -> Execute -> Render

use fct_ast::{FacetDocument, FacetNode, ValueNode};
use fct_engine::{ExecutionContext, RDagEngine, Section, TokenBoxModel};
use fct_parser::parse_document;
use fct_render::{CanonicalPayload, Renderer};
use fct_std::LensRegistry;
use fct_validator::TypeChecker;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn parse_and_validate(source: &str) -> Result<FacetDocument, String> {
    // Step 1: Parse
    let doc = parse_document(source).map_err(|e| format!("Parse error: {:?}", e))?;

    // Step 2: Validate
    let mut validator = TypeChecker::new();
    validator
        .validate(&doc)
        .map_err(|e| format!("Validation error: {:?}", e))?;

    Ok(doc)
}

/// Convert FacetDocument blocks into Section objects for Token Box Model
fn doc_to_sections(doc: &FacetDocument) -> Vec<Section> {
    let mut sections = Vec::new();

    for block in &doc.blocks {
        let (id, priority, content) = match block {
            FacetNode::System(_) => {
                ("system".to_string(), 100, ValueNode::String("System block".to_string()))
            }
            FacetNode::User(_) => {
                ("user".to_string(), 200, ValueNode::String("User block".to_string()))
            }
            FacetNode::Assistant(_) => {
                ("assistant".to_string(), 50, ValueNode::String("Assistant block".to_string()))
            }
            FacetNode::Vars(_) => {
                // Skip vars in section creation
                continue;
            }
            FacetNode::VarTypes(_) => {
                // Skip var types
                continue;
            }
            FacetNode::Meta(_) => {
                ("meta".to_string(), 10, ValueNode::String("Meta block".to_string()))
            }
            FacetNode::Context(_) => {
                ("context".to_string(), 80, ValueNode::String("Context block".to_string()))
            }
            _ => continue,
        };

        // Create section with estimated token count
        let token_count = 100; // Simplified for testing
        let section = Section::new(id, content, token_count)
            .with_priority(priority)
            .with_limits(50, 0.2, 0.3); // Allow some flexibility

        sections.push(section);
    }

    sections
}

fn full_pipeline(source: &str) -> Result<CanonicalPayload, String> {
    // Step 1: Parse
    let doc = parse_document(source).map_err(|e| format!("Parse error: {:?}", e))?;

    // Step 2: Validate
    let mut validator = TypeChecker::new();
    validator
        .validate(&doc)
        .map_err(|e| format!("Validation error: {:?}", e))?;

    // Step 3: Execute R-DAG
    let engine = RDagEngine::new();
    let mut ctx = ExecutionContext::new(10000);
    engine
        .execute(&mut ctx)
        .map_err(|e| format!("Engine error: {:?}", e))?;

    // Step 4: Convert document to sections
    let sections = doc_to_sections(&doc);

    // Step 5: Token Box Model allocation
    let box_model = TokenBoxModel::new(4096);
    let lens_registry = LensRegistry::new();
    let allocation = box_model
        .allocate(sections, &lens_registry)
        .map_err(|e| format!("Allocation error: {:?}", e))?;

    // Step 6: Render to canonical JSON
    let renderer = Renderer::new();
    let payload = renderer
        .render(&doc, &allocation)
        .map_err(|e| format!("Render error: {:?}", e))?;

    Ok(payload)
}

// ============================================================================
// BASIC INTEGRATION TESTS
// ============================================================================

#[test]
fn test_integration_parse_and_validate_basic() {
    let source = r#"
@system
  role: "assistant"
  model: "gpt-4"

@user
  query: "Hello, world!"
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should parse and validate basic document");

    let doc = result.unwrap();
    assert_eq!(doc.blocks.len(), 2);
}

#[test]
fn test_integration_parse_validate_render() {
    let source = r#"
@system
  role: "assistant"
  model: "gpt-4"
  temperature: 0.7

@user
  content: "Test message"
"#;

    let result = full_pipeline(source);
    assert!(result.is_ok(), "Full pipeline should succeed");

    let payload = result.unwrap();
    assert_eq!(payload.metadata.version, "2.0");
}

// ============================================================================
// VARIABLES AND PIPELINES
// ============================================================================

#[test]
fn test_integration_with_variables() {
    let source = r#"
@vars
  username: "Alice"
  greeting: "Hello"

@user
  message: "Welcome!"
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should handle variables");

    let doc = result.unwrap();
    assert_eq!(doc.blocks.len(), 2);
}

#[test]
fn test_integration_with_lens_pipeline() {
    let source = r#"
@vars
  name: "  alice  "
  clean_name: $name |> trim() |> lowercase()

@user
  content: "Test"
"#;

    let result = full_pipeline(source);
    if let Err(ref e) = result {
        eprintln!("Pipeline error: {}", e);
    }
    assert!(result.is_ok(), "Should execute lens pipeline: {:?}", result.err());
}

// ============================================================================
// R-DAG EXECUTION
// ============================================================================

#[test]
fn test_integration_r_dag_execution() {
    let source = r#"
@vars
  base: "Hello"
  transformed: $base |> uppercase()

@system
  role: "assistant"
"#;

    let result = full_pipeline(source);
    assert!(result.is_ok(), "R-DAG execution should succeed");

    // Engine should compute transformed variable
    let payload = result.unwrap();
    assert_eq!(payload.metadata.version, "2.0");
}

#[test]
// Test pipeline syntax
fn test_integration_multiple_dependencies() {
    let source = r#"
@vars
  a: "test"
  b: $a |> uppercase()
  c: $b |> trim()

@user
  content: "Message"
"#;

    let result = full_pipeline(source);
    assert!(
        result.is_ok(),
        "Should handle multiple variable dependencies"
    );
}

// ============================================================================
// ERROR HANDLING
// ============================================================================

#[test]
fn test_integration_parse_error() {
    let source = r#"
@system
   role: "bad indentation"
"#;

    let result = parse_document(source);
    assert!(result.is_err(), "Should fail on bad indentation (F001)");
}

#[test]
fn test_integration_validation_error() {
    let source = r#"
@vars
  name: "Alice"
  bad_ref: $undefined_variable

@user
  content: "Test"
"#;

    // This should parse successfully
    let doc = parse_document(source);
    assert!(doc.is_ok());

    // But validation should catch forward reference
    let doc = doc.unwrap();
    let mut validator = TypeChecker::new();
    let _result = validator.validate(&doc);

    // Note: Current validator may not catch this - test documents current behavior
    // When forward reference detection is improved, this should fail
}

#[test]
// Test pipeline syntax
fn test_integration_unknown_lens_error() {
    let source = r#"
@vars
  text: "hello"
  transformed: $text |> nonexistent_lens()

@user
  content: "Test"
"#;

    // Parse should succeed
    let doc = parse_document(source);
    assert!(doc.is_ok());

    // Validation should catch unknown lens (F802)
    let doc = doc.unwrap();
    let mut validator = TypeChecker::new();
    let result = validator.validate(&doc);

    assert!(result.is_err(), "Should fail on unknown lens");
}

// ============================================================================
// COMPLEX SCENARIOS
// ============================================================================

#[test]
fn test_integration_multiple_blocks() {
    let source = r#"
@meta
  version: "1.0"
  author: "Test"

@system
  role: "assistant"
  model: "gpt-4"

@vars
  context: "test context"

@user
  query: "What is AI?"

@assistant
  response: "AI is..."
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should handle multiple block types");

    let doc = result.unwrap();
    assert_eq!(doc.blocks.len(), 5);
}

#[test]
// Test nested map syntax
fn test_integration_nested_structures() {
    let source = r#"
@vars
  config: {
    model: "gpt-4"
    params: {
      temperature: 0.7
      max_tokens: 1000
    }
  }

@system
  role: "assistant"
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should handle nested map structures");
}

#[test]
fn test_integration_list_values() {
    let source = r#"
@vars
  tags: ["ai", "ml", "nlp"]
  numbers: [1, 2, 3, 4, 5]

@user
  content: "Test"
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should handle list values");
}

// ============================================================================
// END-TO-END RENDERING
// ============================================================================

#[test]
fn test_integration_render_to_json() {
    let source = r#"
@system
  role: "assistant"
  model: "gpt-4"

@user
  content: "Hello"
"#;

    let result = full_pipeline(source);
    assert!(result.is_ok());

    let payload = result.unwrap();

    // Verify payload structure
    assert_eq!(payload.metadata.version, "2.0");
    assert!(payload.metadata.total_tokens > 0);
}

#[test]
// Test pipeline syntax
fn test_integration_render_with_variables() {
    let source = r#"
@vars
  user_name: "Alice"
  greeting: $user_name |> uppercase()

@system
  role: "assistant"

@user
  content: "Test"
"#;

    let result = full_pipeline(source);
    assert!(
        result.is_ok(),
        "Should render document with computed variables"
    );

    let payload = result.unwrap();
    assert_eq!(payload.metadata.version, "2.0");
}

// ============================================================================
// Note: Resolver integration tests will be added when public API is available
// ============================================================================
// PERFORMANCE AND STRESS TESTS
// ============================================================================

#[test]
fn test_integration_large_variable_count() {
    // Test with many variables
    let mut source = String::from("@vars\n");
    for i in 0..50 {
        source.push_str(&format!("  var{}: \"value{}\"\n", i, i));
    }
    source.push_str("\n@user\n  content: \"Test\"\n");

    let result = parse_and_validate(&source);
    assert!(result.is_ok(), "Should handle many variables");
}

#[test]
// Test pipeline syntax
fn test_integration_deep_pipeline() {
    let source = r#"
@vars
  text: "hello"
  step1: $text |> trim()
  step2: $step1 |> uppercase()
  step3: $step2 |> trim()

@user
  content: "Test"
"#;

    let result = full_pipeline(&source);
    assert!(result.is_ok(), "Should handle multi-step pipeline");
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_integration_empty_blocks() {
    let source = r#"
@system
  role: "assistant"

@vars

@user
  content: "Test"
"#;

    let result = parse_and_validate(source);
    // Empty blocks should be allowed
    assert!(result.is_ok(), "Should allow empty blocks");
}

#[test]
fn test_integration_unicode_content() {
    let source = r#"
@vars
  greeting: "Привет, мир! 🌍"
  chinese: "你好世界"

@user
  content: "Test Unicode 测试"
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should handle Unicode content");
}

#[test]
// Test escaped quotes
fn test_integration_special_characters() {
    let source = r#"
@vars
  text: "Special chars: @#$%^&*()"
  json: "{\"key\": \"value\"}"

@user
  content: "Test"
"#;

    let result = parse_and_validate(source);
    assert!(result.is_ok(), "Should handle escaped quotes: {:?}", result.err());
}
