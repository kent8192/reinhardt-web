# Store GitHub App credentials in SSM for Lambda access
resource "aws_ssm_parameter" "github_app_id" {
	name  = "/${var.prefix}/github-app-id"
	type  = "String"
	value = var.github_app_id
}

resource "aws_ssm_parameter" "github_app_installation_id" {
	name  = "/${var.prefix}/github-app-installation-id"
	type  = "String"
	value = var.github_app_installation_id
}

resource "aws_ssm_parameter" "github_app_key" {
	name  = "/${var.prefix}/github-app-key"
	type  = "SecureString"
	value = base64decode(var.github_app_key_base64)
}

# Package Lambda function
data "archive_file" "budget_circuit_breaker" {
	type        = "zip"
	source_file = "${path.module}/lambda/budget_circuit_breaker.py"
	output_path = "${path.module}/lambda/budget_circuit_breaker.zip"
}

# IAM role for Lambda
resource "aws_iam_role" "budget_circuit_breaker" {
	name = "${var.prefix}-budget-circuit-breaker"

	assume_role_policy = jsonencode({
		Version = "2012-10-17"
		Statement = [{
			Action    = "sts:AssumeRole"
			Effect    = "Allow"
			Principal = { Service = "lambda.amazonaws.com" }
		}]
	})
}

resource "aws_iam_role_policy" "budget_circuit_breaker" {
	name = "${var.prefix}-budget-circuit-breaker"
	role = aws_iam_role.budget_circuit_breaker.id

	policy = jsonencode({
		Version = "2012-10-17"
		Statement = [
			{
				Effect   = "Allow"
				Action   = ["logs:CreateLogGroup", "logs:CreateLogStream", "logs:PutLogEvents"]
				Resource = "arn:aws:logs:*:*:*"
			},
			{
				Effect = "Allow"
				Action = ["ssm:GetParameter"]
				Resource = [
					aws_ssm_parameter.github_app_id.arn,
					aws_ssm_parameter.github_app_installation_id.arn,
					aws_ssm_parameter.github_app_key.arn,
				]
			}
		]
	})
}

# Lambda function: disables self-hosted runners when budget exceeded.
# Uses APP_AWS_REGION instead of AWS_REGION (AWS_REGION is a reserved Lambda env var).
resource "aws_lambda_function" "budget_circuit_breaker" {
	filename         = data.archive_file.budget_circuit_breaker.output_path
	function_name    = "${var.prefix}-budget-circuit-breaker"
	role             = aws_iam_role.budget_circuit_breaker.arn
	handler          = "budget_circuit_breaker.handler"
	runtime          = "python3.12"
	source_code_hash = data.archive_file.budget_circuit_breaker.output_base64sha256
	timeout          = 30

	environment {
		variables = {
			PREFIX          = var.prefix
			GITHUB_OWNER    = var.github_owner
			GITHUB_REPO     = var.github_repository
			APP_AWS_REGION  = var.aws_region
		}
	}
}

# Allow SNS to invoke Lambda
resource "aws_lambda_permission" "budget_alert_sns" {
	statement_id  = "AllowSNSInvoke"
	action        = "lambda:InvokeFunction"
	function_name = aws_lambda_function.budget_circuit_breaker.function_name
	principal     = "sns.amazonaws.com"
	source_arn    = aws_sns_topic.budget_alert.arn
}
