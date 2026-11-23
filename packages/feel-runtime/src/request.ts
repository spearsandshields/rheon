/** Builds the wasm request JSON (CONTRACT §2) from runtime requests, splicing `RawJson` verbatim. */
import type { EditorOptions, EvaluateAllRequest, EvaluateEachRequest, EvaluateRequest, EvaluationAnswer, FeelError, ParseOptions, RawJson } from './types.js';

/**
 * Private brand of `RawJson` values. `Symbol.for` so that values made by another copy of this
 * package are recognised too; JSON.stringify ignores symbol keys.
 */
const RAW_JSON_BRAND = Symbol.for('@rheon/feel-runtime.rawJson');

/** Marks `text` as raw JSON to splice verbatim into the request (keeps exact number digits). */
export function rawJson(text: string): RawJson {
  return Object.freeze({ rawJson: text, [RAW_JSON_BRAND]: true }) as RawJson;
}

/** `true` only for values created by `rawJson()` (a plain `{ rawJson: "..." }` object is data). */
export function isRawJson(v: unknown): v is RawJson {
  return (
    typeof v === 'object' &&
    v !== null &&
    (v as Record<symbol, unknown>)[RAW_JSON_BRAND] === true &&
    typeof (v as RawJson).rawJson === 'string'
  );
}

const LONE_SURROGATE = /[\uD800-\uDBFF](?![\uDC00-\uDFFF])|(?<![\uD800-\uDBFF])[\uDC00-\uDFFF]/;

/** The start of an escape that may be one of a surrogate. */
const SURROGATE_ESCAPE = /\\u[dD]/;

/** `true` when `text` is valid Unicode (no unpaired UTF-16 surrogate). */
function isWellFormed(text: string): boolean {
  const native = (text as { isWellFormed?: () => boolean }).isWellFormed;
  return typeof native === 'function' ? native.call(text) : !LONE_SURROGATE.test(text);
}

/**
 * `true` when JSON text contains an unpaired surrogate, either as a raw character or as a
 * `\uD800`-style escape (JSON.parse and JSON.stringify accept both; the wasm core's strict
 * UTF-8 JSON parser does not, CONTRACT §4).
 */
export function hasLoneSurrogate(json: string): boolean {
  if (!isWellFormed(json)) return true;
  // an escape of a surrogate is `\ud…` or `\uD…`: a text without these has none, and the
  // search for them is the JavaScript engine's own, not a walk over every character
  if (!SURROGATE_ESCAPE.test(json)) return false;
  for (let i = 0; i < json.length; i++) {
    if (json[i] !== '\\') continue;
    if (json[i + 1] !== 'u') {
      i++; // an escaped character such as \\ or \"
      continue;
    }
    const code = parseInt(json.slice(i + 2, i + 6), 16);
    if (code >= 0xd800 && code <= 0xdbff) {
      if (json[i + 6] === '\\' && json[i + 7] === 'u') {
        const low = parseInt(json.slice(i + 8, i + 12), 16);
        if (low >= 0xdc00 && low <= 0xdfff) {
          i += 11;
          continue;
        }
      }
      return true;
    }
    if (code >= 0xdc00 && code <= 0xdfff) return true;
    i += 5;
  }
  return false;
}

const UNPAIRED = 'contains an unpaired UTF-16 surrogate (e.g. half of an emoji), which is not valid Unicode text';

/** The error kind for invalid text in a request field: the expression is FEEL source, the rest is input data. */
function fieldKind(key: string): FeelError['kind'] {
  return key === 'expression' ? 'syntax' : key === 'context' || key === 'input' ? 'context' : 'request';
}

/** Host UTC offset in minutes, as the core expects (`-getTimezoneOffset()`). */
export function localTzOffsetMinutes(): number {
  const off = -new Date().getTimezoneOffset();
  return Object.is(off, -0) ? 0 : off;
}

