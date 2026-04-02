use fct_ast::{
    Assertion, AssertionKind, BodyNode, FacetBlock, FacetNode, FunctionSignature, InterfaceNode,
    KeyValueNode, LensCallNode, MockDefinition, OrderedMap, Parameter, PipelineNode, ScalarValue,
    Span, TestBlock, TypeNode, ValueNode,
};
use fct_engine::{TestRunner, TestTelemetry};

fn span() -> Span {
    Span {
        start: 0,
        end: 0,
        line: 1,
        column: 1,
    }
}

fn weather_interface_read() -> FacetNode {
    FacetNode::Interface(InterfaceNode {
        name: "WeatherAPI".to_string(),
        functions: vec![FunctionSignature {
            name: "get_current".to_string(),
            params: vec![Parameter {
                name: "city".to_string(),
                type_node: TypeNode::Primitive("string".to_string()),
                span: span(),
            }],
            return_type: TypeNode::Primitive("string".to_string()),
            effect: Some("read".to_string()),
            span: span(),
        }],
        span: span(),
    })
}

fn policy_allow_weather_tool_call() -> FacetNode {
    let rule = ValueNode::Map(OrderedMap::from([
        ("op".to_string(), ValueNode::String("tool_call".to_string())),
        (
            "name".to_string(),
            ValueNode::String("WeatherAPI.get_current".to_string()),
        ),
        ("effect".to_string(), ValueNode::String("read".to_string())),
    ]));
    FacetNode::Policy(FacetBlock {
        name: "policy".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "allow".to_string(),
            key_kind: Default::default(),
            value: ValueNode::List(vec![rule]),
            span: span(),
        })],
        span: span(),
    })
}

fn primitive_var_types_block(var: &str, ty: &str) -> FacetNode {
    FacetNode::VarTypes(FacetBlock {
        name: "var_types".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: var.to_string(),
            key_kind: Default::default(),
            value: ValueNode::String(ty.to_string()),
            span: span(),
        })],
        span: span(),
    })
}

#[test]
fn test_runner_discovery() {
    let runner = TestRunner::new(1000, 4096);

    // Create a mock document with test blocks
    let doc = fct_ast::FacetDocument {
        blocks: vec![
            fct_ast::FacetNode::Test(TestBlock {
                name: "test1".to_string(),
                vars: OrderedMap::new(),
                input: OrderedMap::new(),
                mocks: Vec::new(),
                assertions: vec![],
                body: Vec::new(),
                span: Span {
                    start: 0,
                    end: 10,
                    line: 1,
                    column: 1,
                },
            }),
            fct_ast::FacetNode::Test(TestBlock {
                name: "test2".to_string(),
                vars: OrderedMap::new(),
                input: OrderedMap::new(),
                mocks: Vec::new(),
                assertions: vec![],
                body: Vec::new(),
                span: Span {
                    start: 20,
                    end: 30,
                    line: 2,
                    column: 1,
                },
            }),
        ],
        span: Span {
            start: 0,
            end: 30,
            line: 1,
            column: 1,
        },
    };

    let tests = runner.discover_tests(&doc);
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "test1");
    assert_eq!(tests[1].name, "test2");
}

#[test]
fn test_runner_with_vars() {
    let runner = TestRunner::new(1000, 4096);

    let mut vars = OrderedMap::new();
    vars.insert(
        "input".to_string(),
        ValueNode::String("test value".to_string()),
    );

    let test_block = TestBlock {
        name: "vars test".to_string(),
        vars,
        input: OrderedMap::new(),
        mocks: Vec::new(),
        assertions: vec![Assertion {
            kind: AssertionKind::Equals {
                target: "output".to_string(),
                expected: ValueNode::String("test value".to_string()),
            },
            span: Span {
                start: 0,
                end: 10,
                line: 1,
                column: 1,
            },
        }],
        body: Vec::new(),
        span: Span {
            start: 0,
            end: 10,
            line: 1,
            column: 1,
        },
    };

    // Create a minimal document
    let doc = fct_ast::FacetDocument {
        blocks: vec![fct_ast::FacetNode::Vars(fct_ast::FacetBlock {
            name: "vars".to_string(),
            attributes: OrderedMap::new(),
            body: vec![],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        })],
        span: Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        },
    };

    let result = runner.run_test(&doc, &test_block);
    assert!(result.is_ok());

    let test_result = result.unwrap();
    assert_eq!(test_result.name, "vars test");
    assert!(!test_result.assertions.is_empty());
}

