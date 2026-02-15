# Git Subtree Operations Guide

This document explains how the `reinhardt-examples` repository is integrated with the main `reinhardt-web` repository using git subtree.

## Table of Contents

- [Overview](#overview)
- [What is Git Subtree?](#what-is-git-subtree)
- [Initial Setup](#initial-setup)
- [Common Operations](#common-operations)
- [Conflict Resolution](#conflict-resolution)
- [Troubleshooting](#troubleshooting)

## Overview

The `reinhardt-examples` repository is embedded in the main `reinhardt-web` repository at the `examples/` directory using git subtree. This allows:

1. **Independent Development**: Examples can be developed and maintained separately
2. **Bidirectional Sync**: Changes can flow both ways (main repo ↔ examples repo)
3. **Version Control**: Each repository maintains its own history
4. **Easy Distribution**: Examples can be used independently or as part of the main framework

## What is Git Subtree?

Git subtree is a Git feature that allows you to:
- Embed one repository inside another
- Keep a full copy of the external repository in your main repository
- Sync changes bidirectionally

**Key Differences from Submodules**:
- **Subtree**: Full copy of files, simpler for users (no `git submodule init` needed)
- **Submodule**: Reference to external repository, requires extra steps

## Initial Setup

This section documents how the subtree was initially set up. **You don't need to run these commands** unless you're setting up a new subtree.

### Step 1: Add Remote

In the main `reinhardt-web` repository:

```bash
cd reinhardt-web
git remote add -f reinhardt-examples https://github.com/kent8192/reinhardt-examples.git
```

This adds the examples repository as a remote named `reinhardt-examples`.

### Step 2: Remove Existing Examples (if any)

```bash
git rm -r examples/
git commit -m "chore: remove examples/ (will be re-added as subtree)"
```

### Step 3: Add Subtree

```bash
git subtree add --prefix=examples reinhardt-examples main --squash
```

Explanation:
- `--prefix=examples`: Place the subtree at `examples/` directory
- `reinhardt-examples`: Remote name
- `main`: Branch to track
- `--squash`: Squash all commits into one (cleaner history)

### Step 4: Push to Main Repository

```bash
git push origin main
```

## Common Operations

### Pulling Updates (Examples → Main Repository)

When changes are made in the `reinhardt-examples` repository, pull them into the main repository:

```bash
cd reinhardt-web
git subtree pull --prefix=examples reinhardt-examples main --squash
```

**What this does**:
1. Fetches latest commits from `reinhardt-examples` repository
2. Merges them into your `examples/` directory
3. Creates a single merge commit (because of `--squash`)

**When to use**:
- After new examples are added to the examples repository
- After bug fixes in the examples repository
- Regular sync to keep examples up to date

**Example Output**:
```
Subtree merge from 'https://github.com/kent8192/reinhardt-examples'
 * branch            main       -> FETCH_HEAD
Merge made by the 'ort' strategy.
 examples/examples-new-feature/... | 10 ++++++++++
 1 file changed, 10 insertions(+)
```

### Pushing Changes (Main Repository → Examples)

When changes are made to `examples/` in the main repository, push them to the examples repository:

```bash
cd reinhardt-web
git subtree push --prefix=examples reinhardt-examples main
```

**What this does**:
1. Extracts commits that touched `examples/` directory
2. Pushes them to the `reinhardt-examples` repository

**When to use**:
- After fixing examples in the main repository
- After adding new examples in the main repository's `examples/` directory
- When you want to share changes back to the examples repository

**Example Output**:
```
git push using:  reinhardt-examples main
Enumerating objects: 5, done.
Counting objects: 100% (5/5), done.
Writing objects: 100% (3/3), 300 bytes | 300.00 KiB/s, done.
Total 3 (delta 0), reused 0 (delta 0)
To https://github.com/kent8192/reinhardt-examples.git
   abc1234..def5678  main -> main
```

### Checking Subtree Status

To see if there are pending changes:

```bash
# In main repository
cd reinhardt-web
git fetch reinhardt-examples
git diff examples reinhardt-examples/main
```

## Conflict Resolution

### Scenario 1: Merge Conflicts During Pull

When pulling changes, you might encounter conflicts:

```bash
git subtree pull --prefix=examples reinhardt-examples main --squash
# CONFLICT (content): Merge conflict in examples/examples-rest-api/src/main.rs
# Automatic merge failed; fix conflicts and then commit the result.
```

**Resolution Steps**:

1. **Identify Conflicts**:
   ```bash
   git status
   ```

2. **Resolve Each Conflict**:
   Open conflicting files and look for conflict markers:
   ```rust
   <<<<<<< HEAD
   // Your changes in main repository
   fn hello() {
       println!("Hello from main");
   }
   =======
   // Changes from examples repository
   fn hello() {
       println!("Hello from examples");
   }
   >>>>>>>
   ```

3. **Edit the File**:
   Choose which changes to keep or merge them manually:
   ```rust
   fn hello() {
       println!("Hello from main and examples");
   }
   ```

4. **Mark as Resolved**:
   ```bash
   git add examples/examples-rest-api/src/main.rs
   ```

5. **Complete the Merge**:
   ```bash
   git commit -m "chore: merge examples from reinhardt-examples repository"
   ```

### Scenario 2: Push Rejected

If your push is rejected:

```bash
git subtree push --prefix=examples reinhardt-examples main
# error: failed to push some refs
```

**Resolution**:

1. **Pull First**:
   ```bash
   git subtree pull --prefix=examples reinhardt-examples main --squash
   ```

2. **Resolve Any Conflicts** (if present)

3. **Push Again**:
   ```bash
   git subtree push --prefix=examples reinhardt-examples main
   ```

## Troubleshooting

### Issue: "fatal: ambiguous argument 'reinhardt-examples/main'"

**Cause**: Remote not configured or not fetched

**Solution**:
```bash
git remote add -f reinhardt-examples https://github.com/kent8192/reinhardt-examples.git
```

### Issue: "Working tree has modifications"

**Cause**: Uncommitted changes in your working directory

**Solution**:
```bash
# Commit or stash your changes first
git add .
git commit -m "chore: save work in progress"

# Or stash
git stash
# Then run subtree command
git stash pop  # After subtree operation
```

### Issue: Subtree Pull Takes Too Long

**Cause**: Large history being processed

**Solution**:
- Be patient (first pull may take longer)
- Consider using `--squash` to reduce commit count
- Already using `--squash`? This is normal for first sync

### Issue: "Updates were rejected because the tip of your current branch is behind"

**Cause**: Remote has newer commits

**Solution**:
```bash
git subtree pull --prefix=examples reinhardt-examples main --squash
# Resolve conflicts if any
git subtree push --prefix=examples reinhardt-examples main
```

### Issue: Accidental Changes to Subtree

**Scenario**: You made changes to `examples/` but meant to work in the examples repository

**Solution 1**: Push to examples repository
```bash
# Commit in main repository
git add examples/
git commit -m "fix: update example"

# Push to examples repository
git subtree push --prefix=examples reinhardt-examples main
```

**Solution 2**: Revert and redo in examples repository
```bash
# Revert in main repository
git reset --hard HEAD~1

# Clone examples repository and make changes there
cd ../reinhardt-examples
# Make changes
git add .
git commit -m "fix: update example"
git push origin main

# Pull back into main repository
cd ../reinhardt-web
git subtree pull --prefix=examples reinhardt-examples main --squash
```

## Best Practices

### 1. Regular Syncing

Sync regularly to avoid large merge conflicts:

```bash
# Weekly or after significant changes
git subtree pull --prefix=examples reinhardt-examples main --squash
```

### 2. Clear Commit Messages

When working in the `examples/` directory of the main repository, use clear commit messages:

```bash
git commit -m "feat(examples): add websocket example"
# NOT: "update stuff"
```

### 3. Test Before Pushing

Before pushing to the examples repository, ensure all tests pass:

```bash
cd examples/local
cargo test --workspace --all --all-features
```

### 4. Communicate Changes

When making significant changes to examples, consider:
- Opening an issue in the examples repository first
- Coordinating with maintainers
- Documenting breaking changes

## Advanced Operations

### Viewing Subtree History

To see commits specific to the subtree:

```bash
git log --oneline -- examples/
```

### Splitting Out Commits

If you need to extract subtree-specific commits:

```bash
git subtree split --prefix=examples --branch examples-only
```

This creates a new branch with only commits that touched `examples/`.

### Changing Subtree Remote

If the examples repository URL changes:

```bash
git remote set-url reinhardt-examples NEW_URL
```

## References

- [Git Subtree Documentation](https://git-scm.com/docs/git-subtree)
- [Atlassian Git Subtree Tutorial](https://www.atlassian.com/git/tutorials/git-subtree)
- [Main Repository](https://github.com/kent8192/reinhardt-web)
- [Examples Repository](https://github.com/kent8192/reinhardt-examples)

---

**Need Help?**

- Open an issue in the examples repository
- Check the main repository's CONTRIBUTING.md
- Ask in GitHub Discussions
