import { readFile } from 'node:fs/promises';
import { Worker } from 'node:worker_threads';
import { fileURLToPath } from 'node:url';

export const WASM_PATH = fileURLToPath(new URL('../../../dist/feel.wasm', import.meta.url));
export const wasmBytes = () => readFile(WASM_PATH);

/** Wraps a node:worker_threads Worker running dist/worker.js as a `WorkerLike`. */
export function createNodeWorker() {
  const w = new Worker(new URL('./worker-bootstrap.mjs', import.meta.url));
  const like = {
    onmessage: null,
    onerror: null,
    postMessage: (m) => w.postMessage(m),
    terminate: () => void w.terminate(),
    raw: w,
  };
  w.on('message', (data) => like.onmessage?.({ data }));
  w.on('error', (error) => like.onerror?.({ error, message: error.message }));
  return like;
}

/**
 * A minimal wasm module with the feel ABI (incl. the reactor's `_initialize`) whose feel_call
 * executes `unreachable` (simulates a Rust panic=abort trap). With `{ initialize: false }` the
 * `_initialize` export is left out (a mis-linked module).
 */
export function trappingModuleBytes({ initialize = true } = {}) {
  const str = (s) => [s.length, ...new TextEncoder().encode(s)];
  const section = (id, body) => [id, body.length, ...body];
  const bytes = [
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...section(1, [4, 0x60, 1, 0x7f, 1, 0x7f, 0x60, 2, 0x7f, 0x7f, 0, 0x60, 2, 0x7f, 0x7f, 1, 0x7e, 0x60, 0, 0]),
    ...section(3, [4, 0, 1, 2, 3]),
    ...section(5, [1, 0x00, 1]),
    ...section(7, [
      initialize ? 5 : 4,
      ...str('memory'), 2, 0,
      ...str('feel_alloc'), 0, 0,
      ...str('feel_free'), 0, 1,
      ...str('feel_call'), 0, 2,
      ...(initialize ? [...str('_initialize'), 0, 3] : []),
    ]),
    ...section(10, [4, 4, 0, 0x41, 0x00, 0x0b, 2, 0, 0x0b, 3, 0, 0x00, 0x0b, 2, 0, 0x0b]),
  ];
  return new Uint8Array(bytes);
}
