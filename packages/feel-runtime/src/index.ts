export * from './types.js';
export { FeelEngine, type FeelEngineOptions, type WorkerLike } from './engine.js';
/**
 * URL of the `feel.wasm` shipped in this package, for Node (`readFile(feelWasmUrl())`) and for
 * browsers that load the package without a bundler. With Vite or another bundler import the
 * asset instead: `import wasmUrl from '@rheon/feel-runtime/feel.wasm?url'`.
 */
export const feelWasmUrl = (): URL => new URL('../feel.wasm', import.meta.url);

export { instantiateFeel, RawFeelInstance, FeelTrap, trapMessage, type FeelEnv, type FeelInstance } from './instance.js';
export { createWasiShim, WasiExit, type WasiShim, type WasiShimOptions } from './wasi.js';
export { rawJson, isRawJson, localTzOffsetMinutes, serializeRequest, buildEvaluateJson, buildParseJson } from './request.js';
export type { ToWorker, FromWorker } from './protocol.js';