#[test]
fn test_runner_with_mocks() {
    let runner = TestRunner::new(1000, 4096);

    let mocks = vec![MockDefinition {
        target: "WeatherAPI.get_current".to_string(),
        return_value: ValueNode::Map({
            let mut map = OrderedMap::new();
            map.insert("temp".to_string(), ValueNode::Scalar(ScalarValue::Int(25)));
            map.insert(
                "condition".to_string(),
                ValueNode::String("Sunny".to_string()),
            );
            map
        }),
        span: Span {
            start: 0,
            end: 10,
            line: 1,
            column: 1,
        },
    }];

    let test_block = TestBlock {
        name: "mock test".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks,
        assertions: vec![Assertion {
            kind: AssertionKind::Contains {
                target: "output".to_string(),
                text: "Sunny".to_string(),
            },
            span: Span {
                start: 0,
                end: 10,
                line: 1,
                column: 1,
            },
        }],
        body: Vec::new(),
        span: Span {
            start: 0,
            end: 10,
            line: 1,
            column: 1,
        },
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read(), policy_allow_weather_tool_call()],
        span: Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        },
    };

    let result = runner.run_test(&doc, &test_block);
    assert!(result.is_ok());

    let test_result = result.unwrap();
    assert_eq!(test_result.name, "mock test");
    let execution_raw = test_result
        .execution_output
        .as_ref()
        .expect("execution output must be present");
    let execution: serde_json::Value =
        serde_json::from_str(execution_raw).expect("execution output must be valid JSON");
    let first_event = execution
        .get("provenance")
        .and_then(|p| p.get("events"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .cloned()
        .expect("expected first guard decision event");
    assert_eq!(
        first_event.get("op").and_then(|v| v.as_str()),
        Some("tool_call")
    );
    assert_eq!(
        first_event.get("name").and_then(|v| v.as_str()),
        Some("WeatherAPI.get_current")
    );
    assert_eq!(
        first_event.get("effect_class").and_then(|v| v.as_str()),
        Some("read")
    );
    assert_eq!(
        first_event.get("mode").and_then(|v| v.as_str()),
        Some("exec")
    );
    assert_eq!(
        first_event.get("decision").and_then(|v| v.as_str()),
        Some("allowed")
    );
    assert!(
        first_event
            .get("input_hash")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .starts_with("sha256:"),
        "tool_call decision must carry deterministic input_hash"
    );
}

#[test]
fn test_runner_mock_tool_call_denied_by_default_policy() {
    let runner = TestRunner::new(1000, 4096);
    let mocks = vec![MockDefinition {
        target: "WeatherAPI.get_current".to_string(),
        return_value: ValueNode::String("ok".to_string()),
        span: span(),
    }];
    let test_block = TestBlock {
        name: "deny-by-default".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks,
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };
    let doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read()],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(!result.passed);
    assert!(result.error.as_deref().unwrap_or_default().contains("F454"));
    let execution_raw = result
        .execution_output
        .as_ref()
        .expect("execution output must be present on failure");
    let execution: serde_json::Value =
        serde_json::from_str(execution_raw).expect("execution output must be valid JSON");
    assert_eq!(
        execution
            .get("provenance")
            .and_then(|p| p.get("events"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("op"))
            .and_then(|v| v.as_str()),
        Some("tool_call")
    );
    assert_eq!(
        execution
            .get("provenance")
            .and_then(|p| p.get("events"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("error_code"))
            .and_then(|v| v.as_str()),
        Some("F454")
    );
}

#[test]
fn test_runner_mock_tool_call_undecidable_policy() {
    let runner = TestRunner::new(1000, 4096);
    let mocks = vec![MockDefinition {
        target: "WeatherAPI.get_current".to_string(),
        return_value: ValueNode::String("ok".to_string()),
        span: span(),
    }];
    let test_block = TestBlock {
        name: "undecidable-guard".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks,
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let rule = ValueNode::Map(OrderedMap::from([
        ("op".to_string(), ValueNode::String("tool_call".to_string())),
        (
            "name".to_string(),
            ValueNode::String("WeatherAPI.get_current".to_string()),
        ),
        (
            "when".to_string(),
            ValueNode::Variable("missing.flag".to_string()),
        ),
    ]));
    let doc = fct_ast::FacetDocument {
        blocks: vec![
            weather_interface_read(),
            FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "allow".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::List(vec![rule]),
                    span: span(),
                })],
                span: span(),
            }),
        ],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(!result.passed);
    assert!(result.error.as_deref().unwrap_or_default().contains("F455"));
    let execution_raw = result
        .execution_output
        .as_ref()
        .expect("execution output must be present on failure");
    let execution: serde_json::Value =
        serde_json::from_str(execution_raw).expect("execution output must be valid JSON");
    assert_eq!(
        execution
            .get("provenance")
            .and_then(|p| p.get("events"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("op"))
            .and_then(|v| v.as_str()),
        Some("tool_call")
    );
    assert_eq!(
        execution
            .get("provenance")
            .and_then(|p| p.get("events"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|e| e.get("error_code"))
            .and_then(|v| v.as_str()),
        Some("F455")
    );
}

#[test]
fn test_runner_mock_tool_call_in_pure_mode_is_f801() {
    let runner = TestRunner::new_with_mode(1000, 4096, fct_engine::ExecutionMode::Pure);
    let mocks = vec![MockDefinition {
        target: "WeatherAPI.get_current".to_string(),
        return_value: ValueNode::String("ok".to_string()),
        span: span(),
    }];
    let test_block = TestBlock {
        name: "pure-mode-tool-call".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks,
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };
    let doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read(), policy_allow_weather_tool_call()],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(!result.passed);
    assert!(result.error.as_deref().unwrap_or_default().contains("F801"));
    let execution_raw = result
        .execution_output
        .as_ref()
        .expect("execution output must be present on failure");
    let execution: serde_json::Value =
        serde_json::from_str(execution_raw).expect("execution output must be valid JSON");
    let first_event = execution
        .get("provenance")
        .and_then(|p| p.get("events"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .cloned()
        .expect("expected first guard decision event");
    assert_eq!(
        first_event.get("op").and_then(|v| v.as_str()),
        Some("tool_call")
    );
    assert_eq!(
        first_event.get("decision").and_then(|v| v.as_str()),
        Some("denied")
    );
    assert_eq!(
        first_event.get("error_code").and_then(|v| v.as_str()),
        Some("F801")
    );
    assert_eq!(
        first_event.get("mode").and_then(|v| v.as_str()),
        Some("pure")
    );
}

#[test]
fn test_runner_bounded_lens_in_pure_mode_is_f803() {
    let runner = TestRunner::new_with_mode(1000, 4096, fct_engine::ExecutionMode::Pure);

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "answer".to_string(),
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
    });

    let test_block = TestBlock {
        name: "pure-mode-bounded-lens".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![vars_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(!result.passed);
    assert!(result.error.as_deref().unwrap_or_default().contains("F803"));
}

#[test]
fn test_runner_run_all_isolates_test_contexts() {
    let runner = TestRunner::new(1000, 4096);

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "q".to_string(),
            key_kind: Default::default(),
            value: ValueNode::Directive(fct_ast::DirectiveNode {
                name: "input".to_string(),
                args: OrderedMap::from([(
                    "type".to_string(),
                    ValueNode::String("string".to_string()),
                )]),
                span: span(),
            }),
            span: span(),
        })],
        span: span(),
    });

    let first = FacetNode::Test(TestBlock {
        name: "first-with-input".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::from([("q".to_string(), ValueNode::String("hello".to_string()))]),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    });
    let second = FacetNode::Test(TestBlock {
        name: "second-no-input".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    });

    let doc = fct_ast::FacetDocument {
        blocks: vec![vars_block, first, second],
        span: span(),
    };

    let results = runner.run_all(&doc);
    assert_eq!(results.len(), 2);
    assert!(results[0].passed);
    assert!(!results[1].passed);
    assert!(results[1]
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("F453"));
}

