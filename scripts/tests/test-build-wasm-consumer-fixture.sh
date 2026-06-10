#!/usr/bin/env bash
# scripts/tests/test-build-wasm-consumer-fixture.sh
#
# Structural tests for .github/scripts/build-wasm-consumer-fixture.sh.
#
# These tests verify the static content of the script to confirm that:
#  - The augment-patch step (git apply wasm-consumer-augment.patch) was
#    removed as part of the URL routing simplification (Issue #4784).
#  - No stale references to the deleted fixture file remain.
#  - The step numbering is correct (4 steps, numbered 1–4).
#
# The tests do NOT execute the script (that requires cargo, reinhardt-admin,
# and a full WASM toolchain), so they are fast and suitable for local CI.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
FIXTURE_SCRIPT="$SCRIPT_DIR/../../.github/scripts/build-wasm-consumer-fixture.sh"

FAIL=0
pass() { echo "  PASS: $1"; }
fail() { echo "  FAIL: $1" >&2; FAIL=1; }

# ---------------------------------------------------------------------------
# 1. Verify removed augment-patch step is no longer present
# ---------------------------------------------------------------------------

if ! grep -q "git apply" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "01 git-apply step removed"
else
    fail "01 git-apply step removed (script still contains 'git apply')"
fi

if ! grep -q "wasm-consumer-augment.patch" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "02 no reference to deleted fixture file"
else
    fail "02 no reference to deleted fixture file (script still references wasm-consumer-augment.patch)"
fi

if ! grep -q "git apply.*augment" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "03 no git-apply augment command"
else
    fail "03 no git-apply augment command (script still calls 'git apply' with augment patch)"
fi

# ---------------------------------------------------------------------------
# 2. Verify the correct number of steps (4 steps, not 5)
# ---------------------------------------------------------------------------

STEP_COUNT=$(grep -c "::group::[0-9]*)" "$FIXTURE_SCRIPT" 2>/dev/null || true)
if [ "$STEP_COUNT" -eq 4 ]; then
    pass "04 script has exactly 4 steps"
else
    fail "04 script has exactly 4 steps (found $STEP_COUNT)"
fi

# ---------------------------------------------------------------------------
# 3. Verify step 4 is the cargo check step (the gate)
# ---------------------------------------------------------------------------

if grep -q "::group::4) cargo check" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "05 step 4 is the cargo check gate"
else
    fail "05 step 4 is the cargo check gate (::group::4) cargo check not found)"
fi

# ---------------------------------------------------------------------------
# 4. Verify step 5 does NOT exist (removed augment step was step 4 in old numbering,
#    and cargo check was step 5 — now renumbered to 4)
# ---------------------------------------------------------------------------

if ! grep -q "::group::5)" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "06 step 5 does not exist (renumbering is correct)"
else
    fail "06 step 5 does not exist (old step 5 still present after renumbering)"
fi

# ---------------------------------------------------------------------------
# 5. Verify the script still scaffolds the project and app (steps 1–3 intact)
# ---------------------------------------------------------------------------

if grep -q "::group::1)" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "07 step 1 (startproject) is present"
else
    fail "07 step 1 (startproject) is present"
fi

if grep -q "::group::2)" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "08 step 2 (startapp) is present"
else
    fail "08 step 2 (startapp) is present"
fi

if grep -q "::group::3)" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "09 step 3 (Rewrite Cargo.toml) is present"
else
    fail "09 step 3 (Rewrite Cargo.toml) is present"
fi

# ---------------------------------------------------------------------------
# 6. Verify the git init / git commit boilerplate was also removed
# ---------------------------------------------------------------------------

if ! grep -q "git init" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "10 git-init removed with augment step"
else
    fail "10 git-init removed with augment step (script still calls git init)"
fi

if ! grep -q "git commit" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "11 git-commit removed with augment step"
else
    fail "11 git-commit removed with augment step (script still calls git commit)"
fi

# ---------------------------------------------------------------------------
# 7. Verify cargo check command still targets wasm32-unknown-unknown
# ---------------------------------------------------------------------------

if grep -q "wasm32-unknown-unknown" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "12 cargo check still targets wasm32-unknown-unknown"
else
    fail "12 cargo check still targets wasm32-unknown-unknown"
fi

# ---------------------------------------------------------------------------
# 8. Verify PASS message is present (script ends with success marker)
# ---------------------------------------------------------------------------

if grep -q "wasm-consumer-fixture: PASS" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "13 PASS message present"
else
    fail "13 PASS message present"
fi

# ---------------------------------------------------------------------------
# 9. Regression: ensure the shebang and set -euo pipefail are still correct
# ---------------------------------------------------------------------------

FIRST_LINE=$(head -1 "$FIXTURE_SCRIPT")
if [ "$FIRST_LINE" = "#!/usr/bin/env bash" ]; then
    pass "14 shebang is correct"
else
    fail "14 shebang is correct (got: $FIRST_LINE)"
fi

if grep -q "set -euo pipefail" "$FIXTURE_SCRIPT" 2>/dev/null; then
    pass "15 set -euo pipefail is present"
else
    fail "15 set -euo pipefail is present"
fi

exit "$FAIL"