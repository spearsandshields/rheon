#!/usr/bin/env node
// Stack headroom check of feel.wasm (docs/CONTRACT.md §1).
//
//   node scripts/stack-headroom.mjs [path/to/feel.wasm] [--report]
//
// Parsing and evaluation recurse, and the frames of a call into the module take space in two
// stacks: the module's shadow stack in linear memory (10 MiB, set in wasm/feel-wasm/build.rs) and
// the native stack the host runs wasm code on (V8: the thread's stack; Wasmtime: at most
// max-wasm-stack bytes). Running out of either is a trap. The engine bounds what a request can
// nest: ASTs deeper than 300 levels are rejected and evaluations stop after 200 nested function
// calls. This check asserts that requests within those bounds evaluate, so that a change that
// makes stack frames larger (a compiler setting, more inlining, larger locals) fails here and not
// in a host.
//
// Requests:
//   - flat nesting: five constructs (a right-nested addition chain, nested lists, nested contexts,
//     nested ifs, nested calls), each at the deepest nesting the parse op accepts, i.e. at the AST
//     depth limit;
//   - recursion: `{f: function(n) if n = 0 then 0 else 0 + (0 + (... (1 + f(n-1)) ...)), r: f(190)}.r`
//     with the body nested d levels: 190 nested calls, each evaluating a d-level-deep body. It must
//     return 190.
//
// Hosts:
//   - Node, once per V8 wasm compiler (Liftoff, the baseline compiler, and TurboFan, the
//     optimising one), each run in its own process with V8 pinned to that compiler. V8 normally
//     starts with Liftoff and moves hot functions to TurboFan while a request runs, and the two
//     use stack frames of different sizes, so an unpinned run reaches a depth that depends on
//     timing; a pinned run is deterministic. In each process:
//     - the main thread (V8's default stack, about 1 MiB): flat nesting. Its recursion depth is
//       reported but not asserted, because an application's main thread has an unknown part of
//       that stack in use already;
//     - a worker_threads worker with a 4 MiB stack (Node's default for workers): flat nesting, and
//       the recursion at d = WORKER_FLOOR; --report also runs workers with 2 and 3 MiB stacks, where
//       the native stack runs out before the shadow stack, to show the native margin;
//   - Wasmtime (.tools/wasmtime, installed by scripts/bootstrap.sh) with a 4 MiB wasm stack
//     (max-wasm-stack; Wasmtime's default is 512 KiB): flat nesting, and the recursion at
//     d = WASMTIME_FLOOR. The module is preloaded as `feel`, which makes Wasmtime instantiate the
//     reactor and call its `_initialize` (confirmed here: a second `_initialize` must trap), and is
//     driven by a generated command module that copies each request into feel's memory, calls
//     `feel_call` and writes the response to stdout.
//
// The floors are 60, the depth release 0.3.0 reaches in the TurboFan worker and in Wasmtime, so a
// change that leaves less headroom than the release had fails here. On macOS arm64 the worker
// (with either compiler) and Wasmtime all reach 64, where the 10 MiB shadow stack runs out before
// their 4 MiB native stacks do. The shadow stack is in linear memory and the same on every host,
// so the floors guard the native stack, whose use depends on the platform and on the V8 and
// Wasmtime versions. A change that alters the measured depths on purpose updates the floors with
// them.
//
// --report checks nothing: it prints the deepest recursion each host reaches (Node main thread, and
// workers with 2, 3 and 4 MiB stacks, with each compiler; Wasmtime with a wasm stack of 512 KiB, 1,
// 2, 4 and 16 MiB) and the outcome of the flat nesting there. Exit status: 0 when every check passed (with --report: when
// every host ran), 1 when a check failed, 2 when a host could not be run.
import { readFile, mkdtemp, writeFile, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { Worker, isMainThread, workerData, parentPort } from 'node:worker_threads';
import { fileURLToPath } from 'node:url';
import os from 'node:os';
import path from 'node:path';
import * as abi from './feel-abi.mjs';

const WORKER_STACK_MB = 4;
const WORKER_FLOOR = 60;
const REPORT_WORKER_STACKS_MB = [2, 3, WORKER_STACK_MB];
const WASMTIME_STACK = 4 * 1024 * 1024;
const WASMTIME_FLOOR = 60;
const REPORT_WASMTIME_STACKS = [512 * 1024, 1024 * 1024, 2 * 1024 * 1024, 4 * 1024 * 1024, 16 * 1024 * 1024];
const V8_COMPILERS = [
  { name: 'Liftoff', flag: '--liftoff-only' },
  { name: 'TurboFan', flag: '--no-liftoff' },
];
// Upper bound of the search for the AST depth limit: the parse op must reject this nesting.
const MAX_NESTING = 400;
const RECURSION_CALLS = 190;
// Internal option: run the Node hosts in this process and print their results as JSON.
const NODE_HOSTS_OPTION = '--node-hosts';

// Expected results are compared without whitespace: long results are displayed on several lines.
const flat = [
  { name: 'addition chain', expression: (d) => `${'1 + ('.repeat(d)}1${')'.repeat(d)}`, display: (d) => `${d + 1}` },
  { name: 'nested lists', expression: (d) => `count(${'['.repeat(d)}1${']'.repeat(d)})`, display: () => '1' },
  { name: 'nested contexts', expression: (d) => `${'{a: '.repeat(d)}1${'}'.repeat(d)}`, display: (d) => `${'{"a":'.repeat(d)}1${'}'.repeat(d)}` },
  { name: 'nested ifs', expression: (d) => `${'if true then '.repeat(d)}1${' else 0'.repeat(d)}`, display: () => '1' },
  { name: 'nested calls', expression: (d) => `${'abs('.repeat(d)}-1${')'.repeat(d)}`, display: () => '1' },
];
const recursion = {
  name: 'recursion',
  expression: (d) => `{f: function(n) if n = 0 then 0 else ${'0 + ('.repeat(d)}1 + f(n-1)${')'.repeat(d)}, r: f(${RECURSION_CALLS})}.r`,
  display: () => `${RECURSION_CALLS}`,
};

const evaluate = (construct, d) => JSON.stringify({ op: 'evaluate', expression: construct.expression(d) });

// The outcome of one evaluation: 'ok', or what went wrong (a trap, an error, a wrong result).
function outcome(construct, d, response) {
  if (response.trap !== undefined) return `trap: ${response.trap}`;
  if (!response.ok) return `error ${response.error?.kind}: ${response.error?.message}`;
  const display = response.result?.display;
  const compact = typeof display === 'string' ? display.replace(/\s+/g, '') : display;
  return compact === construct.display(d) ? 'ok' : `wrong result ${JSON.stringify(display).slice(0, 80)}`;
}

// ------------------------------------------------------------------ Node

// Calls into an instance of the module through the raw ABI (CONTRACT §1); after a trap the
// instance is dead, and the next call gets a fresh one.
async function nodeHost(module) {
  let ex = await abi.instantiate(module);
  return async (json) => {
    try {
      return abi.call(ex, json);
    } catch (e) {
      ex = await abi.instantiate(module);
      return { trap: `${e.constructor.name}: ${e.message}` };
    }
  };
}

// The deepest nesting of each construct that the parse op accepts (binary search below
// MAX_NESTING, which must be rejected).
async function astLimits(call) {
  const limits = {};
  for (const construct of [...flat, recursion]) {
    const accepted = async (d) => {
      const response = await call(JSON.stringify({ op: 'parse', expression: construct.expression(d) }));
      if (response.trap !== undefined) throw new Error(`${construct.name}: the parse op trapped at nesting ${d}: ${response.trap}`);
      return response.ok;
    };
    if (!(await accepted(1))) throw new Error(`${construct.name}: the parse op rejects nesting 1`);
    if (await accepted(MAX_NESTING)) throw new Error(`${construct.name}: the parse op accepts nesting ${MAX_NESTING}: no AST depth limit below it`);
    let lo = 1;
    let hi = MAX_NESTING;
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (await accepted(mid)) lo = mid;
      else hi = mid;
    }
    limits[construct.name] = lo;
  }
  return limits;
}

