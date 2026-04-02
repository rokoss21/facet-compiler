use anyhow::Result;
use fct_render::{CanonicalPayload, GuardDecision};
use sha2::{Digest, Sha256};

use crate::commands::canonical::canonicalize_json;
use crate::commands::guard::normalize_guard_decisions;
use crate::commands::policy::hash_chain_seed_input;

pub fn build_execution_artifact(
    payload: &CanonicalPayload,
    decisions: &[GuardDecision],
) -> Result<serde_json::Value> {
    build_execution_artifact_with_attestation(payload, decisions, None)
}

pub fn build_execution_artifact_with_attestation(
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

    let mut prev = sha256_prefixed(canonicalize_json(&hash_chain_seed_input(payload))?.as_bytes());
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

fn sha256_prefixed(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
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
    use super::build_execution_artifact;
    use fct_render::{CanonicalPayload, Metadata};

    #[test]
    fn build_execution_artifact_emits_hash_chain_head() {
        let payload = CanonicalPayload {
            metadata: Metadata {
                facet_version: "2.1.3".to_string(),
                profile: "hypervisor".to_string(),
                mode: "exec".to_string(),
                host_profile_id: "local.default.v1".to_string(),
                policy_version: "1".to_string(),
                document_hash: "sha256:abc".to_string(),
                policy_hash: Some("sha256:def".to_string()),
                budget_units: 1,
                target_provider_id: "generic-llm".to_string(),
            },
            tools: Vec::new(),
            messages: Vec::new(),
        };

        let artifact = build_execution_artifact(&payload, &[]).expect("artifact");
        assert_eq!(
            artifact
                .pointer("/provenance/hash_chain/algo")
                .expect("algo field"),
            "sha256"
        );
        assert!(artifact
            .pointer("/provenance/hash_chain/head")
            .and_then(|v| v.as_str())
            .is_some());
    }
}
