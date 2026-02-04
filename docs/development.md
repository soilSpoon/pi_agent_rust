# Development

## Building

Pi requires Rust nightly (2024 edition).

```bash
# Build dev binary
cargo build

# Build release binary (optimized)
cargo build --release
```

## Testing

We enforce a strict "no mocks" policy for core logic. Tests use real filesystem operations (in temp dirs) and VCR-style recording for HTTP interactions.

### Unit & Integration Tests

```bash
# Run all tests
cargo test

# Run specific module
cargo test config
cargo test session
```

### Conformance Tests

Conformance tests validate that Pi behaves identically to the legacy TypeScript implementation for tools and core logic.

```bash
cargo test conformance
```

### VCR Mode

Provider tests use recorded "cassettes" to avoid network calls and ensure determinism.

- **Playback (Default)**: Replays recorded responses. Fails if cassette missing.
- **Record**: Makes real API calls and saves cassettes.

```bash
# Run in playback mode (CI default)
VCR_MODE=playback cargo test

# Record new cassettes (requires API keys)
export ANTHROPIC_API_KEY=...
VCR_MODE=record cargo test provider_streaming
```

## Quality Gates

Before submitting a PR, ensure all gates pass:

```bash
# Format check
cargo fmt --check

# Lint check (deny warnings)
cargo clippy --all-targets -- -D warnings

# Tests
cargo test --all-targets
```

## Project Structure

- `src/`: Core Rust source
- `tests/`: Integration and conformance tests
- `docs/`: User and developer documentation
- `legacy_pi_mono_code/`: Reference code from the original TypeScript implementation