#!/usr/bin/env node
// The three test tiers as commands, each a list of steps run in order, with the measured time of
// every step and a budget on the run time:
//
//   node scripts/tier.mjs fix [-p <crate>]… [--test <filter>] [--base <rev>] [--logs <dir>]
//   node scripts/tier.mjs group [--logs <dir>]
//   node scripts/tier.mjs release [--without-large-sets] [--without-speed-suite] [--logs <dir>]
//
// (npm run tier:fix, tier:group, tier:release; arguments after `--`.) A step is a build (compile
// steps: cargo check, test builds with --no-run, the wasm build, clippy, documentation) or a run
// (everything else); the budget judges the run total, so a cold build does not fail a tier. The
// fix tier stops at the first failing step. The group and release tiers run every step and report
// every failure; a step whose prerequisite failed is skipped. An input kept outside the repository
// (the large request sets, IDENTITY_LARGE_SETS; the speed suite, RHEON_BENCH) is required by the
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
import { execFileSync, spawnSync } from 'node:child_process';
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, rmSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

/**
 * Budgets of run time in seconds, per tier and host class. On CI (the CI environment variable
 * set) the times are reported without a budget until five runs exist to set one from.
 */
export const BUDGETS = {
  fix: { local: 120, ci: null },
  group: { local: 360, ci: null },
  release: { local: 1800, ci: null },
};

const root = fileURLToPath(new URL('..', import.meta.url));

/** The tests CI skips: they need a Java service, a TCK checkout at ../tck or a checkout named dsntk. */
const SKIPS = '--skip external_functions --skip dmn_3_0076 --skip compare_the_number_of_models --skip utils::tests::getting_';
/** The Rust copies of the TCK tests (rheon-model-evaluator). */
const TCK_COPIES = 'tests::compatibility::';
const PUBLISHED = ['rheon-feel-number', 'rheon-feel-temporal', 'rheon-feel', 'rheon-feel-regex', 'rheon-feel-grammar', 'rheon-feel-parser', 'rheon-feel-evaluator', 'rheon-model', 'rheon-model-evaluator', 'rheon-feel-wasm'];
const FULL_SIZE = '-p rheon-feel-number -p rheon-feel -p rheon-feel-parser -p rheon-feel-wasm';
const BASELINE = '.tools/baseline/feel.wasm';
const WASM = { wasm: true };

class UsageError extends Error {}

