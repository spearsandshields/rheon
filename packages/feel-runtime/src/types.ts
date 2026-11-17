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
  /** Expression language: `'feel'` (default) or `'b-feel'` (DMN 1.6 chapter 11). */
  language?: Language;
  /** `'strict'` turns any error during evaluation into an `evaluation` error (DMN 1.6 §7.3.8). */
  errorMode?: ErrorMode;
  /**
   * A budget of steps for the evaluation, a positive integer: every function call and every item
   * of a loop or a filter is a step, and an evaluation that takes more ends as an `evaluation`
   * error. No budget by default.
   */
  maxSteps?: number;
}

/** One entry of `evaluateAll`: the fields of an evaluation that are its own. */
export interface Evaluation {
  /** Repeated in the answer of the entry. */
  id?: string | number;
  dialect?: Dialect;
  expression: string;
  /** Unary tests only: the tested value (`?`). */
  input?: unknown | RawJson;
  errorMode?: ErrorMode;
  maxSteps?: number;
}

/** Several expressions over one context, which is sent and read once (CONTRACT §2, `evaluateAll`). */
export interface EvaluateAllRequest {
  /** As for `evaluate`; for all evaluations. */
  context?: Record<string, unknown> | string | RawJson;
  contextFormat?: 'json' | 'feel';
  /** Host UTC offset in minutes. Defaults to `-new Date().getTimezoneOffset()`. */
  tzOffsetMinutes?: number;
  language?: Language;
  evaluations: Evaluation[];
}

/** FEEL or B-FEEL (DMN 1.6 chapter 11). */
export type Language = 'feel' | 'b-feel';

/** Error handling mode of an evaluation (DMN 1.6 §7.3.8). */
export type ErrorMode = 'lenient' | 'strict';

/**
 * Errors recorded during an evaluation: null values the core created with an error reason. A null
 * that the specification defines for its input (a missing context entry) may carry a note as its
 * `nullReason`; a note is no error, so it is not counted here and strict mode does not stop at it.
 */
export interface Diagnostics {
  count: number;
  messages: string[];
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
  | { ok: true; result: FeelResult; warnings: { message: string }[]; diagnostics?: Diagnostics; elapsedMicros: number }
  | { ok: false; error: FeelError };

/** The answer of one entry of `evaluateAll`: what `evaluate` answers for it, with its `id`. */
export type EvaluationAnswer = EvaluateResponse & { id?: string | number };

/**
 * `results` has one answer per entry of `evaluations`, in their order; an entry that fails is a
 * failure in its place. `contextWarnings` are the warnings of reading the context, once. A
 * failure of the request itself (the context, a timeout, a crash) has no results.
 */
export type EvaluateAllResponse =
  | { ok: true; results: EvaluationAnswer[]; contextWarnings: { message: string }[]; elapsedMicros: number }
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
  /** Number backend of the engine: always "dashu" (the pure-Rust decimal128 core). */
  numberBackend?: string;
  version?: string;
  error?: FeelError;
  [key: string]: unknown;
}
