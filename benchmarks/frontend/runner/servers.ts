import { spawn, type ChildProcess } from "node:child_process";
import { performance } from "node:perf_hooks";
import { logExcerpt } from "./commands.js";
import { terminateProcessTree } from "./process-tree.js";

export interface ManagedServer {
  command: string;
  cwd: string;
  process: ChildProcess;
  stdout: string;
  stderr: string;
  startMs: number;
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function startServer(command: string, cwd: string, url: string, timeoutMs: number): Promise<ManagedServer> {
  const started = performance.now();
  const child = spawn(command, {
    cwd,
    detached: true,
    shell: true,
    env: { ...process.env, CI: "1", NEXT_TELEMETRY_DISABLED: "1", NUXT_TELEMETRY_DISABLED: "1" },
    stdio: ["ignore", "pipe", "pipe"]
  });
  const server: ManagedServer = { command, cwd, process: child, stdout: "", stderr: "", startMs: 0 };
  child.stdout.on("data", (chunk) => {
    server.stdout += chunk.toString();
  });
  child.stderr.on("data", (chunk) => {
    server.stderr += chunk.toString();
  });

  while (performance.now() - started < timeoutMs) {
    if (child.exitCode !== null) {
      throw new Error(`server exited before readiness: ${logExcerpt(server.stdout, server.stderr)}`);
    }
    try {
      const response = await fetch(url);
      if (response.ok) {
        server.startMs = performance.now() - started;
        return server;
      }
    } catch {
      await delay(100);
    }
  }
  await stopServer(server);
  throw new Error(`server did not become ready: ${url}`);
}

export async function stopServer(server: ManagedServer): Promise<void> {
  const child = server.process;
  if (!child.pid) {
    child.stdout?.destroy();
    child.stderr?.destroy();
    return;
  }

  const alreadyClosed = child.exitCode !== null || child.signalCode !== null;
  const closed = alreadyClosed
    ? Promise.resolve()
    : new Promise<void>((resolve) => {
        child.once("close", () => resolve());
      });
  terminateProcessTree(child, "SIGTERM");
  const closedGracefully = await Promise.race([
    closed.then(() => true),
    delay(2_000).then(() => false)
  ]);
  if (!closedGracefully) {
    terminateProcessTree(child, "SIGKILL");
    await Promise.race([closed, delay(500)]);
  }
  child.stdout?.destroy();
  child.stderr?.destroy();
  child.unref();
}
