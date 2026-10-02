import { spawn } from "node:child_process";
import { performance } from "node:perf_hooks";
import { terminateProcessTree } from "./process-tree.js";
import { registerSignalCleanup } from "./signal-cleanup.js";
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
    let resolved = false;
    let killTimer: NodeJS.Timeout | undefined;
    let forceTimer: NodeJS.Timeout | undefined;
    let removeSignalCleanup: (() => void) | undefined;
    const timer = setTimeout(() => {
      timedOut = true;
      terminateProcessTree(child, "SIGTERM");
      killTimer = setTimeout(() => {
        terminateProcessTree(child, "SIGKILL");
      }, 2_000);
      forceTimer = setTimeout(() => {
        finish(124);
      }, 2_500);
    }, timeoutMs);
    function childIsClosed(): boolean {
      return child.exitCode !== null || child.signalCode !== null;
    }
    function waitForClose(timeoutMs: number): Promise<void> {
      return new Promise((resolve) => {
        const timeout = setTimeout(resolve, timeoutMs);
        child.once("close", () => {
          clearTimeout(timeout);
          resolve();
        });
      });
    }
    removeSignalCleanup = registerSignalCleanup(async () => {
      if (!child.pid || childIsClosed()) {
        return;
      }
      terminateProcessTree(child, "SIGTERM");
      await waitForClose(2_000);
      if (!childIsClosed()) {
        terminateProcessTree(child, "SIGKILL");
      }
    });
    function finish(code: number): void {
      if (resolved) {
        return;
      }
      resolved = true;
      removeSignalCleanup?.();
      clearTimeout(timer);
      if (killTimer) {
        clearTimeout(killTimer);
      }
      if (forceTimer) {
        clearTimeout(forceTimer);
      }
      child.stdout?.destroy();
      child.stderr?.destroy();
      resolve({
        command,
        cwd,
        exitCode: code,
        stdout,
        stderr,
        durationMs: performance.now() - start,
        timedOut
      });
    }
    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", (error) => {
      stderr += `\n${error.message}`;
      finish(1);
    });
    child.on("close", (code) => {
      finish(code ?? (timedOut ? 124 : 1));
    });
  });
}

export function logExcerpt(stdout: string, stderr: string): string {
  const combined = `${stdout}\n${stderr}`.trim();
  return combined.length <= 4000 ? combined : combined.slice(combined.length - 4000);
}
