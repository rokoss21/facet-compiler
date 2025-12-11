# 01. FACET v2.0 Quick Start Guide

**Reading Time:** 5 minutes | **Difficulty:** Beginner | **Next:** [02-tutorial.md](02-tutorial.md)

Get up and running with FACET in 5 minutes.

---

## Table of Contents

- [What is FACET?](#what-is-facet)
- [Installation](#installation)
- [Your First FACET File](#your-first-facet-file)
- [Running Your First Agent](#running-your-first-agent)
- [Next Steps](#next-steps)

---

## What is FACET?

**FACET** (Formal Agent Configuration & Execution Template) is a deterministic compiler for AI agent behavior. It transforms `.facet` files into canonical JSON for AI models with:

- ✅ **Type safety** - Catch errors before runtime
- ✅ **Deterministic** - Same input → same output, always
- ✅ **Pipeline transformations** - Compose data processing with `|>`
- ✅ **Token budgeting** - Precise context window control

## Installation

### Prerequisites
- Rust 1.70+ ([install here](https://rustup.rs/))

### Build
```bash
git clone https://github.com/yourorg/facet.git
cd facet
cargo build --release
```

### Verify
```bash
cargo run -- --version
# Output: facet-fct 0.1.0-beta
```

---

## Your First FACET File

Create `hello.facet`:

```facet
@system
  role: "assistant"
  model: "gpt-4"
  instructions: "You are a helpful assistant."

@user
  query: "Hello, who are you?"
```

### Validate
```bash
cargo run -- build --input hello.facet
# Output: ✓ Build successful
```

### Execute
```bash
cargo run -- run --input hello.facet --format pretty
```

**Output:**
```json
{
  "metadata": {
    "version": "2.0",
    "total_tokens": 156,
    "budget": 4096
  },
  "system": [
    {
      "role": "system",
      "content": "{\"role\":\"assistant\",\"model\":\"gpt-4\",...}",
      "tokens": 89
    }
  ],
  "user": [
    {
      "role": "user",
      "content": "{\"query\":\"Hello, who are you?\"}",
      "tokens": 67
    }
  ]
}
```

---

## Core Concepts

### 1. Blocks

FACET documents are composed of blocks:

```facet
@meta      # Metadata (version, author, description)
@vars      # Variables and computations
@system    # System configuration
@context   # Additional context for the model
@user      # User input
@assistant # Assistant examples (optional)
```

### 2. Variables and Pipelines

Variables are declared in `@vars` and can use transformation pipelines:

```facet
@vars
  # Simple variable
  user_name: "Alice"

  # Pipeline transformation
  raw_input: "  hello WORLD  "
  clean: $raw_input |> trim() |> lowercase()
  # Result: "hello world"

  # Variable reference
  greeting: $clean |> uppercase()
  # Result: "HELLO WORLD"
```

**Pipeline operator:** `|>` chains transformations left-to-right

**Available lenses:** See [lenses.md](./lenses.md) for all 14 built-in functions

### 3. Data Types

FACET supports standard types:

```facet
@vars
  # Strings
  name: "Alice"
  path: "C:\\Users\\file.txt"  # Escaped backslash
  json: "{\"key\": \"value\"}"   # Escaped quotes

  # Numbers
  count: 42
  pi: 3.14159
  scientific: 1.23e10

  # Booleans
  is_active: true
  is_admin: false

  # Null
  optional: null

  # Lists
  tags: ["ai", "ml", "nlp"]
  numbers: [1, 2, 3, 4, 5]

  # Maps (nested objects)
  config: {
    api: {
      endpoint: "https://api.example.com"
      timeout: 30
    }
    features: {
      streaming: true
    }
  }
```

### 4. Indentation Rules

**CRITICAL:** FACET uses **2-space indentation** (no tabs)

```facet
# ✅ Correct
@system
  role: "assistant"
  model: "gpt-4"

# ❌ Wrong (4 spaces)
@system
    role: "assistant"

# ❌ Wrong (tabs)
@system
→ role: "assistant"
```

**Tip:** Configure your editor to use 2 spaces for `.facet` files

---

## Example Workflows

### Example 1: Text Processing

```facet
@vars
  raw_feedback: "  GREAT PRODUCT!  "
  processed: $raw_feedback |> trim() |> lowercase()

@user
  feedback: $processed
  # Result: "great product!"
```

### Example 2: Configuration

```facet
@meta
  title: "Customer Support Bot"
  version: "1.0"

@vars
  company_name: "Acme Corp"
  support_email: "support@acme.com"

@system
  role: "assistant"
  model: "gpt-4"
  temperature: 0.7
  instructions: "You are a customer support agent for Acme Corp. Be helpful and professional."

@user
  query: "How do I reset my password?"
```

**Run:**
```bash
cargo run -- run --input support_bot.facet --budget 8192
```

### Example 3: RAG Pipeline

```facet
@vars
  # Retrieved documents from vector DB
  doc1: "Python is a programming language."
  doc2: "Machine learning uses neural networks."
  doc3: "Data science involves statistics."

  # Process query
  user_query: "  what is python?  " |> trim() |> lowercase()

  # Clean documents
  context_docs: [
    $doc1 |> trim(),
    $doc2 |> trim(),
    $doc3 |> trim()
  ]

@system
  role: "assistant"
  model: "gpt-4"
  temperature: 0.3
  instructions: "Answer based on provided context. Cite sources."

@context
  documents: $context_docs
  query: $user_query

@user
  question: $user_query
```

---

## CLI Commands

### Build (Validate)
```bash
# Parse and validate
cargo run -- build --input myfile.facet

# With verbose output
cargo run -- -v build --input myfile.facet
```

### Inspect (View AST)
```bash
# See parsed structure
cargo run -- inspect --input myfile.facet
```

### Run (Full Pipeline)
```bash
# Execute and render
cargo run -- run --input myfile.facet

# Custom token budget
cargo run -- run --input myfile.facet --budget 16384

# Pretty-printed output
cargo run -- run --input myfile.facet --format pretty

# Verbose execution log
cargo run -- -v run --input myfile.facet
```

---

## Common Errors

### F001: Invalid Indentation
```facet
# ❌ Wrong
@system
    role: "assistant"  # 4 spaces

# ✅ Correct
@system
  role: "assistant"    # 2 spaces
```

### F002: Tabs Forbidden
```
Error: F002: Tab characters forbidden (use 2 spaces)
```

**Fix:** Convert tabs to spaces in your editor

### F003: Parse Error
```facet
# ❌ Unclosed string
@vars
  name: "Alice

# ✅ Correct
@vars
  name: "Alice"
```

### Type Mismatch in Pipeline
```facet
# ❌ Wrong type
@vars
  number: 42
  result: $number |> trim()  # Error: trim() needs string

# ✅ Correct
@vars
  text: "hello"
  result: $text |> trim()
```

---

## Next Steps

### Learn More
- 📖 [CLI Reference](./cli.md) - Complete command documentation
- 📖 [Lens Reference](./lenses.md) - All 14 built-in transformations
- 📖 [Language Spec](../facet2-specification.md) - Full FACET specification

### Explore Examples
- 🔍 [Basic Prompt](../examples/basic_prompt.facet) - Simple assistant
- 🔍 [RAG Pipeline](../examples/rag_pipeline.facet) - Retrieval-augmented generation
- 🔍 [Advanced Features](../examples/advanced_features.facet) - All language features

### Try These
1. **Modify examples** - Change variables, add pipelines
2. **Create your own** - Start with `@vars` and `@user`
3. **Use verbose mode** - See what happens: `cargo run -- -v run --input myfile.facet`
4. **Experiment with budgets** - Try different `--budget` values

---

## Tips & Best Practices

### ✅ Do
- Use 2-space indentation (never tabs)
- Validate with `build` before `run`
- Use `trim()` to clean user input
- Chain lenses with `|>` for clean transformations
- Use verbose mode (`-v`) when debugging
- Start with small token budgets, increase if needed
- Use `@meta` to document your files

### ❌ Don't
- Don't use tabs for indentation
- Don't skip validation before execution
- Don't forget to escape special characters (`\"`, `\\`)
- Don't hardcode values - use `@vars` for flexibility
- Don't ignore error codes (F001, F002, F003)
- Don't use extremely large budgets unnecessarily

---

## Getting Help

### Documentation
- Run `cargo run -- --help` for CLI help
- Run `cargo run -- run --help` for command-specific help

### Debugging
```bash
# 1. Validate syntax
cargo run -- build --input myfile.facet

# 2. Inspect AST structure
cargo run -- inspect --input myfile.facet

# 3. Run with verbose logging
cargo run -- -v run --input myfile.facet
```

### Common Issues
| Problem | Solution |
|---------|----------|
| `F001` error | Check indentation (must be 2 spaces) |
| `F002` error | Remove tabs, use spaces |
| `F003` error | Check for unclosed strings, brackets, braces |
| Token overflow | Increase `--budget` value |
| Type errors | Check lens input types (see lenses.md) |

---

## Quick Reference Card

```facet
# FACET Syntax Cheat Sheet

# Blocks
@meta      # Metadata
@vars      # Variables
@system    # System config
@context   # Additional context
@user      # User input
@assistant # Examples

# Data Types
string: "text"
int: 42
float: 3.14
bool: true
null: null
list: [1, 2, 3]
map: {key: "value"}

# Pipelines
$var |> lens1() |> lens2(arg) |> lens3()

# Common Lenses
|> trim()              # Remove whitespace
|> lowercase()         # To lowercase
|> uppercase()         # To uppercase
|> split(",")          # Split by delimiter
|> replace("a", "b")   # Replace substring
|> default("fallback") # Fallback value
|> json(2)             # Format as JSON

# CLI
build   --input FILE           # Validate
inspect --input FILE           # View AST
run     --input FILE           # Execute
        --budget 4096          # Token budget
        --format pretty        # Pretty JSON
-v                             # Verbose mode
```

## Next Steps

🎯 **Continue Learning:**
- **[02-tutorial.md](02-tutorial.md)** - Complete step-by-step tutorial
- **[05-examples-guide.md](05-examples-guide.md)** - Practical examples with explanations
- **[06-cli.md](06-cli.md)** - Full command-line reference

🔧 **Deep Dives:**
- **[03-architecture.md](03-architecture.md)** - System architecture overview
- **[04-type-system.md](04-type-system.md)** - Type system reference
- **[07-api-reference.md](07-api-reference.md)** - Rust API documentation

📚 **Resources:**
- **[faq.md](../faq.md)** - Frequently asked questions
- **[12-errors.md](12-errors.md)** - Error codes and troubleshooting
- **[README.md](../README.md)** - Project overview and status

---

**Ready to build deterministic AI agents?** Start with [examples/basic_prompt.facet](../examples/basic_prompt.facet)!


# CLI
build   --input FILE           # Validate
inspect --input FILE           # View AST
run     --input FILE           # Execute
        --budget 4096          # Token budget
        --format pretty        # Pretty JSON
-v                             # Verbose mode
```

## Next Steps

🎯 **Continue Learning:**
- **[02-tutorial.md](02-tutorial.md)** - Complete step-by-step tutorial
- **[05-examples-guide.md](05-examples-guide.md)** - Practical examples with explanations
- **[06-cli.md](06-cli.md)** - Full command-line reference

🔧 **Deep Dives:**
- **[03-architecture.md](03-architecture.md)** - System architecture overview
- **[04-type-system.md](04-type-system.md)** - Type system reference
- **[07-api-reference.md](07-api-reference.md)** - Rust API documentation

📚 **Resources:**
- **[faq.md](../faq.md)** - Frequently asked questions
- **[12-errors.md](12-errors.md)** - Error codes and troubleshooting
- **[README.md](../README.md)** - Project overview and status

---

**Ready to build deterministic AI agents?** Start with [examples/basic_prompt.facet](../examples/basic_prompt.facet)!
