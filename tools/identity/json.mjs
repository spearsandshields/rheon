// Lossless JSON for the identity check: numbers keep their source text, so a number beyond the
// range or the precision of a double is sent, keyed and compared as written.

/** Whether this Node parses JSON losslessly: numbers as JSON.rawJSON of their source text. */
export const lossless = typeof JSON.rawJSON === 'function' && JSON.parse('1.50', (_, v, context) => context && context.source) === '1.50';

/** The value of a JSON text, with every number as JSON.rawJSON of its source text. */
export const parseJson = (text) => JSON.parse(text, (_, v, context) => (typeof v === 'number' ? JSON.rawJSON(context.source) : v));

/** Whether a parsed JSON value is an object: not null, an array or a number (JSON.rawJSON). */
export const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v) && !JSON.isRawJSON(v);

/** JSON with the keys of every object sorted: the key of a request, and the form responses are compared in. */
export const canonical = (value) => JSON.stringify(value, (_, v) => (isObject(v) ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, v[k]])) : v));
