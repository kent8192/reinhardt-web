import fs from "node:fs";
import path from "node:path";

export interface SourceSnapshot {
  file: string;
  contents: string;
}

type CleanupHandler = () => void;
type CleanupSignal = "SIGINT" | "SIGTERM";

let activeSignalCleanup: CleanupHandler | undefined;
let signalHandlersInstalled = false;

export function snapshotSource(root: string, sourcePatchFile: string): SourceSnapshot {
  const file = path.join(root, sourcePatchFile);
  return {
    file,
    contents: fs.readFileSync(file, "utf8")
  };
}

export function restoreSource(snapshot: SourceSnapshot): void {
  fs.writeFileSync(snapshot.file, snapshot.contents);
}

export function installSourceSignalCleanup(snapshot: SourceSnapshot): () => void {
  const cleanup = () => restoreSource(snapshot);
  activeSignalCleanup = cleanup;
  installSignalHandlers();

  return () => {
    if (activeSignalCleanup === cleanup) {
      activeSignalCleanup = undefined;
    }
  };
}

export function patchSource(root: string, sourcePatchFile: string): string {
  const file = path.join(root, sourcePatchFile);
  const original = fs.readFileSync(file, "utf8");
  const marker = `benchmark-patch-${Date.now()}`;
  if (!original.includes("baseline-version")) {
    throw new Error(`source patch marker not found: ${sourcePatchFile}`);
  }
  fs.writeFileSync(file, original.replace("baseline-version", marker));
  return marker;
}

function installSignalHandlers(): void {
  if (signalHandlersInstalled) {
    return;
  }
  signalHandlersInstalled = true;
  process.on("SIGINT", () => restoreAndExit("SIGINT"));
  process.on("SIGTERM", () => restoreAndExit("SIGTERM"));
}

function restoreAndExit(signal: CleanupSignal): never {
  const cleanup = activeSignalCleanup;
  activeSignalCleanup = undefined;
  if (cleanup) {
    cleanup();
  }
  process.exit(signal === "SIGINT" ? 130 : 143);
}
