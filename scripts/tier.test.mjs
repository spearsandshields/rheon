// Checks of the tier runner (scripts/tier.mjs) on step tables of their own, and of the steps of the
// group and release tiers that compare the two evaluators:
//
//   node --test scripts/tier.test.mjs        (npm run test:tier)
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readdirSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { groupSteps, releaseSteps, runSteps } from './tier.mjs';

let dir;
let count = 0;
before(() => {
  dir = mkdtempSync(path.join(os.tmpdir(), 'tier-test-'));
});
after(() => {
  if (dir) rmSync(dir, { recursive: true, force: true });
});

/** Runs a step table: the exit status, the printed lines and the logs directory. */
function run(steps, options = {}) {
  const lines = [];
  const logs = path.join(dir, `logs${++count}`);
  const status = runSteps('test', steps, { logs, cwd: dir, print: (line) => lines.push(line), ...options });
  return { status, lines, out: lines.join('\n'), logs };
}

/** The result column of a step in the final table. */
const resultOf = (r, name) => r.lines.findLast((l) => l.startsWith(`${name.padEnd(28)} `)).trim().split(/\s+/).slice(3).join(' ');

const ok = (name, kind = 'run') => ({ name, kind, command: `echo ${name}` });
const failing = (name) => ({ name, kind: 'run', command: `echo the reason of ${name}; exit 3` });

test('a tier whose steps pass exits 0 and prints the table, the totals and the verdict', () => {
  const r = run([ok('compile', 'build'), ok('tests')]);
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /^step +kind +seconds +result$/m);
  assert.match(r.out, /^compile +build +\d+\.\d +ok$/m);
  assert.match(r.out, /^tests +run +\d+\.\d +ok$/m);
  assert.match(r.out, /^build total \d+\.\d s, run total \d+\.\d s, budget none \(times reported only\)$/m);
  assert.match(r.lines.at(-1), /^test tier: PASS, 2 of 2 steps passed$/);
  // one log per step, in order
  assert.deepEqual(readdirSync(r.logs), ['01-compile.log', '02-tests.log']);
});

test('a failing step fails the tier and prints the end of its log; with stopAtFailure the rest is not run', () => {
  const steps = [ok('first'), failing('second'), ok('third')];
  const going = run(steps);
  assert.equal(going.status, 1, going.out);
  assert.equal(resultOf(going, 'second'), 'FAIL');
  assert.equal(resultOf(going, 'third'), 'ok');
  assert.match(going.out, /^ {2}\| the reason of second$/m);
  assert.match(going.lines.at(-1), /^test tier: FAIL, 2 of 3 steps passed, 1 did not \(second\)$/);
  const stopping = run(steps, { stopAtFailure: true });
  assert.equal(stopping.status, 1, stopping.out);
  assert.equal(resultOf(stopping, 'third'), 'STOPPED');
  assert.match(stopping.lines.at(-1), /2 did not \(second, third\)$/);
});

test('a step whose prerequisite did not pass is skipped', () => {
  const r = run([failing('build-it'), { ...ok('use-it'), needs: ['build-it'] }, { ...ok('use-that'), needs: ['use-it'] }, ok('other')]);
  assert.equal(r.status, 1, r.out);
  assert.equal(resultOf(r, 'use-it'), 'SKIPPED');
  assert.equal(resultOf(r, 'use-that'), 'SKIPPED');
  assert.equal(resultOf(r, 'other'), 'ok');
});

test('the budget judges the run total: over it the tier fails although every step passed', () => {
  const slow = (kind) => ({ name: `slow-${kind}`, kind, command: 'sleep 0.4' });
  const over = run([slow('run'), ok('fast')], { budget: 0.2 });
  assert.equal(over.status, 1, over.out);
  assert.match(over.out, /budget 0\.2 s, OVER BUDGET$/m);
  assert.match(over.lines.at(-1), /^test tier: FAIL, 2 of 2 steps passed$/);
  // build time is not judged
  const build = run([slow('build'), ok('fast')], { budget: 0.2 });
  assert.equal(build.status, 0, build.out);
  assert.match(build.out, /run total 0\.[0-1] s, budget 0\.2 s$/m);
});

test('an input kept outside the repository: missing, it fails the step; with its --without flag the step is NOT RUN and counted', () => {
  const step = { ...ok('large-sets'), input: { env: 'TIER_TEST_SETS', flag: '--without-sets' } };
  const env = { ...process.env, TIER_TEST_SETS: '' };
  const missing = run([step, ok('other')], { env });
  assert.equal(missing.status, 1, missing.out);
  assert.match(missing.out, /^large-sets +FAIL \(needs TIER_TEST_SETS, or --without-sets\)$/m);
  const without = run([step, ok('other')], { env, without: new Set(['--without-sets']) });
  assert.equal(without.status, 0, without.out);
  assert.equal(resultOf(without, 'large-sets'), 'NOT RUN');
  assert.match(without.lines.at(-1), /^test tier: PASS, 1 of 2 steps passed, 1 not run \(--without-sets\)$/);
  const given = run([step], { env: { ...env, TIER_TEST_SETS: '/sets' } });
  assert.equal(resultOf(given, 'large-sets'), 'ok');
});

test('a step runs with its own environment added', () => {
  const r = run([{ name: 'env', kind: 'run', command: 'test "$TIER_TEST_VALUE" = 42', env: { TIER_TEST_VALUE: '42' } }]);
  assert.equal(r.status, 0, r.out);
});

test('usage errors exit 2', () => {
  const tier = fileURLToPath(new URL('./tier.mjs', import.meta.url));
  const cli = (...args) => spawnSync(process.execPath, [tier, ...args], { encoding: 'utf8' });
  for (const args of [[], ['nope'], ['fix', '--frobnicate'], ['fix', '-p'], ['group', '-p', 'x'], ['group', '--without-large-sets'], ['release', '--test', 'x']]) {
    const r = cli(...args);
    assert.equal(r.status, 2, `${args.join(' ')}: ${r.stdout}${r.stderr}`);
    assert.match(r.stderr, /^usage: /m);
  }
});

test('the group tier compares the two evaluators, and the release tier at full size', () => {
  const step = (steps, name) => steps.find((s) => s.name === name) ?? assert.fail(`no step ${name}`);
  const group = groupSteps();
  const backends = step(group, 'backends');
  assert.match(backends.command, /cargo test -p rheon-feel-wasm --test backends -- --ignored --nocapture$/);
  assert.match(backends.env.RHEON_PARSER_CORPUS, /tools\/feel-corpus$/);
  for (const name of ['tck-copies', 'model-evaluator']) {
    assert.equal(step(group, name).env.RHEON_BACKEND_TWINS, '1', name);
    assert.match(step(group, name).command, /RHEON_BACKEND_COVERAGE="\$TIER_LOGS\/twins-coverage.txt" cargo test -p rheon-model-evaluator/, name);
  }
  assert.deepEqual(step(group, 'twins-coverage').needs, ['tck-copies', 'model-evaluator']);
  const full = step(releaseSteps(), 'backends-full');
  assert.match(full.command, /cargo test --release -p rheon-feel-wasm --test backends -- --ignored --nocapture$/);
  assert.equal(full.env.RHEON_FULL_TESTS, '1');
  assert.deepEqual(full.needs, ['full-size-build']);
});
