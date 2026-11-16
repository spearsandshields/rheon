#!/usr/bin/env node
// Full-response identity of two feel.wasm modules, with a ledger of registered differences.
//
//   node equal.mjs <reference.wasm> <candidate.wasm> [source…] [--ledger <file|dir>]… [--exclude <file|dir>]…
//                  [--parse-too] [--record <out.json> --why <basis>] [--explained <out.json>]
//                  [--cache <dir> [--verify-cache]]
//
// A source is a JSON file holding one request or an array of requests, or a directory whose *.json
// files are such sources. Every request goes through both modules (raw ABI, node:wasi; a fresh
// instance after a trap), byte for byte as it is written in its file. Requests that call now() or
// today() are left out: their answers depend on the clock. With --parse-too, every evaluate
// request is also sent as a parse request, the same request with `op` set to "parse". --exclude
// names a source whose requests (with their parse twins) are left out of every source and
// counted in the summary; a ledger entry is still run.
//
// JSON is read losslessly: a number keeps its source text, so a number beyond the range or the
// precision of a double is sent, keyed and compared as written, in requests, responses and the
// ledger alike. The run fails with status 2 on a Node without JSON.rawJSON and the source text
// of a parsed number.
//
// A ledger is a JSON array of registered differences (README.md); a directory adds its *.json
// files. Every ledger entry is run even if no source lists its request: the reference must answer
// `old` and the candidate `new`. A rule entry (rules.mjs) explains a class of differences; its
// examples are run like entries, and a difference of a request with no entry of its own is
// explained by the first rule, in ledger order, that it is an instance of. Any other difference
// fails the run.
//
// --record writes the unregistered differences of the run to <out.json> as ledger entries in the
// full-response form, each with the given `why`, as a draft to check against the spec; a
// response whose canonical text is longer than 65,536 characters is written as the sha256 of that
// text. A difference that a rule explains is not one of them. --explained writes every difference
// that a rule explains, with the rule's name, to <out.json> for review.
//
// --cache keeps the reference's answers to each source file and ledger file in <dir> and reuses
// them while the module, the file, the options and the host are the same (README.md, "The cache
// of reference answers"). A cached trap is evaluated again on every run. --verify-cache evaluates
// the reference in full, fails on any cached answer that differs from the live one and rewrites
// the cache.
//
// Exit status: 0 when every response is identical or registered, 1 on a difference, a ledger entry
// that does not reproduce, a malformed response or a cache mismatch, 2 on a usage or input error.
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, rename, rm, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { MalformedResponse, session } from '../../scripts/feel-abi.mjs';
import { canonical, isObject, lossless, parseJson } from './json.mjs';
import { checkRule, explainedBy } from './rules.mjs';

class InputError extends Error {}

function usage(message) {
  throw new InputError(`${message}\nusage: node equal.mjs <reference.wasm> <candidate.wasm> [source…] [--ledger <file|dir>]… [--exclude <file|dir>]… [--parse-too] [--record <out.json> --why <basis>] [--explained <out.json>] [--cache <dir> [--verify-cache]]`);
}

function parseArgs(argv) {
  const options = { positional: [], ledgers: [], excludes: [], parseToo: false, record: null, why: null, explained: null, cache: null, verifyCache: false };
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    const value = () => (i + 1 < argv.length ? argv[++i] : usage(`${arg} needs a value`));
    if (arg === '--ledger') options.ledgers.push(value());
    else if (arg === '--exclude') options.excludes.push(value());
    else if (arg === '--parse-too') options.parseToo = true;
    else if (arg === '--record') options.record = value();
    else if (arg === '--why') options.why = value();
    else if (arg === '--explained') options.explained = value();
    else if (arg === '--cache') options.cache = value();
    else if (arg === '--verify-cache') options.verifyCache = true;
    else if (arg.startsWith('--')) usage(`unknown option ${arg}`);
    else options.positional.push(arg);
  }
  if (options.positional.length < 2) usage('the reference and the candidate module are required');
  if ((options.record === null) !== (options.why === null)) usage('--record and --why go together');
  if (options.why !== null && options.why.trim() === '') usage('--why must state the basis');
  if (options.verifyCache && options.cache === null) usage('--verify-cache needs --cache');
  return options;
}

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

