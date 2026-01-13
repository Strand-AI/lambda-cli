# Lambda CLI

A command-line tool for interacting with the Lambda Labs cloud GPU API.

## Installation

### Homebrew (macOS/Linux)

```bash
brew tap strand-ai/tap
brew install lambda-cli
```

### From Source

Requires Rust and Cargo. Install from [rust-lang.org](https://www.rust-lang.org/tools/install).

```bash
git clone https://github.com/Strand-AI/lambda-cli.git
cd lambda-cli
cargo install --path .
```

## Configuration

Set your Lambda Labs API key as an environment variable:

```bash
export LAMBDA_API_KEY=your_api_key
```

Or create a `.env` file in the working directory:

```
LAMBDA_API_KEY=your_api_key
```

## Usage

```bash
lambda_cli [COMMAND]
```

### Commands

| Command | Description |
|---------|-------------|
| `list` | List all GPU instance types with pricing and availability |
| `start` | Launch a new GPU instance |
| `stop` | Terminate a running instance |
| `running` | List all running instances |
| `find` | Poll for availability and auto-launch when found |

### Start Instance

```bash
lambda_cli start --gpu <TYPE> --ssh <KEY> [--name <NAME>] [--region <REGION>]
```

Options:
- `--gpu, -g` (required): Instance type (e.g., `gpu_1x_a100`)
- `--ssh, -s` (required): SSH key name registered in Lambda Labs
- `--name, -n` (optional): Name for the instance
- `--region, -r` (optional): Region to launch in (auto-selects if not specified)

Example:
```bash
lambda_cli start --gpu gpu_1x_a10 --ssh my-key --name "training-run-1"
```

### Stop Instance

```bash
lambda_cli stop --instance-id <ID>
```

### Find and Auto-Launch

Polls for availability and automatically launches when capacity is found:

```bash
lambda_cli find --gpu <TYPE> --ssh <KEY> [--interval <SECONDS>] [--name <NAME>]
```

Options:
- `--gpu, -g` (required): Instance type to find
- `--ssh, -s` (required): SSH key name
- `--interval` (default: 10): Polling interval in seconds
- `--name, -n` (optional): Name for the instance when launched

Example:
```bash
lambda_cli find --gpu gpu_8x_h100 --ssh my-key --interval 30 --name "h100-cluster"
```

## Examples

Validate API key:
```bash
lambda_cli
```

List available instances:
```bash
lambda_cli list
```

Start an instance with a name:
```bash
lambda_cli start --gpu gpu_1x_a10 --ssh my-key --name "dev-server"
```

Stop an instance:
```bash
lambda_cli stop --instance-id abc123-def456
```

List running instances:
```bash
lambda_cli running
```

Find and auto-start when available:
```bash
lambda_cli find --gpu gpu_8x_h100 --ssh my-key --interval 30
```

## License

MIT

