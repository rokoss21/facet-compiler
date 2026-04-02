use fct_parser::parse_document;
use fct_resolver::{Resolver, ResolverConfig};
use fct_validator::validate_document;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_subdir(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(".tmp_{}_{}", prefix, nanos)
}

#[test]
fn import_cycle_is_detected_as_f602() {
    let dir_name = unique_temp_subdir("facet_cycle");
    let root = std::env::current_dir().unwrap().join(&dir_name);
    fs::create_dir_all(&root).unwrap();

    fs::write(root.join("a.facet"), "@import \"b.facet\"\n").unwrap();
    fs::write(root.join("b.facet"), "@import \"a.facet\"\n").unwrap();

    let source = format!("@import \"{}/a.facet\"\n", dir_name);
    let doc = parse_document(&source).unwrap();
    let result = validate_document(&doc);

    let _ = fs::remove_dir_all(&root);

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("F602"), "expected F602, got: {}", msg);
}

#[test]
fn filename_circular_without_cycle_is_allowed() {
    let dir_name = unique_temp_subdir("facet_no_cycle");
    let root = std::env::current_dir().unwrap().join(&dir_name);
    fs::create_dir_all(&root).unwrap();

    fs::write(root.join("circular.facet"), "@vars\n  value: \"ok\"\n").unwrap();

    let source = format!("@import \"{}/circular.facet\"\n", dir_name);
    let doc = parse_document(&source).unwrap();
    let result = validate_document(&doc);

    let _ = fs::remove_dir_all(&root);

    assert!(
        result.is_ok(),
        "unexpected validation error: {:?}",
        result.err()
    );
}

#[test]
fn url_import_is_rejected_as_f601() {
    let source = "@import \"https://example.com/a.facet\"\n";
    let doc = parse_document(source).unwrap();
    let result = validate_document(&doc);

    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("F601"), "expected F601, got: {}", msg);
}

#[test]
fn vars_singleton_merge_preserves_first_insertion_order() {
    let dir_name = unique_temp_subdir("facet_vars_singleton");
    let root = std::env::current_dir().unwrap().join(&dir_name);
    fs::create_dir_all(&root).unwrap();

    fs::write(root.join("module.facet"), "@vars\n  y: \"imported\"\n").unwrap();

    let source = format!(
        "@vars\n  x: \"root\"\n@import \"{}/module.facet\"\n@vars\n  z: \"tail\"\n  x: \"override\"\n",
        dir_name
    );
    let doc = parse_document(&source).unwrap();

    let cwd = std::env::current_dir().unwrap();
    let mut resolver = Resolver::new(ResolverConfig {
        allowed_roots: vec![cwd.clone()],
        base_dir: cwd,
    });
    let resolved = resolver.resolve(doc).unwrap();

    let _ = fs::remove_dir_all(&root);

    let vars_blocks: Vec<_> = resolved
        .blocks
        .iter()
        .filter_map(|node| match node {
            fct_ast::FacetNode::Vars(block) => Some(block),
            _ => None,
        })
        .collect();

    assert_eq!(
        vars_blocks.len(),
        1,
        "expected merged singleton @vars block"
    );

    let keys: Vec<_> = vars_blocks[0]
        .body
        .iter()
        .filter_map(|item| match item {
            fct_ast::BodyNode::KeyValue(kv) => Some(kv.key.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(keys, vec!["x", "y", "z"]);

    let x_value = vars_blocks[0]
        .body
        .iter()
        .find_map(|item| match item {
            fct_ast::BodyNode::KeyValue(kv) if kv.key == "x" => Some(&kv.value),
            _ => None,
        })
        .expect("x should exist");
    assert_eq!(x_value, &fct_ast::ValueNode::String("override".to_string()));
}
