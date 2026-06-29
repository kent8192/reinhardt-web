import { chromium, type Browser, type Page } from "@playwright/test";
import { performance } from "node:perf_hooks";
import type { BenchmarkManifest, RuntimeMetric, TargetConfig } from "./types.js";

function summarize(valuesMs: number[]): Pick<RuntimeMetric, "valuesMs" | "meanMs" | "minMs" | "maxMs"> {
  const meanMs = valuesMs.reduce((sum, value) => sum + value, 0) / valuesMs.length;
  return {
    valuesMs,
    meanMs,
    minMs: Math.min(...valuesMs),
    maxMs: Math.max(...valuesMs)
  };
}

async function measureOnce(target: TargetConfig, scenario: string): Promise<{ metric: string; valueMs: number }> {
  const browser = await chromium.launch();
  const context = await browser.newContext();
  const page = await context.newPage();
  try {
    await page.goto(target.url);
    const navigationStart = await page.evaluate(() => performance.timeOrigin);
    await page.locator("[data-benchmark-ready='true']").waitFor();
    if (scenario === "hydration") {
      const metric = target.mode === "csr" ? "boot_ready_ms" : "hydration_ready_ms";
      if (target.mode !== "csr") {
        await page.locator("[data-benchmark-hydrated='true']").waitFor();
      }
      const valueMs = await page.evaluate((start) => Date.now() - start, navigationStart);
      return { metric, valueMs };
    }
    if (scenario === "counter") {
      return { metric: "click_update_ms", valueMs: await measureClick(page, "counter-increment", "counter", /Counter: 1/) };
    }
    if (scenario === "form-input") {
      return { metric: "input_update_ms", valueMs: await measureInput(page) };
    }
    if (scenario === "router") {
      return { metric: "navigation_ms", valueMs: await measureClick(page, "route-detail", "route", /Route: detail/) };
    }
    if (scenario === "keyed-list") {
      return { metric: "list_update_ms", valueMs: await measureClick(page, "list-append", "list-count", /Rows: 1001/) };
    }
    throw new Error(`unsupported scenario: ${scenario}`);
  } finally {
    await context.close();
    await browser.close();
  }
}

async function measureClick(page: Page, action: string, value: string, expected: RegExp): Promise<number> {
  const start = await page.evaluate(() => performance.now());
  await page.locator(`[data-benchmark-action='${action}']`).click();
  await page.locator(`[data-benchmark-value='${value}']`).filter({ hasText: expected }).waitFor();
  const end = await page.evaluate(() => performance.now());
  return end - start;
}

async function measureInput(page: Page): Promise<number> {
  const start = await page.evaluate(() => performance.now());
  await page.locator("[data-benchmark-action='input']").fill("benchmark input");
  await page.locator("[data-benchmark-value='input']").filter({ hasText: /benchmark input/ }).waitFor();
  const end = await page.evaluate(() => performance.now());
  return end - start;
}

export async function measureDevUpdate(
  url: string,
  patch: () => string | Promise<string>,
  timeoutMs: number
): Promise<number> {
  const browser: Browser = await chromium.launch();
  const context = await browser.newContext();
  const page = await context.newPage();
  try {
    await page.goto(url);
    await page.locator("[data-benchmark-ready='true']").waitFor();
    const version = page.locator("[data-benchmark-value='version']");
    await version.filter({ hasText: "baseline-version" }).waitFor({ timeout: timeoutMs });
    const start = performance.now();
    const expectedVersion = await patch();
    if (expectedVersion === "baseline-version") {
      return 0;
    }
    await version.filter({ hasText: expectedVersion }).waitFor({ timeout: timeoutMs });
    const end = performance.now();
    return end - start;
  } finally {
    await context.close();
    await browser.close();
  }
}

export async function runRuntimeMeasurements(manifest: BenchmarkManifest, target: TargetConfig): Promise<RuntimeMetric[]> {
  const metrics: RuntimeMetric[] = [];
  for (const scenario of manifest.scenarios) {
    const samples: number[] = [];
    for (let index = 0; index < manifest.suite.warmup_count + manifest.suite.sample_count; index += 1) {
      const sample = await measureOnce(target, scenario.id);
      if (index >= manifest.suite.warmup_count) {
        samples.push(sample.valueMs);
      }
      if (index === manifest.suite.warmup_count + manifest.suite.sample_count - 1) {
        metrics.push({
          type: "runtime",
          target: target.id,
          scenario: scenario.id,
          metric: sample.metric,
          ...summarize(samples),
          status: "ok"
        });
      }
    }
  }
  return metrics;
}
