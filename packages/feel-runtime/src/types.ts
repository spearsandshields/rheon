/** Public types of the FEEL runtime (docs/CONTRACT.md §4). */

export type Dialect = 'expression' | 'unaryTests';

/**
 * Raw JSON text, spliced verbatim into the request JSON (keeps exact number digits).
 * Create it with `rawJson(text)`: only values made by that function are treated as raw JSON
 * (they carry a private brand), so a real context `{ rawJson: "..." }` is just a context.
 */
export interface RawJson {
  readonly rawJson: string;
}

export interface EvaluateRequest {
  dialect?: Dialect;
  expression: string;
  /**
   * The evaluation context: a JS object (serialized with `JSON.stringify`, so numbers are f64),
   * `{ rawJson }` (exact digits), or a FEEL context literal string (`contextFormat: 'feel'`,
   * which is also the default when `context` is a string).
   */
  context?: Record<string, unknown> | string | RawJson;
  contextFormat?: 'json' | 'feel';
  /** Unary tests only: the tested value (`?`). */
  input?: unknown | RawJson;
  /** Host UTC offset in minutes. Defaults to `-new Date().getTimezoneOffset()`. */
  tzOffsetMinutes?: number;
}

export interface FeelResult {
  display: string;
  feel: string;
  type: string;
  typed: unknown;
  isNull: boolean;
  nullReason?: string | null;
}

export type ErrorKind = 'request' | 'syntax' | 'context' | 'evaluation' | 'internal' | 'timeout' | 'crash' | 'cancelled';

/** Per-request options of `FeelEngine` calls. */
export interface RequestOptions {
  /**
   * Cancels the request: a queued request resolves `{kind:'cancelled'}` at once; a running one
   * too, and its worker is terminated and replaced if it does not finish within `cancelGraceMs`.
   */
  signal?: AbortSignal;
}

export interface FeelError {
  kind: ErrorKind;
  message: string;
}

export type EvaluateResponse =
  | { ok: true; result: FeelResult; warnings: { message: string }[]; elapsedMicros: number }
  | { ok: false; error: FeelError };

export type ParseResponse =
  | { ok: true; tree: string; elapsedMicros: number }
  | { ok: false; error: { kind: string; message: string } };

/** Optional context for `parse` (multi-word names only split correctly when known). */
export interface ParseOptions {
  context?: Record<string, unknown> | string | RawJson;
  contextFormat?: 'json' | 'feel';
}

export interface InfoResponse {
  ok: boolean;
  engine?: string;
  /** Commit of the dsntk fork (`wasm` branch) the engine was built from. */
  engineCommit?: string;
  /** Upstream dsntk-rs commit that fork is rebased on. */
  upstreamCommit?: string;
  /** Number backend compiled into the engine: "dashu" (pure Rust, correctly rounded) or "intel". */
  numberBackend?: string;
  version?: string;
  error?: FeelError;
  [key: string]: unknown;
}
