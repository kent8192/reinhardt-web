# Import block template for bringing a manually-created Organizations account
# under Terraform management during manual apply.
#
# USAGE:
#   New account:      Run `terraform apply` to create a new sub-account.
#   Existing account: Uncomment the import block below with the actual account ID,
#                     then run `terraform init && terraform apply`.
#
# This stack is not planned or applied by CI. Preserve the local state file when
# running these commands manually. See reinhardt-web#5393.
#
# Get account ID: aws organizations list-accounts --query "Accounts[?Name=='reinhardt-ci-runners'].Id"

# Uncomment and set id when importing an existing account:
# import {
#   to = aws_organizations_account.ci_runners
#   id = "123456789012"  # Replace with actual account ID
# }
