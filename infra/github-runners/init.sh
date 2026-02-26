#!/bin/bash
# init.sh: Generate backend.tfvars from terraform.tfvars and initialize Terraform.
#
# Usage:
#   cd infra/github-runners
#   cp terraform.examples.tfvars terraform.tfvars
#   # Edit terraform.tfvars (set aws_account_id, github_app_id, etc.)
#   ./init.sh
#
# What this script does:
#   1. Reads aws_account_id and aws_region from terraform.tfvars
#   2. Generates backend.tfvars with the correct S3 bucket name
#   3. Runs terraform init -backend-config=backend.tfvars

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

TFVARS_FILE="terraform.tfvars"
BACKEND_FILE="backend.tfvars"
STATE_KEY="github-runners/terraform.tfstate"

# Verify terraform.tfvars exists
if [[ ! -f "$TFVARS_FILE" ]]; then
  echo "ERROR: $TFVARS_FILE not found."
  echo "       Copy terraform.examples.tfvars to terraform.tfvars and fill in the values."
  exit 1
fi

# Extract aws_account_id (required - no default)
ACCOUNT_ID=$(awk -F'"' '/^[[:space:]]*aws_account_id[[:space:]]*=/ {print $2}' "$TFVARS_FILE")
if [[ -z "$ACCOUNT_ID" ]]; then
  echo "ERROR: aws_account_id is not set in $TFVARS_FILE"
  exit 1
fi

# Extract aws_region (optional - default us-east-1)
REGION=$(awk -F'"' '/^[[:space:]]*aws_region[[:space:]]*=/ {print $2}' "$TFVARS_FILE")
REGION="${REGION:-us-east-1}"

BUCKET_NAME="reinhardt-ci-terraform-state-${ACCOUNT_ID}"

# Generate backend.tfvars
cat > "$BACKEND_FILE" <<EOF
bucket       = "${BUCKET_NAME}"
key          = "${STATE_KEY}"
region       = "${REGION}"
use_lockfile = true
EOF

echo "Generated ${BACKEND_FILE}:"
echo "  bucket       = \"${BUCKET_NAME}\""
echo "  key          = \"${STATE_KEY}\""
echo "  region       = \"${REGION}\""
echo "  use_lockfile = true"
echo ""

# Run terraform init with generated backend config
terraform init -backend-config="$BACKEND_FILE"
