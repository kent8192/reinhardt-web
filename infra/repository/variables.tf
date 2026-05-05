variable "github_owner" {
  description = "GitHub organization or user that owns the repository."
  type        = string
  default     = "kent8192"
}

variable "github_token" {
  description = "GitHub personal access token with admin repository permissions."
  type        = string
  sensitive   = true
}

variable "repository_name" {
  description = "Name of the GitHub repository to manage."
  type        = string
  default     = "reinhardt"
}

variable "environment_reviewer_user_id" {
  description = "Numeric GitHub user ID of the environment deployment reviewer. Look up via `gh api users/<username> --jq .id`. Using the numeric ID avoids a runtime `data \"github_user\"` lookup that would otherwise fail when the configured token cannot access the `/user` endpoint."
  type        = number

  validation {
    condition     = var.environment_reviewer_user_id > 0
    error_message = "environment_reviewer_user_id must be a positive integer (the reviewer's numeric GitHub user ID)."
  }
}
