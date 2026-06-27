# Copy to terraform.tfvars and fill in values.
# NEVER commit terraform.tfvars (it is gitignored).
#
# Values marked <like-this> MUST be replaced before running terraform apply.
# Pre-filled values are sensible defaults and can be changed if needed.
#
# This stack is excluded from Terraform plan/apply CI because it uses local state
# and management-account credentials. Apply it manually with state in hand. See
# reinhardt-web#5393.

aws_region    = "us-east-1"
account_name  = "reinhardt-ci-runners"
account_email = "<your-email+reinhardt-ci@example.com>" # Must be unique across all AWS accounts
