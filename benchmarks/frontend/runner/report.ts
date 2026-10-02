import fs from "node:fs";
import path from "node:path";
import type { BenchmarkResult, ReportPayload } from "./types.js";

export function writeReports(resultsDir: string, payload: ReportPayload): { jsonPath: string; markdownPath: string } {
  fs.mkdirSync(resultsDir, { recursive: true });
  const stamp = payload.measuredAt.replaceAll(":", "-");
  const jsonPath = path.join(resultsDir, `${stamp}-framework-ui-comparison.json`);
  const markdownPath = path.join(resultsDir, `${stamp}-framework-ui-comparison.md`);
  fs.writeFileSync(jsonPath, `${JSON.stringify(payload, null, 2)}\n`);
  fs.writeFileSync(markdownPath, renderMarkdown(payload));
  return { jsonPath, markdownPath };
}

export function renderMarkdown(payload: ReportPayload): string {
  return [
    `# Frontend Framework Comparison - ${payload.measuredAt.slice(0, 10)}`,
    "",
    `- Suite: \`${payload.suite.name}\``,
    `- Samples: ${payload.suite.sample_count}`,
    `- Warmups: ${payload.suite.warmup_count}`,
    "",
    "## Runtime Browser Metrics",
    "",
    renderRuntimeTable(payload.results),
    "",
    "## Production Build and Bundle Metrics",
    "",
    renderBuildTable(payload.results),
    "",
    "## Development Loop Metrics",
    "",
    renderDevTable(payload.results),
    "",
    "## Target Metadata",
    "",
    "| Target | Label | Mode |",
    "| --- | --- | --- |",
    ...payload.targets.map((target) => `| \`${target.id}\` | ${target.label} | ${target.mode} |`),
    "",
    "## Failures",
    "",
    renderFailures(payload.results),
    "",
    "## Methodology",
    "",
    "Runtime, production build/bundle, and development loop metrics are separate tables. Overall ranking is intentionally omitted.",
    ""
  ].join("\n");
}

function renderRuntimeTable(results: BenchmarkResult[]): string {
  const rows = results.filter((result) => result.type === "runtime");
  if (rows.length === 0) {
    return "No runtime metrics recorded.";
  }
  return [
    "| Target | Scenario | Metric | Mean ms | Min ms | Max ms | Status |",
    "| --- | --- | --- | ---: | ---: | ---: | --- |",
    ...rows.map((row) => `| \`${row.target}\` | \`${row.scenario}\` | \`${row.metric}\` | ${format(row.meanMs)} | ${format(row.minMs)} | ${format(row.maxMs)} | ${row.status} |`)
  ].join("\n");
}

function renderBuildTable(results: BenchmarkResult[]): string {
  const rows = results.filter((result) => result.type === "build");
  if (rows.length === 0) {
    return "No build metrics recorded.";
  }
  return [
    "| Target | Build ms | Start ms | Bytes | Gzip bytes | Brotli bytes | Status |",
    "| --- | ---: | ---: | ---: | ---: | ---: | --- |",
    ...rows.map((row) => `| \`${row.target}\` | ${format(row.prod_build_ms)} | ${format(row.prod_start_ms)} | ${row.bundle_bytes ?? ""} | ${row.bundle_gzip_bytes ?? ""} | ${row.bundle_brotli_bytes ?? ""} | ${row.status} |`)
  ].join("\n");
}

function renderDevTable(results: BenchmarkResult[]): string {
  const rows = results.filter((result) => result.type === "dev");
  if (rows.length === 0) {
    return "No development loop metrics recorded.";
  }
  return [
    "| Target | Dev start ms | HMR update ms | Status |",
    "| --- | ---: | ---: | --- |",
    ...rows.map((row) => `| \`${row.target}\` | ${format(row.dev_start_ms)} | ${format(row.hmr_update_ms)} | ${row.status} |`)
  ].join("\n");
}

function renderFailures(results: BenchmarkResult[]): string {
  const failures = results.filter((result) => result.status === "failed");
  if (failures.length === 0) {
    return "No target or scenario failures recorded.";
  }
  return failures
    .map((failure) => {
      const error = failure.error;
      return `- \`${failure.target}\` ${failure.type}: ${error?.kind ?? "error"} - ${error?.message ?? "unknown error"}`;
    })
    .join("\n");
}

function format(value: number | undefined): string {
  return typeof value === "number" ? value.toFixed(3) : "";
}
