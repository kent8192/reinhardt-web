#!/bin/bash
# Test all examples

set -e

echo "🧪 Testing all examples..."
echo ""

# Determine if cargo-nextest is available
if command -v cargo-nextest &> /dev/null; then
	TEST_CMD="cargo nextest run"
	echo "📦 Using cargo-nextest for faster testing"
else
	TEST_CMD="cargo test"
	echo "📦 Using cargo test (install cargo-nextest for faster execution)"
fi
echo ""

FAILED_EXAMPLES=()

# Test example projects
echo "=== Testing Examples ==="
echo ""

for example in examples-*/; do
	if [ ! -d "$example" ]; then
		continue
	fi

	example_name="${example%/}"
	echo "📦 Testing ${example_name}..."

	cd "$example" || {
		echo "❌ Error: Cannot enter ${example_name} directory"
		FAILED_EXAMPLES+=("$example_name")
		continue
	}

	# Run tests
	if $TEST_CMD --all-features; then
		echo "✅ ${example_name} tests passed"
	else
		echo "❌ ${example_name} tests failed"
		FAILED_EXAMPLES+=("$example_name")
	fi

	cd .. || exit 1
	echo ""
done

# Test common crates
echo "=== Testing Common Crates ==="
echo ""

echo "📦 Testing common crates..."
if cargo test -p example-common -p example-test-macros --all-features; then
	echo "✅ Common crates tests passed"
else
	echo "❌ Common crates tests failed"
	FAILED_EXAMPLES+=("common-crates")
fi
echo ""

# Summary
echo "=== Test Summary ==="
echo ""

if [ ${#FAILED_EXAMPLES[@]} -eq 0 ]; then
	echo "✅ All tests passed!"
	exit 0
else
	echo "❌ Failed examples:"
	for failed in "${FAILED_EXAMPLES[@]}"; do
		echo "   - $failed"
	done
	echo ""
	echo "Total failed: ${#FAILED_EXAMPLES[@]}"
	exit 1
fi