// The deepest recursion body (at most `max`) that evaluates, with the outcome one level deeper.
// `attempt(d)` returns an outcome; the search assumes that what fails at d also fails deeper.
async function deepestRecursion(attempt, max) {
  const shallowest = await attempt(0);
  if (shallowest !== 'ok') return { depth: -1, failure: shallowest };
  const top = await attempt(max);
  if (top === 'ok') return { depth: max, failure: null };
  let lo = 0;
  let hi = max;
  let failure = top;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    const result = await attempt(mid);
    if (result === 'ok') lo = mid;
    else {
      hi = mid;
      failure = result;
    }
  }
  return { depth: lo, failure };
}

// One Node thread: flat nesting at the AST limit, then the recursion at `floor` (when given) or,
// for a report, its deepest reachable depth.
async function runThread(modulePath, limits, floor, report) {
  const call = await nodeHost(await WebAssembly.compile(await readFile(modulePath)));
  const result = { flat: {} };
  for (const construct of flat) {
    const d = limits[construct.name];
    result.flat[construct.name] = outcome(construct, d, await call(evaluate(construct, d)));
  }
  const attempt = async (d) => outcome(recursion, d, await call(evaluate(recursion, d)));
  if (report) result.deepest = await deepestRecursion(attempt, limits[recursion.name]);
  else if (floor !== null) result.recursion = await attempt(floor);
  return result;
}

