/**
 * A minimal WASI preview1 shim: exactly the 10 imports dist/feel.wasm needs (CONTRACT §1).
 * Works in browsers, Web Workers and Node (no `node:wasi`). There is no file system, no
 * environment (empty environ) and no preopened directories; stdout/stderr go to `console`.
 */

const ERRNO_SUCCESS = 0;
const ERRNO_BADF = 8;
const ERRNO_INVAL = 28;
const ERRNO_SPIPE = 70;

/** Thrown by `proc_exit`. */
export class WasiExit extends Error {
  constructor(readonly code: number) {
    super(`wasm called proc_exit(${code})`);
    this.name = 'WasiExit';
  }
}

export interface WasiShimOptions {
  /** Receives every complete line written to fd 1/2. Default: console.log / console.error. */
  onOutput?: (fd: 1 | 2, line: string) => void;
}

export interface WasiShim {
  /** The import object: `{ wasi_snapshot_preview1: {...} }`. */
  readonly imports: WebAssembly.Imports;
  /** Must be called with the instance's memory before running any export. */
  setMemory(memory: WebAssembly.Memory): void;
  /** The last lines written to stderr (e.g. a Rust panic message), most recent last. */
  recentStderr(): string[];
  /** Writes out any partial (unterminated) output lines. */
  flush(): void;
}

const MAX_RECENT = 20;

export function createWasiShim(options: WasiShimOptions = {}): WasiShim {
  let memory: WebAssembly.Memory | null = null;
  const decoders: Record<number, TextDecoder> = { 1: new TextDecoder(), 2: new TextDecoder() };
  const pending: Record<number, string> = { 1: '', 2: '' };
  const recent: string[] = [];
  const onOutput =
    options.onOutput ??
    ((fd: 1 | 2, line: string) => {
      if (fd === 1) console.log(line);
      else console.error(line);
    });

  const mem = (): WebAssembly.Memory => {
    if (!memory) throw new Error('WASI shim: memory not set');
    return memory;
  };
  const view = () => new DataView(mem().buffer);
  const bytes = (ptr: number, len: number) => new Uint8Array(mem().buffer, ptr, len);

  const emit = (fd: 1 | 2, line: string) => {
    if (fd === 2) {
      recent.push(line);
      if (recent.length > MAX_RECENT) recent.shift();
    }
    onOutput(fd, line);
  };

  const nowNanos = (clockId: number): bigint => {
    const perf = globalThis.performance;
    if (clockId === 0 /* realtime */) {
      const ms = perf && typeof perf.timeOrigin === 'number' ? perf.timeOrigin + perf.now() : Date.now();
      return BigInt(Math.round(ms * 1000)) * 1000n;
    }
    // monotonic / process / thread cputime
    const ms = perf ? perf.now() : Date.now();
    return BigInt(Math.round(ms * 1000)) * 1000n;
  };

  const wasi = {
    random_get(ptr: number, len: number): number {
      const buf = bytes(ptr, len);
      for (let off = 0; off < len; off += 65536) {
        globalThis.crypto.getRandomValues(buf.subarray(off, Math.min(len, off + 65536)));
      }
      return ERRNO_SUCCESS;
    },
    clock_time_get(clockId: number, _precision: bigint, out: number): number {
      if (clockId < 0 || clockId > 3) return ERRNO_INVAL;
      view().setBigUint64(out, nowNanos(clockId), true);
      return ERRNO_SUCCESS;
    },
    environ_sizes_get(countOut: number, sizeOut: number): number {
      const v = view();
      v.setUint32(countOut, 0, true);
      v.setUint32(sizeOut, 0, true);
      return ERRNO_SUCCESS;
    },
    environ_get(_environ: number, _buf: number): number {
      return ERRNO_SUCCESS;
    },
    fd_write(fd: number, iovs: number, iovsLen: number, nwrittenOut: number): number {
      if (fd !== 1 && fd !== 2) return ERRNO_BADF;
      const v = view();
      let total = 0;
      for (let i = 0; i < iovsLen; i++) {
        const ptr = v.getUint32(iovs + i * 8, true);
        const len = v.getUint32(iovs + i * 8 + 4, true);
        total += len;
        pending[fd] += decoders[fd].decode(bytes(ptr, len), { stream: true });
      }
      let nl: number;
      while ((nl = pending[fd].indexOf('\n')) >= 0) {
        emit(fd, pending[fd].slice(0, nl));
        pending[fd] = pending[fd].slice(nl + 1);
      }
      v.setUint32(nwrittenOut, total, true);
      return ERRNO_SUCCESS;
    },
    fd_close(fd: number): number {
      return fd <= 2 ? ERRNO_SUCCESS : ERRNO_BADF;
    },
    fd_seek(_fd: number, _offset: bigint, _whence: number, _newOffsetOut: number): number {
      return ERRNO_SPIPE;
    },
    fd_prestat_get(_fd: number, _out: number): number {
      return ERRNO_BADF; // no preopened directories
    },
    fd_prestat_dir_name(_fd: number, _path: number, _len: number): number {
      return ERRNO_BADF;
    },
    proc_exit(code: number): never {
      throw new WasiExit(code);
    },
  };

  return {
    imports: { wasi_snapshot_preview1: wasi as unknown as WebAssembly.ModuleImports },
    setMemory(m) {
      memory = m;
    },
    recentStderr: () => recent.slice(),
    flush() {
      for (const fd of [1, 2] as const) {
        if (pending[fd]) {
          emit(fd, pending[fd]);
          pending[fd] = '';
        }
      }
    },
  };
}
