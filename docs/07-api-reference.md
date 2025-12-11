---
---
# 07. FACET v2.0 Rust API Reference

**Reading Time:** 30-40 minutes | **Difficulty:** Advanced | **Previous:** [06-cli.md](06-cli.md) | **Next:** [08-lenses.md](08-lenses.md)

**Version:** 0.1.0
**Last Updated:** 2025-12-09
**Status:** Production Ready

---

## Table of Contents

- [Overview](#overview)
- [fct-ast](#fct-ast)
- [fct-parser](#fct-parser)
- [fct-validator](#fct-validator)
- [fct-engine](#fct-engine)
- [fct-render](#fct-render)
- [fct-std](#fct-std)
- [fct-wasm](#fct-wasm)
- [Integration Examples](#integration-examples)

---

## Overview

FACET v2.0 provides a modular Rust API organized across 7 crates. All crates follow semantic versioning and are published to [crates.io](https://crates.io).

### Quick Start

```rust
use facet_fct::*; // Re-exports all public APIs

// Parse and compile a FACET file
let content = std::fs::read_to_string("agent.facet")?;
let doc = fct_parser::parse_document(&content)?;
let validated = fct_validator::TypeChecker::new().validate(&doc)?;
let mut engine = fct_engine::RDagEngine::new();
let result = fct_render::render(&validated)?;
```

---

## fct-ast

**Abstract Syntax Tree definitions and serialization.**

### Core Types

#### FacetDocument
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetDocument {
    pub blocks: Vec<FacetNode>,
    pub span: Span,
}
```

#### FacetNode
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum FacetNode {
    Meta(FacetBlock),
    System(FacetBlock),
    User(FacetBlock),
    Assistant(FacetBlock),
    Vars(FacetBlock),
    VarTypes(FacetBlock),
    Context(FacetBlock),
    Import(ImportNode),
    Interface(InterfaceNode),
    Test(TestBlock),
}
```

#### ValueNode
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueNode {
    Scalar(ScalarValue),
    String(String),
    Variable(String),
    List(Vec<ValueNode>),
    Map(HashMap<String, ValueNode>),
    Pipeline(PipelineNode),
    Directive(DirectiveNode),
}
```

### Key Functions

```rust
// Create spans for error reporting
pub fn Span::new(start: usize, end: usize, line: usize, column: usize) -> Span

// Type-safe value construction
pub fn ValueNode::String(s: String) -> ValueNode
pub fn ValueNode::List(items: Vec<ValueNode>) -> ValueNode
pub fn ValueNode::Map(map: HashMap<String, ValueNode>) -> ValueNode
```

---

## fct-parser

**nom-based parser for FACET syntax with comprehensive error handling.**

### Main API

```rust
pub fn parse_document(input: &str) -> Result<FacetDocument, String>
```

Parses a complete FACET document from string input.

**Parameters:**
- `input`: UTF-8 string containing FACET syntax

**Returns:**
- `Ok(FacetDocument)`: Successfully parsed AST
- `Err(String)`: Parse error with F001-F003 error codes

**Example:**
```rust
use fct_parser::parse_document;

let content = r#"
@system
  role: "assistant"

@vars
  name: "Alice"
"#;

let doc = parse_document(content)?;
assert_eq!(doc.blocks.len(), 2);
```

### Error Types

Parser returns specific error codes:
- **F001**: Invalid indentation
- **F002**: Tabs not allowed
- **F003**: Parse error or unclosed structure

---

## fct-validator

**Facet Type System (FTS) implementation with constraint validation.**

### TypeChecker

```rust
pub struct TypeChecker {
    // Internal state for type inference and validation
}

impl TypeChecker {
    pub fn new() -> Self
    pub fn validate(&mut self, doc: &FacetDocument) -> ValidationResult<()>
    pub fn check_variable_resolution(&self, doc: &FacetDocument) -> ValidationResult<()>
    pub fn validate_imports(&self, doc: &FacetDocument) -> ValidationResult<()>
    pub fn check_type_constraints(&self, value: &ValueNode, expected_type: &FacetType) -> ValidationResult<()>
}
```

### Validation Errors

```rust
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("F401: Variable not found: {var}")]
    VariableNotFound { var: String },

    #[error("F402: Type inference failed: {message}")]
    TypeInferenceFailed { message: String },

    #[error("F404: Forward reference detected: variable {var} used before declaration")]
    ForwardReference { var: String },

    #[error("F451: Type mismatch: expected {expected}, got {got} at {location}")]
    TypeMismatch { expected: String, got: String, location: String },

    #[error("F452: Constraint violation: {constraint} failed for value {value}")]
    ConstraintViolation { constraint: String, value: String },

    #[error("F453: Runtime input validation failed: {message}")]
    InputValidationFailed { message: String },

    #[error("F601: Import not found: {path}")]
    ImportNotFound { path: String },

    #[error("F602: Circular import detected: {path}")]
    CircularImport { path: String },

    #[error("F802: Unknown lens: {lens_name}")]
    UnknownLens { lens_name: String },
}
```

### Facet Type System (FTS)

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FacetType {
    Primitive(String),                    // "string", "int", "float", "bool"
    Struct(HashMap<String, FacetType>),   // {name: string, age: int}
    List(Box<FacetType>),                  // List<string>
    Map(Box<FacetType>),                   // Map<string>
    Union(Vec<FacetType>),                 // string | int
    Image { max_dim: Option<u32>, format: Option<String> },
    Audio { max_duration: Option<f64>, format: Option<String> },
    Embedding { size: usize },
}
```

---

## fct-engine

**Reactive DAG execution engine with Token Box Model.**

### RDagEngine

```rust
pub struct RDagEngine {
    // Internal DAG state
}

impl RDagEngine {
    pub fn new() -> Self
    pub fn build(&mut self, doc: &FacetDocument) -> EngineResult<()>
    pub fn validate(&self) -> EngineResult<()>
    pub fn execute(&mut self, ctx: &mut ExecutionContext) -> EngineResult<()>
    pub fn execute_lazy(&mut self, ctx: &ExecutionContext) -> EngineResult<HashMap<String, ValueNode>>
    pub fn get_variable(&self, name: &str) -> Option<&ValueNode>
    pub fn get_all_variables(&self) -> &HashMap<String, ValueNode>
}
```

### ExecutionContext

```rust
pub struct ExecutionContext {
    pub gas_limit: usize,
    pub gas_used: usize,
    pub token_budget: usize,
    pub variables: HashMap<String, ValueNode>,
}

impl ExecutionContext {
    pub fn new(gas_limit: usize) -> Self
    pub fn check_gas(&mut self, cost: usize) -> EngineResult<()>
    pub fn set_variable(&mut self, name: String, value: ValueNode)
    pub fn get_variable(&self, name: &str) -> Option<&ValueNode>
}
```

### Token Box Model

```rust
pub struct TokenBoxModel {
    // Context allocation state
}

impl TokenBoxModel {
    pub fn new(budget: usize) -> Self
    pub fn allocate(&mut self, sections: Vec<Section>) -> Result<AllocationResult, EngineError>
    pub fn allocate_optimal(&mut self, variables: &HashMap<String, ValueNode>, budget: usize) -> Result<AllocationResult, EngineError>
    pub fn get_efficiency(&self) -> f64
}

#[derive(Debug, Clone)]
pub struct Section {
    pub id: String,
    pub content: ValueNode,
    pub priority: u32,
    pub base_size: usize,
    pub min: usize,
    pub grow: f64,
    pub shrink: f64,
    pub current_size: usize,
    pub is_critical: bool,
}

impl Section {
    pub fn new(id: String, content: ValueNode, base_size: usize) -> Self
    pub fn with_priority(mut self, priority: u32) -> Self
    pub fn with_limits(mut self, min: usize, grow: f64, shrink: f64) -> Self
    pub fn critical(mut self) -> Self
}
```

### TestRunner

```rust
pub struct TestRunner {
    gas_limit: usize,
    token_budget: usize,
}

impl TestRunner {
    pub fn new(gas_limit: usize, token_budget: usize) -> Self
    pub fn run_all(&self, doc: &FacetDocument) -> Vec<TestResult>
    pub fn run_single_test(&self, test: &TestBlock, ctx: &TestContext) -> TestResult
    pub fn evaluate_assertions(&self, output: &str, ctx: &TestContext, assertions: &[Assertion]) -> Vec<AssertionResult>
}
```

### Engine Errors

```rust
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("F505: Cyclic dependency detected in variable graph: {cycle}")]
    CyclicDependency { cycle: String },

    #[error("F401: Variable not found: {var}")]
    VariableNotFound { var: String },

    #[error("F902: Compute gas exhausted (limit: {limit})")]
    GasExhausted { limit: usize },

    #[error("Lens execution failed: {message}")]
    LensExecutionFailed { message: String },

    #[error("F901: Critical sections exceed budget (budget: {budget}, required: {required})")]
    BudgetExceeded { budget: usize, required: usize },
}
```

---

## fct-render

**JSON rendering for LLM APIs (OpenAI, Anthropic, etc.).**

### Main API

```rust
pub fn render(doc: &FacetDocument) -> Result<serde_json::Value, EngineError>
```

Renders a validated FACET document to canonical JSON.

**Parameters:**
- `doc`: Validated FacetDocument

**Returns:**
- `Ok(Value)`: Canonical JSON representation
- `Err(EngineError)`: Rendering error

**Example:**
```rust
use fct_render::render;

let json = render(&validated_doc)?;
println!("{}", serde_json::to_string_pretty(&json)?);
```

### Output Format

```json
{
  "system": {
    "role": "assistant",
    "instructions": "..."
  },
  "user": {
    "query": "...",
    "context": {...}
  },
  "variables": {
    "computed_var": "value"
  },
  "metadata": {
    "tokens_used": 1500,
    "gas_used": 500
  }
}
```

---

## fct-std

**Standard lens library with 15 production-ready transformations.**

### LensRegistry

```rust
pub struct LensRegistry {
    lenses: HashMap<String, Box<dyn Lens>>,
}

impl LensRegistry {
    pub fn new() -> Self
    pub fn load_standard() -> Result<Self, Box<dyn std::error::Error>>
    pub fn register(&mut self, name: String, lens: Box<dyn Lens>)
    pub fn get(&self, name: &str) -> Option<&dyn Lens>
    pub fn execute(&self, name: &str, input: &ValueNode, args: &[ValueNode], kwargs: &HashMap<String, ValueNode>) -> Result<ValueNode, Box<dyn std::error::Error>>
}
```

### Available Lenses

#### String Lenses
```rust
// Text processing
trim(value: string) -> string
lowercase(value: string) -> string
uppercase(value: string) -> string
capitalize(value: string) -> string
reverse(value: string) -> string
substring(value: string, start: int, end?: int) -> string
replace(value: string, old: string, new: string) -> string
split(value: string, separator: string) -> List<string>
join(values: List<string>, separator: string) -> string
```

#### List Lenses
```rust
first(list: List<T>) -> T
last(list: List<T>) -> T
nth(list: List<T>, index: int) -> T
slice(list: List<T>, start: int, end?: int) -> List<T>
length(list: List<T>) -> int
unique(list: List<T>) -> List<T>
sort(list: List<Comparable>) -> List<Comparable>
filter(list: List<T>, predicate: string) -> List<T>
map(list: List<T>, transform: string) -> List<T>
```

#### Utility Lenses
```rust
template(template: string, **kwargs) -> string
json_parse(value: string) -> any
json_stringify(value: any) -> string
url_encode(value: string) -> string
url_decode(value: string) -> string
hash(value: string, algorithm?: string) -> string
```

### Custom Lens Implementation

```rust
use fct_std::{Lens, LensContext, LensResult};

#[derive(Debug)]
pub struct CustomLens;

impl Lens for CustomLens {
    fn name(&self) -> &str {
        "custom"
    }

    fn apply(&self, input: &ValueNode, args: &[ValueNode], kwargs: &HashMap<String, ValueNode>, _ctx: &LensContext) -> LensResult<ValueNode> {
        // Custom transformation logic
        Ok(ValueNode::String("transformed".to_string()))
    }
}

// Register custom lens
let mut registry = LensRegistry::new();
registry.register("custom".to_string(), Box::new(CustomLens));
```

---

## fct-wasm

**WebAssembly bindings for browser and Node.js usage.**

### FacetCompiler

```javascript
import { FacetCompiler } from 'facet-fct';

const compiler = new FacetCompiler();

try {
  // Parse FACET source
  const ast = compiler.parse(source);

  // Validate AST
  const validated = compiler.validate(ast);

  // Render to JSON
  const json = compiler.render(validated);

  console.log(json);
} catch (error) {
  console.error('Compilation error:', error);
}
```

### WASM API

```rust
#[wasm_bindgen]
pub struct FacetCompiler {
    validator: fct_validator::TypeChecker,
}

#[wasm_bindgen]
impl FacetCompiler {
    #[wasm_bindgen(constructor)]
    pub fn new() -> FacetCompiler

    #[wasm_bindgen(js_name = parse)]
    pub fn parse_facet(&self, source: &str) -> JsValue

    #[wasm_bindgen(js_name = validate)]
    pub fn validate_ast(&mut self, ast_json: JsValue) -> JsValue

    #[wasm_bindgen(js_name = render)]
    pub fn render_ast(&self, ast_json: JsValue, context_json: Option<JsValue>) -> JsValue

    #[wasm_bindgen(js_name = compile)]
    pub fn compile_facet(&mut self, source: &str, context_json: Option<JsValue>) -> JsValue
}
```

### Standalone Functions

```javascript
import { compile, version, init } from 'facet-fct';

// Initialize WASM module
await init();

// One-shot compilation
const result = compile(facetSource, context);
console.log('Version:', version());
```

---

## Integration Examples

### Basic Compilation Pipeline

```rust
use facet_fct::*;

fn compile_facet(source: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Parse
    let doc = fct_parser::parse_document(source)?;

    // Validate
    let mut validator = fct_validator::TypeChecker::new();
    validator.validate(&doc)?;

    // Execute
    let mut engine = fct_engine::RDagEngine::new();
    engine.build(&doc)?;
    engine.validate()?;

    let mut ctx = fct_engine::ExecutionContext::new(10000);
    engine.execute(&mut ctx)?;

    // Render
    let json = fct_render::render(&doc)?;

    Ok(json)
}
```

### Advanced Usage with Token Budgeting

```rust
use fct_engine::{RDagEngine, ExecutionContext, TokenBoxModel};

fn execute_with_budget(
    doc: &FacetDocument,
    token_budget: usize,
    gas_limit: usize
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Build execution engine
    let mut engine = RDagEngine::new();
    engine.build(doc)?;
    engine.validate()?;

    // Execute with gas limit
    let mut ctx = ExecutionContext::new(gas_limit);
    engine.execute(&mut ctx)?;

    // Allocate context using Token Box Model
    let variables = engine.get_all_variables();
    let mut token_box = TokenBoxModel::new(token_budget);
    let allocation = token_box.allocate_optimal(variables, token_budget)?;

    // Render with allocation info
    let mut json = fct_render::render(doc)?;
    json["allocation"] = serde_json::to_value(&allocation)?;

    Ok(json)
}
```

### Testing Integration

```rust
use fct_engine::TestRunner;

fn run_tests(source: &str) -> Vec<fct_engine::TestResult> {
    let doc = fct_parser::parse_document(source).expect("Parse failed");

    let runner = TestRunner::new(10000, 4096); // gas, tokens
    runner.run_all(&doc)
}
```

### WASM Integration

```javascript
// In Node.js or browser
import { FacetCompiler, compile } from 'facet-fct';

async function compileInBrowser(facetSource) {
    await FacetCompiler.init();

    try {
        const result = compile(facetSource);
        return JSON.parse(result);
    } catch (error) {
        throw new Error(`Compilation failed: ${error}`);
    }
}
```

---

## Error Handling Patterns

### Comprehensive Error Handling

```rust
use fct_parser::parse_document;
use fct_validator::{TypeChecker, ValidationError};
use fct_engine::{RDagEngine, EngineError};

fn compile_with_error_handling(source: &str) -> Result<serde_json::Value, String> {
    // Parse with specific error codes
    let doc = parse_document(source)
        .map_err(|e| format!("Parse error: {}", e))?;

    // Validate with detailed error reporting
    let mut validator = TypeChecker::new();
    validator.validate(&doc)
        .map_err(|e| match e {
            ValidationError::VariableNotFound { var } =>
                format!("F401: Variable '{}' not found", var),
            ValidationError::TypeMismatch { expected, got, location } =>
                format!("F451: Type mismatch at {}: expected {}, got {}", location, expected, got),
            other => format!("Validation error: {}", other),
        })?;

    // Execute with resource monitoring
    let mut engine = RDagEngine::new();
    engine.build(&doc)
        .map_err(|e| format!("Build error: {}", e))?;

    engine.validate()
        .map_err(|e| format!("Validation error: {}", e))?;

    let mut ctx = ExecutionContext::new(10000);
    engine.execute(&mut ctx)
        .map_err(|e| match e {
            EngineError::GasExhausted { limit } =>
                format!("F902: Gas limit {} exceeded", limit),
            EngineError::CyclicDependency { cycle } =>
                format!("F505: Cyclic dependency: {}", cycle),
            other => format!("Execution error: {}", other),
        })?;

    // Render result
    fct_render::render(&doc)
        .map_err(|e| format!("Render error: {}", e))
}
```

## Next Steps

🎯 **Implementation Guides:**
- **[08-lenses.md](08-lenses.md)** - Lens library usage
- **[09-testing.md](09-testing.md)** - Testing frameworks
- **[13-import-system.md](13-import-system.md)** - Import system internals

🔧 **Performance & Security:**
- **[10-performance.md](10-performance.md)** - API performance characteristics
- **[11-security.md](11-security.md)** - Secure API usage
- **[12-errors.md](12-errors.md)** - Error handling in Rust

📚 **Resources:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete specification
- **[PRD](../facetparcer.prd)** - Architecture requirements
- **[README.md](../README.md)** - Integration examples

---

This API reference provides comprehensive coverage of FACET v2.0's Rust interfaces. All crates follow Rust's stability guarantees and semantic versioning. For the latest documentation, see [docs.rs](https://docs.rs) or the inline code documentation.

🎯 **Implementation Guides:**
- **[08-lenses.md](08-lenses.md)** - Lens library usage
- **[09-testing.md](09-testing.md)** - Testing frameworks
- **[13-import-system.md](13-import-system.md)** - Import system internals

🔧 **Performance & Security:**
- **[10-performance.md](10-performance.md)** - API performance characteristics
- **[11-security.md](11-security.md)** - Secure API usage
- **[12-errors.md](12-errors.md)** - Error handling in Rust

📚 **Resources:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete specification
- **[PRD](../facetparcer.prd)** - Architecture requirements
- **[README.md](../README.md)** - Integration examples

---

This API reference provides comprehensive coverage of FACET v2.0's Rust interfaces. All crates follow Rust's stability guarantees and semantic versioning. For the latest documentation, see [docs.rs](https://docs.rs) or the inline code documentation.
