use anyhow::Result;

pub fn canonicalize_json(value: &serde_json::Value) -> Result<String> {
    Ok(serde_json_canonicalizer::to_string(value)?)
}

#[cfg(test)]
mod tests {
    use super::canonicalize_json;

    #[test]
    fn canonicalize_json_is_stable_for_equivalent_maps() {
        let a = serde_json::json!({"b":2,"a":1});
        let b = serde_json::json!({"a":1,"b":2});
        assert_eq!(canonicalize_json(&a).unwrap(), canonicalize_json(&b).unwrap());
    }
}