#[test]
fn test_runner_vars_override_fails_type_check_against_var_types() {
    let runner = TestRunner::new(1000, 4096);

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "age".to_string(),
            key_kind: Default::default(),
            value: ValueNode::Scalar(ScalarValue::Int(21)),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "vars-override-type-mismatch".to_string(),
        vars: OrderedMap::from([("age".to_string(), ValueNode::String("oops".to_string()))]),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![primitive_var_types_block("age", "int"), vars_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(!result.passed);
    assert!(result.error.as_deref().unwrap_or_default().contains("F451"));
}

#[test]
fn test_runner_vars_override_recomputes_dependent_vars() {
    let runner = TestRunner::new(1000, 4096);

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![
            BodyNode::KeyValue(KeyValueNode {
                key: "base".to_string(),
                key_kind: Default::default(),
                value: ValueNode::Scalar(ScalarValue::Int(1)),
                span: span(),
            }),
            BodyNode::KeyValue(KeyValueNode {
                key: "total".to_string(),
                key_kind: Default::default(),
                value: ValueNode::Variable("base".to_string()),
                span: span(),
            }),
        ],
        span: span(),
    });
    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::Variable("total".to_string()),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "vars-override-recompute".to_string(),
        vars: OrderedMap::from([("base".to_string(), ValueNode::Scalar(ScalarValue::Int(7)))]),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![Assertion {
            kind: AssertionKind::Equals {
                target: "canonical.messages[0].content".to_string(),
                expected: ValueNode::String("7".to_string()),
            },
            span: span(),
        }],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![
            primitive_var_types_block("base", "int"),
            vars_block,
            user_block,
        ],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(
        result.passed,
        "expected override to recompute dependent vars"
    );
}

