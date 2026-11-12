import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FeelEngine, rawJson } from '../dist/index.js';
import { wasmBytes, createNodeWorker, trappingModuleBytes } from './helpers.mjs';

const module = await WebAssembly.compile(await wasmBytes());

test('real worker (worker_threads running dist/worker.js): evaluate, parse, info, FIFO', async (t) => {
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: createNodeWorker, timeoutMs: 10000 });
  t.after(() => engine.dispose());
  const info = await engine.info();
  assert.equal(info.numberBackend, 'dashu');
  const [a, b, c] = await Promise.all([
    engine.evaluate({ expression: '1/3' }),
    engine.evaluate({ expression: 'x + 0.2', context: rawJson('{"x": 0.1}') }),
    engine.evaluate({ dialect: 'unaryTests', expression: '[1..10]', input: 11 }),
  ]);
  assert.equal(a.result.display, '0.' + '3'.repeat(34));
  assert.equal(b.result.display, '0.3');
  assert.equal(c.result.display, 'false');
  const p = await engine.parse('expression', 'first name + 1', { context: { 'first name': 1 } });
  assert.equal(p.ok, true);
  assert.match(p.tree, /first name/);
  const bad = await engine.evaluate({ expression: '1', context: rawJson('{') });
  assert.equal(bad.error.kind, 'context');
  // local tz is filled in automatically: today() must match the host's local date
  const d = new Date();
  const iso = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  const today = await engine.evaluate({ expression: 'string(today())' });
  assert.equal(today.result.display, JSON.stringify(iso));
});

test('real worker: timeout terminates the worker, next request works on a fresh one', async (t) => {
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: createNodeWorker, timeoutMs: 400 });
  t.after(() => engine.dispose());
  const t0 = Date.now();
  // ~5 s of work (10M loop iterations in constant memory): far past the 400 ms budget on any host
  const slow = engine.evaluate({ expression: 'count(for i in 1..10000 return sum(for j in 1..1000 return j))' });
  const next = engine.evaluate({ expression: '6 * 7' }); // queued behind the slow one
  const r = await slow;
  const elapsed = Date.now() - t0;
  assert.equal(r.ok, false);
  assert.equal(r.error.kind, 'timeout');
  assert.ok(elapsed >= 400 && elapsed < 3000, `elapsed ${elapsed}`);
  assert.equal((await next).result.display, '42');
  assert.equal(engine.workerStarts, 2);
});

test('real worker: deep nesting and deep recursion are errors, not crashes (worker kept)', async (t) => {
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: createNodeWorker, timeoutMs: 10000 });
  t.after(() => engine.dispose());
  const deep = await engine.evaluate({ expression: '1' + '+1'.repeat(20000) });
  assert.equal(deep.ok, false);
  assert.equal(deep.error.kind, 'syntax');
  assert.match(deep.error.message, /nested too deeply/);
  const recursion = await engine.evaluate({ expression: '{f: function(n) if n = 0 then 0 else 1 + f(n-1), r: f(5000)}.r' });
  assert.equal(recursion.error.kind, 'evaluation');
  assert.match(recursion.error.message, /nested too deeply/);
  assert.equal((await engine.evaluate({ expression: '1 + 1' })).result.display, '2');
  assert.equal(engine.workerStarts, 1);
});

test('real worker: a runaway allocation traps early (512 MiB cap) → crash, worker replaced', async (t) => {
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: createNodeWorker, timeoutMs: 20000 });
  t.after(() => engine.dispose());
  const r = await engine.evaluate({ expression: '{f: function(s, n) if n = 0 then string length(s) else f(s + s, n-1), r: f("ab", 40)}.r' });
  assert.equal(r.error.kind, 'crash');
  assert.match(r.error.message, /memory allocation/);
  assert.equal((await engine.evaluate({ expression: '1 + 1' })).result.display, '2');
  assert.equal(engine.workerStarts, 2);
});

