# Podman Setup Guide

## Overview

This guide explains how to set up Podman as a Docker Desktop alternative for running TestContainers in the Reinhardt project.

Podman is a daemonless container engine that provides a Docker-compatible CLI and API, making it an excellent alternative to Docker Desktop, especially for development environments.

## Prerequisites

- macOS 10.15 (Catalina) or later, Linux, or Windows 10/11 with WSL2
- Homebrew (for macOS) or equivalent package manager

## Installation

### macOS

#### Option 1: Using Homebrew (Recommended)

```bash
# Install Podman Desktop
brew install podman-desktop

# Or install CLI only
brew install podman
```

#### Option 2: Download from Official Site

Download Podman Desktop from [https://podman-desktop.io/downloads](https://podman-desktop.io/downloads)

### Linux

```bash
# Fedora/RHEL/CentOS
sudo dnf install podman

# Ubuntu/Debian
sudo apt-get update
sudo apt-get install podman

# Arch Linux
sudo pacman -S podman
```

### Windows

1. Install WSL2 if not already installed:
   ```powershell
   wsl --install
   ```

2. Download Podman Desktop from [https://podman-desktop.io/downloads](https://podman-desktop.io/downloads)

3. Follow the installation wizard

## Setup Instructions

### 1. Initialize Podman Machine (macOS/Windows)

On macOS and Windows, Podman runs in a lightweight VM called "Podman Machine".

```bash
# Initialize the Podman machine
podman machine init

# Start the Podman machine
podman machine start

# Verify Podman is running
podman version
podman info
```

**Note:** On Linux, Podman runs natively and doesn't require a machine VM.

### 2. Enable Podman Socket (Docker-compatible API)

The Podman socket provides a Docker-compatible API that TestContainers can use.

#### macOS

```bash
# Option A: Using Podman Desktop
# Open Podman Desktop → Settings → Resources → Enable "Start Podman socket on startup"

# Option B: Manual configuration
# SSH into the Podman machine
podman machine ssh

# Enable the Podman socket service
sudo systemctl enable --now podman.socket

# Exit the machine
exit
```

#### Linux

```bash
# Enable the Podman socket service
systemctl --user enable --now podman.socket

# Verify the socket is running
systemctl --user status podman.socket
```

#### Windows (WSL2)

```bash
# Open WSL2 terminal
wsl

# Enable the Podman socket
systemctl --user enable --now podman.socket

# Verify
systemctl --user status podman.socket
```

### 3. Configure Environment Variables

The Reinhardt project includes a `.cargo/config.toml` file that automatically sets the required environment variables for TestContainers to work with Podman:

- `DOCKER_HOST=unix:///var/run/podman/podman.sock`
- `TESTCONTAINERS_DOCKER_SOCKET_OVERRIDE=/var/run/podman/podman.sock`
- `TESTCONTAINERS_RYUK_DISABLED=true`

**No additional configuration is needed** if you're using `cargo test` or `cargo build` in the project directory.

For manual testing outside of Cargo, you can set these variables in your shell:

```bash
# Add to ~/.zshrc, ~/.bashrc, or ~/.profile
export DOCKER_HOST=unix:///var/run/podman/podman.sock
export TESTCONTAINERS_DOCKER_SOCKET_OVERRIDE=/var/run/podman/podman.sock
export TESTCONTAINERS_RYUK_DISABLED=true
```

## Verification

### 1. Verify Podman Installation

```bash
# Check Podman version
podman version

# Check system information
podman info

# List running containers (should be empty initially)
podman ps
```

### 2. Test Container Operations

```bash
# Pull a test image
podman pull postgres:16

# Run a test container
podman run --rm -d --name test-postgres -e POSTGRES_PASSWORD=password postgres:16

# Verify it's running
podman ps

# Stop the container
podman stop test-postgres
```

### 3. Verify TestContainers Integration

Run tests that use TestContainers:

```bash
# Run reinhardt-test tests
cargo test --package reinhardt-test --lib

# Run integration tests with database
TEST_DATABASE_URL=postgres://postgres@localhost:5432/postgres \
  cargo test --package reinhardt-integration-tests --test orm_composite_pk_query_tests

# Run all tests
cargo test --workspace --all --all-features
```

## Podman Machine Management

### Start/Stop the Machine

```bash
# Start the Podman machine
podman machine start

# Stop the Podman machine
podman machine stop

# Check machine status
podman machine list
```

### Resource Configuration

Adjust CPU and memory allocation for the Podman machine:

```bash
# Stop the machine first
podman machine stop

# Remove the existing machine
podman machine rm

# Initialize with custom resources
podman machine init --cpus 4 --memory 8192 --disk-size 100

# Start the machine
podman machine start
```

### Reset Podman Machine

If you encounter persistent issues:

```bash
# Stop and remove the machine
podman machine stop
podman machine rm

# Reinitialize
podman machine init
podman machine start
```

## Troubleshooting

### Issue: "Cannot connect to Podman socket"

**Symptoms:**
```
Error: unable to connect to socket: dial unix /var/run/podman/podman.sock: connect: no such file or directory
```

**Solution:**
1. Ensure Podman machine is running:
   ```bash
   podman machine start
   ```

2. Verify the socket is enabled:
   ```bash
   podman machine ssh
   systemctl status podman.socket
   exit
   ```

3. Restart the Podman machine:
   ```bash
   podman machine stop
   podman machine start
   ```

### Issue: "Permission denied" accessing Podman socket

**Symptoms:**
```
Error: unable to connect to socket: dial unix /var/run/podman/podman.sock: connect: permission denied
```

**Solution:**
1. Check socket permissions:
   ```bash
   podman machine ssh
   ls -la /var/run/podman/podman.sock
   exit
   ```

2. Ensure you're in the correct user group:
   ```bash
   # Linux only
   sudo usermod -aG podman $USER
   newgrp podman
   ```

### Issue: TestContainers timeout or hang

**Symptoms:**
- Tests using TestContainers time out
- Containers don't start properly

**Solution:**
1. Verify `TESTCONTAINERS_RYUK_DISABLED=true` is set (should be automatic via `.cargo/config.toml`)

2. Increase Podman machine resources:
   ```bash
   podman machine stop
   podman machine rm
   podman machine init --cpus 4 --memory 8192
   podman machine start
   ```

3. Check Podman logs:
   ```bash
   podman machine ssh
   journalctl -u podman.socket -f
   ```

### Issue: Container networking issues

**Symptoms:**
- Containers cannot connect to each other
- Port binding fails

**Solution:**
1. Check network configuration:
   ```bash
   podman network ls
   podman network inspect podman
   ```

2. Recreate the default network:
   ```bash
   podman network rm podman
   podman network create podman
   ```

### Issue: Podman machine won't start on macOS

**Symptoms:**
```
Error: qemu exited unexpectedly
```

**Solution:**
1. Ensure QEMU is installed:
   ```bash
   brew install qemu
   ```

2. Remove and reinitialize:
   ```bash
   podman machine stop
   podman machine rm
   podman machine init
   podman machine start
   ```

## Comparison: Podman vs Docker Desktop

| Feature                  | Podman                          | Docker Desktop                |
|--------------------------|---------------------------------|-------------------------------|
| **Daemon**               | Daemonless                      | Requires Docker daemon        |
| **Root privileges**      | Rootless by default             | Requires root/admin           |
| **Resource usage**       | Lower overhead                  | Higher overhead               |
| **License**              | Open source (Apache 2.0)        | Proprietary (paid for orgs)   |
| **Docker compatibility** | Docker CLI compatible           | Native Docker                 |
| **TestContainers**       | Supported (with configuration)  | Supported natively            |
| **Kubernetes**           | Built-in (Podman Desktop)       | Built-in (Docker Desktop)     |

## Additional Resources

- **Podman Official Documentation**: [https://docs.podman.io/](https://docs.podman.io/)
- **Podman Desktop**: [https://podman-desktop.io/](https://podman-desktop.io/)
- **TestContainers Podman Support**: [https://java.testcontainers.org/supported_docker_environment/#podman](https://java.testcontainers.org/supported_docker_environment/#podman)
- **Reinhardt Project**: [https://github.com/kent8192/reinhardt-rs](https://github.com/kent8192/reinhardt-rs)

## Getting Help

If you encounter issues not covered in this guide:

1. Check the [Reinhardt GitHub Issues](https://github.com/kent8192/reinhardt-rs/issues)
2. Consult the [Podman troubleshooting guide](https://github.com/containers/podman/blob/main/troubleshooting.md)
3. Ask in the Reinhardt community channels

---

**Last Updated:** 2025-01-27
