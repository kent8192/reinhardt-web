import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { renderMarkdown, writeReports } from "../report.js";
import type { ReportPayload } from "../types.js";

test("renderMarkdown omits overall ranking", () => {
  const markdown = renderMarkdown(reportPayload("2026-06-29T00:00:00.000Z"));
  assert.match(markdown, /Frontend Framework Comparison/);
  assert.match(markdown, /Overall ranking is intentionally omitted/);
});

test("writeReports uses full measured timestamp in output paths", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "frontend-report-"));
  try {
    const first = writeReports(root, reportPayload("2026-06-29T00:00:00.000Z"));
    const second = writeReports(root, reportPayload("2026-06-29T00:00:01.000Z"));

    assert.notEqual(first.jsonPath, second.jsonPath);
    assert.notEqual(first.markdownPath, second.markdownPath);
    assert.match(path.basename(first.jsonPath), /2026-06-29T00-00-00\.000Z-framework-ui-comparison\.json/);
    assert.match(path.basename(second.markdownPath), /2026-06-29T00-00-01\.000Z-framework-ui-comparison\.md/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

function reportPayload(measuredAt: string): ReportPayload {
  return {
    measuredAt,
    suite: {
      name: "frontend-framework-comparison",
      sample_count: 1,
      warmup_count: 0,
      browser: "chromium",
      timeout_ms: 1000
    },
    targets: [],
    scenarios: [],
    results: [],
    environment: {
      node: "v22.0.0",
      platform: "darwin",
      arch: "arm64"
    }
  };
}
