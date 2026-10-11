'use strict';

const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const { readFileSync } = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');

const root = execFileSync('git', ['rev-parse', '--show-toplevel'], {
	encoding: 'utf8',
}).trim();
const workflow = readFileSync(
	path.join(root, '.github/workflows/warn-invalid-breaking-change.yml'),
	'utf8',
);
const script = workflow.split('          script: |\n')[1];
assert.ok(script, 'the test must execute the actual workflow policy');
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
const policy = new AsyncFunction('github', 'context', 'core', script);
const marker = '<!-- invalid-breaking-change-target-warning -->';
const headSha = '1111111111111111111111111111111111111111';
const baseSha = '2222222222222222222222222222222222222222';

function approvalEvent(overrides = {}) {
	return {
		id: 100, event: 'labeled', label: { name: 'rc-breaking-approved' },
		actor: { login: 'maintainer' }, ...overrides,
	};
}

function pullRequest(overrides = {}) {
	return {
		number: 6276,
		title: 'fix(pages): support RadioInput',
		labels: [],
		base: { ref: 'main' },
		head: { ref: 'fix/issue-6270-form-radio-input', sha: headSha },
		...overrides,
	};
}

async function run(pr, options = {}) {
	const result = { failures: [], comments: [], checks: [], listed: 0, reads: [], eventsListed: 0, permissionsRead: [], collaboratorsListed: 0 };
	const context = {
		repo: { owner: 'kent8192', repo: 'reinhardt-web' },
		sha: baseSha,
		eventName: Object.hasOwn(options, 'dispatchInput') ? 'workflow_dispatch' : 'pull_request_target',
		ref: options.ref ?? 'refs/heads/main',
		payload: Object.hasOwn(options, 'dispatchInput')
			? { inputs: { pull_request_number: options.dispatchInput } }
			: { pull_request: options.payload ?? pr },
	};
	const github = {
		rest: {
			pulls: {
				get: async (request) => {
					result.reads.push(request);
					if (options.readError) throw new Error('HTTP 403 reading PR');
					return { data: pr };
				},
			},
			checks: {
				create: async (check) => {
					assert.equal(result.listed, 0, 'publish before comment lookup');
					assert.deepEqual(result.comments, [], 'publish before comment delivery');
					if (options.checkError) throw new Error('HTTP 403 creating check');
					result.checks.push(check);
				},
			},
			repos: {
				listCollaborators: Symbol('listCollaborators'),
				getCollaboratorPermissionLevel: async (request) => {
					result.permissionsRead.push(request);
					if (options.permissionError) throw new Error('HTTP 403 reading approval actor');
					return { data: options.access ?? { permission: 'admin', role_name: 'admin' } };
				},
			},
			issues: {
				listEvents: Symbol('listEvents'),
				listComments: Symbol('listComments'),
				createComment: async (comment) => {
					assert.equal(result.failures.length, 1, 'fail before posting');
					assertCheck(result, 'failure', pr.head.sha);
					if (options.postError) throw new Error('HTTP 403');
					result.comments.push(comment);
				},
			},
		},
		paginate: async (endpoint, request) => {
			if (endpoint === github.rest.repos.listCollaborators) {
				assert.deepEqual(request, { owner: 'kent8192', repo: 'reinhardt-web', per_page: 100 });
				result.collaboratorsListed += 1;
				if (options.collaboratorsError) throw new Error('HTTP 403 reading base roles');
				return options.collaborators ?? [];
			}
			if (endpoint === github.rest.issues.listEvents) {
				assert.deepEqual(request, {
					owner: 'kent8192', repo: 'reinhardt-web', issue_number: 6276, per_page: 100,
				});
				result.eventsListed += 1;
				if (options.eventsError) throw new Error('HTTP 403 reading approval history');
				return options.events ?? [approvalEvent()];
			}
			assert.equal(endpoint, github.rest.issues.listComments);
			assert.deepEqual(request, {
				owner: 'kent8192', repo: 'reinhardt-web',
				issue_number: 6276, per_page: 100,
			});
			assert.equal(result.failures.length, 1, 'fail before deduplication');
			assertCheck(result, 'failure', pr.head.sha);
			result.listed += 1;
			if (options.listError) throw new Error('HTTP 403 listing comments');
			return options.comments ?? [];
		},
	};
	const core = {
		setFailed: (message) => result.failures.push(message),
		info: () => {},
	};
	try {
		await policy(github, context, core);
	} catch (error) {
		result.error = error.message;
	}
	const dispatch = Object.hasOwn(options, 'dispatchInput');
	const target = dispatch ? Number(options.dispatchInput) : context.payload.pull_request.number;
	const rejectedInput = !Number.isSafeInteger(target) || target <= 0
		|| (dispatch && context.ref !== 'refs/heads/main');
	assert.deepEqual(result.reads, rejectedInput ? [] : [{
		owner: 'kent8192', repo: 'reinhardt-web', pull_number: target,
	}]);
	return result;
}

