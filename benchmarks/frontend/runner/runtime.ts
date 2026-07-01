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

async function measureOnce(
  target: TargetConfig,
  scenario: string,
  timeoutMs: number
): Promise<{ metric: string; valueMs: number }> {
  const browser = await chromium.launch();
  const context = await browser.newContext();
  const page = await context.newPage();
  page.setDefaultTimeout(timeoutMs);
  try {
    await page.goto(target.url, { timeout: timeoutMs });
    const navigationStart = await page.evaluate(() => performance.timeOrigin);
    await page.locator("[data-benchmark-ready='true']").waitFor({ timeout: timeoutMs });
    const hydrated = page.locator("[data-benchmark-hydrated='true']");
    if (scenario === "hydration") {
      const metric = target.mode === "ssr" ? "hydration_ready_ms" : "boot_ready_ms";
      if (target.mode === "ssr") {
        await hydrated.waitFor({ timeout: timeoutMs });
      }
      const valueMs = await page.evaluate((start) => Date.now() - start, navigationStart);
      return { metric, valueMs };
    }
    if (target.mode === "ssr") {
      await hydrated.waitFor({ timeout: timeoutMs });
    }
    if (scenario === "counter") {
      return { metric: "click_update_ms", valueMs: await measureClick(page, "counter-increment", "counter", /Counter: 1/, timeoutMs) };
    }
    if (scenario === "form-input") {
      return { metric: "input_update_ms", valueMs: await measureInput(page, timeoutMs) };
    }
    if (scenario === "router") {
      return { metric: "navigation_ms", valueMs: await measureRouteNavigation(page, timeoutMs) };
    }
    if (scenario === "keyed-list") {
      return { metric: "list_update_ms", valueMs: await measureKeyedListUpdate(page, timeoutMs) };
    }
    throw new Error(`unsupported scenario: ${scenario}`);
  } finally {
    await context.close();
    await browser.close();
  }
}

async function measureClick(page: Page, action: string, value: string, expected: RegExp, timeoutMs: number): Promise<number> {
  const start = await page.evaluate(() => performance.now());
  await page.locator(`[data-benchmark-action='${action}']`).click();
  await page.locator(`[data-benchmark-value='${value}']`).filter({ hasText: expected }).waitFor({ timeout: timeoutMs });
  const end = await page.evaluate(() => performance.now());
  return end - start;
}

async function measureInput(page: Page, timeoutMs: number): Promise<number> {
  const start = await page.evaluate(() => performance.now());
  await page.locator("[data-benchmark-action='input']").fill("benchmark input");
  await page.locator("[data-benchmark-value='input']").filter({ hasText: /benchmark input/ }).waitFor({ timeout: timeoutMs });
  const end = await page.evaluate(() => performance.now());
  return end - start;
}

async function measureRouteNavigation(page: Page, timeoutMs: number): Promise<number> {
  const start = await page.evaluate(() => performance.now());
  await page.locator("[data-benchmark-action='route-detail']").click();
  await page.waitForURL("**/detail", { timeout: timeoutMs });
  await page.locator("[data-benchmark-value='route']").filter({ hasText: /Route: detail/ }).waitFor({ timeout: timeoutMs });
  const end = await page.evaluate(() => performance.now());
  return end - start;
}

async function measureKeyedListUpdate(page: Page, timeoutMs: number): Promise<number> {
  const start = await page.evaluate(() => performance.now());
  await page.locator("[data-benchmark-action='list-reorder']").click();
  await page.locator("[data-benchmark-row='1000']").waitFor({ timeout: timeoutMs });
  await page.locator("[data-benchmark-value='list-first']").filter({ hasText: /First: Row 1000/ }).waitFor({ timeout: timeoutMs });
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
  page.setDefaultTimeout(timeoutMs);
  try {
    await page.goto(url, { timeout: timeoutMs });
    await page.locator("[data-benchmark-ready='true']").waitFor({ timeout: timeoutMs });
    const version = page.locator("[data-benchmark-value='version']");
    await version.filter({ hasText: "baseline-version" }).waitFor({ timeout: timeoutMs });
    const start = performance.now();
    const expectedVersion = await patch();
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
      const sample = await measureOnce(target, scenario.id, manifest.suite.timeout_ms);
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
