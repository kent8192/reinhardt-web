import assert from "node:assert/strict";
import test from "node:test";
import { renderMarkdown } from "../report.js";

test("renderMarkdown omits overall ranking", () => {
  const markdown = renderMarkdown({
    measuredAt: "2026-06-29T00:00:00.000Z",
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
  });
  assert.match(markdown, /Frontend Framework Comparison/);
  assert.match(markdown, /Overall ranking is intentionally omitted/);
});
