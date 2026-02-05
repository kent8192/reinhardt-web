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
