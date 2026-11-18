#!/usr/bin/env node
// Runs the tests of cargo packages with their test programs side by side:
//
//   node scripts/run-tests.mjs [--jobs <n>] <cargo test arguments> [-- <arguments of the test programs>]
//
// `cargo test` builds the test programs and then runs them one after the other. This builds them
// the same way (`cargo test --no-run` with the given arguments), runs `--jobs` of them at once
// (default: 4), each in the directory of its package as cargo does, and then runs the
// documentation tests with `cargo test --doc`. The same tests run with the same arguments; only
// the order and the waiting differ. The output of a test program is printed when it fails; a
// passing one gets a line with its result.
//
// Exit status: 0 every test passed, 1 a test or the build failed, 2 a usage error.
import { spawn, spawnSync } from 'node:child_process';
import path from 'node:path';

const argv = process.argv.slice(2);
let jobs = 4;
if (argv[0] === '--jobs') {
  jobs = Number(argv[1]);
  argv.splice(0, 2);
  if (!Number.isInteger(jobs) || jobs < 1) {
    console.error('usage: node scripts/run-tests.mjs [--jobs <n>] <cargo test arguments> [-- <arguments of the test programs>]');
    process.exit(2);
  }
}
const split = argv.indexOf('--');
const cargoArgs = split < 0 ? argv : argv.slice(0, split);
const testArgs = split < 0 ? [] : argv.slice(split + 1);

// the test programs, from cargo's messages
const build = spawnSync('cargo', ['test', '--no-run', '--message-format=json-render-diagnostics', ...cargoArgs], {
  encoding: 'utf8',
  maxBuffer: 1024 * 1024 * 1024,
  stdio: ['ignore', 'pipe', 'inherit'],
});
if (build.status !== 0) {
  console.error(`run-tests: the build failed (cargo exit ${build.status})`);
  process.exit(1);
}
const programs = [];
for (const line of build.stdout.split('\n')) {
  if (!line.startsWith('{')) continue;
  const message = JSON.parse(line);
  if (message.reason !== 'compiler-artifact' || !message.executable || !message.profile?.test) continue;
  const manifestDir = path.dirname(message.manifest_path);
  programs.push({ executable: message.executable, manifestDir, name: `${path.basename(manifestDir)} ${message.target.name}` });
}
if (programs.length === 0) {
  console.error('run-tests: cargo built no test program');
  process.exit(1);
}

const started = performance.now();
let failed = 0;
let passedTests = 0;
const queue = [...programs];
const runOne = (program) =>
  new Promise((resolve) => {
    const t0 = performance.now();
    const chunks = [];
    const child = spawn(program.executable, testArgs, { cwd: program.manifestDir, env: { ...process.env, CARGO_MANIFEST_DIR: program.manifestDir }, stdio: ['ignore', 'pipe', 'pipe'] });
    child.stdout.on('data', (chunk) => chunks.push(chunk));
    child.stderr.on('data', (chunk) => chunks.push(chunk));
    child.once('error', (e) => {
      chunks.push(Buffer.from(String(e)));
    });
    child.once('close', (status) => {
      const output = Buffer.concat(chunks).toString('utf8');
      const seconds = ((performance.now() - t0) / 1000).toFixed(1);
      const result = output.match(/^test result: .*$/m)?.[0] ?? 'no test result';
      passedTests += Number(result.match(/(\d+) passed/)?.[1] ?? 0);
      if (status === 0) console.log(`ok    ${seconds.padStart(6)} s  ${program.name}: ${result}`);
      else {
        failed++;
        console.log(`FAIL  ${seconds.padStart(6)} s  ${program.name} (exit ${status})`);
        console.log(output);
      }
      resolve();
    });
  });
const worker = async () => {
  for (let program = queue.shift(); program; program = queue.shift()) await runOne(program);
};
await Promise.all(Array.from({ length: Math.min(jobs, programs.length) }, worker));
console.log(`${programs.length} test programs, ${passedTests} tests passed, ${failed} programs failed, ${((performance.now() - started) / 1000).toFixed(1)} s`);

// the documentation tests
const doc = spawnSync('cargo', ['test', '--doc', ...cargoArgs, ...(testArgs.length ? ['--', ...testArgs] : [])], { stdio: 'inherit' });
if (doc.status !== 0) {
  console.error(`run-tests: the documentation tests failed (cargo exit ${doc.status})`);
  failed++;
}
process.exit(failed ? 1 : 0);
