#!/usr/bin/env node
// Full-response identity of two feel.wasm modules, with a ledger of registered differences.
//
//   node equal.mjs <reference.wasm> <candidate.wasm> [source…] [--ledger <file|dir>]…
//                  [--parse-too] [--record <out.json> --why <basis>]
//
// A source is a JSON file holding one request or an array of requests, or a directory whose *.json
// files are such sources. Every request goes through both modules (raw ABI, node:wasi; a fresh
// instance after a trap), byte for byte as it is written in its file. Requests that call now() or
// today() are left out: their answers depend on the clock. With --parse-too, every evaluate
// request is also sent as a parse request, the same request with `op` set to "parse".
//
// JSON is read losslessly: a number keeps its source text, so a number beyond the range or the
// precision of a double is sent, keyed and compared as written, in requests, responses and the
// ledger alike. The run fails with status 2 on a Node without JSON.rawJSON and the source text
// of a parsed number.
//
// A ledger is a JSON array of registered differences (README.md); a directory adds its *.json
// files. Every ledger entry is run even if no source lists its request: the reference must answer
// `old` and the candidate `new`. Any other difference fails the run.
//
// --record writes the unregistered differences of the run to <out.json> as ledger entries in the
// full-response form, each with the given `why`, as a draft to check against the spec.
//
// Exit status: 0 when every response is identical or registered, 1 on a difference, a ledger entry
// that does not reproduce or a malformed response, 2 on a usage or input error.
import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { MalformedResponse, session } from '../../scripts/feel-abi.mjs';

class InputError extends Error {}

function usage(message) {
  throw new InputError(`${message}\nusage: node equal.mjs <reference.wasm> <candidate.wasm> [source…] [--ledger <file|dir>]… [--parse-too] [--record <out.json> --why <basis>]`);
}

function parseArgs(argv) {
  const options = { positional: [], ledgers: [], parseToo: false, record: null, why: null };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => (i + 1 < argv.length ? argv[++i] : usage(`${arg} needs a value`));
    if (arg === '--ledger') options.ledgers.push(value());
    else if (arg === '--parse-too') options.parseToo = true;
    else if (arg === '--record') options.record = value();
    else if (arg === '--why') options.why = value();
    else if (arg.startsWith('--')) usage(`unknown option ${arg}`);
    else options.positional.push(arg);
  }
  if (options.positional.length < 2) usage('the reference and the candidate module are required');
  if ((options.record === null) !== (options.why === null)) usage('--record and --why go together');
  if (options.why !== null && options.why.trim() === '') usage('--why must state the basis');
  return options;
}

/** Whether this Node parses JSON losslessly: numbers as JSON.rawJSON of their source text. */
const lossless = typeof JSON.rawJSON === 'function' && JSON.parse('1.50', (_, v, context) => context && context.source) === '1.50';

/** The value of a JSON text, with every number as JSON.rawJSON of its source text. */
const parseJson = (text) => JSON.parse(text, (_, v, context) => (typeof v === 'number' ? JSON.rawJSON(context.source) : v));

/** Whether a parsed JSON value is an object: not null, an array or a number (JSON.rawJSON). */
const isObject = (v) => v !== null && typeof v === 'object' && !Array.isArray(v) && !JSON.isRawJSON(v);

/** JSON with the keys of every object sorted: the key of a request, and the form responses are compared in. */
const canonical = (value) => JSON.stringify(value, (_, v) => (isObject(v) ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, v[k]])) : v));

const WHITESPACE = new Set([' ', '\t', '\n', '\r']);
const skipWhitespace = (text, i) => {
  while (WHITESPACE.has(text[i])) i++;
  return i;
};

/** The index just past the JSON string that starts at `i`. */
function stringEnd(text, i) {
  for (i++; text[i] !== '"'; i++) if (text[i] === '\\') i++;
  return i + 1;
}

/** The index just past the JSON value that starts at `i`, in a text that is valid JSON. */
function valueEnd(text, i) {
  if (text[i] === '"') return stringEnd(text, i);
  if (text[i] !== '[' && text[i] !== '{') {
    while (i < text.length && !WHITESPACE.has(text[i]) && !',]}'.includes(text[i])) i++;
    return i;
  }
  let depth = 0;
  do {
    if (text[i] === '"') {
      i = stringEnd(text, i);
      continue;
    }
    if (text[i] === '[' || text[i] === '{') depth++;
    else if (text[i] === ']' || text[i] === '}') depth--;
    i++;
  } while (depth > 0);
  return i;
}

