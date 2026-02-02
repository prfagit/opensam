# OpenSAM

- **Website**: [OpenSAM.xyz](https://opensam.xyz)
- **Token**: `$SAM` (Solana)
- **Mint**: `9tbJGeXNHsLEkncTRSUF7Cn2k6Meji5wvz4jpagmpump`
- **Community**: [X Community](https://x.com/i/communities/1964052952760697128)

A lightweight AI assistant in Rust you can run locally or on a VPS. Built for anyone who wants a fast, private helper with memory and safe access to local files.

> *"Kept you waiting, huh?"* — But not for long. Startup time is ~100ms.

## Install (One‑Liner)

```bash
curl -sL https://opensam.xyz/install.sh | bash
```

## What is this?

OpenSAM is a personal AI assistant you can run on any device or VPS. You can chat with it from Telegram, or use the CLI directly. It can read and write files in your local workspace, remember things over time, execute commands you allow, search the web, and integrate with Telegram. Think of it as a minimal, extensible automation layer for your daily workflows.

Named after a certain legendary operative (you know the one), but designed for anyone who wants a practical assistant without the bloat.

## Why OpenSAM

- **Private by default**: Your memory and files live locally.
- **Fast & lightweight**: Minimal overhead, quick startup.
- **Telegram‑first**: Run it anywhere, talk to it from Telegram.
- **Extensible**: Add tools and providers without a giant stack.

## Quick Start

```bash
# Build from source
cargo build --release

# Interactive setup (recommended)
./target/release/opensam setup

# Or initialize config/workspace manually
./target/release/opensam init

# If you used init, add your API key in ~/.opensam/config.json

# Chat
./target/release/opensam engage -m "What's the weather in NYC?"
```

## Zero‑Dev Onboarding (No Setup Skills Required)

This flow is for people who just want a personal assistant with memory and local files.

```bash
# 1) Build once
cargo build --release

# 2) Run the guided setup
./target/release/opensam setup

# 3) Start chatting
./target/release/opensam engage
```

What the setup wizard does:
- Creates your local workspace at `~/.opensam/ops`
- Stores your API key in `~/.opensam/config.json`
- Prepares memory files (`lifepod/MEMORY.md`)
- (Optional) Configures Telegram

## Features

- **Fast**: Rust + Tokio = ~100ms cold start, ~30MB memory footprint
- **Memory**: Persistent sessions and long-term notes stored locally
- **Local files**: Read/write/edit within a safe workspace
- **Tools**: Shell execution, web search (Brave), URL fetching
- **Multi-provider**: OpenRouter or any OpenAI‑compatible endpoint (OpenAI, vLLM, custom)
- **Channels**: Telegram bot integration
- **Scheduled tasks**: Cron-style automation (CLI wiring WIP)
- **Extensible**: Add custom tools by implementing a simple trait

## Configuration

Edit `~/.opensam/config.json`:

```json
{
  "soliton": {
    "openrouter": {
      "api_key": "sk-or-v1-your-key"
    }
  },
  "operative": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4"
    }
  },
  "toolkit": {
    "web": {
      "search": {
        "api_key": "your-brave-api-key"
      }
    }
  },
  "frequency": {
    "telegram": {
      "enabled": true,
      "token": "your-bot-token",
      "allow_from": ["your-telegram-user-id"]
    }
  }
}
```

Get an API key from [OpenRouter](https://openrouter.ai/keys) or use any OpenAI‑compatible endpoint directly.

## CLI Usage

```bash
# Interactive setup
opensam setup

# Initialize workspace (manual path)
opensam init

# Single query
opensam engage -m "Summarize ~/notes.txt"

# Interactive mode
opensam engage

# Start gateway service (for Telegram)
opensam deploy

# Check status
opensam status

# List scheduled jobs
opensam schedule list
```

## Telegram Gateway (Deploy)

1) Create a bot with @BotFather and copy the token.
2) Put your numeric Telegram user ID in `allow_from`.
3) Start the long‑lived gateway service:

```bash
cargo build
./target/debug/opensam deploy
```

Then message your bot in Telegram. The gateway will process messages and reply.

## Tools Available

| Tool | Description |
|------|-------------|
| `read_file` | Read file contents |
| `write_file` | Write/create files |
| `edit_file` | Replace text in files |
| `list_dir` | Directory listing |
| `exec` | Execute shell commands |
| `web_search` | Brave Search integration |
| `web_fetch` | Fetch and parse URLs |
| `message` | Send messages to channels |

## Workspace Structure

```
~/.opensam/
├── config.json          # API keys and settings
├── ops/                 # Working directory
│   ├── DIRECTIVE.md     # System prompts
│   ├── PERSONA.md       # Assistant persona
│   ├── SUBJECT.md       # User preferences
│   ├── lifepod/         # Long-term memory
│   │   └── MEMORY.md
│   ├── arsenal/         # Custom skills/tools
│   └── logs/            # Session history
```

## Architecture

Modular workspace design:

```
crates/
├── opensam/      # CLI binary
├── config/       # Configuration management
├── bus/          # Async message passing
├── provider/     # LLM provider abstraction
├── agent/        # Core agent loop + tools
├── session/      # Conversation persistence
├── channels/     # Telegram integration
├── cron/         # Scheduled task runner
└── heartbeat/    # Periodic wake-up service
```

Each crate has a single responsibility. Mix and match what you need.

## Adding a Custom Tool

```rust
use opensam_agent::tools::ToolTrait;
use async_trait::async_trait;

pub struct MyTool;

#[async_trait]
impl ToolTrait for MyTool {
    fn name(&self) -> &str { "my_tool" }
    fn description(&self) -> &str { "Does something useful" }
    fn parameters(&self) -> serde_json::Value { /* schema */ }
    async fn execute(&self, args: Value) -> Result<String, Error> { /* logic */ }
}
```

Register it in `AgentLoop::register_default_tools()`.

## Development

```bash
# Run tests
cargo test --workspace

# Build debug
cargo build

# Build release
cargo build --release

# Install locally
cargo install --path crates/opensam
```

## Contributing

PRs welcome. The codebase is intentionally small and readable. Check `AGENTS.md` for architecture details.

```bash
git clone https://github.com/prfagit/opensam.git
cd opensam
cargo test
```

## License

MIT — Do what you want, just don't start a mercenary army with it.

---

> *"I'm no hero. Never was, never will be. Just an old killer hired to do some wet work."* 
> 
> But this code? It's solid.
