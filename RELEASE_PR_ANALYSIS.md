# Release PR Behavior Analysis

## Question
Does the CI at commit `4e2c7fdbc1a23f54ad050102e752af2f087629de` generate a new release PR?

## Answer
**No, this CI run did NOT create a new release PR.**

## Follow-up Question
Does the CI at commit `1c8c1dfae798cbc7a476b417ff59bebdef3059ec` (which failed to publish) create a new release PR?

## Answer to Follow-up
**Yes, it SHOULD create a new release PR, but it currently shows "already up-to-date" and does NOT create one. This appears to be an issue with release-plz's behavior after a failed publish.**

## Investigation Summary

### Workflow Run Details
- **Commit**: `4e2c7fdbc1a23f54ad050102e752af2f087629de`
- **Workflow**: Release-plz (`.github/workflows/release-plz.yml`)
- **Run ID**: 21701169069
- **Check Suite ID**: 56467515886
- **Status**: Completed successfully
- **Date**: 2026-02-05T06:20:40Z

### Why No PR Was Created

The workflow logs clearly show:
```
INFO the repository is already up-to-date
release_pr_output: {"prs":[]}
```

This means that release-plz analyzed the repository and determined that:
1. All local versions are already ahead of registry versions
2. Only CHANGELOGs need to be updated (which had already been done)
3. No new release PR is needed because the repository is "already up-to-date"

### How release-plz Works

Based on the configuration in `release-plz.toml` and `.github/workflows/release-plz.yml`:

#### Two-Stage Release Workflow
1. **Stage 1 - Release PR Creation** (`release-plz-pr` job):
   - Triggered on: Push to `main` branch
   - Action: `release-plz release-pr`
   - Purpose: Creates a Release PR (branch: `release-plz-*`) when there are releasable changes
   - Decision criteria:
     - Analyzes conventional commits since last release
     - Compares local versions with crates.io registry
     - Only creates PR if there are version changes needed

2. **Stage 2 - Publish** (`release-plz-release` job):
   - Triggered on: Push to `main` branch
   - Action: `release-plz release`
   - Purpose: Publishes crates to crates.io when a Release PR is merged
   - With `release_always = false`: Only publishes when commit is from a merged `release-plz-*` branch

#### When Does It Create a Release PR?

A Release PR is created when:
- ✅ There are conventional commits (feat:, fix:, etc.) since the last release
- ✅ The commits would result in a version bump
- ✅ The repository is NOT already up-to-date

A Release PR is NOT created when:
- ❌ The repository is already up-to-date (as in this case)
- ❌ All version bumps have already been applied
- ❌ Only CHANGELOG updates remain (which release-plz handles automatically)

### This Specific Case

For commit `4e2c7fdbc1a23f54ad050102e752af2f087629de`:
- The commit message: "Merge pull request #196 from kent8192/copilot/investigate-issue-reason"
- Type: Bug fix commit (`fix(ci):`)
- Result: Version bumps had already been applied in a previous release
- Outcome: Only CHANGELOG updates were needed, no new release PR

### Configuration Summary

Key settings in `release-plz.toml`:
```toml
release_always = false        # Only publish from release-plz-* branch merges
pr_branch_prefix = "release-plz-"  # Release PR branch naming
pr_labels = ["release", "automated"]  # Labels for Release PRs
pr_name = "chore: release"    # Release PR title
```

## Analysis of Failed Release at Commit 1c8c1df

### Workflow Run Details
- **Commit**: `1c8c1dfae798cbc7a476b417ff59bebdef3059ec`
- **Workflow**: Release-plz (`.github/workflows/release-plz.yml`)
- **Run ID**: 21696782575
- **Check Suite ID**: 56455765381
- **Status**: Completed with **failure**
- **Date**: 2026-02-05T02:46:15Z
- **Commit Type**: Merge of Release PR #194 (`release-plz-2026-02-05T01-33-00Z`)

### What Happened
1. **Release PR Job**: ✅ Succeeded
   - Checked for new releases to prepare
   - Result: `the repository is already up-to-date`
   - Output: `release_pr_output: {"prs":[]}`

