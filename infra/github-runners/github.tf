# Repository variable: controls whether self-hosted runners are enabled.
# Set to "false" by budget circuit breaker Lambda when monthly cost exceeded.
# Manual reset: update to "true" in GitHub repo Settings > Variables.
resource "github_actions_variable" "self_hosted_enabled" {
	repository    = var.github_repository
	variable_name = "SELF_HOSTED_ENABLED"
	value         = "true"

	lifecycle {
		# Do not override if Lambda has set it to "false"
		ignore_changes = [value]
	}
}

# Automatically configure GitHub App webhook after infrastructure is created.
# The GitHub Terraform provider does not support GitHub App webhook settings,
# so we use a null_resource with local-exec to call the GitHub API directly.
#
# Generates a GitHub App JWT locally and calls PATCH /app/hook/config to set:
#   - url: the API Gateway webhook endpoint from the github-aws-runners module
#   - secret: the randomly generated webhook secret
#   - active: true
#   - events: ["workflow_job"]
resource "null_resource" "github_app_webhook" {
	# Re-run when webhook URL or secret changes
	triggers = {
		webhook_url    = module.github_runner.webhook.endpoint
		webhook_secret = random_password.webhook_secret.result
	}

	provisioner "local-exec" {
		# Generate GitHub App JWT (valid 10 min) and configure webhook via API
		command = <<-EOT
			set -euo pipefail

			APP_ID="${var.github_app_id}"
			KEY_B64="${var.github_app_key_base64}"
			WEBHOOK_URL="${module.github_runner.webhook.endpoint}"
			WEBHOOK_SECRET="${random_password.webhook_secret.result}"

			# Decode PEM key to temp file
			TMPKEY=$(mktemp /tmp/github_app_key_XXXXXX.pem)
			trap "rm -f $TMPKEY" EXIT
			echo "$KEY_B64" | base64 -d > "$TMPKEY"

			# Generate JWT using openssl (available on macOS/Linux without extra deps)
			NOW=$(date +%s)
			IAT=$((NOW - 60))
			EXP=$((NOW + 540))
			HEADER=$(printf '{"alg":"RS256","typ":"JWT"}' | base64 | tr -d '=' | tr '+/' '-_' | tr -d '\n')
			PAYLOAD=$(printf '{"iat":%d,"exp":%d,"iss":"%s"}' "$IAT" "$EXP" "$APP_ID" | base64 | tr -d '=' | tr '+/' '-_' | tr -d '\n')
			SIG=$(printf '%s.%s' "$HEADER" "$PAYLOAD" | openssl dgst -sha256 -sign "$TMPKEY" | base64 | tr -d '=' | tr '+/' '-_' | tr -d '\n')
			JWT="$HEADER.$PAYLOAD.$SIG"

			# Configure GitHub App webhook
			curl -sf -X PATCH "https://api.github.com/app/hook/config" \
				-H "Authorization: Bearer $JWT" \
				-H "Accept: application/vnd.github+json" \
				-H "X-GitHub-Api-Version: 2022-11-28" \
				-d "{\"url\":\"$WEBHOOK_URL\",\"secret\":\"$WEBHOOK_SECRET\",\"active\":true}" \
				> /dev/null

			# Subscribe to workflow_job events
			curl -sf -X PATCH "https://api.github.com/app" \
				-H "Authorization: Bearer $JWT" \
				-H "Accept: application/vnd.github+json" \
				-H "X-GitHub-Api-Version: 2022-11-28" \
				-d '{"events":["workflow_job"]}' \
				> /dev/null

			echo "GitHub App webhook configured: $WEBHOOK_URL"
		EOT
	}

	depends_on = [module.github_runner]
}