function assertCheck(result, conclusion, sha = headSha) {
	assert.equal(result.checks.length, 1);
	const { output, ...check } = result.checks[0];
	assert.deepEqual(check, {
		owner: 'kent8192',
		repo: 'reinhardt-web',
		name: 'Warn Invalid Breaking Change Target',
		head_sha: sha,
		status: 'completed',
		conclusion,
	});
	assert.equal(output.title, conclusion === 'failure'
		? 'Invalid breaking-change target' : 'Valid breaking-change target');
}

for (const title of [
	'fix!: support RadioInput',
	'fix!(pages): support RadioInput',
	'fix(pages)!: support RadioInput',
]) {
	test(`rejects a main-targeting breaking title: ${title}`, async () => {
		const result = await run(pullRequest({ title }));
		assert.equal(result.error, undefined);
		assertCheck(result, 'failure');
		assert.equal(result.failures.length, 1);
		assert.equal(result.comments.length, 1);
		assert.equal(result.comments[0].issue_number, 6276);
		assert.ok(result.comments[0].body.startsWith(marker));
	});
}

test('a breaking-change label fails even without a breaking title', async () => {
	const result = await run(pullRequest({ labels: [{ name: 'breaking-change' }] }));
	assertCheck(result, 'failure');
	assert.equal(result.failures.length, 1);
	assert.equal(result.comments.length, 1);
});


const securityApproval = [
	{ name: 'security' }, { name: 'rc-breaking-approved' }, { name: 'breaking-change' },
];

for (const title of ['fix!(security): remediate findings', 'fix(auth)!: revalidate sessions']) {
	test(`accepts a maintainer-approved security fix: ${title}`, async () => {
		const result = await run(pullRequest({
			title, labels: securityApproval,
		}));
		assertCheck(result, 'success');
		assert.match(result.checks[0].output.summary, /Approved security fix/);
		assert.deepEqual(result.failures, []);
		assert.deepEqual(result.comments, []);
		assert.equal(result.listed, 0);
	});
}

for (const labels of [[], [{ name: 'security' }], [{ name: 'rc-breaking-approved' }]]) {
	test(`security claims without both approval labels still fail: ${JSON.stringify(labels)}`, async () => {
		const result = await run(pullRequest({
			title: 'fix!(security): remediate findings', labels,
			head: { ref: 'security/remediate-findings', sha: headSha },
		}));
		assertCheck(result, 'failure');
		assert.equal(result.failures.length, 1);
	});
}

for (const access of [
	{ permission: 'read', role_name: 'triage' },
	{ permission: 'write', role_name: 'write' },
	{ permission: 'none', role_name: 'none' },
	{ permission: 'write', role_name: 'custom-labeler' },
]) {
	test(`rejects approval applied by ${access.role_name} access`, async () => {
		const result = await run(pullRequest({
			title: 'fix!(security): remediate findings', labels: securityApproval,
		}), { access });
		assert.equal(result.error, undefined);
		assertCheck(result, 'failure');
		assert.equal(result.failures.length, 1);
		assert.deepEqual(result.permissionsRead, [{
			owner: 'kent8192', repo: 'reinhardt-web', username: 'maintainer',
		}]);
	});
}

