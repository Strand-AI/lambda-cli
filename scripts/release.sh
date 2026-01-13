#!/usr/bin/env bash
set -euo pipefail

# Release script for lambda-cli
# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.2.0

VERSION="${1:-}"

if [[ -z "$VERSION" ]]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.2.0"
    exit 1
fi

# Validate version format (semver)
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Error: Version must be in semver format (e.g., 0.2.0)"
    exit 1
fi

# Check we're on main branch
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [[ "$BRANCH" != "main" ]]; then
    echo "Error: Must be on main branch to release (currently on $BRANCH)"
    exit 1
fi

# Check for uncommitted changes
if ! git diff-index --quiet HEAD --; then
    echo "Error: Uncommitted changes detected. Please commit or stash them first."
    exit 1
fi

# Check if tag already exists
if git rev-parse "v$VERSION" >/dev/null 2>&1; then
    echo "Error: Tag v$VERSION already exists"
    exit 1
fi

echo "Releasing version $VERSION..."

# Check current version in Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')

if [[ "$CURRENT_VERSION" != "$VERSION" ]]; then
    # Update version in Cargo.toml
    sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

    # Update Cargo.lock
    cargo check --quiet

    # Commit version bump
    git add Cargo.toml Cargo.lock
    git commit -m "chore: bump version to $VERSION"
else
    echo "Version already set to $VERSION in Cargo.toml"
fi

# Create and push tag
git tag -a "v$VERSION" -m "Release v$VERSION"

echo "Pushing changes and tag..."
git push origin main
git push origin "v$VERSION"

echo ""
echo "Release v$VERSION initiated!"
echo "CI will now build and publish release artifacts."
echo "Monitor progress at: https://github.com/Strand-AI/lambda-cli/actions"
echo ""
echo "Once the release is published, update the Homebrew formula:"
echo "  ./scripts/update-homebrew.sh $VERSION"