const sha256 = (...parts) => {
  const hash = createHash('sha256');
  for (const part of parts) hash.update(part);
  return hash.digest('hex');
};

/** The value of a JSON file, its text without the surrounding whitespace and the sha256 of its bytes. */
async function readJson(file) {
  try {
    const bytes = await readFile(file);
    const text = bytes.toString('utf8').trim();
    return { content: parseJson(text), text, sha256: sha256(bytes) };
  } catch (e) {
    throw new InputError(`${file}: ${e.message}`);
  }
}

/** The source units, one per file: its requests in order, each with its label and its text. */
async function loadSources(paths) {
  const units = [];
  for (const p of paths) {
    for (const file of await jsonFiles(p)) {
      const { content, text, sha256 } = await readJson(file);
      const label = path.basename(file, '.json');
      const requests = Array.isArray(content) ? itemTexts(text, 0).map((item, i) => ({ label: `${label}#${i}`, request: content[i], text: item })) : [{ label, request: content, text }];
      units.push({ file, sha256, mode: 'requests', requests });
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
  if ('sha256' in outcome && !isDigest(outcome)) throw new Error(`\`${field}\`: a digest is exactly {"sha256": <64 lowercase hex digits>}`);
}

/** Whether a ledger outcome is the digest form, `{"sha256": …}`: the sha256 of the response's canonical text. */
const isDigest = (outcome) => Object.keys(outcome).length === 1 && typeof outcome.sha256 === 'string' && /^[0-9a-f]{64}$/.test(outcome.sha256);

/**
 * The ledger: the registered entries by request key, the rule entries in ledger order, and the
 * ledger units, one per file: its requests in order. The examples of a rule entry are registered
 * like request entries, each with the rule's name.
 */
async function loadLedger(paths) {
  const entries = new Map();
  const rules = [];
  const units = [];
  for (const p of paths) {
    for (const file of await jsonFiles(p)) {
      const { content, text, sha256 } = await readJson(file);
      if (!Array.isArray(content)) throw new InputError(`${file}: a ledger is a JSON array of entries`);
      const entryTexts = itemTexts(text, 0);
      const requests = [];
      units.push({ file, sha256, mode: 'ledger', requests });
      // An entry or an example: its request, keyed and registered at most once across all ledger files.
      const register = (where, entry, entryText, fields) => {
        const request = 'expression' in entry ? { op: 'evaluate', expression: entry.expression } : entry.request;
        const requestText = 'expression' in entry ? JSON.stringify(request) : memberText(entryText, 'request');
        if (usesClock(request)) throw new InputError(`${where}: a request that calls now() or today() has no fixed answer`);
        const key = canonical(request);
        const earlier = entries.get(key);
        if (earlier) throw new InputError(`${where}: the request is already registered by ${earlier.where}`);
        entries.set(key, { where, request, text: requestText, old: entry.old, new: entry.new, ...fields });
        requests.push({ label: where, request, text: requestText });
      };
      content.forEach((entry, i) => {
        const where = `${file} entry #${i}`;
        if (isObject(entry) && 'rule' in entry) {
          let rule;
          try {
            rule = checkRule(entry);
          } catch (e) {
            throw new InputError(`${where}: ${e.message}`);
          }
          const earlier = rules.find((r) => r.name === rule.name);
          if (earlier) throw new InputError(`${where}: the rule ${rule.name} is already stated by ${earlier.where}`);
          rules.push({ ...rule, where });
          const exampleTexts = itemTexts(memberText(entryTexts[i], 'examples'), 0);
          entry.examples.forEach((example, j) => register(`${where} example #${j}`, example, exampleTexts[j], { rule: rule.name }));
          return;
        }
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
        register(where, entry, entryTexts[i], { why: entry.why });
      });
    }
  }
  return { entries, rules, units };
}

/** A compiled module: the sha256 of its file, and `open`, which opens a session on a fresh instance. */
async function load(file) {
  try {
    const bytes = await readFile(file);
    const module = await WebAssembly.compile(bytes);
    return { sha256: sha256(bytes), open: () => session(module) };
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

const matches = (answer, expected) =>
  typeof expected === 'string' ? resultText(answer.value) === expected : isDigest(expected) ? sha256(answer.text) === expected.sha256 : answer.text === canonical(expected);

/** The longest response that --record writes whole; a longer one is written as its digest. */
const RECORDED_WHOLE = 64 * 1024;

/** A response as --record writes it: whole, or as the digest of its canonical text when it is long. */
const recordedOutcome = (answer) => (answer.text.length > RECORDED_WHOLE ? { sha256: sha256(answer.text) } : answer.value);

/** In the result-text form, what besides the text both responses must still agree on. */
const besidesText = (response) => canonical([response.ok, response.result && response.result.type, response.warnings, response.diagnostics]);

/** A request as a ledger entry names it: by its expression when it is a plain evaluate request, otherwise whole. */
function asEntry(request) {
  const isPlain = request && request.op === 'evaluate' && canonical(Object.keys(request).sort()) === '["expression","op"]';
  return isPlain ? { expression: request.expression } : { request };
}

const clip = (s) => (s.length > 400 ? `${s.slice(0, 400)}…` : s);
const short = (value) => clip(value instanceof Answer ? value.text : typeof value === 'string' ? JSON.stringify(value) : canonical(value));
const labelOf = (label, request) => `${label} ${JSON.stringify(request).slice(0, 90)}`;

/** The answers of a fresh instance to the requests of a unit, in order; a response that is not JSON is its MalformedResponse. */
async function evaluateUnit(reference, unit) {
  const answer = await reference.open();
  const outcomes = [];
  for (const item of unit.requests) {
    try {
      outcomes.push(await answerOf(answer, item));
    } catch (e) {
      if (!(e instanceof MalformedResponse)) throw e;
      outcomes.push(e);
    }
  }
  return outcomes;
}

const toolFiles = ['./equal.mjs', './json.mjs', './rules.mjs', '../../scripts/feel-abi.mjs'].map((f) => fileURLToPath(new URL(f, import.meta.url)));
const isReferenceDirectory = (name) => /^[0-9a-f]{64}$/.test(name);

/**
 * The reference's answers to the units of a run, one file per unit:
 * <dir>/<reference sha256>/<16 hex of the unit path's sha256>-<unit file name>.jsonl, a header line
 * (what the answers depend on), one line per request (the sha256 of the request text and the
 * answer, or a trap) and a trailer line with the sha256 of every byte before it.
 */
class Cache {
  static async open(dir, reference, parseToo) {
    const root = path.resolve(dir);
    const own = path.join(root, reference);
    let created = false;
    try {
      await stat(own);
    } catch {
      created = true;
    }
    await mkdir(own, { recursive: true });
    // the reference changed: the answers of the others are of no use
    if (created) for (const name of (await readdir(root)).filter(isReferenceDirectory)) if (name !== reference) await rm(path.join(root, name), { recursive: true, force: true });
    const tool = sha256(...(await Promise.all(toolFiles.map((f) => readFile(f)))));
    return new Cache(own, { reference, parseToo, tool, node: process.version, arch: process.arch, execArgv: process.execArgv });
  }

  constructor(dir, run) {
    this.dir = dir;
    this.run = run;
  }

  file(unit) {
    const absolute = path.resolve(unit.file);
    return path.join(this.dir, `${sha256(absolute).slice(0, 16)}-${path.basename(absolute)}.jsonl`);
  }

  header(unit) {
    const { reference, parseToo, tool, node, arch, execArgv } = this.run;
    return { format: 2, reference, unit: unit.sha256, mode: unit.mode, parseToo, tool, node, arch, execArgv, count: unit.requests.length };
  }

  /** The cached answers of a unit: `{ outcomes }`, `{ rejected: reason }`, or `{}` when there is no file. */
  async read(unit) {
    let bytes;
    try {
      bytes = await readFile(this.file(unit));
    } catch {
      return {};
    }
    const trailerAt = bytes.lastIndexOf(10, bytes.length - 2) + 1;
    let lines, trailer;
    try {
      trailer = JSON.parse(bytes.subarray(trailerAt).toString('utf8'));
      lines = bytes.subarray(0, trailerAt).toString('utf8').split('\n').slice(0, -1).map((line) => JSON.parse(line));
    } catch {
      return { rejected: 'not a cache file' };
    }
    if (!isObject(trailer) || trailer.sha256 !== sha256(bytes.subarray(0, trailerAt))) return { rejected: 'the digest does not match' };
    const [header, ...answers] = lines;
    const expected = this.header(unit);
    const changed = Object.keys(expected).find((k) => !isObject(header) || canonical(header[k]) !== canonical(expected[k]));
    if (changed !== undefined) return { rejected: `${changed} differs` };
    const valid = (line) => isObject(line) && (line.trap === true ? Object.keys(line).length === 2 : typeof line.answer === 'string');
    if (!answers.every(valid)) return { rejected: 'not a cache file' };
    if (answers.length !== unit.requests.length || answers.some((line, i) => line.request !== sha256(unit.requests[i].text))) return { rejected: 'the request list differs' };
    return { outcomes: answers.map((line) => new Answer(line.trap === true ? TRAP : line.answer)) };
  }

  /** Writes the answers of a unit: a temporary file in the same directory, then a rename. */
  async write(unit, outcomes) {
    const lines = [this.header(unit), ...outcomes.map((answer, i) => ({ request: sha256(unit.requests[i].text), ...(answer.text === TRAP ? { trap: true } : { answer: answer.text }) }))];
    const body = lines.map((line) => `${JSON.stringify(line)}\n`).join('');
    const file = this.file(unit);
    const temporary = `${file}.${process.pid}.tmp`;
    await writeFile(temporary, `${body}${JSON.stringify({ sha256: sha256(body) })}\n`);
    await rename(temporary, file);
  }
}

/**
 * The reference's answers by request key. Each unit (a source file or a ledger file) is answered in
 * order on a fresh instance, so its answers depend on the module and the unit alone; a request's
 * answer is the one of the first unit that lists it. A response that is not JSON is kept as its
 * MalformedResponse, and the unit is then not cached.
 *
 * With a cache, a unit's cached answers are used when the file matches; each cached trap is then
 * evaluated again on a fresh instance, and a cached trap that answers makes the file stale. With
 * `verify`, every unit is evaluated and its cached answers are compared with the live ones.
 */
async function referenceAnswers(reference, units, cache, verify, log) {
  const answers = new Map();
  const counts = { cached: 0, evaluated: 0 };
  const mismatches = [];
  for (const unit of units) {
    const cached = cache ? await cache.read(unit) : {};
    let outcomes = cached.outcomes;
    if (outcomes && !verify) {
      for (const [i, answer] of outcomes.entries()) {
        if (answer.text !== TRAP) continue;
        counts.evaluated++;
        let live;
        try {
          live = await answerOf(await reference.open(), unit.requests[i]);
        } catch (e) {
          if (!(e instanceof MalformedResponse)) throw e;
          live = e;
        }
        if (live instanceof MalformedResponse || live.text !== TRAP) {
          cached.rejected = 'a cached trap is answered now';
          outcomes = undefined;
          break;
        }
      }
      if (outcomes) counts.cached += outcomes.filter((answer) => answer.text !== TRAP).length;
    }
    if (cached.rejected) log(`cache: ${cache.file(unit)} rejected (${cached.rejected}), evaluating`);
    if (!outcomes || verify) {
      const live = await evaluateUnit(reference, unit);
      counts.evaluated += live.length;
      if (outcomes) {
        for (const [i, answer] of outcomes.entries()) {
          if (live[i] instanceof MalformedResponse || live[i].text !== answer.text) {
            const { label, request } = unit.requests[i];
            mismatches.push(`CACHE MISMATCH ${labelOf(label, request)}\n  cached: ${short(answer)}\n  live  : ${live[i] instanceof MalformedResponse ? 'not JSON' : short(live[i])}`);
          }
        }
      }
      outcomes = live;
      if (cache && !outcomes.some((outcome) => outcome instanceof MalformedResponse)) await cache.write(unit, outcomes);
    }
    unit.requests.forEach((item, i) => {
      if (!answers.has(item.key)) answers.set(item.key, outcomes[i]);
    });
  }
  return { answers, counts, mismatches };
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (!lossless) throw new InputError('this Node does not parse JSON losslessly (JSON.rawJSON and the source text of a number, Node 21 or later)');
  const [refPath, candPath, ...sourcePaths] = options.positional;
  const { entries: ledger, rules, units: ledgerUnits } = await loadLedger(options.ledgers);
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
  // The requests of an excluded source, with their parse twins, are left out of every source; a
  // ledger entry is run whatever its request.
  const excludedKeys = new Set((await loadSources(options.excludes)).flatMap((unit) => [...unit.requests, ...twinsOf(unit)]).map(({ request }) => canonical(request)));
  let leftOut = 0;
  const kept = (items) =>
    items.filter((item) => {
      item.key = canonical(item.request);
      if (!excludedKeys.has(item.key)) return true;
      leftOut++;
      return false;
    });
  const twins = sourceUnits.map((unit) => kept(twinsOf(unit)));
  for (const unit of sourceUnits) unit.requests = kept(unit.requests);
  const sourced = [...sourceUnits.flatMap((unit) => unit.requests), ...twins.flat()];
  for (const item of ledgerUnits.flatMap((unit) => unit.requests)) item.key = canonical(item.request);

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
  const units = [...sourceUnits.map((unit, i) => ({ ...unit, requests: [...unit.requests, ...twins[i]].filter(({ request }) => !usesClock(request)) })), ...ledgerUnits];

  const reference = await load(refPath);
  const candidate = await load(candPath);
  const cache = options.cache === null ? null : await Cache.open(options.cache, reference.sha256, options.parseToo);
  const { answers: refAnswers, counts, mismatches } = await referenceAnswers(reference, units, cache, options.verifyCache, console.log);
  const cand = await candidate.open();
  const failures = [...mismatches];
  const recorded = [];
  const explained = [];
  const byRule = new Map(rules.map((rule) => [rule.name, 0]));
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
      // A request with no entry of its own: the first rule, in ledger order, that explains it.
      const rule = explainedBy(rules, request, a.value, b.value);
      if (rule) {
        byRule.set(rule.name, byRule.get(rule.name) + 1);
        if (options.explained) explained.push({ rule: rule.name, ...asEntry(request), old: a.value, new: b.value });
        continue;
      }
      failures.push(`DIFF ${labelOf(label, request)}\n  ref : ${short(a)}\n  cand: ${short(b)}`);
      if (options.record) recorded.push({ ...asEntry(request), why: options.why, old: recordedOutcome(a), new: recordedOutcome(b) });
    }
  }

  const shown = 20;
  for (const f of failures.slice(0, shown)) console.log(f);
  if (failures.length > shown) console.log(`… and ${failures.length - shown} more`);
  if (options.explained) {
    await writeFile(options.explained, `${JSON.stringify(explained, null, 2)}\n`);
    console.log(`listed ${explained.length} explained differences in ${options.explained}`);
  }
  if (options.record) {
    await writeFile(options.record, `${JSON.stringify(recorded, null, 2)}\n`);
    console.log(`recorded ${recorded.length} differences in ${options.record}`);
  }
  const count = (kind) => failures.filter((f) => f.startsWith(kind)).length;
  console.log(
    `${work.size} requests (${fromSources} from sources, ${excluded} left out for now()/today(), ${options.excludes.length ? `${leftOut} left out by --exclude, ` : ''}${ledger.size} registered, ${trapped} trapping the reference): ` +
      `${reproduced} registered differences reproduced, ${count('DIFF')} unregistered differences, ` +
      `${count('NOT REPRODUCED')} registered entries not reproduced, ${count('MALFORMED')} malformed responses, ` +
      `${[...byRule.values()].reduce((n, m) => n + m, 0)} differences explained by ${rules.length} rules${rules.length ? ` (${[...byRule].map(([name, n]) => `${name}: ${n}`).join(', ')})` : ''}; ` +
      `reference: ${counts.cached} answers from the cache, ${counts.evaluated} evaluated${options.verifyCache ? `, ${count('CACHE MISMATCH')} cache mismatches` : ''}`,
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
