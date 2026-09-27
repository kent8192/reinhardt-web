#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
python3 -B "$SCRIPT_DIR/test_release_recovery.py"