function runWorker(modulePath, limits, floor, report, stackMb) {
  return new Promise((resolve, reject) => {
    const worker = new Worker(fileURLToPath(import.meta.url), {
      workerData: { modulePath, limits, floor, report },
      resourceLimits: { stackSizeMb: stackMb },
    });
    worker.once('message', resolve);
    worker.once('error', reject);
    worker.once('exit', (code) => reject(new Error(`the worker exited (${code}) without a result`)));
  });
}

// The main thread and the workers, in a Node process with V8 pinned to one compiler.
function runNodeProcess(compiler, modulePath, limits, report) {
  const options = JSON.stringify({ modulePath, limits, report });
  const r = spawnSync(process.execPath, [compiler.flag, '--no-warnings', fileURLToPath(import.meta.url), NODE_HOSTS_OPTION, options], {
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
  });
  if (r.status !== 0) throw new Error(`node ${compiler.flag} failed (${r.error ?? `exit ${r.status ?? r.signal}`}): ${r.stderr.trim()}`);
  return JSON.parse(r.stdout);
}

// ------------------------------------------------------------------ Wasmtime

const root = fileURLToPath(new URL('..', import.meta.url));
const wasmtime = path.join(root, '.tools', 'wasmtime', 'wasmtime');
const encoder = new TextEncoder();

// A WASI command module that sends each request to the preloaded `feel` module and writes each
// response, followed by a newline, to stdout. Responses are JSON without raw newlines.
function commandModule(requests) {
  const bytes = requests.map((json) => encoder.encode(json));
  const watString = (b) => [...b].map((x) => `\\${x.toString(16).padStart(2, '0')}`).join('');
  const calls = bytes
    .map(
      (b, i) => `
    (local.set $p (call $alloc (i32.const ${b.length})))
    (memory.init $r${i} (local.get $p) (i32.const 0) (i32.const ${b.length}))
    (data.drop $r${i})
    (call $respond (call $call (local.get $p) (i32.const ${b.length})))`,
    )
    .join('');
  return `(module
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "feel" "memory" (memory 1))
  (import "feel" "feel_alloc" (func $alloc (param i32) (result i32)))
  (import "feel" "feel_call" (func $call (param i32 i32) (result i64)))
  (import "feel" "feel_free" (func $free (param i32 i32)))
  ;; fd_write reads the iovecs from the memory its caller exports
  (export "memory" (memory 0))
  ${bytes.map((b, i) => `(data $r${i} "${watString(b)}")`).join('\n  ')}
  (global $scratch (mut i32) (i32.const 0))
  ;; writes n bytes at p to stdout (fd_write may write fewer than asked)
  (func $write (param $p i32) (param $n i32)
    (local $iov i32)
    (local.set $iov (global.get $scratch))
    (block $done
      (loop $more
        (br_if $done (i32.eqz (local.get $n)))
        (i32.store (local.get $iov) (local.get $p))
        (i32.store offset=4 (local.get $iov) (local.get $n))
        (if (call $fd_write (i32.const 1) (local.get $iov) (i32.const 1) (i32.add (local.get $iov) (i32.const 8)))
          (then unreachable))
        (local.set $p (i32.add (local.get $p) (i32.load offset=8 (local.get $iov))))
        (local.set $n (i32.sub (local.get $n) (i32.load offset=8 (local.get $iov))))
        (br $more))))
  (func $respond (param $packed i64)
    (local $ptr i32) (local $len i32)
    (local.set $ptr (i32.wrap_i64 (i64.shr_u (local.get $packed) (i64.const 32))))
    (local.set $len (i32.wrap_i64 (local.get $packed)))
    (call $write (local.get $ptr) (local.get $len))
    (call $write (i32.add (global.get $scratch) (i32.const 12)) (i32.const 1))
    (call $free (local.get $ptr) (local.get $len)))
  (func (export "_start")
    (local $p i32)
    ;; scratch: an iovec (8 bytes), the written count (4) and a newline (1)
    (global.set $scratch (call $alloc (i32.const 16)))
    (i32.store8 offset=12 (global.get $scratch) (i32.const 10))${calls}))
`;
}

