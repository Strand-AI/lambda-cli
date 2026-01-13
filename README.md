# Lambda CLI

[![CI](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/Strand-AI/lambda-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A fast CLI for managing [Lambda Labs](https://lambdalabs.com/) cloud GPU instances.

## Installation

**Homebrew:**
```bash
brew install strand-ai/tap/lambda-cli
```

**From source:**
```bash
cargo install --git https://github.com/Strand-AI/lambda-cli
```

## Setup

Get your API key from the [Lambda Labs dashboard](https://cloud.lambdalabs.com/api-keys) and export it:

```bash
export LAMBDA_API_KEY=<your-key>
```

## Commands

| Command | Description |
|---------|-------------|
| `list` | Show available GPU types with pricing and availability |
| `running` | Show your running instances |
| `start` | Launch a new instance |
| `stop` | Terminate an instance |
| `find` | Poll until a GPU type is available, then launch |

## Usage

**List available GPUs:**
```bash
lambda_cli list
```

**Start an instance:**
```bash
lambda_cli start --gpu gpu_1x_a10 --ssh my-key --name "dev-box"
```

**Stop an instance:**
```bash
lambda_cli stop --instance-id <id>
```

**Wait for availability and auto-launch:**
```bash
lambda_cli find --gpu gpu_8x_h100 --ssh my-key --interval 30
```

## Options

### start
| Flag | Description |
|------|-------------|
| `-g, --gpu` | Instance type (required) |
| `-s, --ssh` | SSH key name (required) |
| `-n, --name` | Instance name |
| `-r, --region` | Region (auto-selects if omitted) |

### find
| Flag | Description |
|------|-------------|
| `-g, --gpu` | Instance type to wait for (required) |
| `-s, --ssh` | SSH key name (required) |
| `--interval` | Poll interval in seconds (default: 10) |
| `-n, --name` | Instance name when launched |
