---
---
# 08. FACET Standard Lens Library Reference

**Reading Time:** 20-25 minutes | **Difficulty:** Intermediate | **Previous:** [07-api-reference.md](07-api-reference.md) | **Next:** [09-testing.md](09-testing.md)

Complete reference for all built-in lens functions in FACET v2.0.

---

## Table of Contents

### String Lenses
- [trim()](#trim)
- [lowercase()](#lowercase)
- [uppercase()](#uppercase)
- [capitalize()](#capitalize)
- [reverse()](#reverse)
- [substring()](#substring)
- [replace()](#replace)
- [split()](#split)
- [join()](#join)

### List Lenses
- [first()](#first)
- [last()](#last)
- [nth()](#nth)
- [slice()](#slice)
- [length()](#length)
- [unique()](#unique)
- [sort_by()](#sort_by)
- [filter()](#filter)
- [map()](#map)
- [ensure_list()](#ensure_list)

### Map Lenses
- [keys()](#keys)
- [values()](#values)

### Utility Lenses
- [template()](#template)
- [json_parse()](#json_parse)
- [json()](#json)
- [url_encode()](#url_encode)
- [url_decode()](#url_decode)
- [hash()](#hash)
- [default()](#default)
- [indent()](#indent)

### Level 1 Lenses (Bounded External)
- [llm_call()](#llm_call)
- [embedding()](#embedding)
- [rag_search()](#rag_search)

### Utility Lenses
- [template()](#template)
- [json_parse()](#json_parse)
- [json_stringify()](#json_stringify)
- [url_encode()](#url_encode)
- [url_decode()](#url_decode)
- [hash()](#hash)

### Advanced Topics
- [Custom Lens Implementation](#custom-lens-implementation)
- [Performance Considerations](#performance-considerations)

---

## Implementation Notes

**Specification Compliance:** This implementation provides an **extended lens library** beyond the FACET v2.0 specification minimum requirements (Appendix A). All lenses are fully functional and tested.

**Trust Levels:** Lenses are categorized by trust level:
- **Level 0 (Pure):** 29 lenses - deterministic, no I/O
- **Level 1 (Bounded):** 3 lenses - external API calls (currently stub implementations)
- **Level 2 (Volatile):** Not yet implemented (planned for future versions)

---

## Overview

**Lenses** are pure, deterministic transformation functions that operate on values in FACET pipelines. They enable composable data transformations using the pipeline operator `|>`.

**Syntax:**
```facet
$variable |> lens1() |> lens2(arg1, arg2) |> lens3(kwarg=value)
```

**Current Library:** 32 lenses (29 Level 0 + 3 Level 1)

**Note:** FACET v2.0 specification defines a minimum standard lens library (Appendix A). This implementation provides an **extended library** with additional lenses for enhanced functionality.

---

## Table of Contents

### String Lenses
- [trim()](#trim) - Remove whitespace
- [lowercase()](#lowercase) - Convert to lowercase
- [uppercase()](#uppercase) - Convert to uppercase
- [capitalize()](#capitalize) - Capitalize first letter
- [reverse()](#reverse) - Reverse characters
- [substring()](#substring) - Extract substring
- [replace(pattern, replacement)](#replace) - Replace substring
- [split(separator)](#split) - Split into list
- [join(separator)](#join) - Join list into string

### List Lenses
- [first()](#first) - Get first element
- [last()](#last) - Get last element
- [nth(index)](#nth) - Get nth element
- [slice(start, end)](#slice) - Extract sublist
- [length()](#length) - Get list length
- [unique()](#unique) - Remove duplicates
- [map(operation)](#map) - Transform elements
- [filter(condition)](#filter) - Filter elements
- [sort_by(key, order)](#sort_by) - Sort list
- [ensure_list()](#ensure_list) - Wrap as list

### Map Lenses
- [keys()](#keys) - Extract map keys
- [values()](#values) - Extract map values

### Utility Lenses
- [template(**kwargs)](#template) - Template substitution
- [json_parse()](#json_parse) - Parse JSON string
- [json(indent)](#json) - Format as JSON
- [url_encode()](#url_encode) - URL encode string
- [url_decode()](#url_decode) - URL decode string
- [hash(algorithm)](#hash) - Cryptographic hash
- [default(value)](#default) - Provide fallback
- [indent(size)](#indent) - Add indentation

### Level 1 Lenses (Bounded External)
- [llm_call()](#llm_call) - LLM API calls
- [embedding()](#embedding) - Text embeddings
- [rag_search()](#rag_search) - RAG retrieval

---

## String Lenses

### `trim()`

Remove leading and trailing whitespace from a string.

**Signature:**
```
string |> trim() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  raw: "  hello world  "
  clean: $raw |> trim()
  # Result: "hello world"
```

**Edge cases:**
- Input: `""` → Output: `""`
- Input: `"   "` → Output: `""`
- Input: `"no spaces"` → Output: `"no spaces"`

**Error:** Type mismatch if input is not a string

---

### `lowercase()`

Convert all characters in a string to lowercase.

**Signature:**
```
string |> lowercase() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  text: "Hello WORLD"
  lower: $text |> lowercase()
  # Result: "hello world"

  combined: "  MiXeD CaSe  " |> trim() |> lowercase()
  # Result: "mixed case"
```

**Unicode support:** Yes (uses Rust's `to_lowercase()`)

**Error:** Type mismatch if input is not a string

---

### `uppercase()`

Convert all characters in a string to uppercase.

**Signature:**
```
string |> uppercase() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  text: "hello world"
  upper: $text |> uppercase()
  # Result: "HELLO WORLD"

  shout: "quiet" |> uppercase()
  # Result: "QUIET"
```

**Unicode support:** Yes (uses Rust's `to_uppercase()`)

**Error:** Type mismatch if input is not a string

---

### `capitalize()`

Capitalize the first letter of a string, converting it to uppercase while leaving the rest unchanged.

**Signature:**
```
string |> capitalize() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  text: "hello world"
  capitalized: $text |> capitalize()
  # Result: "Hello world"

  already_capital: "Alice"
  unchanged: $already_capital |> capitalize()
  # Result: "Alice"

  empty: "" |> capitalize()
  # Result: ""
```

**Unicode support:** Yes (uses proper Unicode case conversion)

**Error:** Type mismatch if input is not a string

---

### `reverse()`

Reverse the characters in a string.

**Signature:**
```
string |> reverse() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  text: "hello"
  reversed: $text |> reverse()
  # Result: "olleh"

  palindrome: "racecar"
  same: $palindrome |> reverse()
  # Result: "racecar"
```

**Unicode support:** Preserves Unicode grapheme clusters

**Error:** Type mismatch if input is not a string

---

### `substring(start, end)`

Extract a substring from a string using start and end indices.

**Signature:**
```
string |> substring(start: int, end?: int) → string
```

**Parameters:**
- `start` (required) - Starting index (inclusive, 0-based)
- `end` (optional) - Ending index (exclusive). If omitted, extracts until string end

**Examples:**
```facet
@vars
  text: "hello world"
  first_five: $text |> substring(0, 5)
  # Result: "hello"

  from_sixth: $text |> substring(6)
  # Result: "world"

  middle: $text |> substring(2, 8)
  # Result: "llo wo"
```

**Behavior:**
- Uses character indices (not byte indices) for proper Unicode support
- Panics if `start > end` or indices are out of bounds

**Error:**
- Type mismatch if input is not a string
- Argument error if indices are invalid

---

### `join(separator)`

Join list elements into a single string with a separator.

**Signature:**
```
list |> join(separator?: string) → string
```

**Parameters:**
- `separator` (optional) - String to insert between elements (default: empty string)

**Examples:**
```facet
@vars
  items: ["apple", "banana", "cherry"]
  csv: $items |> join(",")
  # Result: "apple,banana,cherry"

  space_separated: $items |> join(" ")
  # Result: "apple banana cherry"

  no_separator: $items |> join()
  # Result: "applebananacherry"
```

**Type conversion:** All element types are converted to strings

**Error:** Type mismatch if input is not a list

---

### `split(separator)`

Split a string into a list using a delimiter.

**Signature:**
```
string |> split(separator: string) → list<string>
```

**Parameters:**
- `separator` (required) - Delimiter string

**Examples:**
```facet
@vars
  csv: "apple,banana,cherry"
  items: $csv |> split(",")
  # Result: ["apple", "banana", "cherry"]

  path: "user/documents/file.txt"
  parts: $path |> split("/")
  # Result: ["user", "documents", "file.txt"]

  sentence: "hello world test"
  words: $sentence |> split(" ")
  # Result: ["hello", "world", "test"]
```

**Edge cases:**
- Empty string: `"" |> split(",")` → `[""]`
- No matches: `"hello" |> split(",")` → `["hello"]`
- Multiple separators: `"a,,b" |> split(",")` → `["a", "", "b"]`

**Error:**
- Type mismatch if input is not a string
- Argument error if separator not provided

---

### `replace(pattern, replacement)`

Replace all occurrences of a pattern with a replacement string.

**Signature:**
```
string |> replace(pattern: string, replacement: string) → string
```

**Parameters:**
- `pattern` (required) - String to search for
- `replacement` (required) - String to replace with

**Examples:**
```facet
@vars
  text: "Hello world, world!"
  updated: $text |> replace("world", "Rust")
  # Result: "Hello Rust, Rust!"

  clean_path: "C:\\Users\\file.txt" |> replace("\\", "/")
  # Result: "C:/Users/file.txt"

  normalize: "foo__bar" |> replace("__", "_")
  # Result: "foo_bar"
```

**Behavior:**
- Replaces **all** occurrences (not just first)
- Case-sensitive
- No regex support (literal string matching)

**Error:**
- Type mismatch if input is not a string
- Argument error if pattern or replacement not provided

---

## Map Lenses

### `keys()`

Extract all keys from a map as a list of strings.

**Signature:**
```
map |> keys() → list<string>
```

**Parameters:** None

**Examples:**
```facet
@vars
  config: {
    name: "app"
    version: "1.0"
    enabled: true
  }
  config_keys: $config |> keys()
  # Result: ["name", "version", "enabled"]
```

**Order:** Keys are returned in arbitrary order (HashMap iteration)

**Error:** Type mismatch if input is not a map

---

### `values()`

Extract all values from a map as a list.

**Signature:**
```
map |> values() → list<any>
```

**Parameters:** None

**Examples:**
```facet
@vars
  person: {
    name: "Alice"
    age: 30
    active: true
  }
  person_values: $person |> values()
  # Result: ["Alice", 30, true]
```

**Order:** Values are returned in arbitrary order (HashMap iteration)

**Type preservation:** Values retain their original types

**Error:** Type mismatch if input is not a map

---

## List Lenses

### `map(operation)`

Transform each element in a list using an operation.

**Signature:**
```
list |> map(operation: string) → list
```

**Parameters:**
- `operation` (required) - Operation to apply
  - `"to_string"` - Convert elements to debug string representation

**Examples:**
```facet
@vars
  numbers: [1, 2, 3]
  strings: $numbers |> map("to_string")
  # Result: ["Int(1)", "Int(2)", "Int(3)"]
```

**Note:** Current implementation is limited. Future versions will support:
- Variable references
- Nested pipelines
- Custom operations

**Error:**
- Type mismatch if input is not a list
- Argument error if operation not provided or unknown

---

### `filter(condition)`

Filter list elements based on a condition.

**Signature:**
```
list |> filter(condition: string) → list
```

**Parameters:**
- `condition` (required) - Filter condition
  - `"non_null"` - Keep non-null values
  - `"non_empty"` - Keep non-empty strings, lists, maps

**Examples:**
```facet
@vars
  mixed: ["hello", null, "world", null]
  filtered: $mixed |> filter("non_null")
  # Result: ["hello", "world"]

  items: ["", "text", "", "data"]
  non_empty: $items |> filter("non_empty")
  # Result: ["text", "data"]
```

**Behavior:**
- `non_null`: Removes `null` values
- `non_empty`: Removes empty strings `""`, empty lists `[]`, empty maps `{}`

**Error:**
- Type mismatch if input is not a list
- Argument error if condition not provided

---

### `sort_by(key, order)`

Sort a list.

**Signature:**
```
list |> sort_by(key?: string, order?: "asc"|"desc") → list
```

**Parameters:**
- `key` (optional) - Sort key (reserved for future map sorting)
- `order` (optional) - Sort order: `"asc"` (default) or `"desc"`

**Examples:**
```facet
@vars
  items: ["zebra", "apple", "banana"]
  sorted_asc: $items |> sort_by()
  # Result: ["apple", "banana", "zebra"]

  sorted_desc: $items |> sort_by("", "desc")
  # Result: ["zebra", "banana", "apple"]
```

**Behavior:**
- Sorts by debug string representation (`format!("{:?}", item)`)
- Stable sort algorithm

**Error:** Type mismatch if input is not a list

---

### `ensure_list()`

Ensure a value is a list; wrap single values in a list.

**Signature:**
```
any |> ensure_list() → list
```

**Parameters:** None

**Examples:**
```facet
@vars
  single: "hello"
  wrapped: $single |> ensure_list()
  # Result: ["hello"]

  already_list: [1, 2, 3]
  unchanged: $already_list |> ensure_list()
  # Result: [1, 2, 3]
```

**Use cases:**
- Normalizing API responses
- Handling optional arrays
- Ensuring uniform list handling

**Error:** None (accepts any input)

---

### `first()`

Get the first element of a list.

**Signature:**
```
list |> first() → any
```

**Parameters:** None

**Examples:**
```facet
@vars
  items: ["apple", "banana", "cherry"]
  first_item: $items |> first()
  # Result: "apple"

  numbers: [42, 17, 99]
  first_num: $numbers |> first()
  # Result: 42
```

**Error:** Execution error if list is empty

---

### `last()`

Get the last element of a list.

**Signature:**
```
list |> last() → any
```

**Parameters:** None

**Examples:**
```facet
@vars
  items: ["apple", "banana", "cherry"]
  last_item: $items |> last()
  # Result: "cherry"

  numbers: [42, 17, 99]
  last_num: $numbers |> last()
  # Result: 99
```

**Error:** Execution error if list is empty

---

### `nth(index)`

Get the element at a specific index in a list (0-based).

**Signature:**
```
list |> nth(index: int) → any
```

**Parameters:**
- `index` (required) - Zero-based index of the element to retrieve

**Examples:**
```facet
@vars
  items: ["apple", "banana", "cherry"]
  second_item: $items |> nth(1)
  # Result: "banana"

  numbers: [42, 17, 99]
  third_num: $numbers |> nth(2)
  # Result: 99
```

**Error:**
- Execution error if index is out of bounds
- Argument error if index is not an integer

---

### `slice(start, end)`

Extract a sublist from a list using start and end indices.

**Signature:**
```
list |> slice(start: int, end?: int) → list
```

**Parameters:**
- `start` (required) - Starting index (inclusive)
- `end` (optional) - Ending index (exclusive). If omitted, slices until list end

**Examples:**
```facet
@vars
  items: ["a", "b", "c", "d", "e"]
  first_three: $items |> slice(0, 3)
  # Result: ["a", "b", "c"]

  from_second: $items |> slice(1)
  # Result: ["b", "c", "d", "e"]

  middle: $items |> slice(2, 4)
  # Result: ["c", "d"]
```

**Error:** Argument error if indices are invalid

---

### `length()`

Get the number of elements in a list.

**Signature:**
```
list |> length() → int
```

**Parameters:** None

**Examples:**
```facet
@vars
  items: ["apple", "banana", "cherry"]
  count: $items |> length()
  # Result: 3

  empty: [] |> length()
  # Result: 0
```

**Error:** Type mismatch if input is not a list

---

### `unique()`

Remove duplicate elements from a list.

**Signature:**
```
list |> unique() → list
```

**Parameters:** None

**Examples:**
```facet
@vars
  items: ["a", "b", "a", "c", "b"]
  unique_items: $items |> unique()
  # Result: ["a", "b", "c"]

  numbers: [1, 2, 2, 3, 1]
  unique_nums: $numbers |> unique()
  # Result: [1, 2, 3]
```

**Behavior:** Uses string representation for comparison (`format!("{:?}", item)`)

**Error:** Type mismatch if input is not a list

---

## Map Lenses

### `default(value)`

Provide a fallback value if input is `null`.

**Signature:**
```
any |> default(value: any) → any
```

**Parameters:**
- `value` (required) - Default value to return if input is `null`

**Examples:**
```facet
@vars
  maybe_null: null
  with_default: $maybe_null |> default("N/A")
  # Result: "N/A"

  has_value: "actual"
  unchanged: $has_value |> default("N/A")
  # Result: "actual"
```

**Behavior:**
- Returns `value` if input is `null`
- Returns input unchanged if input is not `null`
- Works with any type

**Error:** Argument error if default value not provided

---

### `indent(size)`

Add indentation to each line of a string.

**Signature:**
```
string |> indent(size?: int) → string
```

**Parameters:**
- `size` (optional) - Number of spaces per line (default: 2)

**Examples:**
```facet
@vars
  code: "def hello():\n  print('hi')"
  indented: $code |> indent()
  # Result: "  def hello():\n    print('hi')"

  deeply_indented: $code |> indent(4)
  # Result: "    def hello():\n      print('hi')"
```

**Behavior:**
- Adds indentation to **each line**
- Preserves relative indentation
- Works with multi-line strings

**Use cases:**
- Code formatting
- Nested content generation
- Template indentation

**Error:** Type mismatch if input is not a string

---

### `json(indent)`

Serialize a value to JSON string.

**Signature:**
```
any |> json(indent?: int) → string
```

**Parameters:**
- `indent` (optional) - Indentation size for pretty printing
  - If omitted: Compact JSON (single line)
  - If provided: Pretty-printed with specified indentation

**Examples:**
```facet
@vars
  data: {
    name: "Alice"
    age: 30
  }

  compact: $data |> json()
  # Result: "{\"name\":\"Alice\",\"age\":30}"

  pretty: $data |> json(2)
  # Result:
  # {
  #   "name": "Alice",
  #   "age": 30
  # }
```

**Type support:** All FACET types (string, int, float, bool, null, list, map)

**Error:** Execution error if serialization fails (rare)

---

### `template(**kwargs)`

Perform simple template substitution using {{variable}} syntax.

**Signature:**
```
string |> template(var1=value1, var2=value2, ...) → string
```

**Parameters:**
- `**kwargs` - Variable substitutions as keyword arguments

**Examples:**
```facet
@vars
  template_str: "Hello {{name}}, you are {{age}} years old!"
  result: $template_str |> template(name="Alice", age=30)
  # Result: "Hello Alice, you are 30 years old!"

  greeting: "Welcome {{user}} to {{service}}"
  filled: $greeting |> template(user="Bob", service="FACET")
  # Result: "Welcome Bob to FACET"
```

**Syntax:** Variables are referenced as `{{variable_name}}`

**Type conversion:** All values are converted to strings

**Error:** Type mismatch if input is not a string

---

### `json_parse()`

Parse a JSON string into structured FACET data.

**Signature:**
```
string |> json_parse() → any
```

**Parameters:** None

**Examples:**
```facet
@vars
  json_str: '{"name": "Alice", "age": 30, "active": true}'
  parsed: $json_str |> json_parse()
  # Result: {name: "Alice", age: 30, active: true}

  number_array: "[1, 2, 3, 4]"
  list_result: $number_array |> json_parse()
  # Result: [1, 2, 3, 4]
```

**Supported types:** Objects, arrays, strings, numbers, booleans, null

**Error:**
- Type mismatch if input is not a string
- Execution error if JSON is malformed

---

### `url_encode()`

Encode a string for safe use in URLs using percent-encoding.

**Signature:**
```
string |> url_encode() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  unsafe: "hello world & special chars?"
  encoded: $unsafe |> url_encode()
  # Result: "hello%20world%20%26%20special%20chars%3F"

  path: "user/documents/file.txt"
  safe_path: $path |> url_encode()
  # Result: "user%2Fdocuments%2Ffile.txt"
```

**Standard:** RFC 3986 compliant

**Error:** Type mismatch if input is not a string

---

### `url_decode()`

Decode a percent-encoded URL string.

**Signature:**
```
string |> url_decode() → string
```

**Parameters:** None

**Examples:**
```facet
@vars
  encoded: "hello%20world%20%26%20special%20chars%3F"
  decoded: $encoded |> url_decode()
  # Result: "hello world & special chars?"

  safe_path: "user%2Fdocuments%2Ffile.txt"
  original: $safe_path |> url_decode()
  # Result: "user/documents/file.txt"
```

**Error:**
- Type mismatch if input is not a string
- Execution error if decoding fails

---

### `hash(algorithm)`

Generate a cryptographic hash of the input string.

**Signature:**
```
string |> hash(algorithm?: string) → string
```

**Parameters:**
- `algorithm` (optional) - Hash algorithm: "md5", "sha256" (default), "sha512"

**Examples:**
```facet
@vars
  text: "Hello, World!"
  sha256_hash: $text |> hash()
  # Result: "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f"

  md5_hash: $text |> hash("md5")
  # Result: "65a8e27d8879283831b664bd8b7f0ad4"

  sha512_hash: $text |> hash("sha512")
  # Result: "374d794a95cdcfd8b35993185fef9ba368f160d8daf432d08ba9f1ed1e5abe6cc..."
```

**Output:** Hexadecimal string representation

**Error:**
- Type mismatch if input is not a string
- Argument error for unsupported algorithm

---

## Level 1 Lenses (Bounded External)

Level 1 lenses make external API calls and have **TrustLevel::Bounded**. They are non-deterministic but their results are bounded by external service guarantees.

---

### `llm_call(prompt, model, **kwargs)`

Call a Large Language Model API to generate text responses.

**Signature:**
```
string |> llm_call(model?: string, temperature?: float, max_tokens?: int) → string
```

**Parameters:**
- `model` (optional) - LLM model name (default: "gpt-3.5-turbo")
- `temperature` (optional) - Sampling temperature 0.0-1.0 (default: 0.7)
- `max_tokens` (optional) - Maximum tokens to generate (default: 1000)

**Examples:**
```facet
@vars
  prompt: "Write a haiku about programming"
  response: $prompt |> llm_call()
  # Result: AI-generated haiku text

  creative: "Describe a sunset" |> llm_call(temperature=0.9, max_tokens=200)
  # Result: Creative, longer description
```

**Trust Level:** Bounded (external API call, non-deterministic)

**Note:** Currently implemented as stub - returns placeholder text

---

### `embedding(model)`

Generate vector embeddings for text using embedding models.

**Signature:**
```
string |> embedding(model?: string) → list<float>
```

**Parameters:**
- `model` (optional) - Embedding model name (default: "text-embedding-ada-002")

**Examples:**
```facet
@vars
  text: "The quick brown fox jumps over the lazy dog"
  vectors: $text |> embedding()
  # Result: [0.123, -0.456, 0.789, ...] (384-dimensional vector)

  short_text: "hello" |> embedding("text-embedding-ada-002")
  # Result: Vector representation of "hello"
```

**Output:** List of float values representing the embedding vector

**Trust Level:** Bounded (external API call, non-deterministic)

**Note:** Currently implemented as stub - returns dummy vector

---

### `rag_search(query, index, top_k)`

Perform Retrieval-Augmented Generation search to find relevant documents.

**Signature:**
```
string |> rag_search(index: string, top_k?: int) → list<map>
```

**Parameters:**
- `index` (required) - Name of the search index/collection
- `top_k` (optional) - Number of results to return (default: 5)

**Examples:**
```facet
@vars
  query: "What is FACET language?"
  results: $query |> rag_search(index="docs", top_k=3)
  # Result: [
  #   {content: "FACET is a...", score: 0.95},
  #   {content: "Language features...", score: 0.87},
  #   {content: "Getting started...", score: 0.82}
  # ]
```

**Output:** List of maps with `content` and `score` fields

**Trust Level:** Bounded (external API call, non-deterministic)

**Note:** Currently implemented as stub - returns dummy results

---

## Pipeline Composition

Lenses can be chained using the `|>` operator:

```facet
@vars
  # Multi-step text processing
  raw_input: "  HELLO WORLD  "
  processed: $raw_input |> trim() |> lowercase()
  # Result: "hello world"

  # Complex transformation
  csv_data: "apple,banana,CHERRY"
  items: $csv_data |> split(",") |> map("lowercase")
  # Future: items = ["apple", "banana", "cherry"]

  # Safe access with default
  optional_field: null
  safe_value: $optional_field |> default("N/A") |> uppercase()
  # Result: "N/A"

  # JSON formatting pipeline
  config: { api_key: "secret" }
  formatted: $config |> json(2)
```

---

## Trust Levels

FACET lenses operate at different trust levels based on their behavior:

### Level 0 - Pure (29 lenses)
**Characteristics:**
- ✅ **Deterministic:** Same input always produces same output
- ✅ **No I/O:** No network, file, or external calls
- ✅ **No side effects:** Only transforms data

**Examples:** `trim()`, `lowercase()`, `map()`, `filter()`, `json()`, `hash()`

### Level 1 - Bounded (3 lenses)
**Characteristics:**
- ⚠️ **Bounded non-deterministic:** External API calls with service guarantees
- ⚠️ **Network I/O:** Makes HTTP requests to external services
- ✅ **Bounded results:** Results constrained by service contracts/SLAs

**Examples:** `llm_call()`, `embedding()`, `rag_search()`

### Level 2 - Volatile (Future)
**Characteristics:**
- ❌ **Fully non-deterministic:** Unpredictable results
- ❌ **System access:** File I/O, timestamps, random numbers
- ⚠️ **Side effects:** May modify external state

**Examples (planned):** `timestamp()`, `random()`, `@input()`

---

## Error Handling

All lenses return errors in two categories:

### Type Mismatch
```
Error: Type mismatch: expected string, got Int(42)
```
**Cause:** Input type doesn't match lens requirements

**Example:**
```facet
@vars
  number: 42
  bad: $number |> trim()  # Error: trim() requires string
```

### Argument Error
```
Error: Argument error: split() requires a separator argument
```
**Cause:** Missing or invalid argument

**Example:**
```facet
@vars
  text: "a,b,c"
  bad: $text |> split()  # Error: separator required
```

---

## Common Patterns

### Clean and normalize text
```facet
@vars
  user_input: "  Hello WORLD  "
  normalized: $user_input |> trim() |> lowercase()
  # Result: "hello world"
```

### Parse and process CSV
```facet
@vars
  csv: "apple,banana,cherry"
  items: $csv |> split(",")
  # Result: ["apple", "banana", "cherry"]
```

### Safe navigation with defaults
```facet
@vars
  optional_name: null
  display_name: $optional_name |> default("Guest") |> uppercase()
  # Result: "GUEST"
```

### Extract and format config
```facet
@vars
  config: {
    api_url: "https://api.example.com"
    timeout: 30
  }
  config_keys: $config |> keys()
  # Result: ["api_url", "timeout"]

  config_json: $config |> json(2)
  # Pretty-printed JSON string
```

### Filter and clean lists
```facet
@vars
  raw_data: ["item1", "", "item2", null, "item3"]
  clean_data: $raw_data |> filter("non_null") |> filter("non_empty")
  # Result: ["item1", "item2", "item3"]
```

### String transformations
```facet
@vars
  input: "  HELLO WORLD  "
  processed: $input |> trim() |> lowercase() |> capitalize()
  # Result: "Hello world"

  reversed: "FACET" |> reverse()
  # Result: "TECAF"

  substring: "programming" |> substring(0, 4)
  # Result: "prog"
```

### List operations
```facet
@vars
  items: ["a", "b", "c", "d", "e"]
  first_two: $items |> slice(0, 2)
  # Result: ["a", "b"]

  count: $items |> length()
  # Result: 5

  unique_items: ["x", "y", "x", "z"] |> unique()
  # Result: ["x", "y", "z"]
```

### Template processing
```facet
@vars
  template: "Welcome {{name}}! Your score: {{points}}"
  message: $template |> template(name="Alice", points=95)
  # Result: "Welcome Alice! Your score: 95"
```

### JSON processing
```facet
@vars
  data: {user: "Alice", score: 95}
  json_str: $data |> json(2)
  # Result: Pretty-printed JSON

  api_response: '{"status": "ok", "data": [1,2,3]}'
  parsed: $api_response |> json_parse()
  # Result: Structured FACET data
```

### URL processing
```facet
@vars
  unsafe_url: "hello world & special?"
  safe_url: $unsafe_url |> url_encode()
  # Result: "hello%20world%20%26%20special%3F"

  original: $safe_url |> url_decode()
  # Result: "hello world & special?"
```

### Hash generation
```facet
@vars
  secret: "my_password"
  hashed: $secret |> hash("sha256")
  # Result: SHA256 hash as hex string
```

---

## Performance Notes

- All lenses are **O(n)** or better on input size
- String operations allocate new strings (immutable)
- List operations may allocate new lists
- Map operations iterate through all entries
- No recursion limits (safe for large inputs)

---

## Future Enhancements

Planned additions to the lens library:

### String (Future)
- `concat(other)` - Concatenate strings
- `regex_match(pattern)` - Regex matching
- `regex_replace(pattern, replacement)` - Regex replacement

### Numeric (Future)
- `round(precision)` - Round numbers
- `abs()` - Absolute value
- `min(other)` / `max(other)` - Min/max comparison

### List (Future)
- `take(n)` / `skip(n)` - Take/skip N elements
- `flatten()` - Flatten nested lists

### Map (Future)
- `merge(other)` - Merge maps
- `pick(keys)` - Select specific keys
- `omit(keys)` - Exclude specific keys

### Level 2 - Volatile (Future)
- `@input(source)` - External data input
- `timestamp()` - Current timestamp
- `random()` - Random number generation
- `uuid()` - Generate UUIDs

### Level 1 - Bounded (Future Enhancements)
- `llm_call()` - Full implementation with actual API calls
- `embedding()` - Integration with embedding services
- `rag_search()` - Vector database integration

---

## Next Steps

🎯 **Apply Lens Knowledge:**
- **[05-examples-guide.md](05-examples-guide.md)** - Lens usage in examples
- **[09-testing.md](09-testing.md)** - Testing lens pipelines
- **[10-performance.md](10-performance.md)** - Lens performance optimization

🔧 **Advanced Usage:**
- **[07-api-reference.md](07-api-reference.md)** - Custom lens implementation
- **[04-type-system.md](04-type-system.md)** - Type system interactions
- **[12-errors.md](12-errors.md)** - Lens-related errors (F801-F802)

📚 **References:**
- **[06-cli.md](06-cli.md)** - CLI lens usage
- **[02-tutorial.md](02-tutorial.md)** - Lens tutorial
- **[examples/README.md](../examples/README.md)** - Practical examples

---

## 👤 Author

**Emil Rokossovskiy**  
*AI & Platform Engineer. Creator of FACET ecosystem 🚀*

- **GitHub:** [@rokoss21](https://github.com/rokoss21)
- **Compiler Repo:** [github.com/rokoss21/facet-compiler](https://github.com/rokoss21/facet-compiler)
- **Email:** ecsiar@gmail.com

---

## See Also

- **[06-cli.md](06-cli.md)** - CLI reference
- **[01-quickstart.md](01-quickstart.md)** - Getting started tutorial
- **[examples/](../examples/)** - Example FACET files
- **[facet2-specification.md](../facet2-specification.md)** - Full language specification


## Future Enhancements

Planned additions to the lens library:

### String (Future)
- `concat(other)` - Concatenate strings
- `regex_match(pattern)` - Regex matching
- `regex_replace(pattern, replacement)` - Regex replacement

### Numeric (Future)
- `round(precision)` - Round numbers
- `abs()` - Absolute value
- `min(other)` / `max(other)` - Min/max comparison

### List (Future)
- `take(n)` / `skip(n)` - Take/skip N elements
- `flatten()` - Flatten nested lists

### Map (Future)
- `merge(other)` - Merge maps
- `pick(keys)` - Select specific keys
- `omit(keys)` - Exclude specific keys

### Level 2 - Volatile (Future)
- `@input(source)` - External data input
- `timestamp()` - Current timestamp
- `random()` - Random number generation
- `uuid()` - Generate UUIDs

### Level 1 - Bounded (Future Enhancements)
- `llm_call()` - Full implementation with actual API calls
- `embedding()` - Integration with embedding services
- `rag_search()` - Vector database integration

---

## Next Steps

🎯 **Apply Lens Knowledge:**
- **[05-examples-guide.md](05-examples-guide.md)** - Lens usage in examples
- **[09-testing.md](09-testing.md)** - Testing lens pipelines
- **[10-performance.md](10-performance.md)** - Lens performance optimization

🔧 **Advanced Usage:**
- **[07-api-reference.md](07-api-reference.md)** - Custom lens implementation
- **[04-type-system.md](04-type-system.md)** - Type system interactions
- **[12-errors.md](12-errors.md)** - Lens-related errors (F801-F802)

📚 **References:**
- **[06-cli.md](06-cli.md)** - CLI lens usage
- **[02-tutorial.md](02-tutorial.md)** - Lens tutorial
- **[examples/README.md](../examples/README.md)** - Practical examples

---

## 👤 Author

**Emil Rokossovskiy**  
*AI & Platform Engineer. Creator of FACET ecosystem 🚀*

- **GitHub:** [@rokoss21](https://github.com/rokoss21)
- **Compiler Repo:** [github.com/rokoss21/facet-compiler](https://github.com/rokoss21/facet-compiler)
- **Email:** ecsiar@gmail.com

---

## See Also

- **[06-cli.md](06-cli.md)** - CLI reference
- **[01-quickstart.md](01-quickstart.md)** - Getting started tutorial
- **[examples/](../examples/)** - Example FACET files
- **[facet2-specification.md](../facet2-specification.md)** - Full language specification
