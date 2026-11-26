/** Public types of the FEEL runtime (docs/CONTRACT.md §4). */

/**
 * The start rule that a text is read as: an expression (default), unary tests, a simple
 * expression of S-FEEL (DMN 9.2), or a list of simple or textual expressions (the list of
 * their values). See docs/CONTRACT.md.
 */
export type Dialect = 'expression' | 'unaryTests' | 'simpleExpression' | 'simpleExpressions' | 'textualExpressions';

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
  /** The instant `now()` and `today()` read instead of the clock: an ISO 8601 instant with its offset or `Z`. */
  now?: string;
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
  /** The instant `now()` and `today()` read instead of the clock: an ISO 8601 instant with its offset or `Z`. */
  now?: string;
  language?: Language;
  evaluations: Evaluation[];
}

/** One entry of `evaluateEach`: the context of an evaluation and, for unary tests, the tested value. */
export interface EachEntry {
  /** Repeated in the answer of the entry. */
  id?: string | number;
  /** As for `evaluate`, in the `contextFormat` of the request. */
  context?: Record<string, unknown> | string | RawJson;
  /** Unary tests only: the tested value (`?`). */
  input?: unknown | RawJson;
}

/**
 * One expression over several contexts (CONTRACT §2, `evaluateEach`): the expression is parsed
 * once for the entries whose contexts have the same names.
 */