// Runs Wasmtime processes on one precompiled copy of the module.
async function wasmtimeHost(modulePath) {
  if (!existsSync(wasmtime)) throw new Error(`Wasmtime not found at ${wasmtime}: run ./scripts/bootstrap.sh`);
  const dir = await mkdtemp(path.join(os.tmpdir(), 'feel-stack-'));
  const close = () => rm(dir, { recursive: true, force: true });
  const exec = (args) => spawnSync(wasmtime, args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  const precompiled = path.join(dir, 'feel.cwasm');
  let n = 0;
  // Sends the requests in order in one process; returns their responses, and for a trap the
  // trap message in place of the response of the request that trapped (later ones are not sent).
  // A Wasmtime failure that is not a wasm trap is a failure of the host and throws.
  const run = async (requests, stackBytes, wat = commandModule(requests)) => {
    const command = path.join(dir, `command-${n++}.wat`);
    await writeFile(command, wat);
    const r = exec(['run', '-C', 'cache=n', '-W', `max-wasm-stack=${stackBytes}`, '--allow-precompiled', '--preload', `feel=${precompiled}`, command]);
    const responses = r.stdout.split('\n').filter((line) => line !== '').map((line) => JSON.parse(line));
    if (r.status !== 0) {
      const trap = r.stderr.match(/wasm trap: (.*)/)?.[1];
      if (trap === undefined) throw new Error(`wasmtime failed (${r.error ?? `exit ${r.status ?? r.signal}`}): ${r.stderr.trim().split('\n').pop()}`);
      responses.push({ trap });
    }
    return responses;
  };
  try {
    const compiled = exec(['compile', '-C', 'cache=n', '-o', precompiled, modulePath]);
    if (compiled.status !== 0) throw new Error(`wasmtime compile failed: ${compiled.error ?? compiled.stderr}`);
    // A preloaded reactor must arrive initialized: its `_initialize` traps (`unreachable`) when
    // called again.
    const again = await run([], WASMTIME_STACK, '(module (import "feel" "_initialize" (func $init)) (func (export "_start") (call $init)))');
    const trap = again.at(-1)?.trap;
    if (!/unreachable/.test(trap ?? '')) throw new Error(`Wasmtime did not call _initialize of the preloaded module (${trap ?? 'the second call returned'})`);
  } catch (e) {
    await close();
    throw e;
  }
  return { run, close };
}

async function runWasmtime(host, limits, stackBytes, floor, report) {
  const result = { flat: {} };
  if (report) {
    for (const construct of flat) {
      const d = limits[construct.name];
      result.flat[construct.name] = outcome(construct, d, (await host.run([evaluate(construct, d)], stackBytes))[0]);
    }
    const attempt = async (d) => outcome(recursion, d, (await host.run([evaluate(recursion, d)], stackBytes))[0]);
    result.deepest = await deepestRecursion(attempt, limits[recursion.name]);
  } else {
    const requests = [...flat.map((c) => evaluate(c, limits[c.name])), evaluate(recursion, floor)];
    const responses = await host.run(requests, stackBytes);
    const sent = (i) => responses[i] ?? { trap: 'not sent (an earlier request trapped)' };
    flat.forEach((c, i) => (result.flat[c.name] = outcome(c, limits[c.name], sent(i))));
    result.recursion = outcome(recursion, floor, sent(flat.length));
  }
  return result;
}

// ------------------------------------------------------------------ main

// One line per host, and the failed attempts with their outcomes.
function summary(label, result, floor) {
  const failures = [];
  const flatFailed = Object.entries(result.flat).filter(([, o]) => o !== 'ok');
  for (const [name, o] of flatFailed) failures.push(`${label}: ${name} at the AST limit: ${o}`);
  let line = `${label}: flat nesting at the AST limit ${flatFailed.length ? `FAILED (${flatFailed.map(([n]) => n).join(', ')})` : 'ok'}`;
  if (result.recursion !== undefined) {
    line += `, recursion at depth ${floor} ${result.recursion === 'ok' ? 'ok' : 'FAILED'}`;
    if (result.recursion !== 'ok') failures.push(`${label}: recursion at depth ${floor}: ${result.recursion}`);
  }
  if (result.deepest !== undefined) {
    const { depth, failure } = result.deepest;
    line += `; recursion reaches depth ${depth}${failure ? ` (at ${depth + 1}: ${failure})` : ' (the AST limit)'}`;
  }
  return { line, failures };
}

const mib = (bytes) => (bytes >= 1024 * 1024 ? `${bytes / (1024 * 1024)} MiB` : `${bytes / 1024} KiB`);

async function main(args) {
  const report = args.includes('--report');
  const modulePath = path.resolve(args.find((a) => !a.startsWith('--')) ?? path.join(root, 'dist', 'feel.wasm'));
  console.log(`stack headroom${report ? ' report' : ''}: ${modulePath}`);

  const limits = await astLimits(await nodeHost(await WebAssembly.compile(await readFile(modulePath))));
  console.log(`deepest nesting the parse op accepts: ${Object.entries(limits).map(([n, d]) => `${n} ${d}`).join(', ')}`);
  for (const [host, floor] of [['worker', WORKER_FLOOR], ['Wasmtime', WASMTIME_FLOOR]]) {
    if (floor > limits[recursion.name]) throw new Error(`the ${host} floor ${floor} is above the AST limit of the recursion (${limits[recursion.name]})`);
  }

  const failures = [];
  const show = (label, result, floor = null) => {
    const s = summary(label, result, floor);
    console.log(s.line);
    if (report) s.failures.forEach((f) => console.log(`  ${f}`));
    failures.push(...s.failures);
  };
  for (const compiler of V8_COMPILERS) {
    const { main, workers } = runNodeProcess(compiler, modulePath, limits, report);
    show(`Node main thread, ${compiler.name}`, main);
    for (const { stackMb, result } of workers) show(`Node worker (${stackMb} MiB stack), ${compiler.name}`, result, WORKER_FLOOR);
  }
  const host = await wasmtimeHost(modulePath);
  try {
    for (const stack of report ? REPORT_WASMTIME_STACKS : [WASMTIME_STACK]) {
      show(`Wasmtime (max-wasm-stack ${mib(stack)})`, await runWasmtime(host, limits, stack, WASMTIME_FLOOR, report), WASMTIME_FLOOR);
    }
  } finally {
    await host.close();
  }

  if (report) return 0;
  for (const f of failures) console.error(`FAIL ${f}`);
  console.log(failures.length ? `stack headroom: ${failures.length} check(s) failed` : 'stack headroom ok');
  return failures.length ? 1 : 0;
}

// The Node hosts of one process (see runNodeProcess): main thread, then the workers one after
// another.
async function nodeHosts({ modulePath, limits, report }) {
  const main = await runThread(modulePath, limits, null, report);
  const workers = [];
  for (const stackMb of report ? REPORT_WORKER_STACKS_MB : [WORKER_STACK_MB]) {
    workers.push({ stackMb, result: await runWorker(modulePath, limits, WORKER_FLOOR, report, stackMb) });
  }
  return { main, workers };
}

if (!isMainThread) {
  const { modulePath, limits, floor, report } = workerData;
  parentPort.postMessage(await runThread(modulePath, limits, floor, report));
} else {
  const args = process.argv.slice(2);
  const hostsAt = args.indexOf(NODE_HOSTS_OPTION);
  const run = hostsAt >= 0 ? nodeHosts(JSON.parse(args[hostsAt + 1])).then((r) => (console.log(JSON.stringify(r)), 0)) : main(args);
  // exitCode, not exit(): output to a pipe is written asynchronously and must not be cut off
  run.then(
    (code) => (process.exitCode = code),
    (e) => {
      console.error(`stack headroom: ${e.message}`);
      process.exitCode = 2;
    },
  );
}
