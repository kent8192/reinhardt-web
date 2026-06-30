import fs from "node:fs";
import path from "node:path";
import { brotliCompressSync, gzipSync } from "node:zlib";
import type { TargetConfig } from "./types.js";

const countedExtensions = new Set([".js", ".mjs", ".css", ".wasm"]);

export interface BundleMetrics {
  bundle_bytes: number;
  bundle_gzip_bytes: number;
  bundle_brotli_bytes: number;
  files: string[];
}

function walk(dir: string): string[] {
  if (!fs.existsSync(dir)) {
    throw new Error(`configured bundle path does not exist: ${dir}`);
  }
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = path.join(dir, entry.name);
    return entry.isDirectory() ? walk(fullPath) : [fullPath];
  });
}

export function collectBundleMetrics(target: TargetConfig): BundleMetrics {
  const files = target.bundle_paths
    .flatMap((bundlePath) => walk(path.join(target.root, bundlePath)))
    .filter((file) => countedExtensions.has(path.extname(file)));
  if (files.length === 0) {
    throw new Error(`configured bundle paths contain no counted assets: ${target.bundle_paths.join(", ")}`);
  }

  let bundle_bytes = 0;
  let bundle_gzip_bytes = 0;
  let bundle_brotli_bytes = 0;
  for (const file of files) {
    const content = fs.readFileSync(file);
    bundle_bytes += content.byteLength;
    bundle_gzip_bytes += gzipSync(content).byteLength;
    bundle_brotli_bytes += brotliCompressSync(content).byteLength;
  }

  return {
    bundle_bytes,
    bundle_gzip_bytes,
    bundle_brotli_bytes,
    files: files.map((file) => path.relative(target.root, file)).sort()
  };
}
