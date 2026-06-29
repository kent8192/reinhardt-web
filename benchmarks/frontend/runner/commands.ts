import { spawn } from "node:child_process";
import { performance } from "node:perf_hooks";
import { terminateProcessTree } from "./process-tree.js";
import type { CommandResult } from "./types.js";

export function runShellCommand(command: string, cwd: string, timeoutMs: number): Promise<CommandResult> {
  const start = performance.now();
  return new Promise((resolve) => {
    const child = spawn(command, {
      cwd,
      detached: true,
      shell: true,
      env: { ...process.env, CI: "1", NEXT_TELEMETRY_DISABLED: "1", NUXT_TELEMETRY_DISABLED: "1" },
      stdio: ["ignore", "pipe", "pipe"]
    });
    let stdout = "";
    let stderr = "";
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      terminateProcessTree(child, "SIGTERM");
    }, timeoutMs);
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({
        command,
        cwd,
        exitCode: code ?? 1,
        stdout,
        stderr,
        durationMs: performance.now() - start,
        timedOut
      });
    });
  });
}

export function logExcerpt(stdout: string, stderr: string): string {
  const combined = `${stdout}\n${stderr}`.trim();
  return combined.length <= 4000 ? combined : combined.slice(combined.length - 4000);
}
