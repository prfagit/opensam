# Contributing to OpenSAM

Thanks for your interest in contributing!

## How to Contribute

### Reporting Bugs

- Check existing issues first
- Include steps to reproduce
- Note your OS and Rust version

### Suggesting Features

- Open an issue describing the use case
- Discuss implementation approach

### Pull Requests

1. Fork the repo
2. Create a branch (`git checkout -b feature/cool-thing`)
3. Make your changes
4. Run `cargo test` and `cargo clippy`
5. Format with `cargo fmt`
6. Submit PR

## Development Setup

```bash
git clone https://github.com/prfagit/opensam.git
cd opensam
cargo build
cargo test --workspace
```

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` to catch issues
- Write tests for new features
- Document public APIs

## Project Structure

- `crates/config/` - Configuration & paths
- `crates/bus/` - Message passing
- `crates/provider/` - LLM providers
- `crates/agent/` - Core agent + tools
- `crates/session/` - Persistence
- `crates/channels/` - Telegram/etc
- `crates/cron/` - Scheduled tasks
- `crates/heartbeat/` - Periodic tasks
- `crates/opensam/` - CLI binary

## Testing

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p opensam-agent

# With output
cargo test -- --nocapture
```

## License

By contributing, you agree that your contributions will be licensed under MIT.
