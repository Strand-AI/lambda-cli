# Lambda CLI

[![CI](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![npm](https://img.shields.io/npm/v/@strand-ai/lambda-mcp)](https://www.npmjs.com/package/@strand-ai/lambda-mcp)
[![MCP](https://img.shields.io/badge/MCP-compatible-8A2BE2)](https://modelcontextprotocol.io)
[![Install in VS Code](https://img.shields.io/badge/VS_Code-Install_Server-0098FF?logo=visualstudiocode&logoColor=white)](https://vscode.dev/redirect/mcp/install?name=lambda-labs&config=%7B%22command%22%3A%22npx%22%2C%22args%22%3A%5B%22-y%22%2C%22%40strand-ai%2Flambda-mcp%22%5D%7D)
[![Install in Cursor](https://img.shields.io/badge/Cursor-Install_Server-000000?logo=cursor&logoColor=white)](cursor://anysphere.cursor-deeplink/mcp/install?name=lambda-labs&config=%7B%22command%22%3A%22npx%22%2C%22args%22%3A%5B%22-y%22%2C%22%40strand-ai%2Flambda-mcp%22%5D%7D)

A fast CLI and MCP server for managing [Lambda Labs](https://lambdalabs.com/) cloud GPU instances.

**Two ways to use it:**
- **CLI** (`lambda`) - Direct terminal commands for managing GPU instances
- **MCP Server** (`lambda-mcp`) - Let AI assistants like Claude manage your GPU infrastructure

## Installation

### Homebrew (macOS/Linux)
```bash
brew install strand-ai/tap/lambda-cli
```

### From Source
```bash
cargo install --git https://github.com/Strand-AI/lambda-cli
```

### Pre-built Binaries
Download from [GitHub Releases](https://github.com/Strand-AI/lambda-cli/releases).

## Authentication

Get your API key from the [Lambda Labs dashboard](https://cloud.lambdalabs.com/api-keys).

### Option 1: Environment Variable
```bash
export LAMBDA_API_KEY=<your-key>
```

### Option 2: Command (1Password, etc.)
```bash
export LAMBDA_API_KEY_COMMAND="op read op://Personal/Lambda/api-key"
```

The command is executed at startup and its output is used as the API key. This works with any secret manager.

## Notifications (Optional)

Get notified on Slack, Discord, or Telegram when your instance is ready and SSH-able.

### Configuration

Set one or more of these environment variables:

```bash
# Slack (incoming webhook)
export LAMBDA_NOTIFY_SLACK_WEBHOOK="https://hooks.slack.com/services/T00/B00/XXX"

# Discord (webhook URL)
export LAMBDA_NOTIFY_DISCORD_WEBHOOK="https://discord.com/api/webhooks/123/abc"

# Telegram (bot token + chat ID)
export LAMBDA_NOTIFY_TELEGRAM_BOT_TOKEN="123456:ABC-DEF..."
export LAMBDA_NOTIFY_TELEGRAM_CHAT_ID="123456789"
```

### Setup Guides

**Slack:** Create an [Incoming Webhook](https://api.slack.com/messaging/webhooks) in your workspace.

**Discord:** In channel settings → Integrations → Webhooks → New Webhook → Copy Webhook URL.

**Telegram:**
1. Message [@BotFather](https://t.me/botfather) → `/newbot` → copy the token
2. Message your bot, then visit `https://api.telegram.org/bot<TOKEN>/getUpdates` to find your chat ID

---

## CLI Usage

### Commands

| Command | Description |
|---------|-------------|
| `lambda list` | Show available GPU types with pricing and availability |
| `lambda running` | Show your running instances |
| `lambda start` | Launch a new instance |
| `lambda stop` | Terminate an instance |
| `lambda find` | Poll until a GPU type is available, then launch |

### Examples

**List available GPUs:**
```bash
lambda list
```

**Start an instance:**
```bash
lambda start --gpu gpu_1x_a10 --ssh my-key --name "dev-box"
```

**Stop an instance:**
```bash
lambda stop --instance-id <id>
```

**Wait for availability and auto-launch:**
```bash
lambda find --gpu gpu_8x_h100 --ssh my-key --interval 30
```

### CLI Options

#### start
| Flag | Description |
|------|-------------|
| `-g, --gpu` | Instance type (required) |
| `-s, --ssh` | SSH key name (required) |
| `-n, --name` | Instance name |
| `-r, --region` | Region (auto-selects if omitted) |
| `-f, --filesystem` | Filesystem to attach (must be in same region) |
| `--no-notify` | Disable notifications even if env vars are set |

#### find
| Flag | Description |
|------|-------------|
| `-g, --gpu` | Instance type to wait for (required) |
| `-s, --ssh` | SSH key name (required) |
| `--interval` | Poll interval in seconds (default: 10) |
| `-n, --name` | Instance name when launched |
| `-f, --filesystem` | Filesystem to attach when launched |
| `--no-notify` | Disable notifications even if env vars are set |

Notifications are **automatic** when env vars are configured. Use `--no-notify` to disable:
```bash
lambda start --gpu gpu_1x_a10 --ssh my-key --no-notify
```

---

## MCP Server

The `lambda-mcp` binary is an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server that lets AI assistants manage your Lambda Labs infrastructure.

### Quick Start with npx

The easiest way to use `lambda-mcp` is via npx—no installation required:

```bash
npx @strand-ai/lambda-mcp
```

### Options

| Flag | Description |
|------|-------------|
| `--eager` | Execute API key command at startup instead of on first use |

#### API Key Loading

When using `LAMBDA_API_KEY_COMMAND`, the MCP server defers command execution until the first API request by default. This avoids unnecessary delays when starting Claude Code if you don't use Lambda tools in every session.

Use `--eager` to execute the command at startup instead:

```bash
npx @strand-ai/lambda-mcp --eager
```

> **Note:** The CLI (`lambda`) always executes the API key command at startup since it's used for immediate operations.

### Available Tools

| Tool | Description |
|------|-------------|
| `list_gpu_types` | List all available GPU instance types with pricing, specs, and current availability |
| `start_instance` | Launch a new GPU instance (auto-notifies if configured) |
| `stop_instance` | Terminate a running instance |
| `list_running_instances` | Show all running instances with status and connection details |
| `check_availability` | Check if a specific GPU type is available |

### Auto-Notifications

When notification environment variables are configured, the MCP server automatically sends notifications when instances become SSH-able. No additional flags needed—just set the `LAMBDA_NOTIFY_*` env vars and launch instances as usual.

### Claude Code Setup

```bash
claude mcp add lambda-labs -s user -e LAMBDA_API_KEY=your-api-key -- npx -y @strand-ai/lambda-mcp
```

**With 1Password:**
```bash
claude mcp add lambda-labs -s user -e LAMBDA_API_KEY_COMMAND="op read op://Personal/Lambda/api-key" -- npx -y @strand-ai/lambda-mcp
```

Then restart Claude Code.

### Example Prompts

Once configured, you can ask Claude things like:

- "What GPUs are currently available on Lambda Labs?"
- "Launch an H100 instance with my ssh key 'macbook'"
- "Show me my running instances"
- "Check if any A100s are available"
- "Terminate instance i-abc123"

---

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run CLI
cargo run --bin lambda -- list

# Run MCP server
cargo run --bin lambda-mcp
```

