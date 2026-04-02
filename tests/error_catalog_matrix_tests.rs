use fct_ast::{
    BodyNode, FacetBlock, FacetDocument, FacetNode, KeyValueNode, LensCallNode, OrderedMap,
    PipelineNode, Span, ValueNode,
};
use fct_engine::{ExecutionContext, ExecutionMode, RDagEngine, Section, TokenBoxModel};
use fct_parser::parse_document;
use fct_resolver::{Resolver, ResolverConfig};
use fct_std::LensRegistry;
use fct_validator::{TypeChecker, ValidationProfile};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn validate_source(source: &str) -> Result<FacetDocument, String> {
    let doc = parse_document(source)?;
    let mut checker = TypeChecker::new();
    checker.validate(&doc).map_err(|e| e.to_string())?;
    Ok(doc)
}

fn execute_source(source: &str, gas_limit: usize, mode: ExecutionMode) -> Result<(), String> {
    let doc = validate_source(source)?;
    let mut engine = RDagEngine::new();
    engine.build(&doc).map_err(|e| e.to_string())?;
    engine.validate().map_err(|e| e.to_string())?;
    let mut ctx = ExecutionContext::new_with_mode(gas_limit, mode);
    engine.execute(&mut ctx).map_err(|e| e.to_string())
}

fn execute_source_with_input(
    source: &str,
    gas_limit: usize,
    mode: ExecutionMode,
    input_name: &str,
    input_value: ValueNode,
) -> Result<(), String> {
    let doc = validate_source(source)?;
    let mut engine = RDagEngine::new();
    engine.build(&doc).map_err(|e| e.to_string())?;
    engine.validate().map_err(|e| e.to_string())?;
    let mut ctx = ExecutionContext::new_with_mode(gas_limit, mode);
    ctx.set_input(input_name.to_string(), input_value);
    engine.execute(&mut ctx).map_err(|e| e.to_string())
}

fn assert_error_code(result: Result<(), String>, code: &str) {
    let err = result.expect_err("expected error");
    assert!(
        err.contains(code),
        "expected error code {code}, got: {err}"
    );
}

fn span() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 1,
        column: 1,
    }
}

fn llm_call_vars_doc() -> FacetDocument {
    FacetDocument {
        blocks: vec![FacetNode::Vars(FacetBlock {
            name: "vars".to_string(),
            attributes: OrderedMap::new(),
            body: vec![BodyNode::KeyValue(KeyValueNode {
                key: "out".to_string(),
                key_kind: Default::default(),
                value: ValueNode::Pipeline(PipelineNode {
                    initial: Box::new(ValueNode::String("hello".to_string())),
                    lenses: vec![LensCallNode {
                        name: "llm_call".to_string(),
                        args: vec![],
                        kwargs: OrderedMap::new(),
                        span: span(),
                    }],
                    span: span(),
                }),
                span: span(),
            })],
            span: span(),
        })],
        span: span(),
    }
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    path.push(format!("facet-error-matrix-{prefix}-{}-{ts}", std::process::id()));
    fs::create_dir_all(&path).expect("temp dir should be creatable");
    path
}

#[test]
fn matrix_f001_invalid_indentation() {
    let source = "@vars\n x: \"bad\"\n";
    let err = parse_document(source).expect_err("expected parse error");
    assert!(err.contains("F001"), "expected F001, got: {err}");
}

#[test]
fn matrix_f002_tabs_forbidden() {
    let source = "@vars\n\tx: \"bad\"\n";
    let err = parse_document(source).expect_err("expected parse error");
    assert!(err.contains("F002"), "expected F002, got: {err}");
}

#[test]
fn matrix_f003_malformed_syntax() {
    let source = "@vars\n  x: \"unterminated\n";
    let err = parse_document(source).expect_err("expected parse error");
    assert!(err.contains("F003"), "expected F003, got: {err}");
}

#[test]
fn matrix_f402_attribute_interpolation_forbidden() {
    let source = "@system(when=\"{{blocked}}\")\n  content: \"x\"\n";
    let err = parse_document(source).expect_err("expected parse error");
    assert!(err.contains("F402"), "expected F402, got: {err}");
}

#[test]
fn matrix_f401_unknown_variable() {
    let source = "@vars\n  out: $missing\n";
    assert_error_code(execute_source(source, 1000, ExecutionMode::Exec), "F401");
}

#[test]
fn matrix_f405_invalid_variable_path() {
    let source = "@vars\n  obj: { a: \"ok\" }\n  out: $obj.missing\n";
    assert_error_code(execute_source(source, 1000, ExecutionMode::Exec), "F405");
}

#[test]
fn matrix_f451_type_mismatch() {
    let source = "@var_types\n  age: \"int\"\n@vars\n  age: \"oops\"\n";
    let err = validate_source(source).expect_err("expected validation error");
    assert!(err.contains("F451"), "expected F451, got: {err}");
}

#[test]
fn matrix_f452_constraint_violation() {
    let source = "@vars\n  cfg: {\"x-y\": 1}\n";
    let err = validate_source(source).expect_err("expected validation error");
    assert!(err.contains("F452"), "expected F452, got: {err}");
}

#[test]
fn matrix_f453_runtime_input_validation() {
    let source = "@vars\n  n: @input(type=\"int\")\n";
    assert_error_code(
        execute_source_with_input(
            source,
            1000,
            ExecutionMode::Exec,
            "n",
            ValueNode::String("oops".to_string()),
        ),
        "F453",
    );
}

