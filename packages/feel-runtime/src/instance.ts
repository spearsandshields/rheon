/**
 * In-thread runner: instantiates dist/feel.wasm with the WASI shim and drives the
 * feel_alloc / feel_call / feel_free ABI (CONTRACT §1).
 */
import { serializeRequest, localTzOffsetMinutes, RequestError } from './request.js';
import { createWasiShim, type WasiShimOptions } from './wasi.js';

interface FeelExports {
  memory: WebAssembly.Memory;
  feel_alloc(len: number): number;
  feel_free(ptr: number, len: number): void;
  feel_call(ptr: number, len: number): bigint;
  _initialize: () => void;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder('utf-8', { fatal: true });

/** Rust's allocator failure message, written to stderr before the instance traps at the memory cap. */
const ALLOCATION_FAILED = /memory allocation of (\d+) bytes failed/;

/**
 * Builds the message of a trap from the JS error text and the last stderr lines. A failed
 * allocation (the instance hit the 512 MiB cap of CONTRACT §1) is named as such, so that hosts
 * can tell "the result does not fit in memory" from a bug ("Engine crashed").
 */
export function trapMessage(base: string, stderr: string[]): string {
  const detail = stderr.length ? `${base}\n${stderr.join('\n')}` : base;
  const oom = stderr.find((line) => ALLOCATION_FAILED.test(line));
  if (!oom) return detail;
  const bytes = Number(oom.match(ALLOCATION_FAILED)![1]);
  const mib = Math.round(bytes / (1024 * 1024));
  return `out of memory: the evaluation needed more than the engine's 512 MiB of memory (an allocation of ${mib} MiB failed)\n${detail}`;
}

/** Thrown by `RawFeelInstance.callJson` when the wasm instance trapped (it is now dead). */
export class FeelTrap extends Error {
  constructor(message: string, readonly cause?: unknown) {
    super(message);
    this.name = 'FeelTrap';
  }
}

/** One live wasm instance. After a `FeelTrap` it must be discarded. */
export class RawFeelInstance {
  private dead = false;
  private constructor(
    private readonly ex: FeelExports,
    private readonly shim: ReturnType<typeof createWasiShim>,
  ) {}

  static async create(module: WebAssembly.Module, shimOptions?: WasiShimOptions): Promise<RawFeelInstance> {
    const shim = createWasiShim(shimOptions);
    const instance = await WebAssembly.instantiate(module, shim.imports);
    return RawFeelInstance.init(instance, shim);
  }

  /** Synchronous variant (fine in workers and Node; browsers forbid it on the main thread). */
  static createSync(module: WebAssembly.Module, shimOptions?: WasiShimOptions): RawFeelInstance {
    const shim = createWasiShim(shimOptions);
    return RawFeelInstance.init(new WebAssembly.Instance(module, shim.imports), shim);
  }

  private static init(instance: WebAssembly.Instance, shim: ReturnType<typeof createWasiShim>): RawFeelInstance {
    const ex = instance.exports as unknown as FeelExports;
    // `_initialize` is required: feel.wasm is a WASI reactor (CONTRACT §1). A module without it was
    // linked without crt1-reactor.o and is broken (no constructors, 1 MiB stack).
    for (const name of ['memory', 'feel_alloc', 'feel_free', 'feel_call', '_initialize'] as const) {
      if (!ex[name]) throw new Error(`feel.wasm is missing export '${name}' (CONTRACT §1)`);
    }
    shim.setMemory(ex.memory);
    ex._initialize(); // reactor: exactly once
    return new RawFeelInstance(ex, shim);
  }

  get isDead(): boolean {
    return this.dead;
  }

  /** Current size of the instance's linear memory in bytes (it only ever grows). */
  get memoryBytes(): number {
    return this.ex.memory.buffer.byteLength;
  }

