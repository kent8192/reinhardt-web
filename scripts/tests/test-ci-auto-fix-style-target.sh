#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"
POLICY_FILE="$ROOT_DIR/.github/scripts/auto-fix-style-target.cjs"
WORKFLOW_FILE="$ROOT_DIR/.github/workflows/ci.yml"

if [[ ! -f "$POLICY_FILE" ]]; then
	echo "FAIL: expected shared auto-fix target policy at $POLICY_FILE" >&2
	exit 1
fi

node - "$POLICY_FILE" "$WORKFLOW_FILE" <<'NODE'
const assert = require('node:assert/strict');
const fs = require('node:fs');

const policyFile = process.argv[2];
const workflowFile = process.argv[3];
const { classifyBranchProtection } = require(policyFile);

assert.deepEqual(
  classifyBranchProtection({ protected: false }, []),
  { eligible: true, reason: 'eligible' },
);

assert.deepEqual(
  classifyBranchProtection(
    { protected: true, protection: { enabled: false } },
    [{ type: 'non_fast_forward' }],
  ),
  { eligible: true, reason: 'eligible-non-fast-forward-only' },
);

assert.deepEqual(
  classifyBranchProtection(
    { protected: true, protection: { enabled: false } },
    [{ type: 'required_pull_request' }],
  ),
  { eligible: false, reason: 'protected-branch' },
);

assert.deepEqual(
  classifyBranchProtection(
    { protected: true, protection: { enabled: false } },
    [],
  ),
  { eligible: false, reason: 'protected-branch' },
);

assert.deepEqual(
  classifyBranchProtection(
    { protected: true, protection: { enabled: true } },
    [{ type: 'non_fast_forward' }],
  ),
  { eligible: false, reason: 'protected-branch' },
);

const workflow = fs.readFileSync(workflowFile, 'utf8');
const sharedPolicyCalls = workflow.match(/runAutoFixTargetPolicy\(/g) ?? [];
assert.equal(
  sharedPolicyCalls.length,
  2,
  'initial and pre-commit target checks must share the same policy',
);

const trustedBaseCheckouts = workflow.match(
  /ref: \$\{\{ github\.event\.pull_request\.base\.sha \}\}/g,
) ?? [];
assert.equal(
  trustedBaseCheckouts.length,
  2,
  'both target checks must load policy code from the trusted base commit',
);

const trustedPolicyLoads = workflow.match(
  /require\('\.\/\.auto-fix-policy\/\.github\/scripts\/auto-fix-style-target\.cjs'\)/g,
) ?? [];
assert.equal(
  trustedPolicyLoads.length,
  2,
  'both target checks must execute the trusted base policy copy',
);
NODE

echo "PASS: CI style auto-fix target policy"
