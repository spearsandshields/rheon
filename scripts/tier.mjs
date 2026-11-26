#!/usr/bin/env node
// The three test tiers as commands, each a list of steps run in order, with the measured time of
// every step and a budget on the run time:
//
//   node scripts/tier.mjs fix [-p <crate>]… [--test <filter>] [--base <rev>] [--logs <dir>]
//   node scripts/tier.mjs group [--only rust|module|carried] [--logs <dir>]
//   node scripts/tier.mjs release [--without-large-sets] [--logs <dir>]
//   node scripts/tier.mjs speed [--logs <dir>]
//
// Every tier takes --jobs <n>: the most steps that run at once (default: half the cores, at most
// 4; 1 runs them in the order of the table). Steps that share a build directory or write the same
// files run one after the other whatever --jobs says. The speed tier is the speed suite by
// itself (RHEON_BENCH, the directory of bench.mjs): a measurement, to run on a quiet machine.
//
// (npm run tier:fix, tier:group, tier:release; arguments after `--`.) A step is a build (compile
// steps: cargo check, test builds with --no-run, the wasm build, clippy, documentation) or a run
// (everything else); the budget judges the run total, so a cold build does not fail a tier. The
// fix tier stops at the first failing step. The group and release tiers run every step and report
// every failure; a step whose prerequisite failed is skipped. An input kept outside the repository
// (the large request sets, IDENTITY_LARGE_SETS) is required by the
// release tier: without it the step fails, unless its --without flag is given, and then the step
// is NOT RUN and counted in the verdict.
//
// The fix tier's crates are the -p crates, or else the crates owning the files that differ from
// the merge base with main (--base <rev> names another base), uncommitted and untracked files
// included; a change of the root manifest or lock file tests the workspace. A changed ledger file
// adds the Rust copies of the TCK tests and runs each changed release-tier ledger file on its own;
// a change of the identity tool, this runner or the TCK summary adds that tool's own tests.
//
// Each step writes its output to <logs>/<nn>-<step>.log (default .tools/tier/<tier>/); a failing
// step prints the end of its log. RHEON_WASM_TARGET_DIR, when set, is the CARGO_TARGET_DIR of the
// steps that build for wasm32. The last lines are the table of steps (kind, seconds, result), the
// build and run totals, the budget and the verdict. Exit status: 0 the tier passed, 1 a step
// failed or the run total is over the budget, 2 a usage or environment error.
//
// Before its steps a tier looks at the build directories (target/<profile> and
// target/<target>/<profile>, also of RHEON_WASM_TARGET_DIR): cargo never removes the files of
// builds that later builds replaced, and a directory with hundreds of thousands of them makes every
// compile, link and test start slow (70 GB and 831,000 files made the fix tier take 13 minutes
// instead of 2). A profile directory whose `deps` holds more than BUILD_FILES_LIMIT files is
// removed, and the tier says so: the next build of that profile is a cold one.
import { execFileSync, spawn } from 'node:child_process';
import { closeSync, existsSync, mkdirSync, openSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/** The most files in the `deps` of a build directory before a tier removes the directory. */
export const BUILD_FILES_LIMIT = 60_000;

/**
 * Removes the profile directories below the target directories `targets` whose `deps` holds
 * more than `limit` files; returns those removed, each with the number of files it had.
 */
export function sweepBuildDirectories(targets, { limit = BUILD_FILES_LIMIT, print = console.log } = {}) {
  const removed = [];
  const directories = (dir) => {
    try {
      return readdirSync(dir, { withFileTypes: true })
        .filter((entry) => entry.isDirectory())
        .map((entry) => path.join(dir, entry.name));
    } catch {
      return [];
    }
  };
  for (const target of new Set(targets)) {
    // target/<profile> and target/<target triple>/<profile>
    const profiles = directories(target).flatMap((dir) => [dir, ...directories(dir)]);
    for (const profile of profiles) {
      const deps = path.join(profile, 'deps');
      let files;
      try {
        files = readdirSync(deps).length;
      } catch {
        continue;
      }
      if (files <= limit) continue;
      print(`tier: ${path.relative(root, profile) || profile} holds ${files} files of old builds (more than ${limit}); removing it, the next build there is a cold one`);
      rmSync(profile, { recursive: true, force: true });
      removed.push({ profile, files });
    }
  }
  return removed;
}

/**
 * Budgets of run time in seconds, per tier and host class. On CI (the CI environment variable
 * set) the times are reported without a budget until five runs exist to set one from.
 */
export const BUDGETS = {
  fix: { local: 120, ci: null },
  group: { local: 360, ci: null },
  release: { local: 1800, ci: null },
  speed: { local: null, ci: null },
};

const root = fileURLToPath(new URL('..', import.meta.url));

/** The tests CI skips: they need a TCK checkout at ../tck or a checkout named dsntk. */
const SKIPS = '--skip compare_the_number_of_models --skip utils::tests::getting_';
/** The Rust copies of the TCK tests (rheon-model-evaluator). */
const TCK_COPIES = 'tests::compatibility::';
const PUBLISHED = ['rheon-feel-number', 'rheon-feel-temporal', 'rheon-feel', 'rheon-feel-regex', 'rheon-feel-parser', 'rheon-feel-evaluator', 'rheon-model', 'rheon-model-evaluator', 'rheon-feel-wasm'];
const FULL_SIZE = '-p rheon-feel-number -p rheon-feel -p rheon-feel-parser -p rheon-feel-wasm';
const BASELINE = '.tools/baseline/feel.wasm';
const WASM = { wasm: true };
/** The sets of the group tier that `--only` names. */
const SETS = ['rust', 'module', 'carried'];
/** The crates carried from dsntk with manifest-only changes, and the cargo arguments that name them or the rest (the product). */
const CARRIED_CRATES = ['dsntk', 'dsntk-evaluator', 'dsntk-examples', 'dsntk-gendoc', 'dsntk-server', 'dsntk-workspace'];
const CARRIED = CARRIED_CRATES.map((c) => `-p ${c}`).join(' ');
const PRODUCT = `--workspace ${CARRIED_CRATES.map((c) => `--exclude ${c}`).join(' ')}`;

class UsageError extends Error {}

/** The most steps that run at once unless --jobs says otherwise: lanes keep the cargo steps of one build directory in a row. */
export const DEFAULT_JOBS = Math.max(1, Math.min(4, Math.floor(os.availableParallelism() / 2)));

/**
 * Runs the steps of a tier, up to `jobs` at once; resolves to the exit status. `print` receives
 * every output line. A step starts when the steps it `needs` have passed. Steps of the same
 * `lane` run one at a time in the order of the table (the cargo steps of one build directory,
 * the steps that write the same files); a step that is `alone` runs after every other step, with
 * nothing beside it (a measurement). With `jobs` 1 the steps run in the order of the table.
 */
export function runSteps(tier, steps, { budget = null, stopAtFailure = false, without = new Set(), logs, env = process.env, cwd = root, print = console.log, jobs = DEFAULT_JOBS } = {}) {
  rmSync(logs, { recursive: true, force: true });
  mkdirSync(logs, { recursive: true });
  const results = new Map();
  const names = new Set(steps.map((step) => step.name));
  const pending = steps.map((step, index) => ({ step, index }));
  const lanes = new Set();
  const started = performance.now();
  let running = 0;
  let stopped = false;
  const record = (step, result, seconds = null, note = '') => {
    results.set(step.name, { step, result, seconds, note });
    print(`${step.name.padEnd(28)} ${result}${seconds === null ? '' : ` ${seconds.toFixed(1)} s`}${note ? ` (${note})` : ''}`);
  };

  const finish = () => {
    const all = steps.map((step) => results.get(step.name));
    const total = (kind) => all.filter((r) => r.step.kind === kind && r.seconds !== null).reduce((s, r) => s + r.seconds, 0);
    const failed = all.filter((r) => r.result !== 'ok' && r.result !== 'NOT RUN');
    const notRun = all.filter((r) => r.result === 'NOT RUN');
    const run = total('run');
    const over = budget !== null && run > budget;
    print('');
    print(`${'step'.padEnd(28)} ${'kind'.padEnd(5)} ${'seconds'.padStart(8)}  result`);
    for (const r of all) print(`${r.step.name.padEnd(28)} ${r.step.kind.padEnd(5)} ${(r.seconds === null ? '-' : r.seconds.toFixed(1)).padStart(8)}  ${r.result}`);
    print(`build total ${total('build').toFixed(1)} s, run total ${run.toFixed(1)} s, budget ${budget === null ? 'none (times reported only)' : `${budget} s${over ? ', OVER BUDGET' : ''}`}`);
    print(`elapsed ${((performance.now() - started) / 1000).toFixed(1)} s, up to ${jobs} ${jobs === 1 ? 'step' : 'steps'} at once`);
    const parts = [`${all.length - failed.length - notRun.length} of ${all.length} steps passed`];
    if (failed.length) parts.push(`${failed.length} did not (${failed.map((r) => r.step.name).join(', ')})`);
    if (notRun.length) parts.push(`${notRun.length} not run (${notRun.map((r) => r.note).join(', ')})`);
    const passed = failed.length === 0 && !over;
    print(`${tier} tier: ${passed ? 'PASS' : 'FAIL'}, ${parts.join(', ')}`);
    return passed ? 0 : 1;
  };

  return new Promise((resolve) => {
    const settle = (item, ...recorded) => {
      pending.splice(pending.indexOf(item), 1);
      record(item.step, ...recorded);
    };
    const start = (item) => {
      const { step, index } = item;
      pending.splice(pending.indexOf(item), 1);
      running++;
      if (step.lane) lanes.add(step.lane);
      const log = path.join(logs, `${String(index + 1).padStart(2, '0')}-${step.name}.log`);
      const fd = openSync(log, 'w');
      const t0 = performance.now();
      const child = spawn('bash', ['-c', step.command], { cwd, env: { ...env, ...step.env }, stdio: ['ignore', fd, fd] });
      const done = (status) => {
        closeSync(fd);
        running--;
        if (step.lane) lanes.delete(step.lane);
        const seconds = (performance.now() - t0) / 1000;
        if (status === 0) record(step, 'ok', seconds);
        else {
          record(step, 'FAIL', seconds, `exit ${status}, ${path.relative(cwd, log)}`);
          const lines = readFileSync(log, 'utf8').trimEnd().split('\n');
          for (const line of lines.slice(-40)) print(`  | ${line}`);
          stopped = stopAtFailure;
        }
        schedule();
      };
      child.once('error', () => done(127));
      child.once('close', (status, signal) => done(status ?? `signal ${signal}`));
    };
    const schedule = () => {
      for (const item of [...pending]) {
        const { step, index } = item;
        if (stopped) {
          settle(item, 'STOPPED');
          continue;
        }
        const needs = step.needs ?? [];
        const failedNeed = needs.find((n) => !names.has(n) || (results.has(n) && results.get(n).result !== 'ok'));
        if (failedNeed !== undefined) {
          settle(item, 'SKIPPED', null, `${failedNeed} did not pass`);
          continue;
        }
        if (needs.some((n) => !results.has(n))) continue;
        if (step.input && !env[step.input.env]) {
          if (without.has(step.input.flag)) settle(item, 'NOT RUN', null, step.input.flag);
          else {
            settle(item, 'FAIL', null, `needs ${step.input.env}, or ${step.input.flag}`);
            stopped = stopAtFailure;
          }
          continue;
        }
        if (running >= jobs) continue;
        // a lane runs its steps one at a time, in the order of the table
        if (step.lane && (lanes.has(step.lane) || pending.some((other) => other.index < index && other.step.lane === step.lane))) continue;
        // a step that is alone waits for every other step, and runs with nothing beside it
        if (step.alone ? running > 0 || pending.some((other) => !other.step.alone) : false) continue;
        start(item);
        if (step.alone) break;
      }
      if (running === 0) {
        // steps that wait for each other, or for nothing that will come
        for (const item of [...pending]) settle(item, 'SKIPPED', null, 'its prerequisites did not finish');
        resolve(finish());
      }
    };
    schedule();
  });
}

/** A cargo test as two steps: its build (--no-run) and its run. */
function cargoTest(name, args, testArgs = '', extra = {}) {
  return [
    { name: `${name}-build`, kind: 'build', command: `cargo test --no-run ${args}`, ...extra },
    { name, kind: 'run', command: `cargo test ${args}${testArgs ? ` -- ${testArgs}` : ''}`, needs: [`${name}-build`], ...extra },
  ];
}

/** Steps that build for or run on wasm32 use RHEON_WASM_TARGET_DIR as their target directory when it is set. */
function withWasmTarget(steps, env) {
  return steps.map((s) => {
    const { wasm, ...step } = s;
    return wasm && env.RHEON_WASM_TARGET_DIR ? { ...step, env: { ...step.env, CARGO_TARGET_DIR: env.RHEON_WASM_TARGET_DIR } } : step;
  });
}

// The lanes: cargo locks a build directory per profile, so the debug, the release and the wasm
// builds each run their steps in a row and beside each other. The identity runs share a cache
// directory; the runtime tests and the playground build both build the runtime package.
const DEBUG = { lane: 'debug' };
const RELEASE = { lane: 'release' };
const WASM_LANE = { lane: 'wasm', ...WASM };
const IDENTITY = { lane: 'identity' };
const NODE = { lane: 'node' };

// The sets of the group tier (`--only <set>`): each is a job of the CI workflow.
const RUST = { set: 'rust' };
const MODULE = { set: 'module' };
const CARRIED_SET = { set: 'carried' };

const identity = (options = '') => `bash tools/identity/fetch-baseline.sh && bash tools/identity/run.sh ${BASELINE} dist/feel.wasm${options}`;

/** The steps every tier shares: formatting, compilation and the wasm module with its identity check. */
const head = [
  { name: 'fmt', kind: 'run', command: 'cargo fmt --all --check', ...RUST },
  { name: 'check', kind: 'build', command: `cargo check ${PRODUCT} --all-targets`, ...DEBUG, ...RUST },
];
const wasmBuild = { name: 'build-wasm', kind: 'build', command: 'npm run build:wasm', ...WASM_LANE, ...MODULE };
const identityStep = { name: 'identity', kind: 'run', command: identity(), needs: ['build-wasm'], ...IDENTITY, ...MODULE };
const tckCopies = (needs) => ({ name: 'tck-copies', kind: 'run', command: `cargo test -p rheon-model-evaluator -- ${TCK_COPIES} ${SKIPS}`, needs, ...DEBUG, ...RUST });
const toolTests = {
  identity: { name: 'test-identity', kind: 'run', command: 'npm run test:identity', needs: ['build-wasm'], ...IDENTITY, ...MODULE },
  tier: { name: 'test-tier', kind: 'run', command: 'npm run test:tier', ...RUST },
  tckSummary: { name: 'test-tck-summary', kind: 'run', command: 'npm run test:tck-summary', ...RUST },
};

/** The group tier: the whole workspace and every check of the module. */
function groupSteps() {
  return [
    ...head,
    { name: 'workspace-build', kind: 'build', command: `cargo test ${PRODUCT} --no-run`, ...DEBUG, ...RUST },
    // every test of the product, the test programs side by side (scripts/run-tests.mjs)
    { name: 'workspace', kind: 'run', command: `node scripts/run-tests.mjs ${PRODUCT} -- ${SKIPS}`, needs: ['workspace-build'], ...DEBUG, ...RUST },
    { name: 'clippy', kind: 'build', command: `cargo clippy ${PRODUCT} --lib --tests -- -D warnings`, ...DEBUG, ...RUST },
    toolTests.tier,
    toolTests.tckSummary,
    wasmBuild,
    identityStep,
    toolTests.identity,
    { name: 'oracle-quick', kind: 'run', command: 'bash tools/number-oracle/run.sh quick', needs: ['build-wasm'], ...MODULE },
    ...cargoTest('test-rust-wasm', '-p rheon-feel-wasm --target wasm32-wasip1', '', { env: {}, ...WASM_LANE, ...MODULE }).map((s) => ({ ...s, command: `source ./scripts/env.sh && ${s.command}` })),
    { name: 'test-runtime', kind: 'run', command: 'npm run test:runtime', needs: ['build-wasm'], ...NODE, ...MODULE },
    // after the runtime tests: both build the runtime package
    { name: 'test-codemirror', kind: 'run', command: 'npm run test:codemirror', needs: ['test-runtime'], ...NODE, ...MODULE },
    { name: 'smoke', kind: 'run', command: 'npm run smoke', needs: ['build-wasm'], ...MODULE },
    { name: 'stack', kind: 'run', command: 'npm run stack', needs: ['build-wasm'], ...MODULE },
    { name: 'build-playground', kind: 'build', command: 'npm run build:playground', needs: ['build-wasm'], ...NODE, ...MODULE },
    // the crates carried from dsntk (the command line tool, its server and their helpers): they
    // and their dependencies are built only here, and the TCK runs against the server
    { name: 'carried-build', kind: 'build', command: `cargo test ${CARRIED} --no-run`, lane: 'carried', ...CARRIED_SET },
    { name: 'carried', kind: 'run', command: `node scripts/run-tests.mjs ${CARRIED} -- ${SKIPS}`, needs: ['carried-build'], lane: 'carried', ...CARRIED_SET },
    { name: 'carried-clippy', kind: 'build', command: `cargo clippy ${CARRIED} --all-targets -- -D warnings`, lane: 'carried', ...CARRIED_SET },
    { name: 'tck-server-build', kind: 'build', command: 'cargo build --release --features tck -p dsntk', ...RELEASE, ...CARRIED_SET },
    { name: 'tck', kind: 'run', command: 'bash scripts/tck.sh', needs: ['tck-server-build'], ...RELEASE, ...CARRIED_SET },
  ];
}

/** The release tier: the group tier, then the large and slow checks. */
function releaseSteps() {
  return [
    ...groupSteps(),
    { name: 'identity-release', kind: 'run', command: identity(' --release --verify-cache'), needs: ['build-wasm'], ...IDENTITY },
    { name: 'identity-large', kind: 'run', command: identity(' --large --verify-cache'), needs: ['build-wasm'], input: { env: 'IDENTITY_LARGE_SETS', flag: '--without-large-sets' }, ...IDENTITY },
    { name: 'oracle-full', kind: 'run', command: 'bash tools/number-oracle/run.sh full', needs: ['build-wasm'] },
    ...cargoTest('full-size', `--release ${FULL_SIZE}`, '', { env: { RHEON_FULL_TESTS: '1' }, ...RELEASE }),
    { name: 'parser-corpus', kind: 'run', command: `RHEON_PARSER_CORPUS="$PWD/tools/feel-corpus" cargo test --release -p rheon-feel-parser -- --include-ignored`, needs: ['full-size-build'], ...RELEASE },
    { name: 'stack-report', kind: 'run', command: 'npm run stack -- --report', needs: ['build-wasm'] },
    { name: 'doc', kind: 'build', command: `cargo doc --no-deps ${PUBLISHED.map((c) => `-p ${c}`).join(' ')}`, env: { RUSTDOCFLAGS: '-D warnings' }, ...DEBUG },
  ];
}

/**
 * The speed tier: the speed suite on the module, the previous release and the build back to back.
 * It is a measurement, so it runs by itself and on a quiet machine: right after another tier the
 * machine is warm and the build, measured second, reads slower than it is.
 */
function speedSteps() {
  return [
    wasmBuild,
    {
      name: 'speed-suite',
      kind: 'run',
      // the reference and the candidate back to back, with nothing else running; the results are JSON files beside the logs
      command: 'for m in reference:' + BASELINE + ' candidate:dist/feel.wasm; do node --expose-gc --no-warnings "$RHEON_BENCH/bench.mjs" --modes engine --wasm "$PWD/${m#*:}" --label "${m%%:*}" --json "$TIER_LOGS/speed-${m%%:*}.json" || exit 1; done',
      needs: ['build-wasm'],
      input: { env: 'RHEON_BENCH', flag: '--without-speed-suite' },
    },
  ];
}

/** The workspace crates by their directory relative to the root, from cargo metadata. */
function crateDirs() {
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--no-deps'], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 2 ** 20 }));
  return metadata.packages.map((p) => ({ name: p.name, dir: path.relative(root, path.dirname(p.manifest_path)) }));
}

/** The files that differ from `base`: committed, uncommitted and untracked. */
function changedFiles(base) {
  const git = (...args) => execFileSync('git', args, { cwd: root, encoding: 'utf8' }).split('\n').filter(Boolean);
  const from = base ?? git('merge-base', 'HEAD', 'main')[0];
  return [...new Set([...git('diff', '--name-only', from), ...git('ls-files', '--others', '--exclude-standard')])];
}

/** The fix tier for a change: the touched crates (or `crates`), the fix's own test, the module and the tools the change touches. */
function fixSteps({ crates, test, base }) {
  const files = changedFiles(base);
  let packages = crates;
  if (packages.length === 0) {
    const dirs = crateDirs().sort((a, b) => b.dir.length - a.dir.length);
    const owner = (f) => dirs.find((c) => c.dir !== '' && (f === c.dir || f.startsWith(`${c.dir}/`)));
    packages = files.some((f) => f === 'Cargo.toml' || f === 'Cargo.lock') ? ['--workspace'] : [...new Set(files.map(owner).filter(Boolean).map((c) => c.name))].sort();
  }
  const args = packages.map((p) => (p === '--workspace' ? p : `-p ${p}`)).join(' ');
  const steps = [...head];
  if (packages.length || test) steps.push({ name: 'tests-build', kind: 'build', command: `cargo test --no-run ${args || '--workspace'}`, ...DEBUG });
  if (test) steps.push({ name: 'own-test', kind: 'run', command: `cargo test ${args || '--workspace'} -- ${test}`, needs: ['tests-build'], ...DEBUG });
  if (packages.length) steps.push({ name: 'crate-tests', kind: 'run', command: `node scripts/run-tests.mjs ${args} -- ${SKIPS}`, needs: ['tests-build'], ...DEBUG });
  steps.push(wasmBuild, identityStep);
  const ledgers = files.filter((f) => /^tools\/identity\/(release\/)?registered\/.*\.json$/.test(f));
  if (ledgers.length) {
    steps.push({ name: 'tck-copies-build', kind: 'build', command: 'cargo test --no-run -p rheon-model-evaluator', ...DEBUG }, tckCopies(['tck-copies-build']));
    for (const ledger of ledgers.filter((f) => f.startsWith('tools/identity/release/') && existsSync(path.join(root, f)))) {
      steps.push({ name: `ledger-${path.basename(ledger, '.json')}`, kind: 'run', command: `node tools/identity/equal.mjs ${BASELINE} dist/feel.wasm --ledger ${ledger} --cache .tools/identity-cache`, needs: ['build-wasm'], ...IDENTITY });
    }
  }
  if (files.some((f) => f.startsWith('tools/identity/'))) steps.push(toolTests.identity);
  if (files.some((f) => /^scripts\/tier(\.test)?\.mjs$/.test(f))) steps.push(toolTests.tier);
  if (files.some((f) => /^scripts\/tck-summary(\.test)?\.mjs$/.test(f))) steps.push(toolTests.tckSummary);
  return { steps, packages };
}

function parseArgs(argv) {
  const [tier, ...rest] = argv;
  if (!(tier in BUDGETS)) throw new UsageError(`the tier is one of ${Object.keys(BUDGETS).join(', ')}`);
  const options = { tier, crates: [], test: null, base: null, without: new Set(), logs: path.join(root, '.tools/tier', tier), jobs: DEFAULT_JOBS, only: null };
  for (let i = 0; i < rest.length; i++) {
    const arg = rest[i];
    const value = () => (i + 1 < rest.length ? rest[++i] : (() => { throw new UsageError(`${arg} needs a value`); })());
    if (arg === '--logs') options.logs = path.resolve(value());
    else if (arg === '--jobs') {
      options.jobs = Number(value());
      if (!Number.isInteger(options.jobs) || options.jobs < 1) throw new UsageError('--jobs needs a whole number of at least 1');
    } else if (tier === 'group' && arg === '--only') {
      options.only = value();
      if (!SETS.includes(options.only)) throw new UsageError(`--only names one of ${SETS.join(', ')}`);
    }
    else if (tier === 'fix' && arg === '-p') options.crates.push(value());
    else if (tier === 'fix' && arg === '--test') options.test = value();
    else if (tier === 'fix' && arg === '--base') options.base = value();
    else if (tier === 'release' && arg === '--without-large-sets') options.without.add(arg);
    else throw new UsageError(`unknown option ${arg} for the ${tier} tier`);
  }
  return options;
}

async function main(argv) {
  const options = parseArgs(argv);
  const env = { ...process.env, TIER_LOGS: options.logs };
  let steps;
  if (options.tier === 'fix') {
    const fix = fixSteps(options);
    console.log(`fix tier: ${fix.packages.length ? fix.packages.join(' ') : 'no crate touched'}${options.test ? `, own test ${options.test}` : ''}`);
    steps = fix.steps;
  } else if (options.tier === 'group') steps = groupSteps().filter((step) => options.only === null || step.set === options.only);
  else steps = options.tier === 'release' ? releaseSteps() : speedSteps();
  const budget = BUDGETS[options.tier][env.CI ? 'ci' : 'local'];
  sweepBuildDirectories([env.CARGO_TARGET_DIR ?? path.join(root, 'target'), env.RHEON_WASM_TARGET_DIR].filter(Boolean));
  return runSteps(options.tier, withWasmTarget(steps, env), { budget, stopAtFailure: options.tier === 'fix', without: options.without, logs: options.logs, env, jobs: options.jobs });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    process.exitCode = await main(process.argv.slice(2));
  } catch (e) {
    console.error(`tier: ${e instanceof UsageError ? e.message : e.stack ?? e}`);
    if (e instanceof UsageError) console.error('usage: node scripts/tier.mjs fix [-p <crate>]… [--test <filter>] [--base <rev>] | group [--only rust|module|carried] | release [--without-large-sets] | speed  [--logs <dir>] [--jobs <n>]');
    process.exitCode = 2;
  }
}
