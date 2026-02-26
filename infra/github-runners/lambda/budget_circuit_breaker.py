"""
Budget Circuit Breaker Lambda
Triggered by AWS Budget SNS when monthly cost exceeds threshold.
Updates GitHub repository variable SELF_HOSTED_ENABLED=false to fallback to GitHub-hosted runners.
Manual reset required: update SELF_HOSTED_ENABLED=true in GitHub repo settings.
"""

import json
import os
import time
import urllib.request
import urllib.error
import boto3
import jwt


def get_ssm_parameter(name: str, decrypt: bool = False) -> str:
    ssm = boto3.client("ssm", region_name=os.environ["APP_AWS_REGION"])
    return ssm.get_parameter(Name=name, WithDecryption=decrypt)["Parameter"]["Value"]


def generate_jwt(app_id: str, private_key_pem: str) -> str:
    """Generate GitHub App JWT for API authentication (valid 10 minutes)."""
    now = int(time.time())
    return jwt.encode(
        {"iat": now - 60, "exp": now + 600, "iss": app_id},
        private_key_pem,
        algorithm="RS256",
    )


def get_installation_token(jwt_token: str, installation_id: str) -> str:
    """Get short-lived GitHub App installation token."""
    req = urllib.request.Request(
        f"https://api.github.com/app/installations/{installation_id}/access_tokens",
        method="POST",
        headers={
            "Authorization": f"Bearer {jwt_token}",
            "Accept": "application/vnd.github+json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read())["token"]


def disable_self_hosted_runners(token: str, owner: str, repo: str) -> None:
    """Set SELF_HOSTED_ENABLED=false to fallback to GitHub-hosted runners."""
    data = json.dumps({"value": "false"}).encode()
    req = urllib.request.Request(
        f"https://api.github.com/repos/{owner}/{repo}/actions/variables/SELF_HOSTED_ENABLED",
        method="PATCH",
        data=data,
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "Content-Type": "application/json",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    with urllib.request.urlopen(req) as resp:
        print(f"GitHub variable updated: {resp.status}")


def handler(event, context):
    print(f"Budget alert triggered: {json.dumps(event)}")

    prefix = os.environ["PREFIX"]
    app_id = get_ssm_parameter(f"/{prefix}/github-app-id")
    private_key = get_ssm_parameter(f"/{prefix}/github-app-key", decrypt=True)
    installation_id = get_ssm_parameter(f"/{prefix}/github-app-installation-id")

    jwt_token = generate_jwt(app_id, private_key)
    token = get_installation_token(jwt_token, installation_id)

    owner = os.environ["GITHUB_OWNER"]
    repo = os.environ["GITHUB_REPO"]
    disable_self_hosted_runners(token, owner, repo)

    print(f"Self-hosted runners disabled for {owner}/{repo}. Manual reset required.")
    return {"statusCode": 200, "body": "Self-hosted runners disabled"}