export interface EvaluateEachRequest {
  dialect?: Dialect;
  expression: string;
  /** For the contexts of all entries; `'feel'` by default when the first context is a string. */
  contextFormat?: 'json' | 'feel';
  /** Host UTC offset in minutes. Defaults to `-new Date().getTimezoneOffset()`. */
  tzOffsetMinutes?: number;
  /** The instant `now()` and `today()` read instead of the clock: an ISO 8601 instant with its offset or `Z`. */
  now?: string;
  language?: Language;
  errorMode?: ErrorMode;
  /** A budget of steps for each entry. */
  maxSteps?: number;
  entries: EachEntry[];
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

/** A warning of a response: what it is about, for a program (a code of the list in docs/CONTRACT.md §2), and the message. */
export interface Warning {
  code: string;
  message: string;
}

export interface FeelError {
  kind: ErrorKind;
  /**
   * What failed, for a program: a code of the list in docs/CONTRACT.md §3 (`missing-field`,
   * `syntax-error`, `limit-steps`, …), or for an error that this runtime makes `invalid-context`,
   * `invalid-field`, `conflicting-fields`, `timeout`, `crash` or `cancelled`. Stable; the message can change.
   */
  code: string;
  message: string;
  /** For a syntax error of the expression: where and what its errors are, as `syntax` answers them. */
  diagnostics?: SyntaxDiagnostic[];
}

export type EvaluateResponse =
  | { ok: true; result: FeelResult; warnings: Warning[]; diagnostics?: Diagnostics; elapsedMicros: number }
  | { ok: false; error: FeelError };

/**
 * A step of the way to where a node is evaluated (`explain`): into an iteration of the loop or
 * the filter at a place, or into the body of the function that the invocation at a place runs.
 * Places are `[from, to]` in UTF-16 units.
 */
export type TraceStep = { at: [number, number]; iteration: number } | { at: [number, number]; call: true };

/**
 * A node whose parts `explain` records: its place, or its place with where it is evaluated
 * (`within`, as a record of the node has it) and, for a loop or a filter, the iterations wanted
 * (`[from, to)`, by default the first 20).
 */
export type ExpandEntry = [number, number] | { at: [number, number]; within?: TraceStep[]; iterations?: [number, number] };

/** The fields of `evaluate` for an expression, and which nodes to record. */
export interface ExplainRequest extends Omit<EvaluateRequest, 'dialect' | 'input'> {
  /** The nodes whose parts are recorded too; by default the root of the text and its parts. */
  expand?: ExpandEntry[];
  /** The most records of the answer, 1 to 10000 (default 200); beyond it `traceTruncated` is true. */
  maxRecords?: number;
  /** With true each record has `reads`, the names that the text of its node reads. */
  reads?: boolean;
}

/** A value too long to write whole in a record: its type and, of a list or a context, how many items or entries it has. */
export interface SummarizedValue {
  summarized: true;
  type: string;
  isNull: boolean;
  count?: number;
}

/** The value that an evaluation gave for a node of the text (`explain`). */
export interface TraceRecord {
  /** The place of the node, in UTF-16 units. */
  from: number;
  to: number;
  /** What the node is: `Addition`, `Invocation`, `Entry`, `For`, … (the kinds of the nodes of `syntax`). */
  kind: string;
  /** Of an entry of a context literal: its name. */
  name?: string;
  value: FeelResult | SummarizedValue;
  /** Of the body of a loop or the predicate of a filter: which iteration the record is of. */
  iteration?: number;
  /** Of a loop or a filter: how many iterations it had. */
  iterations?: number;
  /** Where it is evaluated, outermost first; absent for a node evaluated once at the root. Pass it back in `expand`. */
  within?: TraceStep[];
  /** Of a null: the part of the node that the null is from, as an entry of `expand`. */
  nullFrom?: { at: [number, number]; within?: TraceStep[] };
  /** Of a null: the null is the node's own (its parts are not null, or it has none). */
  nullOrigin?: true;
  /** With `nullOrigin`: why the null is the node's own. */
  nullCode?: 'missing-entry' | 'null-input' | 'operator' | 'function' | 'other';
  /** Asked for with `reads`: the names that the text of the node reads from around it, as paths (`loans[].amount`). */
  reads?: string[];
}

/** The answer of `explain`: that of `evaluate`, and the records in the order of the text. */
export type ExplainResponse =
  | { ok: true; result: FeelResult; warnings: Warning[]; diagnostics?: Diagnostics; trace: TraceRecord[]; traceTruncated: boolean; elapsedMicros: number }
  | { ok: false; error: FeelError };

/** The answer of one entry of `evaluateAll`: what `evaluate` answers for it, with its `id`. */
export type EvaluationAnswer = EvaluateResponse & { id?: string | number };

/**
 * `results` has one answer per entry of `evaluations`, in their order; an entry that fails is a
 * failure in its place. `contextWarnings` are the warnings of reading the context, once. A
 * failure of the request itself (the context, a timeout, a crash) has no results.
 */
export type EvaluateAllResponse =
  | { ok: true; results: EvaluationAnswer[]; contextWarnings: Warning[]; elapsedMicros: number }
  | { ok: false; error: FeelError };

/**
 * `results` has one answer per entry of `entries`, in their order: what `evaluate` answers for
 * the expression with that entry's context and input; an entry that fails is a failure in its
 * place. A failure of the request itself (a timeout, a crash) has no results.
 */
export type EvaluateEachResponse =
  | { ok: true; results: EvaluationAnswer[]; elapsedMicros: number }
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
 * names of a context, or the outline of the items of a list. A name that is no context can
 * have its FEEL type (`{ type: "number" }`, `{ type: "list<string>" }`): `outline` writes the
 * types when asked (`types: true`), and a consumer that knows them can write them. The editor
 * operations take an outline in place of the context, so an editor does not send the values
 * with every keystroke.
 */
export type Outline = null | (OutlineForm & OutlineSaid);

/** What a name is: of a type, a context of names, a list of items, or (the outline of a request) only the types. */
export type OutlineForm =
  | { type: string; values?: OutlineValue[] }
  | { names: Record<string, Outline> }
  | { items: Outline }
  | { function: { parameters?: { name: string; type?: string; description?: string }[]; result?: string } }
  | { names?: Record<string, Outline>; types: Record<string, Outline> };

/** A value that a name of a string, number or boolean type can have, alone or with what it stands for. */
export type OutlineValue = string | number | boolean | { value: string | number | boolean; description?: string };

/** What an outline says of a name beside its type. */
export interface OutlineSaid {
  /** What the name is for: the `documentation` of its completion item. */
  description?: string;
  /** That the name should no longer be used: `true`, or the reason. */
  deprecated?: boolean | string;
  /** The types that the type texts of the outline can name; only in the outline of a request. */
  types?: Record<string, Outline>;
}

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
  /** What it is, for a program: of a syntax error a code of the parser, of a check `unknown-name`, `unknown-entry`, `deprecated`, `not-a-value` or `always-null`. */
  code: string;
  /** Only in an answer asked with `check`: a syntax error is an `error`, what a check finds a `warning` or a `hint`. Without it, an error. */
  severity?: 'error' | 'warning' | 'information' | 'hint';
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
  | { ok: true; tokens: [TokenClass, number, number][]; diagnostics: SyntaxDiagnostic[]; truncated: boolean; nodes?: [string, number, number][]; /** Only with `check`: whether the text was checked (it parses, and the analysis ended). */ checked?: boolean; elapsedMicros: number }
  | { ok: false; error: FeelError };

/** An item of `complete`: accepted, its `label` replaces the text from `from` to the answer's `to`. */
export interface CompletionItem {
  label: string;
  kind: 'value' | 'parameter' | 'name' | 'entry' | 'keyword' | 'function' | 'type';
  from: number;
  /** The FEEL type of the item when the engine knows it: of a parameter, and of the result of a built-in function. */
  type: string | null;
  /** More about the item: the first signature of a built-in function. */
  detail: string | null;
  /** One line that says what a built-in function gives, or the description an outline has for the name. */
  documentation?: string;
  /** The text to insert when it is not the label: a parameter with its colon. */
  insertText?: string;
  /** That an outline marks the name as deprecated: `true`, or the reason. */
  deprecated?: true | string;
}

/** A parameter of a signature of a built-in function. */
export interface SignatureParameter {
  name: string;
  /** The FEEL type, as text. */
  type: string;
  optional: boolean;
  /** Stands for any number of arguments, one or more. */
  rest: boolean;
  /** What an outline says the parameter is for. */
  documentation?: string;
}

/** One way to invoke a built-in function. */
export interface FunctionSignature {
  /** `substring(string, start position, length?): string` */
  label: string;
  parameters: SignatureParameter[];
  result: string;
  /** Whether the parameters can be given by name. */
  named: boolean;
  /** The index of the parameter that the argument of the cursor binds to; absent when it binds to none, or arguments are given by name. */
  activeParameter?: number;
}

/** The invocation of a built-in function that the cursor stands in. */
export interface CompletionCall {
  function: string;
  /** One line that says what the function gives; null for a function of an outline without a description. */
  documentation: string | null;
  /** The index of the first signature that has a parameter for the argument of the cursor, else 0. */
  activeSignature: number;
  /** The index of the argument that the cursor is in. */
  argument: number;
  /** The names of the named parameters given before that argument. */
  named: string[];
  signatures: FunctionSignature[];
}

/** What the name at a place of a text is (`hover`). */
export interface Hover {
  from: number;
  to: number;
  kind: 'name' | 'entry' | 'function' | 'variable' | 'parameter' | 'key';
  /** The FEEL type of the name when the engine knows it. */
  type: string | null;
  /** What a built-in function gives, or the description an outline has for the name. */
  documentation?: string;
  /** That an outline marks the name as deprecated: `true`, or the reason. */
  deprecated?: true | string;
  /** Of a function: the ways to invoke it. */
  signatures?: FunctionSignature[];
}

/** A part of a value in scope that an expression reads (`paths`). */
export interface ReadPath {
  /** From a name in scope down: the name of an entry, or `[]` for the items of a list. */
  path: (string | [])[];
  /** The path for people: `loans[].payment`. */
  text: string;
  /** Whether the value at the path is read as a value (all of it can be read), or only what the paths below it read. */
  whole: boolean;
  /** Where the text reads it: `[from, to]` in UTF-16 units. */
  places: [number, number][];
}

/** The answer of `paths`: what the expression reads of the names in scope, exact or wider. */
export type PathsResponse =
  | {
      ok: true;
      paths: ReadPath[];
      /** The names that nothing gives (only with a context or an outline). */
      unknown: { name: string; places: [number, number][] }[];
      /** False when the analysis reached its limit: the expression can read more than `paths` lists. */
      complete: boolean;
      elapsedMicros: number;
    }
  | { ok: false; error: FeelError };

/** The answer of `hover`: null where no name stands at the place. */
export type HoverResponse = { ok: true; hover: Hover | null; elapsedMicros: number } | { ok: false; error: FeelError };

/** The answer of `complete`: what can stand at the cursor (`to`), and what the grammar takes there (`expects`). */
export type CompleteResponse =
  | { ok: true; to: number; expects: string; items: CompletionItem[]; truncated: boolean; call?: CompletionCall; elapsedMicros: number }
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