/** Runs the steps of a tier; returns the exit status. `print` receives every output line. */
export function runSteps(tier, steps, { budget = null, stopAtFailure = false, without = new Set(), logs, env = process.env, cwd = root, print = console.log } = {}) {
  rmSync(logs, { recursive: true, force: true });
  mkdirSync(logs, { recursive: true });
  const results = new Map();
  let stopped = false;
  steps.forEach((step, i) => {
    const log = path.join(logs, `${String(i + 1).padStart(2, '0')}-${step.name}.log`);
    const record = (result, seconds = null, note = '') => {
      results.set(step.name, { step, result, seconds, note });
      print(`${step.name.padEnd(28)} ${result}${seconds === null ? '' : ` ${seconds.toFixed(1)} s`}${note ? ` (${note})` : ''}`);
    };
    if (stopped) return record('STOPPED');
    const failedNeed = (step.needs ?? []).find((n) => results.get(n)?.result !== 'ok');
    if (failedNeed !== undefined) return record('SKIPPED', null, `${failedNeed} did not pass`);
    if (step.input && !env[step.input.env]) {
      if (without.has(step.input.flag)) return record('NOT RUN', null, step.input.flag);
      record('FAIL', null, `needs ${step.input.env}, or ${step.input.flag}`);
      stopped = stopAtFailure;
      return;
    }
    const fd = openSync(log, 'w');
    const t0 = performance.now();
    let status;
    try {
      status = spawnSync('bash', ['-c', step.command], { cwd, env: { ...env, ...step.env }, stdio: ['ignore', fd, fd] }).status;
    } finally {
      closeSync(fd);
    }
    const seconds = (performance.now() - t0) / 1000;
    if (status === 0) return record('ok', seconds);
    record('FAIL', seconds, `exit ${status}, ${path.relative(cwd, log)}`);
    const lines = readFileSync(log, 'utf8').trimEnd().split('\n');
    for (const line of lines.slice(-40)) print(`  | ${line}`);
    stopped = stopAtFailure;
  });

  const all = [...results.values()];
  const total = (kind) => all.filter((r) => r.step.kind === kind && r.seconds !== null).reduce((s, r) => s + r.seconds, 0);
  const failed = all.filter((r) => r.result !== 'ok' && r.result !== 'NOT RUN');
  const notRun = all.filter((r) => r.result === 'NOT RUN');
  const run = total('run');
  const over = budget !== null && run > budget;
  print('');
  print(`${'step'.padEnd(28)} ${'kind'.padEnd(5)} ${'seconds'.padStart(8)}  result`);
  for (const r of all) print(`${r.step.name.padEnd(28)} ${r.step.kind.padEnd(5)} ${(r.seconds === null ? '-' : r.seconds.toFixed(1)).padStart(8)}  ${r.result}`);
  print(`build total ${total('build').toFixed(1)} s, run total ${run.toFixed(1)} s, budget ${budget === null ? 'none (times reported only)' : `${budget} s${over ? ', OVER BUDGET' : ''}`}`);
  const parts = [`${all.length - failed.length - notRun.length} of ${all.length} steps passed`];
  if (failed.length) parts.push(`${failed.length} did not (${failed.map((r) => r.step.name).join(', ')})`);
  if (notRun.length) parts.push(`${notRun.length} not run (${notRun.map((r) => r.note).join(', ')})`);
  const passed = failed.length === 0 && !over;
  print(`${tier} tier: ${passed ? 'PASS' : 'FAIL'}, ${parts.join(', ')}`);
  return passed ? 0 : 1;
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

const identity = (options = '') => `bash tools/identity/fetch-baseline.sh && bash tools/identity/run.sh ${BASELINE} dist/feel.wasm${options}`;

/** The steps every tier shares: formatting, compilation and the wasm module with its identity check. */
const head = [
  { name: 'fmt', kind: 'run', command: 'cargo fmt --all --check' },
  { name: 'check', kind: 'build', command: 'cargo check --workspace --all-targets' },
];
const wasmBuild = { name: 'build-wasm', kind: 'build', command: 'npm run build:wasm', ...WASM };
const identityStep = { name: 'identity', kind: 'run', command: identity(), needs: ['build-wasm'] };
const tckCopies = (needs) => ({ name: 'tck-copies', kind: 'run', command: `cargo test -p rheon-model-evaluator -- ${TCK_COPIES} ${SKIPS}`, needs });
const toolTests = {
  identity: { name: 'test-identity', kind: 'run', command: 'npm run test:identity', needs: ['build-wasm'] },
  tier: { name: 'test-tier', kind: 'run', command: 'npm run test:tier' },
  tckSummary: { name: 'test-tck-summary', kind: 'run', command: 'npm run test:tck-summary' },
};

/** The group tier: the whole workspace and every check of the module. */
function groupSteps() {
  return [
    ...head,
    { name: 'workspace-build', kind: 'build', command: 'cargo test --workspace --no-run' },
    tckCopies(['workspace-build']),
    // the rest of rheon-model-evaluator, and every other crate (a filter cannot name one crate's tests)
    { name: 'model-evaluator', kind: 'run', command: `cargo test -p rheon-model-evaluator -- ${SKIPS} --skip ${TCK_COPIES}`, needs: ['workspace-build'] },
    { name: 'workspace', kind: 'run', command: `cargo test --workspace --exclude rheon-model-evaluator --no-fail-fast -- ${SKIPS}`, needs: ['workspace-build'] },
    { name: 'clippy', kind: 'build', command: 'cargo clippy --workspace --lib --tests -- -D warnings' },
    wasmBuild,
    identityStep,
    toolTests.identity,
    toolTests.tier,
    toolTests.tckSummary,
    { name: 'oracle-quick', kind: 'run', command: 'bash tools/number-oracle/run.sh quick', needs: ['build-wasm'] },
    ...cargoTest('test-rust', '-p rheon-feel-wasm'),
    ...cargoTest('test-rust-wasm', '-p rheon-feel-wasm --target wasm32-wasip1', '', { env: {}, ...WASM }).map((s) => ({ ...s, command: `source ./scripts/env.sh && ${s.command}` })),
    { name: 'test-runtime', kind: 'run', command: 'npm run test:runtime', needs: ['build-wasm'] },
    { name: 'smoke', kind: 'run', command: 'npm run smoke', needs: ['build-wasm'] },
    { name: 'stack', kind: 'run', command: 'npm run stack', needs: ['build-wasm'] },
    { name: 'build-playground', kind: 'build', command: 'npm run build:playground', needs: ['build-wasm'] },
    { name: 'tck-server-build', kind: 'build', command: 'cargo build --release --features tck -p dsntk' },
    { name: 'tck', kind: 'run', command: 'bash scripts/tck.sh', needs: ['tck-server-build'] },
  ];
}

/** The release tier: the group tier, then the large and slow checks. */
function releaseSteps() {
  return [
    ...groupSteps(),
    { name: 'identity-release', kind: 'run', command: identity(' --release --verify-cache'), needs: ['build-wasm'] },
    { name: 'identity-large', kind: 'run', command: identity(' --large --verify-cache'), needs: ['build-wasm'], input: { env: 'IDENTITY_LARGE_SETS', flag: '--without-large-sets' } },
    { name: 'oracle-full', kind: 'run', command: 'bash tools/number-oracle/run.sh full', needs: ['build-wasm'] },
    ...cargoTest('full-size', `--release ${FULL_SIZE}`, '', { env: { RHEON_FULL_TESTS: '1' } }),
    { name: 'parser-corpus', kind: 'run', command: `RHEON_PARSER_CORPUS="$PWD/tools/feel-corpus" cargo test --release -p rheon-feel-parser -- --include-ignored`, needs: ['full-size-build'] },
    { name: 'stack-report', kind: 'run', command: 'npm run stack -- --report', needs: ['build-wasm'] },
    {
      name: 'speed-suite',
      kind: 'run',
      // the reference and the candidate back to back; the results are JSON files beside the logs
      command: 'for m in reference:' + BASELINE + ' candidate:dist/feel.wasm; do node --expose-gc --no-warnings "$RHEON_BENCH/bench.mjs" --modes engine --wasm "$PWD/${m#*:}" --label "${m%%:*}" --json "$TIER_LOGS/speed-${m%%:*}.json" || exit 1; done',
      needs: ['build-wasm'],
      input: { env: 'RHEON_BENCH', flag: '--without-speed-suite' },
    },
    { name: 'doc', kind: 'build', command: `cargo doc --no-deps ${PUBLISHED.map((c) => `-p ${c}`).join(' ')}`, env: { RUSTDOCFLAGS: '-D warnings' } },
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
  if (packages.length || test) steps.push({ name: 'tests-build', kind: 'build', command: `cargo test --no-run ${args || '--workspace'}` });
  if (test) steps.push({ name: 'own-test', kind: 'run', command: `cargo test ${args || '--workspace'} -- ${test}`, needs: ['tests-build'] });
  if (packages.length) steps.push({ name: 'crate-tests', kind: 'run', command: `cargo test ${args} -- ${SKIPS}`, needs: ['tests-build'] });
  steps.push(wasmBuild, identityStep);
  const ledgers = files.filter((f) => /^tools\/identity\/(release\/)?registered\/.*\.json$/.test(f));
  if (ledgers.length) {
    steps.push({ name: 'tck-copies-build', kind: 'build', command: 'cargo test --no-run -p rheon-model-evaluator' }, tckCopies(['tck-copies-build']));
    for (const ledger of ledgers.filter((f) => f.startsWith('tools/identity/release/') && existsSync(path.join(root, f)))) {
      steps.push({ name: `ledger-${path.basename(ledger, '.json')}`, kind: 'run', command: `node tools/identity/equal.mjs ${BASELINE} dist/feel.wasm --ledger ${ledger} --cache .tools/identity-cache`, needs: ['build-wasm'] });
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
  const options = { tier, crates: [], test: null, base: null, without: new Set(), logs: path.join(root, '.tools/tier', tier) };
  for (let i = 0; i < rest.length; i++) {
    const arg = rest[i];
    const value = () => (i + 1 < rest.length ? rest[++i] : (() => { throw new UsageError(`${arg} needs a value`); })());
    if (arg === '--logs') options.logs = path.resolve(value());
    else if (tier === 'fix' && arg === '-p') options.crates.push(value());
    else if (tier === 'fix' && arg === '--test') options.test = value();
    else if (tier === 'fix' && arg === '--base') options.base = value();
    else if (tier === 'release' && (arg === '--without-large-sets' || arg === '--without-speed-suite')) options.without.add(arg);
    else throw new UsageError(`unknown option ${arg} for the ${tier} tier`);
  }
  return options;
}

function main(argv) {
  const options = parseArgs(argv);
  const env = { ...process.env, TIER_LOGS: options.logs };
  let steps;
  if (options.tier === 'fix') {
    const fix = fixSteps(options);
    console.log(`fix tier: ${fix.packages.length ? fix.packages.join(' ') : 'no crate touched'}${options.test ? `, own test ${options.test}` : ''}`);
    steps = fix.steps;
  } else steps = options.tier === 'group' ? groupSteps() : releaseSteps();
  const budget = BUDGETS[options.tier][env.CI ? 'ci' : 'local'];
  return runSteps(options.tier, withWasmTarget(steps, env), { budget, stopAtFailure: options.tier === 'fix', without: options.without, logs: options.logs, env });
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    process.exitCode = main(process.argv.slice(2));
  } catch (e) {
    console.error(`tier: ${e instanceof UsageError ? e.message : e.stack ?? e}`);
    if (e instanceof UsageError) console.error('usage: node scripts/tier.mjs fix [-p <crate>]… [--test <filter>] [--base <rev>] | group | release [--without-large-sets] [--without-speed-suite]  [--logs <dir>]');
    process.exitCode = 2;
  }
}
