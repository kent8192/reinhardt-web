import os from "node:os";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { collectBundleMetrics } from "./bundles.js";
import { logExcerpt, runShellCommand } from "./commands.js";
import { installSourceSignalCleanup, patchSource, restoreSource, snapshotSource, type SourceSnapshot } from "./dev-loop.js";
import { loadManifest } from "./manifest.js";
import { writeReports } from "./report.js";
import { measureDevUpdate, runRuntimeMeasurements } from "./runtime.js";
import { startServer, stopServer, type ManagedServer } from "./servers.js";
import type { BenchmarkManifest, BenchmarkResult, TargetConfig } from "./types.js";

const frontendRoot = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const command = process.argv[2] ?? "check";
const buildArtifactPaths = ["dist", ".next", ".nuxt", ".output", "pkg", "target"];

async function main(): Promise<void> {
  const manifest = loadManifest(frontendRoot);
  if (command === "check") {
    console.log(`frontend-benchmark: checked ${manifest.scenarios.length} scenarios across ${manifest.targets.length} targets`);
    return;
  }
  if (command === "runtime") {
    await writeRun(manifest, await runRuntime(manifest));
    return;
  }
  if (command === "build") {
    await writeRun(manifest, await runBuild(manifest));
    return;
  }
  if (command === "measure") {
    const buildResults = await runBuild(manifest);
    const runtimeResults = await runRuntime(manifest);
    await writeRun(manifest, [...buildResults, ...runtimeResults]);
    return;
  }
  throw new Error(`unknown command: ${command}`);
}

async function runRuntime(manifest: BenchmarkManifest): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = [];
  for (const target of manifest.targets) {
    let server: ManagedServer | undefined;
    try {
      await installAndBuild(target, manifest.suite.timeout_ms);
      server = await startServer(target.preview, target.root, target.url, manifest.suite.timeout_ms);
      results.push(...await runRuntimeMeasurements(manifest, target));
    } catch (error) {
      results.push(failedRuntime(target, error));
    } finally {
      if (server) {
        await safeStopServer(server, `${target.id}: runtime stop`);
      }
    }
  }
  return results;
}

async function runBuild(manifest: BenchmarkManifest): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = [];
  for (const target of manifest.targets) {
    let prodServer: ManagedServer | undefined;
    try {
      debug(`${target.id}: production build start`);
      await installTarget(target, manifest.suite.timeout_ms);
      cleanBuildArtifacts(target);
      const buildResult = await buildTarget(target, manifest.suite.timeout_ms);
      const bundle = collectBundleMetrics(target);
      debug(`${target.id}: production preview start`);
      prodServer = await startServer(target.preview, target.root, target.url, manifest.suite.timeout_ms);
      results.push({
        type: "build",
        target: target.id,
        prod_build_ms: buildResult.durationMs,
        prod_start_ms: prodServer.startMs,
        ...bundle,
        status: "ok"
      });
      debug(`${target.id}: production build recorded`);
    } catch (error) {
      debug(`${target.id}: production build failed`);
      results.push({
        type: "build",
        target: target.id,
        status: "failed",
        error: errorToBenchmarkError(error)
      });
    } finally {
      if (prodServer) {
        debug(`${target.id}: production preview stop`);
        await safeStopServer(prodServer, `${target.id}: production preview stop`);
      }
    }

    let devServer: ManagedServer | undefined;
    let sourceSnapshot: SourceSnapshot | undefined;
    let removeSourceSignalCleanup: (() => void) | undefined;
    try {
      debug(`${target.id}: dev build artifacts clean`);
      cleanBuildArtifacts(target);
      debug(`${target.id}: dev source snapshot`);
      sourceSnapshot = snapshotSource(target.root, target.source_patch_file);
      removeSourceSignalCleanup = installSourceSignalCleanup(sourceSnapshot);
      debug(`${target.id}: dev server start`);
      devServer = await startServer(target.dev, target.root, target.dev_url, manifest.suite.timeout_ms);
      debug(`${target.id}: dev update measure`);
      const hmrMs = await measureDevUpdate(
        target,
        () => patchSource(target.root, target.source_patch_file),
        manifest.suite.timeout_ms
      );
      results.push({
        type: "dev",
        target: target.id,
        dev_start_ms: devServer.startMs,
        hmr_update_ms: hmrMs,
        status: "ok"
      });
      debug(`${target.id}: dev metric recorded`);
    } catch (error) {
      debug(`${target.id}: dev metric failed`);
      results.push({
        type: "dev",
        target: target.id,
        status: "failed",
        error: errorToBenchmarkError(error)
      });
    } finally {
      if (devServer) {
        debug(`${target.id}: dev server stop`);
        await safeStopServer(devServer, `${target.id}: dev server stop`);
      }
      if (sourceSnapshot) {
        safeRestoreSource(sourceSnapshot, `${target.id}: source restore`);
      }
      if (removeSourceSignalCleanup) {
        removeSourceSignalCleanup();
      }
    }
  }
  return results;
}

