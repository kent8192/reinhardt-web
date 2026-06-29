export type TargetMode = "csr" | "ssr" | "wasm";

export type ResultStatus = "ok" | "failed";

export interface SuiteConfig {
  name: string;
  sample_count: number;
  warmup_count: number;
  browser: "chromium";
  timeout_ms: number;
}

export interface ScenarioConfig {
  id: string;
  label: string;
}

export interface TargetConfig {
  id: string;
  label: string;
  mode: TargetMode;
  root: string;
  install: string;
  build: string;
  preview: string;
  dev: string;
  url: string;
  dev_url: string;
  bundle_paths: string[];
  source_patch_file: string;
}

export interface BenchmarkManifest {
  suite: SuiteConfig;
  scenarios: ScenarioConfig[];
  targets: TargetConfig[];
}

export interface CommandResult {
  command: string;
  cwd: string;
  exitCode: number;
  stdout: string;
  stderr: string;
  durationMs: number;
  timedOut: boolean;
}

export interface BenchmarkError {
  kind: string;
  message: string;
  command?: string;
  exitCode?: number;
  logExcerpt?: string;
}

export interface RuntimeMetric {
  type: "runtime";
  target: string;
  scenario: string;
  metric: string;
  valuesMs: number[];
  meanMs: number;
  minMs: number;
  maxMs: number;
  status: ResultStatus;
  error?: BenchmarkError;
}

export interface BuildMetric {
  type: "build";
  target: string;
  prod_build_ms?: number;
  prod_start_ms?: number;
  bundle_bytes?: number;
  bundle_gzip_bytes?: number;
  bundle_brotli_bytes?: number;
  files?: string[];
  status: ResultStatus;
  error?: BenchmarkError;
}

export interface DevMetric {
  type: "dev";
  target: string;
  dev_start_ms?: number;
  hmr_update_ms?: number;
  status: ResultStatus;
  error?: BenchmarkError;
}

export type BenchmarkResult = RuntimeMetric | BuildMetric | DevMetric;

export interface ReportPayload {
  measuredAt: string;
  suite: SuiteConfig;
  targets: Array<Pick<TargetConfig, "id" | "label" | "mode">>;
  scenarios: ScenarioConfig[];
  results: BenchmarkResult[];
  environment: {
    node: string;
    platform: string;
    arch: string;
  };
}
