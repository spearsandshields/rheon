// The raw ABI of feel.wasm (docs/CONTRACT.md §1) for the Node scripts: instantiation under
// node:wasi and one request through feel_alloc / feel_call / feel_free.
import { WASI } from 'node:wasi';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

// Instantiates a compiled module and calls its `_initialize` (the module is a reactor); returns
// the exports.
export async function instantiate(module) {
  const wasi = new WASI({ version: 'preview1', args: [], env: {} });
  const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
  wasi.initialize(instance);
  return instance.exports;
}

// Sends one JSON request text and returns the parsed response. A trap throws; the instance is
// then dead.
export function call(exports, json) {
  const bytes = encoder.encode(json);
  const ptr = exports.feel_alloc(bytes.length);
  new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
  // the i64 arrives signed: pointers >= 2^31 would turn negative
  const packed = BigInt.asUintN(64, exports.feel_call(ptr, bytes.length));
  const responsePtr = Number(packed >> 32n);
  const responseLen = Number(packed & 0xffffffffn);
  const text = decoder.decode(new Uint8Array(exports.memory.buffer, responsePtr, responseLen));
  exports.feel_free(responsePtr, responseLen);
  return JSON.parse(text);
}