#[test]
fn matrix_f454_policy_deny() {
    let doc = llm_call_vars_doc();
    let mut engine = RDagEngine::new();
    engine.build(&doc).expect("build should succeed");
    engine.validate().expect("graph should validate");
    let mut ctx = ExecutionContext::new_with_mode(1000, ExecutionMode::Exec);
    let err = engine.execute(&mut ctx).expect_err("expected policy deny");
    let msg = err.to_string();
    assert!(msg.contains("F454"), "expected F454, got: {msg}");
}

#[test]
fn matrix_f455_guard_undecidable() {
    let mut rule = OrderedMap::new();
    rule.insert("op".to_string(), ValueNode::String("lens_call".to_string()));
    rule.insert("name".to_string(), ValueNode::String("llm_call".to_string()));
    rule.insert(
        "when".to_string(),
        ValueNode::Variable("missing.flag".to_string()),
    );

    let doc = FacetDocument {
        blocks: vec![
            FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "allow".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::List(vec![ValueNode::Map(rule)]),
                    span: span(),
                })],
                span: span(),
            }),
            FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "out".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "llm_call".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
                            span: span(),
                        }],
                        span: span(),
                    }),
                    span: span(),
                })],
                span: span(),
            }),
        ],
        span: span(),
    };

    let mut engine = RDagEngine::new();
    engine.build(&doc).expect("build should succeed");
    engine.validate().expect("graph should validate");
    let mut ctx = ExecutionContext::new_with_mode(1000, ExecutionMode::Exec);
    let err = engine
        .execute(&mut ctx)
        .expect_err("expected guard undecidable");
    let msg = err.to_string();
    assert!(msg.contains("F455"), "expected F455, got: {msg}");
}

#[test]
fn matrix_f456_invalid_effect_declaration() {
    let source = "@interface WeatherAPI\n  fn get_current(city: string) -> string\n";
    let err = validate_source(source).expect_err("expected validation error");
    assert!(err.contains("F456"), "expected F456, got: {err}");
}

#[test]
fn matrix_f505_cycle_detected() {
    let source = "@vars\n  a: $b\n  b: $a\n";
    assert_error_code(execute_source(source, 1000, ExecutionMode::Exec), "F505");
}

#[test]
fn matrix_f601_import_not_found() {
    let source = "@import \"missing___error_matrix___not_found.facet\"\n";
    let err = validate_source(source).expect_err("expected validation error");
    assert!(err.contains("F601"), "expected F601, got: {err}");
}

#[test]
fn matrix_f602_import_cycle() {
    let dir = unique_temp_dir("f602");
    let root_source = "@import \"a.facet\"\n";
    fs::write(dir.join("a.facet"), "@import \"b.facet\"\n").expect("write a.facet");
    fs::write(dir.join("b.facet"), "@import \"a.facet\"\n").expect("write b.facet");

    let parsed = parse_document(root_source).expect("root document should parse");
    let canonical_dir = fs::canonicalize(&dir).expect("temp dir should canonicalize");
    let mut resolver = Resolver::new(ResolverConfig {
        allowed_roots: vec![canonical_dir.clone()],
        base_dir: canonical_dir,
    });

    let err = resolver
        .resolve(parsed)
        .expect_err("expected resolver cycle error");
    let msg = err.to_string();
    assert!(msg.contains("F602"), "expected F602, got: {msg}");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn matrix_f801_profile_disallow() {
    let source = "@vars\n  x: $y\n";
    let doc = parse_document(source).expect("source should parse");
    let mut checker = TypeChecker::new().with_profile(ValidationProfile::Core);
    let err = checker
        .validate(&doc)
        .expect_err("expected core profile violation");
    let msg = err.to_string();
    assert!(msg.contains("F801"), "expected F801, got: {msg}");
}

#[test]
fn matrix_f802_unknown_lens() {
    let source = "@vars\n  x: \"hello\" |> unknown_lens()\n";
    let err = validate_source(source).expect_err("expected validation error");
    assert!(err.contains("F802"), "expected F802, got: {err}");
}

#[test]
fn matrix_f803_pure_cache_miss() {
    let doc = llm_call_vars_doc();
    let mut engine = RDagEngine::new();
    engine.build(&doc).expect("build should succeed");
    engine.validate().expect("graph should validate");
    let mut ctx = ExecutionContext::new_with_mode(1000, ExecutionMode::Pure);
    let err = engine
        .execute(&mut ctx)
        .expect_err("expected pure cache miss");
    let msg = err.to_string();
    assert!(msg.contains("F803"), "expected F803, got: {msg}");
}

#[test]
fn matrix_f901_critical_overflow() {
    let section = Section::new(
        "critical".to_string(),
        ValueNode::String("x".repeat(100)),
        100,
    )
    .with_limits(100, 0.0, 0.0);
    let model = TokenBoxModel::new(10);
    let err = model
        .allocate(vec![section], &LensRegistry::new())
        .expect_err("expected critical overflow");
    let msg = err.to_string();
    assert!(msg.contains("F901"), "expected F901, got: {msg}");
}

#[test]
fn matrix_f902_gas_exhausted() {
    let source = "@vars\n  a: \"1\"\n  b: \"2\"\n";
    assert_error_code(execute_source(source, 1, ExecutionMode::Exec), "F902");
}