  /** Sends one UTF-8 JSON request and returns the response JSON text. Throws `FeelTrap` on a trap. */
  callJson(json: string): string {
    if (this.dead) throw new FeelTrap('feel.wasm instance is dead (it trapped earlier)');
    const ex = this.ex;
    let text: string;
    try {
      const req = encoder.encode(json);
      const ptr = ex.feel_alloc(req.length);
      new Uint8Array(ex.memory.buffer, ptr, req.length).set(req);
      const packed = BigInt.asUintN(64, ex.feel_call(ptr, req.length)); // ownership of ptr moves to wasm
      const respPtr = Number(packed >> 32n);
      const respLen = Number(packed & 0xffffffffn);
      // copy out before freeing (memory.buffer may also be replaced by growth)
      const resp = new Uint8Array(ex.memory.buffer, respPtr, respLen).slice();
      ex.feel_free(respPtr, respLen);
      text = decoder.decode(resp);
    } catch (e) {
      this.dead = true;
      this.shim.flush();
      const stderr = this.shim.recentStderr().filter((l) => l.trim());
      const base = e instanceof Error ? `${e.name}: ${e.message}` : String(e);
      throw new FeelTrap(trapMessage(base, stderr), e);
    }
    return text;
  }
}

export interface FeelEnv {
  /** Applied to `evaluate` requests that do not carry their own. Default: host offset. `null` disables. */
  tzOffsetMinutes?: number | null;
  /** Where wasm stdout/stderr lines go (default console). */
  onOutput?: WasiShimOptions['onOutput'];
  /**
   * Re-create the instance after a call leaves its wasm memory larger than this many bytes
   * (default 256 MiB): wasm memory never shrinks.
   */
  recycleMemoryBytes?: number;
}

export interface FeelInstance {
  /**
   * Runs one request synchronously. `req` is a request object (CONTRACT §2; top-level `{rawJson}`
   * values are spliced verbatim) or a JSON string sent as-is. Returns the parsed response.
   * A trap yields `{ok:false, error:{kind:'crash'}}` and the instance is re-created.
   */
  call(req: object | string): any;
  /** Same as `call` but returns the raw response JSON text. */
  callJson(json: string): string;
  /** Discards the current wasm instance and creates a fresh one. */
  reset(): Promise<void>;
  /** The compiled module (reusable, e.g. to post to a worker). */
  readonly module: WebAssembly.Module;
}

function crashJson(message: string): string {
  return JSON.stringify({ ok: false, error: { kind: 'crash', message } });
}

/**
 * Synchronous in-thread runner for Node, tests and other non-worker hosts (CONTRACT §4).
 *
 * **Trusted input only:** a call runs synchronously on the calling thread and nothing can
 * interrupt it, so a runaway expression (`for i in 1..1000000000 return i`) blocks the thread
 * (e.g. a server's event loop) until it finishes. Use `FeelEngine` (a worker with a timeout) for
 * untrusted input, or a host with fuel/epoch interruption (Wasmtime) on the server.
 */
export async function instantiateFeel(
  source: BufferSource | WebAssembly.Module,
  env: FeelEnv = {},
): Promise<FeelInstance> {
  const module = source instanceof WebAssembly.Module ? source : await WebAssembly.compile(source);
  const shimOptions = env.onOutput ? { onOutput: env.onOutput } : undefined;
  let raw: RawFeelInstance | null = await RawFeelInstance.create(module, shimOptions);

  const ensure = (): RawFeelInstance => {
    if (!raw || raw.isDead) raw = RawFeelInstance.createSync(module, shimOptions);
    return raw;
  };

  const recycleAbove = env.recycleMemoryBytes ?? 256 * 1024 * 1024;
  const callJson = (json: string): string => {
    try {
      const instance = ensure();
      const response = instance.callJson(json);
      if (instance.memoryBytes > recycleAbove) raw = null; // re-created lazily on the next call
      return response;
    } catch (e) {
      if (!(e instanceof FeelTrap)) throw e;
      try {
        raw = RawFeelInstance.createSync(module, shimOptions);
      } catch {
        raw = null; // re-created lazily on the next call
      }
      return crashJson(e.message);
    }
  };

  return {
    module,
    callJson,
    call(req) {
      let json: string;
      if (typeof req === 'string') {
        json = req;
      } else {
        const obj = { ...(req as Record<string, unknown>) };
        if (obj.op === 'evaluate' && obj.tzOffsetMinutes === undefined && env.tzOffsetMinutes !== null) {
          obj.tzOffsetMinutes = env.tzOffsetMinutes ?? localTzOffsetMinutes();
        }
        try {
          json = serializeRequest(obj);
        } catch (e) {
          if (e instanceof RequestError) return { ok: false, error: e.error };
          throw e;
        }
      }
      return JSON.parse(callJson(json));
    },
    async reset() {
      raw = await RawFeelInstance.create(module, shimOptions);
    },
  };
}
