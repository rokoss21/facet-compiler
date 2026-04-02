//! # Inspect Command
//!
//! This module implements the inspect command for the FACET compiler.
//! The inspect command emits structured views (AST, DAG, layout, policy).

use anyhow::{Context, Result};
use console::style;
use fct_ast::{BodyNode, FacetDocument, FacetNode, OrderedMap, ScalarValue, ValueNode};
use fct_engine::{
    count_facet_units_in_value, derive_message_section_id, AllocationResult, ExecutionContext,
    ExecutionMode, RDagEngine, Section, TokenBoxModel,
};
use fct_render::{effective_policy_json_for_document, policy_hash_for_document};
use fct_resolver::{Resolver, ResolverConfig};
use fct_std::LensRegistry;
use fct_validator::TypeChecker;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Serialize)]
struct DagNodeView {
    name: String,
    depends_on: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DagView {
    nodes: Vec<DagNodeView>,
    topological_order: Vec<String>,
}

#[derive(Debug, Serialize)]
struct LayoutSectionView {
    id: String,
    role: String,
    source_index: usize,
    base_size: usize,
    final_size: usize,
    priority: i32,
    min: usize,
    grow: f64,
    shrink: f64,
    critical: bool,
    dropped: bool,
    compressed: bool,
    truncated: bool,
}

#[derive(Debug, Serialize)]
struct LayoutView {
    budget: usize,
    total_size: usize,
    overflow: usize,
    sections: Vec<LayoutSectionView>,
}

/// Inspect command handler
#[allow(clippy::too_many_arguments)]
pub fn execute_inspect(
    input: PathBuf,
    ast_output: Option<PathBuf>,
    dag_output: Option<PathBuf>,
    layout_output: Option<PathBuf>,
    policy_output: Option<PathBuf>,
    budget: usize,
    pure: bool,
    exec: bool,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        warn!("Rate limit exceeded for inspect command");
        eprintln!(
            "{}",
            style("L Rate limit exceeded. Please wait before running another command.").red()
        );
        std::process::exit(1);
    }

    if pure && exec {
        return Err(anyhow::anyhow!("Use only one mode flag: --pure or --exec"));
    }
    let mode = if pure {
        ExecutionMode::Pure
    } else {
        ExecutionMode::Exec
    };
    let mode_label = if pure { "pure" } else { "exec" };

    let source = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read input file: {:?}", input))?;

    let base_dir = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or(std::env::current_dir()?);
    let mut resolver = Resolver::new(ResolverConfig {
        allowed_roots: vec![base_dir.clone()],
        base_dir,
    });
    let phase1 = resolver
        .resolve_phase1(&source)
        .map_err(|e| anyhow::anyhow!("Resolution error: {}", e))?;
    let document_hash = format!(
        "sha256:{:x}",
        Sha256::digest(phase1.resolved_source_form.as_bytes())
    );
    let resolved = phase1.resolved_ast;

    let mut checker = TypeChecker::new();
    checker
        .validate(&resolved)
        .map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;

    let mut engine = RDagEngine::new();
    engine.build(&resolved)?;
    engine.validate()?;

    let mut exec_ctx = ExecutionContext::new_with_mode(10_000, mode);
    engine.execute(&mut exec_ctx)?;

    let dag_view = build_dag_view(&resolved)?;
    let sections = doc_to_sections(&resolved, &exec_ctx.variables)?;
    let allocation = TokenBoxModel::new(budget).allocate(sections, &LensRegistry::new())?;
    let layout_view = build_layout_view(budget, &allocation);

    let ast_view = serde_json::to_value(&resolved)?;
    let policy_view = serde_json::json!({
        "policy_hash": policy_hash_for_document(&resolved)?,
        "effective_policy": effective_policy_json_for_document(&resolved)?,
    });

    let combined = serde_json::json!({
        "metadata": {
            "document_hash": document_hash,
            "mode": mode_label,
            "budget": budget,
        },
        "ast": ast_view,
        "dag": dag_view,
        "layout": layout_view,
        "policy": policy_view,
    });

    let writes_requested = ast_output.is_some()
        || dag_output.is_some()
        || layout_output.is_some()
        || policy_output.is_some();
    if !writes_requested {
        println!("{}", serde_json::to_string_pretty(&combined)?);
        return Ok(());
    }

