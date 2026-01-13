# Lambda CLI

[![CI](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

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

#### find
| Flag | Description |
|------|-------------|
| `-g, --gpu` | Instance type to wait for (required) |
| `-s, --ssh` | SSH key name (required) |
| `--interval` | Poll interval in seconds (default: 10) |
| `-n, --name` | Instance name when launched |

---

## MCP Server

The `lambda-mcp` binary is an [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server that lets AI assistants manage your Lambda Labs infrastructure.

### Quick Start with npx

The easiest way to use `lambda-mcp` is via npx—no installation required:

```bash
npx @strand-ai/lambda-mcp
```

### Available Tools

| Tool | Description |
|------|-------------|
| `list_gpu_types` | List all available GPU instance types with pricing, specs, and current availability |
| `start_instance` | Launch a new GPU instance |
| `stop_instance` | Terminate a running instance |
| `list_running_instances` | Show all running instances with status and connection details |
| `check_availability` | Check if a specific GPU type is available |

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
