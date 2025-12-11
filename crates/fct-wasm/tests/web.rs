use wasm_bindgen_test::wasm_bindgen_test_configure;

wasm_bindgen_test_configure!(run_in_browser);

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen::JsValue;
    use serde_json::json;

    #[wasm_bindgen_test]
    fn parse_simple_facet() {
        let compiler = FacetCompiler::new();
        let result = compiler.parse_facet("@vars { name: \"test\" }");
        
        // Check if result has success property
        let success = js_sys::Reflect::get(&result, &"success".into())
            .unwrap_or(JsValue::FALSE)
            .as_bool()
            .unwrap_or(false);
        
        assert!(success, "Parse should succeed");
    }

    #[wasm_bindgen_test]
    fn validate_simple_ast() {
        let mut compiler = FacetCompiler::new();
        
        // Create a simple AST JSON
        let ast_json = json!({
            "blocks": [{
                "Vars": {
                    "name": "vars",
                    "attributes": {},
                    "body": [{
                        "KeyValue": {
                            "key": "test",
                            "value": { "String": "value" },
                            "span": { "start": 0, "end": 10, "line": 1, "column": 1 }
                        }
                    }],
                    "span": { "start": 0, "end": 10, "line": 1, "column": 1 }
                }
            }],
            "span": { "start": 0, "end": 10, "line": 1, "column": 1 }
        });
        
        let js_value = serde_wasm_bindgen::to_value(&ast_json).unwrap();
        let result = compiler.validate_ast(js_value);
        
        let success = js_sys::Reflect::get(&result, &"success".into())
            .unwrap_or(JsValue::FALSE)
            .as_bool()
            .unwrap_or(false);
        
        assert!(success, "Validation should succeed");
    }

    #[wasm_bindgen_test]
    fn compile_facet() {
        let mut compiler = FacetCompiler::new();
        let result = compiler.compile_facet(
            "@vars { name: \"Alice\" }\n\n@system { role: \"assistant\" }",
            None
        );
        
        let success = js_sys::Reflect::get(&result, &"success".into())
            .unwrap_or(JsValue::FALSE)
            .as_bool()
            .unwrap_or(false);
        
        assert!(success, "Compilation should succeed");
        
        // Check AST is present
        let has_ast = js_sys::Reflect::has(&result, &"ast".into());
        assert!(has_ast, "Result should have AST");
    }

    #[wasm_bindgen_test]
    fn version_returns_string() {
        let ver = version();
        assert!(!ver.is_empty(), "Version should not be empty");
        assert!(ver.contains('.'), "Version should have format x.y.z");
    }
}