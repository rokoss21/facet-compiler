//! # FACET Engine Module
//!
//! This module provides the core execution engine for the FACET language compiler.
//! It combines reactive DAG execution with the Token Box Model for efficient
//! resource allocation and execution of FACET components and pipelines.
//!
//! ## Architecture
//!
//! The engine consists of several key components:
//!
//! - **R-DAG Engine**: Reactive Directed Acyclic Graph execution engine that manages
//!   variable evaluation, dependency tracking, and reactive updates
//! - **Token Box Model**: CSS Box Model-inspired token allocation system for managing
//!   memory and computational resources
//! - **Tokenizer**: Thread-safe token counting and text processing utilities
//! - **Test Runner**: Comprehensive testing framework with performance telemetry
//! - **Error Handling**: Robust error management with detailed error codes and recovery
//!
//! ## Features
//!
//! - **Reactive Execution**: Automatic invalidation and re-evaluation when dependencies change
//! - **Resource Management**: Token-based allocation with memory limits and performance optimization
//! - **Thread Safety**: All components are thread-safe and support concurrent execution
//! - **Performance Monitoring**: Built-in telemetry and performance metrics
//! - **Testing Framework**: Comprehensive testing with mocking and assertion capabilities
//!
//! ## Basic Usage
//!
//! ```ignore
//! use fct_engine::{RDagEngine, ExecutionContext};
//! use fct_ast::FacetDocument;
//!
//! // Parse and validate a FACET document first
//! let document: FacetDocument = todo!("Parse from source");
//!
//! // Create execution context
//! let mut engine = RDagEngine::new();
//! let context = ExecutionContext::new(1000000);
//!
//! // Execute the document
//! match engine.execute(&mut context) {
//!     Ok(results) => println!("Execution successful"),
//!     Err(e) => println!("Execution failed: {}", e),
//! }
//! ```
//!
//! ## Advanced Usage with Token Allocation
//!
//! ```ignore
//! use fct_engine::{RDagEngine, TokenBoxModel, Section};
//! use fct_std::LensRegistry;
//! use fct_ast::ValueNode;
//!
//! // Create token allocation model
//! let mut box_model = TokenBoxModel::new(1000); // 1000 tokens budget
//!
//! // Define sections for different components
//! let ui_content = ValueNode::String("UI prompt content here".to_string());
//! let ui_section = Section::new("ui".to_string(), ui_content, 300);
//!
//! let data_content = ValueNode::String("Data processing content here".to_string());
//! let data_section = Section::new("data".to_string(), data_content, 200);
//!
//! // Allocate tokens
//! let lens_registry = LensRegistry::new();
//! let allocation = box_model.allocate(vec![ui_section, data_section], &lens_registry)?;
//!
//! // Execute with token limits
//! let mut engine = RDagEngine::new();
//! ```
//!
//! ## Testing Framework
//!
//! ```ignore
//! use fct_engine::{TestRunner, TestContext};
//! use fct_ast::{ValueNode, TestBlock};
//!
//! let mut runner = TestRunner::new(1000000, 100000);
//! let context = TestContext::new();
//!
//! // Run a single test
//! let test_block: TestBlock = todo!("Create test from parsed document");
//! let test_result = runner.run_test(&test_block, context)?;
//! ```

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

mod box_model;
mod errors;
mod mock_system;
mod r_dag;
mod test_reporter;
mod test_runner;
mod tokenizer;
mod tool_executor;
mod tool_schemas;

// ============================================================================
// PUBLIC API EXPORTS
// ============================================================================

