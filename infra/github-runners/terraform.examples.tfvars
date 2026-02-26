# Copy to terraform.tfvars and fill in values.
# NEVER commit terraform.tfvars (it is gitignored)
#
# SETUP: After filling in this file, run:
#   ./init.sh
# This script auto-generates backend.tfvars from aws_account_id and runs terraform init.

aws_region     = "us-east-1"
aws_account_id = "123456789012"  # aws sts get-caller-identity --query Account --output text

# GitHub App credentials (from Task 3)
github_app_id              = "TBD"
github_app_installation_id = "TBD"
github_app_key_base64      = "LS0tLS1CRUdJTi..."

# GitHub repository (specify the actual owner username)
github_owner      = "your-github-username"
github_repository = "reinhardt-web"

# Runner configuration
runner_instance_types = ["c6a.2xlarge", "c6i.2xlarge", "c5a.2xlarge"]
runner_max_count      = 30
runner_extra_labels   = ["reinhardt-ci"]
runner_ebs_size_gb    = 200

# Budget circuit breaker
monthly_budget_limit_usd = "100"
budget_alert_email       = "your-email@example.com"

prefix = "reinhardt-ci"
