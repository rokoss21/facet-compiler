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

mod errors;
mod r_dag;
mod box_model;
mod tokenizer;
mod test_runner;
mod test_reporter;
mod tool_executor;
mod tool_schemas;
mod mock_system;

// ============================================================================
// PUBLIC API EXPORTS
// ============================================================================

// Re-export public API for convenient use
pub use errors::{EngineError, EngineResult};
pub use r_dag::{ExecutionContext, GasContext, RDagEngine};
pub use box_model::{AllocationResult, AllocatedSection, Section, TokenBoxModel};
pub use tokenizer::{Tokenizer, count_tokens, count_tokens_in_value};
pub use test_runner::{
    TestRunner, TestResult, TestTelemetry, AssertionResult, TestContext, MockRegistry
};
pub use test_reporter::{
    TestReporter, ReportFormat, TestSuiteReport, TestReportEntry, TestStatus,
    AssertionReport, TelemetryReport, TestSummary, ReportMetadata
};
pub use tool_executor::{
    ToolDefinition, ToolExecutor, ToolHandler, ToolInvocation, ToolResult,
    value_node_to_json, value_node_map_to_json
};
pub use tool_schemas::{
    Provider, SchemaConverter, OpenAITool, OpenAIFunction,
    AnthropicTool, LlamaTool, LlamaFunction,
    create_string_param, create_number_param, create_object_param
};
pub use mock_system::{
    EnhancedMockRegistry, MockDefinition, MockBehavior, MockBuilder
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod token_box_tests {
    use super::*;
    use fct_ast::{LensCallNode, PipelineNode, Span, ValueNode};
    use fct_std::LensRegistry;
    use std::collections::HashMap;

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

        // Fixed load = 100 (critical only)
        // Free space = 1000 - 100 = 900
        // User section has grow weight 0.1, so should get some expansion
        assert!(result.total_size >= 150); // base + expansion
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
        assert!(user_section.final_size >= 50); // with expansion
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
            kwargs: HashMap::new(),
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
}
