import { describe, expect, it } from "vitest";
import { buildSyntheticWebhook } from "../src/payload-builder.js";

describe("buildSyntheticWebhook", () => {
	const baseInput = {
		job: {
			id: 24772441693,
			run_id: 98765,
			status: "queued" as const,
			created_at: "2026-04-22T10:00:00Z",
			labels: ["self-hosted", "linux", "arm64", "reinhardt-ci"],
			workflow_name: "CI",
			name: "Feature Check",
		},
		installationId: 112540205,
		owner: "kent8192",
		repo: "reinhardt-web",
	};

	it("sets action = 'queued'", () => {
		const { body } = buildSyntheticWebhook(baseInput);
		expect(body.action).toBe("queued");
	});

	it("propagates job fields verbatim", () => {
		const { body } = buildSyntheticWebhook(baseInput);
		expect(body.workflow_job.id).toBe(24772441693);
		expect(body.workflow_job.run_id).toBe(98765);
		expect(body.workflow_job.labels).toEqual([
			"self-hosted",
			"linux",
			"arm64",
			"reinhardt-ci",
		]);
	});

	it("sets repository.full_name = owner/repo", () => {
		const { body } = buildSyntheticWebhook(baseInput);
		expect(body.repository.full_name).toBe("kent8192/reinhardt-web");
		expect(body.repository.name).toBe("reinhardt-web");
		expect(body.repository.owner.login).toBe("kent8192");
	});

	it("includes installation.id for upstream webhook Lambda auth", () => {
		const { body } = buildSyntheticWebhook(baseInput);
		expect(body.installation.id).toBe(112540205);
	});

	it("generates fresh UUID per call (delivery_id header)", () => {
		const a = buildSyntheticWebhook(baseInput);
		const b = buildSyntheticWebhook(baseInput);
		expect(a.deliveryId).not.toBe(b.deliveryId);
		// UUID v4 format: 8-4-4-4-12 hex chars
		expect(a.deliveryId).toMatch(
			/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
		);
	});

	it("preserves labels exactly when self-hosted is missing", () => {
		const input = {
			...baseInput,
			job: { ...baseInput.job, labels: ["ubuntu-latest"] },
		};
		const { body } = buildSyntheticWebhook(input);
		expect(body.workflow_job.labels).toEqual(["ubuntu-latest"]);
	});
});