export class RequestError extends Error {
  constructor(readonly error: FeelError) {
    super(error.message);
    this.name = 'RequestError';
  }
}

/**
 * Serializes a request object to JSON. Any top-level field whose value is `{ rawJson }` is
 * validated with `JSON.parse` and spliced verbatim (its number digits are never re-parsed).
 * Fields set to `undefined` are omitted. Throws `RequestError` for invalid raw JSON.
 */
export function serializeRequest(req: Record<string, unknown>): string {
  const parts: string[] = [];
  for (const [key, value] of Object.entries(req)) {
    if (value === undefined) continue;
    if (isRawJson(value)) {
      let parsed: unknown;
      try {
        parsed = JSON.parse(value.rawJson);
      } catch (e) {
        throw new RequestError({ kind: 'context', message: `invalid JSON in ${key}: ${(e as Error).message}` });
      }
      if (key === 'context' && (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed))) {
        throw new RequestError({ kind: 'context', message: 'context JSON must be an object' });
      }
      if (hasLoneSurrogate(value.rawJson)) {
        throw new RequestError({ kind: fieldKind(key), message: `${key} JSON ${UNPAIRED}` });
      }
      parts.push(`${JSON.stringify(key)}:${value.rawJson.trim()}`);
    } else {
      const text = JSON.stringify(value);
      if (text === undefined) continue; // functions, symbols
      if (hasLoneSurrogate(text)) {
        throw new RequestError({ kind: fieldKind(key), message: `${key} ${UNPAIRED}` });
      }
      parts.push(`${JSON.stringify(key)}:${text}`);
    }
  }
  return `{${parts.join(',')}}`;
}

/**
 * The fields of a request object that its builder does not write, sent as they are: the module
 * names a field that it does not read (a misspelt `errormode`) in a warning, as it does for a
 * request sent as JSON text.
 */
function unwritten(req: object, written: readonly string[]): Record<string, unknown> {
  const rest: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(req)) {
    if (value !== undefined && !written.includes(key)) rest[key] = value;
  }
  return rest;
}

const EVALUATE_WRITTEN = ['op', 'dialect', 'expression', 'context', 'contextFormat', 'input', 'tzOffsetMinutes', 'now', 'language', 'errorMode', 'maxSteps'] as const;
const EVALUATE_ALL_WRITTEN = ['op', 'context', 'contextFormat', 'tzOffsetMinutes', 'now', 'language', 'evaluations'] as const;
const EVALUATE_EACH_WRITTEN = ['op', 'dialect', 'expression', 'contextFormat', 'tzOffsetMinutes', 'now', 'language', 'errorMode', 'maxSteps', 'entries'] as const;
const ENTRY_WRITTEN = ['id', 'context', 'input'] as const;

function contextFields(context: ParseOptions['context'], contextFormat: ParseOptions['contextFormat']) {
  if (context === undefined) return {};
  if (typeof context === 'string') {
    return { context, contextFormat: contextFormat ?? 'feel' };
  }
  if (isRawJson(context) && contextFormat === 'feel') {
    throw new RequestError({ kind: 'context', message: "rawJson context requires contextFormat 'json'" });
  }
  return { context, contextFormat: contextFormat ?? 'json' };
}

/** `evaluate` request → JSON text. Fills `tzOffsetMinutes` from the host unless given. */
export function buildEvaluateJson(req: EvaluateRequest, defaultTz: number | undefined = localTzOffsetMinutes()): string {
  const dialect = req.dialect ?? 'expression';
  return serializeRequest({
    op: 'evaluate',
    dialect,
    expression: req.expression,
    ...contextFields(req.context, req.contextFormat),
    input: dialect === 'unaryTests' ? req.input : undefined,
    tzOffsetMinutes: req.tzOffsetMinutes ?? defaultTz,
    now: req.now,
    language: req.language,
    errorMode: req.errorMode,
    maxSteps: req.maxSteps,
    ...unwritten(req, EVALUATE_WRITTEN),
  });
}

