#!/usr/bin/env bash
# Reject direct dynamic-error dependencies and owned source or API-documentation usage.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DEFAULT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SCAN_ROOT="$(cd "${1:-$DEFAULT_ROOT}" && pwd)"

COMMON_OUTPUT_ARGS=(
	--line-number
	--with-filename
	--no-heading
	--color never
)

COMMON_EXCLUSIONS=(
	--glob '!**/.git/**'
	--glob '!**/target/**'
	--glob '!**/vendor/**'
	--glob '!Cargo.lock'
	--glob '!CHANGELOG*'
	--glob '!docs/superpowers/**'
)

FOUND=0

scan() {
	local output
	local status

	set +e
	output=$(cd "$SCAN_ROOT" && rg "$@" "${COMMON_OUTPUT_ARGS[@]}" "${COMMON_EXCLUSIONS[@]}")
	status=$?
	set -e

	if [ "$status" -gt 1 ]; then
		echo "anyhow-check: repository scan failed" >&2
		exit "$status"
	fi
	if [ "$status" -eq 0 ]; then
		printf '%s\n' "$output"
		FOUND=1
	fi
}

scan_cargo_manifests() {
	local manifest
	local manifests
	local metadata_output
	local status
	local workspace_output
	local -a manifest_paths=()

	set +e
	manifests=$(cd "$SCAN_ROOT" && rg --files --glob 'Cargo.toml' "${COMMON_EXCLUSIONS[@]}")
	status=$?
	set -e

	if [ "$status" -gt 1 ]; then
		echo "anyhow-check: repository scan failed" >&2
		exit "$status"
	fi
	if [ "$status" -eq 1 ]; then
		return
	fi

	while IFS= read -r manifest; do
		manifest_paths+=("$SCAN_ROOT/$manifest")
	done < <(printf '%s\n' "$manifests")

	if ! command -v python3 >/dev/null 2>&1; then
		echo "anyhow-check: Python 3.11+ is required" >&2
		exit 1
	fi

	set +e
	metadata_output=$(python3 - "$SCAN_ROOT" "${manifest_paths[@]}" <<'PY'
import json
import os
import shutil
import subprocess
import sys
import tempfile

if sys.version_info < (3, 11):
    print("anyhow-check: Python 3.11+ is required", file=sys.stderr)
    raise SystemExit(2)


def relative_path(path, root):
    return os.path.relpath(path, root).replace(os.sep, "/")


def dependency_context(dependency):
    kind = dependency.get("kind")
    if kind == "dev":
        table_name = "dev-dependencies"
    elif kind == "build":
        table_name = "build-dependencies"
    else:
        table_name = "dependencies"

    target = dependency.get("target")
    if target:
        return f"target.{target}.{table_name}"
    return table_name


def emit_package_diagnostics(package, root, manifest_path=None):
    manifest_path = os.path.realpath(manifest_path or package["manifest_path"])
    relative_manifest = relative_path(manifest_path, root)
    for dependency in package.get("dependencies", []):
        if dependency.get("name") != "anyhow":
            continue
        alias = dependency.get("rename")
        detail = f'{alias} (package = "anyhow")' if alias else "anyhow"
        context = dependency_context(dependency)
        print(
            f"{relative_manifest}:1:remove direct anyhow dependency "
            f"from {context}: {detail}"
        )


def run_metadata(manifest_path, root):
    command = [
        "cargo",
        "metadata",
        "--no-deps",
        "--format-version",
        "1",
        "--manifest-path",
        manifest_path,
    ]
    result = subprocess.run(command, cwd=root, capture_output=True, text=True)
    if result.returncode == 0:
        return result, False

    if "current package believes it's in a workspace when it's not" not in result.stderr:
        return result, False

    with tempfile.TemporaryDirectory(prefix="anyhow-check-") as temp_directory:
        package_directory = os.path.join(temp_directory, "package")
        shutil.copytree(os.path.dirname(manifest_path), package_directory)
        isolated_manifest = os.path.join(package_directory, "Cargo.toml")
        with open(isolated_manifest, "a", encoding="utf-8") as manifest_file:
            manifest_file.write("\n[workspace]\n")
        isolated_command = command[:-1] + [isolated_manifest]
        isolated_result = subprocess.run(
            isolated_command,
            cwd=temp_directory,
            capture_output=True,
            text=True,
        )
        return isolated_result, True


root = os.path.realpath(sys.argv[1])
candidates = {os.path.realpath(path) for path in sys.argv[2:]}
pending = set(candidates)
diagnosed_packages = set()

while pending:
    requested_manifest = min(pending)
    result, isolated = run_metadata(requested_manifest, root)
    if result.returncode != 0:
        relative_manifest = relative_path(requested_manifest, root)
        message = result.stderr.strip() or result.stdout.strip()
        print(
            f"anyhow-check: cargo metadata failed for {relative_manifest}: {message}",
            file=sys.stderr,
        )
        raise SystemExit(result.returncode)

    try:
        metadata = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        relative_manifest = relative_path(requested_manifest, root)
        print(
            f"anyhow-check: invalid cargo metadata JSON for {relative_manifest}: {error}",
            file=sys.stderr,
        )
        raise SystemExit(2)

    if isolated:
        for package in metadata.get("packages", []):
            emit_package_diagnostics(package, root, requested_manifest)
        diagnosed_packages.add(requested_manifest)
        pending.discard(requested_manifest)
        continue

    unit_manifests = {
        os.path.realpath(os.path.join(metadata["workspace_root"], "Cargo.toml"))
    }
    for package in metadata.get("packages", []):
        package_manifest = os.path.realpath(package["manifest_path"])
        unit_manifests.add(package_manifest)
        if package_manifest in candidates and package_manifest not in diagnosed_packages:
            emit_package_diagnostics(package, root)
            diagnosed_packages.add(package_manifest)

    pending.difference_update(unit_manifests)
    pending.discard(requested_manifest)
PY
)
	status=$?
	set -e

	if [ "$status" -ne 0 ]; then
		echo "anyhow-check: Cargo manifest semantic scan failed" >&2
		exit "$status"
	fi

	set +e
	workspace_output=$(awk -v root="$SCAN_ROOT/" '
		function emit_dependency(dependency, detail) {
			detail = dependency == "anyhow" ? "anyhow" : dependency " (package = \"anyhow\")"
			print substr(FILENAME, length(root) + 1) ":1:remove direct anyhow dependency from workspace.dependencies: " detail
		}

		function decode_ascii_escape(value, position, width, digits, index_, digit, digit_value, codepoint) {
			digits = substr(value, position + 2, width)
			if (length(digits) != width) {
				return ""
			}
			codepoint = 0
			for (index_ = 1; index_ <= width; index_++) {
				digit = tolower(substr(digits, index_, 1))
				digit_value = index("0123456789abcdef", digit) - 1
				if (digit_value < 0) {
					return ""
				}
				codepoint = codepoint * 16 + digit_value
			}
			return codepoint > 0 && codepoint < 128 ? sprintf("%c", codepoint) : ""
		}

		function compact_toml_line(value, position, character, in_basic, in_literal, result, escape_type, width, decoded) {
			in_basic = 0
			in_literal = 0
			result = ""
			for (position = 1; position <= length(value); position++) {
				character = substr(value, position, 1)
				if (in_basic) {
					if (character == "\\") {
						escape_type = substr(value, position + 1, 1)
						width = escape_type == "u" ? 4 : escape_type == "U" ? 8 : 0
						if (width > 0) {
							decoded = decode_ascii_escape(value, position, width)
							result = result (decoded == "" ? "?" : decoded)
							position += width + 1
						} else {
							result = result "?"
							position++
						}
					} else if (character == "\"") {
						in_basic = 0
					} else {
						result = result character
					}
				} else if (in_literal) {
					if (character == "\047") {
						in_literal = 0
					} else {
						result = result character
					}
				} else if (character == "\"") {
					in_basic = 1
				} else if (character == "\047") {
					in_literal = 1
				} else if (character == "#") {
					break
				} else if (character !~ /[[:space:]]/) {
					result = result character
				}
			}
			return result
		}

		FNR == 1 {
			workspace_table = 0
			workspace_subtable = ""
			workspace_package_multiline = 0
		}

		/^[[:space:]]*\[/ {
			header = compact_toml_line($0)
			workspace_table = header == "[workspace.dependencies]"
			workspace_subtable = ""
			workspace_package_multiline = 0
			if (header ~ /^\[workspace[.]dependencies[.][[:alnum:]_-]+\]$/) {
				workspace_subtable = header
				sub(/^\[workspace[.]dependencies[.]/, "", workspace_subtable)
				sub(/\]$/, "", workspace_subtable)
				if (workspace_subtable == "anyhow") {
					emit_dependency("anyhow")
				}
			}
			next
		}

		{
			entry = compact_toml_line($0)

			if (workspace_package_multiline && entry == "anyhow") {
				emit_dependency(workspace_subtable)
				workspace_package_multiline = 0
			} else if (workspace_table && entry ~ /^anyhow([.][[:alnum:]_-]+)?=/) {
				emit_dependency("anyhow")
			} else if (workspace_subtable != "" && entry == "package=anyhow") {
				emit_dependency(workspace_subtable)
			} else if (workspace_subtable != "" && entry == "package=") {
				workspace_package_multiline = 1
			} else if (workspace_table && entry ~ /^[[:alnum:]_-]+=\{.*package=anyhow([,}]|$)/) {
				dependency = entry
				sub(/=.*/, "", dependency)
				emit_dependency(dependency)
			} else if (entry ~ /^workspace[.]dependencies[.]anyhow([.][[:alnum:]_-]+)?=/) {
				emit_dependency("anyhow")
			} else if (entry ~ /^workspace[.]dependencies[.][[:alnum:]_-]+[.]package=anyhow$/) {
				dependency = entry
				sub(/^workspace[.]dependencies[.]/, "", dependency)
				sub(/[.]package=anyhow$/, "", dependency)
				emit_dependency(dependency)
			}
		}
	' "${manifest_paths[@]}")
	status=$?
	set -e

	if [ "$status" -ne 0 ]; then
		echo "anyhow-check: workspace dependency scan failed" >&2
		exit "$status"
	fi
	if [ -n "$metadata_output" ]; then
		printf '%s\n' "$metadata_output"
		FOUND=1
	fi
	if [ -n "$workspace_output" ]; then
		printf '%s\n' "$workspace_output"
		FOUND=1
	fi
}

scan_cargo_manifests

scan \
	--glob '*.rs' \
	--regexp 'anyhow[[:space:]]*::' \
	--regexp 'anyhow[[:space:]]*!' \
	--regexp 'use[[:space:]]+anyhow([[:space:]]*;|[[:space:]]*::|[[:space:]]+as[[:space:]])'

scan \
	--glob '*.md' \
	--regexp 'anyhow[[:space:]]*::' \
	--regexp 'anyhow[[:space:]]*!' \
	--regexp 'use[[:space:]]+anyhow[[:space:]]*(;|::)'

if [ "$FOUND" -ne 0 ]; then
	echo "Remove direct anyhow dependencies and replace owned usage with repository error types." >&2
	exit 1
fi

echo "anyhow-check: OK"