#[test]
fn test_runner_resolves_pipeline_message_content_in_canonical_view() {
    let runner = TestRunner::new(1000, 4096);

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "name".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("world".to_string()),
            span: span(),
        })],
        span: span(),
    });
    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::Pipeline(PipelineNode {
                initial: Box::new(ValueNode::Variable("name".to_string())),
                lenses: vec![LensCallNode {
                    name: "uppercase".to_string(),
                    args: vec![],
                    kwargs: OrderedMap::new(),
                    span: span(),
                }],
                span: span(),
            }),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "pipeline-message-content".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![vars_block, user_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(result.passed);

    let canonical_json = result
        .canonical_output
        .as_ref()
        .expect("canonical output should be present");
    let canonical: serde_json::Value =
        serde_json::from_str(canonical_json).expect("canonical output must be valid json");
    assert_eq!(canonical["messages"][0]["content"], "WORLD");
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
            span: Span {
                start: 0,
                end: 10,
                line: 1,
                column: 1,
            },
        },
        Assertion {
            kind: AssertionKind::NotContains {
                target: "output".to_string(),
                text: "world".to_string(),
            },
            span: Span {
                start: 20,
                end: 30,
                line: 2,
                column: 1,
            },
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
        canonical: None,
        execution: None,
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
            span: Span {
                start: 0,
                end: 10,
                line: 1,
                column: 1,
            },
        },
        Assertion {
            kind: AssertionKind::GreaterThan {
                field: "tokens".to_string(),
                value: 10.0,
            },
            span: Span {
                start: 20,
                end: 30,
                line: 2,
                column: 1,
            },
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
        canonical: None,
        execution: None,
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

    let assertions = vec![Assertion {
        kind: AssertionKind::Sentiment {
            target: "output".to_string(),
            expected: "positive".to_string(),
        },
        span: Span {
            start: 0,
            end: 10,
            line: 1,
            column: 1,
        },
    }];

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
        canonical: None,
        execution: None,
    };

    let positive_output = "This is great and helpful!";
    let results = runner.evaluate_assertions(positive_output, &ctx, &assertions);
    assert!(results[0].passed);

    let negative_output = "This is bad and terrible";
    let results = runner.evaluate_assertions(negative_output, &ctx, &assertions);
    assert!(!results[0].passed);
}