2. **Release (Publish) Job**: ❌ Failed
   - Attempted to publish crates to crates.io
   - Published `reinhardt-core 0.1.0-alpha.3` successfully
   - **Failed** when trying to publish `reinhardt-http 0.1.0-alpha.5`
   - Error: `failed to select a version for the requirement 'reinhardt-urls = "^0.1.0-alpha.3"'`
   - Reason: `reinhardt-urls 0.1.0-alpha.3` was not yet on crates.io (only 0.1.0-alpha.2 available)

### The Problem

This reveals a **dependency ordering issue** in the release process:
- `reinhardt-http` depends on `reinhardt-urls ^0.1.0-alpha.3`
- Release PR #194 bumped both versions
- When merging the Release PR, the workflow tried to publish them
- `reinhardt-http` failed because `reinhardt-urls` wasn't published yet (or wasn't in the right order)

### Expected Behavior After Failed Publish

After a Release PR merge that **fails to publish**, the workflow should:
1. Recognize that the versions in the repository are ahead of what's on crates.io
2. Create a new Release PR to retry the release
3. Or at least not report "already up-to-date" when crates failed to publish

### Actual Behavior

The "Release PR" job reports:
```
INFO the repository is already up-to-date
release_pr_output: {"prs":[]}
```

This is **incorrect** because:
- The repository has versions that are NOT on crates.io
- The publish job failed, so those versions were never released
- A new Release PR should be created to retry

### Why This Happens

With `release_always = false`, release-plz checks if the current commit is from a merged Release PR branch (`release-plz-*`). Since commit `1c8c1df` IS from such a merge, the "Release" job runs. However, the "Release PR" job always runs and checks if there are new versions to release.

The issue is that release-plz appears to:
1. Compare local Cargo.toml versions with crates.io registry
2. See that local versions are higher
3. Conclude "already up-to-date" (meaning the local files don't need updating)
4. Not consider that those versions **failed to publish**

### The Real Question

**Should the workflow create a new Release PR after commit 4e2c7fd?**

**Yes**, because:
1. The previous Release PR (merged as `1c8c1df`) failed to publish
2. The repository contains unpublished versions
3. Those versions need to be released to crates.io

However, release-plz shows "already up-to-date" because it only checks if local files need updating, not if the registry publish succeeded.

## Conclusion

### For Commit 4e2c7fd
The CI workflow at commit `4e2c7fdbc1a23f54ad050102e752af2f087629de` correctly did not create a release PR because the repository was already up-to-date (local versions ahead of registry, CHANGELOGs updated).

### For Commit 1c8c1df  
The CI workflow at commit `1c8c1dfae798cbc7a476b417ff59bebdef3059ec` (the failed Release PR merge) shows a problem: it reports "already up-to-date" and creates no new Release PR, even though the publish failed. This is unexpected behavior.

### The Real Issue

The real issue is that **release-plz does not automatically create a new Release PR after a failed publish**. It only checks if local files need updating (version bumps, CHANGELOG), not if crates were successfully published to the registry.

**To fix this**, you would need to:
1. Manually trigger a re-run of the failed workflow, or
2. Make a new commit to main to trigger release-plz again, or
3. Manually create a new Release PR using `release-plz release-pr` locally, or
4. Configure the workflow to detect failed publishes and automatically retry

The current behavior where it says "already up-to-date" after a failed publish is technically correct from release-plz's perspective (the files don't need updating), but it's confusing because the versions aren't actually released to crates.io.

## How to Successfully Publish Updated Crates to crates.io

### Understanding release-plz Change Detection

**Important clarification:** release-plz does **NOT** detect changes primarily by git tags. Instead, it:

1. **Analyzes conventional commits** since the last release
2. **Compares local Cargo.toml versions** with crates.io registry versions
3. **Determines version bumps** based on commit types (feat:, fix:, etc.)
4. **Creates git tags** as part of the publishing process (not as a detection mechanism)

Git tags are created **after** successful publishing, not before. The configuration shows:
```toml
git_release_enable = true
git_tag_enable = true
git_tag_name = "{{ package }}@v{{ version }}"
```

These tags are created by the `release-plz release` command when crates are successfully published.

### Why the Failed Publish Didn't Create Tags

When Release PR #194 merged (commit `1c8c1df`):
1. The "Release PR" job ran and found "already up-to-date" (no new Release PR needed)
2. The "Release" job tried to publish crates but **failed** on `reinhardt-http`
3. Since the publish failed, **no git tags were created**
4. The versions remain in Cargo.toml but aren't on crates.io

