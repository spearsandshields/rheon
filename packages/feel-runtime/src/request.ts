/** Builds the wasm request JSON (CONTRACT §2) from runtime requests, splicing `RawJson` verbatim. */
import type { EvaluateRequest, FeelError, ParseOptions, RawJson } from './types.js';

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
  });
}

export function buildParseJson(dialect: EvaluateRequest['dialect'], expression: string, opts: ParseOptions = {}): string {
  return serializeRequest({
    op: 'parse',
    dialect: dialect ?? 'expression',
    expression,
    ...contextFields(opts.context, opts.contextFormat),
  });
}

export const INFO_JSON = '{"op":"info"}';
