# GitHub repository environments for deployment protection.
#
# The reviewer is identified by their numeric GitHub user ID supplied directly
# via `var.environment_reviewer_user_id`. We avoid `data "github_user"` here
# because the integrations/github provider routes that lookup through the
# configured token, which fails with `401 Bad credentials` whenever the token
# is a GitHub App installation token, an expired PAT, or a PAT lacking
# `read:user`. The numeric user ID is stable, so a static value is strictly
# preferable to a runtime API call. See reinhardt-web#4150.

# Gate for terraform plan on fork PRs.
# Requires manual approval before CI runs plan on external contributions.
resource "github_repository_environment" "fork_review" {
  environment = "fork-review"
  repository  = var.repository_name
  wait_timer  = 0

  prevent_self_review = true

  reviewers {
    users = [var.environment_reviewer_user_id]
  }
}

# Gate for terraform apply post-merge.
# Requires manual approval before applying infrastructure changes to production.
resource "github_repository_environment" "production" {
  environment = "production"
  repository  = var.repository_name
  wait_timer  = 0

  prevent_self_review = true

  reviewers {
    users = [var.environment_reviewer_user_id]
  }
}
