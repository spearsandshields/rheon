// Checks of the TCK summary (scripts/tck-summary.mjs) on small result files:
//
//   node --test scripts/tck-summary.test.mjs        (npm run test:tck-summary)
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const summary = fileURLToPath(new URL('./tck-summary.mjs', import.meta.url));
let dir;
let count = 0;

before(() => {
  dir = mkdtempSync(path.join(os.tmpdir(), 'tck-summary-test-'));
});
after(() => {
  if (dir) rmSync(dir, { recursive: true, force: true });
});

/** A result file of the runner: one row per [folder, test, case, status]. */
function results(rows) {
  const p = path.join(dir, `results${++count}.csv`);
  writeFileSync(p, rows.map((r) => `${[...r, ''].map((f) => `"${f}"`).join(',')}\n`).join(''));
  return p;
}

/** A baseline file listing the failing tests as "folder test case". */
function baseline(failing) {
  const p = path.join(dir, `baseline${++count}.txt`);
  writeFileSync(p, `# a comment\n${failing.map((r) => r.join(' ')).join('\n')}\n`);
  return p;
}

const run = (...args) => {
  const r = spawnSync(process.execPath, [summary, ...args], { encoding: 'utf8' });
  return { status: r.status, out: r.stdout, err: r.stderr };
};

const pass = ['compliance-level-3/0001-a', '0001-a-test-01', '001', 'SUCCESS'];
const failed = ['compliance-level-3/0002-b', '0002-b-test-01', '001', 'FAILURE'];
const ignored = ['compliance-level-3/0003-c', '0003-c-test-01', '001', 'IGNORED'];
const failing = (r) => r.slice(0, 3);

test('a run equal to the baseline exits 0', () => {
  const r = run(results([pass, failed, ignored]), '--baseline', baseline([failing(failed), failing(ignored)]));
  assert.equal(r.status, 0, r.out + r.err);
  assert.match(r.out, /0 newly failing, 0 newly passing, 0 not run/);
  assert.match(r.out, /official\s+1 passed\s+2 failed\s+of 3/);
});

test('a newly failing test exits 1', () => {
  const r = run(results([pass, failed]), '--baseline', baseline([]));
  assert.equal(r.status, 1, r.out + r.err);
  assert.match(r.out, /^ {2}NEW FAIL {2}compliance-level-3\/0002-b 0002-b-test-01 001/m);
});

test('a newly passing test exits 1', () => {
  const r = run(results([pass, [...failing(failed), 'SUCCESS']]), '--baseline', baseline([failing(failed)]));
  assert.equal(r.status, 1, r.out + r.err);
  assert.match(r.out, /^ {2}FIXED {5}compliance-level-3\/0002-b/m);
});

test('a baseline test that did not run exits 1, unless the run is partial', () => {
  const base = baseline([failing(failed)]);
  const full = run(results([pass]), '--baseline', base);
  assert.equal(full.status, 1, full.out + full.err);
  assert.match(full.out, /^ {2}NOT RUN {3}compliance-level-3\/0002-b/m);
  const partial = run(results([pass]), '--baseline', base, '--partial');
  assert.equal(partial.status, 0, partial.out + partial.err);
  // a partial run still compares the tests that ran
  assert.equal(run(results([[...failing(failed), 'SUCCESS']]), '--baseline', base, '--partial').status, 1);
});

test('a result file without rows exits 1', () => {
  const r = run(results([]), '--baseline', baseline([]));
  assert.equal(r.status, 1, r.out + r.err);
  assert.match(r.out, /no rows/);
});

test('--update-baseline writes the failing tests and exits 0', () => {
  const base = baseline([]);
  const r = run(results([pass, failed, ignored]), '--baseline', base, '--update-baseline');
  assert.equal(r.status, 0, r.out + r.err);
  const lines = readFileSync(base, 'utf8').split('\n').filter((l) => l && !l.startsWith('#'));
  assert.deepEqual(lines, [failing(failed).join(' '), failing(ignored).join(' ')]);
  assert.equal(run(results([pass, failed, ignored]), '--baseline', base).status, 0);
});

test('usage errors exit 2', () => {
  const csv = results([pass]);
  assert.equal(run().status, 2);
  assert.equal(run(csv, '--frobnicate').status, 2);
  assert.equal(run(path.join(dir, 'missing.csv')).status, 2);
  assert.equal(run(csv, '--baseline', path.join(dir, 'missing.txt')).status, 2);
  assert.equal(run(csv, '--partial', '--update-baseline').status, 2);
});
