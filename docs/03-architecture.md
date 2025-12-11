# 03. FACET v2.0 Architecture Guide

**Reading Time:** 20-30 minutes | **Difficulty:** Intermediate | **Previous:** [02-tutorial.md](02-tutorial.md) | **Next:** [04-type-system.md](04-type-system.md)

**Last Updated:** 2025-12-09
**Version:** 0.1.0
**Status:** Production Ready

---

## Table of Contents

- [Overview](#overview)
- [System Architecture](#system-architecture)
- [Compilation Pipeline](#compilation-pipeline)
- [Crate Organization](#crate-organization)
- [Core Algorithms](#core-algorithms)
- [Data Flow](#data-flow)
- [Design Decisions](#design-decisions)

---

## Overview

FACET v2.0 is a **deterministic compiler** for AI agent behavior, transforming `.facet` files into canonical JSON with guaranteed reproducibility across all platforms.

### Key Properties

**Determinism:**
- Same input → same output, always
- No random number generation
- Deterministic token allocation
- Stable JSON field ordering

**Type Safety:**
- Static type checking (FTS - Facet Type System)
- Compile-time error detection
- Type inference where possible
- Constraint validation

**Resource Bounded:**
- Token budget enforcement (F901)
- Gas limit for computation (F902)
- No infinite loops
- Predictable memory usage

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        FACET v2.0                           │
│                   Deterministic Compiler                     │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
        ┌────────────────────────────────────┐
        │    Phase 1: PARSING                │
        │  ┌──────────────────────────────┐  │
        │  │  fct-parser (nom-based)      │  │
        │  │  - Tokenization              │  │
        │  │  - Syntax validation         │  │
        │  │  - AST construction          │  │
        │  └──────────────────────────────┘  │
        └────────────────────────────────────┘
                              │
                              ▼ FacetDocument
        ┌────────────────────────────────────┐
        │    Phase 2: RESOLUTION             │
        │  ┌──────────────────────────────┐  │
        │  │  fct-resolver                │  │
        │  │  - Import resolution         │  │
        │  │  - Cycle detection (F602)    │  │
        │  │  - Block merging             │  │
        │  └──────────────────────────────┘  │
        └────────────────────────────────────┘
                              │
                              ▼ Merged AST
        ┌────────────────────────────────────┐
        │    Phase 3: TYPE CHECKING          │
        │  ┌──────────────────────────────┐  │
        │  │  fct-validator (FTS)         │  │
        │  │  - Type inference            │  │
        │  │  - Constraint validation     │  │
        │  │  - Variable resolution       │  │
        │  │  - Lens validation           │  │
        │  └──────────────────────────────┘  │
        └────────────────────────────────────┘
                              │
                              ▼ Typed AST
        ┌────────────────────────────────────┐
        │    Phase 4: REACTIVE COMPUTE       │
        │  ┌──────────────────────────────┐  │
        │  │  fct-engine (R-DAG)          │  │
        │  │  - Dependency graph          │  │
        │  │  - Topological sort          │  │
        │  │  - Cycle detection (F505)    │  │
        │  │  - Variable evaluation       │  │
        │  │  - Lens execution            │  │
        │  │  - Gas accounting (F902)     │  │
        │  └──────────────────────────────┘  │
        └────────────────────────────────────┘
                              │
                              ▼ Evaluated Variables
        ┌────────────────────────────────────┐
        │    Phase 5: LAYOUT                 │
        │  ┌──────────────────────────────┐  │
        │  │  fct-engine (Token Box)      │  │
        │  │  - Token counting            │  │
        │  │  - Budget allocation (F901)  │  │
        │  │  - Section prioritization    │  │
        │  │  - Context packing           │  │
        │  └──────────────────────────────┘  │
        └────────────────────────────────────┘
                              │
                              ▼ Packed Context
        ┌────────────────────────────────────┐
        │    Phase 6: RENDERING              │
        │  ┌──────────────────────────────┐  │
        │  │  fct-render                  │  │
        │  │  - Canonical JSON            │  │
        │  │  - Vendor-specific format    │  │
        │  │  - Schema validation         │  │
        │  └──────────────────────────────┘  │
        └────────────────────────────────────┘
                              │
                              ▼
                    Canonical JSON Output
```

---

## Compilation Pipeline

### Phase 1: Parsing

**Input:** `.facet` source file
**Output:** `FacetDocument` AST
**Crate:** `fct-parser`

**Process:**
1. Tokenization - Break source into tokens
2. Syntax validation - Check grammar rules
3. AST construction - Build tree structure
4. Span tracking - Record source positions

**Error Codes:**
- F001: Invalid indentation
- F002: Tabs not allowed
- F003: Unclosed delimiter

### Phase 2: Resolution

**Input:** `FacetDocument` AST
**Output:** Merged AST with resolved imports
**Crate:** `fct-resolver`

**Process:**
1. Find @import directives
2. Load imported files recursively
3. Detect circular imports (F602)
4. Merge blocks with smart strategy
5. Build single unified AST

**Error Codes:**
- F601: Import not found
- F602: Circular import

### Phase 3: Type Checking

**Input:** Merged AST
**Output:** Typed and validated AST
**Crate:** `fct-validator`

**Process:**
1. Parse @var_types declarations
2. Infer variable types from values
3. Validate type assignments (F451)
4. Check constraints (F452)
5. Validate @input directives (F453)
6. Resolve variable references (F401)
7. Validate lens pipelines (F802)

**Error Codes:**
- F401: Variable not found
- F402: Type inference failed
- F451: Type mismatch
- F452: Constraint violation
- F453: Input validation failed
- F802: Unknown lens

### Phase 4: Reactive Compute (R-DAG)

**Input:** Typed AST
**Output:** Evaluated variables with computed values
**Crate:** `fct-engine`

**Process:**
1. Build dependency graph from @vars
2. Detect cycles (F505)
3. Topological sort for evaluation order
4. Execute variables in dependency order
5. Apply lens pipelines
6. Track gas consumption (F902)

**Key Algorithm:** See [R-DAG Engine](#r-dag-engine)

**Error Codes:**
- F505: Cyclic dependency
- F801: Lens execution failed
- F902: Gas exhausted

### Phase 5: Layout (Token Box Model)

**Input:** Evaluated variables
**Output:** Packed context within budget
**Crate:** `fct-engine`

**Process:**
1. Count tokens for all sections
2. Identify critical sections (shrink=0)
3. Check budget feasibility (F901)
4. Allocate by priority
5. Compress non-critical sections

**Key Algorithm:** See [Token Box Model](#token-box-model)

**Error Codes:**
- F901: Budget exceeded

### Phase 6: Rendering

**Input:** Packed context
**Output:** Canonical JSON
**Crate:** `fct-render`

**Process:**
1. Build JSON structure
2. Apply vendor-specific formatting
3. Stable field ordering (alphabetical)
4. Pretty-print if requested

---

## Crate Organization

FACET v2.0 uses a modular 7-crate architecture:

### 1. `fct-ast`

**Role:** Core data structures
**Exports:** AST node types, Span, serialization

```rust
pub enum FacetNode {
    System(FacetBlock),
    User(FacetBlock),
    Vars(FacetBlock),
    VarTypes(FacetBlock),
    Import(ImportNode),
    Test(TestBlock),
    // ...
}

pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}
```

**Dependencies:** None (foundation crate)
**Lines of Code:** ~600

### 2. `fct-parser`

**Role:** Source → AST transformation
**Parser:** nom combinator-based
**Exports:** `parse_document()`

**Key Features:**
- Indentation-sensitive parsing
- Pipeline operator `|>` support
- Float literal parsing (scientific notation)
- String escaping
- @import directive parsing

**Dependencies:** `fct-ast`, `nom`, `nom_locate`
**Lines of Code:** ~1200

### 3. `fct-resolver`

**Role:** Import resolution and merging
**Exports:** `resolve_imports()`

**Key Features:**
- Recursive import loading
- Cycle detection with path tracking
- Smart block merging (KeyValue replacement)
- Hermetic security (no network access)

**Dependencies:** `fct-ast`, `fct-parser`
**Lines of Code:** ~400

### 4. `fct-validator`

**Role:** Type checking and validation
**Type System:** FTS (Facet Type System)
**Exports:** `TypeChecker`

**FTS Types:**
- Primitives: `string`, `int`, `float`, `bool`, `null`
- Multimodal: `image`, `audio`, `embedding`
- Composites: `list<T>`, `map<K,V>`, `struct{...}`
- Unions: `string | int`

**Validation:**
- Type inference from values
- Constraint checking (min/max/pattern/enum)
- Variable resolution
- Lens signature validation
- @input directive validation

**Dependencies:** `fct-ast`, `fct-std`, `regex`
**Lines of Code:** ~1100

### 5. `fct-std`

**Role:** Standard lens library
**Exports:** `LensRegistry`, 15 built-in lenses

**Lenses:**
- **String:** `trim()`, `lowercase()`, `uppercase()`, `replace()`
- **List:** `join()`, `split()`, `filter()`, `map()`, `sort()`
- **Map:** `keys()`, `values()`, `pick()`, `omit()`
- **Utility:** `default()`, `format()`

**Dependencies:** `fct-ast`
**Lines of Code:** ~500

### 6. `fct-engine`

**Role:** Reactive computation and layout
**Exports:** `RDagEngine`, `TokenBoxModel`, `ExecutionContext`

**Core Algorithms:**
1. **R-DAG** - Reactive Dependency Acyclic Graph
2. **Token Box Model** - Deterministic context packing
3. **Gas Model** - Computation resource tracking
4. **Test Runner** - @test block execution

**Dependencies:** `fct-ast`, `fct-std`
**Lines of Code:** ~800

### 7. `fct-render`

**Role:** AST → JSON transformation
**Exports:** `render_document()`

**Output Formats:**
- Canonical JSON (deterministic)
- OpenAI format
- Anthropic format
- Llama format

**Dependencies:** `fct-ast`, `serde_json`
**Lines of Code:** ~300

---

## Core Algorithms

### R-DAG Engine

**Purpose:** Compute variables in correct dependency order

**Algorithm:**

```
Input: @vars block with variable definitions
Output: HashMap<String, EvaluatedValue>

1. Build Dependency Graph:
   For each variable:
     - Parse value expression
     - Extract variable references ($var)
     - Add edge: var -> referenced_var

2. Cycle Detection:
   Run DFS with color marking:
     - WHITE: Unvisited
     - GRAY: In-progress (on stack)
     - BLACK: Completed
   If GRAY node encountered → F505 error

3. Topological Sort:
   Use DFS post-order traversal
   Reverse post-order = evaluation order

4. Evaluation:
   For each var in topological order:
     - Resolve variable references
     - Execute lens pipeline if present
     - Track gas consumption
     - Store evaluated value
   If gas > limit → F902 error

5. Return evaluated variables
```

**Key Property:** Order-independent declarations

```facet
@vars
  b: $a      # Can reference 'a' before declaration
  a: "value" # R-DAG resolves order automatically
```

**Time Complexity:** O(V + E) where V = variables, E = dependencies

### Token Box Model

**Purpose:** Deterministic context packing within budget

**Algorithm:**

```
Input: Sections with {content, priority, shrink, is_critical}
       Budget in tokens
Output: Packed sections fitting budget

1. Token Counting:
   For each section:
     tokens = accurate_token_count(content)
     Store: {name, tokens, priority, shrink, is_critical}

2. Critical Check:
   critical_tokens = sum(tokens where is_critical)
   If critical_tokens > budget → F901 error

3. Prioritization:
   Sort sections by priority DESC

4. Allocation:
   remaining = budget - critical_tokens
   For each non-critical section (high→low priority):
     If tokens ≤ remaining:
       allocated = tokens
       remaining -= tokens
     Else:
       allocated = remaining * shrink
       remaining = 0
     Store allocation

5. Return packed sections
```

**Example:**

```
Budget: 1000 tokens
Sections:
  - system:    200 tokens, priority=100, critical=true
  - context:   500 tokens, priority=80,  shrink=0.5
  - user:      100 tokens, priority=90,  critical=false

Step 1: Critical = 200, remaining = 800
Step 2: Allocate user (priority 90): 100 tokens, remaining = 700
Step 3: Allocate context (priority 80): 500 tokens, remaining = 200
Result: All sections fit!
```

**Time Complexity:** O(N log N) where N = sections

---

## Data Flow

### Complete Example

**Input File** (`agent.facet`):

```facet
@import "common.facet"

@var_types
  name: "string"
  age: {
    type: "int",
    min: 0,
    max: 120
  }

@vars
  name: "Alice"
  age: 25
  greeting: $name |> trim() |> uppercase()

@system
  role: "assistant"

@user
  query: "Hello, $greeting!"
```

**Phase 1 - Parser:**

```rust
FacetDocument {
    blocks: [
        Import("common.facet"),
        VarTypes({"name": "string", "age": {...}}),
        Vars({"name": "Alice", "age": 25, "greeting": ...}),
        System({"role": "assistant"}),
        User({"query": "Hello, $greeting!"}),
    ]
}
```

**Phase 2 - Resolver:**

Loads `common.facet`, merges blocks → single unified AST

**Phase 3 - Validator:**

- Infers types: name→string, age→int
- Validates age=25 in [0,120] ✓
- Resolves $name in greeting ✓
- Validates lens chain: trim(), uppercase() ✓

**Phase 4 - R-DAG:**

Dependency graph:
```
greeting depends on name
```

Evaluation order:
1. name = "Alice"
2. age = 25
3. greeting = "Alice" |> trim() |> uppercase() = "ALICE"

**Phase 5 - Token Box:**

Token counts:
- system: 10 tokens
- user: 15 tokens (with substitution)

Total: 25 tokens (fits in budget)

**Phase 6 - Render:**

```json
{
  "model": "gpt-4",
  "messages": [
    {
      "role": "system",
      "content": "assistant"
    },
    {
      "role": "user",
      "content": "Hello, ALICE!"
    }
  ]
}
```

---

## Design Decisions

### 1. Why nom for Parsing?

**Decision:** Use nom combinator library
**Alternatives:** lalrpop, pest, hand-written

**Reasons:**
- Zero-copy parsing (performance)
- Excellent error reporting with spans
- Composable parsers
- Production-proven
- Active development

### 2. Why R-DAG over Sequential?

**Decision:** Reactive dependency graph
**Alternatives:** Sequential evaluation

**Reasons:**
- Order-independent declarations (declarative)
- Matches mental model (Excel, React)
- Automatic dependency resolution
- Cycle detection built-in
- Better error messages

### 3. Why Separate Validation Phase?

**Decision:** Parse → Validate → Execute
**Alternatives:** Combined parse+validate

**Reasons:**
- Clear separation of concerns
- Better error messages (parse vs type errors)
- Reusable AST
- Testability
- Supports multiple passes

### 4. Why Token Box Model?

**Decision:** Deterministic token allocation
**Alternatives:** Truncation, approximation

**Reasons:**
- Determinism guarantee
- Priority-based allocation
- Graceful degradation (shrink)
- Critical section protection
- Predictable behavior

### 5. Why 7 Crates?

**Decision:** Modular architecture
**Alternatives:** Monolithic

**Reasons:**
- Clear boundaries
- Independent testing
- Reusable components
- Parallel compilation
- Better organization

### 6. Why Static Typing?

**Decision:** Compile-time type checking
**Alternatives:** Dynamic/runtime

**Reasons:**
- Early error detection
- Better IDE support
- Documentation via types
- Refactoring safety
- Performance (no runtime checks)

### 7. Why Gas Model?

**Decision:** Resource-bounded execution
**Alternatives:** Unlimited

**Reasons:**
- Security (prevents DoS)
- Predictable cost
- Quotas and limits
- Fair resource sharing
- Production safety

---

## Performance Characteristics

| Component | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| Parser | O(n) | O(n) | n = source length |
| Resolver | O(d * n) | O(n) | d = import depth (max 10) |
| Type Checker | O(v + e) | O(v) | v = variables, e = type edges |
| R-DAG Build | O(v + d) | O(v + d) | d = dependencies |
| R-DAG Execute | O(v + l) | O(v) | l = lens operations |
| Token Count | O(s * t) | O(s) | s = sections, t = avg tokens |
| Token Allocation | O(s log s) | O(s) | sorting by priority |
| Render | O(n) | O(n) | n = output size |

**Overall:** O(n + v log v) time, O(n + v) space

**Typical Performance:**
- 1KB file: <10ms
- 10KB file: <100ms
- 100KB file: <1s

---

## Security Considerations

### 1. Hermetic Imports

**Threat:** Arbitrary file access
**Mitigation:** Whitelist-based import resolution

### 2. Resource Limits

**Threat:** DoS via infinite loops
**Mitigation:** Gas model, cycle detection

### 3. Code Injection

**Threat:** Malicious lens execution
**Mitigation:** Sandboxed lens execution, no `eval()`

### 4. Memory Safety

**Threat:** Buffer overflows
**Mitigation:** Rust memory safety guarantees

---

## Testing Strategy

**Unit Tests:** Each crate independently
**Integration Tests:** Full pipeline
**Error Tests:** All 29 error codes
**Fuzzing:** Random input generation
**Property Tests:** Invariant verification

**Coverage:** 100% of error paths

---

## Extensibility Points

### 1. Custom Lenses

Implement `Lens` trait:

```rust
pub trait Lens: Send + Sync {
    fn name(&self) -> &str;
    fn signature(&self) -> LensSignature;
    fn execute(&self, input: &ValueNode, args: &[ValueNode]) -> LensResult;
}
```

### 2. Custom Renderers

Implement renderer for new vendor:

```rust
pub fn render_myvendor(doc: &FacetDocument) -> Result<String> {
    // Custom JSON format
}
```

### 3. Custom Type Validators

Add constraint types in `fct-validator`

### 4. WASM Target

Compile to WebAssembly for browser/Node.js

---

## Future Architecture

### Planned Improvements

1. **Incremental Compilation** - Cache parsed AST
2. **LSP Server** - IDE integration
3. **JIT Compilation** - Hot path optimization
4. **Distributed Execution** - Multi-machine R-DAG
5. **Advanced Caching** - Lens result memoization

---

## References

- [FACET v2.0 Specification](../facet2-specification.md)
- [Error Reference](errors.md)
- [Lens Reference](lenses.md)
- [Testing Guide](testing.md)
- [CLI Reference](cli.md)

## Next Steps

🎯 **Continue Learning:**
- **[04-type-system.md](04-type-system.md)** - Facet Type System (FTS) deep dive
- **[05-examples-guide.md](05-examples-guide.md)** - Practical examples explained
- **[07-api-reference.md](07-api-reference.md)** - Rust API documentation

🔧 **Implementation Details:**
- **[08-lenses.md](08-lenses.md)** - Lens library and transformations
- **[09-testing.md](09-testing.md)** - @test blocks and validation
- **[10-performance.md](10-performance.md)** - Optimization and scaling

📚 **References:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete technical specification
- **[PRD](../facetparcer.prd)** - Product requirements document
- **[README.md](../README.md)** - Project overview

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09

- **[05-examples-guide.md](05-examples-guide.md)** - Practical examples explained
- **[07-api-reference.md](07-api-reference.md)** - Rust API documentation

🔧 **Implementation Details:**
- **[08-lenses.md](08-lenses.md)** - Lens library and transformations
- **[09-testing.md](09-testing.md)** - @test blocks and validation
- **[10-performance.md](10-performance.md)** - Optimization and scaling

📚 **References:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete technical specification
- **[PRD](../facetparcer.prd)** - Product requirements document
- **[README.md](../README.md)** - Project overview

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09
