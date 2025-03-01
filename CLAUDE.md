# Peridio CLI Development Guide

## Build & Test Commands
- Build: `cargo build`
- Release build: `cargo build --release`  
- Run all tests: `cargo test`
- Run specific test: `cargo test test_name`
- Lint code: `cargo clippy`
- Format code: `cargo fmt`

## Code Style Guidelines
- **Imports**: Standard modules first, external crates next, internal imports last
- **Formatting**: 4-space indentation, default rustfmt settings
- **Naming**: snake_case for functions/variables, CamelCase for types, SCREAMING_SNAKE_CASE for constants
- **Error Handling**: Use snafu for error context, custom Error enums with Snafu derive
- **Types**: Explicitly annotate return types, use Rust's type system appropriately
- **Dependencies**: Uses clap for CLI, tokio for async, reqwest for HTTP, serde for serialization

## Project Structure
- API endpoints in src/api/
- Configuration in src/config/
- Utilities in src/utils/
- Integration tests in tests/