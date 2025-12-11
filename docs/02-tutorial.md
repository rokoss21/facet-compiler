---
---
# 02. FACET v2.0 Complete Tutorial

**Reading Time:** 30-60 minutes | **Difficulty:** Beginner → Advanced | **Previous:** [01-quickstart.md](01-quickstart.md) | **Next:** [03-architecture.md](03-architecture.md)

**Learning Path:** Beginner → Intermediate → Advanced
**Prerequisites:** Basic programming knowledge

---

## Table of Contents

1. [Hello World](#1-hello-world)
2. [Variables and References](#2-variables-and-references)
3. [Lens Pipelines](#3-lens-pipelines)
4. [Type System](#4-type-system)
5. [Imports and Modularity](#5-imports-and-modularity)
6. [Token Budget Management](#6-token-budget-management)
7. [Testing with @test Blocks](#7-testing-with-test-blocks)
8. [Advanced Patterns](#8-advanced-patterns)

---

## 1. Hello World

Let's create your first FACET file!

### Step 1: Create `hello.facet`

```facet
@system
  role: "assistant"
  instructions: "You are a helpful AI assistant."

@user
  query: "Hello, world!"
```

### Step 2: Compile it

```bash
$ fct build --input hello.facet
✓ Parsing successful
✓ Validation successful
✓ Build complete
```

### Step 3: Execute and render

```bash
$ fct run --input hello.facet --format pretty
```

**Output:**

```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful AI assistant."
    },
    {
      "role": "user",
      "content": "Hello, world!"
    }
  ]
}
```

**What Happened:**
1. Parser read the `.facet` file
2. Validator checked syntax and types
3. R-DAG resolved variables (none in this case)
4. Renderer created canonical JSON

---

## 2. Variables and References

FACET supports **reactive variables** with the `@vars` block.

### Step 1: Define variables

Create `variables.facet`:

```facet
@vars
  username: "Alice"
  greeting: "Hello"

@system
  role: "assistant"

@user
  query: "$greeting, $username!"
```

**Variable Substitution:** Use `$varname` to reference variables.

### Step 2: Compile and run

```bash
$ fct run --input variables.facet
```

**Output:**

```json
{
  "messages": [
    {
      "role": "user",
      "content": "Hello, Alice!"
    }
  ]
}
```

### Step 3: Nested variables

```facet
@vars
  name: "Bob"
  title: "Dr."
  full_name: "$title $name"

@user
  query: "Welcome, $full_name!"
```

**Output:**

```
"Welcome, Dr. Bob!"
```

### Step 4: Order-independent declarations (R-DAG)

```facet
@vars
  # Can reference 'base' before declaring it!
  derived: $base |> uppercase()
  base: "hello"
```

**FACET v2.0 automatically resolves the dependency order using R-DAG.**

---

## 3. Lens Pipelines

**Lenses** transform data using the `|>` operator.

### Available Lenses

**String Lenses:**
- `trim()` - Remove whitespace
- `lowercase()` - Convert to lowercase
- `uppercase()` - Convert to uppercase
- `replace(old, new)` - String replacement

**List Lenses:**
- `join(sep)` - Join list to string
- `split(sep)` - Split string to list
- `filter(lens)` - Filter elements
- `map(lens)` - Transform elements
- `sort()` - Sort elements

**Map Lenses:**
- `keys()` - Extract keys
- `values()` - Extract values
- `pick(keys)` - Select specific keys
- `omit(keys)` - Remove specific keys

**Utility Lenses:**
- `default(value)` - Provide fallback
- `format(template)` - Template formatting

### Example 1: String transformation

```facet
@vars
  raw_input: "  Hello World  "
  clean: $raw_input |> trim() |> lowercase()

@user
  query: $clean
```

**Result:** `"hello world"`

### Example 2: Chaining multiple lenses

```facet
@vars
  name: "  alice  "
  formatted: $name |> trim() |> uppercase() |> format("User: {}")

@system
  context: $formatted
```

**Result:** `"User: ALICE"`

### Example 3: List processing

```facet
@vars
  tags_str: "rust, compiler, ai"
  tags: $tags_str |> split(",") |> map(trim()) |> sort()
  tags_display: $tags |> join(", ")

@user
  info: "Tags: $tags_display"
```

**Result:** `"Tags: ai, compiler, rust"`

### Example 4: Map operations

```facet
@vars
  user_data: {
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com",
    "password": "secret"
  }
  public_data: $user_data |> omit(["password"])

@system
  user_info: $public_data
```

**Result:** Only name, age, email (password removed)

---

## 4. Type System

FACET has a **static type system** (FTS - Facet Type System).

### Basic Types

```facet
@var_types
  name: "string"
  age: "int"
  score: "float"
  active: "bool"

@vars
  name: "Alice"     # ✓ Valid
  age: 25           # ✓ Valid
  score: 98.5       # ✓ Valid
  active: true      # ✓ Valid
```

### Type Constraints

```facet
@var_types
  age: {
    type: "int",
    min: 0,
    max: 120
  }
  email: {
    type: "string",
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
  }
  role: {
    type: "string",
    enum: ["admin", "user", "guest"]
  }

@vars
  age: 25               # ✓ Valid (0 ≤ 25 ≤ 120)
  email: "a@b.com"      # ✓ Valid (matches pattern)
  role: "admin"         # ✓ Valid (in enum)
```

### Type Errors

```facet
@var_types
  age: {
    type: "int",
    min: 0,
    max: 120
  }

@vars
  age: 150  # ❌ F452: Constraint violation (exceeds max)
```

**Compiler Output:**

```
Error: F452: Constraint violation: max failed for value 150
  --> example.facet:8:8
```

### Complex Types

```facet
@var_types
  users: {
    type: "list",
    element_type: "string"
  }
  config: {
    type: "map",
    value_type: "int"
  }

@vars
  users: ["Alice", "Bob", "Charlie"]
  config: {"timeout": 30, "retries": 3}
```

### Multimodal Types

```facet
@var_types
  avatar: {
    type: "image",
    max_dim: 512,
    format: "png"
  }
  voice: {
    type: "audio",
    max_duration: 60,
    format: "mp3"
  }

@vars
  avatar: @input {type: "image", name: "avatar"}
  voice: @input {type: "audio", name: "voice"}
```

---

## 5. Imports and Modularity

Break large FACET files into reusable modules.

### Step 1: Create common definitions

**File:** `common.facet`

```facet
@var_types
  username: "string"
  user_role: {
    type: "string",
    enum: ["admin", "user", "guest"]
  }

@vars
  app_name: "My AI App"
  version: "1.0.0"
```

### Step 2: Import in main file

**File:** `main.facet`

```facet
@import "common.facet"

@vars
  username: "Alice"
  user_role: "admin"
  welcome: "Welcome to $app_name v$version, $username!"

@system
  role: "assistant"

@user
  query: $welcome
```

### Step 3: Compile

```bash
$ fct run --input main.facet
```

**Result:**

```
"Welcome to My AI App v1.0.0, Alice!"
```

### Circular Import Detection

**File A:**

```facet
@import "file_b.facet"
```

**File B:**

```facet
@import "file_a.facet"  # ❌ Circular import!
```

**Compiler Output:**

```
Error: F602: Circular import detected: file_b.facet
```

---

## 6. Token Budget Management

FACET provides **deterministic token allocation** using the Token Box Model.

### Example: Budget-aware context

```facet
@meta
  model: "gpt-4"
  budget: 4096

@system (priority=100, shrink=0)
  role: "assistant"
  instructions: "You are a helpful AI."

@context (priority=80, shrink=0.5)
  background: "Long context information that can be compressed..."

@user (priority=90, shrink=0)
  query: "User question here"
```

### Priority and Shrink

**priority:** Higher = allocated first (0-100)
**shrink:** Compression factor when budget tight (0.0-1.0)
- `0.0` = Critical (no compression)
- `0.5` = Can compress to 50%
- `1.0` = Can omit completely

### Budget Exceeded Error

```facet
@meta
  budget: 100  # Very small budget

@system (priority=100, shrink=0)
  instructions: "Very long system prompt that exceeds 100 tokens..."
```

**Compiler Output:**

```
Error: F901: Budget exceeded: critical sections require 150 tokens, but budget is 100
```

### Budget Allocation Example

```
Budget: 1000 tokens

Sections:
  - system:  200 tokens, priority=100, shrink=0 (critical)
  - context: 600 tokens, priority=80,  shrink=0.5
  - user:    100 tokens, priority=90,  shrink=0

Allocation:
  1. system:  200 tokens (critical, allocated first)
  2. user:    100 tokens (priority 90, fits in remaining 800)
  3. context: 600 tokens (priority 80, fits in remaining 700)

Total: 900 tokens (fits!)
```

---

## 7. Testing with @test Blocks

FACET has **built-in testing** with `@test` blocks.

### Example 1: Basic assertion

```facet
@vars
  result: "hello" |> uppercase()

@test (name="uppercase test")
  assert:
    - equals: {target: "$result", expected: "HELLO"}
```

**Run Tests:**

```bash
$ fct test --input example.facet
✓ uppercase test passed
```

### Example 2: Mocking external lenses

```facet
@vars
  weather: @external "WeatherAPI.get_current"

@test (name="weather test")
  mock:
    - target: "WeatherAPI.get_current"
      return: {"temp": 72, "conditions": "sunny"}

  assert:
    - equals: {target: "$weather.temp", expected: 72}
```

### Example 3: Multiple assertions

```facet
@test (name="comprehensive test")
  vars:
    input: "  Test String  "

  assert:
    - contains: {target: "$input", text: "Test"}
    - not_contains: {target: "$input", text: "Missing"}
    - matches: {target: "$input", pattern: "\\s+Test\\s+"}
```

### Available Assertions

- `equals` - Value equality
- `not_equals` - Value inequality
- `contains` - String contains text
- `not_contains` - String does not contain
- `matches` - Regex match
- `not_matches` - Regex does not match
- `less_than` - Numeric comparison
- `greater_than` - Numeric comparison
- `true` - Boolean is true
- `false` - Boolean is false
- `null` - Value is null
- `not_null` - Value is not null

---

## 8. Advanced Patterns

### Pattern 1: Dynamic system prompts

```facet
@vars
  user_level: "expert"

  system_prompt: @if($user_level == "expert",
    "Provide detailed technical explanations.",
    "Keep explanations simple and beginner-friendly."
  )

@system
  instructions: $system_prompt
```

### Pattern 2: Prompt templates with conditionals

```facet
@vars
  include_examples: true

  base_prompt: "You are a coding assistant."

  examples: @if($include_examples,
    "Example 1: ...\nExample 2: ...",
    ""
  )

  final_prompt: "$base_prompt\n\n$examples"

@system
  instructions: $final_prompt
```

### Pattern 3: Multi-language support

```facet
@vars
  language: "en"

  greetings: {
    "en": "Hello",
    "es": "Hola",
    "fr": "Bonjour"
  }

  greeting: $greetings |> pick([$language])

@user
  message: "$greeting, user!"
```

### Pattern 4: Composable context sections

```facet
@import "base_context.facet"
@import "user_context.facet"
@import "task_context.facet"

@vars
  full_context: "$base_context\n$user_context\n$task_context"

@context
  combined: $full_context
```

### Pattern 5: Error handling with defaults

```facet
@vars
  user_input: @input {type: "string", name: "query"}
  safe_input: $user_input |> default("No input provided")
  cleaned: $safe_input |> trim() |> lowercase()

@user
  query: $cleaned
```

---

## Common Pitfalls and Solutions

### Pitfall 1: Forward references without R-DAG

**❌ Wrong (in other languages):**

```facet
@vars
  derived: $base   # Error: base not defined yet
  base: "value"
```

**✓ Correct (FACET v2.0):**

```facet
@vars
  derived: $base   # ✓ Works! R-DAG resolves order
  base: "value"
```

### Pitfall 2: Cyclic dependencies

**❌ Wrong:**

```facet
@vars
  a: $b
  b: $a  # Error: F505 Cyclic dependency
```

**✓ Correct:**

```facet
@vars
  base: "value"
  a: $base
  b: $base
```

### Pitfall 3: Type mismatches

**❌ Wrong:**

```facet
@var_types
  age: "int"

@vars
  age: "25"  # Error: F451 Type mismatch (string vs int)
```

**✓ Correct:**

```facet
@var_types
  age: "int"

@vars
  age: 25  # No quotes for integers
```

### Pitfall 4: Budget overruns

**❌ Wrong:**

```facet
@meta
  budget: 100

@system (shrink=0)
  long_prompt: "... 200 tokens ..."  # Error: F901 Budget exceeded
```

**✓ Correct:**

```facet
@meta
  budget: 100

@system (shrink=0.5)  # Allow compression
  long_prompt: "... 200 tokens ..."
```

---

## Next Steps

**Congratulations!** You've completed the FACET v2.0 tutorial.

### Continue Learning:

1. **[Error Reference](errors.md)** - All error codes explained
2. **[Lens Reference](lenses.md)** - Complete lens documentation
3. **[Type System](type-system.md)** - Deep dive into FTS
4. **[Architecture](architecture.md)** - Compiler internals
5. **[CLI Reference](cli.md)** - All commands and options

### Build Something:

- **Personal Assistant** - Create your own AI assistant
- **Customer Service Bot** - Build a support chatbot
- **Code Review Agent** - Automate code review
- **Data Analysis Agent** - Process and analyze data

### Contribute:

- [GitHub Repository](https://github.com/yourorg/facet)
- [Issue Tracker](https://github.com/yourorg/facet/issues)
- [Contributing Guide](../CONTRIBUTING.md)

---

## Next Steps

🎯 **Continue Learning:**
- **[03-architecture.md](03-architecture.md)** - System architecture and design
- **[04-type-system.md](04-type-system.md)** - Deep dive into FTS
- **[05-examples-guide.md](05-examples-guide.md)** - All examples explained

🔧 **Reference Materials:**
- **[06-cli.md](06-cli.md)** - Command-line interface
- **[07-api-reference.md](07-api-reference.md)** - Rust API documentation
- **[08-lenses.md](08-lenses.md)** - Lens library reference

📚 **Advanced Topics:**
- **[09-testing.md](09-testing.md)** - @test blocks and mocking
- **[10-performance.md](10-performance.md)** - Optimization and benchmarking
- **[11-security.md](11-security.md)** - Security best practices

🆘 **Help & Troubleshooting:**
- **[faq.md](../faq.md)** - Frequently asked questions
- **[12-errors.md](12-errors.md)** - Error codes and fixes

---

**Happy coding with FACET!** 🎉

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09

🔧 **Reference Materials:**
- **[06-cli.md](06-cli.md)** - Command-line interface
- **[07-api-reference.md](07-api-reference.md)** - Rust API documentation
- **[08-lenses.md](08-lenses.md)** - Lens library reference

📚 **Advanced Topics:**
- **[09-testing.md](09-testing.md)** - @test blocks and mocking
- **[10-performance.md](10-performance.md)** - Optimization and benchmarking
- **[11-security.md](11-security.md)** - Security best practices

🆘 **Help & Troubleshooting:**
- **[faq.md](../faq.md)** - Frequently asked questions
- **[12-errors.md](12-errors.md)** - Error codes and fixes

---

**Happy coding with FACET!** 🎉

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09
