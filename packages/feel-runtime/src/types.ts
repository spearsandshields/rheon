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
  /** For a syntax error of the expression: where and what its errors are, as `syntax` answers them. */
  diagnostics?: SyntaxDiagnostic[];
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

/**
 * The names of a context without its values (`outline`): null for a name without entries, the
 * names of a context, or the outline of the items of a list. The editor operations take it in
 * place of the context, so an editor does not send the values with every keystroke.
 */
export type Outline = null | { names: Record<string, Outline> } | { items: Outline };

/** What the editor operations (`syntax`, `complete`) know about the names in scope: a context or its outline, not both. */
export interface EditorOptions {
  dialect?: Dialect;
  context?: Record<string, unknown> | string | RawJson;
  contextFormat?: 'json' | 'feel';
  outline?: Outline;
}

/** What a token is in its expression (`syntax`). */
export type TokenClass =
  | 'whitespace'
  | 'comment'
  | 'keyword'
  | 'name'
  | 'number'
  | 'string'
  | 'constant'
  | 'operator'
  | 'punctuation'
  | 'function'
  | 'type'
  | 'key'
  | 'parameter'
  | 'variable'
  | 'property'
  | 'error';

/** A syntax error of a text: `code` is for programs and stable, `message` is for people. Places are UTF-16 offsets. */
export interface SyntaxDiagnostic {
  code: string;
  message: string;
  from: number;
  to: number;
}

/**
 * The answer of `syntax`: every token of the text as `[class, from, to]` (together they are the
 * whole text), every syntax error (at most 100; `truncated` says that there are more), and with
 * `tree` the nodes as `[kind, from, to]`, each before the nodes inside it.
 */
export type SyntaxResponse =
  | { ok: true; tokens: [TokenClass, number, number][]; diagnostics: SyntaxDiagnostic[]; truncated: boolean; nodes?: [string, number, number][]; elapsedMicros: number }
  | { ok: false; error: FeelError };

/** An item of `complete`: accepted, its `label` replaces the text from `from` to the answer's `to`. */
export interface CompletionItem {
  label: string;
  kind: 'name' | 'entry' | 'keyword' | 'function' | 'type';
  from: number;
  /** The FEEL type of the item when the engine knows it (not yet: always null). */
  type: string | null;
  /** More about the item, such as the signature of a function (not yet: always null). */
  detail: string | null;
}

/** The answer of `complete`: what can stand at the cursor (`to`), and what the grammar takes there (`expects`). */
export type CompleteResponse =
  | { ok: true; to: number; expects: string; items: CompletionItem[]; truncated: boolean; elapsedMicros: number }
  | { ok: false; error: FeelError };

/**
 * The answer of `format`: the text in the formatter's one layout. `changed` is false when the
 * text already has it; `reason` says why a text was left as it is although it parses.
 */
export type FormatResponse = { ok: true; formatted: string; changed: boolean; reason?: string; elapsedMicros: number } | { ok: false; error: FeelError };

export type OutlineResponse = { ok: true; outline: Outline; elapsedMicros: number } | { ok: false; error: FeelError };

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
