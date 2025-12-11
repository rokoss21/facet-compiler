# FACET Examples

This directory contains example FACET files demonstrating various features and use cases.

## Available Examples

### 1. `basic_prompt.facet` - Simple AI Assistant

A straightforward example showing:
- Variable declarations (`@vars`)
- String pipeline transformations (`trim()`, `uppercase()`, `lowercase()`)
- System and user blocks with metadata
- Variable substitution

**Try it:**
```bash
# Build (parse + validate)
cargo run -- build --input examples/basic_prompt.facet

# Run full pipeline with verbose output
cargo run -- -v run --input examples/basic_prompt.facet --budget 4096 --format pretty
```

### 2. `rag_pipeline.facet` - RAG (Retrieval-Augmented Generation)

Demonstrates a realistic RAG pattern:
- Document processing pipelines
- Context management with `@context` block
- List literals for multiple documents
- Metadata and retrieval configuration
- Lower temperature for factual answers

**Try it:**
```bash
cargo run -- build --input examples/rag_pipeline.facet
cargo run -- run --input examples/rag_pipeline.facet --budget 8192
```

### 3. `advanced_features.facet` - Complete Feature Showcase

A comprehensive example demonstrating:
- **Scalar types:** int, float, bool, null
- **String pipelines:** complex transformations
- **List literals:** `[1, 2, 3]`, `["a", "b", "c"]`
- **Nested maps:** configuration objects with deep nesting
- **Escaped strings:** JSON, file paths
- **Multiple block types:** `@meta`, `@system`, `@context`, `@user`, `@assistant`
- **Variable references:** `$var_name`

**Try it:**
```bash
# Inspect the AST
cargo run -- inspect --input examples/advanced_features.facet

# Run with custom budgets
cargo run -- run --input examples/advanced_features.facet \
  --budget 8192 \
  --context-budget 20000 \
  --format pretty
```

## CLI Commands

### `build` - Parse and Validate
```bash
cargo run -- build --input <FILE>
cargo run -- -v build --input <FILE>  # verbose mode
```

### `inspect` - View AST
```bash
cargo run -- inspect --input <FILE>
```

### `run` - Full Pipeline
```bash
cargo run -- run --input <FILE> [OPTIONS]

Options:
  -b, --budget <BUDGET>              Token budget for context window [default: 4096]
  -c, --context-budget <BUDGET>      R-DAG execution budget [default: 10000]
  -f, --format <FORMAT>              Output format: json or pretty [default: json]
```

## Common Patterns

### Variable Processing
```facet
@vars
  raw_input: "  messy text  "
  cleaned: $raw_input |> trim() |> lowercase()
```

### Nested Configuration
```facet
@vars
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

### List Handling
```facet
@vars
  models: ["gpt-4", "gpt-3.5-turbo"]
  numbers: [1, 2, 3, 4, 5]
```

### Document Contexts
```facet
@context
  documents: [$doc1, $doc2, $doc3]
  retrieval_method: "semantic_search"
```

## Creating Your Own Examples

1. Start with `@meta` for documentation
2. Define variables in `@vars`
3. Configure system behavior in `@system`
4. Add user input in `@user`
5. Test with `build` first, then `run`

## Next Steps

- Read [docs/cli.md](../docs/cli.md) for complete CLI reference
- See [docs/lenses.md](../docs/lenses.md) for available lens functions
- Check [facet2-specification.md](../facet2-specification.md) for full language specification
