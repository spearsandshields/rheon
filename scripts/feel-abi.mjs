// The raw ABI of feel.wasm (docs/CONTRACT.md §1) for the Node scripts: instantiation under
// node:wasi, one request through feel_alloc / feel_call / feel_free, and a session that answers
// requests one after another and replaces the instance after a trap.
import { WASI } from 'node:wasi';

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

// Instantiates a compiled module and calls its `_initialize` (the module is a reactor); returns
// the exports.
export async function instantiate(module) {
  const wasi = new WASI({ version: 'preview1', args: [], env: {} });
  const instance = await WebAssembly.instantiate(module, wasi.getImportObject());
  wasi.initialize(instance);
  return instance.exports;
}

// A response whose bytes are not UTF-8.
export class MalformedResponse extends Error {}

// Sends one JSON request text and returns a copy of the response bytes. A trap throws; the
// instance is then dead.
function exchange(exports, json) {
  const bytes = encoder.encode(json);
  const ptr = exports.feel_alloc(bytes.length);
  new Uint8Array(exports.memory.buffer, ptr, bytes.length).set(bytes);
  // the i64 arrives signed: pointers >= 2^31 would turn negative
  const packed = BigInt.asUintN(64, exports.feel_call(ptr, bytes.length));
  const responsePtr = Number(packed >> 32n);
  const responseLen = Number(packed & 0xffffffffn);
  const response = new Uint8Array(exports.memory.buffer, responsePtr, responseLen).slice();
  exports.feel_free(responsePtr, responseLen);
  return response;
}

const decode = (bytes) => {
  try {
    return decoder.decode(bytes);
  } catch (e) {
    throw new MalformedResponse(e.message);
  }
};

// Sends one JSON request text and returns the response text; bytes that are not UTF-8 throw
// MalformedResponse. A trap throws; the instance is then dead.
export const callText = (exports, json) => decode(exchange(exports, json));

// Sends one JSON request text and returns the parsed response. A trap throws; the instance is
// then dead.
export const call = (exports, json) => JSON.parse(callText(exports, json));

// A session on a compiled module: a function that sends one JSON request text and resolves to the
// response text, or to `{ trap }` with the error the instance trapped with; the next request then
// runs on a fresh instance. Bytes that are not UTF-8 throw MalformedResponse.
export async function session(module) {
  let exports = await instantiate(module);
  return async (json) => {
    let response;
    try {
      response = exchange(exports, json);
    } catch (trap) {
      exports = await instantiate(module);
      return { trap };
    }
    return decode(response);
  };
}
