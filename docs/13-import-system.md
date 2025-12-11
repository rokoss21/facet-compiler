# 13. FACET Import System Guide

**Reading Time:** 20-25 minutes | **Difficulty:** Intermediate | **Previous:** [12-errors.md](12-errors.md) | **Next:** [faq.md](../faq.md)

**Version:** 0.1.0
**Status:** Production Ready
**Last Updated:** 2025-12-09

---

## Table of Contents

- [Overview](#overview)
- [Basic Usage](#basic-usage)
- [Import Resolution](#import-resolution)
- [Block Merging](#block-merging)
- [Circular Import Detection](#circular-import-detection)
- [Best Practices](#best-practices)
- [Error Handling](#error-handling)
- [Advanced Patterns](#advanced-patterns)

---

## Overview

FACET's **import system** enables modular, reusable agent configurations. Break large files into smaller, focused modules and compose them together.

### Key Features

**Modularity:**
- Split configurations into logical components
- Reuse common definitions across multiple agents
- Keep codebases organized and maintainable

**Safety:**
- Circular import detection (F602)
- File existence validation (F601)
- Hermetic resolution (no network access)
- Deterministic merging

**Performance:**
- Cached parsed ASTs
- Shallow dependency trees (max depth: 10)
- Incremental resolution

---

## Basic Usage

### Syntax

```facet
@import "path/to/file.facet"
```

**Rules:**
- Must be a string literal
- Relative paths from importing file
- `.facet` extension required

### Example 1: Simple import

**File:** `common.facet`

```facet
@var_types
  username: "string"

@vars
  app_name: "My AI App"
```

**File:** `main.facet`

```facet
@import "common.facet"

@vars
  username: "Alice"
  greeting: "Welcome to $app_name, $username!"

@user
  query: $greeting
```

**Result:**

```
"Welcome to My AI App, Alice!"
```

### Example 2: Multiple imports

```facet
@import "types.facet"
@import "config.facet"
@import "prompts.facet"

@vars
  # Use definitions from all imported files
  ...
```

**Import Order:** Files processed in declaration order

---

## Import Resolution

### Resolution Algorithm

```
1. Parse current file
2. For each @import directive:
   a. Resolve path relative to current file
   b. Check if file exists → F601 if not
   c. Check for circular import → F602 if detected
   d. Parse imported file recursively
   e. Collect all blocks from imported file
3. Merge all blocks (imports first, then current file)
4. Return unified AST
```

### Path Resolution

**Relative Paths:**

```facet
@import "./utils.facet"        # Same directory
@import "../common.facet"      # Parent directory
@import "../../shared.facet"   # Grandparent directory
```

**Absolute Paths:**

```facet
@import "/opt/facet/stdlib/logging.facet"  # Unix
@import "C:/facet/stdlib/logging.facet"    # Windows
```

**Search Paths (Future):**

```bash
# Set FACET_PATH environment variable
export FACET_PATH="/usr/local/facet/lib:/opt/facet/modules"

# Then use short names
@import "stdlib/logging.facet"  # Searches FACET_PATH
```

---

## Block Merging

### Merge Strategy

FACET uses **smart merging** for different block types:

| Block Type | Merge Strategy |
|------------|----------------|
| `@system` | Last wins (override) |
| `@user` | Last wins (override) |
| `@assistant` | Last wins (override) |
| `@context` | Append all |
| `@vars` | Merge by key (last wins per key) |
| `@var_types` | Merge by key (last wins per key) |
| `@test` | Append all |
| `@import` | Execute immediately |

### Example: Variable merging

**File:** `base.facet`

```facet
@vars
  a: "from base"
  b: "from base"
```

**File:** `override.facet`

```facet
@import "base.facet"

@vars
  b: "from override"  # Overrides base.b
  c: "from override"  # New variable
```

**Result:**

```facet
@vars
  a: "from base"       # From base.facet
  b: "from override"   # Overridden
  c: "from override"   # Added
```

### Example: Context appending

**File:** `ctx1.facet`

```facet
@context
  info: "Context 1"
```

**File:** `ctx2.facet`

```facet
@context
  info: "Context 2"
```

**File:** `main.facet`

```facet
@import "ctx1.facet"
@import "ctx2.facet"
```

**Result:**

```facet
@context
  info: "Context 1"

@context
  info: "Context 2"
```

**Both contexts preserved!**

---

## Circular Import Detection

### Detection Algorithm

```
maintain import_stack = []

function resolve_imports(file):
  if file in import_stack:
    raise F602("Circular import detected: " + file)

  import_stack.push(file)

  for each @import in file:
    resolve_imports(import_path)

  import_stack.pop()
```

### Example: Circular import

**File A:** `file_a.facet`

```facet
@import "file_b.facet"

@vars
  a_value: "from A"
```

**File B:** `file_b.facet`

```facet
@import "file_a.facet"  # ❌ Circular!

@vars
  b_value: "from B"
```

**Compiler Output:**

```
Error: F602: Circular import detected: file_b.facet
  Import chain: file_a.facet → file_b.facet → file_a.facet
```

### Fix: Extract shared definitions

**File:** `shared.facet`

```facet
@vars
  shared_value: "common"
```

**File A:**

```facet
@import "shared.facet"

@vars
  a_value: $shared_value
```

**File B:**

```facet
@import "shared.facet"

@vars
  b_value: $shared_value
```

**✓ No circular dependency!**

---

## Best Practices

### 1. Organize by Concern

```
project/
├── types/
│   ├── user.facet          # User type definitions
│   ├── product.facet       # Product types
│   └── order.facet         # Order types
├── prompts/
│   ├── system.facet        # System prompts
│   ├── greetings.facet     # Greeting templates
│   └── errors.facet        # Error messages
├── config/
│   ├── models.facet        # Model configurations
│   └── limits.facet        # Rate limits, budgets
└── main.facet              # Main entry point
```

### 2. Use Common Prefix

```
common/
├── common_types.facet
├── common_vars.facet
└── common_prompts.facet

# Import in main.facet
@import "common/common_types.facet"
@import "common/common_vars.facet"
@import "common/common_prompts.facet"
```

### 3. Version Imports

```facet
# Explicitly version dependencies
@import "stdlib/v1/logging.facet"
@import "myorg/agents/v2/customer_service.facet"
```

### 4. Document Dependencies

```facet
# File: agent.facet
# Dependencies:
#   - types/user.facet (user type definitions)
#   - config/models.facet (model configuration)
#   - prompts/greeting.facet (greeting templates)

@import "types/user.facet"
@import "config/models.facet"
@import "prompts/greeting.facet"

@vars
  ...
```

### 5. Limit Import Depth

**Maximum depth:** 10 levels

```facet
# Good: Shallow hierarchy
main.facet
  └─ types.facet

# Bad: Deep hierarchy (avoid)
main.facet
  └─ level1.facet
      └─ level2.facet
          └─ level3.facet
              └─ ... (too deep!)
```

---

## Error Handling

### F601: Import Not Found

**Cause:** Imported file doesn't exist

**Example:**

```facet
@import "nonexistent.facet"  # ❌ File not found
```

**Error Message:**

```
Error: F601: Import not found: nonexistent.facet
  --> main.facet:1:9
  |
1 | @import "nonexistent.facet"
  |         ^^^^^^^^^^^^^^^^^^^
```

**Fix:**

1. Check file path spelling
2. Verify file exists
3. Use correct relative path

### F602: Circular Import

**Cause:** Import cycle detected

**Example:**

```facet
# File A imports B, B imports A
```

**Error Message:**

```
Error: F602: Circular import detected: file_b.facet
  Import chain: file_a.facet → file_b.facet → file_a.facet
  --> file_b.facet:1:9
```

**Fix:**

1. Extract shared code to separate file
2. Restructure dependencies
3. Use dependency injection pattern

---

## Advanced Patterns

### Pattern 1: Conditional Imports (Future)

```facet
# Currently not supported, but planned:
@import @if($env == "prod", "prod.facet", "dev.facet")
```

### Pattern 2: Wildcard Imports (Future)

```facet
# Currently not supported, but planned:
@import "types/*.facet"  # Import all .facet files in types/
```

### Pattern 3: Selective Imports (Future)

```facet
# Currently not supported, but planned:
@import "utils.facet" {
  only: ["trim_string", "format_date"]
}
```

### Pattern 4: Aliased Imports (Future)

```facet
# Currently not supported, but planned:
@import "long/path/to/config.facet" as config

@vars
  timeout: $config.timeout
```

### Pattern 5: Dependency Injection

**File:** `base_agent.facet`

```facet
@var_types
  provider: "string"

@vars
  provider: @input {type: "string", name: "provider"}
```

**File:** `openai_agent.facet`

```facet
@import "base_agent.facet"

@vars
  provider: "openai"
  model: "gpt-4"
```

**File:** `anthropic_agent.facet`

```facet
@import "base_agent.facet"

@vars
  provider: "anthropic"
  model: "claude-3"
```

---

## Performance Characteristics

### Import Resolution Cost

| Operation | Time Complexity | Notes |
|-----------|----------------|-------|
| File lookup | O(1) | Filesystem cache |
| Parse import | O(n) | n = file size |
| Merge blocks | O(b) | b = block count |
| Cycle detection | O(d) | d = import depth |

**Overall:** O(f * n) where f = files, n = avg file size

**Typical Performance:**
- 10 imports: <50ms
- 50 imports: <200ms
- 100 imports: <500ms

### Optimization Tips

1. **Cache parsed files** - Parse once, reuse
2. **Limit import depth** - Keep hierarchy shallow
3. **Use specific imports** - Avoid importing everything
4. **Profile import time** - Use `--verbose` flag

```bash
$ fct build --input main.facet --verbose
Resolving imports...
  ✓ common.facet (12ms)
  ✓ types.facet (8ms)
  ✓ config.facet (5ms)
Total import time: 25ms
```

---

## Security Considerations

### Hermetic Imports

**Threat:** Arbitrary file access
**Mitigation:** Whitelist-based resolution

```facet
# ✓ Allowed: Relative paths
@import "./utils.facet"
@import "../common.facet"

# ❌ Blocked: Absolute system paths (unless whitelisted)
@import "/etc/passwd"

# ❌ Blocked: Network URLs
@import "https://evil.com/malicious.facet"
```

### Sandboxed Resolution

**Properties:**
- No network access
- No exec permissions
- Read-only filesystem
- Limited to whitelisted directories

### Dependency Scanning

```bash
# Future: scan for vulnerabilities
$ fct audit --input main.facet

Scanning dependencies...
  ✓ common.facet (safe)
  ⚠ external.facet (untrusted source)
  ❌ vulnerable.facet (known CVE-2024-1234)
```

---

## Migration Guide

### From FACET v1.x

**v1.x Syntax:**

```facet
# No import system in v1.x
# All definitions in single file
```

**v2.0 Syntax:**

```facet
# Extract to separate files
@import "definitions.facet"
@import "prompts.facet"
```

**Migration Steps:**

1. Identify logical groupings
2. Extract to separate files
3. Add @import directives
4. Test for circular dependencies
5. Validate output unchanged

---

## Troubleshooting

### Issue 1: Import not resolving

**Symptom:** F601 error

**Debug:**

```bash
$ fct build --input main.facet --verbose
Error: F601: Import not found: utils.facet
  Searched paths:
    - ./utils.facet (not found)
    - ../utils.facet (not found)
```

**Solution:** Check file path and spelling

### Issue 2: Unexpected variable values

**Symptom:** Variables have wrong values

**Debug:** Check merge order

```bash
$ fct inspect --input main.facet --show-imports

Import order:
  1. base.facet
  2. override.facet

Variable 'name':
  - From base.facet: "Alice"
  - From override.facet: "Bob"  ← Final value
```

**Solution:** Reorder imports or rename variables

### Issue 3: Circular dependency

**Symptom:** F602 error

**Debug:**

```bash
$ fct build --input main.facet --verbose
Error: F602: Circular import detected
  Import chain:
    main.facet
    → a.facet
    → b.facet
    → a.facet (circular!)
```

**Solution:** Extract shared code to separate file

---

## Future Enhancements

### Planned Features

1. **Conditional imports** - Import based on environment
2. **Wildcard imports** - Import multiple files with glob
3. **Selective imports** - Import only specific definitions
4. **Import aliases** - Rename imported definitions
5. **Package manager** - Centralized module registry
6. **Version constraints** - Semantic versioning support

---

## References

## Next Steps

🎯 **Import Mastery:**
- **[05-examples-guide.md](05-examples-guide.md)** - Import usage in examples
- **[06-cli.md](06-cli.md)** - CLI import handling
- **[faq.md](../faq.md)** - Import FAQs and troubleshooting

🔧 **Advanced Topics:**
- **[03-architecture.md](03-architecture.md)** - Import system architecture
- **[07-api-reference.md](07-api-reference.md)** - Import API
- **[11-security.md](11-security.md)** - Import security

📚 **Resources:**
- **[02-tutorial.md](02-tutorial.md)** - Import tutorial
- **[12-errors.md](12-errors.md)** - Import errors (F601-F602)

---

**References:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete import specification
- **[12-errors.md](12-errors.md)** - Import error codes
- **[02-tutorial.md](02-tutorial.md)** - Import tutorial
- **[03-architecture.md](03-architecture.md)** - Import architecture
- **[04-type-system.md](04-type-system.md)** - Cross-file type checking

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09

- **[11-security.md](11-security.md)** - Import security

📚 **Resources:**
- **[02-tutorial.md](02-tutorial.md)** - Import tutorial
- **[12-errors.md](12-errors.md)** - Import errors (F601-F602)

---

**References:**
- **[facet2-specification.md](../facet2-specification.md)** - Complete import specification
- **[12-errors.md](12-errors.md)** - Import error codes
- **[02-tutorial.md](02-tutorial.md)** - Import tutorial
- **[03-architecture.md](03-architecture.md)** - Import architecture
- **[04-type-system.md](04-type-system.md)** - Cross-file type checking

---

**Author:** Emil Rokossovskiy
**License:** MIT / Apache-2.0
**Last Updated:** 2025-12-09