test('real worker: a worker whose memory grew past recycleMemoryBytes is replaced after the call', async (t) => {
  const engine = await FeelEngine.create({
    wasmModule: module,
    createWorker: createNodeWorker,
    timeoutMs: 20000,
    recycleMemoryBytes: 32 * 1024 * 1024, // the evaluation below grows the memory from 9 to about 57 MiB
  });
  t.after(() => engine.dispose());
  const r = await engine.evaluate({ expression: '{f: function(s, n) if n = 0 then string length(s) else f(s + s, n-1), r: f("ab", 23)}.r' });
  assert.equal(r.result.display, String(2 ** 24));
  assert.equal((await engine.evaluate({ expression: '1 + 1' })).result.display, '2');
  assert.equal(engine.workerStarts, 2);
  // small calls do not recycle
  await engine.evaluate({ expression: '2 + 2' });
  assert.equal(engine.workerStarts, 2);
});

test('real worker: cancelling a running runaway evaluation frees the engine at once', async (t) => {
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: createNodeWorker, timeoutMs: 5000, cancelGraceMs: 50 });
  t.after(() => engine.dispose());
  const t0 = Date.now();
  const first = new AbortController();
  const slow = engine.evaluate({ expression: 'for i in 1..100000000 return i' }, { signal: first.signal });
  const queuedSlow = new AbortController();
  const slow2 = engine.evaluate({ expression: 'for i in 1..100000000 return i' }, { signal: queuedSlow.signal });
  const parse = engine.parse('expression', 'count(for i in 1..10 return i)');
  const latest = engine.evaluate({ expression: 'count(for i in 1..10 return i)' });
  setTimeout(() => {
    first.abort();
    queuedSlow.abort();
  }, 200);
  assert.equal((await slow).error.kind, 'cancelled');
  assert.equal((await slow2).error.kind, 'cancelled');
  assert.equal((await parse).ok, true);
  assert.equal((await latest).result.display, '10');
  const elapsed = Date.now() - t0;
  assert.ok(elapsed < 2500, `latest result after ${elapsed} ms`);
  // an already aborted signal never reaches the worker
  const r = await engine.evaluate({ expression: '1' }, { signal: AbortSignal.abort() });
  assert.equal(r.error.kind, 'cancelled');
});

test('real worker: a quick request cancelled within the grace period keeps the worker', async (t) => {
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: createNodeWorker, timeoutMs: 5000, cancelGraceMs: 1000 });
  t.after(() => engine.dispose());
  const c = new AbortController();
  const p = engine.evaluate({ expression: '6 * 7' }, { signal: c.signal });
  c.abort();
  const r = await p;
  assert.ok(r.ok ? r.result.display === '42' : r.error.kind === 'cancelled');
  assert.equal(engine.workerStarts, 1);
});

test('real worker: wasm unreachable trap → crash', async (t) => {
  const trapModule = await WebAssembly.compile(trappingModuleBytes());
  const engine = await FeelEngine.create({ wasmModule: trapModule, createWorker: createNodeWorker });
  t.after(() => engine.dispose());
  const r = await engine.info();
  assert.equal(r.ok, false);
  assert.equal(r.error.kind, 'crash');
  assert.match(r.error.message, /unreachable/);
});

// ───────── queue / timeout machinery with an injected fake worker ─────────

function fakeWorkerFactory(log) {
  return () => {
    const w = {
      onmessage: null,
      onerror: null,
      terminated: false,
      postMessage(msg) {
        log.push(msg.type === 'call' ? JSON.parse(msg.json).expression ?? JSON.parse(msg.json).op : msg.type);
        if (msg.type === 'init') return queueMicrotask(() => w.onmessage?.({ data: { type: 'ready' } }));
        const req = JSON.parse(msg.json);
        if (req.expression === 'hang') return; // never answers
        if (req.expression === 'trap') {
          return setTimeout(() => w.onmessage?.({ data: { type: 'crash', id: msg.id, message: 'RuntimeError: unreachable' } }), 5);
        }
        if (req.expression === 'die') return setTimeout(() => w.onerror?.({ message: 'boom' }), 5);
        setTimeout(
          () => w.onmessage?.({ data: { type: 'result', id: msg.id, json: JSON.stringify({ ok: true, echo: req }) } }),
          5,
        );
      },
      terminate() {
        w.terminated = true;
      },
    };
    return w;
  };
}

