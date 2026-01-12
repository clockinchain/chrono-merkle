# Contributing to ChronoMerkle

Thank you for your interest in contributing to ChronoMerkle! We welcome contributions from the community. This document provides guidelines and information to help you contribute effectively.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Reporting Issues](#reporting-issues)
- [Documentation](#documentation)

## 🤝 Code of Conduct

This project follows a code of conduct to ensure a welcoming environment for all contributors. By participating, you agree to:

- Be respectful and inclusive
- Focus on constructive feedback
- Accept responsibility for mistakes
- Show empathy towards other contributors
- Help create a positive community

## 🚀 How to Contribute

### Types of Contributions

- **🐛 Bug Reports:** Report bugs and help us improve stability
- **✨ Feature Requests:** Suggest new features or enhancements
- **📖 Documentation:** Improve documentation, tutorials, or examples
- **🧪 Testing:** Add tests, improve test coverage, or fix failing tests
- **🔧 Code:** Submit fixes, optimizations, or new features
- **📊 Performance:** Optimize performance or add benchmarks

### Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally
3. **Create a feature branch** from `main`
4. **Make your changes** following our guidelines
5. **Add tests** for your changes
6. **Run the test suite** to ensure everything works
7. **Submit a pull request**

## 🛠️ Development Setup

### Prerequisites

- **Rust:** Version 1.70 or later
- **Cargo:** Latest stable version
- **Git:** Version 2.0 or later

### Clone and Setup

```bash
# Clone the repository
git clone https://github.com/Olyntar-Labs/chrono-merkle.git
cd chrono-merkle

# Run tests to ensure everything works
cargo test

# Run examples
cargo run --example basic_usage

# Generate documentation
cargo doc --open
```

### IDE Setup

We recommend using one of these IDEs:

- **VS Code** with rust-analyzer extension
- **CLion** with Rust plugin
- **IntelliJ IDEA** with Rust plugin

### Optional Dependencies

For full functionality, you may want to install:

- **PostgreSQL** for database storage features
- **Redis** for caching features
- **Criterion** for benchmarking (automatically handled by Cargo)

## 🏗️ Project Structure

```
chrono-merkle/
├── src/                    # Source code
│   ├── lib.rs             # Main library file
│   ├── error.rs           # Error types
│   ├── hash.rs            # Hash function abstractions
│   ├── node.rs            # Tree node definitions
│   ├── proof.rs           # Proof generation and verification
│   ├── sparse_index.rs    # Timestamp indexing
│   ├── storage.rs         # Storage backends
│   ├── traits.rs          # Public trait definitions
│   ├── tree.rs            # Core tree implementation
│   ├── clockhash.rs       # ClockHash integration
│   ├── security.rs        # Security logging
│   └── serde_impl.rs      # Serialization support
├── examples/              # Usage examples
│   ├── basic_usage.rs
│   ├── blockchain_example.rs
│   ├── programmable_nodes.rs
│   └── clockhash_integration.rs
├── benches/               # Performance benchmarks
│   ├── tree_operations.rs
│   └── clockhash_integration.rs
├── tests/                 # Integration tests
│   ├── integration_tests.rs
│   └── proof_tests.rs
├── .github/               # GitHub configuration
│   └── workflows/
│       └── ci.yml
├── Cargo.toml             # Package configuration
├── README.md              # Project documentation
├── CHANGELOG.md           # Version history
└── CONTRIBUTING.md        # This file
```

## 💻 Coding Standards

### Rust Guidelines

We follow the official Rust coding guidelines:

- Use `rustfmt` for code formatting
- Use `clippy` for linting
- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use meaningful variable and function names
- Add documentation comments for public APIs

### Code Style

```bash
# Format code
cargo fmt

# Run lints
cargo clippy -- -D warnings

# Check for additional issues
cargo clippy --all-targets --all-features -- -D warnings
```

### Commit Messages

Follow conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New features
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Testing changes
- `chore`: Maintenance tasks

Examples:
```
feat(tree): add parallel tree construction
fix(proof): handle edge case in verification
docs(api): update trait documentation
```

### Documentation

- **Public APIs** must have documentation comments
- **Complex logic** should be explained with comments
- **Examples** should be included in documentation
- **Error messages** should be clear and actionable

```rust
/// Creates a new ChronoMerkle tree with the specified hasher.
///
/// # Examples
///
/// ```
/// use chrono_merkle::{ChronoMerkleTree, Blake3Hasher};
///
/// let tree = ChronoMerkleTree::new(Blake3Hasher::default());
/// ```
pub fn new(hasher: H) -> Self {
    // Implementation...
}
```

## 🧪 Testing

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_tree_operations

# Run with output
cargo test -- --nocapture

# Run doctests
cargo test --doc

# Run benchmarks
cargo bench
```

### Test Coverage

We aim for high test coverage:

- **Unit tests** for individual functions and methods
- **Integration tests** for end-to-end functionality
- **Property-based tests** using proptest where applicable
- **Doctests** in documentation examples

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_creation() {
        let tree = ChronoMerkleTree::new(Blake3Hasher::default());
        assert!(tree.is_empty());
        assert_eq!(tree.leaf_count(), 0);
    }

    #[test]
    fn test_insert_and_verify() {
        let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());
        tree.insert(b"test_data", 1000).unwrap();

        assert_eq!(tree.leaf_count(), 1);
        assert!(!tree.is_empty());
    }
}
```

### Property-Based Testing

Use proptest for complex scenarios:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn tree_maintains_consistency(data in arb_data(), timestamps in arb_timestamps()) {
        let mut tree = ChronoMerkleTree::new(Blake3Hasher::default());

        // Insert data
        for (i, (data_item, timestamp)) in data.iter().zip(&timestamps).enumerate() {
            tree.insert(data_item, *timestamp).unwrap();
            assert_eq!(tree.leaf_count(), i + 1);
        }

        // Verify consistency
        prop_assert!(tree.verify_consistency().unwrap());
    }
}
```

