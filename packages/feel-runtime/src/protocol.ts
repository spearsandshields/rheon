/** Messages between `FeelEngine` (main thread) and `worker.ts`. */

export type ToWorker =
  | { type: 'init'; module: WebAssembly.Module }
  | { type: 'call'; id: number; json: string };

export type FromWorker =
  | { type: 'ready' }
  | { type: 'initError'; message: string }
  /** `memoryBytes`: the instance's linear memory size after the call (lets the engine recycle bloated workers). */
  | { type: 'result'; id: number; json: string; memoryBytes?: number }
  | { type: 'crash'; id: number; message: string };
