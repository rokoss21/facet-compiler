use fct_engine::{TestRunner, TestTelemetry};
use fct_ast::{TestBlock, MockDefinition, Assertion, AssertionKind, ValueNode, Span};
use std::collections::HashMap;

#[test]
fn test_runner_discovery() {
    let runner = TestRunner::new(1000, 4096);
    
    // Create a mock document with test blocks
    let doc = fct_ast::FacetDocument {
        blocks: vec![
            fct_ast::FacetNode::Test(TestBlock {
                name: "test1".to_string(),
                vars: HashMap::new(),
                mocks: Vec::new(),
                assertions: vec![], 
                body: Vec::new(),
                span: Span { start: 0, end: 10, line: 1, column: 1 },
            }),
            fct_ast::FacetNode::Test(TestBlock {
                name: "test2".to_string(),
                vars: HashMap::new(),
                mocks: Vec::new(),
                assertions: vec![],
                body: Vec::new(),
                span: Span { start: 20, end: 30, line: 2, column: 1 },
            }),
        ],
        span: Span { start: 0, end: 30, line: 1, column: 1 },
    };
    
    let tests = runner.discover_tests(&doc);
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "test1");
    assert_eq!(tests[1].name, "test2");
}

#[test]
fn test_runner_with_vars() {
    let runner = TestRunner::new(1000, 4096);
    
    let mut vars = HashMap::new();
    vars.insert("input".to_string(), ValueNode::String("test value".to_string()));
    
    let test_block = TestBlock {
        name: "vars test".to_string(),
        vars,
        mocks: Vec::new(),
        assertions: vec![
            Assertion {
                kind: AssertionKind::Equals {
                    target: "output".to_string(),
                    expected: ValueNode::String("test value".to_string()),
                },
                span: Span { start: 0, end: 10, line: 1, column: 1 },
            },
        ],
        body: Vec::new(),
        span: Span { start: 0, end: 10, line: 1, column: 1 },
    };
    
    // Create a minimal document
    let doc = fct_ast::FacetDocument {
        blocks: vec![
            fct_ast::FacetNode::Vars(fct_ast::FacetBlock {
                name: "vars".to_string(),
                attributes: HashMap::new(),
                body: vec![],
                span: Span { start: 0, end: 0, line: 1, column: 1 },
            }),
        ],
        span: Span { start: 0, end: 0, line: 1, column: 1 },
    };
    
    let result = runner.run_test(&doc, &test_block);
    assert!(result.is_ok());
    
    let test_result = result.unwrap();
    assert_eq!(test_result.name, "vars test");
    assert!(test_result.assertions.len() > 0);
}

#[test]
fn test_runner_with_mocks() {
    let runner = TestRunner::new(1000, 4096);
    
    let mocks = vec![
        MockDefinition {
            target: "WeatherAPI.get_current".to_string(),
            return_value: ValueNode::Map({
                let mut map = HashMap::new();
                map.insert("temp".to_string(), ValueNode::Scalar(fct_ast::ScalarValue::Int(25)));
                map.insert("condition".to_string(), ValueNode::String("Sunny".to_string()));
                map
            }),
            span: Span { start: 0, end: 10, line: 1, column: 1 },
        },
    ];
    
    let test_block = TestBlock {
        name: "mock test".to_string(),
        vars: HashMap::new(),
        mocks,
        assertions: vec![
            Assertion {
                kind: AssertionKind::Contains {
                    target: "output".to_string(),
                    text: "Sunny".to_string(),
                },
                span: Span { start: 0, end: 10, line: 1, column: 1 },
            },
        ],
        body: Vec::new(),
        span: Span { start: 0, end: 10, line: 1, column: 1 },
    };
    
    let doc = fct_ast::FacetDocument {
        blocks: vec![],
        span: Span { start: 0, end: 0, line: 1, column: 1 },
    };
    
    let result = runner.run_test(&doc, &test_block);
    assert!(result.is_ok());
    
    let test_result = result.unwrap();
    assert_eq!(test_result.name, "mock test");
}

#[test]
fn test_assertion_contains() {
    let runner = TestRunner::new(1000, 4096);
    
    let assertions = vec![
        Assertion {
            kind: AssertionKind::Contains {
                target: "output".to_string(),
                text: "hello".to_string(),
            },
            span: Span { start: 0, end: 10, line: 1, column: 1 },
        },
        Assertion {
            kind: AssertionKind::NotContains {
                target: "output".to_string(),
                text: "world".to_string(),
            },
            span: Span { start: 20, end: 30, line: 2, column: 1 },
        },
    ];
    
    let ctx = fct_engine::TestContext {
        execution_ctx: fct_engine::ExecutionContext::new(1000),
        mock_registry: fct_engine::MockRegistry::default(),
        telemetry: TestTelemetry {
            tokens_used: 50,
            estimated_cost: 0.001,
            execution_time_ms: 100,
            gas_consumed: 10,
            variables_computed: 5,
        },
    };
    
    let output = "hello there!";
    let results = runner.evaluate_assertions(output, &ctx, &assertions);
    
    assert_eq!(results.len(), 2);
    assert!(results[0].passed); // contains "hello"
    assert!(results[1].passed); // not contains "world"
}

#[test]
fn test_assertion_comparisons() {
    let runner = TestRunner::new(1000, 4096);
    
    let assertions = vec![
        Assertion {
            kind: AssertionKind::LessThan {
                field: "cost".to_string(),
                value: 0.01,
            },
            span: Span { start: 0, end: 10, line: 1, column: 1 },
        },
        Assertion {
            kind: AssertionKind::GreaterThan {
                field: "tokens".to_string(),
                value: 10.0,
            },
            span: Span { start: 20, end: 30, line: 2, column: 1 },
        },
    ];
    
    let ctx = fct_engine::TestContext {
        execution_ctx: fct_engine::ExecutionContext::new(1000),
        mock_registry: fct_engine::MockRegistry::default(),
        telemetry: TestTelemetry {
            tokens_used: 50,
            estimated_cost: 0.005,
            execution_time_ms: 100,
            gas_consumed: 10,
            variables_computed: 5,
        },
    };
    
    let output = "some output";
    let results = runner.evaluate_assertions(output, &ctx, &assertions);
    
    assert_eq!(results.len(), 2);
    assert!(results[0].passed); // cost 0.005 < 0.01
    assert!(results[1].passed); // tokens 50 > 10
}

#[test]
fn test_assertion_sentiment() {
    let runner = TestRunner::new(1000, 4096);
    
    let assertions = vec![
        Assertion {
            kind: AssertionKind::Sentiment {
                target: "output".to_string(),
                expected: "positive".to_string(),
            },
            span: Span { start: 0, end: 10, line: 1, column: 1 },
        },
    ];
    
    let ctx = fct_engine::TestContext {
        execution_ctx: fct_engine::ExecutionContext::new(1000),
        mock_registry: fct_engine::MockRegistry::default(),
        telemetry: TestTelemetry {
            tokens_used: 50,
            estimated_cost: 0.005,
            execution_time_ms: 100,
            gas_consumed: 10,
            variables_computed: 5,
        },
    };
    
    let positive_output = "This is great and helpful!";
    let results = runner.evaluate_assertions(positive_output, &ctx, &assertions);
    assert!(results[0].passed);
    
    let negative_output = "This is bad and terrible";
    let results = runner.evaluate_assertions(negative_output, &ctx, &assertions);
    assert!(!results[0].passed);
}