/** The source texts of the items of the JSON array that starts at `i`, in a text that is valid JSON. */
function itemTexts(text, i) {
  const items = [];
  i = skipWhitespace(text, i + 1);
  while (text[i] !== ']') {
    const end = valueEnd(text, i);
    items.push(text.slice(i, end));
    i = skipWhitespace(text, end);
    if (text[i] === ',') i = skipWhitespace(text, i + 1);
  }
  return items;
}

/** The source text of member `name` of the JSON object `text` (its last one, as JSON.parse takes). */
function memberText(text, name) {
  let found;
  let i = skipWhitespace(text, 1);
  while (text[i] !== '}') {
    const keyEnd = stringEnd(text, i);
    const key = JSON.parse(text.slice(i, keyEnd));
    i = skipWhitespace(text, skipWhitespace(text, keyEnd) + 1);
    const end = valueEnd(text, i);
    if (key === name) found = text.slice(i, end);
    i = skipWhitespace(text, end);
    if (text[i] === ',') i = skipWhitespace(text, i + 1);
  }
  return found;
}

const usesClock = (request) => /\b(now|today)\s*\(/.test(JSON.stringify(request));

/** The JSON files a path stands for: the file itself, or the *.json files of a directory. */
async function jsonFiles(p) {
  let info;
  try {
    info = await stat(p);
  } catch {
    throw new InputError(`${p}: not found`);
  }
  if (!info.isDirectory()) return [p];
  return (await readdir(p)).filter((f) => f.endsWith('.json')).sort().map((f) => path.join(p, f));
}

/** The value of a JSON file and its text without the surrounding whitespace. */
async function readJson(file) {
  try {
    const text = (await readFile(file, 'utf8')).trim();
    return { content: parseJson(text), text };
  } catch (e) {
    throw new InputError(`${file}: ${e.message}`);
  }
}

/** The source units, one per file: its requests in order, each with its label and its text. */
async function loadSources(paths) {
  const units = [];
  for (const p of paths) {
    for (const file of await jsonFiles(p)) {
      const { content, text } = await readJson(file);
      const label = path.basename(file, '.json');
      const requests = Array.isArray(content) ? itemTexts(text, 0).map((item, i) => ({ label: `${label}#${i}`, request: content[i], text: item })) : [{ label, request: content, text }];
      units.push({ file, requests });
    }
  }
  return units;
}

const ENTRY_FIELDS = new Set(['expression', 'request', 'why', 'old', 'new']);

/** What a ledger entry's `old` or `new` may be: the result text, a whole response, or a trap. */
function checkOutcome(outcome, field) {
  if (typeof outcome === 'string') return;
  if (!isObject(outcome)) throw new Error(`\`${field}\` must be a string or a response object`);
  if ('trap' in outcome && canonical(outcome) !== '{"trap":true}') throw new Error(`\`${field}\`: a trap is exactly {"trap": true}`);
}

/** The registered entries by request key, and the ledger units, one per file: its requests in order. */
async function loadLedger(paths) {
  const entries = new Map();
  const units = [];
  for (const p of paths) {
    for (const file of await jsonFiles(p)) {
      const { content, text } = await readJson(file);
      if (!Array.isArray(content)) throw new InputError(`${file}: a ledger is a JSON array of entries`);
      const entryTexts = itemTexts(text, 0);
      const requests = [];
      units.push({ file, requests });
      content.forEach((entry, i) => {
        const where = `${file} entry #${i}`;
        try {
          if (!isObject(entry)) throw new Error('an entry is an object');
          const unknown = Object.keys(entry).filter((k) => !ENTRY_FIELDS.has(k));
          if (unknown.length) throw new Error(`unknown field ${unknown.join(', ')}`);
          if (('expression' in entry) === ('request' in entry)) throw new Error('exactly one of `expression` and `request`');
          if ('expression' in entry && typeof entry.expression !== 'string') throw new Error('`expression` must be a string');
          if (typeof entry.why !== 'string' || entry.why.trim() === '') throw new Error('`why` must state the basis');
          if (!('old' in entry) || !('new' in entry)) throw new Error('`old` and `new` are required');
          checkOutcome(entry.old, 'old');
          checkOutcome(entry.new, 'new');
          if ((typeof entry.old === 'string') !== (typeof entry.new === 'string')) throw new Error('`old` and `new` are both result texts or both responses');
          if (canonical(entry.old) === canonical(entry.new)) throw new Error('`old` and `new` are the same, so nothing differs');
        } catch (e) {
          throw new InputError(`${where}: ${e.message}`);
        }
        const request = 'expression' in entry ? { op: 'evaluate', expression: entry.expression } : entry.request;
        const requestText = 'expression' in entry ? JSON.stringify(request) : memberText(entryTexts[i], 'request');
        if (usesClock(request)) throw new InputError(`${where}: a request that calls now() or today() has no fixed answer`);
        const key = canonical(request);
        const earlier = entries.get(key);
        if (earlier) throw new InputError(`${where}: the request is already registered by ${earlier.where}`);
        entries.set(key, { where, request, text: requestText, why: entry.why, old: entry.old, new: entry.new });
        requests.push({ label: where, request, text: requestText });
      });
    }
  }
  return { entries, units };
}

/** A compiled module: a function that opens a session on a fresh instance. */
async function load(file) {
  try {
    const module = await WebAssembly.compile(await readFile(file));
    return () => session(module);
  } catch (e) {
    throw new InputError(`${file}: ${e.message}`);
  }
}

const TRAP = canonical({ trap: true });

/** A stripped response as its canonical text, the form it is compared in; its value is parsed when needed. */
class Answer {
  constructor(text) {
    this.text = text;
    this.parsed = undefined;
  }

  get value() {
    this.parsed ??= parseJson(this.text);
    return this.parsed;
  }
}

/** The answer of a session to one request: a trap, or the stripped response. A response that is not JSON throws MalformedResponse. */
async function answerOf(answer, { request, text }) {
  const response = await answer(text);
  if (typeof response !== 'string') return new Answer(TRAP);
  let value;
  try {
    value = parseJson(response);
  } catch (e) {
    throw new MalformedResponse(e.message);
  }
  return new Answer(canonical(strip(value, request)));
}

/** The response without the fields that identify the build rather than the answer. */
function strip(response, request) {
  const r = { ...response };
  delete r.elapsedMicros;
  delete r.engineCommit;
  if (request && request.op === 'info') delete r.version;
  return r;
}

/** The result text of a success: `display`, with the JSON quoting of a string removed. */
function resultText(response) {
  const result = response.ok === true && response.result;
  if (!result || typeof result.display !== 'string') return undefined;
  return result.type === 'string' ? JSON.parse(result.display) : result.display;
}

const matches = (answer, expected) => (typeof expected === 'string' ? resultText(answer.value) === expected : answer.text === canonical(expected));

/** In the result-text form, what besides the text both responses must still agree on. */
const besidesText = (response) => canonical([response.ok, response.result && response.result.type, response.warnings, response.diagnostics]);

const clip = (s) => (s.length > 400 ? `${s.slice(0, 400)}…` : s);
const short = (value) => clip(value instanceof Answer ? value.text : typeof value === 'string' ? JSON.stringify(value) : canonical(value));
const labelOf = (label, request) => `${label} ${JSON.stringify(request).slice(0, 90)}`;

/**
 * The reference's answers by request key. Each unit (a source file or a ledger file) is answered in
 * order on a fresh instance, so its answers depend on the module and the unit alone; a request's
 * answer is the one of the first unit that lists it. A response that is not JSON is kept as its
 * MalformedResponse.
 */
async function referenceAnswers(reference, units) {
  const answers = new Map();
  for (const unit of units) {
    const answer = await reference();
    for (const item of unit.requests) {
      let outcome;
      try {
        outcome = await answerOf(answer, item);
      } catch (e) {
        if (!(e instanceof MalformedResponse)) throw e;
        outcome = e;
      }
      if (!answers.has(item.key)) answers.set(item.key, outcome);
    }
  }
  return answers;
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (!lossless) throw new InputError('this Node does not parse JSON losslessly (JSON.rawJSON and the source text of a number, Node 21 or later)');
  const [refPath, candPath, ...sourcePaths] = options.positional;
  const { entries: ledger, units: ledgerUnits } = await loadLedger(options.ledgers);
  const sourceUnits = await loadSources(sourcePaths);
  // With --parse-too, every evaluate request of a unit is also sent as a parse request, the same
  // request with `op` set to "parse".
  const twinsOf = ({ requests }) =>
    !options.parseToo
      ? []
      : requests
          .filter(({ request }) => request && request.op === 'evaluate')
          .map(({ label, request }) => {
            const parse = { ...request, op: 'parse' };
            return { label: `${label} (as parse)`, request: parse, text: JSON.stringify(parse) };
          });
  const twins = sourceUnits.map(twinsOf);
  const sourced = [...sourceUnits.flatMap((unit) => unit.requests), ...twins.flat()];
  for (const item of [...sourced, ...ledgerUnits.flatMap((unit) => unit.requests)]) item.key = canonical(item.request);

  // The work list: the requests of every source unit, then their parse twins, then the requests
  // only the ledger lists; the first occurrence of a request wins.
  const work = new Map();
  let excluded = 0;
  for (const item of sourced) {
    if (usesClock(item.request)) excluded++;
    else if (!work.has(item.key)) work.set(item.key, item);
  }
  const fromSources = work.size;
  for (const [key, entry] of ledger) if (!work.has(key)) work.set(key, { label: entry.where, request: entry.request, text: entry.text, key });
  // What the reference answers: each source unit with its parse twins, then each ledger unit.
  const units = [...sourceUnits.map((unit, i) => ({ file: unit.file, requests: [...unit.requests, ...twins[i]].filter(({ request }) => !usesClock(request)) })), ...ledgerUnits];

  const reference = await load(refPath);
  const candidate = await load(candPath);
  const refAnswers = await referenceAnswers(reference, units);
  const cand = await candidate();
  const failures = [];
  const recorded = [];
  let reproduced = 0;
  let trapped = 0;
  for (const [key, item] of work) {
    const { label, request } = item;
    const a = refAnswers.get(key);
    let b;
    try {
      if (a instanceof MalformedResponse) throw a;
      b = await answerOf(cand, item);
    } catch (e) {
      if (!(e instanceof MalformedResponse)) throw e;
      failures.push(`MALFORMED ${labelOf(label, request)}: the response is not JSON (${e.message})`);
      continue;
    }
    if (a.text === TRAP) trapped++;
    const entry = ledger.get(key);
    if (entry) {
      const problems = [];
      if (!matches(a, entry.old)) problems.push(`  reference answered ${short(a)}\n  registered old     ${short(entry.old)}`);
      if (!matches(b, entry.new)) problems.push(`  candidate answered ${short(b)}\n  registered new     ${short(entry.new)}`);
      if (!problems.length && typeof entry.old === 'string' && besidesText(a.value) !== besidesText(b.value)) {
        problems.push(`  the responses differ beyond the result text\n  ref : ${short(a)}\n  cand: ${short(b)}`);
      }
      if (problems.length) failures.push(`NOT REPRODUCED ${entry.where} ${JSON.stringify(request).slice(0, 90)}\n${problems.join('\n')}`);
      else reproduced++;
    } else if (a.text !== b.text) {
      failures.push(`DIFF ${labelOf(label, request)}\n  ref : ${short(a)}\n  cand: ${short(b)}`);
      if (options.record) {
        const isPlain = request && request.op === 'evaluate' && canonical(Object.keys(request).sort()) === '["expression","op"]';
        recorded.push({ ...(isPlain ? { expression: request.expression } : { request }), why: options.why, old: a.value, new: b.value });
      }
    }
  }

  const shown = 20;
  for (const f of failures.slice(0, shown)) console.log(f);
  if (failures.length > shown) console.log(`… and ${failures.length - shown} more`);
  if (options.record) {
    await writeFile(options.record, `${JSON.stringify(recorded, null, 2)}\n`);
    console.log(`recorded ${recorded.length} differences in ${options.record}`);
  }
  const count = (kind) => failures.filter((f) => f.startsWith(kind)).length;
  console.log(
    `${work.size} requests (${fromSources} from sources, ${excluded} left out for now()/today(), ${ledger.size} registered, ${trapped} trapping the reference): ` +
      `${reproduced} registered differences reproduced, ${count('DIFF')} unregistered differences, ` +
      `${count('NOT REPRODUCED')} registered entries not reproduced, ${count('MALFORMED')} malformed responses`,
  );
  return failures.length ? 1 : 0;
}

// exitCode rather than exit(): output to a pipe is written asynchronously on some platforms
main().then(
  (status) => {
    process.exitCode = status;
  },
  (e) => {
    console.error(e instanceof InputError ? `error: ${e.message}` : e);
    process.exitCode = 2;
  },
);
