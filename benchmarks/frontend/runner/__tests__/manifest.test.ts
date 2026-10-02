import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { loadManifest } from "../manifest.js";

test("loadManifest parses targets and scenarios", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "frontend-manifest-"));
  try {
    writeFixtureManifest(root, {
      sampleCount: "2",
      warmupCount: "1",
      sourcePatchFile: "src/App.jsx"
    });

    const manifest = loadManifest(root);
    assert.equal(manifest.suite.name, "test-suite");
    assert.equal(manifest.scenarios[0].id, "counter");
    assert.equal(manifest.targets[0].id, "app");
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("loadManifest rejects source patch files outside the target root", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "frontend-manifest-"));
  try {
    fs.mkdirSync(path.join(root, "outside"), { recursive: true });
    fs.writeFileSync(path.join(root, "outside", "App.jsx"), "");
    writeFixtureManifest(root, {
      sampleCount: "2",
      warmupCount: "1",
      sourcePatchFile: "../outside/App.jsx"
    });

    assert.throws(() => loadManifest(root), /source_patch_file must stay within target root/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

test("loadManifest rejects fractional sample counts", () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "frontend-manifest-"));
  try {
    writeFixtureManifest(root, {
      sampleCount: "1.5",
      warmupCount: "1",
      sourcePatchFile: "src/App.jsx"
    });

    assert.throws(() => loadManifest(root), /suite\.sample_count must be a positive integer/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});

function writeFixtureManifest(
  root: string,
  options: { sampleCount: string; warmupCount: string; sourcePatchFile: string }
): void {
  fs.mkdirSync(path.join(root, "fixtures", "app", "src"), { recursive: true });
  fs.writeFileSync(path.join(root, "fixtures", "app", "src", "App.jsx"), "");
  fs.writeFileSync(path.join(root, "suite.toml"), `
[suite]
name = "test-suite"
sample_count = ${options.sampleCount}
warmup_count = ${options.warmupCount}
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
source_patch_file = "${options.sourcePatchFile}"
`);
}
