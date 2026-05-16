#!/usr/bin/env bash
# Pre-step for `cargo make runserver`: mirror project `static/` into
# `dist/static/` so the framework's StaticFilesMiddleware can serve
# project-owned assets such as `static/css/style.css` referenced from
# `index.html`. The middleware only watches the `--static-dir`
# (defaulting to `dist/`), so without this copy the project's hand-written
# CSS and JS are unreachable and the SPA fallback returns `index.html`
# for those URLs (see kent8192/reinhardt-web#4483).
#
# Remove this script once kent8192/reinhardt-web#4484 lands and runserver
# auto-mounts the project `static/` directory.
set -euo pipefail

rm -rf dist/static
mkdir -p dist/static
cp -R static/. dist/static/
