---
---
# 12. FACET v2.0 Error Codes Reference

**Reading Time:** 15-20 minutes | **Difficulty:** Intermediate | **Previous:** [11-security.md](11-security.md) | **Next:** [13-import-system.md](13-import-system.md)

This document provides a comprehensive reference for all error codes in FACET v2.0, including examples and troubleshooting guidance.

---

## Table of Contents

- [Syntax Errors (F001-F003)](#syntax-errors-f001-f003)
- [Semantic Errors (F401-F453)](#semantic-errors-f401-f453)
- [Graph Errors (F505)](#graph-errors-f505)
- [Import Errors (F601-F602)](#import-errors-f601-f602)
- [Runtime Errors (F801-F902)](#runtime-errors-f801-f902)

---

## Syntax Errors (F001-F003)

Syntax errors occur during parsing when the FACET code doesn't conform to the language grammar.

### F001: Invalid Indentation

**Description**: Indentation must be consistent and use either 2 spaces or 4 spaces.

**Example**:
```facet
@system
   role: "assistant"  // ❌ 3 spaces - invalid
```

**Fix**: Use consistent 2 or 4 spaces:
```facet
@system
  role: "assistant"  // ✅ 2 spaces
```

### F002: Tabs Not Allowed

**Description**: TAB characters are not allowed for indentation.

**Example**:
```facet
@system
\trole: "assistant"  // ❌ Tab character
```

**Fix**: Use spaces instead of tabs:
```facet
@system
  role: "assistant"  // ✅ Spaces
```

### F003: Unclosed Delimiter

**Description**: A bracket, brace, or quote is not properly closed.

**Examples**:
```facet
@vars
  list: [1, 2, 3  // ❌ Missing ]
  map: {"key": "value"  // ❌ Missing }
  text: "unclosed  // ❌ Missing "
```

**Fix**: Close all delimiters:
```facet
@vars
  list: [1, 2, 3]  // ✅
  map: {"key": "value"}  // ✅
  text: "closed"  // ✅
```

---

## Semantic Errors (F401-F453)

Semantic errors occur during validation when the code is syntactically correct but has logical issues.

### F401: Variable Not Found

**Description**: Reference to a variable that doesn't exist.

**Example**:
```facet
@vars
  name: "Alice"
  greeting: "Hello, $undefined_name!"  // ❌ undefined_name doesn't exist
```

**Fix**: Define the variable or check spelling:
```facet
@vars
  name: "Alice"
  greeting: "Hello, $name!"  // ✅
```

### F402: Type Inference Failed

**Description**: The compiler cannot determine the type of an expression.

**Example**:
```facet
@vars
  ambiguous: null |> unknown_lens()  // ❌ Cannot infer type
```

**Fix**: Provide explicit type or more context:
```facet
@var_types
  value: string

@vars
  value: "hello" |> trim()  // ✅ Type can be inferred
```

### F404: Forward Reference

**Description**: Variable is used before it's declared.

**Example**:
```facet
@vars
  b: $a  // ❌ 'a' used before declaration
  a: "value"
```

**Fix**: Declare variables before using them:
```facet
@vars
  a: "value"
  b: $a  // ✅
```

### F451: Type Mismatch

**Description**: Value doesn't match the declared type.

**Example**:
```facet
@var_types
  age: int

@vars
  age: "not-a-number"  // ❌ String assigned to int
```

**Fix**: Use correct type or change declaration:
```facet
@var_types
  age: string  // ✅ Changed to string

@vars
  age: "25"
```

### F452: Constraint Violation

**Description**: Value violates type constraints.

**Example**:
```facet
@var_types
  age: {
    type: "int"
    min: 0
    max: 120
  }

@vars
  age: 150  // ❌ Exceeds max constraint
```

**Fix**: Use value within constraints:
```facet
@vars
  age: 25  // ✅ Within range
```

### F453: Input Validation Failed

**Description**: Missing or invalid @input directive configuration.

**Example**:
```facet
@vars
  query: @input {name: "query"}  // ❌ Missing type
```

**Fix**: Provide required type argument:
```facet
@vars
  query: @input {type: "string", name: "query"}  // ✅
```

---

## Graph Errors (F505)

Graph errors occur when analyzing variable dependencies.

### F505: Cyclic Dependency

**Description**: Variables depend on each other in a cycle.

**Examples**:
```facet
@vars
  a: $b  // ❌ Direct cycle
  b: $a
```

```facet
@vars
  a: $b  // ❌ Indirect cycle
  b: $c
  c: $a
```

**Fix**: Break the cycle by restructuring dependencies:
```facet
@vars
  base: "common_value"
  a: $base
  b: $base
```

---

## Import Errors (F601-F602)

Import errors occur when handling module imports.

### F601: Import Not Found

**Description**: The imported file cannot be found.

**Example**:
```facet
@import "nonexistent.facet"  // ❌ File doesn't exist
```

**Fix**: Check file path and ensure file exists:
```facet
@import "./utils.facet"  // ✅ Correct path
```

### F602: Circular Import

**Description**: Files import each other in a cycle.

**Example**:
```facet
# File A
@import "file_b.facet"
```

```facet
# File B
@import "file_a.facet"  // ❌ Circular import
```

**Fix**: Use a third file for shared definitions or restructure imports.

---

## Runtime Errors (F801-F902)

Runtime errors occur during execution.

### F801: Lens Execution Failed

**Description**: A lens function failed during execution.

**Example**:
```facet
@vars
  text: "hello"
  result: $text |> split()  // ❌ split() requires delimiter
```

**Fix**: Provide required arguments:
```facet
@vars
  text: "a,b,c"
  result: $text |> split(",")  // ✅
```

### F802: Unknown Lens

**Description**: Reference to a lens that doesn't exist.

**Example**:
```facet
@vars
  text: "hello"
  result: $text |> nonexistent_lens()  // ❌ Lens doesn't exist
```

**Fix**: Use correct lens name or implement custom lens:
```facet
@vars
  text: "hello"
  result: $text |> trim()  // ✅
```

### F901: Budget Exceeded

**Description**: Token budget for critical sections exceeded.

**Example**:
```bash
# Running with budget of 100 tokens
$ fct run document.facet --budget 100  # ❌ Document needs more
```

**Fix**: Increase budget or optimize document:
```bash
$ fct run document.facet --budget 1000  # ✅
```

### F902: Gas Exhausted

**Description**: Computation gas limit exceeded during variable evaluation.

**Example**:
```bash
# Running with gas limit of 10
$ fct run document.facet --gas-limit 10  # ❌ Complex pipeline needs more
```

**Fix**: Increase gas limit or optimize computation:
```bash
$ fct run document.facet --gas-limit 1000  # ✅
```

---

## Troubleshooting Guide

### Common Patterns

1. **Indentation Errors**: Always use spaces, never tabs. Pick 2 or 4 spaces and be consistent.

2. **Variable References**: 
   - Check spelling
   - Ensure variables are declared before use
   - Use `$` prefix for variable references

3. **Type Issues**:
   - Match value types to declarations
   - Check constraint ranges
   - Use explicit types when inference fails

4. **Dependencies**:
   - Avoid circular references
   - Structure variables as a DAG (Directed Acyclic Graph)
   - Use base variables for shared values

### Debugging Tips

1. **Use verbose mode**:
   ```bash
   $ fct run document.facet --verbose
   ```

2. **Check syntax first**:
   ```bash
   $ fct parse document.facet
   ```

3. **Validate types**:
   ```bash
   $ fct validate document.facet
   ```

4. **Run with higher limits for testing**:
   ```bash
   $ fct run document.facet --budget 10000 --gas-limit 10000
   ```

---

## Error Code Quick Reference

| Code | Category | Description |
|------|----------|-------------|
| F001 | Syntax | Invalid indentation |
| F002 | Syntax | Tabs not allowed |
| F003 | Syntax | Unclosed delimiter |
| F401 | Semantic | Variable not found |
| F402 | Semantic | Type inference failed |
| F404 | Semantic | Forward reference |
| F451 | Semantic | Type mismatch |
| F452 | Semantic | Constraint violation |
| F453 | Semantic | Input validation failed |
| F505 | Graph | Cyclic dependency |
| F601 | Import | Import not found |
| F602 | Import | Circular import |
| F801 | Runtime | Lens execution failed |
| F802 | Runtime | Unknown lens |
| F901 | Runtime | Budget exceeded |
| F902 | Runtime | Gas exhausted |

## Next Steps

🎯 **Error Resolution:**
- **[06-cli.md](06-cli.md)** - CLI error handling
- **[07-api-reference.md](07-api-reference.md)** - Programmatic error handling
- **[09-testing.md](09-testing.md)** - Testing error conditions

🔧 **System Components:**
- **[13-import-system.md](13-import-system.md)** - Import-related errors (F601-F602)
- **[08-lenses.md](08-lenses.md)** - Lens-related errors (F801-F802)
- **[04-type-system.md](04-type-system.md)** - Type-related errors (F451-F453)

📚 **Resources:**
- **[faq.md](../faq.md)** - Common error troubleshooting
- **[test_example.facet](../examples/test_example.facet)** - Error testing examples

---

For more information about specific errors, see the [FACET v2.0 Specification](../facet2-specification.md).
| F901 | Runtime | Budget exceeded |
| F902 | Runtime | Gas exhausted |

## Next Steps

🎯 **Error Resolution:**
- **[06-cli.md](06-cli.md)** - CLI error handling
- **[07-api-reference.md](07-api-reference.md)** - Programmatic error handling
- **[09-testing.md](09-testing.md)** - Testing error conditions

🔧 **System Components:**
- **[13-import-system.md](13-import-system.md)** - Import-related errors (F601-F602)
- **[08-lenses.md](08-lenses.md)** - Lens-related errors (F801-F802)
- **[04-type-system.md](04-type-system.md)** - Type-related errors (F451-F453)

📚 **Resources:**
- **[faq.md](../faq.md)** - Common error troubleshooting
- **[test_example.facet](../examples/test_example.facet)** - Error testing examples

---

For more information about specific errors, see the [FACET v2.0 Specification](../facet2-specification.md).