async function installAndBuild(target: TargetConfig, timeoutMs: number) {
  await installTarget(target, timeoutMs);
  return buildTarget(target, timeoutMs);
}

async function installTarget(target: TargetConfig, timeoutMs: number): Promise<void> {
  await runRequired(target.install, target.root, timeoutMs);
}

async function buildTarget(target: TargetConfig, timeoutMs: number) {
  return runRequired(target.build, target.root, timeoutMs);
}

function cleanBuildArtifacts(target: TargetConfig): void {
  for (const artifact of buildArtifactPaths) {
    fs.rmSync(path.join(target.root, artifact), { recursive: true, force: true });
  }
}

async function runRequired(commandText: string, cwd: string, timeoutMs: number) {
  debug(`command start: ${commandText}`);
  const result = await runShellCommand(commandText, cwd, timeoutMs);
  if (result.exitCode !== 0) {
    const error = new Error(`command failed: ${commandText}`);
    Object.assign(error, { commandResult: result });
    throw error;
  }
  debug(`command ok: ${commandText}`);
  return result;
}

async function writeRun(manifest: BenchmarkManifest, results: BenchmarkResult[]): Promise<void> {
  const output = writeReports(path.join(frontendRoot, "results"), {
    measuredAt: new Date().toISOString(),
    suite: manifest.suite,
    targets: manifest.targets.map((target) => ({
      id: target.id,
      label: target.label,
      mode: target.mode
    })),
    scenarios: manifest.scenarios,
    results,
    environment: {
      node: process.version,
      platform: os.platform(),
      arch: os.arch()
    }
  });
  console.log(`frontend-benchmark: wrote ${path.relative(frontendRoot, output.jsonPath)}`);
  console.log(`frontend-benchmark: wrote ${path.relative(frontendRoot, output.markdownPath)}`);
}

function failedRuntime(target: TargetConfig, error: unknown): BenchmarkResult {
  return {
    type: "runtime",
    target: target.id,
    scenario: "suite",
    metric: "runtime",
    status: "failed",
    error: errorToBenchmarkError(error)
  };
}

function errorToBenchmarkError(error: unknown) {
  if (error instanceof Error) {
    const commandResult = (error as Error & { commandResult?: { command: string; exitCode: number; stdout: string; stderr: string } }).commandResult;
    return {
      kind: "error",
      message: error.message,
      command: commandResult?.command,
      exitCode: commandResult?.exitCode,
      logExcerpt: commandResult ? logExcerpt(commandResult.stdout, commandResult.stderr) : undefined
    };
  }
  return {
    kind: "error",
    message: String(error)
  };
}

function debug(message: string): void {
  if (process.env.FRONTEND_BENCHMARK_DEBUG === "1") {
    console.error(`frontend-benchmark: ${message}`);
  }
}

async function safeStopServer(server: ManagedServer, label: string): Promise<void> {
  try {
    await stopServer(server);
  } catch (error) {
    debug(`${label}: ${errorToBenchmarkError(error).message}`);
  }
}

function safeRestoreSource(snapshot: SourceSnapshot, label: string): void {
  try {
    restoreSource(snapshot);
  } catch (error) {
    debug(`${label}: ${errorToBenchmarkError(error).message}`);
  }
}

main().catch((error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`frontend-benchmark: ${message}`);
  process.exitCode = 1;
});