test('fake worker: FIFO, timeout restarts, crash restarts, worker error, dispose', async () => {
  const log = [];
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: fakeWorkerFactory(log), timeoutMs: 50 });
  const results = await Promise.all([
    engine.evaluate({ expression: 'a' }),
    engine.evaluate({ expression: 'hang' }),
    engine.evaluate({ expression: 'b' }),
    engine.evaluate({ expression: 'trap' }),
    engine.evaluate({ expression: 'c' }),
    engine.evaluate({ expression: 'die' }),
    engine.evaluate({ expression: 'd', tzOffsetMinutes: 60 }),
  ]);
  assert.deepEqual(
    results.map((r) => (r.ok ? r.echo.expression : r.error.kind)),
    ['a', 'timeout', 'b', 'crash', 'c', 'crash', 'd'],
  );
  assert.equal(results[0].echo.tzOffsetMinutes, -new Date().getTimezoneOffset() || 0);
  assert.equal(results[6].echo.tzOffsetMinutes, 60);
  assert.deepEqual(log, ['init', 'a', 'hang', 'init', 'b', 'trap', 'init', 'c', 'die', 'init', 'd']);
  assert.equal(engine.workerStarts, 4);

  const pending = engine.evaluate({ expression: 'hang' });
  const queued = engine.evaluate({ expression: 'e' });
  engine.dispose();
  assert.equal((await pending).error.kind, 'internal');
  assert.equal((await queued).error.kind, 'internal');
  assert.equal((await engine.evaluate({ expression: 'f' })).error.kind, 'internal');
});

test('fake worker: invalid rawJson is rejected before reaching the worker', async () => {
  const log = [];
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: fakeWorkerFactory(log), timeoutMs: 50 });
  const r = await engine.evaluate({ expression: 'x', context: rawJson('{"x": 1,}') });
  assert.equal(r.error.kind, 'context');
  const ok = await engine.evaluate({ expression: 'x', context: rawJson('{"x": 0.10000000000000000000001}') });
  assert.equal(ok.ok, true);
  assert.deepEqual(log, ['init', 'x']);
  engine.dispose();
});

test('fake worker: a worker that keeps failing to start is retried with backoff, then the engine gives up', async () => {
  let created = 0;
  const factory = () => {
    created++;
    const first = created === 1;
    const w = {
      onmessage: null,
      onerror: null,
      postMessage(msg) {
        if (msg.type === 'init') {
          if (first) return queueMicrotask(() => w.onmessage?.({ data: { type: 'ready' } }));
          return setTimeout(() => w.onerror?.({ message: 'failed to fetch worker script' }), 1);
        }
        setTimeout(() => w.onmessage?.({ data: { type: 'crash', id: msg.id, message: 'RuntimeError: unreachable' } }), 1);
      },
      terminate() {},
    };
    return w;
  };
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: factory, timeoutMs: 1000 });
  const t0 = Date.now();
  const r1 = await engine.evaluate({ expression: 'trap' });
  assert.equal(r1.error.kind, 'crash');
  const r2 = await engine.evaluate({ expression: 'queued while restarting' });
  const elapsed = Date.now() - t0;
  assert.equal(r2.error.kind, 'internal');
  assert.match(r2.error.message, /FeelEngine unavailable: the worker failed to start 4 times/);
  assert.equal(created, 5, 'initial worker + first restart + 3 retries, no tight loop');
  assert.ok(elapsed >= 2500 && elapsed < 5000, `gave up after ${elapsed} ms (backoff 100/500/2000)`);
  assert.equal(engine.isDisposed, true);
  assert.match(engine.unavailableReason, /failed to start/);
  assert.match((await engine.evaluate({ expression: 'later' })).error.message, /unavailable/);
});

