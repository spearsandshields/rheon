// Automated checks of the identity tool (equal.mjs), run on real modules:
//
//   node --test tools/identity/test.mjs        (npm run test:identity)
//
// The reference is IDENTITY_REFERENCE (default .tools/baseline/feel.wasm, see fetch-baseline.sh) and
// the candidate IDENTITY_CANDIDATE (default dist/feel.wasm, see npm run build:wasm). A missing
// module fails every test. The checks that need a difference between the two modules take the
// requests of the committed ledger (registered/); when the two modules answer all of them alike,
// those checks are skipped with a message.
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseJson } from './json.mjs';
import { checkRule, explainedBy, explains } from './rules.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../..');
const reference = path.resolve(root, process.env.IDENTITY_REFERENCE ?? '.tools/baseline/feel.wasm');
const candidate = path.resolve(root, process.env.IDENTITY_CANDIDATE ?? 'dist/feel.wasm');

let dir;
let fileCount = 0;

/** Writes `content` (a JSON text, or a value written as JSON) to a new file and returns its path. */
function file(content, name = `f${++fileCount}.json`) {
  const p = path.join(dir, name);
  writeFileSync(p, typeof content === 'string' ? content : JSON.stringify(content));
  return p;
}

/** Runs equal.mjs with `args`: its exit status and stdout. */
function equal(...args) {
  const run = spawnSync(process.execPath, [path.join(here, 'equal.mjs'), ...args], { encoding: 'utf8', maxBuffer: 64 * 2 ** 20 });
  return { status: run.status, out: run.stdout, err: run.stderr };
}

/** The summary line of a run. */
const summary = (run) => run.out.trim().split('\n').at(-1);

/** The output of a run without the counts of the reference's answers, which depend on the cache. */
const withoutCounts = (out) => out.replace(/; reference: \d+ answers from the cache, \d+ evaluated/, '');

/** Asserts a usage or input error whose message contains `message`. */
function assertInputError(run, message) {
  assert.equal(run.status, 2, run.out + run.err);
  assert.ok(run.err.includes(message), `expected "${message}" in:\n${run.err}`);
}

// Requests both modules answer, one of them with the clock (left out of every run).
const plain = [
  { op: 'evaluate', expression: '1 + 1' },
  { op: 'evaluate', expression: '"a" + "b"' },
  { op: 'evaluate', expression: 'x * 2', context: { x: 21 } },
  { op: 'evaluate', expression: '1 +' },
  { op: 'evaluate', dialect: 'unaryTests', expression: '> 5', input: 7 },
  { op: 'parse', expression: 'a + b' },
  { op: 'evaluate', expression: 'today()' },
];
// A recursion that exhausts the native stack of Node's main thread: it traps every module there.
const depth = 60;
const trapping = {
  op: 'evaluate',
  expression: `{f: function(n) if n = 0 then 0 else ${'0 + ('.repeat(depth)}1 + f(n-1)${')'.repeat(depth)}, r: f(190)}.r`,
};

/** Up to `n` plain requests of the committed ledger, in the order of its files. */
function ledgerRequests(n) {
  const ledger = path.join(here, 'registered');
  const requests = [];
  for (const f of readdirSync(ledger).filter((f) => f.endsWith('.json')).sort()) {
    for (const entry of JSON.parse(readFileSync(path.join(ledger, f), 'utf8'))) {
      if (requests.length < n && typeof entry.expression === 'string') requests.push({ op: 'evaluate', expression: entry.expression });
    }
  }
  return requests;
}

// The differences between the two modules: a source of ledger requests and the entries --record
// drafts for it (null when the modules answer them alike).
let differing;
let recorded;

