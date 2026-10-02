import assert from "node:assert/strict";
import { EventEmitter } from "node:events";
import test from "node:test";
import { SignalCleanupRegistry, type CleanupSignal } from "../signal-cleanup.js";

type SignalTarget = {
  once(signal: CleanupSignal, listener: () => void): unknown;
};

test("signal cleanup waits for async handlers before exiting", async () => {
  const emitter = new EventEmitter();
  const exits: number[] = [];
  const events: string[] = [];
  const registry = new SignalCleanupRegistry(emitter as SignalTarget, (code) => {
    exits.push(code);
  });

  let finishAsyncCleanup: (() => void) | undefined;
  registry.register(() => {
    events.push("source-restored");
  });
  registry.register(
    () =>
      new Promise<void>((resolve) => {
        finishAsyncCleanup = () => {
          events.push("server-stopped");
          resolve();
        };
      })
  );

  emitter.emit("SIGINT");

  assert.deepEqual(events, ["source-restored"]);
  assert.deepEqual(exits, []);

  finishAsyncCleanup?.();
  await new Promise((resolve) => setImmediate(resolve));

  assert.deepEqual(events, ["source-restored", "server-stopped"]);
  assert.deepEqual(exits, [130]);
});

test("signal cleanup skips unregistered handlers", async () => {
  const emitter = new EventEmitter();
  const exits: number[] = [];
  const events: string[] = [];
  const registry = new SignalCleanupRegistry(emitter as SignalTarget, (code) => {
    exits.push(code);
  });
  const unregister = registry.register(() => {
    events.push("removed");
  });
  registry.register(() => {
    events.push("kept");
  });

  unregister();
  emitter.emit("SIGTERM");
  await new Promise((resolve) => setImmediate(resolve));

  assert.deepEqual(events, ["kept"]);
  assert.deepEqual(exits, [143]);
});
