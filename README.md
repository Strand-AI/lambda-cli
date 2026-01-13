# Lambda CLI

A command-line tool for interacting with the Lambda Labs cloud GPU API.

## Features

- Validate API key
- List all available GPU instance types with pricing and availability
- Start GPU instances with optional naming and region selection
- Stop/terminate running instances
- List all running instances with status
- Auto-find and launch instances when they become available
- Rename instances (if supported by API)

## Installation

Requires Rust and Cargo. Install from [rust-lang.org](https://www.rust-lang.org/tools/install).

```bash
git clone https://github.com/cybrly/lambda_cli.git
cd lambda_cli
cargo build --release
```

The binary will be at `./target/release/lambda_cli`.

## Configuration

Set your Lambda Labs API key as an environment variable:

```bash
export LAMBDA_API_KEY=your_api_key
```

Or create a `.env` file in the project directory:

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
| `rename` | Rename an existing instance |

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

### Rename Instance

```bash
lambda_cli rename --instance-id <ID> --name <NEW_NAME>
```

Note: This may not be supported by the Lambda Labs API. If not, you'll get a clear error message suggesting to use `--name` at launch time instead.

## Examples

Validate API key:
```bash
lambda_cli
```

List available instances:
```bash
lambda_cli list
```

![](images/list.png)

Start an instance with a name:
```bash
lambda_cli start --gpu gpu_1x_a10 --ssh my-key --name "dev-server"
```

![](images/start.png)

Stop an instance:
```bash
lambda_cli stop --instance-id abc123-def456
```

![](images/stop.png)

List running instances:
```bash
lambda_cli running
```

![](images/running.png)

Find and auto-start when available:
```bash
lambda_cli find --gpu gpu_8x_h100 --ssh my-key --interval 30
```

![](images/find.png)

## Changes in v0.2.0

- **Proper error handling**: Replaced all `panic!`/`expect!` with graceful error messages
- **HTTP timeouts**: 30s request timeout, 10s connect timeout (prevents hanging)
- **Instance naming**: New `--name` flag for `start` and `find` commands
- **Region selection**: New `--region` flag to specify launch region
- **Rename command**: New `rename` command (API support pending)
- **Better polling**: Instance startup now polls for ready state instead of fixed 2-min sleep
- **Improved display**: Shows instance names, types, and regions in running list
- **Input validation**: SSH key required for `find` command, region validation
- **Updated dependencies**: crossterm 0.28, reqwest 0.12, etc.

## License

MIT
