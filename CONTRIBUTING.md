# Contributing to FACET v2.0 Compiler

Thank you for your interest in contributing to FACET! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

- Be respectful and constructive in all interactions
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

### Prerequisites

- Rust 1.70+ ([install rustup](https://rustup.rs/))
- Git
- Familiarity with Rust and compiler design concepts

### Development Setup

```bash
# Clone the repository
git clone https://github.com/rokoss21/facet-compiler.git
cd facet-fct

# Build the project
cargo build

# Run tests
cargo test --all

# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features -- -D warnings
```

## Development Workflow

### 1. Pick an Issue

- Check the [issue tracker](https://github.com/rokoss21/facet-compiler/issues)
- Look for issues labeled `good first issue` or `help wanted`
- Comment on the issue to indicate you're working on it

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

### 3. Make Your Changes

Follow these guidelines:

#### Code Style

- **Formatting**: Run `cargo fmt --all` before committing
- **Linting**: Run `cargo clippy` and fix all warnings
- **Naming**: Use clear, descriptive names for variables and functions
- **Comments**: Add comments for complex logic, but prefer self-documenting code

#### Testing

- Add tests for new functionality
- Ensure all existing tests pass: `cargo test --all`
- Aim for comprehensive test coverage:
  - Unit tests for individual functions
  - Integration tests for module interactions
  - Error path tests (test failure scenarios)

#### Documentation

- Update relevant documentation in `docs/`
- Add rustdoc comments for public APIs
- Update README.md if adding new features

### 4. Commit Your Changes

Use clear, descriptive commit messages:

```
type(scope): Brief description

Longer explanation if needed.

Fixes #123
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `test`: Adding or updating tests
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `chore`: Build process or tooling changes

Examples:
```
feat(parser): Add support for float literals

Implements scientific notation (1.23e10) and standard floats (3.14).

Closes #45
```

```
fix(engine): Remove debug println!() from production code

Debug statements were accidentally left in the R-DAG execution path.
```

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a Pull Request on GitHub with:
- Clear title describing the change
- Description explaining:
  - What problem does this solve?
  - How does it solve it?
  - Any breaking changes?
- Reference to related issues

## Pull Request Guidelines

### PR Checklist

Before submitting, ensure:

- [ ] Code follows project style guidelines (`cargo fmt`, `cargo clippy`)
- [ ] All tests pass (`cargo test --all`)
- [ ] New tests added for new functionality
- [ ] Documentation updated (README, docs/, rustdoc comments)
- [ ] Commit messages are clear and descriptive
- [ ] PR description explains the changes
- [ ] No unrelated changes included

### Review Process

1. Maintainers will review your PR
2. Address any requested changes
3. Once approved, a maintainer will merge your PR

## Project Structure

```
FACET2/
├── crates/
│   ├── fct-ast/        # AST definitions
│   ├── fct-parser/     # Parser (nom-based)
│   ├── fct-resolver/   # Import resolution
│   ├── fct-validator/  # Type checker (FTS)
│   ├── fct-engine/     # R-DAG + Token Box Model
│   ├── fct-std/        # Standard lens library
│   └── fct-render/     # JSON renderer
├── src/main.rs         # CLI entry point
├── docs/               # Documentation
└── examples/           # Example .facet files
```

## Areas for Contribution

### Good First Issues

- Adding new lenses to `fct-std`
- Improving error messages
- Adding more tests
- Documentation improvements
- Example `.facet` files

### Advanced Contributions

- Parser improvements (error recovery, better diagnostics)
- Type system enhancements
- Performance optimizations
- WASM target support
- Vendor-specific renderers (Anthropic, Llama)

## Testing Guidelines

### Unit Tests

Place unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

### Integration Tests

Create files in `tests/` directory:

```rust
// tests/integration_test.rs
use fct_engine::*;

#[test]
fn test_end_to_end() {
    // Full pipeline test
}
```

## Error Codes

FACET uses structured error codes:

- **F001-F099**: Parser errors
- **F401-F499**: Resolver errors
- **F451-F499**: Validator errors
- **F501-F599**: Engine errors
- **F801-F899**: Lens errors
- **F901-F999**: Token Box Model errors

When adding new errors, use the next available code in the appropriate range.

## Documentation

### Rustdoc Comments

```rust
/// Brief description of the function.
///
/// More detailed explanation with examples:
///
/// # Examples
///
/// ```
/// use fct_std::trim;
/// assert_eq!(trim("  hello  "), "hello");
/// ```
///
/// # Errors
///
/// Returns `LensError` if...
pub fn my_function() -> Result<()> {
    // ...
}
```

## Getting Help

- Open an issue for questions
- Join discussions in issues and PRs
- Check existing documentation in `docs/`

## License

By contributing, you agree that your contributions will be licensed under both the MIT License and Apache License 2.0 (dual-licensed).

## Recognition

Contributors will be acknowledged in:
- `CONTRIBUTORS.md` file
- Release notes
- Project documentation

Thank you for contributing to FACET! 🚀