for (const access of [
	{ permission: 'write', role_name: 'maintain' },
	{ permission: 'admin', role_name: 'custom-admin' },
]) {
	test(`accepts approval applied by ${access.role_name} access`, async () => {
		const result = await run(pullRequest({
			title: 'fix!(security): remediate findings', labels: securityApproval,
		}), { access });
		assertCheck(result, 'success');
		assert.match(result.checks[0].output.summary, /verified maintainer @maintainer/);
	});
}

for (const events of [
	[],
	[approvalEvent({ event: 'unlabeled' })],
	[approvalEvent({ actor: null })],
	[approvalEvent({ label: { name: 'security' } })],
]) {
	test(`fails closed without a current approval event: ${JSON.stringify(events)}`, async () => {
		const result = await run(pullRequest({
			title: 'fix!(security): remediate findings', labels: securityApproval,
		}), { events });
		assertCheck(result, 'failure');
		assert.deepEqual(result.permissionsRead, []);
	});
}

test('a newer triager reapplication supersedes an old maintainer approval', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), {
		events: [
			approvalEvent({ id: 103, actor: { login: 'triager' } }),
			approvalEvent({ id: 101 }),
			approvalEvent({ id: 102, event: 'unlabeled' }),
		],
		access: { permission: 'read', role_name: 'triage' },
	});
	assertCheck(result, 'failure');
	assert.equal(result.permissionsRead[0].username, 'triager');
});

test('a maintainer can approve again after an unauthorized label event', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), {
		events: [
			approvalEvent({ id: 102, actor: { login: 'maintainer' } }),
			approvalEvent({ id: 101, actor: { login: 'triager' } }),
		],
	});
	assertCheck(result, 'success');
	assert.equal(result.permissionsRead[0].username, 'maintainer');
});

for (const option of ['eventsError', 'permissionError']) {
	test(`${option} publishes failure instead of retaining an old success`, async () => {
		const result = await run(pullRequest({
			title: 'fix!(security): remediate findings', labels: securityApproval,
		}), { [option]: true, comments: [{ body: marker }] });
		assert.equal(result.error, undefined);
		assertCheck(result, 'failure');
		assert.equal(result.failures.length, 1);
		assert.deepEqual(result.comments, []);
	});
}

for (const title of ['fix!(security): remediate findings', 'fix(security)!: remediate findings']) {
	test(`requires the announcement label for approved breaking title: ${title}`, async () => {
		const result = await run(pullRequest({
			title, labels: securityApproval.filter(label => label.name !== 'breaking-change'),
		}));
		assertCheck(result, 'failure');
		assert.equal(result.eventsListed, 0);
		assert.deepEqual(result.permissionsRead, []);
	});
}

test('recognizes a custom Maintain base role from returned permission flags', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), { access: { permission: 'write', role_name: 'custom-maintainer', user: { permissions: { maintain: true } } } });
	assertCheck(result, 'success');
	assert.equal(result.collaboratorsListed, 0);
});

test('resolves a custom Maintain base role when the permission response omits flags', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), {
		access: { permission: 'write', role_name: 'custom-maintainer' },
		collaborators: [{ login: 'MAINTAINER', permissions: { maintain: true, admin: false } }],
	});
	assertCheck(result, 'success');
	assert.equal(result.collaboratorsListed, 1);
});

test('cannot infer Maintain access from a custom role name or another collaborator', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), {
		access: { permission: 'write', role_name: 'custom-maintainer' },
		collaborators: [{ login: 'someone-else', permissions: { maintain: true } }],
	});
	assertCheck(result, 'failure');
});

test('custom-role lookup failure cannot retain an approved result', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), { access: { permission: 'write', role_name: 'custom-maintainer' }, collaboratorsError: true });
	assert.equal(result.error, undefined);
	assertCheck(result, 'failure');
});

