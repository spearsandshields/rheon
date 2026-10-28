/**
 * `FeelEngine`: runs feel.wasm in a module Web Worker (CONTRACT §4).
 *
 * - The wasm is fetched and compiled once (main thread); the `WebAssembly.Module` is posted to
 *   each worker, so restarting a worker is cheap.
 * - Requests are serialized in FIFO order, one in flight at a time.
 * - Timeout: the worker is terminated, the request resolves `{kind:'timeout'}`, a fresh worker
 *   is started for the next request.
 * - Trap (panic / unreachable / stack overflow): resolves `{kind:'crash'}` and the worker is
 *   replaced by a fresh one.
 * - Cancellation: a request whose `AbortSignal` fires resolves `{kind:'cancelled'}`; if it was
 *   already running for longer than `cancelGraceMs`, the worker is terminated and replaced, so a
 *   superseded runaway evaluation does not hold up the next request.
 * - Memory: wasm memory never shrinks, so a worker whose instance grew past
 *   `recycleMemoryBytes` is replaced after the request that grew it.
 * - Restart failures: a worker that fails to start is retried with backoff; after
 *   `RESTART_BACKOFF_MS.length` consecutive failures the engine gives up (`unavailable`).
 */
import { buildEvaluateJson, buildParseJson, INFO_JSON, RequestError } from './request.js';
import type { FromWorker, ToWorker } from './protocol.js';
import type {
  Dialect,
  ErrorKind,
  EvaluateRequest,
  EvaluateResponse,
  InfoResponse,
  ParseOptions,
  ParseResponse,
  RequestOptions,
} from './types.js';

/** The subset of the Web `Worker` API the engine uses (lets tests inject other workers). */
export interface WorkerLike {
  postMessage(message: ToWorker): void;
  terminate(): void;
  onmessage: ((ev: { data: FromWorker }) => void) | null;
  onerror: ((ev: { message?: string; error?: unknown; preventDefault?: () => void }) => void) | null;
}

export interface FeelEngineOptions {
  /** URL of feel.wasm (fetched + compiled once). One of `wasmUrl` / `wasmModule` / `wasmBytes` is required. */
  wasmUrl?: string | URL;
  /** An already compiled module. */
  wasmModule?: WebAssembly.Module;
  /** The raw wasm bytes. */
  wasmBytes?: BufferSource;
  /** Per-request timeout in ms (default 2000). */
  timeoutMs?: number;
  /**
   * A cancelled request that is already running is allowed to finish for this long (ms) before
   * the worker is terminated and replaced (default 100). Quick evaluations finish normally;
   * runaway ones are cut short instead of running into `timeoutMs`.
   */
  cancelGraceMs?: number;
  /** Replace the worker after a request leaves its wasm memory larger than this (default 256 MiB). */
  recycleMemoryBytes?: number;
  /** A starting worker that is not ready after this long (ms) counts as a failed start (default 15000). */
  startupTimeoutMs?: number;
  /** Override worker creation (tests, custom bundlers). Default: the bundled module worker. */
  createWorker?: () => WorkerLike;
}

interface Job {
  id: number;
  json: string;
  resolve: (response: any) => void;
  signal?: AbortSignal;
  onAbort?: () => void;
  startedAt?: number;
}

const DEFAULT_TIMEOUT_MS = 2000;
const DEFAULT_CANCEL_GRACE_MS = 100;
const DEFAULT_RECYCLE_MEMORY_BYTES = 256 * 1024 * 1024;
const DEFAULT_STARTUP_TIMEOUT_MS = 15000;
/** Delays before retrying a worker that failed to start; its length is the number of retries. */
export const RESTART_BACKOFF_MS = [100, 500, 2000] as const;

function defaultCreateWorker(): WorkerLike {
  // This exact `new Worker(new URL('./worker.js', import.meta.url), { type: 'module' })` form is
  // what Vite / webpack 5 / Parcel recognise to bundle the worker.
  return new Worker(new URL('./worker.js', import.meta.url), { type: 'module' }) as unknown as WorkerLike;
}

