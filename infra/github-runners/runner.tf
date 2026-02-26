# Ubuntu 22.04 AMI for runners
data "aws_ami" "ubuntu" {
	most_recent = true
	owners      = ["099720109477"] # Canonical

	filter {
		name   = "name"
		values = ["ubuntu/images/hvm-ssd/ubuntu-22.04-amd64-server-*"]
	}
	filter {
		name   = "virtualization-type"
		values = ["hvm"]
	}
}

# JIT self-hosted runner infrastructure.
# Source: github-aws-runners/terraform-aws-github-runner (migrated from philips-labs Jan 2025)
# Handles: Webhook -> Lambda -> SQS -> EC2 Spot -> ephemeral runner -> terminate
module "github_runner" {
	source  = "github-aws-runners/github-runner/aws"
	version = "~> 6.1"

	aws_region = var.aws_region

	# Use default VPC for simplicity
	vpc_id     = data.aws_vpc.default.id
	subnet_ids = data.aws_subnets.default.ids

	prefix = var.prefix

	# GitHub App authentication (more secure than PAT)
	github_app = {
		key_base64     = var.github_app_key_base64
		id             = var.github_app_id
		webhook_secret = random_password.webhook_secret.result
	}

	# Runner configuration
	runners = {
		reinhardt-ci = {
			runner_os   = "linux"
			runner_arch = "x64"

			# Spot fleet with fallback instance types for availability
			instance_types            = var.runner_instance_types
			spot_price_bid_percentage = 150 # Bid 150% of on-demand to win most spots

			# Custom labels for runs-on matching in workflows
			runner_extra_labels = var.runner_extra_labels

			# Scale-to-zero: no idle runners = $0 when no CI runs
			min_runners = 0
			max_runners = var.runner_max_count

			# Ephemeral: each runner executes exactly ONE job, then auto-terminates
			enable_ephemeral_runners = true

			# JIT config: short-lived token instead of persistent runner token (security)
			enable_jit_config = true

			# AMI: Ubuntu 22.04
			ami_id = data.aws_ami.ubuntu.id

			# 200GB gp3 EBS: no need for free-disk-space cleanup step
			block_device_mappings = [{
				device_name           = "/dev/xvda"
				delete_on_termination = true
				volume_size           = var.runner_ebs_size_gb
				volume_type           = "gp3"
				iops                  = 3000 # gp3 baseline (included at no extra cost)
				throughput            = 125  # MB/s (included at no extra cost)
				encrypted             = true
			}]

			# Pre-installation script: Docker, TestContainers images, mold, etc.
			userdata_pre_install = file("${path.module}/runner-userdata.sh")

			# Repository scope (not org-wide, for security isolation)
			enable_organization_runners = false

			# CloudWatch Logs for debugging (ephemeral runners delete themselves)
			enable_cloudwatch_agent = true

			# SSM access for shell debugging (no SSH port needed)
			enable_ssm_on_runners = true

			# Scale-down: check every minute, terminate idle runners immediately
			# (ephemeral runners self-terminate after job, this is a safety net)
			scale_down_schedule_expression  = "cron(* * * * ? *)"
			minimum_running_time_in_minutes = 0
			runner_boot_time_in_minutes     = 10 # Allow 10 min for cold start
		}
	}
}
