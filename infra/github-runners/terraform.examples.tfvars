# Copy to terraform.tfvars and fill in values.
# NEVER commit terraform.tfvars (it is gitignored)

aws_region = "us-east-1"

# GitHub App credentials (from Task 3)
github_app_id              = "12345678"
github_app_installation_id = "87654321"
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
