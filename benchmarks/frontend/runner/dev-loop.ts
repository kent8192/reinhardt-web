import fs from "node:fs";
import path from "node:path";
import { registerSignalCleanup } from "./signal-cleanup.js";

export interface SourceSnapshot {
  file: string;
  contents: string;
}

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
  return registerSignalCleanup(cleanup);
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