test('manual reevaluation invalidates an old approval after access revocation', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), { dispatchInput: '6276', access: { permission: 'read', role_name: 'triage' } });
	assertCheck(result, 'failure');
	assert.equal(result.permissionsRead.length, 1);
});

test('manual reevaluation accepts a current maintainer on the current PR head', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), { dispatchInput: '6276' });
	assertCheck(result, 'success');
});

for (const dispatchInput of ['', '-1', '1.5', 'not-a-pr']) {
	test(`rejects invalid manual reevaluation input: ${dispatchInput}`, async () => {
		const result = await run(pullRequest(), { dispatchInput });
		assert.deepEqual(result.reads, []);
		assert.deepEqual(result.checks, []);
		assert.equal(result.failures.length, 1);
	});
}

test('manual reevaluation rejects an untrusted workflow ref', async () => {
	const result = await run(pullRequest(), { dispatchInput: '6276', ref: 'refs/heads/feature/untrusted' });
	assert.deepEqual(result.checks, []);
	assert.equal(result.failures.length, 1);
});

test('security approval requires a breaking title as well as all labels', async () => {
	const result = await run(pullRequest({ labels: securityApproval }));
	assertCheck(result, 'failure');
	assert.equal(result.failures.length, 1);
	assert.equal(result.eventsListed, 0);
});

test('current approval labels override an older unapproved event', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), {
		payload: pullRequest({ title: 'fix!(security): remediate findings' }),
		comments: [{ body: marker }],
	});
	assertCheck(result, 'success');
	assert.deepEqual(result.failures, []);
	assert.equal(result.listed, 0);
});

for (const removedLabel of ['security', 'rc-breaking-approved', 'breaking-change']) {
	test(`removing a required security exception label restores failure: ${removedLabel}`, async () => {
		const result = await run(pullRequest({
			title: 'fix!(security): remediate findings', labels: securityApproval.filter(label => label.name !== removedLabel),
		}), {
			payload: pullRequest({ title: 'fix!(security): remediate findings', labels: securityApproval }),
			comments: [{ body: marker }],
		});
		assertCheck(result, 'failure');
		assert.equal(result.failures.length, 1);
		assert.deepEqual(result.comments, []);
	});
}

test('an existing warning still fails without posting a duplicate', async () => {
	const result = await run(pullRequest({ title: 'fix!(pages): support RadioInput' }), {
		comments: [{ body: 'unrelated' }, { body: `${marker}\nExisting warning` }],
	});
	assert.equal(result.error, undefined);
	assertCheck(result, 'failure');
	assert.equal(result.failures.length, 1);
	assert.equal(result.listed, 1);
	assert.deepEqual(result.comments, []);
});

test('comment delivery failure cannot turn an invalid target green', async () => {
	const result = await run(pullRequest({ title: 'fix!: support RadioInput' }), {
		postError: true,
	});
	assert.equal(result.error, 'HTTP 403');
	assertCheck(result, 'failure');
	assert.equal(result.failures.length, 1);
});

for (const pr of [
	pullRequest(),
	pullRequest({ title: 'fix!: support RadioInput', base: { ref: 'develop/0.4.0' } }),
	pullRequest({ title: 'feat!: release transition', head: { ref: 'develop/0.4.0', sha: headSha } }),
]) {
	test(`accepts ${pr.head.ref} -> ${pr.base.ref}: ${pr.title}`, async () => {
		const result = await run(pr);
		assert.equal(result.error, undefined);
		assertCheck(result, 'success');
		assert.deepEqual(result.failures, []);
		assert.deepEqual(result.comments, []);
		assert.equal(result.listed, 0);
	});
}

test('an unversioned develop branch does not bypass the check', async () => {
	const result = await run(pullRequest({
		title: 'feat!: change API', head: { ref: 'develop/next', sha: headSha },
	}));
	assertCheck(result, 'failure');
	assert.equal(result.failures.length, 1);
});

