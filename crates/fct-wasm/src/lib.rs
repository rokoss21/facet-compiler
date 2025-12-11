use wasm_bindgen::prelude::*;
use fct_ast::FacetDocument;
use fct_parser;
use fct_validator;
use fct_engine;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_wasm_bindgen;
use std::collections::HashMap;

// Initialize panic hook for better error messages in console
#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
}

/// Result types for WASM interface
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParseResult {
    success: bool,
    ast: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    success: bool,
    errors: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderResult {
    success: bool,
    output: Option<serde_json::Value>,
    error: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileResult {
    success: bool,
    ast: Option<serde_json::Value>,
    rendered: Option<serde_json::Value>,
    errors: Vec<String>,
}

/// FACET WebAssembly Compiler
#[wasm_bindgen]
pub struct FacetCompiler {
    validator: fct_validator::TypeChecker,
}

#[wasm_bindgen]
impl FacetCompiler {
    /// Create a new FACET compiler instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> FacetCompiler {
        FacetCompiler {
            validator: fct_validator::TypeChecker::new(),
        }
    }

    /// Parse FACET source code into AST
    #[wasm_bindgen(js_name = parse)]
    pub fn parse_facet(&self, source: &str) -> JsValue {
        match fct_parser::parse_document(source) {
            Ok(doc) => {
                let json = serde_json::to_value(&doc).unwrap_or_default();
                let result = ParseResult {
                    success: true,
                    ast: Some(json),
                    error: None,
                };
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
            }
            Err(e) => {
                let result = ParseResult {
                    success: false,
                    ast: None,
                    error: Some(e.to_string()),
                };
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
            }
        }
    }

    /// Validate parsed AST
    #[wasm_bindgen(js_name = validate)]
    pub fn validate_ast(&mut self, ast_json: JsValue) -> JsValue {
        // Convert JsValue to FacetDocument
        let doc: Result<FacetDocument, _> = serde_wasm_bindgen::from_value(ast_json);
        
        match doc {
            Ok(document) => {
                match self.validator.validate(&document) {
                    Ok(_) => {
                        let result = ValidationResult {
                            success: true,
                            errors: vec![],
                        };
                        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
                    }
                    Err(e) => {
                        let result = ValidationResult {
                            success: false,
                            errors: vec![e.to_string()],
                        };
                        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
                    }
                }
            }
            Err(e) => {
                let result = ValidationResult {
                    success: false,
                    errors: vec![format!("Invalid AST: {}", e)],
                };
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
            }
        }
    }

    /// Render AST to final output
    #[wasm_bindgen(js_name = render)]
    pub fn render_ast(&self, ast_json: JsValue, context_json: Option<JsValue>) -> JsValue {
        let doc: Result<FacetDocument, _> = serde_wasm_bindgen::from_value(ast_json);
        
        if let Err(e) = doc {
            let result = RenderResult {
                success: false,
                output: None,
                error: Some(format!("Invalid AST: {}", e)),
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED);
        }

        let document = doc.unwrap();
        
        // Parse context if provided
        let _context_map = if let Some(ctx) = context_json {
            match serde_wasm_bindgen::from_value::<HashMap<String, serde_json::Value>>(ctx) {
                Ok(map) => Some(map),
                Err(e) => {
                    let result = RenderResult {
                        success: false,
                        output: None,
                        error: Some(format!("Invalid context: {}", e)),
                    };
                    return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED);
                }
            }
        } else {
            None
        };

        // Build R-DAG and execute
        let mut engine = fct_engine::RDagEngine::new();
        match engine.build(&document) {
            Ok(_) => {
                match engine.validate() {
                    Ok(_) => {
                        let mut ctx = fct_engine::ExecutionContext::new(10000);
                        match engine.execute(&mut ctx) {
                            Ok(_) => {
                                // For WASM, return simplified output without full rendering
                                let output = json!({
                                    "blocks": document.blocks.len(),
                                    "variables": ctx.variables.len(),
                                    "executed": true
                                });
                                let result = RenderResult {
                                    success: true,
                                    output: Some(output),
                                    error: None,
                                };
                                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
                            }
                            Err(e) => {
                                let result = RenderResult {
                                    success: false,
                                    output: None,
                                    error: Some(format!("Execution error: {}", e)),
                                };
                                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
                            }
                        }
                    }
                    Err(e) => {
                        let result = RenderResult {
                            success: false,
                            output: None,
                            error: Some(format!("Validation error: {}", e)),
                        };
                        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
                    }
                }
            }
            Err(e) => {
                let result = RenderResult {
                    success: false,
                    output: None,
                    error: Some(format!("Build error: {}", e)),
                };
                serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
            }
        }
    }

    /// Compile FACET source code (parse + validate + render)
    #[wasm_bindgen(js_name = compile)]
    pub fn compile_facet(&mut self, source: &str, context_json: Option<JsValue>) -> JsValue {
        // Parse
        let parse_result = self.parse_facet(source);
        let parse_success = js_sys::Reflect::get(&parse_result, &"success".into())
            .unwrap_or(JsValue::FALSE)
            .as_bool()
            .unwrap_or(false);

        if !parse_success {
            let error = js_sys::Reflect::get(&parse_result, &"error".into())
                .unwrap_or(JsValue::UNDEFINED);
            let result = CompileResult {
                success: false,
                ast: None,
                rendered: None,
                errors: vec![error.as_string().unwrap_or_default()],
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED);
        }

        let ast = js_sys::Reflect::get(&parse_result, &"ast".into())
            .unwrap_or(JsValue::UNDEFINED);

        // Validate
        let validate_result = self.validate_ast(ast.clone());
        let validate_success = js_sys::Reflect::get(&validate_result, &"success".into())
            .unwrap_or(JsValue::FALSE)
            .as_bool()
            .unwrap_or(false);

        if !validate_success {
            let errors_js = js_sys::Reflect::get(&validate_result, &"errors".into())
                .unwrap_or(JsValue::UNDEFINED);
            let errors: Vec<String> = serde_wasm_bindgen::from_value(errors_js)
                .unwrap_or_default();
            let result = CompileResult {
                success: false,
                ast: serde_wasm_bindgen::from_value(ast).ok(),
                rendered: None,
                errors,
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED);
        }

        // Render
        let render_result = self.render_ast(ast.clone(), context_json);
        let render_success = js_sys::Reflect::get(&render_result, &"success".into())
            .unwrap_or(JsValue::FALSE)
            .as_bool()
            .unwrap_or(false);

        if !render_success {
            let error = js_sys::Reflect::get(&render_result, &"error".into())
                .unwrap_or(JsValue::UNDEFINED);
            let result = CompileResult {
                success: false,
                ast: serde_wasm_bindgen::from_value(ast).ok(),
                rendered: None,
                errors: vec![error.as_string().unwrap_or_default()],
            };
            return serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED);
        }

        let output = js_sys::Reflect::get(&render_result, &"output".into())
            .unwrap_or(JsValue::UNDEFINED);
        
        let result = CompileResult {
            success: true,
            ast: serde_wasm_bindgen::from_value(ast).ok(),
            rendered: serde_wasm_bindgen::from_value(output).ok(),
            errors: vec![],
        };
        
        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::UNDEFINED)
    }
}

/// Convenience function for one-shot compilation
#[wasm_bindgen]
pub fn compile(source: &str, context: Option<JsValue>) -> JsValue {
    let mut compiler = FacetCompiler::new();
    compiler.compile_facet(source, context)
}

/// Get version information
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Initialize the WASM module (call this before using other functions)
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}