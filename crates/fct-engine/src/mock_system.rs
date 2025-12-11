// ============================================================================
// MOCK SYSTEM - Enhanced Mocking for Testing
// ============================================================================
// Comprehensive mocking system for interfaces, lenses, and tools

use crate::errors::{EngineError, EngineResult};
use crate::tool_executor::{ToolInvocation, ToolResult};
use fct_ast::{ScalarValue, ValueNode};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// MOCK TYPES
// ============================================================================

/// Mock behavior - static value or dynamic handler
#[derive(Clone)]
pub enum MockBehavior {
    /// Static return value
    Static(ValueNode),
    /// Dynamic handler function
    Dynamic(Arc<dyn Fn(&HashMap<String, ValueNode>) -> EngineResult<ValueNode> + Send + Sync>),
}

/// Mock definition with metadata
#[derive(Clone)]
pub struct MockDefinition {
    /// Target identifier (e.g., "WeatherAPI.get_current", "llm_call", "tool_name")
    pub target: String,
    /// Mock behavior
    pub behavior: MockBehavior,
    /// Number of times this mock has been called
    pub call_count: Arc<Mutex<usize>>,
    /// Whether this mock is active
    pub enabled: bool,
}

impl MockDefinition {
    /// Create a static mock
    pub fn static_mock(target: String, value: ValueNode) -> Self {
        Self {
            target,
            behavior: MockBehavior::Static(value),
            call_count: Arc::new(Mutex::new(0)),
            enabled: true,
        }
    }

    /// Create a dynamic mock
    pub fn dynamic_mock<F>(target: String, handler: F) -> Self
    where
        F: Fn(&HashMap<String, ValueNode>) -> EngineResult<ValueNode> + Send + Sync + 'static,
    {
        Self {
            target,
            behavior: MockBehavior::Dynamic(Arc::new(handler)),
            call_count: Arc::new(Mutex::new(0)),
            enabled: true,
        }
    }

    /// Execute mock and return value
    pub fn execute(&self, args: &HashMap<String, ValueNode>) -> EngineResult<ValueNode> {
        // Increment call count
        if let Ok(mut count) = self.call_count.lock() {
            *count += 1;
        }

        match &self.behavior {
            MockBehavior::Static(value) => Ok(value.clone()),
            MockBehavior::Dynamic(handler) => handler(args),
        }
    }

    /// Get call count
    pub fn get_call_count(&self) -> usize {
        self.call_count.lock().map(|c| *c).unwrap_or(0)
    }

    /// Reset call count
    pub fn reset_call_count(&self) {
        if let Ok(mut count) = self.call_count.lock() {
            *count = 0;
        }
    }
}

// ============================================================================
// ENHANCED MOCK REGISTRY
// ============================================================================

/// Enhanced mock registry with tool support
#[derive(Clone, Default)]
pub struct EnhancedMockRegistry {
    /// Interface mocks (e.g., "WeatherAPI.get_current")
    pub interface_mocks: HashMap<String, MockDefinition>,
    /// Lens mocks (e.g., "trim", "uppercase")
    pub lens_mocks: HashMap<String, MockDefinition>,
    /// Tool mocks (e.g., "get_weather", "search_documents")
    pub tool_mocks: HashMap<String, MockDefinition>,
    /// Global mock enable/disable
    pub enabled: bool,
}

impl EnhancedMockRegistry {
    /// Create new mock registry
    pub fn new() -> Self {
        Self {
            interface_mocks: HashMap::new(),
            lens_mocks: HashMap::new(),
            tool_mocks: HashMap::new(),
            enabled: true,
        }
    }

    /// Register interface mock
    pub fn mock_interface(&mut self, target: String, mock: MockDefinition) {
        self.interface_mocks.insert(target, mock);
    }

    /// Register lens mock
    pub fn mock_lens(&mut self, target: String, mock: MockDefinition) {
        self.lens_mocks.insert(target, mock);
    }

    /// Register tool mock
    pub fn mock_tool(&mut self, target: String, mock: MockDefinition) {
        self.tool_mocks.insert(target, mock);
    }

    /// Add static interface mock (convenience method)
    pub fn add_interface_mock(&mut self, target: String, value: ValueNode) {
        self.interface_mocks
            .insert(target.clone(), MockDefinition::static_mock(target, value));
    }

    /// Add static lens mock (convenience method)
    pub fn add_lens_mock(&mut self, target: String, value: ValueNode) {
        self.lens_mocks
            .insert(target.clone(), MockDefinition::static_mock(target, value));
    }

    /// Add static tool mock (convenience method)
    pub fn add_tool_mock(&mut self, target: String, value: ValueNode) {
        self.tool_mocks
            .insert(target.clone(), MockDefinition::static_mock(target, value));
    }

