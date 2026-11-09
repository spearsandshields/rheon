# @rheon/feel-runtime

[![npm](https://img.shields.io/npm/v/@rheon/feel-runtime.svg)](https://www.npmjs.com/package/@rheon/feel-runtime)

JS/TS runtime for the [rheon](https://github.com/spearsandshields/rheon) FEEL engine (a fork of
[dsntk](https://github.com/dsntk/dsntk-rs)) compiled to WebAssembly (`wasm32-wasip1`, pure-Rust
decimal128 numbers: exact 34-digit results). It implements
[`docs/CONTRACT.md`](https://github.com/spearsandshields/rheon/blob/main/docs/CONTRACT.md) §4.
The package ships the module itself, `feel.wasm` (about 2.9 MB, 0.8 MB gzipped), so nothing has to
be compiled. Try it in the [playground](https://spearsandshields.github.io/rheon/).

```sh
npm install @rheon/feel-runtime
```

- `FeelEngine`: runs `feel.wasm` in a module Web Worker. It provides a FIFO queue, a per-request timeout (the worker is terminated and restarted) and crash recovery.
- `instantiateFeel`: a synchronous in-thread runner for Node, tests and workers you manage yourself.
- `createWasiShim`: the 10-function WASI preview1 shim the module needs. It does not use `node:wasi`, so it works in browsers and Node.

## Usage in a browser or Vite app

```ts
import { FeelEngine, rawJson } from '@rheon/feel-runtime';
import wasmUrl from '@rheon/feel-runtime/feel.wasm?url'; // Vite: emitted as a hashed asset. Or copy feel.wasm into public/.

const engine = await FeelEngine.create({ wasmUrl, timeoutMs: 2000 });

await engine.evaluate({ expression: '1/3' });
// → { ok: true, result: { display: "0.3333333333333333333333333333333333", type: "number", ... }, warnings: [], elapsedMicros }

// Exact digits from user-typed JSON: pass the text, not a JS object (JS numbers are f64).
await engine.evaluate({ expression: 'x + 0.2', context: rawJson('{"x": 0.1000000000000000000000000000000001}') });

// FEEL context literal (a string context defaults to contextFormat 'feel')
await engine.evaluate({ expression: 'b * 2', context: '{a: 1, b: a + 1}' });

// Unary tests: the input is bound to `?`; the result is `? in (<tests>)`
await engine.evaluate({ dialect: 'unaryTests', expression: '[1..10], > 20', input: 25 });

// Parse tree (dsntk AstNode::tree). Pass the context so multi-word names split correctly.
await engine.parse('expression', 'first name + 1', { context: { 'first name': 1 } });

await engine.info(); // { ok, engine: "rheon", engineCommit, upstreamCommit, numberBackend: "dashu", version }
engine.dispose();
```

Results never throw. Every failure is a response of the form `{ ok: false, error: { kind, message } }`:

| kind | meaning |
|---|---|
| `request` / `syntax` / `context` / `evaluation` / `internal` | reported by the wasm core (CONTRACT §3). Invalid `rawJson` is reported as `context` before the request reaches the worker. |
| `timeout` | ran past `timeoutMs`. The worker was terminated and a fresh one is started for the next request. |
| `crash` | the instance trapped (a Rust panic, `unreachable`, a failed allocation at the 512 MiB cap, or a stack overflow). The worker is replaced. The message includes any panic text the module wrote to stderr; when that text is Rust's `memory allocation of N bytes failed`, the message starts with `out of memory:` (see `trapMessage`). |
| `cancelled` | the request's `AbortSignal` fired (`evaluate(req, { signal })`). A running request gets `cancelGraceMs` (default 100) to finish, then its worker is terminated and replaced. |
| `internal` (`FeelEngine disposed`) | the request was pending or sent after `dispose()`. |
| `internal` (`FeelEngine unavailable: …`) | the worker failed to start 4 times in a row (retried after 100, 500 and 2000 ms); `engine.isDisposed` is then `true` and `engine.unavailableReason` says why. Reload. |

Other options: `recycleMemoryBytes` (default 256 MiB; a worker whose wasm memory grew past it is replaced after the request), `startupTimeoutMs` (default 15000; a worker that does not become ready in time counts as a failed start). `rawJson(text)` values are branded: a plain object `{ rawJson: "..." }` is ordinary data. Text with an unpaired UTF-16 surrogate is rejected before reaching the core (`syntax` for the expression, `context` for context/input).

How the engine works:

- **Loading.** The wasm is fetched and compiled once, on the main thread. `compileStreaming` is used when the server sends `application/wasm`. The compiled `WebAssembly.Module` is posted to each worker, so a restart only re-instantiates it. You can also pass `wasmModule` or `wasmBytes` instead of `wasmUrl`.
- **Time zone.** `tzOffsetMinutes` defaults to `-new Date().getTimezoneOffset()` on every request, so `now()` and `today()` follow the host's local time. Set it on the request to override. It is a fixed offset with no DST rules.
- **Worker bundling.** The worker is created with `new Worker(new URL('./worker.js', import.meta.url), { type: 'module' })`. Vite, webpack 5 and Parcel bundle this form automatically. Two cases need attention:
  - If Vite pre-bundles the package from `node_modules`, the `new URL` form breaks. Add `optimizeDeps: { exclude: ['@rheon/feel-runtime'] }`. A linked workspace package (`file:` or `workspace:` dependency) is not pre-bundled.
  - Use `worker: { format: 'es' }` if your Vite config sets a non-ES worker format.
- **Custom workers.** `createWorker: () => WorkerLike` replaces worker creation, for example with your own `new Worker(...)` call or a `worker_threads` adapter (see `test/helpers.mjs` and `test/worker-bootstrap.mjs`). The worker script is also exported as `@rheon/feel-runtime/worker`.

## Usage in Node (in-thread)

```js
import { readFile } from 'node:fs/promises';
import { feelWasmUrl, instantiateFeel, rawJson } from '@rheon/feel-runtime';

const feel = await instantiateFeel(await readFile(feelWasmUrl()), { tzOffsetMinutes: -240 });
feel.call({ op: 'evaluate', expression: '0.1 + 0.2' }).result.display; // "0.3"
feel.call({ op: 'evaluate', expression: 'x', context: rawJson('{"x": 1.000000000000000000001}') });
feel.call('{"op":"info"}'); // a string is sent verbatim
```

`call(req)` is synchronous. It takes a CONTRACT §2 request object or a JSON string and returns the parsed response.

- Any top-level `{ rawJson }` value is spliced verbatim into the request.
- For `evaluate` requests, `env.tzOffsetMinutes` is applied when the request has no offset of its own. It defaults to the host offset; set it to `null` to disable.
- A trap returns `{ ok: false, error: { kind: 'crash' } }` and the instance is re-created synchronously, with the module reused.
- **Trusted input only.** There is no timeout in-thread: a runaway expression blocks the calling thread (in a server, the event loop) until it finishes. Use `FeelEngine` for untrusted or long-running input.
- `env.recycleMemoryBytes` (default 256 MiB): the instance is re-created after a call that left its memory larger (wasm memory never shrinks).

## Known limits (from the core)

- Long iterations are slow in dsntk (`sum(for i in 1..100000 return i)` takes about a minute). That is what the timeout is for.
- Expressions nested deeper than 300 levels are a `syntax` error; more than 200 nested function calls stop with an `evaluation` error (CONTRACT §2).
- A boolean expression used as a unary test (`? > 5`) is compared with the input, not treated as satisfied when true. Write `> 5` instead. See CONTRACT §2.

## Building from source

```sh
npm ci && npm run bootstrap      # at the repository root: wasm target + wasmtime
npm run build:wasm               # → dist/feel.wasm (Rust, wasm32-wasip1, `wasm` profile)
npm run build:runtime            # tsc → packages/feel-runtime/dist/
npm run test:runtime             # node:test against ../../dist/feel.wasm
npm pack -w @rheon/feel-runtime  # the tarball; `prepack` copies feel.wasm and the license files in
```

## License

MIT OR Apache-2.0, at your option; see `LICENSE-MIT`, `LICENSE` and `NOTICE` in the package.
The engine is a fork of dsntk by Dariusz Depta (same licenses); its changes are listed in
[docs/FORK.md](https://github.com/spearsandshields/rheon/blob/main/docs/FORK.md).
