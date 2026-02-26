# SNS topic for budget alerts
resource "aws_sns_topic" "budget_alert" {
	name = "${var.prefix}-budget-alert"
}

# Email subscription for human notification
resource "aws_sns_topic_subscription" "budget_alert_email" {
	topic_arn = aws_sns_topic.budget_alert.arn
	protocol  = "email"
	endpoint  = var.budget_alert_email
}

# Lambda subscription: triggers automatic fallback to GitHub-hosted runners
resource "aws_sns_topic_subscription" "budget_alert_lambda" {
	topic_arn = aws_sns_topic.budget_alert.arn
	protocol  = "lambda"
	endpoint  = aws_lambda_function.budget_circuit_breaker.arn
}

# Monthly budget with alert at 80% (warning) and 100% (disable self-hosted)
resource "aws_budgets_budget" "ci_monthly" {
	name         = "${var.prefix}-monthly-budget"
	budget_type  = "COST"
	limit_amount = var.monthly_budget_limit_usd
	limit_unit   = "USD"
	time_unit    = "MONTHLY"

	# Cost filter: only count resources tagged to this project
	cost_filter {
		name   = "TagKeyValue"
		values = ["user:Project$reinhardt"]
	}

	# Warning at 80%: email only
	notification {
		comparison_operator        = "GREATER_THAN"
		threshold                  = 80
		threshold_type             = "PERCENTAGE"
		notification_type          = "ACTUAL"
		subscriber_sns_topic_arns  = [aws_sns_topic.budget_alert.arn]
		subscriber_email_addresses = [var.budget_alert_email]
	}

	# Critical at 100%: email + Lambda (disables self-hosted runners)
	notification {
		comparison_operator        = "GREATER_THAN"
		threshold                  = 100
		threshold_type             = "PERCENTAGE"
		notification_type          = "ACTUAL"
		subscriber_sns_topic_arns  = [aws_sns_topic.budget_alert.arn]
		subscriber_email_addresses = [var.budget_alert_email]
	}
}