// Re-export public API for convenient use
pub use box_model::{AllocatedSection, AllocationResult, Section, TokenBoxModel};
pub use errors::{EngineError, EngineResult};
pub use mock_system::{EnhancedMockRegistry, MockBehavior, MockBuilder, MockDefinition};
pub use r_dag::{ExecutionContext, ExecutionGuardDecision, ExecutionMode, GasContext, RDagEngine};
pub use test_reporter::{
    AssertionReport, ReportFormat, ReportMetadata, TelemetryReport, TestReportEntry, TestReporter,
    TestStatus, TestSuiteReport, TestSummary,
};
pub use test_runner::{
    AssertionResult, MockRegistry, TestContext, TestResult, TestRunner, TestTelemetry,
};
pub use tokenizer::{
    count_facet_units, count_facet_units_in_value, count_tokens, count_tokens_in_value, Tokenizer,
};
pub use tool_executor::{
    value_node_map_to_json, value_node_to_json, ToolDefinition, ToolExecutor, ToolHandler,
    ToolInvocation, ToolResult,
};
pub use tool_schemas::{
    create_number_param, create_object_param, create_string_param, AnthropicTool, LlamaFunction,
    LlamaTool, OpenAIFunction, OpenAITool, Provider, SchemaConverter,
};

/// Derive deterministic canonical section id for a message role and 1-based occurrence index.
pub fn derive_message_section_id(role: &str, ordinal: usize) -> String {
    format!("{}#{}", role, ordinal)
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod token_box_tests {
    use super::*;
    use fct_ast::{LensCallNode, OrderedMap, PipelineNode, Span, ValueNode};
    use fct_std::LensRegistry;

    #[test]
    fn test_section_creation() {
        let content = ValueNode::String("test content".to_string());
        let section = Section::new("test".to_string(), content.clone(), 50)
            .with_priority(100)
            .with_limits(10, 0.5, 0.3);

        assert_eq!(section.id, "test");
        assert_eq!(section.priority, 100);
        assert_eq!(section.base_size, 50);
        assert_eq!(section.min, 10);
        assert_eq!(section.grow, 0.5);
        assert_eq!(section.shrink, 0.3);
        assert_eq!(section.current_size, 50);
        assert!(!section.is_critical);
    }

    #[test]
    fn test_critical_section() {
        let content = ValueNode::String("critical".to_string());
        let section = Section::new("critical".to_string(), content, 100).with_limits(50, 0.0, 0.0); // shrink = 0

        assert!(section.is_critical);
    }

    #[test]
    fn test_basic_allocation_no_compression() {
        let model = TokenBoxModel::new(1000);
        let lens_registry = LensRegistry::new();

        let sections = vec![
            Section::new(
                "system".to_string(),
                ValueNode::String("System prompt".to_string()),
                100,
            )
            .with_priority(100)
            .with_limits(50, 0.0, 0.0), // critical
            Section::new(
                "user".to_string(),
                ValueNode::String("User query".to_string()),
                50,
            )
            .with_priority(200)
            .with_limits(0, 0.1, 0.2), // flexible
        ];

        let result = model.allocate(sections, &lens_registry).unwrap();

        // When total fits budget, sections are preserved as-is.
        assert_eq!(result.total_size, 150);
        assert_eq!(result.overflow, 0);
        assert_eq!(result.sections.len(), 2);

        // Find sections by ID for reliable checking
        let system_section = result
            .sections
            .iter()
            .find(|a| a.section.id == "system")
            .unwrap();
        let user_section = result
            .sections
            .iter()
            .find(|a| a.section.id == "user")
            .unwrap();

        assert!(!system_section.was_compressed);
        assert!(!system_section.was_dropped);
        assert_eq!(system_section.final_size, 100); // critical doesn't change

        assert!(!user_section.was_compressed);
        assert!(!user_section.was_dropped);
        assert_eq!(user_section.final_size, 50);
    }

    #[test]
    fn test_critical_overflow() {
        let model = TokenBoxModel::new(100);
        let lens_registry = LensRegistry::new();

        let sections = vec![
            Section::new(
                "critical1".to_string(),
                ValueNode::String("Critical section 1".to_string()),
                80,
            )
            .with_priority(100)
            .with_limits(40, 0.0, 0.0), // critical
            Section::new(
                "critical2".to_string(),
                ValueNode::String("Critical section 2".to_string()),
                50,
            )
            .with_priority(200)
            .with_limits(20, 0.0, 0.0), // critical
        ];

        let result = model.allocate(sections, &lens_registry);
        assert!(result.is_err());

        match result.unwrap_err() {
            EngineError::BudgetExceeded { budget, required } => {
                assert_eq!(budget, 100);
                assert_eq!(required, 130); // 80 + 50
            }
            _ => panic!("Expected BudgetExceeded error"),
        }
    }

    #[test]
    fn test_compression_needed() {
        let model = TokenBoxModel::new(120); // Small budget to force compression
        let lens_registry = LensRegistry::new();

        // Create a simple compression strategy using trim lens
        let trim_lens = LensCallNode {
            name: "trim".to_string(),
            args: vec![],
            kwargs: OrderedMap::new(),
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let compression_strategy = PipelineNode {
            initial: Box::new(ValueNode::String("".to_string())), // unused
            lenses: vec![trim_lens],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let sections = vec![
            Section::new(
                "critical".to_string(),
                ValueNode::String("Critical content".to_string()),
                50,
            )
            .with_priority(100)
            .with_limits(50, 0.0, 0.0), // critical, cannot compress
            Section::new(
                "flexible".to_string(),
                ValueNode::String("  Flexible content with padding  ".to_string()),
                100,
            )
            .with_priority(200)
            .with_limits(20, 0.1, 0.5)
            .with_strategy(compression_strategy), // Can compress
        ];

        let result = model.allocate(sections, &lens_registry).unwrap();

        // Critical section takes 50
        // Flexible section has 100 but budget is 120 total
        // So flexible needs to be reduced or compressed
        assert_eq!(result.overflow, 0);
        assert!(result.total_size <= 120);

        let flexible_section = result
            .sections
            .iter()
            .find(|a| a.section.id == "flexible")
            .unwrap();

        // Should be compressed or truncated
        assert!(
            flexible_section.was_compressed || flexible_section.final_size < 100,
            "Expected compression or truncation"
        );
    }

    #[test]
    fn test_unknown_compression_lens_returns_f802() {
        let model = TokenBoxModel::new(50);
        let lens_registry = LensRegistry::new();

        let unknown_lens = LensCallNode {
            name: "unknown_compress".to_string(),
            args: vec![],
            kwargs: OrderedMap::new(),
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let compression_strategy = PipelineNode {
            initial: Box::new(ValueNode::String("".to_string())),
            lenses: vec![unknown_lens],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let sections = vec![Section::new(
            "flexible".to_string(),
            ValueNode::String("content that must be compressed".to_string()),
            100,
        )
        .with_priority(100)
        .with_limits(0, 0.0, 0.5)
        .with_strategy(compression_strategy)];

        let err = model.allocate(sections, &lens_registry).unwrap_err();
        assert!(matches!(
            err,
            EngineError::UnknownLens { ref name } if name == "unknown_compress"
        ));
        assert!(err.to_string().contains("F802"));
    }

    #[test]
    fn test_pure_mode_rejects_non_level0_strategy_lens_with_f801() {
        let model = TokenBoxModel::new(10);
        let lens_registry = LensRegistry::new();

        let bounded_lens = LensCallNode {
            name: "llm_call".to_string(),
            args: vec![],
            kwargs: OrderedMap::new(),
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let strategy = PipelineNode {
            initial: Box::new(ValueNode::String("".to_string())),
            lenses: vec![bounded_lens],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let sections = vec![Section::new(
            "flexible".to_string(),
            ValueNode::String("content that requires compression".to_string()),
            80,
        )
        .with_priority(100)
        .with_limits(0, 0.0, 1.0)
        .with_strategy(strategy)];

        let err = model
            .allocate_with_mode(sections, &lens_registry, ExecutionMode::Pure)
            .unwrap_err();
        assert!(matches!(err, EngineError::LensExecutionFailed { .. }));
        assert!(err.to_string().contains("F801"));
    }

    #[test]
    fn test_exec_mode_rejects_nondeterministic_strategy_lens_with_f801() {
        let model = TokenBoxModel::new(10);
        let lens_registry = LensRegistry::new();

        let nondeterministic_lens = LensCallNode {
            name: "llm_call".to_string(),
            args: vec![],
            kwargs: OrderedMap::new(),
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let strategy = PipelineNode {
            initial: Box::new(ValueNode::String("".to_string())),
            lenses: vec![nondeterministic_lens],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let sections = vec![Section::new(
            "flexible".to_string(),
            ValueNode::String("content that requires compression".to_string()),
            80,
        )
        .with_priority(100)
        .with_limits(0, 0.0, 1.0)
        .with_strategy(strategy)];

        let err = model
            .allocate_with_mode(sections, &lens_registry, ExecutionMode::Exec)
            .unwrap_err();
        assert!(matches!(err, EngineError::LensExecutionFailed { .. }));
        assert!(err.to_string().contains("F801"));
        assert!(err.to_string().contains("must be deterministic"));
    }

    #[test]
    fn test_preserves_input_order_when_no_compression() {
        let model = TokenBoxModel::new(10_000);
        let lens_registry = LensRegistry::new();

        let sections = vec![
            Section::new(
                "user".to_string(),
                ValueNode::String("user".to_string()),
                10,
            )
            .with_priority(200)
            .with_limits(0, 0.1, 0.1),
            Section::new(
                "system".to_string(),
                ValueNode::String("system".to_string()),
                10,
            )
            .with_priority(100)
            .with_limits(0, 0.1, 0.1),
            Section::new(
                "assistant".to_string(),
                ValueNode::String("assistant".to_string()),
                10,
            )
            .with_priority(300)
            .with_limits(0, 0.1, 0.1),
        ];

        let result = model.allocate(sections, &lens_registry).unwrap();
        let ids: Vec<String> = result
            .sections
            .iter()
            .map(|s| s.section.id.clone())
            .collect();
        assert_eq!(ids, vec!["user", "system", "assistant"]);
    }

    #[test]
    fn test_preserves_input_order_after_compression() {
        let model = TokenBoxModel::new(120);
        let lens_registry = LensRegistry::new();

        let sections = vec![
            Section::new(
                "user".to_string(),
                ValueNode::String("user".to_string()),
                80,
            )
            .with_priority(300)
            .with_limits(20, 0.0, 0.5),
            Section::new(
                "system".to_string(),
                ValueNode::String("system".to_string()),
                50,
            )
            .with_priority(100)
            .with_limits(50, 0.0, 0.0),
            Section::new(
                "assistant".to_string(),
                ValueNode::String("assistant".to_string()),
                80,
            )
            .with_priority(200)
            .with_limits(20, 0.0, 0.5),
        ];

        let result = model.allocate(sections, &lens_registry).unwrap();
        let ids: Vec<String> = result
            .sections
            .iter()
            .map(|s| s.section.id.clone())
            .collect();
        assert_eq!(ids, vec!["user", "system", "assistant"]);
    }

    #[test]
    fn test_utf8_truncation_preserves_char_boundaries() {
        let model = TokenBoxModel::new(9);
        let lens_registry = LensRegistry::new();

        let text = "🙂🙂🙂".to_string(); // 12 UTF-8 bytes
        let section = Section::new("user".to_string(), ValueNode::String(text), 12)
            .with_priority(100)
            .with_limits(0, 0.0, 0.5);

        let result = model.allocate(vec![section], &lens_registry).unwrap();
        let allocated = result
            .sections
            .iter()
            .find(|s| s.section.id == "user")
            .expect("user section should exist");

        assert!(!allocated.was_dropped);
        assert!(allocated.was_truncated);
        assert_eq!(allocated.final_size, 8);
        assert_eq!(
            allocated.section.content,
            ValueNode::String("🙂🙂".to_string())
        );
    }

    #[test]
    fn test_section_drops_when_over_budget_and_at_min() {
        let model = TokenBoxModel::new(5);
        let lens_registry = LensRegistry::new();

        let section = Section::new(
            "flex".to_string(),
            ValueNode::String("abcdef".to_string()),
            6,
        )
        .with_priority(100)
        .with_limits(6, 0.0, 0.5);

        let result = model.allocate(vec![section], &lens_registry).unwrap();
        let allocated = result
            .sections
            .iter()
            .find(|s| s.section.id == "flex")
            .expect("section should exist");

        assert!(allocated.was_dropped);
        assert_eq!(allocated.final_size, 0);
        assert_eq!(result.total_size, 0);
        assert_eq!(result.overflow, 0);
    }

    #[test]
    fn test_non_string_list_truncation_updates_content() {
        let lens_registry = LensRegistry::new();
        let original_content = ValueNode::List(vec![
            ValueNode::String("abcd".to_string()),
            ValueNode::String("efgh".to_string()),
        ]);
        let tokenizer = Tokenizer::new().unwrap();
        let base_size = tokenizer.count_facet_units_in_value(&original_content);
        assert_eq!(base_size, 8);

        let model = TokenBoxModel::new(6);
        let section = Section::new("user".to_string(), original_content, base_size)
            .with_priority(100)
            .with_limits(0, 0.0, 0.5);

        let result = model.allocate(vec![section], &lens_registry).unwrap();
        let allocated = result
            .sections
            .iter()
            .find(|s| s.section.id == "user")
            .expect("user section should exist");

        assert!(allocated.was_truncated);
        assert_eq!(allocated.final_size, 6);
        assert_eq!(
            allocated.section.content,
            ValueNode::List(vec![
                ValueNode::String("abcd".to_string()),
                ValueNode::String("ef".to_string())
            ])
        );
    }

    #[test]
    fn test_non_string_multimodal_list_truncation_is_materialized() {
        let lens_registry = LensRegistry::new();

        let mut item1 = OrderedMap::new();
        item1.insert("type".to_string(), ValueNode::String("text".to_string()));
        item1.insert("text".to_string(), ValueNode::String("alpha".to_string()));

        let mut item2 = OrderedMap::new();
        item2.insert("type".to_string(), ValueNode::String("text".to_string()));
        item2.insert("text".to_string(), ValueNode::String("beta".to_string()));

        let content = ValueNode::List(vec![ValueNode::Map(item1), ValueNode::Map(item2)]);
        let tokenizer = Tokenizer::new().unwrap();
        let base_size = tokenizer.count_facet_units_in_value(&content);
        let model = TokenBoxModel::new(base_size - 2);
        let section = Section::new("user".to_string(), content, base_size)
            .with_priority(100)
            .with_limits(0, 0.0, 0.5);

        let result = model.allocate(vec![section], &lens_registry).unwrap();
        let allocated = result
            .sections
            .iter()
            .find(|s| s.section.id == "user")
            .expect("user section should exist");

        assert!(allocated.was_truncated);
        assert_eq!(allocated.final_size, base_size - 2);
        assert_eq!(
            tokenizer.count_facet_units_in_value(&allocated.section.content),
            allocated.final_size
        );

        match &allocated.section.content {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 2);
                match &items[1] {
                    ValueNode::Map(map) => {
                        assert_eq!(
                            map.get("type"),
                            Some(&ValueNode::String("text".to_string()))
                        );
                        assert_eq!(map.get("text"), Some(&ValueNode::String("be".to_string())));
                    }
                    _ => panic!("expected map multimodal item"),
                }
            }
            _ => panic!("expected list content"),
        }
    }
}