    if let Some(path) = ast_output {
        write_json_file(&path, &combined["ast"])?;
        println!("wrote ast view: {}", path.display());
    }
    if let Some(path) = dag_output {
        write_json_file(&path, &combined["dag"])?;
        println!("wrote dag view: {}", path.display());
    }
    if let Some(path) = layout_output {
        write_json_file(&path, &combined["layout"])?;
        println!("wrote layout view: {}", path.display());
    }
    if let Some(path) = policy_output {
        write_json_file(&path, &combined["policy"])?;
        println!("wrote policy view: {}", path.display());
    }

    Ok(())
}

fn build_dag_view(doc: &FacetDocument) -> Result<DagView> {
    let vars = collect_merged_vars(doc);

    let mut nodes = Vec::with_capacity(vars.len());
    let mut deps_by_var: OrderedMap<String, Vec<String>> = OrderedMap::new();

    for (name, value) in &vars {
        let deps = dedupe_preserve_order(extract_dependencies(value));
        nodes.push(DagNodeView {
            name: name.clone(),
            depends_on: deps.clone(),
        });
        deps_by_var.insert(name.clone(), deps);
    }

    let topo = deterministic_topological_order(&deps_by_var)?;
    Ok(DagView {
        nodes,
        topological_order: topo,
    })
}

fn collect_merged_vars(doc: &FacetDocument) -> OrderedMap<String, ValueNode> {
    let mut vars = OrderedMap::new();
    for node in &doc.blocks {
        if let FacetNode::Vars(vars_block) = node {
            for body in &vars_block.body {
                if let BodyNode::KeyValue(kv) = body {
                    vars.insert(kv.key.clone(), kv.value.clone());
                }
            }
        }
    }
    vars
}

fn deterministic_topological_order(
    deps_by_var: &OrderedMap<String, Vec<String>>,
) -> Result<Vec<String>> {
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    let mut order_index: HashMap<String, usize> = HashMap::new();

    for (idx, name) in deps_by_var.keys().enumerate() {
        in_degree.insert(name.clone(), 0);
        adj.insert(name.clone(), Vec::new());
        order_index.insert(name.clone(), idx);
    }

    for (name, deps) in deps_by_var {
        for dep in deps {
            if !deps_by_var.contains_key(dep) {
                return Err(anyhow::anyhow!("F401: Variable not found: {}", dep));
            }
            adj.entry(dep.clone()).or_default().push(name.clone());
            *in_degree.entry(name.clone()).or_insert(0) += 1;
        }
    }

    let mut queue: BinaryHeap<Reverse<(usize, String)>> = BinaryHeap::new();
    for name in deps_by_var.keys() {
        if in_degree.get(name).copied().unwrap_or(0) == 0 {
            let idx = *order_index.get(name).expect("order index");
            queue.push(Reverse((idx, name.clone())));
        }
    }

    let mut sorted = Vec::with_capacity(deps_by_var.len());
    while let Some(Reverse((_, name))) = queue.pop() {
        sorted.push(name.clone());
        if let Some(neighbors) = adj.get(&name) {
            for neighbor in neighbors {
                if let Some(deg) = in_degree.get_mut(neighbor) {
                    *deg -= 1;
                    if *deg == 0 {
                        let idx = *order_index.get(neighbor).expect("order index");
                        queue.push(Reverse((idx, neighbor.clone())));
                    }
                }
            }
        }
    }

    if sorted.len() != deps_by_var.len() {
        return Err(anyhow::anyhow!("F505: Cyclic dependency detected in @vars"));
    }

    Ok(sorted)
}

fn extract_dependencies(value: &ValueNode) -> Vec<String> {
    let mut deps = Vec::new();
    match value {
        ValueNode::Variable(var_ref) => deps.push(base_var_name(var_ref).to_string()),
        ValueNode::Pipeline(p) => {
            deps.extend(extract_dependencies(&p.initial));
            for lens in &p.lenses {
                for arg in &lens.args {
                    deps.extend(extract_dependencies(arg));
                }
                for arg in lens.kwargs.values() {
                    deps.extend(extract_dependencies(arg));
                }
            }
        }
        ValueNode::List(items) => {
            for item in items {
                deps.extend(extract_dependencies(item));
            }
        }
        ValueNode::Map(map) => {
            for val in map.values() {
                deps.extend(extract_dependencies(val));
            }
        }
        _ => {}
    }
    deps
}

fn dedupe_preserve_order(items: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for item in items {
        if seen.insert(item.clone()) {
            out.push(item);
        }
    }
    out
}

fn base_var_name(var_ref: &str) -> &str {
    var_ref.split('.').next().unwrap_or(var_ref)
}