#[test]
fn test_runner_mock_tool_call_uses_test_input_for_policy_cond() {
    let runner = TestRunner::new(1000, 4096);

    let mocks = vec![MockDefinition {
        target: "WeatherAPI.get_current".to_string(),
        return_value: ValueNode::String("ok".to_string()),
        span: span(),
    }];

    let mut input = OrderedMap::new();
    input.insert(
        "allow_tool".to_string(),
        ValueNode::Scalar(ScalarValue::Bool(true)),
    );

    let test_block = TestBlock {
        name: "input-driven-policy".to_string(),
        vars: OrderedMap::new(),
        input,
        mocks,
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "allow_tool".to_string(),
            key_kind: Default::default(),
            value: ValueNode::Directive(fct_ast::DirectiveNode {
                name: "input".to_string(),
                args: OrderedMap::from([(
                    "type".to_string(),
                    ValueNode::String("bool".to_string()),
                )]),
                span: span(),
            }),
            span: span(),
        })],
        span: span(),
    });

    let rule = ValueNode::Map(OrderedMap::from([
        ("op".to_string(), ValueNode::String("tool_call".to_string())),
        (
            "name".to_string(),
            ValueNode::String("WeatherAPI.get_current".to_string()),
        ),
        ("effect".to_string(), ValueNode::String("read".to_string())),
        (
            "when".to_string(),
            ValueNode::Variable("allow_tool".to_string()),
        ),
    ]));
    let policy_block = FacetNode::Policy(FacetBlock {
        name: "policy".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "allow".to_string(),
            key_kind: Default::default(),
            value: ValueNode::List(vec![rule]),
            span: span(),
        })],
        span: span(),
    });

    let doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read(), vars_block, policy_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(
        result.passed,
        "expected tool_call to be allowed by input-driven policy"
    );
    assert!(result.error.is_none());
}

#[test]
fn test_runner_mode_policy_interaction_matrix() {
    let test_block = TestBlock {
        name: "mode-policy-matrix".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![MockDefinition {
            target: "WeatherAPI.get_current".to_string(),
            return_value: ValueNode::String("ok".to_string()),
            span: span(),
        }],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    // exec + no policy => deterministic deny (F454)
    let exec_default_deny_doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read()],
        span: span(),
    };
    let exec_runner = TestRunner::new_with_mode(1000, 4096, fct_engine::ExecutionMode::Exec);
    let exec_default_deny = exec_runner
        .run_test(&exec_default_deny_doc, &test_block)
        .unwrap();
    assert!(!exec_default_deny.passed);
    assert!(exec_default_deny
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("F454"));

    // exec + undecidable policy condition => fail-closed undecidable (F455)
    let undecidable_rule = ValueNode::Map(OrderedMap::from([
        ("op".to_string(), ValueNode::String("tool_call".to_string())),
        (
            "name".to_string(),
            ValueNode::String("WeatherAPI.get_current".to_string()),
        ),
        (
            "when".to_string(),
            ValueNode::Variable("missing.flag".to_string()),
        ),
    ]));
    let exec_undecidable_doc = fct_ast::FacetDocument {
        blocks: vec![
            weather_interface_read(),
            FacetNode::Policy(FacetBlock {
                name: "policy".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "allow".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::List(vec![undecidable_rule]),
                    span: span(),
                })],
                span: span(),
            }),
        ],
        span: span(),
    };
    let exec_undecidable = exec_runner
        .run_test(&exec_undecidable_doc, &test_block)
        .unwrap();
    assert!(!exec_undecidable.passed);
    assert!(exec_undecidable
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("F455"));

    // pure + otherwise allowed policy => runtime I/O prohibited first (F801)
    let pure_allow_doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read(), policy_allow_weather_tool_call()],
        span: span(),
    };
    let pure_runner = TestRunner::new_with_mode(1000, 4096, fct_engine::ExecutionMode::Pure);
    let pure_disallow = pure_runner.run_test(&pure_allow_doc, &test_block).unwrap();
    assert!(!pure_disallow.passed);
    assert!(pure_disallow
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("F801"));
}