    /// Add dynamic tool mock
    pub fn add_tool_handler<F>(&mut self, target: String, handler: F)
    where
        F: Fn(&HashMap<String, ValueNode>) -> EngineResult<ValueNode> + Send + Sync + 'static,
    {
        self.tool_mocks
            .insert(target.clone(), MockDefinition::dynamic_mock(target, handler));
    }

    /// Check if interface is mocked
    pub fn is_interface_mocked(&self, target: &str) -> bool {
        self.enabled && self.interface_mocks.contains_key(target)
    }

    /// Check if lens is mocked
    pub fn is_lens_mocked(&self, target: &str) -> bool {
        self.enabled && self.lens_mocks.contains_key(target)
    }

    /// Check if tool is mocked
    pub fn is_tool_mocked(&self, target: &str) -> bool {
        self.enabled && self.tool_mocks.contains_key(target)
    }

    /// Execute interface mock
    pub fn execute_interface_mock(
        &self,
        target: &str,
        args: &HashMap<String, ValueNode>,
    ) -> EngineResult<ValueNode> {
        if !self.enabled {
            return Err(EngineError::ExecutionError {
                message: "Mocking is disabled".to_string(),
            });
        }

        self.interface_mocks
            .get(target)
            .ok_or_else(|| EngineError::ExecutionError {
                message: format!("No mock registered for interface '{}'", target),
            })?
            .execute(args)
    }

    /// Execute lens mock
    pub fn execute_lens_mock(
        &self,
        target: &str,
        args: &HashMap<String, ValueNode>,
    ) -> EngineResult<ValueNode> {
        if !self.enabled {
            return Err(EngineError::ExecutionError {
                message: "Mocking is disabled".to_string(),
            });
        }

        self.lens_mocks
            .get(target)
            .ok_or_else(|| EngineError::ExecutionError {
                message: format!("No mock registered for lens '{}'", target),
            })?
            .execute(args)
    }

    /// Execute tool mock
    pub fn execute_tool_mock(
        &self,
        target: &str,
        args: &HashMap<String, ValueNode>,
    ) -> EngineResult<ValueNode> {
        if !self.enabled {
            return Err(EngineError::ExecutionError {
                message: "Mocking is disabled".to_string(),
            });
        }

        self.tool_mocks
            .get(target)
            .ok_or_else(|| EngineError::ExecutionError {
                message: format!("No mock registered for tool '{}'", target),
            })?
            .execute(args)
    }

    /// Intercept tool invocation and return mock result if available
    pub fn intercept_tool_call(&self, invocation: &ToolInvocation) -> Option<ToolResult> {
        if !self.enabled || !self.is_tool_mocked(&invocation.tool_name) {
            return None;
        }

        match self.execute_tool_mock(&invocation.tool_name, &invocation.arguments) {
            Ok(value) => Some(ToolResult {
                tool_name: invocation.tool_name.clone(),
                result: value,
                error: None,
                invocation_id: invocation.invocation_id.clone(),
            }),
            Err(e) => Some(ToolResult {
                tool_name: invocation.tool_name.clone(),
                result: ValueNode::Scalar(ScalarValue::Null),
                error: Some(e.to_string()),
                invocation_id: invocation.invocation_id.clone(),
            }),
        }
    }

    /// Clear all mocks
    pub fn clear(&mut self) {
        self.interface_mocks.clear();
        self.lens_mocks.clear();
        self.tool_mocks.clear();
    }

    /// Reset all call counts
    pub fn reset_call_counts(&self) {
        for mock in self.interface_mocks.values() {
            mock.reset_call_count();
        }
        for mock in self.lens_mocks.values() {
            mock.reset_call_count();
        }
        for mock in self.tool_mocks.values() {
            mock.reset_call_count();
        }
    }

    /// Get call count for a mock
    pub fn get_call_count(&self, target: &str) -> usize {
        self.interface_mocks
            .get(target)
            .or_else(|| self.lens_mocks.get(target))
            .or_else(|| self.tool_mocks.get(target))
            .map(|m| m.get_call_count())
            .unwrap_or(0)
    }

    /// Get total number of registered mocks
    pub fn mock_count(&self) -> usize {
        self.interface_mocks.len() + self.lens_mocks.len() + self.tool_mocks.len()
    }
}

// ============================================================================
// MOCK BUILDER - Fluent API
// ============================================================================

/// Fluent mock builder
pub struct MockBuilder {
    target: String,
    behavior: Option<MockBehavior>,
}