before(() => {
  for (const m of [reference, candidate]) assert.ok(existsSync(m), `module ${m} not found: run tools/identity/fetch-baseline.sh and npm run build:wasm`);
  dir = mkdtempSync(path.join(os.tmpdir(), 'identity-test-'));
  // Requests of the ledger with numbers in their context that a double cannot hold: a recorded entry
  // must keep them as written.
  differing = file(ledgerRequests(4).map((r) => ({ ...r, context: { big: JSON.rawJSON('1234567890123456789012345678901234567890'), tiny: JSON.rawJSON('1.5e-6176') } })));
  const out = path.join(dir, 'recorded.json');
  const run = equal(reference, candidate, differing, '--record', out, '--why', 'the basis');
  recorded = run.status === 1 && JSON.parse(readFileSync(out, 'utf8')).length > 0 ? out : null;
});

after(() => {
  if (dir) rmSync(dir, { recursive: true, force: true });
});

const needsDifference = (t) => {
  if (recorded) return false;
  t.skip('the two modules answer every request of the committed ledger alike');
  return true;
};

test('a module against itself is identical, and a clock request is left out', () => {
  const source = file(plain);
  for (const m of [reference, candidate]) {
    const run = equal(m, m, source, '--parse-too');
    assert.equal(run.status, 0, run.out + run.err);
    assert.match(summary(run), /^11 requests \(11 from sources, 2 left out for now\(\)\/today\(\), 0 registered, 0 trapping the reference\): 0 registered differences reproduced, 0 unregistered differences/);
  }
});

test('a trap is the response {"trap": true}, and the same trap on both sides is identical', () => {
  const run = equal(reference, reference, file([trapping]));
  assert.equal(run.status, 0, run.out + run.err);
  assert.match(summary(run), /1 trapping the reference/);
  // The instance is replaced after a trap: the next request is answered.
  const entry = { expression: '1 + 1', why: 'test', old: '2', new: '3' };
  const next = equal(reference, reference, file([trapping]), '--ledger', file([entry]));
  assert.equal(next.status, 1);
  assert.match(next.out, /candidate answered .*"display":"2"/);
});

test('an unregistered difference fails the run', (t) => {
  if (needsDifference(t)) return;
  const run = equal(reference, candidate, differing);
  assert.equal(run.status, 1, run.out + run.err);
  assert.match(run.out, /^DIFF /m);
  assert.doesNotMatch(summary(run), / 0 unregistered differences/);
});

test('--record writes entries that pass as a ledger, numbers as written', (t) => {
  if (needsDifference(t)) return;
  const text = readFileSync(recorded, 'utf8');
  assert.ok(text.includes('1234567890123456789012345678901234567890') && text.includes('1.5e-6176'), text.slice(0, 400));
  const entries = JSON.parse(text);
  assert.ok(entries.every((e) => e.why === 'the basis' && 'request' in e && typeof e.old === 'object' && typeof e.new === 'object'));
  const run = equal(reference, candidate, differing, '--ledger', recorded);
  assert.equal(run.status, 0, run.out + run.err);
  assert.match(summary(run), new RegExp(`: ${entries.length} registered differences reproduced, 0 unregistered differences, 0 registered entries not reproduced`));
});

test('a registered change that one side does not make fails the run', (t) => {
  if (needsDifference(t)) return;
  // the candidate answers `old`: the change is not made
  const notMade = equal(reference, reference, '--ledger', recorded);
  assert.equal(notMade.status, 1, notMade.out + notMade.err);
  assert.match(notMade.out, /^NOT REPRODUCED /m);
  assert.match(notMade.out, /candidate answered/);
  assert.doesNotMatch(notMade.out, /reference answered/);
  // the reference answers `new`: the entry's `old` is wrong
  const wrongOld = equal(candidate, candidate, '--ledger', recorded);
  assert.equal(wrongOld.status, 1, wrongOld.out + wrongOld.err);
  assert.match(wrongOld.out, /reference answered/);
  assert.doesNotMatch(wrongOld.out, /candidate answered/);
});

