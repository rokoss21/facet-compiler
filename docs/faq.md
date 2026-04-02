---
permalink: /faq.html
---

# FACET v2.1.3 Frequently Asked Questions
**Version:** 0.1.2
**Last Updated:** 2026-04-02
**Status:** Spec-aligned guidance

## Table of Contents

- [General Questions](#general-questions)
- [Installation & Setup](#installation--setup)
- [Language Syntax](#language-syntax)
- [Type System](#type-system)
- [Execution Model](#execution-model)
- [Performance](#performance)
- [Security](#security)
- [Integration](#integration)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)

---

## General Questions

### What is FACET?

**FACET** (Formal Agent Configuration & Execution Template) is a **Neural Architecture Description Language (NADL)** designed to define, validate, and execute AI agent behaviors in a deterministic, resource-bounded, and type-safe manner.

FACET is best understood as an execution control layer: it does not replace model inference, it constrains system behavior around model calls with deterministic compilation and explicit policy boundaries.

### How is FACET v2.1.3 different from v1.x?

| Aspect | FACET v1.x | FACET v2.1.3 |
|--------|------------|------------|
| **Architecture** | Template system | Compiled language |
| **Execution** | Runtime interpretation | Compile-time optimization |
| **Type Safety** | None | Full static typing (FTS) |
| **Performance** | Variable | Deterministic, optimized |
| **Reproducibility** | Platform-dependent | Deterministic canonical assembly |
| **Resource Control** | None | Token budgeting, gas limits |

### Why "Neural Architecture Description Language"?

FACET describes the **architecture** of AI agent behavior - the structure, data flow, and transformation pipeline - rather than just templating text. It's a **declarative language** for defining AI system blueprints.

### Is FACET open source?

Yes! FACET v2.1.3 is licensed under MIT OR Apache-2.0 and available on GitHub. The compiler is written in Rust for performance and safety.

### What does FACET NOT solve?

- FACET does not make LLM inference output deterministic.
- FACET does not guarantee correctness of third-party APIs.
- FACET does not replace application/business logic.
- FACET does not provide legal certification by itself.

See also: [Execution Model](15-execution-model.html).

---

## Installation & Setup

### How do I install FACET?

```bash
# Install Rust (required)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/rokoss21/facet-compiler
cd facet-compiler
cargo build --release

# Add to PATH
export PATH="$PWD/target/release:$PATH"
```

### Can I use FACET without Rust?

Yes! FACET provides multiple deployment options:

**WebAssembly (Browser/Node.js):**
```javascript
import { compile } from 'facet-fct';
const result = compile(facetSource);
```

**Docker:**
```bash
docker run facet/fct compile --input agent.facet
```

**Pre-built binaries** for Linux, macOS, and Windows.

### What's the minimum Rust version?

FACET requires **Rust 1.70+**. We recommend using `rustup` for easy version management:

```bash
rustup update
rustup install 1.70
rustup default 1.70
```

---

## Language Syntax

### How do I define a basic agent?

```facet
@system
  role: "assistant"
  model: "gpt-5.2"
  temperature: 0.7

@user
  query: "Hello, world!"

@vars
  greeting: "Hello from FACET!"
```

### What's the difference between @vars and @user?

- **`@user`**: Input data from the user (queries, context, parameters)
- **`@vars`**: Computed variables and transformations
- **`@system`**: Agent configuration and instructions

Variables in `@vars` can reference `@user` data but not vice versa.

### How do imports work?

```facet
@import "common/prompts.facet"
@import "./local/utils.facet"

@system
  role: $common_role  # From imported file
```

Imports are resolved at compile time with cycle detection (F602).

### What's the difference between `:` and `=`?

```facet
@vars
  # Key-value pairs (most common)
  name: "Alice"
  age: 25

  # Normative v2.1.3 style uses ':'
  city: "Minsk"
```

In FACET v2.1.3 specification, map entries use `key: value`. Treat `:` as the canonical syntax.

### How do I write multi-line strings?

```facet
@system
  instructions: "
    You are a helpful AI assistant.
    Always be polite and accurate.
    Use markdown formatting.
  "
```

Multi-line strings preserve indentation and newlines.

### Can I use comments?

```facet
# This is a comment
@system
  content: "You are a helpful assistant."  # Inline comment
```

FACET supports line comments with `#`.

---

## Type System

### What types does FACET support?

**Primitive Types:**
- `string` - UTF-8 text
- `int` - 64-bit integers
- `float` - 64-bit floats
- `bool` - true/false
- `null` - null value

**Composite Types:**
- `List<T>` - arrays of any type
- `Map<K,V>` - key-value dictionaries
- `{name: string, age: int}` - structs
- `string | int` - unions

**Multimodal Types:**
- `image(max_dim=1024, format="png")`
- `audio(max_duration=300.0, format="mp3")`
- `embedding(size=1536)`

### How does type inference work?

FACET uses **bidirectional type inference**:

```facet
@vars
  name: "Alice"        # Inferred: string
  count: 42           # Inferred: int
  active: true        # Inferred: bool
  items: ["a", "b"]   # Inferred: List<string>
```

Types are inferred from literals and propagated through pipelines.

### How do constraints work?

```facet
@vars
  age: @input(type="int", default=30)
  email: @input(type="string")
  score: 85
```

Constraints are validated at compile time and runtime.

### What's the difference between compile-time and runtime errors?

- **Compile-time errors**: Type mismatches, missing variables, constraint violations
- **Runtime errors**: Gas exhaustion, token budget exceeded, execution failures

Compile-time errors prevent deployment; runtime errors occur during execution.

---

## Execution Model

### How does FACET execute code?

FACET uses a **5-phase execution model**:

1. **Resolution**: Parse AST, resolve imports
2. **Type Checking**: Validate types using FTS
3. **Reactive Compute**: Execute R-DAG (dependency graph)
4. **Layout**: Token allocation using Token Box Model
5. **Render**: Generate canonical JSON

### What is R-DAG?

**R-DAG (Reactive Dependency Graph)** automatically resolves variable dependencies:

```facet
@vars
  user_input: $query                    # Phase 1
  cleaned: $user_input |> trim()        # Phase 2 (depends on Phase 1)
  greeting: "Hello, " |> append($cleaned) # Phase 3 (depends on Phase 2)
```

Variables are computed in topological order automatically.

### How does token budgeting work?

```facet
@system
  budget: 4096    # Total tokens available

@vars
  summary: $long_text |> summarize(max_tokens=500)
  analysis: $data |> analyze()  # Uses remaining budget
```

The **Token Box Model** allocates tokens proportionally based on priority and content size.

### What are gas limits?

Gas prevents infinite loops and resource exhaustion:

```facet
@system
  gas_limit: 10000  # Maximum computation steps

@vars
  result: $data |> complex_transform()  # Limited by gas
```

Each lens operation consumes gas. Execution fails with F902 if gas is exhausted.

### How do lenses work?

Lenses are pure functions that transform data:

```facet
@vars
  name: "alice" |> capitalize() |> trim()
  words: $text |> split(" ") |> unique()
  json: $data |> json_stringify()
```

Lenses are composable and execute in pipeline order.

---

## Performance

### How fast is FACET compilation?

**Typical performance:**
- **Cold start**: <50ms
- **Warm execution**: <10ms
- **Large files**: <500ms (20KB+)
- **Memory usage**: <50MB peak

### What's the memory footprint?

**Runtime memory:**
- **Baseline**: 8MB
- **Per variable**: +2KB
- **Per lens**: +5KB
- **Token allocation**: +0.1KB per token

### How does FACET scale?

**Horizontal scaling:**
- Stateless compilation
- Independent executions
- Parallel pipeline processing

**Vertical scaling:**
- Linear time complexity O(n)
- Memory efficient (streaming parsers)
- CPU optimized (SIMD operations)

### Can FACET handle large files?

Yes! FACET uses:
- **Streaming parsers** for large inputs
- **Memory-mapped files** for big FACET sources
- **Lazy evaluation** for unused variables
- **Incremental compilation** for caching

---

## Security

### Is FACET secure?

FACET provides deterministic control points for execution security:

- **Zero-trust model** with defense-in-depth
- **Deterministic compiler/runtime boundaries**
- **Resource bounding** (token budgets, gas limits)
- **Type safety** (prevents injection attacks)
- **Hermetic compile phases** and import sandboxing

### How does FACET prevent attacks?

**Input Validation:**
```facet
@vars
  query: @input(type="string")
```

**Resource Limits:**
```facet
@system
  budget: 4096
  gas_limit: 10000
  timeout_ms: 30000
```

**Sandboxing:**
- No file system access during execution
- No network access during compilation
- Isolated execution environment

### Does FACET support audit logging?

Yes! FACET provides comprehensive audit trails:

```json
{
  "timestamp": "2025-12-09T10:30:00Z",
  "operation": "execute",
  "file_hash": "a1b2c3...",
  "tokens_used": 1250,
  "gas_used": 450,
  "execution_time_ms": 120,
  "success": true
}
```

### Is FACET compliant with regulations?

FACET can provide technical evidence for audits (policy hash, execution artifacts, deterministic error surface), but compliance is determined at organization/system level, not by FACET alone.

---

## Integration

### How do I integrate FACET with OpenAI?

```rust
use facet_fct::*;
use serde_json::json;

let facet_source = r#"
@system
  role: "assistant"
  model: "gpt-5.2"

@user
  query: "Hello!"

@vars
  context: "You are helpful."
"#;

let json = compile_facet(facet_source)?;

// Send to OpenAI
let response = openai::chat::completions(&json).await?;
```

### Can FACET work with other LLM providers?

Yes! FACET generates **canonical JSON** that works with:

- **OpenAI**: GPT-5.2, GPT-5.2-chat-latest
- **Anthropic**: Claude Sonnet 4.6
- **Google**: Gemini family
- **Meta**: Llama models
- **Local models**: Via OpenAI-compatible APIs

### How do I use FACET in a web application?

```javascript
import { FacetCompiler } from 'facet-fct';

const compiler = new FacetCompiler();
const facetCode = `
@system role: "assistant"
@user query: "Hello!"
`;

const result = compiler.compile(facetCode);
const json = JSON.parse(result);
// Send to your LLM API
```

### What's the difference between CLI and library usage?

**CLI (Command Line):**
```bash
fct build --input agent.facet --output agent.json
fct run --input agent.facet --budget 4096
```

**Library (Rust):**
```rust
use facet_fct::*;

let doc = parse_document(source)?;
let validated = validate(&doc)?;
let result = execute(&validated)?;
```

**WASM (Browser):**
```javascript
import { compile } from 'facet-fct';
const result = compile(source);
```

---

## Troubleshooting

### Common Error Codes

**F001: Invalid indentation**
```
Fix: Use consistent 2 or 4 spaces, no tabs
```

**F002: Tabs not allowed**
```
Fix: Replace tabs with spaces
```

**F003: Parse error**
```
Fix: Check for unclosed brackets, quotes, or invalid syntax
```

**F401: Variable not found**
```
Fix: Ensure variable is defined before use
```

**F451: Type mismatch**
```
Fix: Check type compatibility in assignments
```

**F505: Cyclic dependency**
```
Fix: Remove circular variable references
```

**F902: Gas exhausted**
```
Fix: Increase gas_limit or optimize computation
```

**F901: Budget exceeded**
```
Fix: Increase token budget or reduce content size
```

### Performance Issues

**Slow compilation:**
- Use `cargo build --release`
- Enable incremental compilation
- Split large files into smaller ones

**High memory usage:**
- Use streaming mode for large files
- Implement lazy evaluation
- Reduce variable count

**Token inefficiency:**
- Optimize context priority settings
- Use compression lenses
- Remove redundant information

### Debugging Tips

**Enable verbose logging:**
```bash
fct run --input agent.facet --verbose --log-level debug
```

**Inspect AST:**
```bash
fct inspect --input agent.facet
```

**Profile execution:**
```bash
fct run --input agent.facet --profile --profile-output profile.json
```

---

## Contributing

### How can I contribute?

**Code Contributions:**
```bash
git clone https://github.com/rokoss21/facet-compiler
cd facet-compiler
cargo test  # Ensure tests pass
# Make your changes
cargo test  # Run tests again
```

**Documentation:**
- Fix typos or unclear explanations
- Add examples or use cases
- Translate documentation

**Testing:**
- Add test cases for new features
- Improve existing test coverage
- Report bugs with reproduction steps

### Development Setup

```bash
# Install dependencies
rustup install 1.70
rustup default 1.70

# Clone and setup
git clone https://github.com/rokoss21/facet-compiler
cd facet-compiler

# Run full test suite
cargo test --all

# Run benchmarks
cargo bench

# Check code quality
cargo clippy -- -D warnings
cargo fmt --check
```

### Where can I get help?

- **GitHub Issues**: Bug reports and feature requests
- **Documentation**: Comprehensive guides in `docs/`
- **Examples**: Working code samples in `examples/`
- **Tests**: 54 test cases showing usage patterns

### What's the roadmap?

**Q1 2026:**
- Enhanced lens library (20+ lenses)
- WASM performance optimizations
- Cloud deployment templates

**Q2 2026:**
- Visual FACET editor (web-based)
- Integration SDKs (Python, Node.js, Go)
- Enterprise security features

**Q3 2026:**
- Multi-model support
- Advanced constraint system
- Plugin architecture

---

## Still Have Questions?

- 📖 **Read the docs**: [02-tutorial.md](02-tutorial.html), [03-architecture.md](03-architecture.html)
- 🔍 **Check examples**: [05-examples-guide.md](05-examples-guide.html)
- 🧪 **Run tests**: `cargo test` to see working code
- 💬 **Open an issue**: GitHub Issues for questions
- 📧 **Community**: (Coming soon)

**Complete Documentation Map:**
- **[01-quickstart.md](01-quickstart.html)** - Get started in 5 minutes
- **[02-tutorial.md](02-tutorial.html)** - Complete learning path
- **[03-architecture.md](03-architecture.html)** - System architecture
- **[04-type-system.md](04-type-system.html)** - Type system reference
- **[05-examples-guide.md](05-examples-guide.html)** - Practical examples
- **[06-cli.md](06-cli.html)** - Command-line interface
- **[07-api-reference.md](07-api-reference.html)** - Rust API documentation
- **[08-lenses.md](08-lenses.html)** - Lens library reference
- **[09-testing.md](09-testing.html)** - Testing framework
- **[10-performance.md](10-performance.html)** - Performance optimization
- **[11-security.md](11-security.html)** - Security best practices
- **[12-errors.md](12-errors.html)** - Error codes and troubleshooting
- **[13-import-system.md](13-import-system.html)** - Import system guide

**FACET v2.1.3 is designed to be approachable yet powerful. Don't hesitate to experiment!** 🚀
