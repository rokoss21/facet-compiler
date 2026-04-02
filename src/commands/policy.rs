use fct_render::CanonicalPayload;

pub fn hash_chain_seed_input(payload: &CanonicalPayload) -> serde_json::Value {
    serde_json::json!({
        "facet_version": payload.metadata.facet_version,
        "host_profile_id": payload.metadata.host_profile_id,
        "document_hash": payload.metadata.document_hash,
        "policy_hash": payload.metadata.policy_hash,
        "policy_version": payload.metadata.policy_version,
        "profile": payload.metadata.profile,
        "mode": payload.metadata.mode,
    })
}

#[cfg(test)]
mod tests {
    use super::hash_chain_seed_input;
    use fct_render::{CanonicalPayload, Metadata};

    #[test]
    fn hash_chain_seed_contains_required_policy_fields() {
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

        let seed = hash_chain_seed_input(&payload);
        assert_eq!(seed.get("policy_version").unwrap(), "1");
        assert_eq!(seed.get("policy_hash").unwrap(), "sha256:def");
        assert_eq!(seed.get("profile").unwrap(), "hypervisor");
        assert_eq!(seed.get("mode").unwrap(), "exec");
    }
}