test('a ledger entry with a wrong old or new fails the run; numbers are compared as written', () => {
  const wrongNew = equal(reference, reference, '--ledger', file([{ expression: '1 + 1', why: 'test', old: '2', new: '3' }]));
  assert.equal(wrongNew.status, 1);
  assert.match(wrongNew.out, /candidate answered/);
  assert.doesNotMatch(wrongNew.out, /reference answered/);
  const wrongOld = equal(reference, reference, '--ledger', file([{ expression: '1 + 1', why: 'test', old: '3', new: '2' }]));
  assert.equal(wrongOld.status, 1);
  assert.match(wrongOld.out, /reference answered/);
  assert.doesNotMatch(wrongOld.out, /candidate answered/);
  // The context holds a 40-digit integer and 1.5e-6176, sent as written: both equal the same
  // literals in the expression, so the reference answers `old`. Through a double they would not.
  const big = '1234567890123456789012345678901234567890';
  const exact = `[{"request": {"op": "evaluate", "expression": "x = ${big} and y = 1.5e-6176", "context": {"x": ${big}, "y": 1.5e-6176}}, "why": "test", "old": "true", "new": "false"}]`;
  const run = equal(reference, reference, '--ledger', file(exact));
  assert.equal(run.status, 1);
  assert.match(run.out, /candidate answered/);
  assert.doesNotMatch(run.out, /reference answered/);
  // two requests that differ beyond the precision of a double are two requests
  const twins = `[{"op": "evaluate", "expression": "x", "context": {"x": ${big}1}}, {"op": "evaluate", "expression": "x", "context": {"x": ${big}2}}]`;
  assert.match(summary(equal(reference, reference, file(twins))), /^2 requests \(2 from sources/);
});

test('a malformed ledger is an input error', () => {
  const ok = { expression: '1 + 1', why: 'test', old: '2', new: '3' };
  const cases = [
    ['{"expression": "1"}', 'a ledger is a JSON array of entries'],
    ['[1, 2', null],
    [[1], 'an entry is an object'],
    [[{ ...ok, extra: 1 }], 'unknown field extra'],
    [[{ ...ok, request: { op: 'evaluate', expression: '1' } }], 'exactly one of `expression` and `request`'],
    [[{ ...ok, expression: 1 }], '`expression` must be a string'],
    [[{ ...ok, why: ' ' }], '`why` must state the basis'],
    [[{ expression: '1', why: 'test', old: '1' }], '`old` and `new` are required'],
    [[{ ...ok, old: 2 }], '`old` must be a string or a response object'],
    [[{ ...ok, old: { trap: false }, new: { ok: true } }], 'a trap is exactly {"trap": true}'],
    [[{ ...ok, old: { ok: true } }], '`old` and `new` are both result texts or both responses'],
    [[{ ...ok, new: '2' }], '`old` and `new` are the same'],
    [[{ ...ok, expression: 'now()' }], 'a request that calls now() or today() has no fixed answer'],
  ];
  for (const [content, message] of cases) {
    const ledger = file(content);
    // a file that is not JSON is named in the message
    assertInputError(equal(reference, reference, '--ledger', ledger), message ?? ledger);
  }
  // a request is registered at most once across all ledger files
  const once = file([ok]);
  const twice = file([{ request: { expression: '1 + 1', op: 'evaluate' }, why: 'test', old: '2', new: '4' }]);
  assertInputError(equal(reference, reference, '--ledger', once, '--ledger', twice), 'the request is already registered by');
});

test('usage errors', () => {
  const source = file(plain);
  assertInputError(equal(reference), 'the reference and the candidate module are required');
  assertInputError(equal(reference, reference, source, '--record', path.join(dir, 'x.json')), '--record and --why go together');
  assertInputError(equal(reference, reference, source, '--record', path.join(dir, 'x.json'), '--why', ' '), '--why must state the basis');
  assertInputError(equal(reference, reference, source, '--frobnicate'), 'unknown option --frobnicate');
  assertInputError(equal(reference, reference, path.join(dir, 'missing.json')), 'missing.json: not found');
  assertInputError(equal(reference, path.join(dir, 'missing.wasm'), source), 'missing.wasm');
});

test('the reference answers each source on its own instance: the order of the sources does not matter', (t) => {
  if (needsDifference(t)) return;
  const [first, second] = ledgerRequests(4).reduce((halves, r, i) => (halves[i % 2].push(r), halves), [[], []]);
  const a = file([...first, plain[0], trapping]);
  const b = file([plain[0], ...second, trapping, plain[1]]);
  const forward = equal(reference, candidate, a, b, '--parse-too', '--ledger', recorded);
  const backward = equal(reference, candidate, b, a, '--parse-too', '--ledger', recorded);
  assert.equal(forward.status, backward.status);
  const lines = (run) => run.out.split('\n').filter((l) => /^(DIFF|NOT REPRODUCED|MALFORMED) /.test(l)).sort();
  assert.deepEqual(lines(forward), lines(backward));
  assert.equal(summary(forward), summary(backward));
});

// ------------------------------------------------------------------ rule entries

/** A response as the modules write it, parsed losslessly: `ok`, a boolean result, warnings and diagnostics. */
const response = ({ ok = true, display = 'false', warnings = [], count, messages = [] }) =>
  parseJson(JSON.stringify({ ok, result: { type: 'boolean', display }, warnings: warnings.map((message) => ({ message })), ...(count === undefined ? {} : { diagnostics: { count: 0, messages } }) }).replace('"count":0', `"count":${count}`));

/** A rule entry with one allowed path and one example, made from the old and the new response of the example. */
const ruleEntry = (allow, old, neu, extra = {}) => ({ rule: 'a-rule', why: 'the basis', allow: [allow], examples: [{ expression: 'x', old, new: neu }], ...extra });

test('rules: each relation explains an instance and refuses a near miss', () => {
  const lost = { at: 'diagnostics.count', change: 'decreases' };
  const messages = { at: 'diagnostics.messages', change: 'loses', each: '^\\[addition\\] ' };
  const two = response({ count: 2, messages: ['[addition] a', '[addition] b'] });
  const one = response({ count: 1, messages: ['[addition] a'] });
  const none = response({});
  const rule = checkRule({ rule: 'fewer', why: 'w', allow: [lost, messages], examples: [{ expression: 'x', old: two, new: one }] });
  assert.equal(explains(rule, two, one), true);
  // an absent field is 0 and the empty list; the object it emptied compares equal to an absent one
  assert.equal(explains(rule, two, none), true);
  // a lost element that does not match `each`
  assert.match(explains(rule, response({ count: 2, messages: ['[addition] a', 'other'] }), one), /does not match `each`/);
  // the count must decrease, strictly
  assert.match(explains(rule, one, response({ count: 1, messages: [] })), /1 to 1 is not a decrease/);
  // the kept elements stay in order
  assert.match(explains(rule, response({ count: 2, messages: ['[addition] a', '[addition] b'] }), response({ count: 1, messages: ['[addition] c'] })), /does not match|subsequence/);
  // a second field that differs
  assert.match(explains(rule, two, response({ display: 'true', count: 1, messages: ['[addition] a'] })), /differ beyond the allowed paths/);
  // a changed `ok`
  assert.match(explains(rule, two, { ...one, ok: false }), /differ beyond the allowed paths/);
  // a trap on either side, and a value that is no response
  assert.match(explains(rule, { trap: true }, one), /trap/);
  assert.match(explains(rule, two, { trap: true }), /trap/);
  assert.match(explains(rule, 'false', one), /must be responses/);

  const gains = checkRule(ruleEntry({ at: 'warnings', change: 'gains', each: '^\\{"message":"truncated' }, response({}), response({ warnings: ['truncated 1'] })));
  assert.equal(explains(gains, response({ warnings: ['w'] }), response({ warnings: ['truncated 1', 'w', 'truncated 2'] })), true);
  assert.match(explains(gains, response({ warnings: ['w'] }), response({ warnings: ['w', 'rounded'] })), /does not match `each`/);
  assert.match(explains(gains, response({ warnings: ['w'] }), response({ warnings: [] })), /subsequence/);

  const more = checkRule(ruleEntry({ at: 'diagnostics.count', change: 'increases' }, response({ count: 1 }), response({ count: 2 })));
  assert.equal(explains(more, response({ count: 0 }), response({ count: 3 })), true);
  assert.match(explains(more, response({ count: 3 }), response({ count: 2 })), /not an increase/);
  // integers are compared exactly from their source text, beyond the precision of a double
  const big = (n) => parseJson(`{"ok":true,"diagnostics":{"count":${n},"messages":[]}}`);
  assert.equal(explains(more, big('9007199254740993'), big('9007199254740994')), true);
  assert.match(explains(more, big('1.5'), big('2')), /must be integers/);

  const rewrite = { at: 'warnings[*].message', change: 'rewrite', pattern: 'rounded to (\\S+)', replacement: 'rounded: $1' };
  const rewritten = checkRule(ruleEntry(rewrite, response({ warnings: ['rounded to 2'] }), response({ warnings: ['rounded: 2'] })));
  assert.equal(explains(rewritten, response({ warnings: ['a', 'rounded to 1'] }), response({ warnings: ['a', 'rounded: 1'] })), true);
  assert.match(explains(rewritten, response({ warnings: ['rounded to 1'] }), response({ warnings: ['rounded: 2'] })), /not the new text/);
  assert.match(explains(rewritten, response({ warnings: ['rounded to 1'] }), response({ warnings: ['rounded: 1', 'b'] })), /equal length/);
  const display = checkRule(ruleEntry({ at: 'result.display', change: 'rewrite', pattern: '^false$', replacement: 'true' }, response({}), response({ display: 'true' })));
  assert.equal(explains(display, response({}), response({ display: 'true' })), true);
  assert.match(explains(display, response({ display: 'null' }), response({ display: 'true' })), /not the new text/);
});

test('rules: the first rule in ledger order that applies and explains a difference counts it', () => {
  const old = response({ count: 1, messages: ['m'] });
  const neu = response({});
  const lost = { at: 'diagnostics.count', change: 'decreases' };
  const messages = { at: 'diagnostics.messages', change: 'loses', each: '.' };
  const make = (name, allow, applies, example = { old, new: neu }) => checkRule({ rule: name, why: 'w', allow, examples: [{ expression: 'x and y', ...example }], ...(applies ? { applies: { expression: applies } } : {}) });
  const onlyCount = make('only-count', [lost], undefined, { old: response({ count: 2 }), new: response({ count: 1 }) });
  const ands = make('ands', [lost, messages], ' and ');
  const any = make('any', [lost, messages]);
  const request = (expression) => ({ op: 'evaluate', expression });
  assert.equal(explainedBy([onlyCount, ands, any], request('x and y'), old, neu).name, 'ands');
  assert.equal(explainedBy([onlyCount, ands, any], request('x or y'), old, neu).name, 'any');
  assert.equal(explainedBy([any, ands], request('x and y'), old, neu).name, 'any');
  assert.equal(explainedBy([onlyCount, ands], request('x or y'), old, neu), undefined);
  // a request without an expression never matches `applies`
  assert.equal(explainedBy([ands], { op: 'info' }, old, neu), undefined);
});

test('rules: a malformed rule entry is refused with its reason', () => {
  const old = response({ count: 1 });
  const neu = response({ count: 0 });
  const lost = { at: 'diagnostics.count', change: 'decreases' };
  const cases = [
    [{ ...ruleEntry(lost, old, neu), extra: 1 }, 'unknown field extra'],
    [{ ...ruleEntry(lost, old, neu), rule: 'Not A Name' }, '`rule` must be a name'],
    [{ ...ruleEntry(lost, old, neu), why: ' ' }, '`why` must state the basis'],
    [{ ...ruleEntry(lost, old, neu), allow: [] }, '`allow` must list at least one path'],
    [ruleEntry({ at: 'diagnostics.count', change: 'grows' }, old, neu), '`change` must be one of'],
    [ruleEntry({ at: 'diagnostics.messages', change: 'loses' }, old, neu), '`each` must be a regular expression'],
    [ruleEntry({ ...lost, each: 'x' }, old, neu), '`decreases` takes no each'],
    [ruleEntry({ at: 'diagnostics..count', change: 'decreases' }, old, neu), 'is not a path'],
    [ruleEntry({ at: 'warnings', change: 'gains', each: '(' }, old, neu), '`each`: Invalid regular expression'],
    [ruleEntry({ at: 'warnings', change: 'rewrite', pattern: 'a' }, old, neu), '`replacement` must be a string'],
    [{ ...ruleEntry(lost, old, neu), allow: [lost, lost] }, 'a path is allowed at most once'],
    [{ ...ruleEntry(lost, old, neu), examples: [] }, '`examples` must hold at least one example'],
    [{ ...ruleEntry(lost, old, neu), examples: [{ expression: 'x', old: '1', new: '2' }] }, 'whole responses'],
    [{ ...ruleEntry(lost, old, neu), applies: { expression: '^y' } }, 'does not match `applies`'],
    [{ ...ruleEntry(lost, old, neu), applies: { text: 'x' } }, '`applies` is {"expression": "<regex>"}'],
    [ruleEntry(lost, neu, old), 'not an instance of the rule'],
    [ruleEntry(lost, old, old), 'not an instance of the rule'],
  ];
  for (const [entry, message] of cases) assert.throws(() => checkRule(entry), (e) => e.message.includes(message), message);
});

test('rule entries in a ledger: examples are run like entries, and a malformed rule is an input error', () => {
  // the candidate is the reference, so it does not answer the example's `new`
  const old = { ok: true, result: { type: 'boolean', display: 'false', feel: 'false', isNull: false, json: false }, warnings: [], diagnostics: { count: 1, messages: ['m'] } };
  const neu = { ok: true, result: { type: 'boolean', display: 'false', feel: 'false', isNull: false, json: false }, warnings: [] };
  const lost = { at: 'diagnostics.count', change: 'decreases' };
  const messages = { at: 'diagnostics.messages', change: 'loses', each: '.' };
  const rule = { rule: 'fewer-errors', why: 'test', allow: [lost, messages], examples: [{ expression: 'false and (1 + "a")', old, new: neu }] };
  const notReproduced = equal(reference, reference, '--ledger', file([rule]));
  assert.equal(notReproduced.status, 1, notReproduced.out + notReproduced.err);
  assert.match(notReproduced.out, /^NOT REPRODUCED \S+ entry #0 example #0 /m);
  assert.match(summary(notReproduced), /1 registered, .*0 differences explained by 1 rules \(fewer-errors: 0\)/);
  // a rule that is not well formed, an example that is no instance of its rule, a rule stated twice,
  // and an example whose request another entry registers
  assertInputError(equal(reference, reference, '--ledger', file([{ ...rule, allow: [] }])), 'entry #0: `allow` must list at least one path');
  assertInputError(equal(reference, reference, '--ledger', file([{ ...rule, examples: [{ ...rule.examples[0], old: neu, new: old }] }])), 'entry #0: example #0: not an instance of the rule');
  const other = { ...rule, examples: [{ expression: 'true or (1 + "a")', old, new: neu }] };
  assertInputError(equal(reference, reference, '--ledger', file([rule, other])), 'entry #1: the rule fewer-errors is already stated by');
  assertInputError(equal(reference, reference, '--ledger', file([rule, { expression: 'false and (1 + "a")', why: 'test', old: '1', new: '2' }])), 'entry #1: the request is already registered by');
});

// ------------------------------------------------------------------ the cache of reference answers

/** A new, empty cache directory. */
const cacheDir = () => path.join(dir, `cache${++fileCount}`);

/** The cache files under a cache directory, with the reference directories they are in. */
function cacheFiles(cache) {
  const references = readdirSync(cache);
  return { references, files: references.flatMap((r) => readdirSync(path.join(cache, r)).map((f) => path.join(cache, r, f))) };
}

/** Rewrites the lines of a cache file (header, then one line per request) with `edit`, and signs it again. */
function resign(cacheFile, edit) {
  const lines = readFileSync(cacheFile, 'utf8').split('\n').slice(0, -2).map((line) => JSON.parse(line));
  const body = edit(lines).map((line) => `${JSON.stringify(line)}\n`).join('');
  writeFileSync(cacheFile, `${body}${JSON.stringify({ sha256: createHash('sha256').update(body).digest('hex') })}\n`);
}

/**
 * Runs equal.mjs three times, without a cache, with a cold one and with the warm one, and asserts
 * the same exit status, the same output apart from the reference's counts and the same --record
 * draft. Returns the three runs.
 */
function sameWithCache(args, { record = false } = {}) {
  const cache = cacheDir();
  const draft = path.join(dir, `draft${++fileCount}.json`);
  const runs = [[], ['--cache', cache], ['--cache', cache]].map((extra) => {
    const run = equal(...args, ...(record ? ['--record', draft, '--why', 'the basis'] : []), ...extra);
    return { ...run, draft: record ? readFileSync(draft, 'utf8') : null };
  });
  for (const run of runs.slice(1)) {
    assert.equal(run.status, runs[0].status, run.out + run.err);
    assert.equal(withoutCounts(run.out), withoutCounts(runs[0].out));
    assert.equal(run.draft, runs[0].draft);
  }
  assert.match(summary(runs[0]), /; reference: 0 answers from the cache, \d+ evaluated$/);
  assert.match(summary(runs[1]), /; reference: 0 answers from the cache, \d+ evaluated$/);
  return runs;
}

test('a cached run answers as an uncached one: identical modules, traps, wrong entries', () => {
  const [, , warm] = sameWithCache([reference, reference, file(plain), '--parse-too']);
  assert.equal(warm.status, 0);
  assert.match(summary(warm), /; reference: 11 answers from the cache, 0 evaluated$/);
  // a cached trap is evaluated again on every run
  const [, , trap] = sameWithCache([reference, reference, file([trapping, plain[0]])]);
  assert.equal(trap.status, 0);
  assert.match(summary(trap), /1 trapping the reference.*; reference: 1 answers from the cache, 1 evaluated$/);
  const trapEntry = { request: trapping, why: 'test', old: { trap: true }, new: { ok: false } };
  assert.equal(sameWithCache([reference, reference, '--ledger', file([trapEntry])])[2].status, 1);
  assert.equal(sameWithCache([reference, reference, '--ledger', file([{ expression: '1 + 1', why: 'test', old: '3', new: '2' }])])[2].status, 1);
  assert.equal(sameWithCache([reference, reference, '--ledger', file([{ expression: '1 + 1', why: 'test', old: '2', new: '3' }])])[2].status, 1);
});

test('a cached run answers as an uncached one: differences and --record', (t) => {
  if (needsDifference(t)) return;
  const [, , warm] = sameWithCache([reference, candidate, differing], { record: true });
  assert.equal(warm.status, 1);
  assert.match(summary(warm), /; reference: [1-9]\d* answers from the cache/);
  assert.equal(sameWithCache([reference, candidate, differing, '--ledger', recorded])[2].status, 0);
});

test('a cache file that does not match is rejected and evaluated again', () => {
  const cache = cacheDir();
  const source = file(plain.slice(0, 3));
  const run = (...extra) => equal(reference, reference, source, '--cache', cache, ...extra);
  const expectRejected = (r, reason, fromCache = 0) => {
    assert.equal(r.status, 0, r.out + r.err);
    assert.match(r.out, new RegExp(`^cache: \\S+ rejected \\(${reason}\\), evaluating$`, 'm'));
    assert.match(summary(r), new RegExp(`; reference: ${fromCache} answers from the cache`));
  };
  assert.equal(run().status, 0);
  const [cacheFile] = cacheFiles(cache).files;
  // a flipped byte
  writeFileSync(cacheFile, readFileSync(cacheFile, 'utf8').replace('"display\\":\\"2\\"', '"display\\":\\"3\\"'));
  expectRejected(run(), 'the digest does not match');
  assert.match(summary(run()), /; reference: 3 answers from the cache, 0 evaluated$/);
  // a cached trap that the module answers
  resign(cacheFile, ([header, first, ...rest]) => [header, { request: first.request, trap: true }, ...rest]);
  expectRejected(run(), 'a cached trap is answered now');
  // another --parse-too
  expectRejected(run('--parse-too'), 'parseToo differs');
  // an edited source
  writeFileSync(source, JSON.stringify(plain.slice(0, 4)));
  expectRejected(run(), 'unit differs');
  // not a cache file
  writeFileSync(cacheFile, 'x\n');
  expectRejected(run(), 'not a cache file');
  // another reference: its own directory, and the directory of the old one is removed
  assert.equal(equal(candidate, candidate, source, '--cache', cache).status, 0);
  const { references } = cacheFiles(cache);
  assert.deepEqual(references, [createHash('sha256').update(readFileSync(candidate)).digest('hex')]);
});

test('--verify-cache finds a cached answer that differs from the live one', () => {
  const cache = cacheDir();
  const source = file(plain.slice(0, 3));
  assert.equal(equal(reference, reference, source, '--cache', cache).status, 0);
  const [cacheFile] = cacheFiles(cache).files;
  // a wrong answer with a valid digest is used as it is...
  resign(cacheFile, ([header, first, ...rest]) => [header, { ...first, answer: first.answer.replaceAll('"2', '"3') }, ...rest]);
  const misled = equal(reference, reference, source, '--cache', cache);
  assert.equal(misled.status, 1);
  assert.match(misled.out, /^DIFF /m);
  // ...until --verify-cache evaluates it, fails and rewrites the file
  const verified = equal(reference, reference, source, '--cache', cache, '--verify-cache');
  assert.equal(verified.status, 1, verified.out + verified.err);
  assert.match(verified.out, /^CACHE MISMATCH /m);
  assert.match(summary(verified), /; reference: 0 answers from the cache, 3 evaluated, 1 cache mismatches$/);
  const again = equal(reference, reference, source, '--cache', cache, '--verify-cache');
  assert.equal(again.status, 0, again.out + again.err);
  assert.match(summary(again), /, 0 cache mismatches$/);
  assertInputError(equal(reference, reference, source, '--verify-cache'), '--verify-cache needs --cache');
});

// ------------------------------------------------------------------ bisect.sh

test('bisect.sh tells a module that makes the recorded changes from one that does not', (t) => {
  if (needsDifference(t)) return;
  const cache = cacheDir();
  const bisect = (...args) => spawnSync('bash', [path.join(here, 'bisect.sh'), ...args], { encoding: 'utf8', env: { ...process.env, IDENTITY_CACHE: cache } });
  const bad = bisect(reference, recorded, candidate);
  assert.equal(bad.status, 1, bad.stdout + bad.stderr);
  assert.match(bad.stdout, /^bisect: bad, every entry reproduces$/m);
  const good = bisect(reference, recorded, reference);
  assert.equal(good.status, 0, good.stdout + good.stderr);
  assert.match(good.stdout, /^bisect: good, not every entry reproduces$/m);
  // a module that cannot be loaded is skipped, a usage error ends the bisect
  assert.equal(bisect(reference, recorded, path.join(dir, 'missing.wasm')).status, 125);
  assert.equal(bisect(reference).status, 128);
});
