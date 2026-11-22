import { test } from 'node:test';
import assert from 'node:assert/strict';
import { instantiateFeel, createWasiShim, rawJson, isRawJson, trapMessage } from '../dist/index.js';
import { wasmBytes, trappingModuleBytes } from './helpers.mjs';

const feel = await instantiateFeel(await wasmBytes(), { tzOffsetMinutes: -240 });
const ev = (expression, extra = {}) => feel.call({ op: 'evaluate', expression, ...extra });

test('imports are WASI functions the 10-function shim provides (CONTRACT §1)', async () => {
  const module = await WebAssembly.compile(await wasmBytes());
  const imports = WebAssembly.Module.imports(module);
  const shim = createWasiShim();
  const provided = Object.keys(shim.imports.wasi_snapshot_preview1).sort();
  assert.equal(provided.length, 10);
  assert.deepEqual(imports.map((i) => i.module).filter((m) => m !== 'wasi_snapshot_preview1'), []);
  const names = imports.map((i) => i.name).sort();
  assert.deepEqual(names.filter((n) => !provided.includes(n)), [], 'imports outside the shim list');
  // the pure-Rust engine (dashu backend) needs only these; the C library needed the fd_* ones too
  assert.deepEqual(names, ['clock_time_get', 'environ_get', 'environ_sizes_get', 'fd_write', 'proc_exit', 'random_get']);
});

test('info', () => {
  const r = feel.call({ op: 'info' });
  assert.equal(r.ok, true);
  assert.equal(r.engine, 'rheon');
  assert.equal(r.numberBackend, 'dashu');
  assert.match(r.engineCommit, /^[0-9a-f]{40}(-dirty)?$/);
  assert.match(r.upstreamCommit, /^[0-9a-f]{40}$/);
});

test('exact decimal arithmetic', () => {
  assert.equal(ev('0.1 + 0.2').result.display, '0.3');
  assert.equal(ev('1/3').result.display, '0.' + '3'.repeat(34));
  assert.equal(ev('exp(1)').result.display, '2.718281828459045235360287471352662');
  assert.equal(ev('decimal(-2.5, 0)').result.display, '-2');
  const r = ev('9007199254740993 + 0');
  assert.equal(r.result.display, '9007199254740993');
  assert.equal(r.result.type, 'number');
  assert.equal(typeof r.elapsedMicros, 'number');
});

test('rawJson context keeps exact digits end-to-end; JS objects go through f64', () => {
  const ctx = '{"x": 0.1000000000000000000000000000000001, "big": 12345678901234567890123, "tiny": 0.00000000001}';
  const r = ev('big + tiny', { context: rawJson(ctx) });
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.equal(r.result.display, '12345678901234567890123.00000000001'); // 34 significant digits
  const exact = ev('x', { context: rawJson(ctx) });
  assert.equal(exact.result.display, '0.1000000000000000000000000000000001');
  const viaF64 = ev('x', { context: { x: 0.1000000000000000000000000000000001 } });
  assert.equal(viaF64.result.display, '0.1');
});

test('context with spaced names, FEEL context string, nested output', () => {
  const r = ev("Grace's age + 1", { context: { "Grace's age": 40 } });
  assert.equal(r.result.display, '41');
  const f = ev('b * 2', { context: '{a: 1, b: a + 1}', contextFormat: 'feel' });
  assert.equal(f.result.display, '4');
  const l = ev('{total: sum(items), avg: mean(items)}', { context: { items: [1, 2, 4] } });
  assert.equal(l.result.display, '{"total": 7, "avg": 2.333333333333333333333333333333333}');
  assert.match(l.result.type, /^context</);
});