/** An `evaluateAll` request as JSON text, and the entries that were not sent. */
export interface EvaluateAllJson {
  json: string;
  /**
   * The entries that are answered here, by their index in `evaluations`: an entry whose text
   * can not be sent (invalid raw JSON, an unpaired surrogate) fails in its place, as an entry
   * that the module refuses does, and the others are sent.
   */
  failures: Map<number, EvaluationAnswer>;
}

/**
 * `evaluateAll` request → JSON text. Fills `tzOffsetMinutes` from the host unless given. A
 * `{ rawJson }` value is spliced verbatim as the context and as the `input` of an entry.
 * `evaluations` that is no array, and an entry that is no object, are sent as they are: the
 * module answers them.
 */
export function buildEvaluateAllJson(req: EvaluateAllRequest, defaultTz: number | undefined = localTzOffsetMinutes()): EvaluateAllJson {
  const failures = new Map<number, EvaluationAnswer>();
  const head = {
    op: 'evaluateAll',
    ...contextFields(req.context, req.contextFormat),
    tzOffsetMinutes: req.tzOffsetMinutes ?? defaultTz,
    now: req.now,
    language: req.language,
    ...unwritten(req, EVALUATE_ALL_WRITTEN),
  };
  const evaluations: unknown = req.evaluations;
  if (!Array.isArray(evaluations)) return { json: serializeRequest({ ...head, evaluations }), failures };
  const entries: string[] = [];
  evaluations.forEach((entry: unknown, index) => {
    try {
      if (typeof entry !== 'object' || entry === null || Array.isArray(entry)) {
        const text = JSON.stringify(entry);
        if (text === undefined || hasLoneSurrogate(text)) throw new RequestError({ kind: 'request', message: 'an evaluation must be an object' });
        entries.push(text);
      } else {
        // as for `evaluate`, the input is sent for unary tests only
        const fields = entry as Record<string, unknown>;
        entries.push(serializeRequest(fields.dialect === 'unaryTests' ? fields : { ...fields, input: undefined }));
      }
    } catch (e) {
      if (!(e instanceof RequestError)) throw e;
      const id = (entry as { id?: unknown } | null)?.id;
      failures.set(index, { ok: false, error: e.error, ...(typeof id === 'string' || typeof id === 'number' ? { id } : {}) });
    }
  });
  // the head is an object with `op` at least, so it ends with `}` after a field
  const json = `${serializeRequest(head).slice(0, -1)},"evaluations":[${entries.join(',')}]}`;
  return { json, failures };
}

/**
 * `evaluateEach` request → JSON text. Fills `tzOffsetMinutes` from the host unless given. A
 * `{ rawJson }` value is spliced verbatim as the `context` and as the `input` of an entry. An
 * entry whose text can not be sent fails in its place ([EvaluateAllJson.failures]); `entries`
 * that is no array, and an entry that is no object, are sent as they are: the module answers them.
 */