fn doc_to_sections(
    doc: &FacetDocument,
    computed_vars: &HashMap<String, ValueNode>,
) -> Result<Vec<Section>> {
    let mut sections = Vec::new();
    let mut system_count = 0usize;
    let mut user_count = 0usize;
    let mut assistant_count = 0usize;

    for node in &doc.blocks {
        let (role, ordinal, block) = match node {
            FacetNode::System(b) => {
                system_count += 1;
                ("system", system_count, b)
            }
            FacetNode::User(b) => {
                user_count += 1;
                ("user", user_count, b)
            }
            FacetNode::Assistant(b) => {
                assistant_count += 1;
                ("assistant", assistant_count, b)
            }
            _ => continue,
        };

        if !should_emit_message_block(block, computed_vars)? {
            continue;
        }

        let mut id = derive_message_section_id(role, ordinal);
        let mut priority = 500;
        let mut min = 0usize;
        let mut grow = 0.0f64;
        let mut shrink = 0.0f64;
        let mut strategy = None;
        let mut content = None;

        for body in &block.body {
            let BodyNode::KeyValue(kv) = body else {
                continue;
            };
            match kv.key.as_str() {
                "content" => content = Some(resolve_message_value(&kv.value, computed_vars)?),
                "id" => {
                    if let ValueNode::String(v) = &kv.value {
                        id = v.clone();
                    }
                }
                "priority" => {
                    if let Some(v) = as_i32(&kv.value) {
                        priority = v;
                    }
                }
                "min" => {
                    if let Some(v) = as_usize(&kv.value) {
                        min = v;
                    }
                }
                "grow" => {
                    if let Some(v) = as_f64(&kv.value) {
                        grow = v;
                    }
                }
                "shrink" => {
                    if let Some(v) = as_f64(&kv.value) {
                        shrink = v;
                    }
                }
                "strategy" => {
                    if let ValueNode::Pipeline(p) = &kv.value {
                        strategy = Some(p.clone());
                    }
                }
                _ => {}
            }
        }

        let content = content.unwrap_or_else(|| ValueNode::String(String::new()));
        let base_size = count_facet_units_in_value(&content);
        let mut section = Section::new(id, content, base_size)
            .with_priority(priority)
            .with_limits(min, grow, shrink);
        if let Some(strategy_pipeline) = strategy {
            section = section.with_strategy(strategy_pipeline);
        }
        sections.push(section);
    }

    Ok(sections)
}

fn as_i32(v: &ValueNode) -> Option<i32> {
    match v {
        ValueNode::Scalar(ScalarValue::Int(i)) => i32::try_from(*i).ok(),
        _ => None,
    }
}

fn as_usize(v: &ValueNode) -> Option<usize> {
    match v {
        ValueNode::Scalar(ScalarValue::Int(i)) if *i >= 0 => usize::try_from(*i).ok(),
        _ => None,
    }
}

fn as_f64(v: &ValueNode) -> Option<f64> {
    match v {
        ValueNode::Scalar(ScalarValue::Int(i)) => Some(*i as f64),
        ValueNode::Scalar(ScalarValue::Float(f)) => Some(*f),
        _ => None,
    }
}

fn should_emit_message_block(
    block: &fct_ast::FacetBlock,
    computed_vars: &HashMap<String, ValueNode>,
) -> Result<bool> {
    let Some(when_value) = block.attributes.get("when") else {
        return Ok(true);
    };

    match when_value {
        ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(*v),
        ValueNode::Variable(var_ref) => match resolve_variable_ref(var_ref, computed_vars)? {
            ValueNode::Scalar(ScalarValue::Bool(v)) => Ok(v),
            _ => Err(anyhow::anyhow!(
                "F451: Type mismatch: 'when' must evaluate to bool"
            )),
        },
        _ => Err(anyhow::anyhow!(
            "F451: Type mismatch: 'when' must be bool or variable reference"
        )),
    }
}

fn resolve_message_value(
    value: &ValueNode,
    computed_vars: &HashMap<String, ValueNode>,
) -> Result<ValueNode> {
    match value {
        ValueNode::Variable(var_ref) => resolve_variable_ref(var_ref, computed_vars),
        ValueNode::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_message_value(item, computed_vars)?);
            }
            Ok(ValueNode::List(out))
        }
        ValueNode::Map(map) => {
            let mut out = OrderedMap::new();
            for (k, v) in map {
                out.insert(k.clone(), resolve_message_value(v, computed_vars)?);
            }
            Ok(ValueNode::Map(out))
        }
        ValueNode::Pipeline(_) => Err(anyhow::anyhow!(
            "Unresolved pipeline in message content; expected computed value"
        )),
        ValueNode::Directive(_) => Err(anyhow::anyhow!(
            "Unresolved directive in message content; expected computed value"
        )),
        _ => Ok(value.clone()),
    }
}

