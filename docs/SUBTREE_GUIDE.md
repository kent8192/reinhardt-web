# Git Subtree Guide for Reinhardt Examples

This document explains how the `reinhardt-examples` repository is integrated with the main `reinhardt-web` repository using git subtree.

## Overview

The examples are maintained in a separate repository ([kent8192/reinhardt-examples](https://github.com/kent8192/reinhardt-examples)) and embedded in this repository at `examples/` using git subtree. This allows independent development of examples while keeping them accessible within the main framework repository.

## Why Git Subtree?

- **Simplicity**: No need for `git submodule init` or `update`
- **Complete Copy**: Full example code is included in the main repository
- **Bidirectional Sync**: Changes can flow both ways
- **Independent Versioning**: Examples have their own version and release cycle

## Repository Structure

```
reinhardt-web/               # Main repository
└── examples/                # ← Git subtree (points to reinhardt-examples)
    ├── examples-*/          # Example projects
    ├── common/              # Shared utilities
    ├── test-macros/         # Test helper macros
    ├── scripts/             # Helper scripts
    └── ...

reinhardt-examples/          # Independent repository
├── examples-*/              # Example projects
├── common/                  # Shared utilities
├── test-macros/             # Test helper macros
├── scripts/                 # Helper scripts
└── ...
```

## Common Operations

### Pulling Updates from Examples Repository

When changes are made in the `reinhardt-examples` repository, pull them into the main repository:

```bash
git subtree pull --prefix=examples reinhardt-examples main --squash
```

**When to use**:
- After new examples are added to the examples repository
- After bug fixes in the examples repository
- Regular sync to keep examples up to date

### Pushing Changes to Examples Repository

When changes are made to `examples/` in the main repository, push them to the examples repository:

```bash
git subtree push --prefix=examples reinhardt-examples main
```

**When to use**:
- After fixing examples in the main repository
- After adding new examples in the main repository's `examples/` directory

### Checking Subtree Status

To see if there are pending changes:

```bash
git fetch reinhardt-examples
git diff examples reinhardt-examples/main
```

## Best Practices

### 1. Regular Syncing

Sync regularly to avoid large merge conflicts:

```bash
# Weekly or after significant changes
git subtree pull --prefix=examples reinhardt-examples main --squash
```

### 2. Clear Commit Messages

When working in the `examples/` directory, use clear commit messages:

```bash
git commit -m "feat(examples): add websocket example"
```

### 3. Test Before Pushing

Before pushing to the examples repository, ensure all tests pass:

```bash
cd examples/local
cargo test --workspace --all --all-features
```

## Troubleshooting

### Issue: "fatal: ambiguous argument 'reinhardt-examples/main'"

**Cause**: Remote not configured

**Solution**:
```bash
git remote add -f reinhardt-examples https://github.com/kent8192/reinhardt-examples.git
```

### Issue: Merge Conflicts

**Cause**: Conflicting changes in both repositories

**Solution**:
1. Identify conflicts: `git status`
2. Resolve conflicts manually
3. Stage resolved files: `git add examples/...`
4. Complete merge: `git commit`

### Issue: Push Rejected

**Cause**: Remote has newer commits

**Solution**:
```bash
# Pull first
git subtree pull --prefix=examples reinhardt-examples main --squash

# Then push
git subtree push --prefix=examples reinhardt-examples main
```

## Advanced Operations

### Viewing Subtree History

```bash
git log --oneline -- examples/
```

### Checking Remote Configuration

```bash
git remote -v | grep reinhardt-examples
```

## References

- [Git Subtree Documentation](https://git-scm.com/docs/git-subtree)
- [Examples Repository](https://github.com/kent8192/reinhardt-examples)
- [Examples Repository Guide](../examples/SUBTREE_OPERATIONS.md)

---

For more detailed information about subtree operations from the examples repository perspective, see the [SUBTREE_OPERATIONS.md](../examples/SUBTREE_OPERATIONS.md) file in the examples directory.
