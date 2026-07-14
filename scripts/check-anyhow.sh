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
	local output
	local status
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

	set +e
	output=$(awk -v root="$SCAN_ROOT/" '
		function canonical_toml_string(value, simple_string) {
			if (simple_string && (value == "package" || value == "anyhow")) {
				return "\"" value "\""
			}
			return "\"\""
		}

		function decode_ascii_unicode_escape(value, position, width, digits, digit_index, digit, digit_value, codepoint) {
			digits = substr(value, position + 2, width)
			if (length(digits) != width) {
				return ""
			}

			codepoint = 0
			for (digit_index = 1; digit_index <= width; digit_index++) {
				digit = tolower(substr(digits, digit_index, 1))
				digit_value = index("0123456789abcdef", digit) - 1
				if (digit_value < 0) {
					return ""
				}
				codepoint = codepoint * 16 + digit_value
			}

			if (codepoint < 1 || codepoint > 127) {
				return ""
			}
			return sprintf("%c", codepoint)
		}

		function normalized_table(value, position, character, in_basic_string, in_literal_string, result, escape_type, decoded, width) {
			in_basic_string = 0
			in_literal_string = 0
			result = ""

			for (position = 1; position <= length(value); position++) {
				character = substr(value, position, 1)
				if (in_basic_string) {
					if (character == "\\") {
						escape_type = substr(value, position + 1, 1)
						width = escape_type == "u" ? 4 : escape_type == "U" ? 8 : 0
						if (width > 0) {
							decoded = decode_ascii_unicode_escape(value, position, width)
							result = result (decoded == "" ? "?" : decoded)
							position += width + 1
						} else {
							result = result "?"
							position++
						}
					} else if (character == "\"") {
						in_basic_string = 0
					} else {
						result = result character
					}
				} else if (in_literal_string) {
					if (character == "\047") {
						in_literal_string = 0
					} else {
						result = result character
					}
				} else if (character == "\"") {
					in_basic_string = 1
				} else if (character == "\047") {
					in_literal_string = 1
				} else if (character !~ /[[:space:]]/) {
					result = result character
				}
			}

			return result
		}

		function scan_toml_line(value, position, character, escape_type, decoded) {
			line_code = value
			line_tokens = lexer_multiline ? lexer_pending_tokens : ""
			line_brace_delta = 0
			line_started_in_multiline = lexer_multiline

			if (lexer_multiline && !lexer_skip_whitespace) {
				if (lexer_initial_newline) {
					lexer_initial_newline = 0
				} else {
					lexer_string_value = lexer_string_value "\n"
				}
			}

			for (position = 1; position <= length(value); position++) {
				character = substr(value, position, 1)

				if (lexer_multiline && lexer_skip_whitespace) {
					if (character ~ /[[:space:]]/) {
						continue
					}
					lexer_skip_whitespace = 0
				}

				if (lexer_basic_string) {
					if (lexer_escaped) {
						lexer_string_value = lexer_string_value character
						lexer_escaped = 0
					} else if (character == "\\") {
						escape_type = substr(value, position + 1, 1)
						if (escape_type == "u") {
							decoded = decode_ascii_unicode_escape(value, position, 4)
						} else if (escape_type == "U") {
							decoded = decode_ascii_unicode_escape(value, position, 8)
						} else {
							decoded = ""
						}

						if (decoded != "") {
							lexer_string_value = lexer_string_value decoded
							lexer_initial_newline = 0
							position += escape_type == "u" ? 5 : 9
						} else if (lexer_multiline && position == length(value)) {
							lexer_initial_newline = 0
							lexer_skip_whitespace = 1
						} else {
							lexer_simple_string = 0
							lexer_initial_newline = 0
							lexer_escaped = 1
						}
					} else if (lexer_multiline && substr(value, position, 3) == "\"\"\"") {
						line_tokens = line_tokens canonical_toml_string(lexer_string_value, lexer_simple_string)
						lexer_basic_string = 0
						lexer_multiline = 0
						lexer_initial_newline = 0
						position += 2
					} else if (!lexer_multiline && character == "\"") {
						line_tokens = line_tokens canonical_toml_string(lexer_string_value, lexer_simple_string)
						lexer_basic_string = 0
					} else {
						lexer_string_value = lexer_string_value character
						lexer_initial_newline = 0
					}
				} else if (lexer_literal_string) {
					if (lexer_multiline && substr(value, position, 3) == "\047\047\047") {
						line_tokens = line_tokens canonical_toml_string(lexer_string_value, lexer_simple_string)
						lexer_literal_string = 0
						lexer_multiline = 0
						lexer_initial_newline = 0
						position += 2
					} else if (!lexer_multiline && character == "\047") {
						line_tokens = line_tokens canonical_toml_string(lexer_string_value, lexer_simple_string)
						lexer_literal_string = 0
					} else {
						lexer_string_value = lexer_string_value character
						lexer_initial_newline = 0
					}
				} else if (substr(value, position, 3) == "\"\"\"") {
					lexer_string_value = ""
					lexer_simple_string = 1
					lexer_basic_string = 1
					lexer_multiline = 1
					lexer_initial_newline = 1
					position += 2
				} else if (substr(value, position, 3) == "\047\047\047") {
					lexer_string_value = ""
					lexer_simple_string = 1
					lexer_literal_string = 1
					lexer_multiline = 1
					lexer_initial_newline = 1
					position += 2
				} else if (character == "\"") {
					lexer_string_value = ""
					lexer_simple_string = 1
					lexer_basic_string = 1
				} else if (character == "\047") {
					lexer_string_value = ""
					lexer_simple_string = 1
					lexer_literal_string = 1
				} else if (character == "#") {
					line_code = substr(value, 1, position - 1)
					break
				} else {
					if (character !~ /[[:space:]]/) {
						line_tokens = line_tokens character
					}
					if (character == "{") {
						line_brace_delta++
					} else if (character == "}") {
						line_brace_delta--
					}
				}
			}

			if (lexer_multiline) {
				lexer_pending_tokens = line_tokens
				line_tokens = ""
			} else {
				lexer_pending_tokens = ""
			}
		}

		function is_dependency_table(value) {
			return value ~ /^\[(dependencies|dev-dependencies|build-dependencies)\]$/ || value ~ /^\[workspace[.]dependencies\]$/ || value ~ /^\[target[.].*[.](dependencies|dev-dependencies|build-dependencies)\]$/
		}

		function is_dependency_subtable(value) {
			return value ~ /^\[(dependencies|dev-dependencies|build-dependencies)[.][[:alnum:]_-]+\]$/ || value ~ /^\[workspace[.]dependencies[.][[:alnum:]_-]+\]$/ || value ~ /^\[target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.][[:alnum:]_-]+\]$/
		}

		function is_prohibited_dependency_entry(value) {
			return value ~ /^anyhow([.][[:alnum:]_-]+)?=/ || value ~ /^[[:alnum:]_-]+[.]package=anyhow$/
		}

		function is_prohibited_dependency_tokens(value) {
			return value ~ /^(anyhow|"anyhow")([.][[:alnum:]_-]+)?=/ || value ~ /^[[:alnum:]_-]+[.](package|"package")="anyhow"$/
		}

		function is_prohibited_root_dependency_entry(value) {
			if (value ~ /^(dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^(dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else if (value ~ /^workspace[.]dependencies[.]/) {
				sub(/^workspace[.]dependencies[.]/, "", value)
			} else if (value ~ /^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else {
				return 0
			}
			return is_prohibited_dependency_entry(value)
		}

		function is_prohibited_root_dependency_tokens(value) {
			if (value ~ /^(dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^(dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else if (value ~ /^workspace[.]dependencies[.]/) {
				sub(/^workspace[.]dependencies[.]/, "", value)
			} else if (value ~ /^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else {
				return 0
			}
			return is_prohibited_dependency_tokens(value)
		}

		function is_root_dependency_inline_start(value) {
			if (value ~ /^(dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^(dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else if (value ~ /^workspace[.]dependencies[.]/) {
				sub(/^workspace[.]dependencies[.]/, "", value)
			} else if (value ~ /^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/) {
				sub(/^target[.].*[.](dependencies|dev-dependencies|build-dependencies)[.]/, "", value)
			} else {
				return 0
			}
			return value ~ /^[[:alnum:]_-]+=\{/
		}

		function has_anyhow_package_field(value) {
			return value ~ /(^|[,{}])("package"|package)="anyhow"([,}]|$)/
		}

		function emit_match() {
			print substr(FILENAME, length(root) + 1) ":" FNR ":" $0
		}

		FNR == 1 {
			table = ""
			dependency_table = 0
			dependency_subtable = 0
			feature_table = 0
			inline_dependency_entry = 0
			inline_dependency_depth = 0
			lexer_basic_string = 0
			lexer_literal_string = 0
			lexer_multiline = 0
			lexer_escaped = 0
			lexer_initial_newline = 0
			lexer_skip_whitespace = 0
			lexer_pending_tokens = ""
			lexer_string_value = ""
			lexer_simple_string = 1
		}

		{
			scan_toml_line($0)
			code = line_code
			tokens = line_tokens

			if (!line_started_in_multiline && code ~ /^[[:space:]]*\[/) {
				table = normalized_table(code)
				dependency_table = is_dependency_table(table)
				dependency_subtable = is_dependency_subtable(table)
				feature_table = table == "[features]"
				inline_dependency_entry = 0
				inline_dependency_depth = 0
				if (dependency_subtable && table ~ /[.]anyhow\]$/) {
					emit_match()
				}
				next
			}

			compact = code
			gsub(/[[:space:]"\047]/, "", compact)
			if (line_started_in_multiline) {
				compact = ""
			}
			matched = 0

			if (!inline_dependency_entry && (dependency_table && compact ~ /^[[:alnum:]_-]+=\{/ || table == "" && is_root_dependency_inline_start(compact))) {
				inline_dependency_entry = 1
				inline_dependency_depth = 0
			}

			if (table == "" && (is_prohibited_root_dependency_entry(compact) || is_prohibited_root_dependency_tokens(tokens))) {
				matched = 1
			} else if (dependency_table && (is_prohibited_dependency_entry(compact) || is_prohibited_dependency_tokens(tokens))) {
				matched = 1
			} else if (dependency_subtable && (compact == "package=anyhow" || has_anyhow_package_field(tokens))) {
				matched = 1
			}

			if (inline_dependency_entry && has_anyhow_package_field(tokens)) {
				matched = 1
			}

			if ((table == "" && compact ~ /^features[.][[:alnum:]_-]+=/ || feature_table) && compact ~ /dep:anyhow/) {
				matched = 1
			}

			if (matched) {
				emit_match()
			}

			if (inline_dependency_entry) {
				inline_dependency_depth += line_brace_delta
				if (inline_dependency_depth <= 0) {
					inline_dependency_entry = 0
					inline_dependency_depth = 0
				}
			}
		}
	' "${manifest_paths[@]}")
	status=$?
	set -e

	if [ "$status" -ne 0 ]; then
		echo "anyhow-check: repository scan failed" >&2
		exit "$status"
	fi
	if [ -n "$output" ]; then
		printf '%s\n' "$output"
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
