'use strict';

const NON_WRITE_BLOCKING_RULES = new Set(['non_fast_forward']);

function classifyBranchProtection(branch, rules) {
	if (!branch.protected) {
		return { eligible: true, reason: 'eligible' };
	}

	const classicProtectionEnabled = branch.protection?.enabled !== false;
	const onlyNonWriteBlockingRules =
		Array.isArray(rules) &&
		rules.length > 0 &&
		rules.every((rule) => NON_WRITE_BLOCKING_RULES.has(rule.type));

	// reinhardt-web#5695: GitHub reports branches covered only by the
	// non-fast-forward ruleset as protected even though normal commits remain valid.
	if (!classicProtectionEnabled && onlyNonWriteBlockingRules) {
		return { eligible: true, reason: 'eligible-non-fast-forward-only' };
	}

	return { eligible: false, reason: 'protected-branch' };
}

function isProjectReadOnlyBranch(headRef) {
	return (
		headRef === 'main' ||
		headRef === 'master' ||
		headRef.startsWith('develop/') ||
		headRef.startsWith('release/') ||
		headRef.startsWith('release-plz-') ||
		headRef.startsWith('develop-release-plz-')
	);
}

async function runAutoFixTargetPolicy({ github, context, core, noticePrefix }) {
	const pr = context.payload.pull_request;
	const headRef = pr.head.ref;
	const repository = `${context.repo.owner}/${context.repo.repo}`;

	function finish(eligible, reason) {
		core.setOutput('eligible', eligible ? 'true' : 'false');
		core.setOutput('reason', reason);
		core.notice(`${noticePrefix}: ${reason}`);
	}

	if (pr.head.repo.full_name !== repository) {
		finish(false, 'fork-pull-request');
		return;
	}

	if (isProjectReadOnlyBranch(headRef)) {
		finish(false, 'project-read-only-branch');
		return;
	}

	try {
		const branchResponse = await github.rest.repos.getBranch({
			owner: context.repo.owner,
			repo: context.repo.repo,
			branch: headRef,
		});
		const branch = branchResponse.data;
		let rules = [];

		if (branch.protected) {
			const rulesResponse = await github.request(
				'GET /repos/{owner}/{repo}/rules/branches/{branch}',
				{
					owner: context.repo.owner,
					repo: context.repo.repo,
					branch: headRef,
				},
			);
			rules = rulesResponse.data;
		}

		const result = classifyBranchProtection(branch, rules);
		finish(result.eligible, result.reason);
	} catch (error) {
		core.warning(`Failed to evaluate branch protection for ${headRef}: ${error.message}`);
		finish(false, 'branch-protection-check-failed');
	}
}

module.exports = {
	classifyBranchProtection,
	runAutoFixTargetPolicy,
};
