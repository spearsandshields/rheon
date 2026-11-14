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
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

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
