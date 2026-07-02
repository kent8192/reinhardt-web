import fs from "node:fs";
import path from "node:path";
import toml from "toml";
import type { BenchmarkManifest, ScenarioConfig, TargetConfig, TargetMode } from "./types.js";

const modes = new Set<TargetMode>(["csr", "ssr", "wasm"]);

function requireString(value: unknown, pathName: string): string {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`${pathName} must be a non-empty string`);
  }
  return value;
}

function requireNumber(value: unknown, pathName: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    throw new Error(`${pathName} must be a positive number`);
  }
  return value;
}

function requireInteger(value: unknown, pathName: string): number {
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${pathName} must be a positive integer`);
  }
  return value;
}

function requireStringArray(value: unknown, pathName: string): string[] {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`${pathName} must be a non-empty string array`);
  }
  for (const [index, item] of value.entries()) {
    if (typeof item !== "string" || item.length === 0) {
      throw new Error(`${pathName}.${index} must be a non-empty string`);
    }
  }
  return value;
}

function scenarioEntries(value: unknown): ScenarioConfig[] {
  if (!value || typeof value !== "object") {
    throw new Error("scenarios must be a table");
  }
  return Object.entries(value).map(([id, raw]) => ({
    id,
    label: requireString((raw as { label?: unknown }).label, `scenarios.${id}.label`)
  }));
}

function targetEntries(value: unknown, frontendRoot: string): TargetConfig[] {
  if (!value || typeof value !== "object") {
    throw new Error("targets must be a table");
  }
  return Object.entries(value).map(([id, raw]) => {
    const table = raw as Record<string, unknown>;
    const mode = requireString(table.mode, `targets.${id}.mode`) as TargetMode;
    if (!modes.has(mode)) {
      throw new Error(`targets.${id}.mode must be one of csr, ssr, wasm`);
    }
    const root = path.resolve(frontendRoot, requireString(table.root, `targets.${id}.root`));
    if (!fs.existsSync(root)) {
      throw new Error(`targets.${id}.root does not exist: ${root}`);
    }
    const sourcePatchFile = requireString(table.source_patch_file, `targets.${id}.source_patch_file`);
    const sourcePatchPath = path.resolve(root, sourcePatchFile);
    const relativePatchPath = path.relative(root, sourcePatchPath);
    if (relativePatchPath.startsWith("..") || path.isAbsolute(relativePatchPath)) {
      throw new Error(`targets.${id}.source_patch_file must stay within target root`);
    }
    if (!fs.existsSync(sourcePatchPath)) {
      throw new Error(`targets.${id}.source_patch_file does not exist: ${sourcePatchFile}`);
    }
    return {
      id,
      label: requireString(table.label, `targets.${id}.label`),
      mode,
      root,
      install: requireString(table.install, `targets.${id}.install`),
      build: requireString(table.build, `targets.${id}.build`),
      preview: requireString(table.preview, `targets.${id}.preview`),
      dev: requireString(table.dev, `targets.${id}.dev`),
      url: requireString(table.url, `targets.${id}.url`),
      dev_url: requireString(table.dev_url, `targets.${id}.dev_url`),
      bundle_paths: requireStringArray(table.bundle_paths, `targets.${id}.bundle_paths`),
      source_patch_file: sourcePatchFile
    };
  });
}

export function loadManifest(frontendRoot: string): BenchmarkManifest {
  const manifestPath = path.join(frontendRoot, "suite.toml");
  const parsed = toml.parse(fs.readFileSync(manifestPath, "utf8"));
  const suite = {
    name: requireString(parsed.suite?.name, "suite.name"),
    sample_count: requireInteger(parsed.suite?.sample_count, "suite.sample_count"),
    warmup_count: requireInteger(parsed.suite?.warmup_count, "suite.warmup_count"),
    browser: requireString(parsed.suite?.browser, "suite.browser") as "chromium",
    timeout_ms: requireNumber(parsed.suite?.timeout_ms, "suite.timeout_ms")
  };
  if (suite.browser !== "chromium") {
    throw new Error("suite.browser must be chromium");
  }
  const scenarios = scenarioEntries(parsed.scenarios);
  if (scenarios.length === 0) {
    throw new Error("at least one scenario is required");
  }
  const targets = targetEntries(parsed.targets, frontendRoot);
  if (targets.length === 0) {
    throw new Error("at least one target is required");
  }
  return { suite, scenarios, targets };
}
