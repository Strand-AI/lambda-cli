#!/usr/bin/env bash
set -euo pipefail

# Update Homebrew formula after a release
# Usage: ./scripts/update-homebrew.sh <version>
# Example: ./scripts/update-homebrew.sh 0.2.0

VERSION="${1:-}"
HOMEBREW_TAP_PATH="${HOMEBREW_TAP_PATH:-$HOME/Repos/homebrew-tap}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semver format (e.g., 0.2.0)"
    exit 1
fi

TARBALL_URL="https://github.com/Strand-AI/lambda-cli/archive/refs/tags/v${VERSION}.tar.gz"

echo "Downloading tarball to compute SHA256..."
SHA256=$(curl -sL "$TARBALL_URL" | shasum -a 256 | cut -d' ' -f1)

if [[ -z "$SHA256" || "$SHA256" == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855" ]]; then
    echo "Error: Failed to download tarball or release not found"
    echo "URL: $TARBALL_URL"
    echo "Make sure the release v$VERSION exists on GitHub"
    exit 1
fi

echo "SHA256: $SHA256"

FORMULA_PATH="$HOMEBREW_TAP_PATH/Formula/lambda-cli.rb"

if [[ ! -f "$FORMULA_PATH" ]]; then
    echo "Error: Formula not found at $FORMULA_PATH"
    echo "Set HOMEBREW_TAP_PATH to your homebrew-tap repo path"
    exit 1
fi

echo "Updating formula at $FORMULA_PATH..."

cat > "$FORMULA_PATH" << EOF
class LambdaCli < Formula
  desc "CLI tool for Lambda Labs cloud GPU API"
  homepage "https://github.com/Strand-AI/lambda-cli"
  url "https://github.com/Strand-AI/lambda-cli/archive/refs/tags/v${VERSION}.tar.gz"
  sha256 "${SHA256}"
  license "MIT"
  head "https://github.com/Strand-AI/lambda-cli.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "lambda ${VERSION}", shell_output("#{bin}/lambda_cli --version")
  end
end
EOF

echo "Formula updated!"
echo ""
echo "To publish the update:"
echo "  cd $HOMEBREW_TAP_PATH"
echo "  git add Formula/lambda-cli.rb"
echo "  git commit -m 'lambda-cli: update to $VERSION'"
echo "  git push origin main"
