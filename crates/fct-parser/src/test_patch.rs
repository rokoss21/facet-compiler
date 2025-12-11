// This file contains the patch to apply to parser.rs for @test block handling

use fct_ast::{
    Assertion, AssertionKind, BodyNode, FacetBlock, FacetNode, 
    MockDefinition, TestBlock, ValueNode
};
use std::collections::HashMap;

// Replace the test block parsing in facet_block function
pub fn parse_test_block(
    name: String,
    attributes: HashMap<String, ValueNode>,
    body: Vec<BodyNode>,
    span: fct_ast::Span,
) -> FacetNode {
    // Parse test sections (vars, mock, assert)
    let (mut vars, mut mocks, mut assertions) = (
        HashMap::new(),
        Vec::new(),
        Vec::new(),
    );
    
    for body_node in &body {
        if let BodyNode::KeyValue(kv) = body_node {
            match kv.key.as_str() {
                "vars" => {
                    if let ValueNode::Map(map) = &kv.value {
                        vars = map.clone();
                    }
                }
                "mock" => {
                    if let ValueNode::Map(map) = &kv.value {
                        for (target, value) in map {
                            mocks.push(MockDefinition {
                                target: target.clone(),
                                return_value: value.clone(),
                                span: kv.span,
                            });
                        }
                    }
                }
                "assert" => {
                    if let ValueNode::List(items) = &kv.value {
                        for item in items {
                            if let ValueNode::Map(assert_map) = item {
                                for (assert_type, assert_value) in assert_map {
                                    let assertion = crate::test_parser::parse_assertion(
                                        assert_type, 
                                        assert_value, 
                                        &kv.span
                                    );
                                    assertions.push(assertion);
                                }
                            }
                        }
                    }
                }
                _ => {} // Ignore other keys in test block
            }
        }
    }
    
    FacetNode::Test(TestBlock {
        name,
        vars,
        mocks,
        assertions,
        body,
        span,
    })
}