#[test]
fn test_runner_assertions_can_read_canonical_paths() {
    let runner = TestRunner::new(1000, 4096);

    let system_block = FacetNode::System(FacetBlock {
        name: "system".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("You are helpful.".to_string()),
            span: span(),
        })],
        span: span(),
    });
    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("Hi".to_string()),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "canonical-path-assertions".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![
            Assertion {
                kind: AssertionKind::Equals {
                    target: "canonical.messages[0].role".to_string(),
                    expected: ValueNode::String("system".to_string()),
                },
                span: span(),
            },
            Assertion {
                kind: AssertionKind::Equals {
                    target: "canonical.messages[1].role".to_string(),
                    expected: ValueNode::String("user".to_string()),
                },
                span: span(),
            },
        ],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![system_block, user_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(result.passed, "expected canonical path assertions to pass");
}

#[test]
fn test_runner_canonical_omits_when_false_message_blocks() {
    let runner = TestRunner::new(1000, 4096);

    let mut system_attrs = OrderedMap::new();
    system_attrs.insert(
        "when".to_string(),
        ValueNode::Scalar(ScalarValue::Bool(false)),
    );
    let system_block = FacetNode::System(FacetBlock {
        name: "system".to_string(),
        attributes: system_attrs,
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("hidden system".to_string()),
            span: span(),
        })],
        span: span(),
    });
    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("visible user".to_string()),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "when-false-omits-message".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![system_block, user_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(result.passed);
    let canonical_raw = result.canonical_output.expect("expected canonical output");
    let canonical: serde_json::Value = serde_json::from_str(&canonical_raw).unwrap();
    let messages = canonical
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array");
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].get("role").and_then(|v| v.as_str()),
        Some("user")
    );
}

#[test]
fn test_runner_canonical_resolves_when_variable() {
    let runner = TestRunner::new(1000, 4096);

    let vars_block = FacetNode::Vars(FacetBlock {
        name: "vars".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "emit_system".to_string(),
            key_kind: Default::default(),
            value: ValueNode::Scalar(ScalarValue::Bool(false)),
            span: span(),
        })],
        span: span(),
    });
    let mut system_attrs = OrderedMap::new();
    system_attrs.insert(
        "when".to_string(),
        ValueNode::Variable("emit_system".to_string()),
    );
    let system_block = FacetNode::System(FacetBlock {
        name: "system".to_string(),
        attributes: system_attrs,
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("hidden system".to_string()),
            span: span(),
        })],
        span: span(),
    });
    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("visible user".to_string()),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "when-var-omits-message".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![vars_block, system_block, user_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(result.passed);
    let canonical_raw = result.canonical_output.expect("expected canonical output");
    let canonical: serde_json::Value = serde_json::from_str(&canonical_raw).unwrap();
    let messages = canonical
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array");
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].get("role").and_then(|v| v.as_str()),
        Some("user")
    );
}