test('unary tests', () => {
  const ut = (expression, input) => feel.call({ op: 'evaluate', dialect: 'unaryTests', expression, input });
  assert.equal(ut('[1..10]', 5).result.display, 'true');
  assert.equal(ut('> 5, < 2', 1).result.display, 'true');
  assert.equal(ut('> 5, < 2', 3).result.display, 'false');
  assert.equal(ut('"a","b"', 'b').result.display, 'true');
  assert.equal(ut('-', 42).result.display, 'true');
  const exact = ut('> 0.1', rawJson('0.1000000000000000000000000000000001'));
  assert.equal(exact.result.display, 'true');
  const missing = feel.call({ op: 'evaluate', dialect: 'unaryTests', expression: '> 5' });
  assert.equal(missing.ok, true);
  assert.ok(missing.warnings.length >= 1);
});

test('parse', () => {
  const p = feel.call({ op: 'parse', expression: 'x + y * 2', context: { x: 1, y: 2 } });
  assert.equal(p.ok, true);
  assert.match(p.tree, /Mul/);
  assert.doesNotMatch(p.tree, /\x1b\[/);
  const u = feel.call({ op: 'parse', dialect: 'unaryTests', expression: '[1..10], > 20' });
  assert.equal(u.ok, true);
  const bad = feel.call({ op: 'parse', expression: '1 +' });
  assert.equal(bad.ok, false);
  assert.equal(bad.error.kind, 'syntax');
});

test('errors and nulls keep the instance healthy', () => {
  assert.equal(ev('1 +').error.kind, 'syntax');
  const n = ev('1 + "a"');
  assert.equal(n.ok, true);
  assert.equal(n.result.isNull, true);
  assert.equal(typeof n.result.nullReason, 'string');
  assert.equal(feel.call('not json').error.kind, 'request');
  assert.equal(feel.call({ op: 'nope' }).error.kind, 'request');
  assert.equal(ev('1', { context: rawJson('{oops') }).error.kind, 'context');
  assert.equal(ev('1', { context: rawJson('[1]') }).error.kind, 'context');
  assert.equal(ev('1', { context: '{a: }', contextFormat: 'feel' }).error.kind, 'context');
  assert.equal(ev('2 * 21').result.display, '42');
});

test('tzOffsetMinutes: env default and per-request override', () => {
  // today() differs between UTC+14 and UTC-12 at any instant (26 h apart)
  const east = ev('string(today())', { tzOffsetMinutes: 840 }).result.feel;
  const west = ev('string(today())', { tzOffsetMinutes: -720 }).result.feel;
  assert.notEqual(east, west);
  const t = ev('now()').result.feel; // env tz -240 applied
  assert.match(t, /-04:00|T/);
  assert.equal(ev('1', { tzOffsetMinutes: 5000 }).error.kind, 'request');
});

test('now: a fixed instant for now() and today() of one request', () => {
  const pinned = { now: '2001-02-03T04:05:06Z', tzOffsetMinutes: 0 };
  assert.equal(ev('string(today())', pinned).result.feel, '"2001-02-03"');
  assert.equal(ev('string(now())', { ...pinned, tzOffsetMinutes: 120 }).result.feel, '"2001-02-03T06:05:06"');
  assert.equal(ev('1', { now: '2001-02-03' }).error.kind, 'request');
  assert.notEqual(ev('string(today())', { tzOffsetMinutes: 0 }).result.feel, '"2001-02-03"');
});

test('deep nesting is a syntax error, deep recursion an evaluation error (no crash)', () => {
  for (const e of ['1' + '+1'.repeat(20000), '['.repeat(5000) + '1' + ']'.repeat(5000), '{a:'.repeat(3000) + '1' + '}'.repeat(3000)]) {
    const r = ev(e);
    assert.equal(r.error.kind, 'syntax', e.slice(0, 20));
    assert.match(r.error.message, /nested too deeply/);
  }
  // a syntax error after a very deep left operand (partial AST dropped by the parser)
  assert.equal(ev('1' + '+1'.repeat(20000) + '+').error.kind, 'syntax');
  const r = ev('{f: function(n) f(n + 1), r: f(1)}.r');
  assert.equal(r.error.kind, 'evaluation');
  assert.equal(ev('{f: function(n) if n = 0 then 0 else 1 + f(n-1), r: f(150)}.r').result.display, '150');
  assert.equal(ev('2 + 2').result.display, '4');
});

test('a module without _initialize is rejected (CONTRACT §1)', async () => {
  await assert.rejects(instantiateFeel(trappingModuleBytes({ initialize: false })), /missing export '_initialize'/);
});

test('rawJson is branded: a real context with a single "rawJson" key is data', () => {
  const r = feel.call({ op: 'evaluate', expression: 'rawJson', context: { rawJson: 'hello' } });
  assert.equal(r.result.display, '"hello"');
  const spliced = feel.call({ op: 'evaluate', expression: 'x', context: rawJson('{"x": 0.1000000000000000000000000000000001}') });
  assert.equal(spliced.result.display, '0.1000000000000000000000000000000001');
  assert.equal(isRawJson(rawJson('1')), true);
  assert.equal(isRawJson({ rawJson: '1' }), false);
});

test('unpaired UTF-16 surrogates are rejected with a clear kind before reaching the core', () => {
  const ctx = feel.call({ op: 'evaluate', expression: 'x', context: rawJson('{"x":"\\ud800"}') });
  assert.equal(ctx.error.kind, 'context');
  assert.match(ctx.error.message, /unpaired UTF-16 surrogate/);
  assert.equal(feel.call({ op: 'evaluate', expression: 'x', context: { x: '\ud800' } }).error.kind, 'context');
  assert.equal(feel.call({ op: 'evaluate', expression: 'x', context: { ['k\udc00']: 1 } }).error.kind, 'context');
  assert.equal(feel.call({ op: 'evaluate', expression: '"\ud800"' }).error.kind, 'syntax');
  assert.equal(feel.call({ op: 'evaluate', dialect: 'unaryTests', expression: '"a"', input: '\ud800' }).error.kind, 'context');
  // valid pairs (raw and escaped) are fine, as is an escaped backslash followed by "ud800"
  assert.equal(feel.call({ op: 'evaluate', expression: 'x', context: rawJson('{"x":"\\ud83d\\ude00"}') }).result.display, '"😀"');
  assert.equal(feel.call({ op: 'evaluate', expression: 'x', context: { x: '😀' } }).result.display, '"😀"');
  assert.equal(feel.call({ op: 'evaluate', expression: 'x', context: rawJson('{"x":"\\\\ud800"}') }).result.display, '"\\\\ud800"');
  // the FEEL lexer escape forms no longer crash the engine
  assert.equal(feel.call({ op: 'evaluate', expression: '"\\uD800"' }).error.kind, 'syntax');
  assert.equal(feel.call({ op: 'evaluate', expression: '"\\uD83D\\uDE40"' }).result.display, '"🙀"');
});

test('instantiateFeel re-creates an instance whose memory grew past recycleMemoryBytes', async () => {
  const small = await instantiateFeel(await wasmBytes(), { recycleMemoryBytes: 32 * 1024 * 1024, onOutput: () => {} });
  const r = small.call({ op: 'evaluate', expression: '{f: function(s, n) if n = 0 then string length(s) else f(s + s, n-1), r: f("ab", 22)}.r' });
  assert.equal(r.result.display, String(2 ** 23));
  assert.equal(small.call({ op: 'evaluate', expression: '1 + 1' }).result.display, '2');
});

test('wasm trap (unreachable) → crash response and re-instantiation', async () => {
  const lines = [];
  const trap = await instantiateFeel(trappingModuleBytes(), { onOutput: (fd, l) => lines.push([fd, l]) });
  const r1 = trap.call({ op: 'info' });
  assert.equal(r1.ok, false);
  assert.equal(r1.error.kind, 'crash');
  assert.match(r1.error.message, /unreachable/);
  const r2 = trap.call({ op: 'info' }); // fresh instance, traps again (not "dead instance")
  assert.match(r2.error.message, /unreachable/);
});

test('1000 consecutive calls on one instance stay correct', () => {
  for (let i = 0; i < 1000; i++) assert.equal(ev('count(for i in 1..50 return i / 7)').result.display, '50');
});

test('trapMessage names a failed allocation at the memory cap as out of memory', () => {
  const stderr = ['memory allocation of 268435456 bytes failed'];
  const message = trapMessage('RuntimeError: unreachable', stderr);
  assert.match(message, /^out of memory: .*512 MiB.*256 MiB/);
  assert.ok(message.includes('RuntimeError: unreachable\nmemory allocation of 268435456 bytes failed'));
  // other traps keep the plain form
  assert.equal(trapMessage('RuntimeError: unreachable', ['thread panicked at x']), 'RuntimeError: unreachable\nthread panicked at x');
  assert.equal(trapMessage('RuntimeError: unreachable', []), 'RuntimeError: unreachable');
});

test('evaluateAll: several expressions over one context, each answered as evaluate answers it', () => {
  const context = { price: 12.5, qty: 4, lines: [{ n: 1 }, { n: 2 }] };
  const evaluations = [
    { id: 'total', expression: 'price * qty' },
    { expression: 'sum(lines.n)' },
    { id: 7, dialect: 'unaryTests', expression: '> 10', input: 11 },
    { expression: '1 +' },
    { expression: 'price + "a"', errorMode: 'strict' },
  ];
  const all = feel.call({ op: 'evaluateAll', context, evaluations });
  assert.equal(all.ok, true);
  assert.deepEqual(all.contextWarnings, []);
  assert.equal(all.results.length, evaluations.length);
  for (const [index, entry] of evaluations.entries()) {
    const { id, ...fields } = entry;
    const single = feel.call({ op: 'evaluate', context, ...fields });
    const { elapsedMicros: _a, id: answerId, ...answer } = all.results[index];
    const { elapsedMicros: _b, ...expected } = single;
    assert.deepEqual(answer, expected, entry.expression);
    assert.equal(answerId, id);
  }
  assert.equal(all.results[0].result.display, '50');
  assert.equal(all.results[3].error.kind, 'syntax');
  assert.equal(all.results[4].error.kind, 'evaluation');
});

test('evaluateAll: raw JSON context and inputs keep their digits; an entry that can not be sent fails in its place', () => {
  const all = feel.call({
    op: 'evaluateAll',
    context: rawJson('{"x": 0.1000000000000000000000000000000001}'),
    evaluations: [
      { id: 'a', expression: 'x + 0' },
      { id: 'b', dialect: 'unaryTests', expression: '> 1', input: rawJson('{') },
      { id: 'c', dialect: 'unaryTests', expression: '1.000000000000000000001', input: rawJson('1.000000000000000000001') },
      { id: 'd', expression: '"\ud800"' },
      'not an object',
      { id: 'e', expression: 'x > 0' },
    ],
  });
  assert.equal(all.ok, true);
  assert.deepEqual(all.results.map((r) => r.id), ['a', 'b', 'c', 'd', undefined, 'e']);
  assert.deepEqual(all.results.map((r) => r.ok), [true, false, true, false, false, true]);
  assert.equal(all.results[0].result.display, '0.1000000000000000000000000000000001');
  assert.equal(all.results[1].error.kind, 'context');
  assert.equal(all.results[2].result.display, 'true');
  assert.equal(all.results[3].error.kind, 'syntax');
  assert.equal(all.results[4].error.kind, 'request');
  assert.equal(all.results[5].result.display, 'true');
});

test('evaluateAll: a failure of the request has no results; the host offset is filled in', () => {
  const bad = feel.call({ op: 'evaluateAll', context: rawJson('{'), evaluations: [{ expression: '1' }] });
  assert.deepEqual([bad.ok, bad.error.kind, bad.results], [false, 'context', undefined]);
  const none = feel.call({ op: 'evaluateAll', evaluations: 'x' });
  assert.deepEqual([none.ok, none.error.kind], [false, 'request']);
  const empty = feel.call({ op: 'evaluateAll', evaluations: [] });
  assert.deepEqual([empty.ok, empty.results], [true, []]);
  // tzOffsetMinutes: -240 of this instance
  const hour = (display) => display.slice(0, 14);
  const zone = feel.call({ op: 'evaluateAll', evaluations: [{ expression: 'string(now())' }] });
  assert.equal(hour(zone.results[0].result.display), hour(ev('string(now())').result.display));
  const utc = feel.call({ op: 'evaluateAll', tzOffsetMinutes: 0, evaluations: [{ expression: 'string(now())' }] });
  assert.notEqual(hour(utc.results[0].result.display), hour(zone.results[0].result.display));
});

test('functions that see their own context leave no memory behind', async () => {
  // a context literal whose functions and entries hold each other: the scopes of the functions
  // are freed with the evaluation, so the memory of the instance does not grow with the calls
  const { RawFeelInstance } = await import('../dist/index.js');
  const raw = await RawFeelInstance.create(await WebAssembly.compile(await wasmBytes()));
  for (const expression of [
    '{f: function() 1, g: f, big: for i in 1..20000 return i, r: g()}.r',
    '{o: {h: function() 1}, big: for i in 1..20000 return i, f: function() o.h(), r: f()}.r',
    '{g: function() 1, f: function() g(), big: for i in 1..20000 return i, r: f()}.r',
    '{f: function(n) if n = 0 then 0 else f(n - 1), big: for i in 1..20000 return i, r: f(20)}.r',
  ]) {
    const json = JSON.stringify({ op: 'evaluate', expression });
    assert.equal(JSON.parse(raw.callJson(json)).ok, true, expression);
    const before = raw.memoryBytes;
    for (let i = 0; i < 300; i++) raw.callJson(json);
    assert.ok(raw.memoryBytes - before < 8 * 1024 * 1024, `${expression}: grew by ${raw.memoryBytes - before} bytes`);
  }
});

test('the check for unpaired surrogates in JSON text reads every escape as a walk over all characters does', async () => {
  const { hasLoneSurrogate } = await import('../dist/request.js');
  // the walk over every character: the definition
  const byCharacter = (json) => {
    if (!json.isWellFormed()) return true;
    for (let i = 0; i < json.length; i++) {
      if (json[i] !== '\\') continue;
      if (json[i + 1] !== 'u') { i++; continue; }
      const code = parseInt(json.slice(i + 2, i + 6), 16);
      if (code >= 0xd800 && code <= 0xdbff) {
        if (json[i + 6] === '\\' && json[i + 7] === 'u') {
          const low = parseInt(json.slice(i + 8, i + 12), 16);
          if (low >= 0xdc00 && low <= 0xdfff) { i += 11; continue; }
        }
        return true;
      }
      if (code >= 0xdc00 && code <= 0xdfff) return true;
      i += 5;
    }
    return false;
  };
  const pieces = ['a', '"', ' ', '\\\\', '\\"', '\\n', '\\u0041', '\\ud83d', '\\uD83D', '\\ude00', '\\uDE00', '\\ud83d\\ude00', '\\', 'u', 'd83d', '\\u', '\\ud8', '😀', '\ud83d', 'é'];
  let state = 0x9e3779b9;
  const next = (below) => { state ^= state << 13; state >>>= 0; state ^= state >>> 17; state ^= state << 5; state >>>= 0; return state % below; };
  let unpaired = 0;
  const cases = 40000;
  for (let n = 0; n < cases; n++) {
    const text = Array.from({ length: next(9) }, () => pieces[next(pieces.length)]).join('');
    const expected = byCharacter(text);
    unpaired += expected ? 1 : 0;
    assert.equal(hasLoneSurrogate(text), expected, JSON.stringify(text));
  }
  assert.ok(unpaired > cases / 10 && unpaired < cases - cases / 10, `${unpaired} of ${cases} texts have an unpaired surrogate`);
});
