// Runs dist/worker.js (written for Web Workers) inside node:worker_threads by providing the
// `self.postMessage` / `self.onmessage` surface it uses.
import { parentPort } from 'node:worker_threads';

let handler = null;
globalThis.postMessage = (m) => parentPort.postMessage(m);
Object.defineProperty(globalThis, 'onmessage', {
  configurable: true,
  get: () => handler,
  set: (f) => {
    handler = f;
  },
});
parentPort.on('message', (data) => handler?.({ data }));
await import('../dist/worker.js');