test('fake worker: a transient init failure after a restart is retried and the engine recovers', async () => {
  let created = 0;
  const log = [];
  const factory = () => {
    created++;
    const n = created;
    const w = {
      onmessage: null,
      onerror: null,
      postMessage(msg) {
        if (msg.type === 'init') {
          if (n === 2) {
            return queueMicrotask(() => w.onmessage?.({ data: { type: 'initError', message: 'RangeError: Out of memory: Cannot allocate Wasm memory for new instance' } }));
          }
          return queueMicrotask(() => w.onmessage?.({ data: { type: 'ready' } }));
        }
        const req = JSON.parse(msg.json);
        log.push(req.expression);
        if (req.expression === 'hang') return;
        setTimeout(() => w.onmessage?.({ data: { type: 'result', id: msg.id, json: JSON.stringify({ ok: true, echo: req }) } }), 1);
      },
      terminate() {},
    };
    return w;
  };
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: factory, timeoutMs: 50 });
  assert.equal((await engine.evaluate({ expression: 'hang' })).error.kind, 'timeout');
  const r = await engine.evaluate({ expression: 'after' });
  assert.equal(r.ok, true);
  assert.equal(r.echo.expression, 'after');
  assert.equal(created, 3);
  assert.equal(engine.isDisposed, false);
  engine.dispose();
});

test('fake worker: a worker that never becomes ready times out and is retried', async () => {
  let created = 0;
  const factory = () => {
    created++;
    const n = created;
    const w = {
      onmessage: null,
      onerror: null,
      postMessage(msg) {
        if (msg.type === 'init') {
          if (n === 2) return; // hangs during startup
          return queueMicrotask(() => w.onmessage?.({ data: { type: 'ready' } }));
        }
        const req = JSON.parse(msg.json);
        if (req.expression === 'trap') {
          return setTimeout(() => w.onmessage?.({ data: { type: 'crash', id: msg.id, message: 'boom' } }), 1);
        }
        setTimeout(() => w.onmessage?.({ data: { type: 'result', id: msg.id, json: JSON.stringify({ ok: true, echo: req }) } }), 1);
      },
      terminate() {},
    };
    return w;
  };
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: factory, timeoutMs: 1000, startupTimeoutMs: 100 });
  assert.equal((await engine.evaluate({ expression: 'trap' })).error.kind, 'crash');
  const r = await engine.evaluate({ expression: 'next' });
  assert.equal(r.echo.expression, 'next');
  assert.equal(created, 3);
  engine.dispose();
});

test('fake worker: queued requests can be cancelled; memoryBytes above the threshold recycles the worker', async () => {
  let created = 0;
  const factory = () => {
    created++;
    const w = {
      onmessage: null,
      onerror: null,
      postMessage(msg) {
        if (msg.type === 'init') return queueMicrotask(() => w.onmessage?.({ data: { type: 'ready' } }));
        const req = JSON.parse(msg.json);
        const memoryBytes = req.expression === 'big' ? 300 * 1024 * 1024 : 16 * 1024 * 1024;
        setTimeout(() => w.onmessage?.({ data: { type: 'result', id: msg.id, json: JSON.stringify({ ok: true, echo: req }), memoryBytes } }), 5);
      },
      terminate() {},
    };
    return w;
  };
  const engine = await FeelEngine.create({ wasmModule: module, createWorker: factory });
  const c = new AbortController();
  const a = engine.evaluate({ expression: 'a' });
  const b = engine.evaluate({ expression: 'b' }, { signal: c.signal });
  c.abort();
  assert.equal((await a).echo.expression, 'a');
  assert.equal((await b).error.kind, 'cancelled');
  assert.equal(created, 1);
  await engine.evaluate({ expression: 'big' });
  assert.equal(created, 2);
  assert.equal((await engine.evaluate({ expression: 'c' })).echo.expression, 'c');
  engine.dispose();
});

test('create() rejects when the worker cannot instantiate', async () => {
  const badWorker = () => {
    const w = {
      onmessage: null,
      onerror: null,
      postMessage: () => queueMicrotask(() => w.onmessage?.({ data: { type: 'initError', message: 'LinkError: x' } })),
      terminate() {},
    };
    return w;
  };
  await assert.rejects(FeelEngine.create({ wasmModule: module, createWorker: badWorker }), /LinkError/);
});

test('create() rejects with the HTTP status when the wasm URL is missing (404)', async (t) => {
  const { createServer } = await import('node:http');
  const server = createServer((req, res) => {
    res.statusCode = 404;
    res.end('not found');
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  t.after(() => server.close());
  const url = `http://127.0.0.1:${server.address().port}/feel.wasm`;
  await assert.rejects(FeelEngine.create({ wasmUrl: url, createWorker: createNodeWorker }), /HTTP 404/);
});