impl MockBuilder {
    /// Create new mock builder
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            behavior: None,
        }
    }

    /// Set static return value
    pub fn returns(mut self, value: ValueNode) -> Self {
        self.behavior = Some(MockBehavior::Static(value));
        self
    }

    /// Set dynamic handler
    pub fn with_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&HashMap<String, ValueNode>) -> EngineResult<ValueNode> + Send + Sync + 'static,
    {
        self.behavior = Some(MockBehavior::Dynamic(Arc::new(handler)));
        self
    }

    /// Build mock definition
    pub fn build(self) -> EngineResult<MockDefinition> {
        let behavior = self.behavior.ok_or_else(|| EngineError::ExecutionError {
            message: "Mock behavior not set".to_string(),
        })?;

        Ok(MockDefinition {
            target: self.target.clone(),
            behavior,
            call_count: Arc::new(Mutex::new(0)),
            enabled: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_mock() {
        let mock = MockDefinition::static_mock(
            "test_tool".to_string(),
            ValueNode::String("mocked result".to_string()),
        );

        let args = HashMap::new();
        let result = mock.execute(&args).unwrap();

        assert_eq!(result, ValueNode::String("mocked result".to_string()));
        assert_eq!(mock.get_call_count(), 1);
    }

    #[test]
    fn test_dynamic_mock() {
        let mock = MockDefinition::dynamic_mock("calc".to_string(), |args| {
            if let Some(ValueNode::Scalar(ScalarValue::Int(x))) = args.get("x") {
                Ok(ValueNode::Scalar(ScalarValue::Int(x * 2)))
            } else {
                Ok(ValueNode::Scalar(ScalarValue::Int(0)))
            }
        });

        let mut args = HashMap::new();
        args.insert(
            "x".to_string(),
            ValueNode::Scalar(ScalarValue::Int(21)),
        );

        let result = mock.execute(&args).unwrap();
        assert_eq!(result, ValueNode::Scalar(ScalarValue::Int(42)));
    }

    #[test]
    fn test_mock_registry() {
        let mut registry = EnhancedMockRegistry::new();

        // Add tool mock
        registry.add_tool_mock(
            "weather".to_string(),
            ValueNode::String("Sunny".to_string()),
        );

        assert!(registry.is_tool_mocked("weather"));
        assert!(!registry.is_tool_mocked("unknown"));

        // Execute mock
        let args = HashMap::new();
        let result = registry.execute_tool_mock("weather", &args).unwrap();
        assert_eq!(result, ValueNode::String("Sunny".to_string()));
    }

    #[test]
    fn test_mock_builder() {
        let mock = MockBuilder::new("test")
            .returns(ValueNode::String("result".to_string()))
            .build()
            .unwrap();

        assert_eq!(mock.target, "test");
        assert_eq!(mock.get_call_count(), 0);

        let result = mock.execute(&HashMap::new()).unwrap();
        assert_eq!(result, ValueNode::String("result".to_string()));
        assert_eq!(mock.get_call_count(), 1);
    }

    #[test]
    fn test_intercept_tool_call() {
        let mut registry = EnhancedMockRegistry::new();
        registry.add_tool_mock(
            "search".to_string(),
            ValueNode::List(vec![ValueNode::String("result1".to_string())]),
        );

        let invocation = ToolInvocation {
            tool_name: "search".to_string(),
            arguments: HashMap::new(),
            invocation_id: Some("test-1".to_string()),
        };

        let result = registry.intercept_tool_call(&invocation).unwrap();
        assert_eq!(result.tool_name, "search");
        assert!(result.error.is_none());
        assert_eq!(result.invocation_id, Some("test-1".to_string()));
    }

    #[test]
    fn test_call_counts() {
        let mut registry = EnhancedMockRegistry::new();
        registry.add_tool_mock("counter".to_string(), ValueNode::Scalar(ScalarValue::Int(1)));

        let args = HashMap::new();

        // Execute multiple times
        registry.execute_tool_mock("counter", &args).unwrap();
        registry.execute_tool_mock("counter", &args).unwrap();
        registry.execute_tool_mock("counter", &args).unwrap();

        assert_eq!(registry.get_call_count("counter"), 3);

        // Reset
        registry.reset_call_counts();
        assert_eq!(registry.get_call_count("counter"), 0);
    }

    #[test]
    fn test_clear_mocks() {
        let mut registry = EnhancedMockRegistry::new();
        registry.add_tool_mock("tool1".to_string(), ValueNode::Scalar(ScalarValue::Int(1)));
        registry.add_lens_mock("lens1".to_string(), ValueNode::String("lens".to_string()));

        assert_eq!(registry.mock_count(), 2);

        registry.clear();
        assert_eq!(registry.mock_count(), 0);
    }
}
