use fct_engine::ExecutionGuardDecision;
use fct_render::GuardDecision;

pub fn merge_guard_decisions(
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

    normalize_guard_decisions(&merged)
}

pub fn normalize_guard_decisions(decisions: &[GuardDecision]) -> Vec<GuardDecision> {
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

#[cfg(test)]
mod tests {
    use super::merge_guard_decisions;
    use fct_engine::ExecutionGuardDecision;
    use fct_render::GuardDecision;

    #[test]
    fn merge_guard_decisions_resequences_all_events() {
        let engine = vec![ExecutionGuardDecision {
            seq: 9,
            op: "lens_call".to_string(),
            name: "trim".to_string(),
            effect_class: Some("read".to_string()),
            mode: "exec".to_string(),
            decision: "allowed".to_string(),
            policy_rule_id: None,
            input_hash: "sha256:a".to_string(),
            error_code: None,
        }];
        let render = vec![GuardDecision {
            seq: 42,
            op: "message_emit".to_string(),
            name: "user#1".to_string(),
            effect_class: None,
            mode: "exec".to_string(),
            decision: "allowed".to_string(),
            policy_rule_id: None,
            input_hash: "sha256:b".to_string(),
            error_code: None,
        }];

        let merged = merge_guard_decisions(&engine, &render);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].seq, 1);
        assert_eq!(merged[1].seq, 2);
    }
}
