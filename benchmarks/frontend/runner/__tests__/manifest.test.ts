import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { loadManifest } from "../manifest.js";

test("loadManifest parses targets and scenarios", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "frontend-manifest-"));
  fs.mkdirSync(path.join(root, "fixtures", "app", "src"), { recursive: true });
  fs.writeFileSync(path.join(root, "fixtures", "app", "src", "App.jsx"), "");
  fs.writeFileSync(path.join(root, "suite.toml"), `
[suite]
name = "test-suite"
sample_count = 2
warmup_count = 1
browser = "chromium"
timeout_ms = 1000

[scenarios.counter]
label = "Counter"

[targets.app]
label = "App"
mode = "csr"
root = "fixtures/app"
install = "npm install"
build = "npm run build"
preview = "npm run preview"
dev = "npm run dev"
url = "http://127.0.0.1:1"
dev_url = "http://127.0.0.1:2"
bundle_paths = ["dist"]
source_patch_file = "src/App.jsx"
`);

  const manifest = loadManifest(root);
  assert.equal(manifest.suite.name, "test-suite");
  assert.equal(manifest.scenarios[0].id, "counter");
  assert.equal(manifest.targets[0].id, "app");
});
