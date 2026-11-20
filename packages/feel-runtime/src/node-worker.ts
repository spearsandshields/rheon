/**
 * The script of a worker thread of Node (`createNodeWorker`): it gives the engine's worker
 * script, which is written for a Web Worker, the `postMessage` and `onmessage` it uses, and
 * runs it.
 */
import { parentPort } from 'node:worker_threads';

if (parentPort === null) {
  throw new Error('@rheon/feel-runtime/node-worker runs in a worker thread only');
}
const port = parentPort;
let handler: ((event: { data: unknown }) => void) | null = null;
const scope = globalThis as unknown as { postMessage: (message: unknown) => void };
scope.postMessage = (message) => port.postMessage(message);
Object.defineProperty(globalThis, 'onmessage', {
  configurable: true,
  get: () => handler,
  set: (f: typeof handler) => {
    handler = f;
  },
});
port.on('message', (data) => handler?.({ data }));
await import('./worker.js');
