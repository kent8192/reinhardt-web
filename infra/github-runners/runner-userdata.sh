#!/bin/bash
# Pre-installation setup for reinhardt CI runners.
# Executed by github-aws-runners module before runner registration.

set -eo pipefail

# Install required system packages
apt-get update -qq
apt-get install -y --no-install-recommends \
	docker.io \
	docker-compose-v2 \
	build-essential \
	pkg-config \
	libssl-dev \
	protobuf-compiler \
	mold \
	clang \
	lld \
	curl \
	jq

# Enable Docker (required for TestContainers)
systemctl enable --now docker
usermod -aG docker ubuntu

# Optimize Docker for CI workloads
cat > /etc/docker/daemon.json << 'EOF'
{
  "max-concurrent-downloads": 10,
  "storage-driver": "overlay2"
}
EOF
systemctl restart docker

# Pre-pull TestContainers images (eliminates pull latency during CI jobs)
docker pull postgres:17-alpine
docker pull mysql:8.0
docker pull redis:7-alpine
docker pull rabbitmq:3-management-alpine

# Create .testcontainers.properties to force Docker socket (not Podman)
mkdir -p /home/ubuntu
cat > /home/ubuntu/.testcontainers.properties << 'EOF'
docker.client.strategy=org.testcontainers.dockerclient.UnixSocketClientProviderStrategy
EOF
chown -R ubuntu:ubuntu /home/ubuntu/.testcontainers.properties