async function compileFrom(opts: FeelEngineOptions): Promise<WebAssembly.Module> {
  if (opts.wasmModule) return opts.wasmModule;
  if (opts.wasmBytes) return WebAssembly.compile(opts.wasmBytes);
  if (!opts.wasmUrl) throw new Error('FeelEngine: one of wasmUrl, wasmModule or wasmBytes is required');
  const response = await fetch(opts.wasmUrl);
  if (!response.ok) throw new Error(`FeelEngine: fetching ${String(opts.wasmUrl)} failed: HTTP ${response.status}`);
  const type = response.headers.get('content-type') ?? '';
  if (typeof WebAssembly.compileStreaming === 'function' && type.startsWith('application/wasm')) {
    return WebAssembly.compileStreaming(response);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

function failure(kind: ErrorKind, message: string) {
  return { ok: false as const, error: { kind, message } };
}

const CANCELLED = () => failure('cancelled', 'request was cancelled');

export class FeelEngine {
  readonly timeoutMs: number;
  readonly cancelGraceMs: number;
  readonly recycleMemoryBytes: number;
  readonly startupTimeoutMs: number;
  private readonly module: WebAssembly.Module;
  private readonly createWorker: () => WorkerLike;
  private worker: WorkerLike | null = null;
  private generation = 0;
  private ready = false;
  private disposed = false;
  private unavailable: string | null = null;
  private queue: Job[] = [];
  private current: Job | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private cancelTimer: ReturnType<typeof setTimeout> | null = null;
  private startupTimer: ReturnType<typeof setTimeout> | null = null;
  private retryTimer: ReturnType<typeof setTimeout> | null = null;
  private startFailures = 0;
  private nextId = 1;
  private startup: { resolve: () => void; reject: (e: Error) => void } | null = null;
  /** Number of workers started so far (1 + restarts). Useful for diagnostics/tests. */
  workerStarts = 0;

  private constructor(module: WebAssembly.Module, opts: FeelEngineOptions) {
    this.module = module;
    this.timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    this.cancelGraceMs = opts.cancelGraceMs ?? DEFAULT_CANCEL_GRACE_MS;
    this.recycleMemoryBytes = opts.recycleMemoryBytes ?? DEFAULT_RECYCLE_MEMORY_BYTES;
    this.startupTimeoutMs = opts.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT_MS;
    this.createWorker = opts.createWorker ?? defaultCreateWorker;
  }

  /** Loads + compiles the wasm once, spawns the worker and waits until it is ready. */
  static async create(opts: FeelEngineOptions): Promise<FeelEngine> {
    const module = await compileFrom(opts);
    const engine = new FeelEngine(module, opts);
    await new Promise<void>((resolve, reject) => {
      engine.startup = { resolve, reject };
      engine.spawn();
    });
    return engine;
  }

  /** `true` after `dispose()`, or when the engine gave up restarting its worker (see `unavailableReason`). */
  get isDisposed(): boolean {
    return this.disposed;
  }

  /** Why the engine became unusable on its own (workers kept failing to start), else `null`. */
  get unavailableReason(): string | null {
    return this.unavailable;
  }

  evaluate(req: EvaluateRequest, opts: RequestOptions = {}): Promise<EvaluateResponse> {
    let json: string;
    try {
      json = buildEvaluateJson(req);
    } catch (e) {
      if (e instanceof RequestError) return Promise.resolve({ ok: false, error: e.error });
      return Promise.resolve(failure('request', String(e)));
    }
    return this.enqueue(json, opts.signal);
  }

  parse(dialect: Dialect, expression: string, opts: ParseOptions & RequestOptions = {}): Promise<ParseResponse> {
    let json: string;
    try {
      json = buildParseJson(dialect, expression, opts);
    } catch (e) {
      if (e instanceof RequestError) return Promise.resolve({ ok: false, error: e.error });
      return Promise.resolve(failure('request', String(e)));
    }
    return this.enqueue(json, opts.signal);
  }

  info(): Promise<InfoResponse> {
    return this.enqueue(INFO_JSON);
  }

  /** Sends a raw request JSON (CONTRACT §2) as-is. */
  callJson(json: string, opts: RequestOptions = {}): Promise<any> {
    return this.enqueue(json, opts.signal);
  }

  /** Terminates the worker. Pending and later requests resolve `{kind:'internal', message:'FeelEngine disposed'}`. */
  dispose(): void {
    this.shutDown('FeelEngine disposed');
  }

  // ───────────────────────────── internals ─────────────────────────────

  private shutDown(message: string): void {
    if (this.disposed) return;
    this.disposed = true;
    this.clearTimer();
    this.clearCancelTimer();
    this.clearStartupTimer();
    if (this.retryTimer !== null) {
      clearTimeout(this.retryTimer);
      this.retryTimer = null;
    }
    this.killWorker();
    const pending = [...(this.current ? [this.current] : []), ...this.queue];
    this.current = null;
    this.queue = [];
    for (const job of pending) this.settle(job, failure('internal', message));
  }

  private enqueue(json: string, signal?: AbortSignal): Promise<any> {
    if (this.disposed) return Promise.resolve(failure('internal', this.disposedMessage()));
    if (signal?.aborted) return Promise.resolve(CANCELLED());
    return new Promise((resolve) => {
      const job: Job = { id: this.nextId++, json, resolve, signal };
      if (signal) {
        job.onAbort = () => this.cancel(job);
        signal.addEventListener('abort', job.onAbort, { once: true });
      }
      this.queue.push(job);
      this.pump();
    });
  }

  private disposedMessage(): string {
    return this.unavailable ? `FeelEngine unavailable: ${this.unavailable}` : 'FeelEngine disposed';
  }

  /** Resolves a job exactly once and detaches its abort listener. */
  private settle(job: Job, response: any): void {
    if (job.signal && job.onAbort) job.signal.removeEventListener('abort', job.onAbort);
    job.onAbort = undefined;
    job.resolve(response);
  }

  private cancel(job: Job): void {
    const index = this.queue.indexOf(job);
    if (index >= 0) {
      this.queue.splice(index, 1);
      this.settle(job, CANCELLED());
      return;
    }
    if (this.current !== job) return; // already settled
    const ranFor = job.startedAt === undefined ? 0 : Date.now() - job.startedAt;
    const kill = () => {
      if (this.current !== job) return; // it finished within the grace period
      this.current = null;
      this.clearTimer();
      this.clearCancelTimer();
      this.settle(job, CANCELLED());
      this.restart();
    };
    if (ranFor >= this.cancelGraceMs) kill();
    else {
      this.clearCancelTimer();
      this.cancelTimer = setTimeout(kill, this.cancelGraceMs - ranFor);
    }
  }

  private spawn(): void {
    const gen = ++this.generation;
    this.ready = false;
    this.workerStarts++;
    let worker: WorkerLike;
    try {
      worker = this.createWorker();
    } catch (e) {
      this.onStartFailure(`cannot start worker: ${e instanceof Error ? e.message : String(e)}`);
      return;
    }
    this.worker = worker;
    worker.onmessage = (ev) => {
      if (gen === this.generation) this.onMessage(ev.data);
    };
    worker.onerror = (ev) => {
      ev.preventDefault?.();
      if (gen !== this.generation) return;
      const err = ev.error instanceof Error ? `${ev.error.name}: ${ev.error.message}` : ev.message ?? 'worker error';
      if (this.ready) this.onWorkerError(err);
      else this.onStartFailure(err);
    };
    this.clearStartupTimer();
    this.startupTimer = setTimeout(() => {
      this.startupTimer = null;
      if (gen === this.generation && !this.ready) this.onStartFailure(`worker did not become ready within ${this.startupTimeoutMs} ms`);
    }, this.startupTimeoutMs);
    worker.postMessage({ type: 'init', module: this.module });
  }

  private killWorker(): void {
    this.generation++; // ignore any late messages of the old worker
    this.ready = false;
    this.clearStartupTimer();
    if (this.worker) {
      this.worker.onmessage = null;
      this.worker.onerror = null;
      this.worker.terminate();
      this.worker = null;
    }
  }

  private restart(): void {
    this.killWorker();
    if (!this.disposed) this.spawn();
  }

  private onMessage(msg: FromWorker): void {
    switch (msg.type) {
      case 'ready':
        this.ready = true;
        this.startFailures = 0;
        this.clearStartupTimer();
        if (this.startup) {
          this.startup.resolve();
          this.startup = null;
        }
        this.pump();
        return;
      case 'initError':
        this.onStartFailure(`feel.wasm instantiation failed: ${msg.message}`);
        return;
      case 'result': {
        const job = this.takeCurrent(msg.id);
        if (!job) return;
        let response: any;
        try {
          response = JSON.parse(msg.json);
        } catch (e) {
          response = failure('internal', `invalid response JSON: ${(e as Error).message}`);
        }
        this.settle(job, response);
        // wasm memory never shrinks: hand a bloated instance's memory back by replacing the worker
        if (typeof msg.memoryBytes === 'number' && msg.memoryBytes > this.recycleMemoryBytes) this.restart();
        else this.pump();
        return;
      }
      case 'crash': {
        const job = this.takeCurrent(msg.id);
        if (job) this.settle(job, failure('crash', msg.message));
        this.restart();
        return;
      }
    }
  }

  /** A ready worker failed outside a reported trap (uncaught error): fail the current request, restart. */
  private onWorkerError(message: string): void {
    const job = this.current;
    this.current = null;
    this.clearTimer();
    this.clearCancelTimer();
    if (job) this.settle(job, failure('crash', message));
    this.restart();
  }

  /** A worker failed before it became ready (script load error, instantiation failure, startup timeout). */
  private onStartFailure(message: string): void {
    this.killWorker();
    if (this.startup) {
      this.startup.reject(new Error(message));
      this.startup = null;
      this.shutDown(message);
      return;
    }
    this.startFailures++;
    if (this.startFailures > RESTART_BACKOFF_MS.length) {
      this.unavailable = `the worker failed to start ${this.startFailures} times in a row (last error: ${message})`;
      this.shutDown(this.disposedMessage());
      return;
    }
    // transient failures (e.g. memory of a terminated worker not released yet): retry with backoff
    const delay = RESTART_BACKOFF_MS[this.startFailures - 1];
    this.retryTimer = setTimeout(() => {
      this.retryTimer = null;
      if (!this.disposed) this.spawn();
    }, delay);
  }

  private takeCurrent(id: number): Job | null {
    if (!this.current || this.current.id !== id) return null;
    const job = this.current;
    this.current = null;
    this.clearTimer();
    this.clearCancelTimer();
    return job;
  }

  private clearTimer(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private clearCancelTimer(): void {
    if (this.cancelTimer !== null) {
      clearTimeout(this.cancelTimer);
      this.cancelTimer = null;
    }
  }

  private clearStartupTimer(): void {
    if (this.startupTimer !== null) {
      clearTimeout(this.startupTimer);
      this.startupTimer = null;
    }
  }

  private pump(): void {
    if (this.disposed || !this.ready || this.current || !this.worker) return;
    const job = this.queue.shift();
    if (!job) return;
    this.current = job;
    job.startedAt = Date.now();
    this.timer = setTimeout(() => this.onTimeout(job), this.timeoutMs);
    this.worker.postMessage({ type: 'call', id: job.id, json: job.json });
  }

  private onTimeout(job: Job): void {
    if (this.current !== job) return;
    this.timer = null;
    this.current = null;
    this.clearCancelTimer();
    this.settle(job, failure('timeout', `evaluation exceeded ${this.timeoutMs} ms; the engine was restarted`));
    this.restart();
  }
}
