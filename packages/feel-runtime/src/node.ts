/**
 * `FeelEngine` in Node: a worker of `node:worker_threads` in the place of a Web Worker.
 *
 * ```js
 * import { readFile } from 'node:fs/promises';
 * import { FeelEngine, feelWasmUrl } from '@rheon/feel-runtime';
 * import { createNodeWorker } from '@rheon/feel-runtime/node';
 *
 * const engine = await FeelEngine.create({ wasmBytes: await readFile(feelWasmUrl()), createWorker: createNodeWorker });
 * ```
 *
 * The engine is the one of the browser: the queue, the timeout that ends a worker and starts a
 * fresh one, the recovery after a trap. This module is for Node only; a bundle for a browser
 * must not import it.
 */
import { Worker } from 'node:worker_threads';
import type { ResourceLimits } from 'node:worker_threads';
import type { WorkerLike } from './engine.js';
import type { FromWorker, ToWorker } from './protocol.js';

/** A `WorkerLike` that is a worker of `node:worker_threads`, with that worker. */
export interface NodeWorkerLike extends WorkerLike {
  /** The worker thread itself. */
  readonly worker: Worker;
}

export interface NodeWorkerOptions {
  /**
   * The resource limits of the worker thread. Keep `stackSizeMb` at Node's default of 4 or
   * more: the engine is measured with that stack, and a smaller one lets a nested recursion
   * trap earlier.
   */
  resourceLimits?: ResourceLimits;
}

/** A function for `FeelEngineOptions.createWorker` whose workers have `options`. */
export function nodeWorkers(options: NodeWorkerOptions = {}): () => NodeWorkerLike {
  return () => createNodeWorker(options);
}

/** Starts one worker thread that runs the engine's worker script; pass it as `createWorker`. */
export function createNodeWorker(options: NodeWorkerOptions = {}): NodeWorkerLike {
  const worker = new Worker(new URL('./node-worker.js', import.meta.url), { resourceLimits: options.resourceLimits });
  const like: NodeWorkerLike = {
    onmessage: null,
    onerror: null,
    postMessage: (message: ToWorker) => worker.postMessage(message),
    terminate: () => void worker.terminate(),
    worker,
  };
  worker.on('message', (data) => like.onmessage?.({ data: data as FromWorker }));
  worker.on('error', (error) => like.onerror?.({ error, message: error.message }));
  return like;
}
