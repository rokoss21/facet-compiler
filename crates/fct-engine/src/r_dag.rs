// ============================================================================
// REACTIVE DEPENDENCY GRAPH (R-DAG)
// ============================================================================

use crate::errors::{EngineError, EngineResult};
use fct_ast::{
    BodyNode, FacetDocument, FacetNode, OrderedMap, PipelineNode, ScalarValue, ValueNode,
    FACET_VERSION, POLICY_VERSION,
};
use fct_std::{LensContext, LensRegistry, TrustLevel};
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

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
    insertion_order: Vec<String>,
    order_index: HashMap<String, usize>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            insertion_order: Vec::new(),
            order_index: HashMap::new(),
        }
    }

    /// Build graph from @vars block
    pub fn build_from_document(&mut self, doc: &FacetDocument) -> EngineResult<()> {
        self.nodes.clear();
        self.insertion_order.clear();
        self.order_index.clear();

        for block in &doc.blocks {
            if let FacetNode::Vars(vars_block) = block {
                for body_node in &vars_block.body {
                    if let BodyNode::KeyValue(kv) = body_node {
                        let mut dependencies = Self::extract_dependencies(&kv.value);
                        let mut seen = HashSet::new();
                        dependencies.retain(|dep| seen.insert(dep.clone()));

                        let node = VarNode {
                            name: kv.key.clone(),
                            value: kv.value.clone(),
                            dependencies,
                        };

                        if self.nodes.contains_key(&kv.key) {
                            self.nodes.insert(kv.key.clone(), node);
                        } else {
                            let idx = self.insertion_order.len();
                            self.order_index.insert(kv.key.clone(), idx);
                            self.insertion_order.push(kv.key.clone());
                            self.nodes.insert(kv.key.clone(), node);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Extract variable dependencies from a value
    fn extract_dependencies(value: &ValueNode) -> Vec<String> {
        let mut deps = Vec::with_capacity(8); // Pre-allocate with reasonable capacity

        match value {
            ValueNode::Variable(var_name) => {
                deps.push(base_var_name(var_name).to_string());
            }
            ValueNode::Directive(d) => {
                // @input considered leaf, no deps
                if d.name != "input" {
                    // other directives ignored
                }
            }
            ValueNode::Pipeline(pipeline) => {
                // Dependencies in initial value
                deps.extend(Self::extract_dependencies(&pipeline.initial));

                // Dependencies in lens arguments
                for lens in &pipeline.lenses {
                    for arg in &lens.args {
                        deps.extend(Self::extract_dependencies(arg));
                    }
                    for arg in lens.kwargs.values() {
                        deps.extend(Self::extract_dependencies(arg));
                    }
                }
            }
            ValueNode::List(items) => {
                for item in items {
                    deps.extend(Self::extract_dependencies(item));
                }
            }
            ValueNode::Map(map) => {
                for val in map.values() {
                    deps.extend(Self::extract_dependencies(val));
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

        for node_name in &self.insertion_order {
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
        for node_name in &self.insertion_order {
            in_degree.insert(node_name.clone(), 0);
            adj_list.insert(node_name.clone(), Vec::with_capacity(4)); // Most nodes have few dependencies
        }

        // Build adjacency list and in-degree map
        for node_name in &self.insertion_order {
            let node = self.nodes.get(node_name).expect("node must exist");
            for dep in &node.dependencies {
                if !self.nodes.contains_key(dep) {
                    return Err(EngineError::VariableNotFound { var: dep.clone() });
                }

                adj_list
                    .entry(dep.clone())
                    .or_default()
                    .push(node_name.clone());

                *in_degree.entry(node_name.clone()).or_insert(0) += 1;
            }
        }

        // Find all nodes with in-degree 0 in deterministic merged insertion order.
        let mut queue: BinaryHeap<Reverse<(usize, String)>> = BinaryHeap::new();
        for node_name in &self.insertion_order {
            if in_degree.get(node_name).copied().unwrap_or(0) == 0 {
                let idx = *self
                    .order_index
                    .get(node_name)
                    .expect("order index must exist for every node");
                queue.push(Reverse((idx, node_name.clone())));
            }
        }

        let mut sorted = Vec::with_capacity(node_count);

        while let Some(Reverse((_, node_name))) = queue.pop() {
            sorted.push(node_name.clone());

            if let Some(neighbors) = adj_list.get(&node_name) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            let idx = *self
                                .order_index
                                .get(neighbor)
                                .expect("order index must exist for neighbor");
                            queue.push(Reverse((idx, neighbor.clone())));
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

fn base_var_name(var_ref: &str) -> &str {
    var_ref.split('.').next().unwrap_or(var_ref)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    Pure,
    Exec,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionGuardDecision {
    pub seq: usize,
    pub op: String,
    pub name: String,
    pub effect_class: Option<String>,
    pub mode: String,
    pub decision: String,
    pub policy_rule_id: Option<String>,
    pub input_hash: String,
    pub error_code: Option<String>,
}

#[derive(Debug, Clone)]
struct LensPolicyDecision {
    allowed: bool,
    policy_rule_id: Option<String>,
    error_code: Option<String>,
}

/// Execution context
pub struct ExecutionContext {
    pub variables: HashMap<String, ValueNode>,
    pub runtime_inputs: HashMap<String, ValueNode>,
    pub lens_cache: HashMap<String, ValueNode>,
    pub guard_decisions: Vec<ExecutionGuardDecision>,
    pub effective_policy: Option<OrderedMap<String, ValueNode>>,
    pub policy_hash: Option<String>,
    pub gas: GasContext,
    pub lens_registry: LensRegistry,
    pub mode: ExecutionMode,
    pub host_profile_id: String,
    variables_frozen: bool,
    next_guard_seq: usize,
}

impl ExecutionContext {
    pub fn new(gas_limit: usize) -> Self {
        Self::new_with_mode(gas_limit, ExecutionMode::Exec)
    }

    pub fn new_with_mode(gas_limit: usize, mode: ExecutionMode) -> Self {
        Self {
            variables: HashMap::new(),
            runtime_inputs: HashMap::new(),
            lens_cache: HashMap::new(),
            guard_decisions: Vec::new(),
            effective_policy: None,
            policy_hash: None,
            gas: GasContext::new(gas_limit),
            lens_registry: LensRegistry::new(),
            mode,
            host_profile_id: "local.default.v1".to_string(),
            variables_frozen: false,
            next_guard_seq: 1,
        }
    }

    pub fn set_variable(&mut self, name: String, value: ValueNode) -> EngineResult<()> {
        if self.variables_frozen {
            return Err(EngineError::ConstraintViolation {
                message: "Computed variable map is immutable after Phase 3 execution".to_string(),
            });
        }
        self.variables.insert(name, value);
        Ok(())
    }

    pub fn get_variable(&self, name: &str) -> Option<&ValueNode> {
        self.variables.get(name)
    }

    pub fn set_input(&mut self, name: String, value: ValueNode) {
        self.runtime_inputs.insert(name, value);
    }

    pub fn set_inputs(&mut self, inputs: HashMap<String, ValueNode>) {
        self.runtime_inputs = inputs;
    }

    pub fn get_input(&self, name: &str) -> Option<&ValueNode> {
        self.runtime_inputs.get(name)
    }

    pub fn set_lens_cache_entry(&mut self, key: String, value: ValueNode) {
        self.lens_cache.insert(key, value);
    }

    pub fn get_lens_cache_entry(&self, key: &str) -> Option<&ValueNode> {
        self.lens_cache.get(key)
    }

    pub fn record_guard_decision(&mut self, mut decision: ExecutionGuardDecision) {
        decision.seq = self.next_guard_seq;
        self.next_guard_seq += 1;
        self.guard_decisions.push(decision);
    }

    pub fn freeze_variables(&mut self) {
        self.variables_frozen = true;
    }

    pub fn is_variables_frozen(&self) -> bool {
        self.variables_frozen
    }
}

/// R-DAG Execution Engine
pub struct RDagEngine {
    graph: DependencyGraph,
    effective_policy: Option<OrderedMap<String, ValueNode>>,
}

impl RDagEngine {
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
            effective_policy: None,
        }
    }

    /// Build graph from document
    pub fn build(&mut self, doc: &FacetDocument) -> EngineResult<()> {
        self.graph.build_from_document(doc)?;
        self.effective_policy = collect_effective_policy(doc);
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
                let value = self.evaluate_value(&node.value, ctx, Some(&node_name))?;
                ctx.set_variable(node_name.clone(), value)?;
            }
        }

        ctx.effective_policy = self.effective_policy.clone();
        ctx.policy_hash = compute_policy_hash(self.effective_policy.as_ref())?;
        ctx.freeze_variables();
        Ok(())
    }

    /// Evaluate a value node (resolve variables, execute pipelines)
    fn evaluate_value(
        &self,
        value: &ValueNode,
        ctx: &mut ExecutionContext,
        current_var: Option<&str>,
    ) -> EngineResult<ValueNode> {
        match value {
            ValueNode::Variable(var_name) => self.resolve_variable_ref(var_name, ctx),
            ValueNode::Directive(d) => {
                if d.name == "input" {
                    self.resolve_input_directive(d, current_var, ctx)
                } else {
                    Ok(ValueNode::Directive(d.clone()))
                }
            }
            ValueNode::Pipeline(pipeline) => {
                // Execute lens pipeline
                self.execute_pipeline(pipeline, ctx, current_var)
            }
            ValueNode::List(items) => {
                let mut evaluated_items = Vec::new();
                for item in items {
                    evaluated_items.push(self.evaluate_value(item, ctx, current_var)?);
                }
                Ok(ValueNode::List(evaluated_items))
            }
            ValueNode::Map(map) => {
                let mut evaluated_map = OrderedMap::new();
                for (key, val) in map {
                    evaluated_map.insert(key.clone(), self.evaluate_value(val, ctx, current_var)?);
                }
                Ok(ValueNode::Map(evaluated_map))
            }
            // Literals evaluate to themselves
            other => Ok(other.clone()),
        }
    }

    fn resolve_input_directive(
        &self,
        directive: &fct_ast::DirectiveNode,
        current_var: Option<&str>,
        ctx: &mut ExecutionContext,
    ) -> EngineResult<ValueNode> {
        let var_name = current_var.ok_or_else(|| EngineError::InputValidationFailed {
            message: "@input(...) used outside variable evaluation context".to_string(),
        })?;

        let declared_type = match directive.args.get("type") {
            Some(ValueNode::String(s)) => s.as_str(),
            _ => {
                return Err(EngineError::InputValidationFailed {
                    message: format!(
                        "@input(type=...) is required and must be string for '{}'",
                        var_name
                    ),
                })
            }
        };

        let value = if let Some(provided) = ctx.get_input(var_name) {
            provided.clone()
        } else if let Some(default) = directive.args.get("default") {
            default.clone()
        } else {
            return Err(EngineError::InputValidationFailed {
                message: format!("Missing required runtime input for '{}'", var_name),
            });
        };

        if !Self::value_matches_runtime_type(&value, declared_type) {
            return Err(EngineError::InputValidationFailed {
                message: format!(
                    "Input '{}' does not satisfy declared type '{}'",
                    var_name, declared_type
                ),
            });
        }

        Ok(value)
    }

    fn value_matches_runtime_type(value: &ValueNode, declared_type: &str) -> bool {
        match declared_type {
            "any" => true,
            "string" => matches!(value, ValueNode::String(_)),
            "int" => matches!(value, ValueNode::Scalar(ScalarValue::Int(_))),
            "float" => matches!(
                value,
                ValueNode::Scalar(ScalarValue::Float(_)) | ValueNode::Scalar(ScalarValue::Int(_))
            ),
            "bool" => matches!(value, ValueNode::Scalar(ScalarValue::Bool(_))),
            "null" => matches!(value, ValueNode::Scalar(ScalarValue::Null)),
            _ => false,
        }
    }

    fn resolve_variable_ref(
        &self,
        var_ref: &str,
        ctx: &ExecutionContext,
    ) -> EngineResult<ValueNode> {
        let mut parts = var_ref.split('.');
        let base = parts.next().unwrap_or(var_ref);
        let segments: Vec<&str> = parts.collect();

        let mut current =
            ctx.get_variable(base)
                .cloned()
                .ok_or_else(|| EngineError::VariableNotFound {
                    var: base.to_string(),
                })?;

        if segments.is_empty() {
            return Ok(current);
        }

        for segment in segments {
            if segment.chars().all(|c| c.is_ascii_digit()) {
                return Err(EngineError::ExecutionError {
                    message: format!(
                        "F452: numeric indexing is not standardized in v2.1.3 for '{}'",
                        var_ref
                    ),
                });
            }

            current =
                match current {
                    ValueNode::Map(map) => map.get(segment).cloned().ok_or_else(|| {
                        EngineError::InvalidVariablePath {
                            path: var_ref.to_string(),
                        }
                    })?,
                    _ => {
                        return Err(EngineError::InvalidVariablePath {
                            path: var_ref.to_string(),
                        })
                    }
                };
        }

        Ok(current)
    }

    /// Execute a lens pipeline
    fn execute_pipeline(
        &self,
        pipeline: &PipelineNode,
        ctx: &mut ExecutionContext,
        current_var: Option<&str>,
    ) -> EngineResult<ValueNode> {
        // Evaluate initial value
        let mut current_value = self.evaluate_value(&pipeline.initial, ctx, current_var)?;

        // Create lens context
        let lens_ctx = LensContext {
            variables: ctx.variables.clone(),
        };

        // Execute each lens in sequence
        for lens_call in &pipeline.lenses {
            let (signature, effect_class, lens_version) = {
                let lens = ctx.lens_registry.get(&lens_call.name).ok_or_else(|| {
                    EngineError::UnknownLens {
                        name: lens_call.name.clone(),
                    }
                })?;
                (
                    lens.signature(),
                    lens.effect_class().map(str::to_string),
                    lens.version().to_string(),
                )
            };

            if matches!(
                signature.trust_level,
                TrustLevel::Bounded | TrustLevel::Volatile
            ) {
                let effect = effect_class.as_deref().ok_or_else(|| {
                    EngineError::InvalidEffectDeclaration {
                        message: format!(
                            "Lens '{}' (trust_level={:?}) is missing required effect_class",
                            lens_call.name, signature.trust_level
                        ),
                    }
                })?;
                if !is_valid_effect_class(effect) {
                    return Err(EngineError::InvalidEffectDeclaration {
                        message: format!(
                            "Lens '{}' has invalid effect_class '{}'",
                            lens_call.name, effect
                        ),
                    });
                }
            }

            let pre_lens_input = current_value.clone();

            if ctx.mode == ExecutionMode::Pure {
                match signature.trust_level {
                    TrustLevel::Pure => {}
                    TrustLevel::Bounded => {
                        let cache_key = self.level1_cache_key(
                            &lens_call.name,
                            &lens_version,
                            &pre_lens_input,
                            &lens_call.args,
                            &lens_call.kwargs,
                            &ctx.host_profile_id,
                        )?;
                        if let Some(cached) = ctx.get_lens_cache_entry(&cache_key).cloned() {
                            current_value = cached;
                            continue;
                        }
                        return Err(EngineError::ExecutionError {
                            message: format!(
                                "Pure cache miss (Level-1 lens '{}' requires cache-only hit)",
                                lens_call.name
                            ),
                        });
                    }
                    TrustLevel::Volatile => {
                        return Err(EngineError::LensExecutionFailed {
                            message: format!(
                                "Lens '{}' is disallowed in pure mode",
                                lens_call.name
                            ),
                        });
                    }
                }
            }

            // Guard check for dangerous lens operations before invocation.
            if self.should_guard_lens_call(signature.trust_level, ctx.mode) {
                let guard =
                    self.evaluate_lens_call_policy(&lens_call.name, effect_class.as_deref(), ctx)?;
                let input_hash = self.lens_call_input_hash(
                    &lens_call.name,
                    &lens_version,
                    &pre_lens_input,
                    &lens_call.args,
                    &lens_call.kwargs,
                    &ctx.host_profile_id,
                )?;
                ctx.record_guard_decision(ExecutionGuardDecision {
                    seq: 0,
                    op: "lens_call".to_string(),
                    name: lens_call.name.clone(),
                    effect_class: effect_class.clone(),
                    mode: match ctx.mode {
                        ExecutionMode::Pure => "pure".to_string(),
                        ExecutionMode::Exec => "exec".to_string(),
                    },
                    decision: if guard.allowed {
                        "allowed".to_string()
                    } else {
                        "denied".to_string()
                    },
                    policy_rule_id: guard.policy_rule_id.clone(),
                    input_hash,
                    error_code: guard.error_code.clone(),
                });

                if guard.error_code.as_deref() == Some("F455") {
                    return Err(EngineError::GuardUndecidable {
                        name: lens_call.name.clone(),
                    });
                }
                if !guard.allowed {
                    return Err(EngineError::PolicyDenied {
                        name: lens_call.name.clone(),
                    });
                }
            }

            // Evaluate arguments
            let mut evaluated_args = Vec::new();
            for arg in &lens_call.args {
                evaluated_args.push(self.evaluate_value(arg, ctx, current_var)?);
            }

            let mut evaluated_kwargs = HashMap::new();
            for (key, val) in &lens_call.kwargs {
                evaluated_kwargs.insert(key.clone(), self.evaluate_value(val, ctx, current_var)?);
            }

            // Look up lens in registry
            let lens =
                ctx.lens_registry
                    .get(&lens_call.name)
                    .ok_or_else(|| EngineError::UnknownLens {
                        name: lens_call.name.clone(),
                    })?;

            let lens_gas_cost = lens.gas_cost(&current_value, &evaluated_args, &evaluated_kwargs);
            ctx.gas.consume(lens_gas_cost)?;

            // Execute lens
            current_value = lens
                .execute(current_value, evaluated_args, evaluated_kwargs, &lens_ctx)
                .map_err(|e| EngineError::LensExecutionFailed {
                    message: format!("Lens '{}' failed: {}", lens_call.name, e),
                })?;

            if ctx.mode == ExecutionMode::Exec
                && matches!(signature.trust_level, TrustLevel::Bounded)
            {
                let cache_key = self.level1_cache_key(
                    &lens_call.name,
                    &lens_version,
                    &pre_lens_input,
                    &lens_call.args,
                    &lens_call.kwargs,
                    &ctx.host_profile_id,
                )?;
                ctx.set_lens_cache_entry(cache_key, current_value.clone());
            }
        }

        Ok(current_value)
    }

    fn should_guard_lens_call(&self, trust_level: TrustLevel, mode: ExecutionMode) -> bool {
        matches!(trust_level, TrustLevel::Volatile)
            || (matches!(trust_level, TrustLevel::Bounded) && mode == ExecutionMode::Exec)
    }

    fn evaluate_lens_call_policy(
        &self,
        lens_name: &str,
        effect_class: Option<&str>,
        ctx: &ExecutionContext,
    ) -> EngineResult<LensPolicyDecision> {
        let Some(policy_map) = self.effective_policy.as_ref() else {
            return Ok(LensPolicyDecision {
                allowed: false,
                policy_rule_id: None,
                error_code: Some("F454".to_string()),
            });
        };

        if let Some(deny_rules) = policy_map.get("deny").and_then(as_rule_list) {
            for rule in deny_rules {
                match rule_matches_lens_call(rule, lens_name, effect_class, Some(&ctx.variables)) {
                    RuleMatch::Matched(rule_id) => {
                        return Ok(LensPolicyDecision {
                            allowed: false,
                            policy_rule_id: rule_id,
                            error_code: Some("F454".to_string()),
                        });
                    }
                    RuleMatch::Undecidable(rule_id) => {
                        return Ok(LensPolicyDecision {
                            allowed: false,
                            policy_rule_id: rule_id,
                            error_code: Some("F455".to_string()),
                        });
                    }
                    RuleMatch::NoMatch => {}
                }
            }
        }

        if let Some(allow_rules) = policy_map.get("allow").and_then(as_rule_list) {
            for rule in allow_rules {
                match rule_matches_lens_call(rule, lens_name, effect_class, Some(&ctx.variables)) {
                    RuleMatch::Matched(rule_id) => {
                        return Ok(LensPolicyDecision {
                            allowed: true,
                            policy_rule_id: rule_id,
                            error_code: None,
                        });
                    }
                    RuleMatch::Undecidable(rule_id) => {
                        return Ok(LensPolicyDecision {
                            allowed: false,
                            policy_rule_id: rule_id,
                            error_code: Some("F455".to_string()),
                        });
                    }
                    RuleMatch::NoMatch => {}
                }
            }
        }

        Ok(default_guard_decision(policy_map, "lens_call", false))
    }

    fn lens_call_input_hash(
        &self,
        lens_name: &str,
        lens_version: &str,
        input: &ValueNode,
        args: &[ValueNode],
        named_args: &OrderedMap<String, ValueNode>,
        host_profile_id: &str,
    ) -> EngineResult<String> {
        let cache_key = self.level1_cache_key(
            lens_name,
            lens_version,
            input,
            args,
            named_args,
            host_profile_id,
        )?;
        Ok(format!("sha256:{}", cache_key))
    }

    fn level1_cache_key(
        &self,
        lens_name: &str,
        lens_version: &str,
        input: &ValueNode,
        args: &[ValueNode],
        named_args: &OrderedMap<String, ValueNode>,
        host_profile_id: &str,
    ) -> EngineResult<String> {
        let envelope = serde_json::json!({
            "lens": {
                "name": lens_name,
                "version": lens_version,
            },
            "input": value_node_to_json(input)?,
            "args": value_nodes_to_json(args)?,
            "named_args": ordered_map_to_json(named_args)?,
            "host_profile_id": host_profile_id,
            "facet_version": FACET_VERSION,
        });
        let canonical = canonicalize_json(&envelope)?;
        Ok(format!("{:x}", Sha256::digest(canonical.as_bytes())))
    }
}

fn collect_effective_policy(document: &FacetDocument) -> Option<OrderedMap<String, ValueNode>> {
    let mut effective: OrderedMap<String, ValueNode> = OrderedMap::new();
    let mut seen_policy = false;

    for node in &document.blocks {
        if let FacetNode::Policy(policy_block) = node {
            seen_policy = true;
            for body in &policy_block.body {
                if let BodyNode::KeyValue(kv) = body {
                    if let Some(existing) = effective.get_mut(&kv.key) {
                        let merged =
                            merge_policy_value(&kv.key, existing.clone(), kv.value.clone());
                        *existing = merged;
                    } else {
                        effective.insert(kv.key.clone(), kv.value.clone());
                    }
                }
            }
        }
    }

    seen_policy.then_some(effective)
}

fn compute_policy_hash(
    effective_policy: Option<&OrderedMap<String, ValueNode>>,
) -> EngineResult<Option<String>> {
    let Some(policy_map) = effective_policy else {
        return Ok(None);
    };

    let envelope = serde_json::json!({
        "policy_version": POLICY_VERSION,
        "policy": ordered_map_to_json(policy_map)?,
    });
    let canonical = canonicalize_json(&envelope)?;
    let digest = Sha256::digest(canonical.as_bytes());
    Ok(Some(format!("sha256:{:x}", digest)))
}

fn merge_policy_value(key: &str, old: ValueNode, new: ValueNode) -> ValueNode {
    match (old, new) {
        (ValueNode::Map(old_map), ValueNode::Map(new_map)) => {
            ValueNode::Map(merge_policy_maps(old_map, new_map))
        }
        (ValueNode::List(old_list), ValueNode::List(new_list))
            if key == "allow" || key == "deny" =>
        {
            ValueNode::List(merge_policy_lists(old_list, new_list))
        }
        (_, replacement) => replacement,
    }
}

fn merge_policy_maps(
    mut old_map: OrderedMap<String, ValueNode>,
    new_map: OrderedMap<String, ValueNode>,
) -> OrderedMap<String, ValueNode> {
    for (key, new_value) in new_map {
        if let Some(old_value) = old_map.get_mut(&key) {
            *old_value = merge_policy_value(&key, old_value.clone(), new_value);
        } else {
            old_map.insert(key, new_value);
        }
    }
    old_map
}

fn merge_policy_lists(old_items: Vec<ValueNode>, new_items: Vec<ValueNode>) -> Vec<ValueNode> {
    let mut merged = Vec::new();
    let mut id_index: OrderedMap<String, usize> = OrderedMap::new();

    for item in old_items {
        if let Some(id) = policy_rule_id(&item) {
            id_index.insert(id.to_string(), merged.len());
        }
        merged.push(item);
    }

    for item in new_items {
        if let Some(id) = policy_rule_id(&item) {
            if let Some(pos) = id_index.get(id).copied() {
                let old_item = merged[pos].clone();
                merged[pos] = merge_policy_value("", old_item, item);
            } else {
                id_index.insert(id.to_string(), merged.len());
                merged.push(item);
            }
        } else {
            merged.push(item);
        }
    }

    merged
}

fn policy_rule_id(item: &ValueNode) -> Option<&str> {
    match item {
        ValueNode::Map(map) => match map.get("id") {
            Some(ValueNode::String(id)) => Some(id.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn as_rule_list(value: &ValueNode) -> Option<&Vec<ValueNode>> {
    match value {
        ValueNode::List(items) => Some(items),
        _ => None,
    }
}

enum RuleMatch {
    Matched(Option<String>),
    Undecidable(Option<String>),
    NoMatch,
}

fn rule_matches_lens_call(
    rule: &ValueNode,
    lens_name: &str,
    lens_effect_class: Option<&str>,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> RuleMatch {
    let ValueNode::Map(map) = rule else {
        return RuleMatch::NoMatch;
    };

    let Some(ValueNode::String(op)) = map.get("op") else {
        return RuleMatch::NoMatch;
    };
    if op != "lens_call" {
        return RuleMatch::NoMatch;
    }
    let rule_id = match map.get("id") {
        Some(ValueNode::String(id)) => Some(id.clone()),
        _ => None,
    };

    let name_match = match map.get("name") {
        Some(ValueNode::String(pattern)) => matcher_matches(pattern, lens_name),
        Some(_) => return RuleMatch::Undecidable(rule_id),
        None => true,
    };
    if !name_match {
        return RuleMatch::NoMatch;
    }

    if let Some(effect_matcher) = map.get("effect") {
        match effect_matcher {
            ValueNode::String(pattern) => {
                let Some(effect_class) = lens_effect_class else {
                    return RuleMatch::NoMatch;
                };
                if !matcher_matches(pattern, effect_class) {
                    return RuleMatch::NoMatch;
                }
            }
            _ => return RuleMatch::Undecidable(rule_id),
        }
    }

    let when_eval = match map.get("when") {
        None => true,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return RuleMatch::Undecidable(rule_id),
        },
    };
    if !when_eval {
        return RuleMatch::NoMatch;
    }

    let unless_eval = match map.get("unless") {
        None => false,
        Some(cond) => match eval_policy_cond(cond, computed_vars) {
            Ok(v) => v,
            Err(()) => return RuleMatch::Undecidable(rule_id),
        },
    };
    if unless_eval {
        return RuleMatch::NoMatch;
    }

    RuleMatch::Matched(rule_id)
}

fn default_guard_decision(
    policy_map: &OrderedMap<String, ValueNode>,
    op: &str,
    fallback_allow: bool,
) -> LensPolicyDecision {
    if let Some(defaults_node) = policy_map.get("defaults") {
        let defaults_map = match defaults_node {
            ValueNode::Map(map) => map,
            _ => {
                return LensPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F455".to_string()),
                }
            }
        };

        if let Some(op_default) = defaults_map.get(op) {
            return match op_default {
                ValueNode::String(s) if s == "allow" => LensPolicyDecision {
                    allowed: true,
                    policy_rule_id: None,
                    error_code: None,
                },
                ValueNode::String(s) if s == "deny" => LensPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F454".to_string()),
                },
                ValueNode::Scalar(ScalarValue::Bool(true)) => LensPolicyDecision {
                    allowed: true,
                    policy_rule_id: None,
                    error_code: None,
                },
                ValueNode::Scalar(ScalarValue::Bool(false)) => LensPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F454".to_string()),
                },
                _ => LensPolicyDecision {
                    allowed: false,
                    policy_rule_id: None,
                    error_code: Some("F455".to_string()),
                },
            };
        }
    }

    LensPolicyDecision {
        allowed: fallback_allow,
        policy_rule_id: None,
        error_code: if fallback_allow {
            None
        } else {
            Some("F454".to_string())
        },
    }
}

fn eval_policy_cond(
    cond: &ValueNode,
    computed_vars: Option<&HashMap<String, ValueNode>>,
) -> Result<bool, ()> {
    match cond {
        ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(*v),
        ValueNode::Variable(var_ref) => {
            let vars = computed_vars.ok_or(())?;
            let value = resolve_policy_var(var_ref, vars)?;
            match value {
                ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(*v),
                _ => Err(()),
            }
        }
        ValueNode::Map(map) => {
            if map.len() != 1 {
                return Err(());
            }
            let (op, arg) = map.iter().next().ok_or(())?;
            match op.as_str() {
                "not" => Ok(!eval_policy_cond(arg, computed_vars)?),
                "all" => {
                    let items = match arg {
                        ValueNode::List(items) if !items.is_empty() => items,
                        _ => return Err(()),
                    };
                    for item in items {
                        if !eval_policy_cond(item, computed_vars)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                }
                "any" => {
                    let items = match arg {
                        ValueNode::List(items) if !items.is_empty() => items,
                        _ => return Err(()),
                    };
                    for item in items {
                        if eval_policy_cond(item, computed_vars)? {
                            return Ok(true);
                        }
                    }
                    Ok(false)
                }
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

fn resolve_policy_var<'a>(
    var_ref: &str,
    vars: &'a HashMap<String, ValueNode>,
) -> Result<&'a ValueNode, ()> {
    let mut parts = var_ref.split('.');
    let base = parts.next().ok_or(())?;
    let mut current = vars.get(base).ok_or(())?;

    for seg in parts {
        if seg.chars().all(|c| c.is_ascii_digit()) {
            return Err(());
        }
        current = match current {
            ValueNode::Map(map) => map.get(seg).ok_or(())?,
            _ => return Err(()),
        };
    }

    Ok(current)
}

fn matcher_matches(pattern: &str, value: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix(".*") {
        value.starts_with(prefix)
    } else {
        pattern == value
    }
}

fn is_valid_effect_class(effect: &str) -> bool {
    matches!(
        effect,
        "read" | "write" | "external" | "payment" | "filesystem" | "network"
    ) || {
        let mut parts = effect.split('.');
        matches!(parts.next(), Some("x"))
            && parts.next().map(is_identifier).unwrap_or(false)
            && parts.next().map(is_identifier).unwrap_or(false)
            && parts.next().is_none()
    }
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn value_nodes_to_json(items: &[ValueNode]) -> EngineResult<serde_json::Value> {
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        out.push(value_node_to_json(item)?);
    }
    Ok(serde_json::Value::Array(out))
}

fn ordered_map_to_json(map: &OrderedMap<String, ValueNode>) -> EngineResult<serde_json::Value> {
    let mut out = serde_json::Map::new();
    for (k, v) in map {
        out.insert(k.clone(), value_node_to_json(v)?);
    }
    Ok(serde_json::Value::Object(out))
}

fn value_node_to_json(value: &ValueNode) -> EngineResult<serde_json::Value> {
    match value {
        ValueNode::Scalar(ScalarValue::Int(v)) => Ok(serde_json::json!(v)),
        ValueNode::Scalar(ScalarValue::Float(v)) => Ok(serde_json::json!(v)),
        ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(serde_json::json!(v)),
        ValueNode::Scalar(ScalarValue::Null) => Ok(serde_json::Value::Null),
        ValueNode::String(v) => Ok(serde_json::json!(v)),
        ValueNode::Variable(v) => Ok(serde_json::json!(format!("${v}"))),
        ValueNode::Directive(d) => serde_json::to_value(d).map_err(EngineError::JsonError),
        ValueNode::Pipeline(p) => serde_json::to_value(p).map_err(EngineError::JsonError),
        ValueNode::List(items) => value_nodes_to_json(items),
        ValueNode::Map(map) => ordered_map_to_json(map),
    }
}

fn canonicalize_json(value: &serde_json::Value) -> EngineResult<String> {
    Ok(serde_json_canonicalizer::to_string(value)?)
}

impl Default for RDagEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fct_ast::{DirectiveNode, FacetBlock, KeyValueNode, LensCallNode, OrderedMap, Span};
    use fct_std::{Lens, LensContext, LensError, LensSignature};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static BOUNDED_COUNTING_LENS_EXECUTIONS: AtomicUsize = AtomicUsize::new(0);

    struct BoundedNoopLens;

    impl Lens for BoundedNoopLens {
        fn execute(
            &self,
            input: ValueNode,
            _args: Vec<ValueNode>,
            _kwargs: HashMap<String, ValueNode>,
            _ctx: &LensContext,
        ) -> Result<ValueNode, LensError> {
            Ok(input)
        }

        fn signature(&self) -> LensSignature {
            LensSignature {
                name: "bounded_noop".to_string(),
                input_type: "any".to_string(),
                output_type: "any".to_string(),
                trust_level: TrustLevel::Bounded,
                deterministic: true,
            }
        }

        fn effect_class(&self) -> Option<&'static str> {
            Some("external")
        }
    }

    struct BoundedCountingLens;

    impl Lens for BoundedCountingLens {
        fn execute(
            &self,
            input: ValueNode,
            _args: Vec<ValueNode>,
            _kwargs: HashMap<String, ValueNode>,
            _ctx: &LensContext,
        ) -> Result<ValueNode, LensError> {
            BOUNDED_COUNTING_LENS_EXECUTIONS.fetch_add(1, Ordering::SeqCst);
            Ok(input)
        }

        fn signature(&self) -> LensSignature {
            LensSignature {
                name: "bounded_counting".to_string(),
                input_type: "any".to_string(),
                output_type: "any".to_string(),
                trust_level: TrustLevel::Bounded,
                deterministic: true,
            }
        }

        fn effect_class(&self) -> Option<&'static str> {
            Some("external")
        }
    }

    struct BoundedNoEffectLens;

    impl Lens for BoundedNoEffectLens {
        fn execute(
            &self,
            input: ValueNode,
            _args: Vec<ValueNode>,
            _kwargs: HashMap<String, ValueNode>,
            _ctx: &LensContext,
        ) -> Result<ValueNode, LensError> {
            Ok(input)
        }

        fn signature(&self) -> LensSignature {
            LensSignature {
                name: "bounded_no_effect".to_string(),
                input_type: "any".to_string(),
                output_type: "any".to_string(),
                trust_level: TrustLevel::Bounded,
                deterministic: true,
            }
        }
    }

    struct VolatileNoopLens;

    impl Lens for VolatileNoopLens {
        fn execute(
            &self,
            input: ValueNode,
            _args: Vec<ValueNode>,
            _kwargs: HashMap<String, ValueNode>,
            _ctx: &LensContext,
        ) -> Result<ValueNode, LensError> {
            Ok(input)
        }

        fn signature(&self) -> LensSignature {
            LensSignature {
                name: "volatile_noop".to_string(),
                input_type: "any".to_string(),
                output_type: "any".to_string(),
                trust_level: TrustLevel::Volatile,
                deterministic: false,
            }
        }

        fn effect_class(&self) -> Option<&'static str> {
            Some("external")
        }
    }

    struct HighGasLens;

    impl Lens for HighGasLens {
        fn execute(
            &self,
            input: ValueNode,
            _args: Vec<ValueNode>,
            _kwargs: HashMap<String, ValueNode>,
            _ctx: &LensContext,
        ) -> Result<ValueNode, LensError> {
            Ok(input)
        }

        fn signature(&self) -> LensSignature {
            LensSignature {
                name: "high_gas".to_string(),
                input_type: "any".to_string(),
                output_type: "any".to_string(),
                trust_level: TrustLevel::Pure,
                deterministic: true,
            }
        }

        fn gas_cost(
            &self,
            _input: &ValueNode,
            _args: &[ValueNode],
            _kwargs: &HashMap<String, ValueNode>,
        ) -> usize {
            100
        }
    }

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
    fn test_lens_gas_cost_is_enforced() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "x".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "high_gas".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        engine.validate().unwrap();

        let mut ctx = ExecutionContext::new(10);
        ctx.lens_registry.register(Box::new(HighGasLens));

        let err = engine
            .execute(&mut ctx)
            .expect_err("expected gas exhaustion");
        assert!(
            matches!(err, EngineError::GasExhausted { .. }),
            "expected F902 gas exhaustion, got: {err:?}"
        );
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert!(graph.detect_cycles().is_ok());
        assert_eq!(graph.topological_sort().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_pipeline_execution() {
        // Create a simple document with pipeline: "  HELLO  " |> trim() |> lowercase()
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "greeting".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("  HELLO  ".to_string())),
                        lenses: vec![
                            LensCallNode {
                                name: "trim".to_string(),
                                args: vec![],
                                kwargs: OrderedMap::new(),
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
                                kwargs: OrderedMap::new(),
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
    fn test_pipeline_unknown_lens_returns_f802() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "greeting".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "unknown_lens".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        engine.validate().unwrap();

        let mut ctx = ExecutionContext::new(1000);
        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(
            err,
            EngineError::UnknownLens { ref name } if name == "unknown_lens"
        ));
        assert!(err.to_string().contains("F802"));
    }

    #[test]
    fn test_pipeline_with_args() {
        // Create document with pipeline: "a,b,c" |> split(",")
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "items".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("a,b,c".to_string())),
                        lenses: vec![LensCallNode {
                            name: "split".to_string(),
                            args: vec![ValueNode::String(",".to_string())],
                            kwargs: OrderedMap::new(),
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

    #[test]
    fn test_topological_sort_uses_vars_insertion_order_for_independent_nodes() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![
                    BodyNode::KeyValue(KeyValueNode {
                        key: "b".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::String("B".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "a".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::String("A".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "c".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::String("C".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                ],
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

        let mut graph = DependencyGraph::new();
        graph.build_from_document(&doc).unwrap();
        assert_eq!(
            graph.topological_sort().unwrap(),
            vec!["b".to_string(), "a".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_topological_sort_preserves_first_insertion_on_override() {
        let doc = FacetDocument {
            blocks: vec![
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![
                        BodyNode::KeyValue(KeyValueNode {
                            key: "a".to_string(),
                            key_kind: Default::default(),
                            value: ValueNode::String("old".to_string()),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 1,
                                column: 1,
                            },
                        }),
                        BodyNode::KeyValue(KeyValueNode {
                            key: "b".to_string(),
                            key_kind: Default::default(),
                            value: ValueNode::String("B".to_string()),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 1,
                                column: 1,
                            },
                        }),
                    ],
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }),
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![
                        BodyNode::KeyValue(KeyValueNode {
                            key: "a".to_string(),
                            key_kind: Default::default(),
                            value: ValueNode::String("new".to_string()),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 1,
                                column: 1,
                            },
                        }),
                        BodyNode::KeyValue(KeyValueNode {
                            key: "c".to_string(),
                            key_kind: Default::default(),
                            value: ValueNode::String("C".to_string()),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 1,
                                column: 1,
                            },
                        }),
                    ],
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 1,
                        column: 1,
                    },
                }),
            ],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let mut graph = DependencyGraph::new();
        graph.build_from_document(&doc).unwrap();
        assert_eq!(
            graph.topological_sort().unwrap(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn test_missing_dependency_returns_f401() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "x".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Variable("missing.dep".to_string()),
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

        let mut graph = DependencyGraph::new();
        graph.build_from_document(&doc).unwrap();
        let err = graph.topological_sort().unwrap_err();
        assert!(matches!(err, EngineError::VariableNotFound { .. }));
        assert!(err.to_string().contains("F401"));
    }

    #[test]
    fn test_cycle_detection_returns_f505() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![
                    BodyNode::KeyValue(KeyValueNode {
                        key: "a".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Variable("b".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "b".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Variable("a".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 2,
                            column: 1,
                        },
                    }),
                ],
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).expect("build must succeed");
        let err = engine.validate().expect_err("cycle must fail validation");
        assert!(matches!(err, EngineError::CyclicDependency { .. }));
        assert!(err.to_string().contains("F505"));
    }

    #[test]
    fn test_execute_freezes_computed_variable_map() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "x".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::String("hello".to_string()),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).expect("build must succeed");
        engine.validate().expect("validate must succeed");

        let mut ctx = ExecutionContext::new(100);
        engine.execute(&mut ctx).expect("execute must succeed");
        assert!(ctx.is_variables_frozen());

        let err = ctx
            .set_variable("y".to_string(), ValueNode::String("late".to_string()))
            .expect_err("mutating frozen variable map must fail");
        assert!(matches!(err, EngineError::ConstraintViolation { .. }));
        assert!(err.to_string().contains("F452"));
    }

    #[test]
    fn test_execute_does_not_mutate_input_ast() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![
                    BodyNode::KeyValue(KeyValueNode {
                        key: "name".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::String("World".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "greeting".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Pipeline(PipelineNode {
                            initial: Box::new(ValueNode::Variable("name".to_string())),
                            lenses: vec![LensCallNode {
                                name: "uppercase".to_string(),
                                args: vec![],
                                kwargs: OrderedMap::new(),
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
                    }),
                ],
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
        let snapshot = doc.clone();

        let mut engine = RDagEngine::new();
        engine.build(&doc).expect("build must succeed");
        engine.validate().expect("validate must succeed");

        let mut ctx = ExecutionContext::new(100);
        engine.execute(&mut ctx).expect("execute must succeed");

        assert_eq!(doc, snapshot, "Phase 3 must not mutate resolved AST");
    }

    #[test]
    fn test_execute_materializes_effective_policy_and_hash() {
        let mut allow_rule = OrderedMap::new();
        allow_rule.insert("op".to_string(), ValueNode::String("lens_call".to_string()));
        allow_rule.insert("name".to_string(), ValueNode::String("trim".to_string()));

        let doc = FacetDocument {
            blocks: vec![
                FacetNode::Policy(FacetBlock {
                    name: "policy".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "allow".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::List(vec![ValueNode::Map(allow_rule)]),
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
                }),
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "x".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::String("v".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 2,
                            column: 1,
                        },
                    })],
                    span: Span {
                        start: 0,
                        end: 0,
                        line: 2,
                        column: 1,
                    },
                }),
            ],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let mut engine = RDagEngine::new();
        engine.build(&doc).expect("build must succeed");
        engine.validate().expect("validate must succeed");

        let mut ctx = ExecutionContext::new(100);
        engine.execute(&mut ctx).expect("execute must succeed");

        let effective = ctx
            .effective_policy
            .as_ref()
            .expect("effective policy must be materialized");
        let envelope = serde_json::json!({
            "policy_version": POLICY_VERSION,
            "policy": ordered_map_to_json(effective).expect("policy to json"),
        });
        let canonical = canonicalize_json(&envelope).expect("canonical policy envelope");
        let expected_hash = format!("sha256:{:x}", Sha256::digest(canonical.as_bytes()));

        assert_eq!(ctx.policy_hash.as_deref(), Some(expected_hash.as_str()));
    }

    #[test]
    fn test_execute_without_policy_sets_policy_hash_to_none() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "x".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::String("v".to_string()),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).expect("build must succeed");
        engine.validate().expect("validate must succeed");

        let mut ctx = ExecutionContext::new(100);
        engine.execute(&mut ctx).expect("execute must succeed");

        assert!(ctx.effective_policy.is_none());
        assert!(ctx.policy_hash.is_none());
    }

    #[test]
    fn test_variable_path_traversal_success() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![
                    BodyNode::KeyValue(KeyValueNode {
                        key: "user".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Map(OrderedMap::from([(
                            "profile".to_string(),
                            ValueNode::Map(OrderedMap::from([(
                                "name".to_string(),
                                ValueNode::String("Alice".to_string()),
                            )])),
                        )])),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "display".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Variable("user.profile.name".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                ],
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new(1_000);
        engine.execute(&mut ctx).unwrap();
        assert_eq!(
            ctx.get_variable("display"),
            Some(&ValueNode::String("Alice".to_string()))
        );
    }

    #[test]
    fn test_variable_path_missing_field_returns_f405() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![
                    BodyNode::KeyValue(KeyValueNode {
                        key: "user".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Map(OrderedMap::from([(
                            "name".to_string(),
                            ValueNode::String("Alice".to_string()),
                        )])),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "display".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Variable("user.profile.name".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                ],
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new(1_000);
        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::InvalidVariablePath { .. }));
        assert!(err.to_string().contains("F405"));
    }

    #[test]
    fn test_variable_path_numeric_index_returns_f452() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![
                    BodyNode::KeyValue(KeyValueNode {
                        key: "items".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Map(OrderedMap::from([(
                            "0".to_string(),
                            ValueNode::String("x".to_string()),
                        )])),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                    BodyNode::KeyValue(KeyValueNode {
                        key: "first".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Variable("items.0".to_string()),
                        span: Span {
                            start: 0,
                            end: 0,
                            line: 1,
                            column: 1,
                        },
                    }),
                ],
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new(1_000);
        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::ExecutionError { .. }));
        assert!(err.to_string().contains("F452"));
    }

    #[test]
    fn test_input_uses_runtime_value_when_provided() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "query".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Directive(DirectiveNode {
                        name: "input".to_string(),
                        args: OrderedMap::from([(
                            "type".to_string(),
                            ValueNode::String("string".to_string()),
                        )]),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();

        let mut ctx = ExecutionContext::new(1_000);
        ctx.set_input("query".to_string(), ValueNode::String("hello".to_string()));
        engine.execute(&mut ctx).unwrap();

        assert_eq!(
            ctx.get_variable("query"),
            Some(&ValueNode::String("hello".to_string()))
        );
    }

    #[test]
    fn test_input_uses_default_when_not_provided() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "n".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Directive(DirectiveNode {
                        name: "input".to_string(),
                        args: OrderedMap::from([
                            ("type".to_string(), ValueNode::String("int".to_string())),
                            (
                                "default".to_string(),
                                ValueNode::Scalar(ScalarValue::Int(3)),
                            ),
                        ]),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new(1_000);
        engine.execute(&mut ctx).unwrap();

        assert_eq!(
            ctx.get_variable("n"),
            Some(&ValueNode::Scalar(ScalarValue::Int(3)))
        );
    }

    #[test]
    fn test_input_missing_without_default_returns_f453() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "query".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Directive(DirectiveNode {
                        name: "input".to_string(),
                        args: OrderedMap::from([(
                            "type".to_string(),
                            ValueNode::String("string".to_string()),
                        )]),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new(1_000);
        let err = engine.execute(&mut ctx).unwrap_err();

        assert!(matches!(err, EngineError::InputValidationFailed { .. }));
        assert!(err.to_string().contains("F453"));
    }

    #[test]
    fn test_input_type_mismatch_returns_f453() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "n".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Directive(DirectiveNode {
                        name: "input".to_string(),
                        args: OrderedMap::from([(
                            "type".to_string(),
                            ValueNode::String("int".to_string()),
                        )]),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();

        let mut ctx = ExecutionContext::new(1_000);
        ctx.set_input("n".to_string(), ValueNode::String("oops".to_string()));
        let err = engine.execute(&mut ctx).unwrap_err();

        assert!(matches!(err, EngineError::InputValidationFailed { .. }));
        assert!(err.to_string().contains("F453"));
    }

    #[test]
    fn test_input_default_type_mismatch_returns_f453() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "n".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Directive(DirectiveNode {
                        name: "input".to_string(),
                        args: OrderedMap::from([
                            ("type".to_string(), ValueNode::String("int".to_string())),
                            ("default".to_string(), ValueNode::String("oops".to_string())),
                        ]),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();

        let mut ctx = ExecutionContext::new(1_000);
        let err = engine.execute(&mut ctx).unwrap_err();

        assert!(matches!(err, EngineError::InputValidationFailed { .. }));
        assert!(err.to_string().contains("F453"));
    }

    #[test]
    fn test_input_pipeline_base_executes() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "query".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::Directive(DirectiveNode {
                            name: "input".to_string(),
                            args: OrderedMap::from([(
                                "type".to_string(),
                                ValueNode::String("string".to_string()),
                            )]),
                            span: Span {
                                start: 0,
                                end: 0,
                                line: 1,
                                column: 1,
                            },
                        })),
                        lenses: vec![LensCallNode {
                            name: "trim".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();

        let mut ctx = ExecutionContext::new(1_000);
        ctx.set_input(
            "query".to_string(),
            ValueNode::String("  hello  ".to_string()),
        );
        engine.execute(&mut ctx).unwrap();

        assert_eq!(
            ctx.get_variable("query"),
            Some(&ValueNode::String("hello".to_string()))
        );
    }

    #[test]
    fn test_pure_mode_rejects_bounded_lens_with_f803() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Pure);
        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::ExecutionError { .. }));
        assert!(err.to_string().contains("F803"));
    }

    #[test]
    fn test_pure_mode_uses_level1_cache_hit() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "bounded_noop".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Pure);
        ctx.lens_registry.register(Box::new(BoundedNoopLens));

        let cache_key = engine
            .level1_cache_key(
                "bounded_noop",
                "1",
                &ValueNode::String("hello".to_string()),
                &[],
                &OrderedMap::new(),
                "local.default.v1",
            )
            .unwrap();
        ctx.set_lens_cache_entry(cache_key, ValueNode::String("cached-hit".to_string()));

        engine.execute(&mut ctx).unwrap();
        assert_eq!(
            ctx.get_variable("answer"),
            Some(&ValueNode::String("cached-hit".to_string()))
        );
    }

    #[test]
    fn test_level1_cache_key_matches_appendix_c_envelope_and_is_stable() {
        let engine = RDagEngine::new();
        let key1 = engine
            .level1_cache_key(
                "bounded_noop",
                "1",
                &ValueNode::String("hello".to_string()),
                &[],
                &OrderedMap::new(),
                "local.default.v1",
            )
            .unwrap();
        let key2 = engine
            .level1_cache_key(
                "bounded_noop",
                "1",
                &ValueNode::String("hello".to_string()),
                &[],
                &OrderedMap::new(),
                "local.default.v1",
            )
            .unwrap();

        let envelope = serde_json::json!({
            "lens": {
                "name": "bounded_noop",
                "version": "1",
            },
            "input": "hello",
            "args": [],
            "named_args": serde_json::json!({}),
            "host_profile_id": "local.default.v1",
            "facet_version": FACET_VERSION,
        });
        let expected = format!(
            "{:x}",
            Sha256::digest(canonicalize_json(&envelope).unwrap().as_bytes())
        );

        assert_eq!(key1, expected);
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_pure_mode_allows_level0_lens() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("  HELLO ".to_string())),
                        lenses: vec![LensCallNode {
                            name: "trim".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Pure);
        engine.execute(&mut ctx).unwrap();
        assert_eq!(
            ctx.get_variable("answer"),
            Some(&ValueNode::String("HELLO".trim().to_string()))
        );
    }

    #[test]
    fn test_exec_mode_bounded_lens_default_denied_by_guard_with_f454() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "bounded_noop".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedNoopLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::PolicyDenied { .. }));
        assert!(err.to_string().contains("F454"));
        assert_eq!(ctx.guard_decisions.len(), 1);
        assert_eq!(ctx.guard_decisions[0].op, "lens_call");
        assert_eq!(ctx.guard_decisions[0].decision, "denied");
        assert_eq!(ctx.guard_decisions[0].error_code.as_deref(), Some("F454"));
        let expected_input_obj = serde_json::json!({
            "lens": {
                "name": "bounded_noop",
                "version": "1",
            },
            "input": "hello",
            "args": [],
            "named_args": serde_json::json!({}),
            "host_profile_id": "local.default.v1",
            "facet_version": FACET_VERSION,
        });
        let expected_hash = format!(
            "sha256:{:x}",
            Sha256::digest(canonicalize_json(&expected_input_obj).unwrap().as_bytes())
        );
        assert_eq!(ctx.guard_decisions[0].input_hash, expected_hash);
    }

    #[test]
    fn test_exec_mode_guard_denied_bounded_lens_not_executed_before_allow() {
        BOUNDED_COUNTING_LENS_EXECUTIONS.store(0, Ordering::SeqCst);

        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "bounded_counting".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedCountingLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::PolicyDenied { .. }));
        assert_eq!(BOUNDED_COUNTING_LENS_EXECUTIONS.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_pure_mode_rejects_volatile_lens_with_f801() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "volatile_noop".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Pure);
        ctx.lens_registry.register(Box::new(VolatileNoopLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::LensExecutionFailed { .. }));
        assert!(err.to_string().contains("F801"));
    }

    #[test]
    fn test_exec_mode_volatile_lens_default_denied_by_guard_with_f454() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "volatile_noop".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(VolatileNoopLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::PolicyDenied { .. }));
        assert!(err.to_string().contains("F454"));
        assert_eq!(ctx.guard_decisions.len(), 1);
        assert_eq!(ctx.guard_decisions[0].op, "lens_call");
        assert_eq!(ctx.guard_decisions[0].name, "volatile_noop");
        assert_eq!(ctx.guard_decisions[0].decision, "denied");
        assert_eq!(ctx.guard_decisions[0].error_code.as_deref(), Some("F454"));
    }

    #[test]
    fn test_exec_mode_bounded_lens_requires_effect_class_f456() {
        let doc = FacetDocument {
            blocks: vec![FacetNode::Vars(FacetBlock {
                name: "vars".to_string(),
                attributes: OrderedMap::new(),
                body: vec![BodyNode::KeyValue(KeyValueNode {
                    key: "answer".to_string(),
                    key_kind: Default::default(),
                    value: ValueNode::Pipeline(PipelineNode {
                        initial: Box::new(ValueNode::String("hello".to_string())),
                        lenses: vec![LensCallNode {
                            name: "bounded_no_effect".to_string(),
                            args: vec![],
                            kwargs: OrderedMap::new(),
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

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedNoEffectLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::InvalidEffectDeclaration { .. }));
        assert!(err.to_string().contains("F456"));
    }

    #[test]
    fn test_exec_mode_bounded_lens_effect_matcher_allows() {
        let policy_allow_rule = ValueNode::Map(OrderedMap::from([
            (
                "id".to_string(),
                ValueNode::String("allow-bounded".to_string()),
            ),
            ("op".to_string(), ValueNode::String("lens_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("bounded_noop".to_string()),
            ),
            (
                "effect".to_string(),
                ValueNode::String("external".to_string()),
            ),
        ]));

        let doc = FacetDocument {
            blocks: vec![
                FacetNode::Policy(FacetBlock {
                    name: "policy".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "allow".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::List(vec![policy_allow_rule]),
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
                }),
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "answer".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Pipeline(PipelineNode {
                            initial: Box::new(ValueNode::String("hello".to_string())),
                            lenses: vec![LensCallNode {
                                name: "bounded_noop".to_string(),
                                args: vec![],
                                kwargs: OrderedMap::new(),
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
                }),
            ],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedNoopLens));
        engine.execute(&mut ctx).unwrap();

        assert_eq!(
            ctx.get_variable("answer"),
            Some(&ValueNode::String("hello".to_string()))
        );
        let expected_cache_key = engine
            .level1_cache_key(
                "bounded_noop",
                "1",
                &ValueNode::String("hello".to_string()),
                &[],
                &OrderedMap::new(),
                "local.default.v1",
            )
            .unwrap();
        assert_eq!(
            ctx.get_lens_cache_entry(&expected_cache_key),
            Some(&ValueNode::String("hello".to_string()))
        );
        assert_eq!(ctx.guard_decisions.len(), 1);
        assert_eq!(ctx.guard_decisions[0].decision, "allowed");
        assert_eq!(
            ctx.guard_decisions[0].effect_class.as_deref(),
            Some("external")
        );
    }

    #[test]
    fn test_exec_mode_bounded_lens_effect_matcher_mismatch_denies() {
        let policy_allow_rule = ValueNode::Map(OrderedMap::from([
            ("op".to_string(), ValueNode::String("lens_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("bounded_noop".to_string()),
            ),
            ("effect".to_string(), ValueNode::String("read".to_string())),
        ]));

        let doc = FacetDocument {
            blocks: vec![
                FacetNode::Policy(FacetBlock {
                    name: "policy".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "allow".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::List(vec![policy_allow_rule]),
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
                }),
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "answer".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Pipeline(PipelineNode {
                            initial: Box::new(ValueNode::String("hello".to_string())),
                            lenses: vec![LensCallNode {
                                name: "bounded_noop".to_string(),
                                args: vec![],
                                kwargs: OrderedMap::new(),
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
                }),
            ],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedNoopLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::PolicyDenied { .. }));
        assert!(err.to_string().contains("F454"));
    }

    #[test]
    fn test_exec_mode_bounded_lens_allowed_with_policy_short_circuit_any() {
        let policy_allow_rule = ValueNode::Map(OrderedMap::from([
            (
                "id".to_string(),
                ValueNode::String("allow-bounded".to_string()),
            ),
            ("op".to_string(), ValueNode::String("lens_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("bounded_noop".to_string()),
            ),
            (
                "when".to_string(),
                ValueNode::Map(OrderedMap::from([(
                    "any".to_string(),
                    ValueNode::List(vec![
                        ValueNode::Scalar(ScalarValue::Bool(true)),
                        ValueNode::Variable("missing.flag".to_string()),
                    ]),
                )])),
            ),
        ]));

        let doc = FacetDocument {
            blocks: vec![
                FacetNode::Policy(FacetBlock {
                    name: "policy".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "allow".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::List(vec![policy_allow_rule]),
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
                }),
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "answer".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Pipeline(PipelineNode {
                            initial: Box::new(ValueNode::String("hello".to_string())),
                            lenses: vec![LensCallNode {
                                name: "bounded_noop".to_string(),
                                args: vec![],
                                kwargs: OrderedMap::new(),
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
                }),
            ],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedNoopLens));
        engine.execute(&mut ctx).unwrap();

        assert_eq!(
            ctx.get_variable("answer"),
            Some(&ValueNode::String("hello".to_string()))
        );
        assert_eq!(ctx.guard_decisions.len(), 1);
        assert_eq!(ctx.guard_decisions[0].decision, "allowed");
        assert_eq!(
            ctx.guard_decisions[0].policy_rule_id.as_deref(),
            Some("allow-bounded")
        );
    }

    #[test]
    fn test_exec_mode_bounded_lens_policy_undecidable_returns_f455() {
        BOUNDED_COUNTING_LENS_EXECUTIONS.store(0, Ordering::SeqCst);

        let policy_allow_rule = ValueNode::Map(OrderedMap::from([
            (
                "id".to_string(),
                ValueNode::String("allow-bounded".to_string()),
            ),
            ("op".to_string(), ValueNode::String("lens_call".to_string())),
            (
                "name".to_string(),
                ValueNode::String("bounded_counting".to_string()),
            ),
            (
                "when".to_string(),
                ValueNode::Variable("missing.flag".to_string()),
            ),
        ]));

        let doc = FacetDocument {
            blocks: vec![
                FacetNode::Policy(FacetBlock {
                    name: "policy".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "allow".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::List(vec![policy_allow_rule]),
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
                }),
                FacetNode::Vars(FacetBlock {
                    name: "vars".to_string(),
                    attributes: OrderedMap::new(),
                    body: vec![BodyNode::KeyValue(KeyValueNode {
                        key: "answer".to_string(),
                        key_kind: Default::default(),
                        value: ValueNode::Pipeline(PipelineNode {
                            initial: Box::new(ValueNode::String("hello".to_string())),
                            lenses: vec![LensCallNode {
                                name: "bounded_counting".to_string(),
                                args: vec![],
                                kwargs: OrderedMap::new(),
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
                }),
            ],
            span: Span {
                start: 0,
                end: 0,
                line: 1,
                column: 1,
            },
        };

        let mut engine = RDagEngine::new();
        engine.build(&doc).unwrap();
        let mut ctx = ExecutionContext::new_with_mode(1_000, ExecutionMode::Exec);
        ctx.lens_registry.register(Box::new(BoundedCountingLens));

        let err = engine.execute(&mut ctx).unwrap_err();
        assert!(matches!(err, EngineError::GuardUndecidable { .. }));
        assert!(err.to_string().contains("F455"));
        assert_eq!(ctx.guard_decisions.len(), 1);
        assert_eq!(ctx.guard_decisions[0].decision, "denied");
        assert_eq!(ctx.guard_decisions[0].error_code.as_deref(), Some("F455"));
        assert_eq!(BOUNDED_COUNTING_LENS_EXECUTIONS.load(Ordering::SeqCst), 0);
    }
}
