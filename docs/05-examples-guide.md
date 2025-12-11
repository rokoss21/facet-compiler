---
---
# 05. FACET Examples Guide

**Reading Time:** 45-60 minutes | **Difficulty:** Beginner → Advanced | **Previous:** [04-type-system.md](04-type-system.md) | **Next:** [06-cli.md](06-cli.md)

**Version:** 1.0
**Last Updated:** 2025-12-09
**Status:** Production Ready

---

## Table of Contents

- [Overview](#overview)
- [Getting Started](#getting-started)
- [Example 1: Basic Prompt (`basic.facet`)](#example-1-basic-prompt-basicfacet)
- [Example 2: Variables & Pipelines (`basic_prompt.facet`)](#example-2-variables--pipelines-basic_promptfacet)
- [Example 3: Pipeline Operations (`pipeline_test.facet`)](#example-3-pipeline-operations-pipeline_testfacet)
- [Example 4: Type System (`types_test.facet`)](#example-4-type-system-types_testfacet)
- [Example 5: RAG Pipeline (`rag_pipeline.facet`)](#example-5-rag-pipeline-rag_pipelinefacet)
- [Example 6: Advanced Features (`advanced_features.facet`)](#example-6-advanced-features-advanced_featuresfacet)
- [Example 7: Testing (`test_example.facet`)](#example-7-testing-test_examplefacet)
- [Common Patterns](#common-patterns)
- [Creating Your Own Examples](#creating-your-own-examples)
- [Troubleshooting](#troubleshooting)

---

## Overview

FACET ships with 8 comprehensive examples demonstrating different aspects of the language. Each example is runnable and shows real-world usage patterns.

### Examples Directory Structure

```
examples/
├── basic.facet           # Minimal agent
├── basic_prompt.facet    # Variables & string processing
├── pipeline_test.facet   # Lens pipelines & data structures
├── types_test.facet      # Type system & constraints
├── rag_pipeline.facet    # Complex document processing
├── advanced_features.facet # Complete feature showcase
├── test_example.facet    # @test blocks & mocking
└── README.md            # Quick reference
```

### Running Examples

All examples can be run with:

```bash
# Validate syntax and types
fct build --input examples/basic.facet

# Execute full pipeline
fct run --input examples/basic.facet --budget 4096

# Inspect AST structure
fct inspect --input examples/basic.facet

# Run tests (for test_example.facet)
fct test --input examples/test_example.facet
```

---

## Getting Started

### Prerequisites

1. **Install FACET:**
   ```bash
   cargo build --release
   export PATH="$PWD/target/release:$PATH"
   ```

2. **Verify installation:**
   ```bash
   fct --version
   ```

3. **Test basic functionality:**
   ```bash
   fct build --input examples/basic.facet
   ```

### Example Workflow

1. **Start simple:** `basic.facet`
2. **Add variables:** `basic_prompt.facet`
3. **Try pipelines:** `pipeline_test.facet`
4. **Add types:** `types_test.facet`
5. **Build complex systems:** `rag_pipeline.facet`
6. **Explore advanced features:** `advanced_features.facet`
7. **Add testing:** `test_example.facet`

---

## Example 1: Basic Prompt (`basic.facet`)

**File:** `examples/basic.facet`
**Purpose:** Minimal FACET agent demonstrating core structure
**Concepts:** `@system`, `@user` blocks, basic JSON output

### Source Code

```facet
@system
  role: "assistant"
  model: "gpt-4"

@user
  name: "User"
  query: "Hello"
```

### What It Does

This is the simplest possible FACET file that defines:
- **System configuration:** AI assistant using GPT-4
- **User input:** Basic greeting with metadata

### Output

```json
{
  "system": {
    "role": "assistant",
    "model": "gpt-4"
  },
  "user": {
    "name": "User",
    "query": "Hello"
  }
}
```

### Key Learning Points

- **Required blocks:** Every FACET file needs `@system` and `@user`
- **Minimal structure:** Just role, model, and query
- **JSON output:** FACET compiles to clean, canonical JSON
- **No variables:** This example doesn't use `@vars` or transformations

### Try It

```bash
# Basic validation
fct build --input examples/basic.facet

# Full execution
fct run --input examples/basic.facet --format pretty

# Inspect internal structure
fct inspect --input examples/basic.facet
```

---

## Example 2: Variables & Pipelines (`basic_prompt.facet`)

**File:** `examples/basic_prompt.facet`
**Purpose:** Introduction to variables and string processing pipelines
**Concepts:** `@vars`, `@meta`, lens pipelines, variable substitution

### Source Code

```facet
@meta
  title: "Simple AI Assistant"
  version: "1.0"
  description: "Basic example with variable substitution"

@vars
  user_name: "Alice"
  task: "  write a poem  "

  # Clean and format inputs using lens pipelines
  clean_name: $user_name |> trim() |> uppercase()
  clean_task: $task |> trim() |> lowercase()

@system
  role: "assistant"
  model: "gpt-4"
  temperature: 0.7
  instructions: "You are a helpful AI assistant. Be concise and friendly."

@user
  name: $clean_name
  content: "Hello! I would like you to help me with this task:"
  task_description: $clean_task
```

### What It Does

**Input Processing:**
- Raw `user_name`: `"Alice"` → Clean: `"ALICE"`
- Raw `task`: `"  write a poem  "` → Clean: `"write a poem"`

**Pipeline Execution:**
1. `trim()` removes whitespace
2. `uppercase()` transforms case
3. `lowercase()` normalizes text

### Variable Resolution

Variables are resolved in dependency order:
1. `user_name` and `task` (literals)
2. `clean_name` (depends on `user_name`)
3. `clean_task` (depends on `task`)

### Output

```json
{
  "system": {
    "role": "assistant",
    "model": "gpt-4",
    "temperature": 0.7,
    "instructions": "You are a helpful AI assistant. Be concise and friendly."
  },
  "user": {
    "name": "ALICE",
    "content": "Hello! I would like you to help me with this task:",
    "task_description": "write a poem"
  }
}
```

### Key Learning Points

- **`@meta` block:** Optional metadata for documentation
- **`@vars` block:** Variable definitions and transformations
- **Lens pipelines:** `|> trim() |> uppercase()` chains operations
- **Variable substitution:** `$variable_name` in any block
- **Dependency resolution:** R-DAG automatically orders execution

### Try It

```bash
# See the transformation in action
fct run --input examples/basic_prompt.facet --format pretty

# Change inputs and see results
# Edit the file and modify user_name/task values
```

---

## Example 3: Pipeline Operations (`pipeline_test.facet`)

**File:** `examples/pipeline_test.facet`
**Purpose:** Data structure manipulation and list handling
**Concepts:** Lists, string pipelines, complex data structures

### Source Code

```facet
@system
  role: "assistant"
  model: "gpt-4"

@vars
  username: "Alice"
  greeting: $username |> trim() |> lowercase()
  formatted: "Hello" |> trim() |> uppercase()

@user
  query: "Test pipeline"
  items:
    - item1
    - item2
    - item3
```

### What It Does

**String Transformations:**
- `"Alice"` → `"alice"` (lowercase)
- `"Hello"` → `"HELLO"` (uppercase)

**List Structure:**
- YAML-style list with `-` bullets
- Converted to JSON array format

### Output

```json
{
  "system": {
    "role": "assistant",
    "model": "gpt-4"
  },
  "user": {
    "query": "Test pipeline",
    "items": ["item1", "item2", "item3"]
  },
  "variables": {
    "username": "Alice",
    "greeting": "alice",
    "formatted": "HELLO"
  }
}
```

### Key Learning Points

- **List syntax:** `- item` creates JSON arrays
- **Variable-only output:** `@vars` appear in `variables` section
- **Pipeline composition:** Multiple lenses in sequence
- **Case transformations:** `lowercase()` and `uppercase()` lenses

### Try It

```bash
# See variable processing
fct run --input examples/pipeline_test.facet --format pretty

# Experiment with different lenses
# Try: $username |> trim() |> capitalize()
```

---

## Example 4: Type System (`types_test.facet`)

**File:** `examples/types_test.facet`
**Purpose:** Introduction to FACET Type System (FTS)
**Concepts:** `@var_types`, type constraints, validation

### Source Code

```facet
@var_types
  username: "string"
  age: { type: "int", min: 0, max: 150 }
  score: { type: "float", min: 0.0, max: 100.0 }
  active: "bool"

@vars
  username: "Alice"
  age: 25
  score: 95.5
  active: true

@system
  role: "assistant"
  model: "gpt-4"

@user
  query: "Hello"
```

### What It Does

**Type Declarations:**
- `username`: Must be string
- `age`: Integer between 0-150
- `score`: Float between 0.0-100.0
- `active`: Boolean

**Validation:**
- Compile-time type checking
- Runtime constraint validation
- Error reporting with F451-F453 codes

### Type Checking Process

1. **Parse `@var_types`:** Define expected types
2. **Validate `@vars`:** Check assignments against types
3. **Runtime checks:** Additional constraints during execution

### Output

```json
{
  "system": {
    "role": "assistant",
    "model": "gpt-4"
  },
  "user": {
    "query": "Hello"
  },
  "variables": {
    "username": "Alice",
    "age": 25,
    "score": 95.5,
    "active": true
  }
}
```

### Key Learning Points

- **`@var_types` block:** Type declarations (optional but recommended)
- **Constraint objects:** `{ type: "int", min: 0, max: 150 }`
- **Static validation:** Errors caught at compile time
- **Type inference:** Types inferred when `@var_types` not specified

### Try It

```bash
# Valid compilation
fct build --input examples/types_test.facet

# Try invalid values to see type errors:
# age: 200  # F452: Constraint violation (max 150)
# score: "95.5"  # F451: Type mismatch (expected float, got string)
```

---

## Example 5: RAG Pipeline (`rag_pipeline.facet`)

**File:** `examples/rag_pipeline.facet`
**Purpose:** Retrieval-Augmented Generation pattern
**Concepts:** Document processing, context management, complex pipelines

### Source Code

```facet
@meta
  title: "RAG Question Answering System"
  version: "2.0"
  description: "Demonstrates retrieval-augmented generation pattern"

@vars
  # Raw retrieved documents (would come from vector DB in production)
  doc1: "  Python is a high-level programming language.  "
  doc2: "  Machine learning models require training data.  "
  doc3: "  Neural networks consist of layers of nodes.  "

  # User query processing
  raw_query: "  What is Python?  "
  query: $raw_query |> trim() |> lowercase()

  # Document processing pipeline: trim whitespace
  context_doc1: $doc1 |> trim()
  context_doc2: $doc2 |> trim()
  context_doc3: $doc3 |> trim()

  # Metadata
  retrieval_count: 3
  confidence_threshold: 0.75

@system
  role: "assistant"
  model: "gpt-4"
  temperature: 0.3
  max_tokens: 500
  instructions: "You are a knowledgeable assistant. Answer questions based on the provided context. If information is not in the context, say so."

@context
  # Retrieved documents that provide grounding for the answer
  documents: [
    $context_doc1,
    $context_doc2,
    $context_doc3
  ]
  retrieval_method: "semantic_search"
  retrieval_count: $retrieval_count

@user
  query: $query
  instruction: "Based on the provided documents, please answer the following question:"
```

### What It Does

**Document Processing:**
- Raw docs with extra whitespace
- Cleaned via `|> trim()` pipelines
- Organized into context array

**Query Processing:**
- User input cleaned and normalized
- Lower temperature for factual answers

**Context Management:**
- `@context` block for grounding data
- List of retrieved documents
- Retrieval metadata

### R-DAG Execution Order

1. `raw_query` → `query` (string processing)
2. `doc1` → `context_doc1` (document cleaning)
3. `doc2` → `context_doc2` (parallel processing)
4. `doc3` → `context_doc3` (parallel processing)
5. `documents` array assembled from cleaned docs

### Output

```json
{
  "system": {
    "role": "assistant",
    "model": "gpt-4",
    "temperature": 0.3,
    "max_tokens": 500,
    "instructions": "You are a knowledgeable assistant..."
  },
  "context": {
    "documents": [
      "Python is a high-level programming language.",
      "Machine learning models require training data.",
      "Neural networks consist of layers of nodes."
    ],
    "retrieval_method": "semantic_search",
    "retrieval_count": 3
  },
  "user": {
    "query": "what is python?",
    "instruction": "Based on the provided documents..."
  }
}
```

### Key Learning Points

- **`@context` block:** Grounding data for RAG
- **List literals:** `[item1, item2, item3]` syntax
- **Parallel processing:** Multiple independent pipelines
- **Temperature control:** Lower for factual answers
- **Complex data flow:** Documents → Context → User query

### Try It

```bash
# See RAG structure
fct run --input examples/rag_pipeline.facet --format pretty

# Experiment with different documents or queries
# Change temperature and see behavior differences
```

---

## Example 6: Advanced Features (`advanced_features.facet`)

**File:** `examples/advanced_features.facet`
**Purpose:** Complete FACET feature showcase
**Concepts:** All block types, nested structures, complex data types

### Source Code

```facet
@meta
  title: "FACET v2.0 Feature Showcase"
  version: "2.0.0"
  tags: ["demo", "showcase", "tutorial"]

@vars
  # Scalar values
  app_name: "MyApp"
  version_number: 2.5
  is_production: false
  max_retries: 3

  # String processing
  raw_input: "  Hello World  "
  processed: $raw_input |> trim() |> uppercase()

  # List literals
  supported_models: ["gpt-4", "gpt-3.5-turbo", "claude-3"]
  retry_delays: [1, 2, 5, 10]

  # Nested map
  config: {
    api: {
      endpoint: "https://api.example.com"
      timeout: 30
      retry: true
    }
    features: {
      streaming: true
      caching: false
    }
  }

@system
  role: "assistant"
  model: "gpt-4"
  temperature: 0.7
  max_tokens: 2000
  capabilities: ["Text generation", "Code analysis", "Data processing"]

@context
  application: $app_name
  version: $version_number
  settings: $config
  models: $supported_models

@user
  name: "Developer"
  role: "system_architect"
  query: "Analyze the system configuration and suggest improvements"
  metadata: {
    session_id: "abc123"
    timestamp: "2024-01-15T10:30:00Z"
    locale: "en-US"
  }

@assistant
  example_response: "I'll analyze the configuration systematically..."
  tone: "professional"
  format: "structured"
```

### What It Does

**Complete FACET Structure:**
- `@meta`: Documentation and metadata
- `@vars`: All data types and transformations
- `@system`: Full AI configuration
- `@context`: Application state and settings
- `@user`: Rich user input with metadata
- `@assistant`: Response guidance (optional)

**Data Type Showcase:**
- **Scalars:** string, int, float, bool
- **Lists:** Arrays of primitives
- **Maps:** Nested configuration objects
- **Pipelines:** Complex string transformations

### Advanced Patterns

**Deep Nesting:**
```facet
config: {
  api: {
    endpoint: "https://api.example.com"
    timeout: 30
  }
}
```

**Cross-Block References:**
- `@vars` data used in `@context`
- `@system` config affects behavior
- `@user` input drives responses

### Output

Complex JSON with all FACET features represented.

### Key Learning Points

- **All block types:** `@meta`, `@system`, `@context`, `@user`, `@assistant`
- **Complex data structures:** Deep nesting, arrays, objects
- **Cross-references:** Variables used across blocks
- **Rich configuration:** Full AI agent specification
- **Modular design:** Separated concerns (system, context, user)

### Try It

```bash
# Inspect the full AST
fct inspect --input examples/advanced_features.facet

# Run with different budgets
fct run --input examples/advanced_features.facet --budget 8192 --format pretty
```

---

## Example 7: Testing (`test_example.facet`)

**File:** `examples/test_example.facet`
**Purpose:** In-language testing with mocks and assertions
**Concepts:** `@test` blocks, mocking, assertions, telemetry

### Source Code

```facet
@vars
  user_query: @input(type="string")

@system
  role: "helpful weather assistant"

@user
  $user_query

@test "basic greeting"
  vars:
    user_query: "Do I need an umbrella today?"

  mock:
    WeatherAPI.get_current: { temp: 10, condition: "Rain" }

  assert:
    - output contains "umbrella"
    - output sentiment "helpful"
    - cost < 0.01

@test "weather check"
  vars:
    user_query: "What's the weather like?"

  mock:
    WeatherAPI.get_current: { temp: 25, condition: "Sunny" }

  assert:
    - output contains "sunny"
    - output not contains "umbrella"
    - tokens < 100
```

### What It Does

**Test Structure:**
- `@test` blocks define test cases
- `vars:` override input variables
- `mock:` simulate external dependencies
- `assert:` validate outputs and telemetry

**Test Execution:**
1. Override variables with test values
2. Apply mocks for deterministic behavior
3. Execute pipeline and capture results
4. Run assertions on output, cost, tokens, etc.

### Running Tests

```bash
# Run all tests
fct test --input examples/test_example.facet

# Run specific test
fct test --input examples/test_example.facet --filter "basic greeting"

# JSON output
fct test --input examples/test_example.facet --output json
```

### Key Learning Points

- **`@test` blocks:** In-language testing framework
- **Mocking:** Simulate external APIs for deterministic tests
- **Assertions:** Multiple types (contains, sentiment, cost, tokens)
- **Telemetry:** Test execution metrics and costs
- **Isolation:** Tests run independently

### Try It

```bash
# Run the test suite
fct test --input examples/test_example.facet --output summary

# See detailed results
fct test --input examples/test_example.facet --output verbose
```

---

## Common Patterns

### Variable Processing Pipeline

```facet
@vars
  # Input cleaning
  raw_input: "  messy USER input  "
  clean_input: $raw_input |> trim() |> lowercase() |> capitalize()

  # Data transformation
  items: ["item1", "item2", "item3"]
  processed_items: $items |> map("uppercase") |> unique()
```

### Configuration Management

```facet
@vars
  config: {
    api: {
      endpoint: "https://api.example.com"
      timeout: 30
      retries: 3
    }
    features: {
      streaming: true
      caching: true
    }
  }

  # Extract values for use
  api_url: $config.api.endpoint
  use_streaming: $config.features.streaming
```

### Context Building

```facet
@context
  # Build context from multiple sources
  user_info: {
    name: $user_name
    preferences: $user_prefs
  }

  # Document collections
  documents: [
    $retrieved_doc1,
    $retrieved_doc2,
    $generated_summary
  ]

  # Metadata
  timestamp: @current_timestamp()
  session_id: $session_id
```

### Error Handling in Tests

```facet
@test "error handling"
  vars:
    invalid_input: ""

  assert:
    - output contains "error"
    - cost < 0.05  # Even errors should be cheap
```

---

## Creating Your Own Examples

### Step-by-Step Guide

1. **Start with structure:**
   ```facet
   @meta
     title: "My Example"
     description: "What this demonstrates"

   @vars
     # Your variables here

   @system
     role: "assistant"
     model: "gpt-4"

   @user
     query: "Your prompt here"
   ```

2. **Add complexity gradually:**
   - Start with literals
   - Add variable references (`$var`)
   - Try simple lenses (`|> trim()`)
   - Add list/map structures
   - Include `@context` for grounding
   - Add `@test` blocks for validation

3. **Test incrementally:**
   ```bash
   # Start simple
   fct build --input my_example.facet

   # Add complexity
   fct run --input my_example.facet --budget 4096

   # Add tests
   fct test --input my_example.facet
   ```

### Best Practices

- **Use `@meta`** for documentation
- **Keep `@vars`** focused on data transformation
- **Put configuration** in `@system`
- **Use `@context`** for grounding data
- **Add `@test` blocks** for validation
- **Use constraints** in `@var_types`
- **Comment complex pipelines**

### Debugging Tips

- **Start with `fct inspect`** to see AST structure
- **Use `fct build`** to check syntax/types
- **Add `--verbose`** for detailed execution logs
- **Test small changes** incrementally
- **Use simple values** first, then complex structures

---

## Troubleshooting

### Common Issues

**"Variable not found" (F401):**
```
Problem: $undefined_var used before definition
Solution: Ensure variable is defined in @vars block
```

**"Type mismatch" (F451):**
```
Problem: Wrong type assigned to variable
Solution: Check @var_types declarations or use type inference
```

**"Tabs not allowed" (F002):**
```
Problem: Used tab characters for indentation
Solution: Replace tabs with spaces (2 or 4 space indentation)
```

**"Cyclic dependency" (F505):**
```
Problem: Variables reference each other circularly
Solution: Restructure dependencies or use different variable names
```

**"Gas exhausted" (F902):**
```
Problem: Computation too expensive
Solution: Simplify pipelines or increase gas_limit
```

### Getting Help

- **Check examples:** All examples are runnable and tested
- **Read docs:** `docs/tutorial.md` for learning path
- **Use inspect:** `fct inspect --input file.facet` for debugging
- **Test incrementally:** Build often, test small changes
- **Check constraints:** Use `@var_types` for validation

### Performance Tuning

- **Use `|> trim()` early** to reduce downstream processing
- **Cache expensive operations** in separate variables
- **Use appropriate token budgets** (start with 4096)
- **Profile with `--verbose`** to identify bottlenecks
- **Simplify pipelines** that don't add value

---

## Next Steps

🎯 **Apply What You've Learned:**
- **[06-cli.md](06-cli.md)** - Full command-line interface reference
- **[07-api-reference.md](07-api-reference.md)** - Rust API for programmatic usage
- **[08-lenses.md](08-lenses.md)** - Complete lens library reference

🔧 **Advanced Usage:**
- **[09-testing.md](09-testing.md)** - Write robust @test blocks
- **[10-performance.md](10-performance.md)** - Optimize for production
- **[11-security.md](11-security.md)** - Secure deployment practices

📚 **Resources:**
- **[examples/README.md](../examples/README.md)** - Quick examples reference
- **[faq.md](../faq.md)** - Common questions and answers
- **[12-errors.md](12-errors.md)** - Troubleshooting guide

---

This guide covers all FACET examples with detailed explanations and runnable code. Each example builds on the previous one, creating a complete learning path from basic concepts to advanced patterns.

**Ready to create your own FACET files? Start with the examples and experiment!** 🚀

🎯 **Apply What You've Learned:**
- **[06-cli.md](06-cli.md)** - Full command-line interface reference
- **[07-api-reference.md](07-api-reference.md)** - Rust API for programmatic usage
- **[08-lenses.md](08-lenses.md)** - Complete lens library reference

🔧 **Advanced Usage:**
- **[09-testing.md](09-testing.md)** - Write robust @test blocks
- **[10-performance.md](10-performance.md)** - Optimize for production
- **[11-security.md](11-security.md)** - Secure deployment practices

📚 **Resources:**
- **[examples/README.md](../examples/README.md)** - Quick examples reference
- **[faq.md](../faq.md)** - Common questions and answers
- **[12-errors.md](12-errors.md)** - Troubleshooting guide

---

This guide covers all FACET examples with detailed explanations and runnable code. Each example builds on the previous one, creating a complete learning path from basic concepts to advanced patterns.

**Ready to create your own FACET files? Start with the examples and experiment!** 🚀
