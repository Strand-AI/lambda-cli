# Contributing to Lambda CLI

## Development

```bash
git clone https://github.com/Strand-AI/lambda-cli.git
cd lambda-cli
cargo build
cargo test
```

## Pull Requests

1. Fork the repo and create a branch
2. Make your changes
3. Run `cargo fmt` and `cargo clippy`
4. Open a PR against `main`

## Releasing

Releases are automated via PR labels. To create a release:

1. Open a PR with your changes
2. Add one of these labels:
   - `release:patch` - Bug fixes (0.2.0 → 0.2.1)
   - `release:minor` - New features (0.2.0 → 0.3.0)
   - `release:major` - Breaking changes (0.2.0 → 1.0.0)
3. Merge the PR

When merged, CI will automatically:
- Bump the version in `Cargo.toml`
- Create a git tag
- Build binaries for Linux, macOS, and Windows
- Create a GitHub release
- Update the Homebrew formula
- Publish to npm (`@strand-ai/lambda-mcp`)

No label = no release. PRs without release labels are regular code changes.
