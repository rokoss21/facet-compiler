---
---
# 04. FACET Type System (FTS) Reference

**Reading Time:** 25-35 minutes | **Difficulty:** Intermediate | **Previous:** [03-architecture.md](03-architecture.md) | **Next:** [05-examples-guide.md](05-examples-guide.md)

**Version:** 0.1.0
**Status:** Production Ready
**Last Updated:** 2025-12-09

---

## Table of Contents

- [Overview](#overview)
- [Primitive Types](#primitive-types)
- [Multimodal Types](#multimodal-types)
- [Composite Types](#composite-types)
- [Type Constraints](#type-constraints)
- [Type Inference](#type-inference)
- [Type Errors](#type-errors)
- [Advanced Patterns](#advanced-patterns)

---

## Overview

The **Facet Type System (FTS)** provides static type checking for AI agent configurations, catching errors at compile time instead of runtime.

### Key Properties

**Static Typing:**
- Types checked before execution
- Compile-time error detection
- No runtime type errors

**Type Inference:**
- Automatic type detection from values
- Minimal type annotations required
- Explicit types for documentation

**Constraint Validation:**
- Range constraints (min/max)
- Pattern matching (regex)
- Enum validation
- Custom validators

---

## Primitive Types

### 1. String

**Description:** UTF-8 text sequences

**Syntax:**

```facet
@var_types
  name: "string"

@vars
  name: "Alice"
```

**Constraints:**

```facet
@var_types
  email: {
    type: "string",
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
  }
  role: {
    type: "string",
    enum: ["admin", "user", "guest"]
  }
  username: {
    type: "string",
    min_length: 3,
    max_length: 20
  }

@vars
  email: "alice@example.com"  # ✓ Matches pattern
  role: "admin"                # ✓ In enum
  username: "alice123"         # ✓ Length 8 (3 ≤ 8 ≤ 20)
```

**Error Examples:**

```facet
@var_types
  email: {
    type: "string",
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
  }

@vars
  email: "invalid-email"  # ❌ F452: Pattern constraint failed
```

### 2. Integer

**Description:** Whole numbers (-2^63 to 2^63-1)

**Syntax:**

```facet
@var_types
  age: "int"

@vars
  age: 25
```

**Constraints:**

```facet
@var_types
  age: {
    type: "int",
    min: 0,
    max: 120
  }
  percentage: {
    type: "int",
    min: 0,
    max: 100
  }

@vars
  age: 25         # ✓ Valid (0 ≤ 25 ≤ 120)
  percentage: 75  # ✓ Valid (0 ≤ 75 ≤ 100)
```

**Error Examples:**

```facet
@var_types
  age: {
    type: "int",
    min: 0,
    max: 120
  }

@vars
  age: 150  # ❌ F452: Constraint violation (exceeds max)
  age: -5   # ❌ F452: Constraint violation (below min)
```

### 3. Float

**Description:** Floating-point numbers (IEEE 754 double precision)

**Syntax:**

```facet
@var_types
  temperature: "float"

@vars
  temperature: 98.6
  scientific: 1.23e-4
```

**Constraints:**

```facet
@var_types
  temperature: {
    type: "float",
    min: -273.15,  # Absolute zero
    max: 1000.0
  }

@vars
  temperature: 25.5  # ✓ Valid
```

### 4. Boolean

**Description:** True or false values

**Syntax:**

```facet
@var_types
  is_active: "bool"

@vars
  is_active: true
  is_admin: false
```

**No Constraints Available** (always `true` or `false`)

### 5. Null

**Description:** Absence of value

**Syntax:**

```facet
@var_types
  optional_field: "null"

@vars
  optional_field: null
```

**Use Case:** Optional fields, default values

---

## Multimodal Types

FACET supports **multimodal AI** inputs beyond text.

### 1. Image

**Description:** Image data with format and size constraints

**Syntax:**

```facet
@var_types
  avatar: {
    type: "image",
    max_dim: 512,
    format: "png"
  }

@vars
  avatar: @input {type: "image", name: "avatar"}
```

**Constraints:**

- `max_dim`: Maximum width/height in pixels
- `format`: Image format (`"png"`, `"jpeg"`, `"webp"`)

**Error Example:**

```facet
@var_types
  profile_pic: {
    type: "image",
    max_dim: 256
  }

@vars
  profile_pic: @input {
    type: "image",
    name: "pic",
    max_dim: 1024  # ❌ F452: Exceeds type constraint
  }
```

### 2. Audio

**Description:** Audio data with duration and format constraints

**Syntax:**

```facet
@var_types
  voice_input: {
    type: "audio",
    max_duration: 60,
    format: "mp3"
  }

@vars
  voice_input: @input {type: "audio", name: "voice"}
```

**Constraints:**

- `max_duration`: Maximum length in seconds
- `format`: Audio format (`"mp3"`, `"wav"`, `"ogg"`)

### 3. Embedding

**Description:** Vector embeddings for semantic search

**Syntax:**

```facet
@var_types
  text_embedding: {
    type: "embedding",
    size: 1536  # OpenAI ada-002 size
  }

@vars
  text_embedding: @external "OpenAI.embed"
```

**Constraints:**

- `size`: Embedding dimension (must match model)

---

## Composite Types

### 1. List<T>

**Description:** Ordered collection of elements

**Syntax:**

```facet
@var_types
  tags: {
    type: "list",
    element_type: "string"
  }

@vars
  tags: ["ai", "compiler", "rust"]
```

**Nested Lists:**

```facet
@var_types
  matrix: {
    type: "list",
    element_type: {
      type: "list",
      element_type: "int"
    }
  }

@vars
  matrix: [[1, 2], [3, 4]]
```

**Constraints:**

```facet
@var_types
  tags: {
    type: "list",
    element_type: "string",
    min_length: 1,
    max_length: 10
  }

@vars
  tags: ["tag1", "tag2"]  # ✓ Length 2 (1 ≤ 2 ≤ 10)
```

### 2. Map<K, V>

**Description:** Key-value mappings (dictionaries)

**Syntax:**

```facet
@var_types
  config: {
    type: "map",
    key_type: "string",
    value_type: "int"
  }

@vars
  config: {
    "timeout": 30,
    "retries": 3
  }
```

**Nested Maps:**

```facet
@var_types
  settings: {
    type: "map",
    key_type: "string",
    value_type: {
      type: "map",
      key_type: "string",
      value_type: "string"
    }
  }

@vars
  settings: {
    "user": {"name": "Alice", "role": "admin"}
  }
```

### 3. Struct

**Description:** Fixed-field records (like TypeScript interfaces)

**Syntax:**

```facet
@var_types
  user: {
    type: "struct",
    fields: {
      name: {type: "string"},
      age: {type: "int", min: 0},
      email: {
        type: "string",
        pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
      }
    }
  }

@vars
  user: {
    "name": "Alice",
    "age": 30,
    "email": "alice@example.com"
  }
```

**Optional Fields:**

```facet
@var_types
  user: {
    type: "struct",
    fields: {
      name: {type: "string"},
      nickname: {type: "string", optional: true}
    }
  }

@vars
  user: {"name": "Alice"}  # ✓ nickname is optional
```

### 4. Union

**Description:** One of several types

**Syntax:**

```facet
@var_types
  id: {
    type: "union",
    types: ["string", "int"]
  }

@vars
  id: "abc123"  # ✓ Valid (string)
  # OR
  id: 12345     # ✓ Valid (int)
```

**Complex Unions:**

```facet
@var_types
  value: {
    type: "union",
    types: [
      {type: "string"},
      {type: "int"},
      {type: "list", element_type: "string"}
    ]
  }

@vars
  value: "text"         # ✓ Valid
  # OR
  value: 42             # ✓ Valid
  # OR
  value: ["a", "b"]     # ✓ Valid
```

---

## Type Constraints

### Range Constraints (Int/Float)

```facet
@var_types
  age: {
    type: "int",
    min: 0,
    max: 120
  }
  temperature: {
    type: "float",
    min: -273.15,
    max: 1000.0
  }
```

**Validation:** Value must be in [min, max] range

### Pattern Constraints (String)

```facet
@var_types
  email: {
    type: "string",
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.[a-zA-Z]{2,}$"
  }
  phone: {
    type: "string",
    pattern: "^\\+?[1-9]\\d{1,14}$"  # E.164 format
  }
```

**Validation:** String must match regex pattern

### Enum Constraints (String)

```facet
@var_types
  role: {
    type: "string",
    enum: ["admin", "user", "guest"]
  }
  status: {
    type: "string",
    enum: ["pending", "approved", "rejected"]
  }
```

**Validation:** String must be one of enum values

### Length Constraints (String/List)

```facet
@var_types
  username: {
    type: "string",
    min_length: 3,
    max_length: 20
  }
  tags: {
    type: "list",
    element_type: "string",
    min_length: 1,
    max_length: 10
  }
```

**Validation:** Length must be in [min_length, max_length] range

---

## Type Inference

FACET **automatically infers types** from values when `@var_types` is not specified.

### Basic Inference

```facet
@vars
  name: "Alice"     # Inferred: string
  age: 25           # Inferred: int
  score: 98.5       # Inferred: float
  active: true      # Inferred: bool
  empty: null       # Inferred: null
```

### Collection Inference

```facet
@vars
  tags: ["rust", "ai"]              # Inferred: list<string>
  counts: [1, 2, 3]                 # Inferred: list<int>
  config: {"key": "value"}          # Inferred: map<string, string>
  mixed: {"name": "Alice", age: 30} # Inferred: struct{name: string, age: int}
```

### Pipeline Inference

```facet
@vars
  input: "Hello"                # Inferred: string
  output: $input |> uppercase() # Inferred: string (lens preserves type)
  items: "a,b,c" |> split(",")  # Inferred: list<string>
```

### When to Use Explicit Types

**Use explicit types when:**
1. Adding constraints (min/max/pattern)
2. Documenting API contracts
3. Enforcing invariants
4. Validating external inputs

---

## Type Errors

### F451: Type Mismatch

**Description:** Value doesn't match declared type

**Example:**

```facet
@var_types
  age: "int"

@vars
  age: "25"  # ❌ F451: Type mismatch (string vs int)
```

**Fix:** Use correct type

```facet
@vars
  age: 25  # ✓ No quotes for integers
```

### F452: Constraint Violation

**Description:** Value violates type constraints

**Example:**

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

**Fix:** Use value within constraints

```facet
@vars
  age: 25  # ✓ Within range [0, 120]
```

### F453: Input Validation Failed

**Description:** @input directive missing required fields

**Example:**

```facet
@vars
  query: @input {name: "query"}  # ❌ F453: Missing 'type' field
```

**Fix:** Provide all required fields

```facet
@vars
  query: @input {type: "string", name: "query"}  # ✓ Complete
```

---

## Advanced Patterns

### Pattern 1: Discriminated Unions

```facet
@var_types
  message: {
    type: "struct",
    fields: {
      kind: {type: "string", enum: ["text", "image"]},
      content: {type: "string"}
    }
  }

@vars
  text_message: {
    "kind": "text",
    "content": "Hello!"
  }
  image_message: {
    "kind": "image",
    "content": "data:image/png;base64,..."
  }
```

### Pattern 2: Recursive Types

```facet
@var_types
  tree_node: {
    type: "struct",
    fields: {
      value: {type: "int"},
      children: {
        type: "list",
        element_type: "tree_node"  # Recursive reference
      }
    }
  }

@vars
  tree: {
    "value": 1,
    "children": [
      {"value": 2, "children": []},
      {"value": 3, "children": []}
    ]
  }
```

### Pattern 3: Generic Constraints

```facet
@var_types
  result: {
    type: "union",
    types: [
      {
        type: "struct",
        fields: {
          success: {type: "bool", enum: [true]},
          data: {type: "string"}
        }
      },
      {
        type: "struct",
        fields: {
          success: {type: "bool", enum: [false]},
          error: {type: "string"}
        }
      }
    ]
  }

@vars
  success_result: {"success": true, "data": "Result"}
  error_result: {"success": false, "error": "Failed"}
```

### Pattern 4: Type Guards with Lenses

```facet
@vars
  user_input: @input {type: "string", name: "input"}
  safe_input: $user_input |> default("") |> trim()
  # Type: string (non-null after default())
```

---

## Type System Rules

### Rule 1: Soundness

**No false negatives:** If typechecker accepts, runtime won't fail with type error

### Rule 2: Gradual Typing

**Mix inference and annotation:** Explicit types where needed, inference elsewhere

### Rule 3: Constraint Propagation

**Constraints flow through pipelines:**

```facet
@var_types
  name: {
    type: "string",
    min_length: 3
  }

@vars
  name: @input {type: "string", name: "name"}
  # Constraint min_length=3 automatically applies to input
```

### Rule 4: Structural Typing

**Compatibility based on structure, not names:**

```facet
type A = {name: string, age: int}
type B = {name: string, age: int, email: string}

# B is compatible with A (has all required fields)
```

---

## Performance Considerations

### Type Checking Cost

- **Parse:** O(n) where n = source size
- **Type Inference:** O(v) where v = variables
- **Constraint Validation:** O(v * c) where c = avg constraints

**Typical Performance:**
- 100 variables: <1ms
- 1000 variables: <10ms
- 10000 variables: <100ms

### Optimization Tips

1. **Cache type information** - Reuse inferred types
2. **Lazy validation** - Only validate used variables
3. **Incremental checking** - Only recheck changed parts

---

## Comparison with Other Type Systems

| Feature | FACET FTS | TypeScript | Python typing | JSON Schema |
|---------|-----------|------------|---------------|-------------|
| Static checking | ✓ | ✓ | ⚠️ (optional) | ❌ (runtime) |
| Type inference | ✓ | ✓ | ⚠️ (limited) | ❌ |
| Constraints | ✓ | ⚠️ (limited) | ❌ | ✓ |
| Multimodal | ✓ | ❌ | ❌ | ❌ |
| Gradual typing | ✓ | ✓ | ✓ | ❌ |

---

## References

- [FACET v2.0 Specification](../facet2-specification.md)
- [Error Reference](errors.md)
- [02-tutorial.md](02-tutorial.md) - Complete learning tutorial
- [03-architecture.md](03-architecture.md) - System architecture overview

## Next Steps

🎯 **Apply Your Knowledge:**
- **[05-examples-guide.md](05-examples-guide.md)** - Type system in practice
- **[08-lenses.md](08-lenses.md)** - Lens transformations with types
- **[09-testing.md](09-testing.md)** - Type-safe testing

🔧 **Advanced Topics:**
- **[10-performance.md](10-performance.md)** - Type system performance characteristics
- **[12-errors.md](12-errors.md)** - Type-related error codes (F451-F453)
- **[07-api-reference.md](07-api-reference.md)** - Type system Rust API

📚 **References:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete FTS specification
- **[faq.md](../faq.md)** - Type system FAQs

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09

- **[08-lenses.md](08-lenses.md)** - Lens transformations with types
- **[09-testing.md](09-testing.md)** - Type-safe testing

🔧 **Advanced Topics:**
- **[10-performance.md](10-performance.md)** - Type system performance characteristics
- **[12-errors.md](12-errors.md)** - Type-related error codes (F451-F453)
- **[07-api-reference.md](07-api-reference.md)** - Type system Rust API

📚 **References:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete FTS specification
- **[faq.md](../faq.md)** - Type system FAQs

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09