### Solutions to Publish the Updated Crates

#### Option 1: Re-run the Failed Workflow ~~(Recommended)~~ ⚠️ **WILL FAIL**

**Important:** Re-running the failed workflow at commit `1c8c1df` will **NOT work** because:

1. The workflow checks out the specific commit where it ran (the Release PR merge commit `1c8c1df`)
2. At that commit, the dependency issue still exists in Cargo.toml
3. Re-running will hit the exact same dependency ordering problem

~~1. Go to the failed workflow run: https://github.com/kent8192/reinhardt-web/actions/runs/21696782575~~
~~2. Click "Re-run all jobs" or "Re-run failed jobs"~~
~~3. The "Release" job will attempt to publish again~~

**This option only works if:**
- The dependency issue was external (crates.io registry was temporarily unavailable)
- The required dependencies are now available on crates.io (published by another means)
- But in this case, the dependency ordering issue exists in the code itself at that commit

#### Option 2: Make a New Commit to Trigger release-plz (Recommended)

Since re-running won't work, you need to trigger the workflow on a newer commit:

1. **Option A - If dependency issue is fixed in later commits:**
   - The dependency issue may already be fixed in later commits to main
   - Any push to main will trigger the workflow
   - Make a small commit (e.g., update documentation) to trigger it
   - The workflow will run on the latest main commit where the issue might be resolved

2. **Option B - If dependency issue still exists:**
   - Fix the dependency ordering in the code (ensure proper publish order)
   - Commit the fix to main
   - This triggers the release-plz workflow on the new commit
   
3. **What happens:**
   - Since versions are already bumped, it will show "already up-to-date" for Release PR
   - But the "Release" job will attempt to publish again with the fixed code

#### Option 3: Manual Publish with release-plz CLI

If you have the release-plz CLI installed locally:

```bash
# Clone the repository
git clone https://github.com/kent8192/reinhardt-web.git
cd reinhardt-web

# Checkout the main branch
git checkout main

# Run release-plz release manually
release-plz release --git-token $GITHUB_TOKEN --registry-token $CARGO_REGISTRY_TOKEN
```

This will attempt to publish all crates that have versions ahead of crates.io.

#### Option 4: Fix the Dependency Ordering Issue

The root cause was that `reinhardt-http` depends on `reinhardt-urls ^0.1.0-alpha.3`, but that version wasn't published yet. To prevent this in the future:

1. **Ensure proper dependency order** in the release process
2. **Use `dependencies_update = true`** in `release-plz.toml` (currently set to `false`) to let release-plz manage dependency version updates
3. **Add a dependency graph check** to ensure dependencies are published before dependents

The current configuration has:
```toml
dependencies_update = false
```

Changing this to `true` would let release-plz automatically update dependency versions in the workspace, which might help avoid version mismatches.

### What Happens When the Publish Succeeds

When crates are successfully published:

1. **Crates uploaded to crates.io** ✅
2. **Git tags created** for each published crate (e.g., `reinhardt-http@v0.1.0-alpha.5`) ✅
3. **GitHub releases created** (if `git_release_enable = true`) ✅
4. **Future commits will trigger new Release PRs** because release-plz will detect new commits since the last tags ✅

### Detection After Failed Publish vs After Feature PR

You asked: "I think release-plz detect changes by git tags, so it can detect differences when feature PRs merged into main crate unless git tag created in failed PR, right?"

**Correction:** The detection mechanism is:

1. **For Release PR creation**: 
   - Analyzes conventional commits since last release
   - Compares local Cargo.toml versions with crates.io
   - If local > crates.io AND CHANGELOGs not yet updated → Create Release PR
   - If local > crates.io BUT CHANGELOGs already updated → "already up-to-date"

2. **For Publishing**:
   - With `release_always = false`, only publishes if commit is from `release-plz-*` branch merge
   - Publishes crates where local version > crates.io version
   - Creates git tags **after** successful publish

So in the failed publish case:
- ❌ No git tags were created (publish failed)
- ✅ Versions are still in Cargo.toml (ahead of crates.io)
- ✅ Future feature PR merges will NOT create new Release PR (versions already bumped)
- ⚠️ The unpublished versions need manual intervention to publish

The key insight: **Git tags mark successful releases, they don't trigger detection**. Commits and version comparisons drive the detection logic.