#[test]
fn test_runner_canonical_omits_body_when_false_message_blocks() {
    let runner = TestRunner::new(1000, 4096);

    let system_block = FacetNode::System(FacetBlock {
        name: "system".to_string(),
        attributes: OrderedMap::new(),
        body: vec![
            BodyNode::KeyValue(KeyValueNode {
                key: "when".to_string(),
                key_kind: Default::default(),
                value: ValueNode::Scalar(ScalarValue::Bool(false)),
                span: span(),
            }),
            BodyNode::KeyValue(KeyValueNode {
                key: "content".to_string(),
                key_kind: Default::default(),
                value: ValueNode::String("hidden system".to_string()),
                span: span(),
            }),
        ],
        span: span(),
    });
    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("visible user".to_string()),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "body-when-false-omits-message".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![system_block, user_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(result.passed);
    let canonical_raw = result.canonical_output.expect("expected canonical output");
    let canonical: serde_json::Value = serde_json::from_str(&canonical_raw).unwrap();
    let messages = canonical
        .get("messages")
        .and_then(|v| v.as_array())
        .expect("messages array");
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].get("role").and_then(|v| v.as_str()),
        Some("user")
    );
}

#[test]
fn test_runner_applies_context_budget_and_defaults_to_layout() {
    let runner = TestRunner::new(1000, 4096);

    let context_block = FacetNode::Context(FacetBlock {
        name: "context".to_string(),
        attributes: OrderedMap::new(),
        body: vec![
            BodyNode::KeyValue(KeyValueNode {
                key: "budget".to_string(),
                key_kind: Default::default(),
                value: ValueNode::Scalar(ScalarValue::Int(5)),
                span: span(),
            }),
            BodyNode::KeyValue(KeyValueNode {
                key: "defaults".to_string(),
                key_kind: Default::default(),
                value: ValueNode::Map(OrderedMap::from([
                    (
                        "priority".to_string(),
                        ValueNode::Scalar(ScalarValue::Int(123)),
                    ),
                    ("min".to_string(), ValueNode::Scalar(ScalarValue::Int(0))),
                    ("grow".to_string(), ValueNode::Scalar(ScalarValue::Int(0))),
                    ("shrink".to_string(), ValueNode::Scalar(ScalarValue::Int(1))),
                ])),
                span: span(),
            }),
        ],
        span: span(),
    });

    let user_block = FacetNode::User(FacetBlock {
        name: "user".to_string(),
        attributes: OrderedMap::new(),
        body: vec![BodyNode::KeyValue(KeyValueNode {
            key: "content".to_string(),
            key_kind: Default::default(),
            value: ValueNode::String("abcdefghij".to_string()),
            span: span(),
        })],
        span: span(),
    });

    let test_block = TestBlock {
        name: "context-budget-layout".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![],
        assertions: vec![],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![context_block, user_block],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(
        result.passed,
        "expected layout to honor context defaults/budget"
    );
    assert!(
        result.telemetry.tokens_used <= 5,
        "expected context budget to cap token usage, got {}",
        result.telemetry.tokens_used
    );
}

#[test]
fn test_runner_assertions_can_read_execution_paths() {
    let runner = TestRunner::new(1000, 4096);

    let test_block = TestBlock {
        name: "execution-path-assertions".to_string(),
        vars: OrderedMap::new(),
        input: OrderedMap::new(),
        mocks: vec![MockDefinition {
            target: "WeatherAPI.get_current".to_string(),
            return_value: ValueNode::String("ok".to_string()),
            span: span(),
        }],
        assertions: vec![Assertion {
            kind: AssertionKind::Equals {
                target: "execution.provenance.events[0].op".to_string(),
                expected: ValueNode::String("tool_call".to_string()),
            },
            span: span(),
        }],
        body: Vec::new(),
        span: span(),
    };

    let doc = fct_ast::FacetDocument {
        blocks: vec![weather_interface_read(), policy_allow_weather_tool_call()],
        span: span(),
    };

    let result = runner.run_test(&doc, &test_block).unwrap();
    assert!(result.passed, "expected execution path assertions to pass");
}
