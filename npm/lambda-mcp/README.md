# Lambda MCP Server

> [!CAUTION]
> **UNOFFICIAL PROJECT** — This is a community-built tool, not affiliated with or endorsed by Lambda.

[![CI](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/Strand-AI/lambda-cli/blob/main/LICENSE)
[![npm](https://img.shields.io/npm/v/@strand-ai/lambda-mcp)](https://www.npmjs.com/package/@strand-ai/lambda-mcp)
[![MCP](https://img.shields.io/badge/MCP-compatible-8A2BE2)](https://modelcontextprotocol.io)

An [MCP (Model Context Protocol)](https://modelcontextprotocol.io/) server that lets AI assistants manage your [Lambda](https://lambda.ai/) cloud GPU infrastructure.

[![Install in VS Code](https://img.shields.io/badge/VS_Code-Install_Server-0098FF?logo=visualstudiocode&logoColor=white)](https://vscode.dev/redirect/mcp/install?name=lambda&config=%7B%22command%22%3A%22npx%22%2C%22args%22%3A%5B%22-y%22%2C%22%40strand-ai%2Flambda-mcp%22%5D%7D)
[![Install in Cursor](https://img.shields.io/badge/Cursor-Install_Server-000000?logo=cursor&logoColor=white)](cursor://anysphere.cursor-deeplink/mcp/install?name=lambda&config=%7B%22command%22%3A%22npx%22%2C%22args%22%3A%5B%22-y%22%2C%22%40strand-ai%2Flambda-mcp%22%5D%7D)

## Quick Start

```bash
npx @strand-ai/lambda-mcp
```

## Authentication

Get your API key from the [Lambda dashboard](https://cloud.lambda.ai/api-keys/cloud-api).

### Option 1: Environment Variable
```bash
export LAMBDA_API_KEY=<your-key>
```

### Option 2: Command (1Password, etc.)
```bash
export LAMBDA_API_KEY_COMMAND="op read op://Personal/Lambda/api-key"
```

## Claude Code Setup

```bash
claude mcp add lambda -s user -e LAMBDA_API_KEY=your-api-key -- npx -y @strand-ai/lambda-mcp
```

**With 1Password CLI:**
```bash
claude mcp add lambda -s user -e LAMBDA_API_KEY_COMMAND="op read op://Personal/Lambda/api-key" -- npx -y @strand-ai/lambda-mcp
```

Then restart Claude Code.

## Available Tools

| Tool | Description |
|------|-------------|
| `list_gpu_types` | List all available GPU instance types with pricing, specs, and current availability |
| `start_instance` | Launch a new GPU instance |
| `stop_instance` | Terminate a running instance |
| `list_running_instances` | Show all running instances with status and connection details |
| `check_availability` | Check if a specific GPU type is available |

## Example Prompts

Once configured, you can ask Claude things like:

- "What GPUs are currently available on Lambda?"
- "Launch an H100 instance with my ssh key 'macbook'"
- "Show me my running instances"
- "Check if any A100s are available"
- "Terminate instance i-abc123"

## Notifications (Optional)

Get notified on Slack, Discord, or Telegram when your instance is ready and SSH-able.

```bash
# Slack
export LAMBDA_NOTIFY_SLACK_WEBHOOK="https://hooks.slack.com/services/T00/B00/XXX"

# Discord
export LAMBDA_NOTIFY_DISCORD_WEBHOOK="https://discord.com/api/webhooks/123/abc"

# Telegram
export LAMBDA_NOTIFY_TELEGRAM_BOT_TOKEN="123456:ABC-DEF..."
export LAMBDA_NOTIFY_TELEGRAM_CHAT_ID="123456789"
```

## CLI

Looking for the CLI? See the [full documentation](https://github.com/Strand-AI/lambda-cli).

## License

MIT
