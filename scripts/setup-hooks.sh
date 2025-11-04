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

# Create symlink for pre-push hook
if [ -f "$HOOKS_DIR/pre-push" ]; then
    echo "⚠️  Existing pre-push hook found, creating backup..."
    mv "$HOOKS_DIR/pre-push" "$HOOKS_DIR/pre-push.backup"
fi

ln -sf "../../$SCRIPT_DIR/pre-push" "$HOOKS_DIR/pre-push"
chmod +x "$SCRIPT_DIR/pre-push"

echo "✅ Git hooks installed successfully!"
echo ""
echo "Pre-push hook will now run 'cargo make fmt' and 'cargo make clippy'"
echo "before each push."
echo ""
echo "To bypass the hook (emergency only), use:"
echo "  git push --no-verify"
