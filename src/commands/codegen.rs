//! # Codegen Command
//!
//! This module implements the code generation command for the FACET compiler.
//! The codegen command generates SDKs from FACET interface definitions.

use anyhow::{Result, Context};
use console::style;
use std::path::PathBuf;
use tracing::info;
use std::fs;

// Icon constants
const CODEGEN_EMOJI: console::Emoji = console::Emoji("🔧", "[CODEGEN] ");
const SUCCESS_EMOJI: console::Emoji = console::Emoji("✅", "");
const ERROR_EMOJI: console::Emoji = console::Emoji("❌", "");

/// Codegen command handler
pub fn execute_codegen(
    input: PathBuf,
    output: PathBuf,
    language: String,
    name: Option<String>,
    rate_limiter: &crate::commands::DefaultRateLimiter,
) -> Result<()> {
    // Check rate limit
    if rate_limiter.check().is_err() {
        eprintln!("{}", style("Rate limit exceeded. Please wait before running another command.").red());
        std::process::exit(1);
    }

    info!("Generating SDK for {:?} in {}", input, language);
    println!("{} Generating SDK", CODEGEN_EMOJI);
    println!("{} Input file: {:?}", CODEGEN_EMOJI, input);
    println!("{} Output directory: {:?}", CODEGEN_EMOJI, output);
    println!("{} Target language: {}", CODEGEN_EMOJI, language);

    // Validate language
    let normalized_lang = match language.to_lowercase().as_str() {
        "typescript" | "ts" => "typescript",
        "python" | "py" => "python",
        "rust" | "rs" => "rust",
        _ => {
            eprintln!("{} Unsupported language: {}. Supported: typescript, python, rust", ERROR_EMOJI, language);
            return Err(anyhow::anyhow!("Unsupported language: {}", language));
        }
    };

    // Validate input file exists
    if !input.exists() {
        return Err(anyhow::anyhow!("Input file does not exist: {:?}", input));
    }

    // Create output directory
    fs::create_dir_all(&output)
        .with_context(|| format!("Failed to create output directory: {:?}", output))?;

    // Parse the FACET document
    let content = fs::read_to_string(&input)
        .with_context(|| format!("Failed to read input file: {:?}", input))?;

    let document = fct_parser::parse_document(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse FACET document: {}", e))?;

    // Extract SDK name
    let sdk_name = name.unwrap_or_else(|| {
        input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("generated_sdk")
            .to_string()
    });

    println!("{} SDK name: {}", CODEGEN_EMOJI, sdk_name);

    // Extract interfaces from the document
    let interfaces = extract_interfaces(&document);

    if interfaces.is_empty() {
        println!("{} No interfaces found in document", CODEGEN_EMOJI);
        return Ok(());
    }

    println!("{} Found {} interface(s)", CODEGEN_EMOJI, interfaces.len());

    // Generate SDK based on language
    match normalized_lang {
        "typescript" => generate_typescript_sdk(&interfaces, &sdk_name, &output)?,
        "python" => generate_python_sdk(&interfaces, &sdk_name, &output)?,
        "rust" => generate_rust_sdk(&interfaces, &sdk_name, &output)?,
        _ => unreachable!(), // Already validated above
    }

    println!("{} SDK generated successfully!", SUCCESS_EMOJI);
    Ok(())
}

/// Extract interface definitions from parsed document
fn extract_interfaces(document: &fct_ast::FacetDocument) -> Vec<InterfaceInfo> {
    let mut interfaces = Vec::new();

    for block in &document.blocks {
        if let fct_ast::FacetNode::Interface(interface_block) = block {
            let methods = interface_block.functions
                .iter()
                .map(|func| MethodInfo {
                    name: func.name.clone(),
                    parameters: func.params
                        .iter()
                        .map(|param| ParameterInfo {
                            name: param.name.clone(),
                            param_type: Some(type_node_to_string(&param.type_node)),
                            default_value: None, // TODO: Handle default values if needed
                        })
                        .collect(),
                    return_type: Some(type_node_to_string(&func.return_type)),
                    description: None, // TODO: Extract description from comments if needed
                })
                .collect();

            interfaces.push(InterfaceInfo {
                name: interface_block.name.clone(),
                methods,
                description: None, // TODO: Extract description from comments if needed
            });
        }
    }

    interfaces
}

/// Convert TypeNode to string representation
fn type_node_to_string(type_node: &fct_ast::TypeNode) -> String {
    match type_node {
        fct_ast::TypeNode::Primitive(name) => match name.as_str() {
            "string" => "string".to_string(),
            "number" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "object" => "object".to_string(),
            _ => name.clone(),
        },
        fct_ast::TypeNode::Struct(_) => "object".to_string(),
        fct_ast::TypeNode::List(inner_type) => format!("{}[]", type_node_to_string(inner_type)),
        fct_ast::TypeNode::Map(_) => "object".to_string(),
        fct_ast::TypeNode::Union(_) => "any".to_string(),
        fct_ast::TypeNode::Image { .. } => "string".to_string(),
        fct_ast::TypeNode::Audio { .. } => "string".to_string(),
        fct_ast::TypeNode::Embedding { size } => format!("number[] /* embedding: {} dims */", size),
    }
}

/// Generate TypeScript SDK
fn generate_typescript_sdk(interfaces: &[InterfaceInfo], sdk_name: &str, output: &PathBuf) -> Result<()> {
    println!("{} Generating TypeScript SDK...", CODEGEN_EMOJI);

    let mut types_content = String::new();
    let mut client_content = String::new();

    // Generate types
    types_content.push_str(&format!("// {} SDK TypeScript Types\n", sdk_name));
    types_content.push_str("// Generated by FACET Codegen\n\n");

    for interface in interfaces {
        if let Some(description) = &interface.description {
            types_content.push_str(&format!("/**\n * {}\n */\n", description));
        }
        types_content.push_str(&format!("export interface {} {{\n", interface.name));

        for method in &interface.methods {
            if let Some(description) = &method.description {
                types_content.push_str(&format!("  /**\n   * {}\n   */\n", description));
            }

            let params: Vec<String> = method.parameters
                .iter()
                .map(|p| {
                    // TODO: Handle default values
                    format!("{}: {}", p.name, type_to_ts(&p.param_type))
                })
                .collect();

            let return_type = method.return_type
                .as_ref()
                .map(|t| type_to_ts(&Some(t.to_string())))
                .unwrap_or("void".to_string());

            types_content.push_str(&format!("  {}({}): Promise<{}>;\n", method.name, params.join(", "), return_type));
        }

        types_content.push_str("}\n\n");
    }

    // Generate client
    client_content.push_str(&format!("// {} SDK Client\n", sdk_name));
    client_content.push_str("// Generated by FACET Codegen\n\n");
    client_content.push_str("export class FACETClient {\n");
    client_content.push_str("  private baseUrl: string;\n\n");
    client_content.push_str("  constructor(baseUrl: string = 'https://api.facet.ai') {\n");
    client_content.push_str("    this.baseUrl = baseUrl;\n");
    client_content.push_str("  }\n\n");

    for interface in interfaces {
        client_content.push_str(&format!("  // {} interface\n", interface.name));

        for method in &interface.methods {
            client_content.push_str(&format!("  async {}(", method.name));

            let params: Vec<String> = method.parameters
                .iter()
                .map(|p| format!("{}: {}", p.name, type_to_ts(&p.param_type)))
                .collect();

            client_content.push_str(&params.join(", "));
            client_content.push_str("): Promise<");

            let return_type = method.return_type
                .as_ref()
                .map(|t| type_to_ts(&Some(t.to_string())))
                .unwrap_or("void".to_string());
            client_content.push_str(&return_type);
            client_content.push_str("> {\n");

            client_content.push_str("    // TODO: Implement actual API call\n");
            client_content.push_str(&format!("    throw new Error('Method {} not implemented');\n", method.name));
            client_content.push_str("  }\n\n");
        }
    }

    client_content.push_str("}\n");

    // Write files
    let types_path = output.join(format!("{}.types.ts", sdk_name.to_lowercase()));
    let client_path = output.join(format!("{}.ts", sdk_name.to_lowercase()));

    fs::write(&types_path, types_content)
        .with_context(|| format!("Failed to write types file: {:?}", types_path))?;

    fs::write(&client_path, client_content)
        .with_context(|| format!("Failed to write client file: {:?}", client_path))?;

    println!("{} TypeScript files generated:", SUCCESS_EMOJI);
    println!("  - {:?}", types_path);
    println!("  - {:?}", client_path);

    Ok(())
}

/// Generate Python SDK
fn generate_python_sdk(interfaces: &[InterfaceInfo], sdk_name: &str, output: &PathBuf) -> Result<()> {
    println!("{} Generating Python SDK...", CODEGEN_EMOJI);

    let mut content = String::new();

    content.push_str(&format!(r#"# {} SDK
# Generated by FACET Codegen

from typing import Dict, Any, Optional, List, Union
from dataclasses import dataclass
import asyncio
import json

"#, sdk_name));

    for interface in interfaces {
        if let Some(description) = &interface.description {
            content.push_str(&format!(r#"""{}"""

"#, description));
        }

        content.push_str(&format!(r#"class {}:
    """{} interface"""

    def __init__(self, base_url: str = "https://api.facet.ai"):
        self.base_url = base_url

"#, interface.name, interface.name));

        for method in &interface.methods {
            if let Some(description) = &method.description {
                content.push_str(&format!(r#"    """
    {}
    """
"#, description));
            }

            let params: Vec<String> = method.parameters
                .iter()
                .map(|p| {
                    // TODO: Handle default values
                    format!("{}: {}", p.name, type_to_python(&p.param_type))
                })
                .collect();

            let return_annotation = method.return_type
                .as_ref()
                .map(|t| format!(" -> {}", type_to_python(&Some(t.to_string()))))
                .unwrap_or_default();

            content.push_str(&format!(r#"    async def {}({}){}:
        """TODO: Implement {} method"""
        # TODO: Implement actual API call
        raise NotImplementedError("Method {} not implemented")
"#, method.name, params.join(", "), return_annotation, method.name, method.name));

            content.push_str("\n\n");
        }

        content.push_str("\n");
    }

    // Write __init__.py file
    let init_path = output.join("__init__.py");
    fs::write(&init_path, content)
        .with_context(|| format!("Failed to write Python file: {:?}", init_path))?;

    println!("{} Python SDK generated: {:?}", SUCCESS_EMOJI, init_path);
    Ok(())
}

/// Generate Rust SDK
fn generate_rust_sdk(interfaces: &[InterfaceInfo], sdk_name: &str, output: &PathBuf) -> Result<()> {
    println!("{} Generating Rust SDK...", CODEGEN_EMOJI);

    let mut content = String::new();

    content.push_str(&format!(r#"//! {} SDK
//! Generated by FACET Codegen

use serde::{{Deserialize, Serialize}};
use std::collections::HashMap;

"#, sdk_name));

    for interface in interfaces {
        if let Some(description) = &interface.description {
            content.push_str(&format!(r#"/// {}
///
{}
"#, interface.name, description));
        }

        content.push_str(&format!(r#"pub struct {}Client {{{{
    base_url: String,
}}}}

impl {}Client {{{{
    pub fn new(base_url: impl Into<String>) -> Self {{{{
        Self {{{{
            base_url: base_url.into(),
        }}}}
    }}}}

"#, interface.name, interface.name));

        for method in &interface.methods {
            if let Some(description) = &method.description {
                content.push_str(&format!(r#"    ///
    {}
    pub async fn {}("#, description, method.name));
            } else {
                content.push_str(&format!(r#"    pub async fn {}("#, method.name));
            }

            let params: Vec<String> = method.parameters
                .iter()
                .map(|p| format!("{}: {}", p.name, type_to_rust(&p.param_type)))
                .collect();

            let return_type = method.return_type
                .as_ref()
                .map(|t| type_to_rust(&Some(t.to_string())))
                .unwrap_or("()".to_string());

            content.push_str(&params.join(", "));
            content.push_str(&format!(") -> Result<{}, Box<dyn std::error::Error>> {{\n", return_type));
            content.push_str(&format!("        // TODO: Implement {} method\n", method.name));
            content.push_str("        Err(\"Not implemented\".into())\n");
            content.push_str("    }\n\n");
        }

        content.push_str("}\n\n");
    }

    // Write lib.rs file
    let lib_path = output.join("lib.rs");
    fs::write(&lib_path, content)
        .with_context(|| format!("Failed to write Rust file: {:?}", lib_path))?;

    // Write Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}-sdk"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
tokio = {{ version = "1.0", features = ["full"] }}
"#, sdk_name.to_lowercase());

    let cargo_path = output.join("Cargo.toml");
    fs::write(&cargo_path, cargo_toml)
        .with_context(|| format!("Failed to write Cargo.toml: {:?}", cargo_path))?;

    println!("{} Rust SDK generated:", SUCCESS_EMOJI);
    println!("  - {:?}", lib_path);
    println!("  - {:?}", cargo_path);

    Ok(())
}

// Helper structs and functions
#[derive(Debug)]
struct InterfaceInfo {
    name: String,
    methods: Vec<MethodInfo>,
    description: Option<String>,
}

#[derive(Debug)]
struct MethodInfo {
    name: String,
    parameters: Vec<ParameterInfo>,
    return_type: Option<String>,
    description: Option<String>,
}

#[derive(Debug)]
struct ParameterInfo {
    name: String,
    param_type: Option<String>,
    default_value: Option<fct_ast::ValueNode>,
}

// Type conversion helpers
fn type_to_ts(type_str: &Option<String>) -> String {
    match type_str {
        None => "any".to_string(),
        Some(t) => match t.as_str() {
            "string" => "string".to_string(),
            "number" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "object" => "any".to_string(),
            _ => t.clone(),
        }
    }
}

fn type_to_python(type_str: &Option<String>) -> String {
    match type_str {
        None => "Any".to_string(),
        Some(t) => match t.as_str() {
            "string" => "str".to_string(),
            "number" => "float".to_string(),
            "boolean" => "bool".to_string(),
            "object" => "Dict[str, Any]".to_string(),
            _ => t.clone(),
        }
    }
}

fn type_to_rust(type_str: &Option<String>) -> String {
    match type_str {
        None => "serde_json::Value".to_string(),
        Some(t) => match t.as_str() {
            "string" => "String".to_string(),
            "number" => "f64".to_string(),
            "boolean" => "bool".to_string(),
            "object" => "serde_json::Value".to_string(),
            _ => format!("serde_json::Value /* {} */", t),
        }
    }
}