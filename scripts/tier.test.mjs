// Checks of the tier runner (scripts/tier.mjs) on step tables of their own:
//
//   node --test scripts/tier.test.mjs        (npm run test:tier)
import { test, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { runSteps, sweepBuildDirectories } from './tier.mjs';

let dir;
let count = 0;
before(() => {
  dir = mkdtempSync(path.join(os.tmpdir(), 'tier-test-'));
});
after(() => {
  if (dir) rmSync(dir, { recursive: true, force: true });
});

/** Runs a step table: the exit status, the printed lines and the logs directory. */
async function run(steps, options = {}) {
  const lines = [];
  const logs = path.join(dir, `logs${++count}`);
  const status = await runSteps('test', steps, { logs, cwd: dir, print: (line) => lines.push(line), jobs: 1, ...options });
  return { status, lines, out: lines.join('\n'), logs };
}

/** The result column of a step in the final table. */
const resultOf = (r, name) => r.lines.findLast((l) => l.startsWith(`${name.padEnd(28)} `)).trim().split(/\s+/).slice(3).join(' ');

const ok = (name, kind = 'run') => ({ name, kind, command: `echo ${name}` });
const failing = (name) => ({ name, kind: 'run', command: `echo the reason of ${name}; exit 3` });

test('a tier whose steps pass exits 0 and prints the table, the totals and the verdict', async () => {
  const r = await run([ok('compile', 'build'), ok('tests')]);
  assert.equal(r.status, 0, r.out);
  assert.match(r.out, /^step +kind +seconds +result$/m);
  assert.match(r.out, /^compile +build +\d+\.\d +ok$/m);
  assert.match(r.out, /^tests +run +\d+\.\d +ok$/m);
  assert.match(r.out, /^build total \d+\.\d s, run total \d+\.\d s, budget none \(times reported only\)$/m);
  assert.match(r.lines.at(-1), /^test tier: PASS, 2 of 2 steps passed$/);
  // one log per step, in order
  assert.deepEqual(readdirSync(r.logs), ['01-compile.log', '02-tests.log']);
});

test('a failing step fails the tier and prints the end of its log; with stopAtFailure the rest is not run', async () => {
  const steps = [ok('first'), failing('second'), ok('third')];
  const going = await run(steps);
  assert.equal(going.status, 1, going.out);
  assert.equal(resultOf(going, 'second'), 'FAIL');
  assert.equal(resultOf(going, 'third'), 'ok');
  assert.match(going.out, /^ {2}\| the reason of second$/m);
  assert.match(going.lines.at(-1), /^test tier: FAIL, 2 of 3 steps passed, 1 did not \(second\)$/);
  const stopping = await run(steps, { stopAtFailure: true });
  assert.equal(stopping.status, 1, stopping.out);
  assert.equal(resultOf(stopping, 'third'), 'STOPPED');
  assert.match(stopping.lines.at(-1), /2 did not \(second, third\)$/);
});

test('a step whose prerequisite did not pass is skipped', async () => {
  const r = await run([failing('build-it'), { ...ok('use-it'), needs: ['build-it'] }, { ...ok('use-that'), needs: ['use-it'] }, ok('other')]);
  assert.equal(r.status, 1, r.out);
  assert.equal(resultOf(r, 'use-it'), 'SKIPPED');
  assert.equal(resultOf(r, 'use-that'), 'SKIPPED');
  assert.equal(resultOf(r, 'other'), 'ok');
});

test('the budget judges the run total: over it the tier fails although every step passed', async () => {
  const slow = (kind) => ({ name: `slow-${kind}`, kind, command: 'sleep 0.4' });
  const over = await run([slow('run'), ok('fast')], { budget: 0.2 });
  assert.equal(over.status, 1, over.out);
  assert.match(over.out, /budget 0\.2 s, OVER BUDGET$/m);
  assert.match(over.lines.at(-1), /^test tier: FAIL, 2 of 2 steps passed$/);
  // build time is not judged
  const build = await run([slow('build'), ok('fast')], { budget: 0.2 });
  assert.equal(build.status, 0, build.out);
  assert.match(build.out, /run total 0\.[0-1] s, budget 0\.2 s$/m);
});

test('an input kept outside the repository: missing, it fails the step; with its --without flag the step is NOT RUN and counted', async () => {
  const step = { ...ok('large-sets'), input: { env: 'TIER_TEST_SETS', flag: '--without-sets' } };
  const env = { ...process.env, TIER_TEST_SETS: '' };
  const missing = await run([step, ok('other')], { env });
  assert.equal(missing.status, 1, missing.out);
  assert.match(missing.out, /^large-sets +FAIL \(needs TIER_TEST_SETS, or --without-sets\)$/m);
  const without = await run([step, ok('other')], { env, without: new Set(['--without-sets']) });
  assert.equal(without.status, 0, without.out);
  assert.equal(resultOf(without, 'large-sets'), 'NOT RUN');
  assert.match(without.lines.at(-1), /^test tier: PASS, 1 of 2 steps passed, 1 not run \(--without-sets\)$/);
  const given = await run([step], { env: { ...env, TIER_TEST_SETS: '/sets' } });
  assert.equal(resultOf(given, 'large-sets'), 'ok');
});

test('a step runs with its own environment added', async () => {
  const r = await run([{ name: 'env', kind: 'run', command: 'test "$TIER_TEST_VALUE" = 42', env: { TIER_TEST_VALUE: '42' } }]);
  assert.equal(r.status, 0, r.out);
});

test('usage errors exit 2', () => {
  const tier = fileURLToPath(new URL('./tier.mjs', import.meta.url));
  const cli = (...args) => spawnSync(process.execPath, [tier, ...args], { encoding: 'utf8' });
  for (const args of [[], ['nope'], ['fix', '--frobnicate'], ['fix', '-p'], ['group', '-p', 'x'], ['group', '--without-large-sets'], ['release', '--test', 'x'], ['group', '--only', 'nope'], ['release', '--only', 'rust'], ['group', '--jobs', '0']]) {
    const r = cli(...args);
    assert.equal(r.status, 2, `${args.join(' ')}: ${r.stdout}${r.stderr}`);
    assert.match(r.stderr, /^usage: /m);
  }
});

test('steps without a lane run at once, up to jobs; the table and the logs keep the order of the steps', async () => {
  const sleep = (name) => ({ name, kind: 'run', command: 'sleep 0.5' });
  const t0 = performance.now();
  const r = await run([sleep('a'), sleep('b'), sleep('c')], { jobs: 3 });
  const elapsed = (performance.now() - t0) / 1000;
  assert.equal(r.status, 0, r.out);
  assert.ok(elapsed < 1.2, `three steps of 0.5 s took ${elapsed.toFixed(2)} s`);
  assert.match(r.out, /^elapsed \d+\.\d s, up to 3 steps at once$/m);
  assert.deepEqual(readdirSync(r.logs), ['01-a.log', '02-b.log', '03-c.log']);
  const table = r.lines.slice(r.lines.findIndex((l) => l.startsWith('step ')) + 1).slice(0, 3).map((l) => l.split(/\s+/)[0]);
  assert.deepEqual(table, ['a', 'b', 'c']);
});

test('steps of one lane run one at a time in the order of the table; another lane runs beside them', async () => {
  const stamp = (name, lane) => ({ name, kind: 'run', lane, command: `echo start-${name} >> order.txt; sleep 0.3; echo end-${name} >> order.txt` });
  const r = await run([stamp('one', 'x'), stamp('two', 'x'), stamp('other', 'y')], { jobs: 4 });
  assert.equal(r.status, 0, r.out);
  const order = spawnSync('cat', [path.join(dir, 'order.txt')], { encoding: 'utf8' }).stdout.trim().split('\n');
  rmSync(path.join(dir, 'order.txt'));
  assert.ok(order.indexOf('end-one') < order.indexOf('start-two'), order.join(' '));
  assert.ok(order.indexOf('start-other') < order.indexOf('end-one'), order.join(' '));
});

test('a step that is alone runs after the others, with nothing beside it', async () => {
  const stamp = (name, extra = {}) => ({ name, kind: 'run', command: `echo start-${name} >> alone.txt; sleep 0.2; echo end-${name} >> alone.txt`, ...extra });
  const r = await run([stamp('measure', { alone: true }), stamp('a'), stamp('b')], { jobs: 4 });
  assert.equal(r.status, 0, r.out);
  const order = spawnSync('cat', [path.join(dir, 'alone.txt')], { encoding: 'utf8' }).stdout.trim().split('\n');
  rmSync(path.join(dir, 'alone.txt'));
  assert.deepEqual(order.slice(-2), ['start-measure', 'end-measure']);
});

test('a step waits for the step it needs, also when that one stands later in the table', async () => {
  const r = await run([{ ...ok('use-it'), needs: ['make-it'] }, ok('make-it', 'build')], { jobs: 2 });
  assert.equal(r.status, 0, r.out);
  assert.ok(r.lines.findIndex((l) => l.startsWith('make-it')) < r.lines.findIndex((l) => l.startsWith('use-it')), r.out);
});

test('a build directory that holds more files of old builds than the limit is removed, another is kept', () => {
  const target = mkdtempSync(path.join(os.tmpdir(), 'tier-target-'));
  try {
    const fill = (profile, files) => {
      const deps = path.join(target, profile, 'deps');
      mkdirSync(deps, { recursive: true });
      for (let i = 0; i < files; i++) writeFileSync(path.join(deps, `f${i}.o`), '');
    };
    fill('debug', 12);
    fill('release', 3);
    fill(path.join('wasm32-wasip1', 'debug'), 11);
    fill(path.join('wasm32-wasip1', 'wasm'), 2);
    mkdirSync(path.join(target, 'doc'));
    const said = [];
    const removed = sweepBuildDirectories([target, target, path.join(target, 'missing')], { limit: 10, print: (line) => said.push(line) });
    assert.deepEqual(removed.map((r) => [path.relative(target, r.profile), r.files]).sort(), [['debug', 12], [path.join('wasm32-wasip1', 'debug'), 11]]);
    assert.equal(said.length, 2);
    assert.match(said[0], /holds 1[12] files of old builds \(more than 10\); removing it/);
    assert.equal(existsSync(path.join(target, 'debug')), false);
    assert.equal(existsSync(path.join(target, 'wasm32-wasip1', 'debug')), false);
    assert.equal(existsSync(path.join(target, 'release', 'deps', 'f0.o')), true);
    assert.equal(existsSync(path.join(target, 'wasm32-wasip1', 'wasm', 'deps', 'f0.o')), true);
    assert.equal(existsSync(path.join(target, 'doc')), true);
  } finally {
    rmSync(target, { recursive: true, force: true });
  }
});
