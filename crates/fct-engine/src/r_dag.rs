// ============================================================================
// REACTIVE DEPENDENCY GRAPH (R-DAG)
// ============================================================================

use crate::errors::{EngineError, EngineResult};
use fct_ast::{BodyNode, FacetDocument, FacetNode, PipelineNode, ValueNode};
use fct_std::{LensContext, LensRegistry};
use std::collections::{HashMap, HashSet, VecDeque};

/// Node in the dependency graph
#[derive(Debug, Clone)]
pub struct VarNode {
    #[allow(dead_code)] // Field is used by serialization
    pub name: String,
    pub value: ValueNode,
    pub dependencies: Vec<String>,
}

/// Reactive Dependency Graph
pub struct DependencyGraph {
    nodes: HashMap<String, VarNode>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// Build graph from @vars block
    pub fn build_from_document(&mut self, doc: &FacetDocument) -> EngineResult<()> {
        for block in &doc.blocks {
            if let FacetNode::Vars(vars_block) = block {
                for body_node in &vars_block.body {
                    if let BodyNode::KeyValue(kv) = body_node {
                        let dependencies = self.extract_dependencies(&kv.value);

                        let node = VarNode {
                            name: kv.key.clone(),
                            value: kv.value.clone(),
                            dependencies,
                        };

                        self.nodes.insert(kv.key.clone(), node);
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract variable dependencies from a value
    fn extract_dependencies(&self, value: &ValueNode) -> Vec<String> {
        let mut deps = Vec::with_capacity(8); // Pre-allocate with reasonable capacity

        match value {
            ValueNode::Variable(var_name) => {
                deps.push(var_name.clone());
            }
            ValueNode::Directive(d) => {
                // @input considered leaf, no deps
                if d.name != "input" {
                    // other directives ignored
                }
            }
            ValueNode::Pipeline(pipeline) => {
                // Dependencies in initial value
                deps.extend(self.extract_dependencies(&pipeline.initial));

                // Dependencies in lens arguments
                for lens in &pipeline.lenses {
                    for arg in &lens.args {
                        deps.extend(self.extract_dependencies(arg));
                    }
                    for arg in lens.kwargs.values() {
                        deps.extend(self.extract_dependencies(arg));
                    }
                }
            }
            ValueNode::List(items) => {
                for item in items {
                    deps.extend(self.extract_dependencies(item));
                }
            }
            ValueNode::Map(map) => {
                for val in map.values() {
                    deps.extend(self.extract_dependencies(val));
                }
            }
            _ => {}
        }

        deps
    }

    /// Detect cycles using DFS
    pub fn detect_cycles(&self) -> EngineResult<()> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_name in self.nodes.keys() {
            if !visited.contains(node_name) {
                self.dfs_detect_cycle(node_name, &mut visited, &mut rec_stack, &mut vec![])?;
            }
        }

        Ok(())
    }

    fn dfs_detect_cycle(
        &self,
        node_name: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> EngineResult<()> {
        visited.insert(node_name.to_string());
        rec_stack.insert(node_name.to_string());
        path.push(node_name.to_string());

        if let Some(node) = self.nodes.get(node_name) {
            for dep in &node.dependencies {
                if !visited.contains(dep) {
                    self.dfs_detect_cycle(dep, visited, rec_stack, path)?;
                } else if rec_stack.contains(dep) {
                    // Cycle detected
                    path.push(dep.clone());
                    let cycle = path.join(" -> ");
                    return Err(EngineError::CyclicDependency { cycle });
                }
            }
        }

        rec_stack.remove(node_name);
        path.pop();

        Ok(())
    }

    /// Topological sort (Kahn's algorithm)
    pub fn topological_sort(&self) -> EngineResult<Vec<String>> {
        let node_count = self.nodes.len();
        let mut in_degree: HashMap<String, usize> = HashMap::with_capacity(node_count);
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::with_capacity(node_count);

        // Initialize
        for node_name in self.nodes.keys() {
            in_degree.insert(node_name.clone(), 0);
            adj_list.insert(node_name.clone(), Vec::with_capacity(4)); // Most nodes have few dependencies
        }

        // Build adjacency list and in-degree map
        for (node_name, node) in &self.nodes {
            for dep in &node.dependencies {
                if !self.nodes.contains_key(dep) {
                    // Dependency not found - might be @input or external
                    continue;
                }

                adj_list
                    .entry(dep.clone())
                    .or_default()
                    .push(node_name.clone());

                *in_degree.entry(node_name.clone()).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut sorted = Vec::with_capacity(node_count);

        while let Some(node_name) = queue.pop_front() {
            sorted.push(node_name.clone());

            if let Some(neighbors) = adj_list.get(&node_name) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // Check if all nodes are sorted (no cycles)
        if sorted.len() != self.nodes.len() {
            return Err(EngineError::CyclicDependency {
                cycle: "Cycle detected during topological sort".to_string(),
            });
        }

        Ok(sorted)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// R-DAG EXECUTION ENGINE
// ============================================================================

/// Gas context for compute quota enforcement
#[derive(Debug, Clone)]
pub struct GasContext {
    pub limit: usize,
    pub consumed: usize,
}

impl GasContext {
    pub fn new(limit: usize) -> Self {
        Self { limit, consumed: 0 }
    }

    pub fn consume(&mut self, amount: usize) -> EngineResult<()> {
        self.consumed += amount;
        if self.consumed > self.limit {
            return Err(EngineError::GasExhausted { limit: self.limit });
        }
        Ok(())
    }
}

/// Execution context
pub struct ExecutionContext {
    pub variables: HashMap<String, ValueNode>,
    pub gas: GasContext,
    pub lens_registry: LensRegistry,
}

impl ExecutionContext {
    pub fn new(gas_limit: usize) -> Self {
        Self {
            variables: HashMap::new(),
            gas: GasContext::new(gas_limit),
            lens_registry: LensRegistry::new(),
        }
    }

    pub fn set_variable(&mut self, name: String, value: ValueNode) {
        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&ValueNode> {
        self.variables.get(name)
    }
}

/// R-DAG Execution Engine
pub struct RDagEngine {
    graph: DependencyGraph,
}

impl RDagEngine {
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
        }
    }

    /// Build graph from document
    pub fn build(&mut self, doc: &FacetDocument) -> EngineResult<()> {
        self.graph.build_from_document(doc)?;
        Ok(())
    }

    /// Validate graph (detect cycles)
    pub fn validate(&self) -> EngineResult<()> {
        self.graph.detect_cycles()?;
        Ok(())
    }

    /// Execute graph and compute all variables
    pub fn execute(&self, ctx: &mut ExecutionContext) -> EngineResult<()> {
        // Get topological order
        let order = self.graph.topological_sort()?;

        // Execute nodes in order
        for node_name in order {
            ctx.gas.consume(1)?; // Each variable evaluation costs 1 gas

            if let Some(node) = self.graph.nodes.get(&node_name) {
                // Evaluate the variable
                let value = self.evaluate_value(&node.value, ctx)?;
                ctx.set_variable(node_name.clone(), value);
            }
        }

        Ok(())
    }

    /// Evaluate a value node (resolve variables, execute pipelines)
    fn evaluate_value(&self, value: &ValueNode, ctx: &ExecutionContext) -> EngineResult<ValueNode> {
        match value {
            ValueNode::Variable(var_name) => {
                // Lookup variable value
                ctx.get_variable(var_name)
                    .cloned()
                    .ok_or_else(|| EngineError::VariableNotFound {
                        var: var_name.clone(),
                    })
            }
            ValueNode::Directive(d) => {
                if d.name == "input" {
                    // Inputs should be resolved upstream; here we return Null if missing
                    Ok(ValueNode::Scalar(fct_ast::ScalarValue::Null))
                } else {
                    Ok(ValueNode::Directive(d.clone()))
                }
            }
            ValueNode::Pipeline(pipeline) => {
                // Execute lens pipeline
                self.execute_pipeline(pipeline, ctx)
            }
            ValueNode::List(items) => {
                let mut evaluated_items = Vec::new();
                for item in items {
                    evaluated_items.push(self.evaluate_value(item, ctx)?);
                }
                Ok(ValueNode::List(evaluated_items))
            }
            ValueNode::Map(map) => {
                let mut evaluated_map = HashMap::new();
                for (key, val) in map {
                    evaluated_map.insert(key.clone(), self.evaluate_value(val, ctx)?);
                }
                Ok(ValueNode::Map(evaluated_map))
            }
            // Literals evaluate to themselves
            other => Ok(other.clone()),
        }
    }

    /// Execute a lens pipeline
    fn execute_pipeline(
        &self,
        pipeline: &PipelineNode,
        ctx: &ExecutionContext,
    ) -> EngineResult<ValueNode> {
        // Evaluate initial value
        let mut current_value = self.evaluate_value(&pipeline.initial, ctx)?;

        // Create lens context
        let lens_ctx = LensContext {
            variables: ctx.variables.clone(),
        };

        // Execute each lens in sequence
        for lens_call in &pipeline.lenses {
            // Look up lens in registry
            let lens = ctx.lens_registry.get(&lens_call.name).ok_or_else(|| {
                EngineError::LensExecutionFailed {
                    message: format!("Unknown lens: {}", lens_call.name),
                }
            })?;

            // Evaluate arguments
            let mut evaluated_args = Vec::new();
            for arg in &lens_call.args {
                evaluated_args.push(self.evaluate_value(arg, ctx)?);
            }

            let mut evaluated_kwargs = HashMap::new();
            for (key, val) in &lens_call.kwargs {
                evaluated_kwargs.insert(key.clone(), self.evaluate_value(val, ctx)?);
            }

            // Execute lens
            current_value = lens
                .execute(current_value, evaluated_args, evaluated_kwargs, &lens_ctx)
                .map_err(|e| EngineError::LensExecutionFailed {
                    message: format!("Lens '{}' failed: {}", lens_call.name, e),
                })?;
        }

        Ok(current_value)
    }
}

impl Default for RDagEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_consumption() {
        let mut gas = GasContext::new(10);
        assert!(gas.consume(5).is_ok());
        assert_eq!(gas.consumed, 5);
        assert!(gas.consume(3).is_ok());
        assert_eq!(gas.consumed, 8);
        assert!(gas.consume(5).is_err()); // Exceeds limit
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.detect_cycles().is_ok());
        assert_eq!(graph.topological_sort().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_pipeline_execution() {
        use fct_ast::{FacetBlock, KeyValueNode, LensCallNode, Span};

        // Create a simple document with pipeline: "  HELLO  " |> trim() |> lowercase()
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: HashMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "greeting".to_string(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("  HELLO  ".to_string())),
                        lenses: vec![
                            LensCallNode {
                                name: "trim".to_string(),
                                args: vec![],
                                kwargs: HashMap::new(),
                                span: Span {
                                    start: 0,
                                    end: 0,
                                    line: 1,
                                    column: 1,
                                },
                            },
                            LensCallNode {
                                name: "lowercase".to_string(),
                                args: vec![],
                                kwargs: HashMap::new(),
                                span: Span {
                                    start: 0,
                                    end: 0,
                                    line: 1,
                                    column: 1,
                                },
                            },
                        ],
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
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
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        // Build and execute
        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        engine.validate().unwrap();

        let mut ctx = ExecutionContext::new(1000);
        engine.execute(&mut ctx).unwrap();

        // Check result
        let result = ctx.get_variable("greeting").unwrap();
        assert_eq!(result, &ValueNode::String("hello".to_string()));
    }

    #[test]
    fn test_pipeline_with_args() {
        use fct_ast::{FacetBlock, KeyValueNode, LensCallNode, Span};

        // Create document with pipeline: "a,b,c" |> split(",")
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: HashMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "items".to_string(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("a,b,c".to_string())),
                        lenses: vec![LensCallNode {
                            name: "split".to_string(),
                            args: vec![ValueNode::String(",".to_string())],
                            kwargs: HashMap::new(),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 1,
                                column: 1,
                            },
                        }],
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
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
            })],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        // Build and execute
        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();

        let mut ctx = ExecutionContext::new(1000);
        engine.execute(&mut ctx).unwrap();

        // Check result
        let result = ctx.get_variable("items").unwrap();
        match result {
            ValueNode::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], ValueNode::String("a".to_string()));
                assert_eq!(items[1], ValueNode::String("b".to_string()));
                assert_eq!(items[2], ValueNode::String("c".to_string()));
            }
            _ => panic!("Expected list"),
        }
    }
}
