---
permalink: /11-security.html
---

# 11. FACET Security Model & Best Practices
**Reading Time:** 20-25 minutes | **Difficulty:** Intermediate | **Previous:** [10-performance.md](10-performance.html) | **Next:** [12-errors.md](12-errors.html)
**Version:** 0.1.2
**Last Updated:** 2026-04-02
**Status:** Spec-aligned guidance

## Table of Contents

- [Security Principles](#security-principles)
- [Security Boundaries](#security-boundaries)
- [Threat Model](#threat-model)
- [Secure Configuration](#secure-configuration)
- [Input Validation](#input-validation)
- [Execution Safety](#execution-safety)
- [Network Security](#network-security)
- [Error Handling](#error-handling)
- [Audit Logging](#audit-logging)
- [Compliance Posture](#compliance-posture)
- [Security Checklist](#security-checklist)

---

## Security Principles

FACET provides deterministic, fail-closed controls around LLM execution boundaries. Security is implemented as explicit constraints in parsing, typing, policy, and runtime guard decisions.

### Core Security Tenets

**1. Deterministic Execution**
- Deterministic compilation and canonical rendering for fixed normalized inputs
- Deterministic merge, R-DAG traversal, and policy decision order
- Reproducible execution artifacts for auditing (when enabled)

**2. Resource Bounding**
- Token budget enforcement prevents runaway costs
- Gas limits prevent infinite computation
- Memory bounds prevent DoS attacks
- Timeout controls prevent resource exhaustion

**3. Type Safety**
- Static type checking prevents type confusion attacks
- Constraint validation prevents malformed inputs
- Runtime type enforcement prevents injection attacks

**4. Hermetic Execution**
- No external network access during compilation/type-check phases
- File access restricted to import sandbox during resolution
- Isolated execution environment
- Minimal attack surface

---

## Security Boundaries

### FACET Controls Directly

- Parsing and normalization guarantees (UTF-8, NFC, LF, no tabs)
- Import sandbox (`allowlisted roots`, no absolute paths, no `..`, no URLs)
- Type constraints and placement checks
- Policy/guard fail-closed enforcement for guarded operations
- Deterministic canonical request assembly and bounded layout

### FACET Does Not Control Directly

- Model hallucinations or provider-side sampling behavior
- Trustworthiness of external APIs/tools beyond declared effects
- Organization-level legal certifications by itself

FACET is a control layer for execution correctness and evidence. It is not a standalone compliance certificate.

---

## Threat Model

### Attack Vectors

**1. Malicious Input Attacks**
- Malformed FACET files causing crashes
- Type confusion exploits
- Constraint bypass attacks
- Parser exploits

**2. Resource Exhaustion Attacks**
- Infinite loops via circular dependencies
- Memory exhaustion via large inputs
- Token budget bypass
- CPU exhaustion via complex computations

**3. Information Disclosure**
- Error messages leaking sensitive data
- AST exposure in error outputs
- Debug information leakage
- Execution telemetry exposure

**4. Injection Attacks**
- Template injection in string literals
- Lens pipeline injection
- Import path manipulation
- Variable name conflicts

### Threat Actors

- **Malicious Users:** Attempting to crash or exploit the system
- **Insider Threats:** Authorized users abusing privileges
- **Supply Chain Attacks:** Compromised dependencies or imports
- **Network Attacks:** MITM on import resolution

---

## Secure Configuration

### Production Deployment

```bash
# Example host-level controls (implementation-specific)
export FACET_MODE=exec
export FACET_PROFILE=hypervisor
export FACET_IMPORT_ROOTS=/srv/facet/modules
export FACET_AUDIT_LOG=/var/log/facet/execution.json
```

### Security Headers

```json
{
  "security": {
    "level": "strict",
    "audit": {
      "enabled": true,
      "log_level": "detailed",
      "destination": "/secure/audit/facets.log"
    },
    "network": {
      "isolation": true,
      "allowed_domains": ["api.openai.com", "api.anthropic.com"],
      "certificate_validation": true
    },
    "execution": {
      "timeout_ms": 30000,
      "max_memory_mb": 512,
      "gas_limit": 100000
    }
  }
}
```

---

## Input Validation

### FACET File Validation

**1. Syntax Validation**
```bash
# Validate syntax only (safe for untrusted input)
fct validate --input malicious.facet --syntax-only
```

**2. Type Checking**
```bash
# Full type checking with constraint validation
fct validate --input user_input.facet --strict
```

**3. Import Security**
- Import paths must be relative to allowlisted roots
- Absolute paths and `..` traversal are rejected (`F601`)
- File existence validation before import
- Circular import detection

### Runtime Input Validation

**1. Context Variables**
```rust
// Validate context before execution
let validated_context = validator.validate_context(user_input)?;
engine.execute_with_context(validated_context)?;
```

**2. User Queries**
```facet
@input
  query: @input(type="string", pattern="^[a-zA-Z0-9\\s]{1,500}$", required=true)
```

---

## Execution Safety

### Resource Limits

**1. Token Budgeting**
```facet
@system
  budget: 4096  // Maximum tokens for execution
  gas_limit: 10000  // Computation gas limit
```

**2. Time Bounds**
```bash
# Set execution timeout
fct run --input agent.facet --timeout 30s --budget 4096
```

**3. Memory Limits**
```rust
let mut engine = RDagEngine::new();
engine.set_memory_limit(512 * 1024 * 1024)?; // 512MB
```

### Safe Execution Modes

**1. Sandbox Mode**
```bash
# Execute in isolated environment
fct run --input agent.facet --sandbox --no-network
```

**2. Dry Run Mode**
```bash
# Validate without actual API calls
fct run --input agent.facet --dry-run --mock-apis
```

---

## Network Security

### API Communication

**1. TLS Enforcement**
- All LLM API calls use TLS 1.3+
- Certificate validation enabled
- No insecure connections allowed

**2. Request Signing**
```rust
// Sign API requests for authenticity
let signed_request = security.sign_api_request(request)?;
client.send(signed_request)?;
```

**3. Rate Limiting**
```facet
@system
  rate_limit: {
    requests_per_minute: 60,
    burst_limit: 10
  }
```

### Import Resolution

**1. Secure Import Paths**
```facet
@import "trusted/library.facet"  // ✅ Relative to allowlisted root
@import "./local/utils.facet"    // ✅ Relative trusted
@import "../external.facet"      // ❌ Path traversal blocked
```

**2. Content Validation**
- SHA256 verification of imported files
- Digital signatures for trusted imports
- Content scanning for malicious patterns

---

## Error Handling

### Secure Error Messages

**1. Information Leakage Prevention**
```rust
// Safe error - no internal details
return Err("F401: Variable not found".to_string());

// Unsafe error - exposes internal state
return Err(format!("Variable '{}' not found in context {:?}", var, context));
```

**2. Error Level Configuration**
```bash
# Production: minimal error details
fct run --input agent.facet --error-level minimal

# Development: full stack traces
fct run --input agent.facet --error-level detailed
```

### Exception Safety

**1. Panic Prevention**
- All panics converted to controlled errors
- Resource cleanup on error paths
- No unhandled exceptions in production

**2. Graceful Degradation**
```rust
match result {
    Ok(output) => output,
    Err(e) if e.is_recoverable() => fallback_response,
    Err(e) => return secure_error_response(e.code()),
}
```

---

## Audit Logging

### Comprehensive Audit Trail

**1. Execution Logging**
```json
{
  "timestamp": "2025-12-09T10:30:00Z",
  "operation": "execute",
  "file": "agent.facet",
  "user": "service-account",
  "input_hash": "a1b2c3...",
  "output_hash": "d4e5f6...",
  "tokens_used": 1250,
  "execution_time_ms": 450,
  "success": true
}
```

**2. Security Events**
```json
{
  "level": "warning",
  "event": "constraint_violation",
  "code": "F452",
  "message": "Pattern constraint failed",
  "file": "user_input.facet",
  "line": 15,
  "details": {
    "field": "user_query",
    "expected_pattern": "^[a-zA-Z0-9\\s]+$",
    "actual_value": "malicious<script>alert('xss')</script>"
  }
}
```

### Log Security

**1. Tamper-Proof Logs**
- Cryptographic signing of log entries
- Append-only log files
- Integrity verification

**2. Log Rotation**
```bash
# Rotate logs securely
logrotate -s /var/log/facets.state /etc/logrotate.d/facets
```

---

## Compliance Posture

FACET provides technical controls that can support audits, but FACET itself is not a legal/compliance certification.

### Evidence FACET Can Provide

- Deterministic canonical request artifacts
- Typed policy configuration and policy hash
- Guard decision logs and hash chain (Hypervisor profile)
- Explicit error surface (`F*` and namespaced host diagnostics)

### Organization Responsibilities

- Map FACET controls to your internal control framework
- Maintain operational evidence (access, change management, incident response)
- Run independent legal/compliance assessments for your environment

---

## Security Checklist

### Pre-Deployment Checklist

- [ ] **TLS 1.3+** enabled for all API communications
- [ ] **Input validation** implemented for all user inputs
- [ ] **Resource limits** configured (tokens, gas, memory, time)
- [ ] **Audit logging** enabled with tamper-proof storage
- [ ] **Network isolation** active during execution
- [ ] **Error messages** sanitized for production
- [ ] **Import validation** prevents path traversal
- [ ] **Type checking** enabled for all FACET files

### Production Monitoring

- [ ] **Security event alerting** configured
- [ ] **Resource usage monitoring** active
- [ ] **Log integrity verification** automated
- [ ] **Regular security scans** scheduled
- [ ] **Incident response plan** documented and tested
- [ ] **Backup and recovery** procedures verified

### Continuous Security

- [ ] **Dependency updates** automated and tested
- [ ] **Security training** for development team
- [ ] **Code review security checklist** enforced
- [ ] **Penetration testing** quarterly schedule
- [ ] **Security metrics** tracked and reported

---

## Best Practices

### Development Security

**1. Code Review Requirements**
- Security review required for all changes
- Automated security scanning in CI/CD
- Threat modeling for new features

**2. Secure Coding Guidelines**
```rust
// ✅ Secure: bounded loops
for i in 0..max_iterations {
    if gas_exhausted() { break; }
    // safe computation
}

// ❌ Insecure: unbounded computation
loop {
    // potential DoS
}
```

### Operational Security

**1. Deployment Hardening**
```dockerfile
# Minimal container image
FROM scratch
COPY facet /facet
USER facetuser
EXPOSE 8080
```

**2. Service Mesh Integration**
- Mutual TLS between services
- Service identity verification
- Traffic encryption end-to-end

**3. Zero-Trust Architecture**
- Every request authenticated and authorized
- Minimal privilege principle
- Continuous verification

## Next Steps

- **[06-cli.md](06-cli.html)** - CLI runtime flags and operational controls
- **[12-errors.md](12-errors.html)** - Security-relevant error codes and failure modes
- **[13-import-system.md](13-import-system.html)** - Import sandbox and deterministic resolution
- **[09-testing.md](09-testing.html)** - Test patterns for policy and guard behavior
- **[FACET v2.1.3 specification](https://github.com/rokoss21/facet-compiler/blob/master/FACET-v2.1.3-Production-Language-Specification.md)** - Normative security and policy sections
