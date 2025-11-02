#!/bin/bash
# Setup git hooks for Reinhardt project

set -e

HOOKS_DIR=".git/hooks"
SCRIPT_DIR="scripts/git-hooks"

echo "🔧 Setting up git hooks for Reinhardt..."

# Check if .git directory exists
if [ ! -d ".git" ]; then
    echo "❌ Error: .git directory not found"
    echo "Please run this script from the repository root"
    exit 1
fi

# Create symlink for pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
    echo "⚠️  Existing pre-commit hook found, creating backup..."
    mv "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit.backup"
fi

ln -sf "../../$SCRIPT_DIR/pre-commit" "$HOOKS_DIR/pre-commit"
chmod +x "$SCRIPT_DIR/pre-commit"

echo "✅ Git hooks installed successfully!"
echo ""
echo "Pre-commit hook will now run 'cargo make fmt' and 'cargo make clippy'"
echo "before each commit."
echo ""
echo "To bypass the hook (emergency only), use:"
echo "  git commit --no-verify -m \"message\""
