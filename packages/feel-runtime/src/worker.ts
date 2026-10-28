/**
 * Module Web Worker hosting one feel.wasm instance (spawned by `FeelEngine`).
 * Protocol: see protocol.ts. The engine posts the compiled `WebAssembly.Module` once (`init`),
 * then one `call` at a time. On a trap the worker reports `crash`; the engine then terminates
 * it and starts a fresh worker with the same module.
 */
import { FeelTrap, RawFeelInstance } from './instance.js';
import type { FromWorker, ToWorker } from './protocol.js';

interface WorkerScope {
  postMessage(msg: FromWorker): void;
  onmessage: ((ev: { data: ToWorker }) => void) | null;
}

const scope = globalThis as unknown as WorkerScope;
let instance: RawFeelInstance | null = null;

const post = (msg: FromWorker) => scope.postMessage(msg);

scope.onmessage = (ev) => {
  const msg = ev.data;
  if (msg.type === 'init') {
    RawFeelInstance.create(msg.module).then(
      (inst) => {
        instance = inst;
        post({ type: 'ready' });
      },
      (e) => post({ type: 'initError', message: e instanceof Error ? `${e.name}: ${e.message}` : String(e) }),
    );
    return;
  }
  if (msg.type === 'call') {
    if (!instance) {
      post({ type: 'crash', id: msg.id, message: 'worker received a call before init' });
      return;
    }
    try {
      const json = instance.callJson(msg.json);
      post({ type: 'result', id: msg.id, json, memoryBytes: instance.memoryBytes });
    } catch (e) {
      const message = e instanceof FeelTrap ? e.message : e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      post({ type: 'crash', id: msg.id, message });
    }
  }
};
