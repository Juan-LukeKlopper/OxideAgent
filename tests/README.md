# Testing Structure

This directory contains all tests for the OxideAgent project.

## Directory Structure

- `unit/` - Unit tests for individual components
- `integration/` - Integration tests for component interactions
- `utils/` - Test utilities and mock objects

## Test Organization

Tests are organized by the component they test:

### Unit Tests
- `unit/agents/` - Tests for agent implementations
- `unit/tools/` - Tests for tool implementations
- `unit/session/` - Tests for session management
- `unit/config/` - Tests for configuration management
- `unit/container/` - Tests for dependency injection container

### Integration Tests
- `integration/orchestrator/` - Tests for orchestrator functionality
- `integration/tui/` - Tests for TUI components
- `integration/end_to_end/` - End-to-end workflow tests

## Running Tests

```bash
# Run all tests
cargo test

# Run unit tests only
cargo test --lib

# Run integration tests only
cargo test --test integration

# Run tests with coverage
cargo tarpaulin --out Html
```