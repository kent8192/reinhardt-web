import { spawn, type ChildProcess } from "node:child_process";
import net from "node:net";
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

const activeServers = new Set<ManagedServer>();
const cleanupSignals: NodeJS.Signals[] = ["SIGINT", "SIGTERM"];
let signalCleanupInstalled = false;
let handlingCleanupSignal = false;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function startServer(command: string, cwd: string, url: string, timeoutMs: number): Promise<ManagedServer> {
  await assertPortAvailable(url);
  const started = performance.now();
  const child = spawn(command, {
    cwd,
    detached: true,
    shell: true,
    env: { ...process.env, CI: "1", NEXT_TELEMETRY_DISABLED: "1", NUXT_TELEMETRY_DISABLED: "1" },
    stdio: ["ignore", "pipe", "pipe"]
  });
  let spawnError: Error | undefined;
  child.once("error", (error) => {
    spawnError = error;
  });
  const server: ManagedServer = { command, cwd, process: child, stdout: "", stderr: "", startMs: 0 };
  registerServer(server);
  child.stdout.on("data", (chunk) => {
    server.stdout += chunk.toString();
  });
  child.stderr.on("data", (chunk) => {
    server.stderr += chunk.toString();
  });

  while (performance.now() - started < timeoutMs) {
    if (spawnError) {
      await stopServer(server);
      throw new Error(`server failed to start: ${spawnError.message}`);
    }
    if (child.exitCode !== null) {
      throw new Error(`server exited before readiness: ${logExcerpt(server.stdout, server.stderr)}`);
    }
    try {
      const response = await fetch(url);
      if (response.ok) {
        server.startMs = performance.now() - started;
        return server;
      }
      await delay(100);
    } catch {
      await delay(100);
    }
  }
  await stopServer(server);
  throw new Error(`server did not become ready: ${url}`);
}

export async function stopServer(server: ManagedServer): Promise<void> {
  const child = server.process;
  try {
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
  } finally {
    activeServers.delete(server);
  }
}

function registerServer(server: ManagedServer): void {
  activeServers.add(server);
  installServerSignalCleanup();
}

function installServerSignalCleanup(): void {
  if (signalCleanupInstalled) {
    return;
  }
  signalCleanupInstalled = true;
  for (const signal of cleanupSignals) {
    process.once(signal, () => {
      void stopActiveServersForSignal(signal);
    });
  }
}

async function stopActiveServersForSignal(signal: NodeJS.Signals): Promise<void> {
  if (handlingCleanupSignal) {
    return;
  }
  handlingCleanupSignal = true;
  try {
    await Promise.allSettled([...activeServers].map((server) => stopServer(server)));
  } finally {
    process.exit(signal === "SIGINT" ? 130 : 143);
  }
}

async function assertPortAvailable(urlText: string): Promise<void> {
  const url = new URL(urlText);
  const port = Number(url.port || (url.protocol === "https:" ? 443 : 80));
  await new Promise<void>((resolve, reject) => {
    const socket = net.createConnection({ host: url.hostname, port });
    let settled = false;
    const finish = (error?: Error) => {
      if (settled) {
        return;
      }
      settled = true;
      socket.destroy();
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    };
    socket.setTimeout(500);
    socket.once("connect", () => {
      finish(new Error(`server port is already in use before startup: ${urlText}`));
    });
    socket.once("error", (error: NodeJS.ErrnoException) => {
      if (error.code === "ECONNREFUSED") {
        finish();
      } else {
        finish(error);
      }
    });
    socket.once("timeout", () => finish());
  });
}