fn resolve_variable_ref(
    var_ref: &str,
    computed_vars: &HashMap<String, ValueNode>,
) -> Result<ValueNode> {
    let mut parts = var_ref.split('.');
    let base = parts.next().unwrap_or(var_ref);
    let mut current = computed_vars
        .get(base)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("F401: Variable not found: {}", base))?;

    for segment in parts {
        if segment.chars().all(|c| c.is_ascii_digit()) {
            return Err(anyhow::anyhow!(
                "F452: Numeric indexing is not standardized in v2.1.3: {}",
                var_ref
            ));
        }

        current = match current {
            ValueNode::Map(map) => map
                .get(segment)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("F405: Invalid variable path: {}", var_ref))?,
            _ => return Err(anyhow::anyhow!("F405: Invalid variable path: {}", var_ref)),
        };
    }

    Ok(current)
}

fn build_layout_view(budget: usize, allocation: &AllocationResult) -> LayoutView {
    let sections = allocation
        .sections
        .iter()
        .map(|item| LayoutSectionView {
            id: item.section.id.clone(),
            role: section_role(&item.section.id).to_string(),
            source_index: item.section.source_index,
            base_size: item.section.base_size,
            final_size: item.final_size,
            priority: item.section.priority,
            min: item.section.min,
            grow: item.section.grow,
            shrink: item.section.shrink,
            critical: item.section.is_critical,
            dropped: item.was_dropped,
            compressed: item.was_compressed,
            truncated: item.was_truncated,
        })
        .collect();

    LayoutView {
        budget,
        total_size: allocation.total_size,
        overflow: allocation.overflow,
        sections,
    }
}

fn section_role(id: &str) -> &str {
    if id.starts_with("system") {
        "system"
    } else if id.starts_with("user") {
        "user"
    } else if id.starts_with("assistant") {
        "assistant"
    } else {
        "unknown"
    }
}

fn write_json_file(path: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory for {:?}", path))?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).with_context(|| format!("Failed to write {:?}", path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use governor::{Quota, RateLimiter};
    use nonzero_ext::nonzero;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn dag_view_preserves_first_insertion_order_under_overrides() {
        let source = r#"
@vars
  a: "x"
  b: "y"

@vars
  b: "z"
  c: $a
"#;
        let doc = fct_parser::parse_document(source).expect("parse");
        let view = build_dag_view(&doc).expect("dag view");

        let node_names: Vec<&str> = view.nodes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(node_names, vec!["a", "b", "c"]);
        assert_eq!(view.topological_order, vec!["a", "b", "c"]);
    }

    #[test]
    fn execute_inspect_writes_requested_views() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-inspect-{}", nonce));
        fs::create_dir_all(&test_dir).expect("create temp dir");

        let input_path = test_dir.join("input.facet");
        let ast_path = test_dir.join("out").join("ast.json");
        let dag_path = test_dir.join("out").join("dag.json");
        let layout_path = test_dir.join("out").join("layout.json");
        let policy_path = test_dir.join("out").join("policy.json");

        let source = r#"
@vars
  show: true
  name: "World"

@system
  content: "System"

@user(when=$show)
  content: $name

@policy
  allow: [{ op: "message_emit", name: "user#1" }]
"#;
        fs::write(&input_path, source).expect("write source");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        execute_inspect(
            input_path,
            Some(ast_path.clone()),
            Some(dag_path.clone()),
            Some(layout_path.clone()),
            Some(policy_path.clone()),
            512,
            false,
            true,
            &limiter,
        )
        .expect("inspect should succeed");

        let ast_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&ast_path).expect("read ast"))
                .expect("ast json");
        let dag_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&dag_path).expect("read dag"))
                .expect("dag json");
        let layout_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&layout_path).expect("read layout"))
                .expect("layout json");
        let policy_json: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&policy_path).expect("read policy"))
                .expect("policy json");

        assert!(ast_json.get("blocks").is_some(), "ast view missing blocks");
        assert!(
            dag_json.get("topological_order").is_some(),
            "dag view missing topological_order"
        );
        assert!(
            layout_json.get("sections").is_some(),
            "layout view missing sections"
        );
        assert!(
            policy_json.get("policy_hash").is_some(),
            "policy view missing policy_hash"
        );
        assert!(
            policy_json.get("effective_policy").is_some(),
            "policy view missing effective_policy"
        );

        let _ = fs::remove_dir_all(&test_dir);
    }
}