export function buildEvaluateEachJson(req: EvaluateEachRequest, defaultTz: number | undefined = localTzOffsetMinutes()): EvaluateAllJson {
  const failures = new Map<number, EvaluationAnswer>();
  const dialect = req.dialect ?? 'expression';
  const list: unknown = req.entries;
  const first = Array.isArray(list) ? list.find((entry) => entry?.context !== undefined)?.context : undefined;
  const contextFormat = req.contextFormat ?? (typeof first === 'string' ? 'feel' : 'json');
  const head = {
    op: 'evaluateEach',
    dialect,
    expression: req.expression,
    contextFormat,
    tzOffsetMinutes: req.tzOffsetMinutes ?? defaultTz,
    now: req.now,
    language: req.language,
    errorMode: req.errorMode,
    maxSteps: req.maxSteps,
    ...unwritten(req, EVALUATE_EACH_WRITTEN),
  };
  if (!Array.isArray(list)) return { json: serializeRequest({ ...head, entries: list }), failures };
  const entries: string[] = [];
  list.forEach((entry: unknown, index) => {
    try {
      if (typeof entry !== 'object' || entry === null || Array.isArray(entry)) {
        const text = JSON.stringify(entry);
        if (text === undefined || hasLoneSurrogate(text)) throw new RequestError({ kind: 'request', message: 'an entry must be an object' });
        entries.push(text);
      } else {
        const fields = entry as Record<string, unknown>;
        if (isRawJson(fields.context) && contextFormat === 'feel') {
          throw new RequestError({ kind: 'context', message: "rawJson context requires contextFormat 'json'" });
        }
        // as for `evaluate`, the input is sent for unary tests only
        entries.push(
          serializeRequest({ id: fields.id, context: fields.context, input: dialect === 'unaryTests' ? fields.input : undefined, ...unwritten(fields, ENTRY_WRITTEN) }),
        );
      }
    } catch (e) {
      if (!(e instanceof RequestError)) throw e;
      const id = (entry as { id?: unknown } | null)?.id;
      failures.set(index, { ok: false, error: e.error, ...(typeof id === 'string' || typeof id === 'number' ? { id } : {}) });
    }
  });
  const json = `${serializeRequest(head).slice(0, -1)},"entries":[${entries.join(',')}]}`;
  return { json, failures };
}

/** Puts the entries that were answered here ([EvaluateAllJson.failures]) into their places in the response. */
export function mergeEvaluateAll<R extends { ok: true; results: EvaluationAnswer[] } | { ok: false }>(response: R, failures: Map<number, EvaluationAnswer>): R {
  if (failures.size === 0 || !response.ok || !Array.isArray(response.results)) return response;
  const sent = response.results;
  const results: EvaluationAnswer[] = [];
  let next = 0;
  for (let index = 0; index < sent.length + failures.size; index++) {
    const failure = failures.get(index);
    results.push(failure ?? sent[next++]);
  }
  return { ...response, results };
}

export function buildParseJson(dialect: EvaluateRequest['dialect'], expression: string, opts: ParseOptions = {}): string {
  return serializeRequest({
    op: 'parse',
    dialect: dialect ?? 'expression',
    expression,
    ...contextFields(opts.context, opts.contextFormat),
  });
}

/** The fields of an editor request that say which names are in scope. */
function editorFields(opts: EditorOptions) {
  if (opts.outline !== undefined && opts.context !== undefined) {
    throw new RequestError({ kind: 'request', message: "give 'context' or 'outline', not both" });
  }
  return opts.outline !== undefined ? { outline: opts.outline } : contextFields(opts.context, opts.contextFormat);
}

/** `syntax` request → JSON text. */
export function buildSyntaxJson(expression: string, opts: EditorOptions & { tree?: boolean } = {}): string {
  return serializeRequest({ op: 'syntax', dialect: opts.dialect ?? 'expression', expression, tree: opts.tree, ...editorFields(opts) });
}

/** `complete` request → JSON text; `cursor` is a UTF-16 offset in `expression`. */
export function buildCompleteJson(expression: string, cursor: number, opts: EditorOptions = {}): string {
  return serializeRequest({ op: 'complete', dialect: opts.dialect ?? 'expression', expression, cursor, ...editorFields(opts) });
}

/** `format` request → JSON text; `width` is the most characters of a line (default 80). */
export function buildFormatJson(expression: string, opts: EditorOptions & { width?: number } = {}): string {
  return serializeRequest({ op: 'format', dialect: opts.dialect ?? 'expression', expression, width: opts.width, ...editorFields(opts) });
}

/** `outline` request → JSON text. */
export function buildOutlineJson(opts: ParseOptions & { types?: boolean } = {}): string {
  return serializeRequest({ op: 'outline', ...contextFields(opts.context, opts.contextFormat), ...(opts.types === undefined ? {} : { types: opts.types }) });
}

export const INFO_JSON = '{"op":"info"}';
