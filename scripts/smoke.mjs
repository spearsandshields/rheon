#!/usr/bin/env node
// Smoke test for dist/feel.wasm under node:wasi: runs a few requests through the raw
// feel_alloc / feel_call / feel_free ABI (docs/CONTRACT.md §1) and prints the responses.
// Usage: node scripts/smoke.mjs [path/to/feel.wasm] ['{"op":"evaluate",...}' ...]
// Exits non-zero if a built-in check fails.
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { instantiate, call as callRaw } from './feel-abi.mjs';

const defaultWasm = fileURLToPath(new URL('../dist/feel.wasm', import.meta.url));
const args = process.argv.slice(2);
const wasmPath = args[0] && !args[0].trim().startsWith('{') ? args.shift() : defaultWasm;

const module = await WebAssembly.compile(await readFile(wasmPath));
const ex = await instantiate(module);
const call = (request) => callRaw(ex, typeof request === 'string' ? request : JSON.stringify(request));

const tz = -new Date().getTimezoneOffset();
const checks = [
  [{ op: 'info' }, (r) => r.ok && r.numberBackend === 'dashu'],
  [{ op: 'evaluate', expression: '0.1 + 0.2' }, (r) => r.result?.display === '0.3'],
  [{ op: 'evaluate', expression: '1/3' }, (r) => r.result?.display === '0.' + '3'.repeat(34)],
  [{ op: 'evaluate', expression: 'exp(1)' }, (r) => r.result?.display === '2.718281828459045235360287471352662'],
  [{ op: 'evaluate', expression: 'decimal(-2.5, 0)' }, (r) => r.result?.display === '-2'],
  ['{"op":"evaluate","expression":"Mike\'s age + 0.1","context":{"Mike\'s age":40.000000000000000000001}}',
    (r) => r.result?.display === '40.100000000000000000001'],
  [{ op: 'evaluate', expression: '{total: sum(items), avg: mean(items)}', context: { items: [1, 2, 4] } },
    (r) => r.result?.display === '{"avg": 2.333333333333333333333333333333333, "total": 7}'],
  [{ op: 'evaluate', dialect: 'unaryTests', expression: '> 5, < 2', input: 1 }, (r) => r.result?.display === 'true'],
  [{ op: 'evaluate', expression: 'today() = date(now())', tzOffsetMinutes: tz }, (r) => r.result?.display === 'true'],
  [{ op: 'evaluate', expression: 'string(now())', tzOffsetMinutes: tz }, (r) => r.ok],
  [{ op: 'evaluate', expression: '1 + "a"' }, (r) => r.result?.isNull && typeof r.result.nullReason === 'string'],
  [{ op: 'evaluate', expression: '1 +' }, (r) => r.ok === false && r.error.kind === 'syntax'],
  [{ op: 'parse', expression: 'x + y * 2', context: { x: 1, y: 2 } }, (r) => r.ok && r.tree.includes('Mul')],
  ['not json', (r) => r.ok === false && r.error.kind === 'request'],
];

let failed = 0;
if (args.length) {
  for (const a of args) console.log(JSON.stringify(call(a), null, 2));
} else {
  for (const [request, check] of checks) {
    const response = call(request);
    const ok = check(response);
    if (!ok) failed++;
    console.log(`${ok ? 'PASS' : 'FAIL'} ${typeof request === 'string' ? request : JSON.stringify(request)}`);
    console.log('     ' + JSON.stringify(response));
  }
  // leak check: memory must not keep growing over many calls
  const leakReq = { op: 'evaluate', expression: 'for i in 1..20 return i / 7' };
  for (let i = 0; i < 100; i++) call(leakReq); // warm up the allocator
  const before = ex.memory.buffer.byteLength;
  for (let i = 0; i < 2000; i++) call(leakReq);
  const after = ex.memory.buffer.byteLength;
  const leakOk = after === before;
  if (!leakOk) failed++;
  console.log(`${leakOk ? 'PASS' : 'FAIL'} 2000 calls: memory ${before} -> ${after} bytes`);
  // memory check: 26 doublings of a string, on a fresh instance, build a 128 MiB result under the
  // 512 MiB cap; shared strings keep the growth of linear memory near the live data (each level's
  // string plus the result), at most 2.2 times the result
  const doubling = { op: 'evaluate', expression: '{f: function(s, n) if n = 0 then string length(s) else f(s + s, n-1), r: f("ab", 26)}.r' };
  const fresh = await instantiate(module);
  const initial = fresh.memory.buffer.byteLength;
  let outcome;
  try {
    outcome = callRaw(fresh, JSON.stringify(doubling)).result?.display;
  } catch (trap) {
    outcome = `trap: ${trap.message}`;
  }
  const growth = fresh.memory.buffer.byteLength - initial;
  const resultBytes = 2 ** 27;
  const memoryOk = outcome === String(resultBytes) && growth <= 2.2 * resultBytes;
  if (!memoryOk) failed++;
  console.log(`${memoryOk ? 'PASS' : 'FAIL'} 26 string doublings: ${outcome}, memory grew by ${(growth / resultBytes).toFixed(2)} times the result`);
  console.log(failed ? `${failed} check(s) failed` : 'smoke ok');
}
process.exit(failed ? 1 : 0);
