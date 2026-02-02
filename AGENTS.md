# Developer Guide

Architecture and development guide for OpenSAM.

## Overview

OpenSAM is a Rust-based AI agent framework. It's designed to be:
- **Modular**: Use only the crates you need
- **Fast**: Async I/O with minimal overhead  
- **Hackable**: Easy to extend with custom tools

## Architecture

### Crate Layout

```
crates/
├── opensam/       # CLI entry point
├── config/        # Configuration & paths (~/.opensam/)
├── bus/           # Async message passing (tokio mpsc)
├── provider/      # LLM provider abstraction
├── agent/         # Core agent loop + toolkit
├── session/       # Conversation persistence
├── channels/      # Telegram bot integration
├── cron/          # Scheduled task runner
└── heartbeat/     # Periodic task checker
```

### Data Flow

```
User Input → Channel/Bus → Agent Loop → Provider
                ↓              ↓
           Session Store    Tools (filesystem, web, etc.)
```

## Key Concepts

### The Agent Loop

The `AgentLoop` in `crates/agent/src/loop_agent.rs` is the core:

1. Receives messages from the bus
2. Builds context (system prompt + history)
3. Calls LLM provider
4. Executes any tool calls
5. Returns response

### Tools

Tools implement the `ToolTrait`:

```rust
#[async_trait]
pub trait ToolTrait: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;  // JSON schema
    async fn execute(&self, args: Value) -> Result<String, Error>;
}
```

Default tools are registered in `AgentLoop::register_default_tools()`.

### Providers

The `Provider` trait abstracts LLM APIs:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat(&self, params: ChatParams) -> Result<ChatResponse>;
    fn default_model(&self) -> String;
    fn is_configured(&self) -> bool;
}
```

Currently implements OpenRouter (which supports Anthropic, OpenAI, etc.).

## Adding Features

### New Tool

1. Create struct in `crates/agent/src/tools/`
2. Implement `ToolTrait`
3. Register in `AgentLoop::register_default_tools()`
4. Add test

### New Provider

1. Create file in `crates/provider/src/`
2. Implement `Provider` trait
3. Export in `crates/provider/src/lib.rs`

### New Channel

1. Create file in `crates/channels/src/`
2. Implement `Channel` trait
3. Add to `ChannelManager`

## Configuration

Config is loaded from `~/.opensam/config.json`:

```json
{
  "soliton": {
    "openrouter": {
      "api_key": "..."
    }
  },
  "operative": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4"
    }
  }
}
```

Paths are managed in `crates/config/src/paths.rs`.

## Testing

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p opensam-agent

# With output
cargo test -- --nocapture
```

## Code Style

- Use `thiserror` for error types
- Use `tracing` for logs (not println)
- Async functions use `async-trait`
- Prefer `impl Trait` for function args
- Document public APIs

## Build & Release

```bash
# Debug
cargo build

# Release
cargo build --release

# Install locally
cargo install --path crates/opensam

# Cross-compile (requires cross)
cross build --release --target x86_64-unknown-linux-musl
```

## Debugging

Enable debug logging:

```bash
RUST_LOG=debug cargo run -- engage -m "test"
```

---

*Built with Rust, coffee, and the occasional cardboard box.*
