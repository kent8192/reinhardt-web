output "webhook_endpoint" {
	description = "GitHub App webhook URL - configure this in GitHub App settings under Webhook URL"
	value       = module.github_runner.webhook.endpoint
}

output "webhook_secret" {
	description = "GitHub App webhook secret - configure this in GitHub App settings"
	value       = random_password.webhook_secret.result
	sensitive   = true
}

output "runner_labels" {
	description = "Labels to use in GitHub Actions runs-on for self-hosted runners"
	value       = jsonencode(concat(["self-hosted", "linux", "x64"], var.runner_extra_labels))
}