## 🔄 Pull Request Process

### Before Submitting

1. **Ensure tests pass:** `cargo test`
2. **Format code:** `cargo fmt`
3. **Run lints:** `cargo clippy`
4. **Update documentation:** `cargo doc`
5. **Add tests** for new functionality

### Creating a Pull Request

1. **Fork the repository** on GitHub
2. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** and commit them
4. **Push to your fork:**
   ```bash
   git push origin feature/your-feature-name
   ```
5. **Create a Pull Request** on GitHub

### PR Template

Use this template for your PR description:

```markdown
## Description
Brief description of the changes.

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] Benchmarks added/updated

## Checklist
- [ ] Code follows project style guidelines
- [ ] Documentation updated
- [ ] Tests pass locally
- [ ] Commit messages follow conventional format
```

### Review Process

1. **Automated checks** run (CI, tests, linting)
2. **Code review** by maintainers
3. **Feedback** and iteration
4. **Approval** and merge

## 🐛 Reporting Issues

### Bug Reports

Use the bug report template:

```markdown
**Describe the bug**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Run '....'
3. See error

**Expected behavior**
A clear description of what you expected to happen.

**Environment**
- OS: [e.g., Ubuntu 20.04]
- Rust version: [e.g., 1.70.0]
- ChronoMerkle version: [e.g., 0.1.0]

**Additional context**
Add any other context about the problem here.
```

### Feature Requests

Use the feature request template:

```markdown
**Is your feature request related to a problem?**
A clear description of what the problem is.

**Describe the solution you'd like**
A clear description of what you want to happen.

**Describe alternatives you've considered**
A clear description of any alternative solutions.

**Additional context**
Add any other context or screenshots about the feature request here.
```

## 📚 Documentation

### Building Documentation

```bash
# Build documentation
cargo doc

# Open in browser
cargo doc --open

# Build with private items
cargo doc --document-private-items
```

### Writing Documentation

- Use Markdown for README and guides
- Use rustdoc comments for API documentation
- Include code examples in documentation
- Keep examples runnable and tested

### Documentation Structure

- **README.md:** Project overview and quick start
- **API Documentation:** Generated from code comments
- **Examples:** Runnable code samples
- **CHANGELOG.md:** Version history
- **CONTRIBUTING.md:** This file

## 🎯 Areas for Contribution

### High Priority

- **Performance optimizations** in tree operations
- **Additional storage backends** (MongoDB, DynamoDB)
- **WebAssembly support** improvements
- **Advanced benchmarking** and profiling

### Medium Priority

- **Additional hash functions** (SHA-3, KangarooTwelve)
- **Compression algorithms** for storage
- **Network protocol** for distributed trees
- **Visualization tools** (graph rendering)

### Good for Beginners

- **Documentation improvements**
- **Additional test cases**
- **Example applications**
- **Performance benchmarks**

## 📞 Getting Help

- **Issues:** https://github.com/Olyntar-Labs/chrono-merkle/issues
- **Discussions:** https://github.com/Olyntar-Labs/chrono-merkle/discussions
- **Documentation:** https://docs.rs/chrono-merkle

## 🙏 Recognition

Contributors will be recognized in:
- CHANGELOG.md for significant contributions
- GitHub repository contributors list
- Project documentation

Thank you for contributing to ChronoMerkle! 🚀