test('current breaking metadata overrides a stale nonbreaking event', async () => {
	const result = await run(pullRequest({ title: 'fix!: support RadioInput' }), {
		payload: pullRequest(),
	});
	assertCheck(result, 'failure');
	assert.equal(result.failures.length, 1);
});

test('removing a mistaken label clears failure despite an old warning', async () => {
	const result = await run(pullRequest(), {
		payload: pullRequest({ labels: [{ name: 'breaking-change' }] }),
		comments: [{ body: marker }],
	});
	assertCheck(result, 'success');
	assert.deepEqual(result.failures, []);
	assert.equal(result.listed, 0);
});

test('retargeted current metadata overrides an old main event', async () => {
	const result = await run(pullRequest({
		title: 'fix!: support RadioInput', base: { ref: 'develop/0.4.0' },
	}), { payload: pullRequest({ title: 'fix!: support RadioInput' }) });
	assertCheck(result, 'success');
	assert.deepEqual(result.failures, []);
});

test('publishes to the current fork head instead of the base or stale event SHA', async () => {
	const currentSha = '3333333333333333333333333333333333333333';
	const result = await run(pullRequest({
		head: {
			ref: 'fix/fork-change', sha: currentSha,
			repo: { full_name: 'contributor/reinhardt-web' },
		},
	}), { payload: pullRequest() });
	assert.equal(result.error, undefined);
	assertCheck(result, 'success', currentSha);
});

test('comment lookup failure preserves the published failure', async () => {
	const result = await run(pullRequest({ title: 'fix!: support RadioInput' }), {
		listError: true,
	});
	assert.equal(result.error, 'HTTP 403 listing comments');
	assertCheck(result, 'failure');
	assert.deepEqual(result.comments, []);
});

test('check publication errors propagate without posting a warning', async () => {
	const result = await run(pullRequest({ title: 'fix!: support RadioInput' }), {
		checkError: true,
	});
	assert.equal(result.error, 'HTTP 403 creating check');
	assert.deepEqual(result.checks, []);
	assert.deepEqual(result.comments, []);
	assert.equal(result.listed, 0);
});

test('metadata read errors propagate without publishing a policy result', async () => {
	const result = await run(pullRequest(), { readError: true });
	assert.equal(result.error, 'HTTP 403 reading PR');
	assert.deepEqual(result.checks, []);
	assert.deepEqual(result.comments, []);
	assert.equal(result.listed, 0);
});

test('metadata changes trigger the trusted required check', () => {
	assert.match(workflow, /pull_request_target:/);
	const events = workflow.match(/types: \[([^\]]+)\]/)[1].split(',').map(s => s.trim());
	for (const event of ['edited', 'labeled', 'unlabeled', 'synchronize', 'ready_for_review']) {
		assert.ok(events.includes(event), `missing ${event} trigger`);
	}
	assert.doesNotMatch(workflow, /actions\/checkout/);
});

test('the publisher has check permission and a distinct automatic job name', () => {
	assert.match(workflow, /^  checks: write$/m);
	assert.match(workflow, /^    name: Publish Breaking Change Target Check$/m);
	assert.doesNotMatch(workflow, /^    name: Warn Invalid Breaking Change Target$/m);
});

test('custom Write base permissions cannot approve an exception', async () => {
	const result = await run(pullRequest({
		title: 'fix!(security): remediate findings', labels: securityApproval,
	}), {
		access: { permission: 'write', role_name: 'custom-maintainer' },
		collaborators: [{ login: 'maintainer', permissions: { maintain: false, admin: false, push: true } }],
	});
	assertCheck(result, 'failure');
});

test('manual reevaluation is registered and shares the PR concurrency key', () => {
	assert.match(workflow, /workflow_dispatch:\s+inputs:\s+pull_request_number:/);
	assert.match(workflow, /github\.event\.pull_request\.number \|\| inputs\.pull_request_number/);
});
