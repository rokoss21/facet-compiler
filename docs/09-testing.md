---
---
# 09. FACET v2.0 Testing Guide

**Reading Time:** 15-20 minutes | **Difficulty:** Intermediate | **Previous:** [08-lenses.md](08-lenses.md) | **Next:** [10-performance.md](10-performance.md)

**Specification Compliance:** Implements FACET v2.0 specification Section 13 (@test blocks) with extended features.

---

## Table of Contents

- [Overview](#overview)
- [Syntax](#syntax)
- [Variable Overrides](#variable-overrides)
- [Mocking](#mocking)
- [Assertions](#assertions)
- [Running Tests](#running-tests)
- [Test Results](#test-results)
- [Best Practices](#best-practices)
- [Examples](#examples)
- [Reference](#reference)

---

## Overview

FACET v2.0 includes a built-in testing system using `@test` blocks. These blocks allow you to:
- Define test inputs and variable overrides
- Mock external interfaces and lenses
- Assert expected outputs and telemetry
- Run automated test suites

## Syntax

```facet
@test "test name"
  vars:
    # Override @input values
    variable_name: value
    
  mock:
    # Mock interface calls
    InterfaceName.method: { return: value }
    
    # Mock lens calls
    lens_name: { return: value }
    
  assert:
    # Assertions on output and telemetry
    - output contains "expected text"
    - cost < 0.01
    - tokens < 100
    - sentiment "positive"
```

## Test Sections

### vars Section
Override `@input` values for the test:

```facet
@test "user input test"
  vars:
    user_query: "What is the weather today?"
    user_location: "New York"
```

### mock Section
Define mock return values for interfaces and lenses:

```facet
@test "with mocked weather API"
  mock:
    WeatherAPI.get_current: { temp: 25, condition: "Sunny" }
    WeatherAPI.get_forecast: { 
      forecast: ["sunny", "sunny", "cloudy"]
    }
    llm_complete: { text: "The weather will be sunny." }
```

### assert Section
Define assertions to validate test results:

#### Output Assertions
- `contains` - Check if output contains text
- `not_contains` - Check if output doesn't contain text
- `equals` - Check exact match
- `matches` - Regex pattern match

#### Telemetry Assertions
- `cost < value` - Check estimated cost
- `tokens < value` - Check token usage
- `time < value` - Check execution time (ms)
- `gas < value` - Check gas consumption

#### Sentiment Assertion
- `sentiment "positive|negative|neutral"` - Basic sentiment analysis

#### Boolean Assertions
- `true` - Value should be truthy
- `false` - Value should be falsy
- `null` - Value should be null
- `not_null` - Value should not be null

## Examples

### Basic Test
```facet
@vars
  user_query: @input(type="string")
  
@system
  role: "helpful assistant"
  
@user
  $user_query

@test "basic response"
  vars:
    user_query: "Hello, world!"
    
  assert:
    - output contains "Hello"
    - output sentiment "positive"
    - cost < 0.01
```

### Test with Mocks
```facet
@vars
  user_query: @input(type="string")
  location: @input(type="string")
  
@system
  role: "weather assistant"
  tools: [$WeatherAPI]
  
@user
  "What's the weather in $location?"

@test "weather query"
  vars:
    user_query: "What's the weather in Tokyo?"
    location: "Tokyo"
    
  mock:
    WeatherAPI.get_current: { temp: 22, condition: "Clear" }
    
  assert:
    - output contains "22"
    - output contains "Clear"
    - tokens < 50
```

### Complex Test with Multiple Assertions
```facet
@test "comprehensive test"
  vars:
    prompt: "Summarize this text"
    text: "This is a very long text that needs summarization..."
    
  mock:
    llm_summarize: { 
      summary: "This text is about summarization."
      tokens_used: 150
    }
    
  assert:
    - output contains "summarization"
    - output not contains "very long text"
    - tokens < 200
    - cost < 0.05
    - time < 5000
    - sentiment "neutral"
```

## Running Tests

### Command Line
```bash
# Run all tests
fct test -i document.facet

# Run specific test
fct test -i document.facet --filter "weather query"

# Verbose output
fct test -i document.facet --output verbose

# JSON output for CI/CD
fct test -i document.facet --output json

# Custom resource limits
fct test -i document.facet --budget 8192 --gas-limit 20000
```

### Output Formats

#### Summary (default)
```
Test Results:
  ✓ basic response
  ✓ weather query
  ✓ comprehensive test

Total: 3
Passed: 3
```

#### Verbose
```
🧪 Test: basic response
✅ PASSED (150ms, 45 tokens, $0.0012)
  ✓ Expected output to contain 'Hello'
  ✓ Expected sentiment 'positive'
  ✓ Expected cost < 0.01

🧪 Test: weather query
✅ PASSED (200ms, 52 tokens, $0.0015)
  ✓ Expected output to contain '22'
  ✓ Expected output to contain 'Clear'
  ✓ Expected tokens < 50

Summary: 2/2 passed
```

#### JSON
```json
{
  "tests": [
    {
      "name": "basic response",
      "passed": true,
      "assertions": [...],
      "telemetry": {...},
      "error": null
    }
  ],
  "summary": {
    "total": 2,
    "passed": 2,
    "failed": 0
  }
}
```

## Best Practices

1. **Descriptive Test Names** - Use clear, descriptive names
2. **Specific Assertions** - Test both positive and negative cases
3. **Mock External Dependencies** - Ensure tests are deterministic
4. **Resource Limits** - Set reasonable cost and token limits
5. **Isolation** - Each test should be independent

## Integration with CI/CD

```yaml
# GitHub Actions example
- name: Run FACET tests
  run: |
    fct test -i document.facet --output json > test-results.json
    
- name: Check test results
  run: |
    jq -e '.summary.failed == 0' test-results.json
```

## Troubleshooting

### Common Issues

1. **Mock Not Found**
   ```
   Error: Missing mock for Level 1 lens: llm_complete
   ```
   Solution: Add mock for the lens in the test's `mock` section

2. **Assertion Failed**
   ```
   ✗ Expected output to contain 'umbrella'
       Actual: It's sunny today!
   ```
   Solution: Check mock values or adjust assertion

3. **Resource Limit Exceeded**
   ```
   ✗ Expected cost < 0.01
       Actual: 0.015
   ```
   Solution: Adjust test expectations or optimize the FACET document

## Next Steps

🎯 **Testing in Practice:**
- **[05-examples-guide.md](05-examples-guide.md)** - Complete testing example
- **[07-api-reference.md](07-api-reference.md)** - TestRunner API
- **[10-performance.md](10-performance.md)** - Testing performance

🔧 **Advanced Testing:**
- **[11-security.md](11-security.md)** - Security testing
- **[12-errors.md](12-errors.md)** - Test-related errors
- **[13-import-system.md](13-import-system.md)** - Testing imported modules

📚 **Resources:**
- **[test_example.facet](../examples/test_example.facet)** - Complete test example
- **[faq.md](../faq.md)** - Testing FAQs

---

## Reference

See [FACET v2.0 Specification](../facet2-specification.md) for complete details on:
- [Section 13: Testing (@test)](../facet2-specification.md#13-testing-test)
- [Error Codes](../facet2-specification.md#appendix-c-normative-error-code-catalog)
- [Execution Model](../facet2-specification.md#7-execution-model)
   ```
   ✗ Expected output to contain 'umbrella'
       Actual: It's sunny today!
   ```
   Solution: Check mock values or adjust assertion

3. **Resource Limit Exceeded**
   ```
   ✗ Expected cost < 0.01
       Actual: 0.015
   ```
   Solution: Adjust test expectations or optimize the FACET document

## Next Steps

🎯 **Testing in Practice:**
- **[05-examples-guide.md](05-examples-guide.md)** - Complete testing example
- **[07-api-reference.md](07-api-reference.md)** - TestRunner API
- **[10-performance.md](10-performance.md)** - Testing performance

🔧 **Advanced Testing:**
- **[11-security.md](11-security.md)** - Security testing
- **[12-errors.md](12-errors.md)** - Test-related errors
- **[13-import-system.md](13-import-system.md)** - Testing imported modules

📚 **Resources:**
- **[test_example.facet](../examples/test_example.facet)** - Complete test example
- **[faq.md](../faq.md)** - Testing FAQs

---

## Reference

See [FACET v2.0 Specification](../facet2-specification.md) for complete details on:
- [Section 13: Testing (@test)](../facet2-specification.md#13-testing-test)
- [Error Codes](../facet2-specification.md#appendix-c-normative-error-code-catalog)
- [Execution Model](../facet2-specification.md#7-execution-model)