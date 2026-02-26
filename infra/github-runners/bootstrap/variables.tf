variable "aws_region" {
	description = "AWS region"
	type        = string
	default     = "us-east-1"
}

variable "state_bucket_name" {
	description = "Globally unique S3 bucket name for Terraform state storage"
	type        = string
	# Example: "reinhardt-ci-terraform-state-123456789012"
}
