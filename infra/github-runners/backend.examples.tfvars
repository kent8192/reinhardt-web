# Copy to backend.tfvars and fill in values from bootstrap outputs.
# Run: terraform init -backend-config=backend.tfvars
# NEVER commit backend.tfvars (it is gitignored)

bucket       = "reinhardt-ci-terraform-state-YOUR_ACCOUNT_ID"
key          = "github-runners/terraform.tfstate"
region       = "us-east-1"
use_lockfile = true
