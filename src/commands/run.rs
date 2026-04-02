//! # Run Command
//!
//! This module implements the run command for the FACET compiler.
//! The run command executes the full pipeline: parse, resolve, validate, compute, and render.

use anyhow::{Context, Result};
use console::style;
use fct_ast::{BodyNode, FacetDocument, FacetNode, OrderedMap, PipelineNode, ScalarValue, ValueNode};
use fct_engine::{
    count_facet_units_in_value, derive_message_section_id, ExecutionContext,
    ExecutionGuardDecision, ExecutionMode, RDagEngine, Section, TokenBoxModel,
};
use fct_parser::parse_document;
use fct_render::{
    to_json_compact, to_json_string, CanonicalPayload, GuardDecision, RenderContext, Renderer,
};
use fct_resolver::{Resolver, ResolverConfig};
use fct_std::{LensContext, LensRegistry, TrustLevel};
use fct_validator::TypeChecker;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use tracing::info;

/// Run command handler
pub fn execute_run(
    input: std::path::PathBuf,
    runtime_input: Option<std::path::PathBuf>,
    budget: usize,
    context_budget: usize,
    format: String,
    pure: bool,
    exec: bool,
    _no_progress: bool,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        eprintln!(
            "{}",
            style("L Rate limit exceeded. Please wait before running another command.").red()
        );
        std::process::exit(1);
    }

    info!("Starting full pipeline for file: {:?}", input);
    info!("Budget: {}, Context budget: {}", budget, context_budget);

    if pure && exec {
        return Err(anyhow::anyhow!("Use only one mode flag: --pure or --exec"));
    }
    let mode = if pure { "pure" } else { "exec" };

    let source = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read input file: {:?}", input))?;
    let parsed = parse_document(&source).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

    let base_dir = input
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or(std::env::current_dir()?);
    let mut resolver = Resolver::new(ResolverConfig {
        allowed_roots: vec![base_dir.clone()],
        base_dir,
    });
    let resolved_source_form = resolver
        .resolve_source_form(&source)
        .map_err(|e| anyhow::anyhow!("Resolution error: {}", e))?;
    let resolved = resolver
        .resolve(parsed)
        .map_err(|e| anyhow::anyhow!("Resolution error: {}", e))?;
    let document_hash = sha256_prefixed(resolved_source_form.as_bytes());

    let mut checker = TypeChecker::new();
    checker
        .validate(&resolved)
        .map_err(|e| anyhow::anyhow!("Validation error: {}", e))?;

    let mut engine = RDagEngine::new();
    engine.build(&resolved)?;
    engine.validate()?;
    let execution_mode = if pure {
        ExecutionMode::Pure
    } else {
        ExecutionMode::Exec
    };
    let mut exec_ctx = ExecutionContext::new_with_mode(context_budget, execution_mode);
    if let Some(runtime_input_path) = runtime_input {
        let runtime_inputs = load_runtime_inputs(&runtime_input_path)?;
        exec_ctx.set_inputs(runtime_inputs);
    }
    engine.execute(&mut exec_ctx)?;

    let effective_budget = effective_layout_budget(&resolved, budget);
    let lens_registry = LensRegistry::new();
    let sections = doc_to_sections(&resolved, &exec_ctx.variables, &lens_registry, execution_mode)?;
    let box_model = TokenBoxModel::new(effective_budget);
    let allocation = box_model.allocate_with_mode(sections, &lens_registry, execution_mode)?;

    let renderer = Renderer::new();
    let render_output = renderer.render_with_trace(
        &resolved,
        &allocation,
        RenderContext {
            document_hash: Some(document_hash),
            policy_hash: exec_ctx.policy_hash.clone(),
            profile: Some("hypervisor".to_string()),
            mode: Some(mode.to_string()),
            host_profile_id: Some("local.default.v1".to_string()),
            budget_units: Some(effective_budget),
            target_provider_id: Some("unknown-provider".to_string()),
            computed_vars: Some(exec_ctx.variables.clone()),
        },
    )?;
    let payload = render_output.payload;

    let guard_decisions =
        merge_guard_decisions(&exec_ctx.guard_decisions, &render_output.guard_decisions);
    let execution_artifact = build_execution_artifact(&payload, &guard_decisions)?;
    let execution_json = canonicalize_json(&execution_artifact)?;
    let execution_path = input
        .parent()
        .map(|p| p.join("execution.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("execution.json"));
    fs::write(&execution_path, execution_json)
        .with_context(|| format!("Failed to write execution artifact: {:?}", execution_path))?;

    match format.as_str() {
        "json" => println!("{}", to_json_compact(&payload)?),
        "pretty" => println!("{}", to_json_string(&payload)?),
        other => {
            return Err(anyhow::anyhow!(
                "Unsupported format '{}'. Use 'json' or 'pretty'",
                other
            ));
        }
    }

    Ok(())
}

fn effective_layout_budget(doc: &FacetDocument, host_budget: usize) -> usize {
    context_budget_from_doc(doc).unwrap_or(host_budget)
}

fn context_budget_from_doc(doc: &FacetDocument) -> Option<usize> {
    for block in &doc.blocks {
        if let FacetNode::Context(ctx) = block {
            for body in &ctx.body {
                if let BodyNode::KeyValue(kv) = body {
                    if kv.key == "budget" {
                        if let ValueNode::Scalar(ScalarValue::Int(v)) = kv.value {
                            if v >= 0 {
                                return Some(v as usize);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn load_runtime_inputs(path: &std::path::Path) -> Result<HashMap<String, ValueNode>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("Failed to read runtime input file: {:?}", path))?;
    let json: serde_json::Value = serde_json::from_str(&raw)
        .with_context(|| format!("Runtime input file is not valid JSON: {:?}", path))?;

    let obj = json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Runtime input JSON root must be an object: {:?}", path))?;

    let mut out = HashMap::new();
    for (k, v) in obj {
        out.insert(k.clone(), json_to_value_node(v)?);
    }
    Ok(out)
}

fn json_to_value_node(value: &serde_json::Value) -> Result<ValueNode> {
    Ok(match value {
        serde_json::Value::Null => ValueNode::Scalar(ScalarValue::Null),
        serde_json::Value::Bool(b) => ValueNode::Scalar(ScalarValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ValueNode::Scalar(ScalarValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                ValueNode::Scalar(ScalarValue::Float(f))
            } else {
                return Err(anyhow::anyhow!("Unsupported JSON number: {}", n));
            }
        }
        serde_json::Value::String(s) => ValueNode::String(s.clone()),
        serde_json::Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(json_to_value_node(item)?);
            }
            ValueNode::List(out)
        }
        serde_json::Value::Object(map) => {
            let mut out = OrderedMap::new();
            for (k, v) in map {
                out.insert(k.clone(), json_to_value_node(v)?);
            }
            ValueNode::Map(out)
        }
    })
}

fn doc_to_sections(
    doc: &FacetDocument,
    computed_vars: &HashMap<String, ValueNode>,
    lens_registry: &LensRegistry,
    mode: ExecutionMode,
) -> Result<Vec<Section>> {
    let mut sections = Vec::new();
    let defaults = context_layout_defaults_from_doc(doc);
    let mut system_count = 0usize;
    let mut user_count = 0usize;
    let mut assistant_count = 0usize;

    for block in &doc.blocks {
        let (derived_id, body) = match block {
            FacetNode::System(b) => {
                system_count += 1;
                (derive_message_section_id("system", system_count), b)
            }
            FacetNode::User(b) => {
                user_count += 1;
                (derive_message_section_id("user", user_count), b)
            }
            FacetNode::Assistant(b) => {
                assistant_count += 1;
                (derive_message_section_id("assistant", assistant_count), b)
            }
            _ => continue,
        };

        if !should_emit_message_block(body, computed_vars)? {
            continue;
        }

        let layout = resolve_section_layout(body, &defaults, &derived_id);
        let content = block_content_or_default(body, computed_vars, lens_registry, mode)?;
        let base_size = count_facet_units_in_value(&content);
        let mut section = Section::new(layout.id, content, base_size)
            .with_priority(layout.priority)
            .with_limits(layout.min, layout.grow, layout.shrink);
        if let Some(strategy) = layout.strategy {
            section = section.with_strategy(strategy);
        }
        sections.push(section);
    }

    Ok(sections)
}

#[derive(Debug, Clone, Copy)]
struct LayoutDefaults {
    priority: i32,
    min: usize,
    grow: f64,
    shrink: f64,
}

#[derive(Debug, Clone)]
struct SectionLayout {
    id: String,
    priority: i32,
    min: usize,
    grow: f64,
    shrink: f64,
    strategy: Option<PipelineNode>,
}

fn context_layout_defaults_from_doc(doc: &FacetDocument) -> LayoutDefaults {
    let mut out = LayoutDefaults {
        priority: 500,
        min: 0,
        grow: 0.0,
        shrink: 0.0,
    };

    for block in &doc.blocks {
        let FacetNode::Context(ctx) = block else {
            continue;
        };
        for body in &ctx.body {
            let BodyNode::KeyValue(kv) = body else {
                continue;
            };
            if kv.key != "defaults" {
                continue;
            }
            let ValueNode::Map(defaults) = &kv.value else {
                continue;
            };
            if let Some(v) = defaults.get("priority").and_then(value_as_i32) {
                out.priority = v;
            }
            if let Some(v) = defaults.get("min").and_then(value_as_usize) {
                out.min = v;
            }
            if let Some(v) = defaults.get("grow").and_then(value_as_f64) {
                out.grow = v;
            }
            if let Some(v) = defaults.get("shrink").and_then(value_as_f64) {
                out.shrink = v;
            }
        }
    }

    out
}

fn resolve_section_layout(
    block: &fct_ast::FacetBlock,
    defaults: &LayoutDefaults,
    derived_id: &str,
) -> SectionLayout {
    let mut layout = SectionLayout {
        id: derived_id.to_string(),
        priority: defaults.priority,
        min: defaults.min,
        grow: defaults.grow,
        shrink: defaults.shrink,
        strategy: None,
    };

    for body in &block.body {
        let BodyNode::KeyValue(kv) = body else {
            continue;
        };
        match kv.key.as_str() {
            "id" => {
                if let ValueNode::String(v) = &kv.value {
                    layout.id = v.clone();
                }
            }
            "priority" => {
                if let Some(v) = value_as_i32(&kv.value) {
                    layout.priority = v;
                }
            }
            "min" => {
                if let Some(v) = value_as_usize(&kv.value) {
                    layout.min = v;
                }
            }
            "grow" => {
                if let Some(v) = value_as_f64(&kv.value) {
                    layout.grow = v;
                }
            }
            "shrink" => {
                if let Some(v) = value_as_f64(&kv.value) {
                    layout.shrink = v;
                }
            }
            "strategy" => {
                if let ValueNode::Pipeline(p) = &kv.value {
                    layout.strategy = Some(p.clone());
                }
            }
            _ => {}
        }
    }

    layout
}

fn value_as_i32(value: &ValueNode) -> Option<i32> {
    match value {
        ValueNode::Scalar(ScalarValue::Int(v)) => Some((*v).clamp(0, i32::MAX as i64) as i32),
        _ => None,
    }
}

fn value_as_usize(value: &ValueNode) -> Option<usize> {
    match value {
        ValueNode::Scalar(ScalarValue::Int(v)) if *v >= 0 => Some(*v as usize),
        _ => None,
    }
}

fn value_as_f64(value: &ValueNode) -> Option<f64> {
    match value {
        ValueNode::Scalar(ScalarValue::Int(v)) if *v >= 0 => Some(*v as f64),
        ValueNode::Scalar(ScalarValue::Float(v)) if *v >= 0.0 => Some(*v),
        _ => None,
    }
}

fn should_emit_message_block(
    block: &fct_ast::FacetBlock,
    computed_vars: &HashMap<String, ValueNode>,
) -> Result<bool> {
    let attr_when = match block.attributes.get("when") {
        Some(v) => eval_when_atom(v, computed_vars)?,
        None => true,
    };

    let mut body_when = true;
    for body in &block.body {
        let BodyNode::KeyValue(kv) = body else {
            continue;
        };
        if kv.key == "when" {
            body_when = eval_when_atom(&kv.value, computed_vars)?;
        }
    }

    Ok(attr_when && body_when)
}

fn eval_when_atom(
    when_value: &ValueNode,
    computed_vars: &HashMap<String, ValueNode>,
) -> Result<bool> {
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

fn block_content_or_default(
    block: &fct_ast::FacetBlock,
    computed_vars: &HashMap<String, ValueNode>,
    lens_registry: &LensRegistry,
    mode: ExecutionMode,
) -> Result<ValueNode> {
    for body in &block.body {
        if let BodyNode::KeyValue(kv) = body {
            if kv.key == "content" {
                return resolve_message_value(&kv.value, computed_vars, lens_registry, mode);
            }
        }
    }
    Ok(ValueNode::String(format!("{} block", block.name)))
}

fn resolve_message_value(
    value: &ValueNode,
    computed_vars: &HashMap<String, ValueNode>,
    lens_registry: &LensRegistry,
    mode: ExecutionMode,
) -> Result<ValueNode> {
    match value {
        ValueNode::Variable(var_ref) => resolve_variable_ref(var_ref, computed_vars),
        ValueNode::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(resolve_message_value(item, computed_vars, lens_registry, mode)?);
            }
            Ok(ValueNode::List(out))
        }
        ValueNode::Map(map) => {
            let mut out = OrderedMap::new();
            for (k, v) in map {
                out.insert(
                    k.clone(),
                    resolve_message_value(v, computed_vars, lens_registry, mode)?,
                );
            }
            Ok(ValueNode::Map(out))
        }
        ValueNode::Pipeline(pipeline) => {
            let mut current =
                resolve_message_value(&pipeline.initial, computed_vars, lens_registry, mode)?;
            let ctx = LensContext {
                variables: computed_vars.clone(),
            };

            for lens_call in &pipeline.lenses {
                let lens = lens_registry.get(&lens_call.name).ok_or_else(|| {
                    anyhow::anyhow!("F802: Unknown lens in message content: {}", lens_call.name)
                })?;
                let signature = lens.signature();
                if signature.trust_level != TrustLevel::Pure {
                    return Err(anyhow::anyhow!(
                        "F801: Message content lens '{}' must be Level-0 (pure)",
                        lens_call.name
                    ));
                }

                let mut resolved_args = Vec::with_capacity(lens_call.args.len());
                for arg in &lens_call.args {
                    resolved_args.push(resolve_message_value(
                        arg,
                        computed_vars,
                        lens_registry,
                        mode,
                    )?);
                }
                let mut resolved_kwargs = HashMap::with_capacity(lens_call.kwargs.len());
                for (k, v) in &lens_call.kwargs {
                    resolved_kwargs.insert(
                        k.clone(),
                        resolve_message_value(v, computed_vars, lens_registry, mode)?,
                    );
                }

                current = lens
                    .execute(current, resolved_args, resolved_kwargs, &ctx)
                    .map_err(|e| anyhow::anyhow!("F801: Message content lens execution failed: {}", e))?;
            }
            Ok(current)
        }
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

fn merge_guard_decisions(
    engine_decisions: &[ExecutionGuardDecision],
    render_decisions: &[GuardDecision],
) -> Vec<GuardDecision> {
    let mut merged = Vec::with_capacity(engine_decisions.len() + render_decisions.len());

    for decision in engine_decisions {
        merged.push(GuardDecision {
            seq: 0,
            op: decision.op.clone(),
            name: decision.name.clone(),
            effect_class: decision.effect_class.clone(),
            mode: decision.mode.clone(),
            decision: decision.decision.clone(),
            policy_rule_id: decision.policy_rule_id.clone(),
            input_hash: decision.input_hash.clone(),
            error_code: decision.error_code.clone(),
        });
    }
    merged.extend(render_decisions.iter().cloned());

    for (idx, decision) in merged.iter_mut().enumerate() {
        decision.seq = idx + 1;
    }
    merged
}

fn build_execution_artifact(
    payload: &CanonicalPayload,
    decisions: &[GuardDecision],
) -> Result<serde_json::Value> {
    build_execution_artifact_with_attestation(payload, decisions, None)
}

fn build_execution_artifact_with_attestation(
    payload: &CanonicalPayload,
    decisions: &[GuardDecision],
    attestation: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    let metadata = serde_json::json!({
        "facet_version": payload.metadata.facet_version,
        "host_profile_id": payload.metadata.host_profile_id,
        "document_hash": payload.metadata.document_hash,
        "policy_hash": payload.metadata.policy_hash,
        "policy_version": payload.metadata.policy_version,
    });

    let normalized = normalize_guard_decisions(decisions);
    let events: Vec<serde_json::Value> = normalized
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()?;

    let h0_input = serde_json::json!({
        "facet_version": payload.metadata.facet_version,
        "host_profile_id": payload.metadata.host_profile_id,
        "document_hash": payload.metadata.document_hash,
        "policy_hash": payload.metadata.policy_hash,
        "policy_version": payload.metadata.policy_version,
        "profile": payload.metadata.profile,
        "mode": payload.metadata.mode,
    });

    let mut prev = sha256_prefixed(canonicalize_json(&h0_input)?.as_bytes());
    for event in &events {
        let chain_input = serde_json::json!({
            "prev": prev,
            "event": event
        });
        prev = sha256_prefixed(canonicalize_json(&chain_input)?.as_bytes());
    }

    let attestation_value = match attestation {
        Some(value) => validate_attestation_envelope(value)?,
        None => serde_json::Value::Null,
    };

    Ok(serde_json::json!({
        "metadata": metadata,
        "provenance": {
            "events": events,
            "hash_chain": {
                "algo": "sha256",
                "head": prev
            }
        },
        "attestation": attestation_value
    }))
}

fn normalize_guard_decisions(decisions: &[GuardDecision]) -> Vec<GuardDecision> {
    decisions
        .iter()
        .enumerate()
        .map(|(idx, d)| {
            let mut out = d.clone();
            out.seq = idx + 1;
            out
        })
        .collect()
}

fn sha256_prefixed(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn canonicalize_json(value: &serde_json::Value) -> Result<String> {
    Ok(serde_json_canonicalizer::to_string(value)?)
}

fn validate_attestation_envelope(attestation: serde_json::Value) -> Result<serde_json::Value> {
    let obj = attestation
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Attestation must be an object"))?;

    if obj.len() != 3 {
        return Err(anyhow::anyhow!(
            "Attestation must contain exactly: algo, key_id, sig"
        ));
    }

    let algo = obj
        .get("algo")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Attestation.algo must be a string"))?;
    let key_id = obj
        .get("key_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Attestation.key_id must be a string"))?;
    let sig = obj
        .get("sig")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Attestation.sig must be a string"))?;

    let namespaced_algo = algo.starts_with("x.")
        && algo.split('.').count() >= 3
        && !algo.split('.').any(|seg| seg.is_empty());
    if algo != "ed25519" && !namespaced_algo {
        return Err(anyhow::anyhow!(
            "Attestation.algo must be 'ed25519' or namespaced 'x.<host>.<algo>'"
        ));
    }
    if key_id.trim().is_empty() {
        return Err(anyhow::anyhow!("Attestation.key_id must be non-empty"));
    }
    if sig.is_empty()
        || !sig
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow::anyhow!(
            "Attestation.sig must be non-empty base64url (unpadded)"
        ));
    }

    Ok(serde_json::json!({
        "algo": algo,
        "key_id": key_id,
        "sig": sig,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use fct_ast::{LensCallNode, PipelineNode, Span};
    use fct_render::Metadata;
    use governor::{Quota, RateLimiter};
    use nonzero_ext::nonzero;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_payload() -> CanonicalPayload {
        CanonicalPayload {
            metadata: Metadata {
                facet_version: "2.1.3".to_string(),
                profile: "hypervisor".to_string(),
                mode: "exec".to_string(),
                host_profile_id: "local.default.v1".to_string(),
                policy_version: "1".to_string(),
                document_hash: "sha256:abc".to_string(),
                policy_hash: Some("sha256:def".to_string()),
                budget_units: 32000,
                target_provider_id: "generic-llm".to_string(),
            },
            tools: Vec::new(),
            messages: Vec::new(),
        }
    }

    #[test]
    fn resolve_message_value_evaluates_pipeline_from_computed_vars() {
        let value = ValueNode::Pipeline(PipelineNode {
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
        });
        let computed_vars = HashMap::from([(
            "name".to_string(),
            ValueNode::String("world".to_string()),
        )]);
        let lens_registry = LensRegistry::new();

        let resolved =
            resolve_message_value(&value, &computed_vars, &lens_registry, ExecutionMode::Exec)
                .expect("message pipeline should resolve");
        assert_eq!(resolved, ValueNode::String("WORLD".to_string()));
    }

    #[test]
    fn resolve_message_value_rejects_non_pure_lens_in_message_content() {
        let value = ValueNode::Pipeline(PipelineNode {
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
        });
        let computed_vars = HashMap::new();
        let lens_registry = LensRegistry::new();

        let err = resolve_message_value(&value, &computed_vars, &lens_registry, ExecutionMode::Exec)
            .expect_err("non-pure message lens must be rejected");
        let text = err.to_string();
        assert!(text.contains("F801"));
        assert!(text.contains("Level-0 (pure)"));
    }

    #[test]
    fn effective_layout_budget_prefers_context_budget() {
        let source = r#"
@context
  budget: 123

@user
  content: "hello"
"#;
        let doc = parse_document(source).expect("doc should parse");
        assert_eq!(effective_layout_budget(&doc, 4096), 123);
    }

    #[test]
    fn effective_layout_budget_uses_host_budget_when_context_absent() {
        let source = r#"
@user
  content: "hello"
"#;
        let doc = parse_document(source).expect("doc should parse");
        assert_eq!(effective_layout_budget(&doc, 4096), 4096);
    }

    #[test]
    fn doc_to_sections_applies_context_defaults_and_message_overrides() {
        let source = r#"
@context
  budget: 500
  defaults: { priority: 610, min: 3, grow: 0.7, shrink: 0.4 }

@user
  content: "first"

@user
  id: "u.custom"
  priority: 10
  min: 1
  grow: 2
  shrink: 0
  strategy: " text " |> trim()
  content: "second"
"#;
        let doc = parse_document(source).expect("doc should parse");
        let sections = doc_to_sections(
            &doc,
            &HashMap::new(),
            &LensRegistry::new(),
            ExecutionMode::Exec,
        )
        .expect("sections should build");

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].id, "user#1");
        assert_eq!(sections[0].priority, 610);
        assert_eq!(sections[0].min, 3);
        assert_eq!(sections[0].grow, 0.7);
        assert_eq!(sections[0].shrink, 0.4);

        assert_eq!(sections[1].id, "u.custom");
        assert_eq!(sections[1].priority, 10);
        assert_eq!(sections[1].min, 1);
        assert_eq!(sections[1].grow, 2.0);
        assert_eq!(sections[1].shrink, 0.0);
        assert!(sections[1].strategy.is_some());
        assert!(sections[1].is_critical);
    }

    #[test]
    fn doc_to_sections_applies_body_when_gating() {
        let source = r#"
@user
  when: false
  content: "hidden"

@user
  content: "visible"
"#;
        let doc = parse_document(source).expect("doc should parse");
        let sections = doc_to_sections(
            &doc,
            &HashMap::new(),
            &LensRegistry::new(),
            ExecutionMode::Exec,
        )
        .expect("sections should build");

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id, "user#2");
    }

    #[test]
    fn execute_run_writes_execution_json_artifact() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-run-artifact-{}", nonce));
        std::fs::create_dir_all(&test_dir).expect("create temp dir");

        let input_path = test_dir.join("input.facet");
        let source = r#"
@system
  content: "You are a helpful assistant."

@user
  content: "Hello"
"#;
        std::fs::write(&input_path, source).expect("write facet file");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        execute_run(
            input_path.clone(),
            None,
            1024,
            2048,
            "json".to_string(),
            false,
            true,
            true,
            &limiter,
        )
        .expect("run should succeed");

        let artifact_path = test_dir.join("execution.json");
        assert!(artifact_path.exists(), "execution.json must be written");

        let artifact = std::fs::read_to_string(&artifact_path).expect("read execution.json");
        let parsed: serde_json::Value = serde_json::from_str(&artifact).expect("valid JSON");
        assert!(parsed.get("metadata").is_some(), "metadata section missing");
        assert!(parsed.get("provenance").is_some(), "provenance section missing");
        assert!(
            parsed
                .get("provenance")
                .and_then(|v| v.get("hash_chain"))
                .is_some(),
            "hash_chain missing"
        );

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn execute_run_fails_closed_with_f455_for_undecidable_lens_guard() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-run-f455-{}", nonce));
        std::fs::create_dir_all(&test_dir).expect("create temp dir");

        let input_path = test_dir.join("input.facet");
        let source = r#"
@vars
  gate: @input(type="any", default="not-bool")
  out: "hello" |> llm_call()

@policy
  allow: [{ op: "lens_call", name: "llm_call", when: $gate }]
"#;
        std::fs::write(&input_path, source).expect("write facet file");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        let err = execute_run(
            input_path.clone(),
            None,
            1024,
            2048,
            "json".to_string(),
            false,
            true,
            true,
            &limiter,
        )
        .expect_err("undecidable guard must fail closed");
        let text = err.to_string();
        assert!(text.contains("F455"), "expected F455, got: {text}");

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn execute_run_rejects_non_level0_message_lens_with_f801() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let test_dir = std::env::temp_dir().join(format!("facet-run-f801-message-{}", nonce));
        std::fs::create_dir_all(&test_dir).expect("create temp dir");

        let input_path = test_dir.join("input.facet");
        let source = r#"
@user
  content: "hello" |> llm_call()
"#;
        std::fs::write(&input_path, source).expect("write facet file");

        let limiter = RateLimiter::direct(Quota::per_second(nonzero!(10u32)));
        let err = execute_run(
            input_path.clone(),
            None,
            1024,
            2048,
            "json".to_string(),
            false,
            true,
            true,
            &limiter,
        )
        .expect_err("message content non-level0 lens must be rejected");
        let text = err.to_string();
        assert!(text.contains("F801"), "expected F801, got: {text}");

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn execution_artifact_metadata_has_required_fields() {
        let payload = sample_payload();
        let artifact = build_execution_artifact(&payload, &[]).expect("artifact must build");

        let md = artifact
            .get("metadata")
            .and_then(|v| v.as_object())
            .expect("metadata object");

        assert_eq!(md.get("facet_version").unwrap(), "2.1.3");
        assert_eq!(md.get("host_profile_id").unwrap(), "local.default.v1");
        assert_eq!(md.get("document_hash").unwrap(), "sha256:abc");
        assert_eq!(md.get("policy_hash").unwrap(), "sha256:def");
        assert_eq!(md.get("policy_version").unwrap(), "1");
        assert_eq!(md.len(), 5, "metadata should contain only Appendix F fields");
    }

    #[test]
    fn execution_artifact_events_are_resequenced_without_gaps() {
        let payload = sample_payload();
        let decisions = vec![
            GuardDecision {
                seq: 7,
                op: "message_emit".to_string(),
                name: "system#1".to_string(),
                effect_class: None,
                mode: "exec".to_string(),
                decision: "allowed".to_string(),
                policy_rule_id: None,
                input_hash: "sha256:a".to_string(),
                error_code: None,
            },
            GuardDecision {
                seq: 42,
                op: "lens_call".to_string(),
                name: "trim".to_string(),
                effect_class: Some("read".to_string()),
                mode: "exec".to_string(),
                decision: "denied".to_string(),
                policy_rule_id: Some("r1".to_string()),
                input_hash: "sha256:b".to_string(),
                error_code: Some("F454".to_string()),
            },
        ];

        let artifact = build_execution_artifact(&payload, &decisions).expect("artifact");
        let events = artifact
            .get("provenance")
            .and_then(|v| v.get("events"))
            .and_then(|v| v.as_array())
            .expect("events array");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].get("seq").unwrap(), 1);
        assert_eq!(events[1].get("seq").unwrap(), 2);
    }

    #[test]
    fn execution_artifact_event_has_required_schema_fields() {
        let payload = sample_payload();
        let decisions = vec![GuardDecision {
            seq: 99,
            op: "tool_expose".to_string(),
            name: "WeatherAPI.get_current".to_string(),
            effect_class: Some("read".to_string()),
            mode: "exec".to_string(),
            decision: "allowed".to_string(),
            policy_rule_id: Some("allow-1".to_string()),
            input_hash: "sha256:abc123".to_string(),
            error_code: None,
        }];

        let artifact = build_execution_artifact(&payload, &decisions).expect("artifact");
        let event = artifact
            .get("provenance")
            .and_then(|v| v.get("events"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_object())
            .expect("first event object");

        let required = [
            "seq",
            "op",
            "name",
            "effect_class",
            "mode",
            "decision",
            "policy_rule_id",
            "input_hash",
        ];

        for key in required {
            assert!(event.contains_key(key), "missing required event key '{}'", key);
        }
    }

    #[test]
    fn execution_artifact_preserves_input_hash_values() {
        let payload = sample_payload();
        let decisions = vec![
            GuardDecision {
                seq: 4,
                op: "tool_call".to_string(),
                name: "WeatherAPI.get_current".to_string(),
                effect_class: Some("read".to_string()),
                mode: "exec".to_string(),
                decision: "allowed".to_string(),
                policy_rule_id: Some("allow-tool".to_string()),
                input_hash: "sha256:tool".to_string(),
                error_code: None,
            },
            GuardDecision {
                seq: 1,
                op: "lens_call".to_string(),
                name: "trim".to_string(),
                effect_class: Some("read".to_string()),
                mode: "exec".to_string(),
                decision: "allowed".to_string(),
                policy_rule_id: Some("allow-lens".to_string()),
                input_hash: "sha256:lens".to_string(),
                error_code: None,
            },
            GuardDecision {
                seq: 20,
                op: "tool_expose".to_string(),
                name: "WeatherAPI.get_current".to_string(),
                effect_class: Some("read".to_string()),
                mode: "exec".to_string(),
                decision: "allowed".to_string(),
                policy_rule_id: Some("allow-expose".to_string()),
                input_hash: "sha256:expose".to_string(),
                error_code: None,
            },
            GuardDecision {
                seq: 8,
                op: "message_emit".to_string(),
                name: "user#1".to_string(),
                effect_class: None,
                mode: "exec".to_string(),
                decision: "allowed".to_string(),
                policy_rule_id: Some("allow-message".to_string()),
                input_hash: "sha256:message".to_string(),
                error_code: None,
            },
        ];

        let artifact = build_execution_artifact(&payload, &decisions).expect("artifact");
        let events = artifact
            .get("provenance")
            .and_then(|v| v.get("events"))
            .and_then(|v| v.as_array())
            .expect("events array");

        let hashes: Vec<&str> = events
            .iter()
            .map(|e| {
                e.get("input_hash")
                    .and_then(|v| v.as_str())
                    .expect("input_hash string")
            })
            .collect();
        assert_eq!(
            hashes,
            vec!["sha256:tool", "sha256:lens", "sha256:expose", "sha256:message"]
        );
    }

    #[test]
    fn execution_artifact_hash_chain_replay_matches_head() {
        let payload = sample_payload();
        let decisions = vec![
            GuardDecision {
                seq: 3,
                op: "tool_expose".to_string(),
                name: "WeatherAPI.get_current".to_string(),
                effect_class: Some("read".to_string()),
                mode: "exec".to_string(),
                decision: "allowed".to_string(),
                policy_rule_id: Some("allow-expose".to_string()),
                input_hash: "sha256:aaa".to_string(),
                error_code: None,
            },
            GuardDecision {
                seq: 9,
                op: "message_emit".to_string(),
                name: "user#1".to_string(),
                effect_class: None,
                mode: "exec".to_string(),
                decision: "denied".to_string(),
                policy_rule_id: None,
                input_hash: "sha256:bbb".to_string(),
                error_code: Some("F454".to_string()),
            },
        ];

        let artifact = build_execution_artifact(&payload, &decisions).expect("artifact");
        let events = artifact
            .get("provenance")
            .and_then(|v| v.get("events"))
            .and_then(|v| v.as_array())
            .expect("events array");
        let emitted_head = artifact
            .get("provenance")
            .and_then(|v| v.get("hash_chain"))
            .and_then(|v| v.get("head"))
            .and_then(|v| v.as_str())
            .expect("head");

        let h0_input = serde_json::json!({
            "facet_version": payload.metadata.facet_version,
            "host_profile_id": payload.metadata.host_profile_id,
            "document_hash": payload.metadata.document_hash,
            "policy_hash": payload.metadata.policy_hash,
            "policy_version": payload.metadata.policy_version,
            "profile": payload.metadata.profile,
            "mode": payload.metadata.mode,
        });

        let mut replay_head = sha256_prefixed(canonicalize_json(&h0_input).unwrap().as_bytes());
        for event in events {
            let chain_input = serde_json::json!({
                "prev": replay_head,
                "event": event,
            });
            replay_head = sha256_prefixed(canonicalize_json(&chain_input).unwrap().as_bytes());
        }

        assert_eq!(replay_head, emitted_head);
    }

    #[test]
    fn execution_artifact_hash_chain_changes_on_event_tamper() {
        let payload = sample_payload();
        let decisions = vec![GuardDecision {
            seq: 1,
            op: "tool_call".to_string(),
            name: "WeatherAPI.get_current".to_string(),
            effect_class: Some("read".to_string()),
            mode: "exec".to_string(),
            decision: "allowed".to_string(),
            policy_rule_id: None,
            input_hash: "sha256:toolhash".to_string(),
            error_code: None,
        }];

        let artifact = build_execution_artifact(&payload, &decisions).expect("artifact");
        let original_head = artifact
            .get("provenance")
            .and_then(|v| v.get("hash_chain"))
            .and_then(|v| v.get("head"))
            .and_then(|v| v.as_str())
            .expect("head")
            .to_string();

        let mut tampered_event = artifact
            .get("provenance")
            .and_then(|v| v.get("events"))
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .cloned()
            .expect("event");
        tampered_event["name"] = serde_json::Value::String("WeatherAPI.refund".to_string());

        let h0_input = serde_json::json!({
            "facet_version": payload.metadata.facet_version,
            "host_profile_id": payload.metadata.host_profile_id,
            "document_hash": payload.metadata.document_hash,
            "policy_hash": payload.metadata.policy_hash,
            "policy_version": payload.metadata.policy_version,
            "profile": payload.metadata.profile,
            "mode": payload.metadata.mode,
        });

        let h0 = sha256_prefixed(canonicalize_json(&h0_input).unwrap().as_bytes());
        let tampered_chain_input = serde_json::json!({
            "prev": h0,
            "event": tampered_event,
        });
        let tampered_head =
            sha256_prefixed(canonicalize_json(&tampered_chain_input).unwrap().as_bytes());

        assert_ne!(tampered_head, original_head);
    }

    #[test]
    fn execution_artifact_includes_valid_attestation_envelope() {
        let payload = sample_payload();
        let artifact = build_execution_artifact_with_attestation(
            &payload,
            &[],
            Some(serde_json::json!({
                "algo": "ed25519",
                "key_id": "k1",
                "sig": "AbCdEf0123_-",
            })),
        )
        .expect("artifact with attestation");

        let attestation = artifact
            .get("attestation")
            .and_then(|v| v.as_object())
            .expect("attestation object");
        assert_eq!(attestation.get("algo").unwrap(), "ed25519");
        assert_eq!(attestation.get("key_id").unwrap(), "k1");
        assert_eq!(attestation.get("sig").unwrap(), "AbCdEf0123_-");
    }

    #[test]
    fn execution_artifact_rejects_invalid_attestation_algo() {
        let payload = sample_payload();
        let err = build_execution_artifact_with_attestation(
            &payload,
            &[],
            Some(serde_json::json!({
                "algo": "rsa2048",
                "key_id": "k1",
                "sig": "AbCdEf0123_-",
            })),
        )
        .unwrap_err();

        assert!(err.to_string().contains("Attestation.algo"));
    }

    #[test]
    fn execution_artifact_rejects_invalid_attestation_sig() {
        let payload = sample_payload();
        let err = build_execution_artifact_with_attestation(
            &payload,
            &[],
            Some(serde_json::json!({
                "algo": "x.acme.ed25519",
                "key_id": "k2",
                "sig": "not+base64url",
            })),
        )
        .unwrap_err();

        assert!(err.to_string().contains("Attestation.sig"));
    }
}
