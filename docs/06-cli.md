---
---
# 06. FACET CLI Reference

**Reading Time:** 15-20 minutes | **Difficulty:** Beginner | **Previous:** [05-examples-guide.md](05-examples-guide.md) | **Next:** [07-api-reference.md](07-api-reference.md)

Complete command-line interface documentation for the FACET v2.0 Compiler (`fct`).

**Specification Compliance:** Implements all commands defined in FACET v2.0 specification Section 16, plus additional features for enhanced usability.

---

## Table of Contents

- [Overview](#overview)
- [Global Options](#global-options)
- [Commands](#commands)
  - [build](#build)
  - [inspect](#inspect)
  - [run](#run)
  - [test](#test)
  - [codegen](#codegen)
- [Output Formats](#output-formats)
- [Exit Codes](#exit-codes)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

---

## Overview

```bash
fct [OPTIONS] <COMMAND>
```

The FACET compiler provides five main commands:
- `build` - Parse, resolve, and validate FACET documents
- `inspect` - View the parsed Abstract Syntax Tree (AST)
- `run` - Execute the full compilation pipeline and render output
- `test` - Run @test blocks and generate test reports
- `codegen` - Generate SDKs from FACET interfaces (TypeScript, Python, Rust)

## Global Options

### `-v, --verbose`
Enable verbose output with detailed pipeline information.

```bash
fct -v build --input myfile.facet
fct --verbose run --input myfile.facet
```

**Output examples:**
```
[INFO] Parsing: examples/basic_prompt.facet
[INFO] Parsed 4 blocks
[INFO] Resolving imports...
```

### `--no-progress`
Disable progress bars and spinners for CI/CD environments.

```bash
fct --no-progress build --input myfile.facet
```

### `--json-logs`
Output logs in JSON format for structured logging.

```bash
fct --json-logs build --input myfile.facet
```
[INFO] Validating types...
[INFO] All checks passed
```

### `-h, --help`
Display help information for the CLI or specific command.

```bash
fct --help
fct build --help
fct run --help
```

### `-V, --version`
Display the compiler version.

```bash
fct --version
```

---

## Commands

### `build` - Parse and Validate

Parse, resolve imports, and validate a FACET document without executing it.

**Usage:**
```bash
fct build --input <FILE>
```

**Options:**
- `-i, --input <FILE>` - Input FACET file path (required)

**Examples:**
```bash
# Basic validation
fct build --input examples/basic_prompt.facet

# With verbose output
fct -v build --input examples/rag_pipeline.facet
```

**Success output:**
```
✓ Build successful
```

**Error output:**
```json
{
  "error": "F003: Parse error...",
  "code": "F003"
}
```

---

### `inspect` - View AST

Display the parsed Abstract Syntax Tree in debug format.

**Usage:**
```bash
fct inspect --input <FILE>
```

**Options:**
- `-i, --input <FILE>` - Input FACET file path (required)

**Examples:**
```bash
fct inspect --input examples/basic.facet
```

**Output format:**
```rust
FacetDocument {
    blocks: [
        System(
            FacetBlock {
                name: "system",
                body: [
                    KeyValue {
                        key: "role",
                        value: String("assistant"),
                        ...
                    }
                ],
                ...
            }
        ),
        ...
    ],
    span: Span { ... }
}
```

**Use cases:**
- Debugging parser issues
- Understanding document structure
- Learning FACET AST representation

---

### `run` - Full Pipeline Execution

Execute the complete FACET compilation pipeline:
1. **Parse** - Convert FACET syntax to AST
2. **Resolve** - Resolve imports and merge blocks
3. **Validate** - Type-check using Facet Type System (FTS)
4. **Compute** - Execute R-DAG (Reactive Dependency Graph)
5. **Allocate** - Apply Token Box Model for context budgeting
6. **Render** - Generate canonical JSON output

**Usage:**
```bash
fct run --input <FILE> [OPTIONS]
```

**Options:**

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--input` | `-i` | PATH | required | Input FACET file |
| `--budget` | `-b` | usize | 4096 | Token budget for context window |
| `--context-budget` | `-c` | usize | 10000 | Execution context budget for R-DAG |
| `--format` | `-f` | string | json | Output format: `json` or `pretty` |

**Examples:**

```bash
# Basic execution with defaults (budget: 4096, ctx: 10000)
fct run --input examples/basic_prompt.facet

# Custom token budget for large context
fct run --input examples/rag_pipeline.facet --budget 8192

# Pretty-printed JSON output
fct run --input examples/advanced_features.facet --format pretty

# Full configuration with verbose logging
fct -v run \\
  --input examples/rag_pipeline.facet \\
  --budget 16384 \\
  --context-budget 50000 \\
  --format pretty
```

**Output format:**

```json
{
  "metadata": {
    "name": "facet_document",
    "version": "2.0",
    "created_at": "2025-12-09T08:51:33Z",
    "total_tokens": 486,
    "budget": 4096,
    "overflow": 0
  },
  "system": [ /* system messages */ ],
  "tools": [ /* tool definitions */ ],
  "examples": [ /* few-shot examples */ ],
  "history": [ /* conversation history */ ],
  "user": [ /* user messages */ ],
  "assistant": [ /* assistant messages */ ]
}
```

---

### `test` - Run Test Blocks

Execute `@test` blocks defined in FACET documents and generate test reports.

**Usage:**
```bash
fct test --input <FILE> [OPTIONS]
```

**Options:**

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--input` | `-i` | `PATH` | - | Input FACET file path |
| `--filter` | `-f` | `STRING` | - | Filter tests by name pattern |
| `--output` | - | `STRING` | `summary` | Output format: `summary`, `verbose`, `json` |
| `--budget` | - | `NUMBER` | `4096` | Token budget for test execution |
| `--gas-limit` | - | `NUMBER` | `10000` | Gas limit for test execution |

**Examples:**

```bash
# Run all tests with summary output
fct test --input examples/test_suite.facet

# Run specific test by pattern
fct test --input examples/test_suite.facet --filter "basic_*"

# Verbose output with custom budgets
fct test --input examples/test_suite.facet \
  --filter "integration_*" \
  --output verbose \
  --budget 8192 \
  --gas-limit 20000

# JSON output for CI/CD integration
fct test --input examples/test_suite.facet --output json
```

**Output formats:**

**Summary (default):**
```
Test Results: examples/test_suite.facet
========================================
✅ basic_arithmetic (2ms)
✅ string_operations (1ms)
✅ lens_execution (3ms)
❌ failed_assertion (5ms) - Assertion failed: expected true, got false

Results: 3 passed, 1 failed
Time: 11ms
```

**Verbose:**
```
Running test: basic_arithmetic
✅ Assertion: result == 42
✅ Assertion: cost < 100
✅ Assertion: execution_time < 10ms

Running test: failed_assertion
❌ Assertion: result == "expected" (got "actual")
Details: Expected string "expected", got "actual"
```

**JSON:**
```json
{
  "file": "examples/test_suite.facet",
  "results": [
    {
      "name": "basic_arithmetic",
      "status": "passed",
      "duration_ms": 2,
      "assertions": 3
    },
    {
      "name": "failed_assertion",
      "status": "failed",
      "duration_ms": 5,
      "error": "Assertion failed: expected true, got false"
    }
  ],
  "summary": {
    "total": 4,
    "passed": 3,
    "failed": 1,
    "time_ms": 11
  }
}
```

---

### `codegen` - Generate SDKs

Generate multi-language SDKs from FACET interface definitions.

**Usage:**
```bash
fct codegen --input <FILE> --output <DIR> [OPTIONS]
```

**Options:**

| Option | Short | Type | Default | Description |
|--------|-------|------|---------|-------------|
| `--input` | `-i` | `PATH` | - | Input FACET file with interfaces |
| `--output` | `-o` | `PATH` | - | Output directory for generated code |
| `--language` | `-l` | `STRING` | `typescript` | Target language: `typescript`, `python`, `rust` |
| `--name` | - | `STRING` | - | SDK name (default: derived from input file) |

**Examples:**

```bash
# Generate TypeScript SDK
fct codegen --input api.facet --output ./sdk --language typescript --name MyAPI

# Generate Python SDK with default name
fct codegen --input api.facet --output ./sdk/python --language python

# Generate Rust SDK
fct codegen --input api.facet --output ./sdk/rust --language rust
```

**Supported languages:**

**TypeScript:**
- Generates typed interfaces and classes
- Includes runtime validation
- Compatible with Node.js and browsers

**Python:**
- Generates Pydantic models and client classes
- Includes async/await support
- Compatible with Python 3.8+

**Rust:**
- Generates Serde-compatible structs
- Includes builder patterns
- Compatible with Rust 1.60+

**Output structure:**
```
output/
├── typescript/
│   ├── src/
│   │   ├── interfaces.ts
│   │   ├── client.ts
│   │   └── index.ts
│   ├── package.json
│   ├── tsconfig.json
│   └── README.md
├── python/
│   ├── src/
│   │   ├── __init__.py
│   │   ├── models.py
│   │   └── client.py
│   ├── setup.py
│   ├── requirements.txt
│   └── README.md
└── rust/
    ├── src/
    │   ├── lib.rs
    │   ├── models.rs
    │   └── client.rs
    ├── Cargo.toml
    └── README.md
```

---

## Parameter Details

### Token Budget (`--budget`)

Controls the context window size for the Token Box Model.

**Purpose:** Limits the total number of tokens allocated across all sections (system, user, context, etc.)

**Typical values:**
- `4096` - GPT-3.5-Turbo, small contexts
- `8192` - GPT-4, medium contexts
- `16384` - GPT-4-32k, large contexts
- `32768` - Claude 2, very large contexts
- `128000` - Claude 3 Opus, extended contexts

**Example:**
```bash
# For GPT-4 Turbo with 128k context
fct run --input myfile.facet --budget 128000
```

### Context Budget (`--context-budget`)

Controls the execution budget for the R-DAG (Reactive Dependency Graph) engine.

**Purpose:** Prevents infinite loops and runaway computations during variable resolution and lens execution.

**Default:** 10000 (sufficient for most use cases)

**When to increase:**
- Deep pipeline chains (`a |> b |> c |> d |> ...`)
- Many interdependent variables
- Complex recursive computations
- Large documents with 100+ variables

**Example:**
```bash
# For complex documents with many dependencies
fct run --input complex.facet --context-budget 50000
```

### Output Format (`--format`)

Controls JSON output formatting.

**Options:**
- `json` (default) - Compact, single-line JSON
- `pretty` - Pretty-printed with indentation

**json format:**
```json
{"metadata":{"name":"facet_document","version":"2.0"},...}
```

**pretty format:**
```json
{
  "metadata": {
    "name": "facet_document",
    "version": "2.0"
  },
  ...
}
```

**Use cases:**
- `json` - Piping to other tools, production use
- `pretty` - Debugging, human reading, development

---

## Error Codes

FACET uses standardized error codes:

| Code | Category | Description |
|------|----------|-------------|
| F001 | Parser | Invalid indentation |
| F002 | Parser | Tab characters forbidden (use 2 spaces) |
| F003 | Parser | Parse error or unclosed structure |
| F451 | Validator | Type error |
| F452 | Validator | Forward reference |
| F802 | Validator | Unknown lens function |

**Error output format:**
```json
{
  "error": "F003: Parse error at line 5, column 3: unexpected token",
  "code": "F003"
}
```

---

## Exit Codes

- `0` - Success
- `1` - Error (see JSON error output on stderr)

---

## Environment Variables

Currently, FACET does not use environment variables. All configuration is done via command-line flags.

---

## Piping and Integration

### Pipe to jq for JSON processing
```bash
fct run --input myfile.facet | jq '.metadata'
fct run --input myfile.facet | jq '.user[] | .content'
```

### Save output to file
```bash
fct run --input myfile.facet > output.json
fct run --input myfile.facet --format pretty > output.pretty.json
```

### Check exit code
```bash
if fct build --input myfile.facet; then
  echo "Valid FACET file"
else
  echo "Invalid FACET file"
fi
```

### Batch processing
```bash
for file in examples/*.facet; do
  echo "Processing $file..."
  fct build --input "$file" || echo "Failed: $file"
done
```

---

## Tips and Best Practices

### 1. Always validate before running
```bash
fct build --input myfile.facet && fct run --input myfile.facet
```

### 2. Use verbose mode for debugging
```bash
fct -v run --input myfile.facet
```

### 3. Start with small budgets, increase as needed
```bash
# Start
fct run --input myfile.facet --budget 2048

# If overflow > 0 in output, increase
fct run --input myfile.facet --budget 4096
```

### 4. Use pretty format during development
```bash
fct run --input myfile.facet --format pretty
```

### 5. Inspect AST when parser errors are unclear
```bash
fct inspect --input myfile.facet
```

---

## Troubleshooting

### "F001: Invalid indentation"
**Problem:** FACET requires exactly 2 spaces per indentation level.

**Solution:**
```facet
# Wrong
@system
    role: "assistant"  # 4 spaces

# Correct
@system
  role: "assistant"    # 2 spaces
```

### "F002: Tab characters forbidden"
**Problem:** Tabs are not allowed; use spaces.

**Solution:** Configure your editor to insert spaces instead of tabs.

### "F003: Parse error"
**Problem:** Syntax error in FACET document.

**Solution:**
1. Use `inspect` to find the error location
2. Check for:
   - Unclosed strings
   - Unclosed braces `{` or brackets `[`
   - Missing colons `:` after keys
   - Invalid characters

### "overflow > 0" in metadata
**Problem:** Content exceeded token budget.

**Solution:**
```bash
# Increase budget
fct run --input myfile.facet --budget 8192
```

### High execution time
**Problem:** Complex R-DAG computation.

**Solution:**
- Simplify variable dependencies
- Reduce pipeline chain length
- Increase `--context-budget` if hitting limit

---

## See Also

## Next Steps

🎯 **Continue Learning:**
- **[07-api-reference.md](07-api-reference.md)** - Rust API for programmatic usage
- **[08-lenses.md](08-lenses.md)** - Lens functions reference
- **[09-testing.md](09-testing.md)** - @test blocks usage

🔧 **Advanced Topics:**
- **[10-performance.md](10-performance.md)** - CLI performance optimization
- **[11-security.md](11-security.md)** - Secure CLI usage
- **[13-import-system.md](13-import-system.md)** - Import system details

📚 **References:**
- **[01-quickstart.md](01-quickstart.md)** - Quick start guide
- **[05-examples-guide.md](05-examples-guide.md)** - Practical examples
- **[12-errors.md](12-errors.md)** - CLI error codes

---

## 👤 Author

**Emil Rokossovskiy**  
*AI & Platform Engineer. Creator of FACET ecosystem 🚀*

- **GitHub:** [@rokoss21](https://github.com/rokoss21)
- **Compiler Repo:** [github.com/rokoss21/facet-compiler](https://github.com/rokoss21/facet-compiler)
- **Email:** ecsiar@gmail.com

---

**Related Documentation:**
- **[08-lenses.md](08-lenses.md)** - Available lens functions
- **[01-quickstart.md](01-quickstart.md)** - Getting started tutorial
- **[facet2-specification.md](../facet2-specification.md)** - Full language specification
- **[examples/README.md](../examples/README.md)** - Example files

fct run --input myfile.facet --budget 8192
```

### High execution time
**Problem:** Complex R-DAG computation.

**Solution:**
- Simplify variable dependencies
- Reduce pipeline chain length
- Increase `--context-budget` if hitting limit

---

## See Also

## Next Steps

🎯 **Continue Learning:**
- **[07-api-reference.md](07-api-reference.md)** - Rust API for programmatic usage
- **[08-lenses.md](08-lenses.md)** - Lens functions reference
- **[09-testing.md](09-testing.md)** - @test blocks usage

🔧 **Advanced Topics:**
- **[10-performance.md](10-performance.md)** - CLI performance optimization
- **[11-security.md](11-security.md)** - Secure CLI usage
- **[13-import-system.md](13-import-system.md)** - Import system details

📚 **References:**
- **[01-quickstart.md](01-quickstart.md)** - Quick start guide
- **[05-examples-guide.md](05-examples-guide.md)** - Practical examples
- **[12-errors.md](12-errors.md)** - CLI error codes

---

## 👤 Author

**Emil Rokossovskiy**  
*AI & Platform Engineer. Creator of FACET ecosystem 🚀*

- **GitHub:** [@rokoss21](https://github.com/rokoss21)
- **Compiler Repo:** [github.com/rokoss21/facet-compiler](https://github.com/rokoss21/facet-compiler)
- **Email:** ecsiar@gmail.com

---

**Related Documentation:**
- **[08-lenses.md](08-lenses.md)** - Available lens functions
- **[01-quickstart.md](01-quickstart.md)** - Getting started tutorial
- **[facet2-specification.md](../facet2-specification.md)** - Full language specification
- **[examples/README.md](../examples/README.md)** - Example files
