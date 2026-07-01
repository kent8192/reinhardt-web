import { spawn } from "node:child_process";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));
const sourceFile = path.join(root, "src", "client.rs");
const host = readArg("--host", "127.0.0.1");
const port = readArg("--port", "4410");

let activeBuild;
let vite;
let buildPending = false;
let buildRunning = false;
let shuttingDown = false;

await buildWasm();
startVite();
fs.watchFile(sourceFile, { interval: 250 }, () => {
  scheduleBuild();
});

process.on("SIGINT", shutdown);
process.on("SIGTERM", shutdown);

function readArg(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 ? process.argv[index + 1] ?? fallback : fallback;
}

function buildWasm() {
  return new Promise((resolve, reject) => {
    activeBuild = spawn("wasm-pack", ["build", "--target", "web", "--out-dir", "pkg"], {
      cwd: root,
      detached: true,
      env: { ...process.env, CARGO_BUILD_JOBS: "1" },
      stdio: "inherit"
    });
    activeBuild.once("error", reject);
    activeBuild.once("close", (code) => {
      activeBuild = undefined;
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`wasm-pack build failed with exit code ${code ?? 1}`));
      }
    });
  });
}

function startVite() {
  vite = spawn(
    process.execPath,
    [path.join(root, "node_modules", "vite", "bin", "vite.js"), "--host", host, "--port", port],
    {
      cwd: root,
      detached: true,
      env: process.env,
      stdio: "inherit"
    }
  );
  vite.once("exit", (code) => {
    if (!shuttingDown) {
      process.exit(code ?? 1);
    }
  });
}

function scheduleBuild() {
  buildPending = true;
  if (!buildRunning) {
    void runPendingBuilds();
  }
}

async function runPendingBuilds() {
  buildRunning = true;
  while (buildPending && !shuttingDown) {
    buildPending = false;
    try {
      await buildWasm();
    } catch (error) {
      console.error(error instanceof Error ? error.message : String(error));
    }
  }
  buildRunning = false;
}

function shutdown() {
  if (shuttingDown) {
    return;
  }
  shuttingDown = true;
  fs.unwatchFile(sourceFile);
  void Promise.allSettled([stopChild(activeBuild), stopChild(vite)]).then(() => {
    process.exit(0);
  });
  setTimeout(() => process.exit(0), 5000).unref();
}

function stopChild(child) {
  if (!child) {
    return Promise.resolve();
  }
  return new Promise((resolve) => {
    const finish = () => resolve();
    if (child.exitCode !== null || child.signalCode !== null) {
      finish();
      return;
    }
    child.once("close", finish);
    terminateProcessGroup(child, "SIGTERM");
    setTimeout(() => {
      if (child.exitCode === null && child.signalCode === null) {
        terminateProcessGroup(child, "SIGKILL");
      }
    }, 2000).unref();
  });
}

function terminateProcessGroup(child, signal) {
  if (!child.pid) {
    return;
  }
  try {
    process.kill(-child.pid, signal);
  } catch {
    child.kill(signal);
  }
}
