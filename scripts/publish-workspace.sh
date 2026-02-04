#!/usr/bin/env bash
set -e

# Script to publish workspace crates to crates.io, skipping already-published versions
# This script is necessary because `cargo publish --workspace` fails when any crate version
# already exists on crates.io, which happens during retry attempts or when only some crates
# were updated in a release.

echo "Publishing workspace crates to crates.io..."

# Get all workspace members with their names and versions
# Exclude packages with publish = [] (i.e., publish = false)
workspace_members=$(cargo metadata --format-version 1 --no-deps | jq -r '
  .packages[] | 
  select(.source == null and .publish == null) | 
  "\(.name):\(.version):\(.manifest_path)"
')

# Track if any crate was published
any_published=false

# Process each workspace member
while IFS=: read -r name version manifest_path; do
  echo ""
  echo "Checking $name@$version..."
  
  # Check if this version already exists on crates.io
  if cargo search "$name" --limit 1 | grep -q "^$name = \"$version\""; then
    echo "  ⏭️  Skipping $name@$version (already published)"
    continue
  fi
  
  echo "  📦 Publishing $name@$version..."
  
  # Get the directory containing the Cargo.toml
  crate_dir=$(dirname "$manifest_path")
  
  # Publish the crate
  if cargo publish --manifest-path "$manifest_path"; then
    echo "  ✅ Successfully published $name@$version"
    any_published=true
    # Wait a bit for crates.io to register the new version
    # This is important for dependency resolution in subsequent publishes
    sleep 10
  else
    echo "  ❌ Failed to publish $name@$version"
    exit 1
  fi
done <<< "$workspace_members"

echo ""
if [ "$any_published" = true ]; then
  echo "✅ Workspace publishing completed successfully!"
else
  echo "ℹ️  No new crate versions to publish (all versions already exist on crates.io)"
fi
