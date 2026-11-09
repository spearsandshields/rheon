# rheon FEEL wasm contract

This is the single source of truth shared by the Rust core (`wasm/feel-wasm`), the JS runtime (`packages/feel-runtime`) and the playground (`apps/playground`). Change it here first.

## 1. Wasm module (`dist/feel.wasm`)

- Target: `wasm32-wasip1`, built as a **reactor** (`cdylib` linked with Rust's self-contained `crt1-reactor.o`, see `wasm/feel-wasm/build.rs`). `_initialize` **is** exported: the host calls it exactly once after instantiation (a second call traps), e.g. `wasi.initialize(instance)` or `instance.exports._initialize()`.
- Shadow stack: 8 MiB (stack-first layout). dsntk's parser/evaluator are recursive, and in a browser the wasm frames also use the JS engine's native stack (a worker's is smaller than Node's main thread's). The core therefore rejects ASTs deeper than 300 levels and stops evaluations with more than 200 nested function calls (§2, §3); within those limits a stack overflow is still possible for pathological combinations (a recursion whose body is itself nested hundreds of levels deep) and is a trap ("crash": discard the instance).
- Linear memory starts at ~10 MiB (stack + data), grows on demand and is capped at **512 MiB** (`--max-memory`): a runaway allocation fails early (a trap) instead of growing the host to 4 GiB. Memory never shrinks; hosts should recycle instances that grew large (the JS runtime does above 256 MiB).
- Both limits are checked by `scripts/build-wasm.sh`, which also requires the `_initialize` export; `build.rs` fails the build when `crt1-reactor.o` is missing instead of silently producing a module without `_initialize`. `build.rs` also reads the engine version (the dsntk fork commit pinned in the root `Cargo.toml` and the upstream base commit) and fails when the pins are inconsistent.
- The engine is pure Rust (dsntk fork with the `dashu` decimal128 backend): no C code is linked, so no C toolchain or sysroot is involved in the build.
- Release builds have integer overflow checks on: an integer overflow anywhere in dsntk is a panic (a trap), never a silently wrapped wrong result.
- Imports: only `wasi_snapshot_preview1.*`, and only from this list of 10, which every host must be able to provide with a small shim: `random_get`, `clock_time_get`, `environ_get`, `environ_sizes_get`, `fd_write`, `fd_close`, `fd_seek`, `fd_prestat_get`, `fd_prestat_dir_name`, `proc_exit`. Nothing else. A build may import a subset: the current pure-Rust module imports six (`random_get`, `clock_time_get`, `environ_get`, `environ_sizes_get`, `fd_write`, `proc_exit`); the `fd_close`/`fd_seek`/`fd_prestat_*` entries were needed by the former C decimal library and stay in the shim so every host keeps working whatever the build pulls in.
- Exports:

| Export | Signature | Meaning |
|---|---|---|
| `memory` | memory | linear memory |
| `feel_alloc` | `(len: i32) -> i32` | Allocate `len` bytes for a request and return the pointer |
| `feel_free` | `(ptr: i32, len: i32)` | Free a buffer returned by `feel_alloc` or by `feel_call` |
| `feel_call` | `(ptr: i32, len: i32) -> i64` | Take ownership of the UTF-8 JSON request at `ptr`. Return `(resp_ptr << 32) \| resp_len`; the caller must `feel_free(resp_ptr, resp_len)`. The i64 is signed in JS (BigInt): unpack it as unsigned (`BigInt.asUintN(64, …)`) |

The three ABI functions exist only in the wasm32 build (native builds use the Rust functions `feel_wasm::call` / `call_bytes`).

A Rust panic aborts the instance (`unreachable` trap). The host must treat the instance as dead and create a new one. Never re-enter a trapped instance.

## 2. Request JSON

```jsonc
{
  "op": "evaluate" | "parse" | "info",
  "dialect": "expression" | "unaryTests",   // evaluate/parse; default "expression"
  "expression": "string",                    // evaluate/parse
  "context": { /* JSON object */ },          // evaluate; optional, default {}
  "contextFormat": "json" | "feel",          // optional, default "json"; with "feel", `context` is a STRING holding a FEEL context literal
  "input": <JSON value>,                      // unaryTests only: the value tested (`?`). If absent, context["?"] is used
  "tzOffsetMinutes": -240                     // optional: host local UTC offset in minutes, used by now()/today()
}
```

- `parse` also accepts `context`/`contextFormat` (optional): the parser needs to know which multi-word names exist (`Grace's age + 1` only parses as a name if `Grace's age` is in the context). For `dialect: "unaryTests"`, `?` is always a known name.
- Unary tests are evaluated as dsntk's decision tables do: the tested value is bound to the name `?` in the scope and the result is `? in (<unary tests>)`, i.e. a boolean (or null). Note (dsntk at the pinned commit): a boolean *expression* used as a test, e.g. `? > 5`, is **compared** with the input (`7 = true` → `false`) instead of being "satisfied when true" as in DMN 1.2+. Use `> 5` instead. The response carries a warning for every test that is a comparison/logical expression (or `not(...)`) using `?`.
- If neither `input` nor `context["?"]` is given for unary tests, `?` is null and a warning is returned.
- A unary-test endpoint that is an arithmetic expression must be parenthesised: dsntk parses `> 0.1 + 0.2` as `(? > 0.1) + 0.2`, which is a typed null (`invalid argument type … actual type is boolean`); `> (0.1 + 0.2)` and `>= (x + 1)` work. Literal and name endpoints (`> 5`, `> x`, `[x..y]`) need no parentheses.
- `tzOffsetMinutes`: integer in `-1080..=1080` (±18:00), otherwise a `request` error (every other integer, including `i64::MIN`). It is a fixed offset (no DST rules), applied to every "local" (zone-less) date-time computation of that request — `now()`, `today()`, and comparisons/subtractions of local date-times — via a process-global override in rheon-feel-temporal (the fork's "local offset override" commit), reset after every request. Without it, local time is **UTC on every target**: chrono's `Local` is hard-wired to UTC on `wasm32-wasip1`, and native builds apply offset 0 too instead of the host zone, so results never depend on where the core runs.
- Limits: an expression (or FEEL context literal) whose AST is deeper than **300** levels is rejected (`syntax`, resp. `context`) before any recursive code runs; an evaluation with more than **200** nested function calls (recursion) is stopped and reported as an `evaluation` error, never as a partial result.

How JSON input becomes FEEL values (`contextFormat: "json"`):
- JSON number → FEEL number, parsed from the **exact source text** (serde_json `arbitrary_precision`). It never passes through f64.
- string → string, boolean → boolean, null → null, array → list, object → context. Keys become FEEL names, which may contain spaces or `'`, e.g. `"Grace's age"`. Leading/trailing whitespace of a key is trimmed (`" x "` is `x`); inner whitespace is kept as is (`"a  b"` can not be referenced as `a b`).
- JSON nested deeper than about 125 levels (in `context` or `input`) is a `context` error (the request parser's recursion limit).
- No implicit date parsing: users write `date("2026-10-16")` in FEEL.
- Numbers with more than 34 significant digits, or so small that they underflow decimal128's exponent range (below 1e-6176: to a subnormal value or to 0), are rounded (half-even) by the number core; a warning is returned. Numbers too large for decimal128 (above ~1e6145) are a `context` error.
- With `contextFormat: "feel"`, the string is evaluated as a FEEL context literal (e.g. `{a: 1, b: a + 1, "Grace's age": 40}`); a blank string is an empty context. Any non-context result is a `context` error.

## 3. Response JSON

Success:

```jsonc
{
  "ok": true,
  "result": {
    "display": "string",   // pretty, JSON-like text with EXACT FEEL digits, e.g. {"a": 0.3333333333333333333333333333333333}; temporal values as their FEEL string, null as null
    "feel": "string",      // dsntk's own FEEL text form (Value::to_string)
    "type": "string",      // FEEL type name, e.g. "number", "string", "date", "list<number>", "context<...>", "Null"
    "typed": { /* rheon_feel::dto::ValueDto: type+text per value, TCK style */ },
    "isNull": false,
    "nullReason": "string?" // when dsntk produced null carrying an error message (value_null!(...)), that message
  },
  "warnings": [ { "message": "string" } ],
  "elapsedMicros": 123
}
```

`display` rules: numbers in dsntk's exact decimal text (a zero with an exponent outside `-34..=0`, e.g. `0 * 1e-6000`, is shown as `0`); strings JSON-quoted/escaped; `true`/`false`/`null`; temporal values, ranges and other non-JSON values as their FEEL text JSON-quoted (`"2026-10-16"`, `"P1DT2H"`, `"[1..10]"`); functions as their quoted type (`"function<Any>->Any"`); lists/contexts as `[...]`/`{"key": value}` on one line when the line fits 80 columns, otherwise expanded with two-space indentation. `warnings` currently reports rounded/underflowed input numbers, missing unary-test input and boolean unary tests using `?`.

`typed` is dsntk's `ValueDto`; for values it has no DTO form (ranges, functions, built-in functions) all three fields are `null` (`{"simple": null, "components": null, "list": null}`).

Non-finite decimals: FEEL has no infinity/NaN. Any arithmetic or function result that overflows decimal128 (e.g. `9e6144 * 10`, `9e6144 + 9e6144`, `1 / 1e-6176`) is `null` with nullReason `result is not a finite number (decimal overflow)`.

Numbers are IEEE 754 decimal128 (34 significant digits, half-even rounding, exponent range -6176..=6111), computed by the repository's pure-Rust decimal core. The exact operations (`+ - * /`, `sqrt`, rounding, comparisons, formatting) are bit-identical to Intel's BID128 library (the backend native dsntk uses). `exp`, `ln`, `log` and `**` with a non-integer exponent are **correctly rounded** here, which native dsntk is not always (1 ulp in roughly 11–16 % of inputs), so those results can differ from a native dsntk in the last digit. `**` with an integer exponent is Intel-identical but **not** correctly rounded: it is a binary exponentiation with 34-digit rounding after each multiplication, so the error grows with the exponent (`1.1 ** 100000` is 294 ulp off, `1.0001 ** 1000000` about 89,000 ulp).

Scale rounding (`round up/down/half up/half down`, `floor`, `ceiling` with a scale) returns the scaled value also when its coefficient is 1 (`round up(0.1, 1)` = `0.1`, `round up(10, -1)` = `10`); upstream dsntk at the base commit returns `1` there (fixed in this repository). Date-times and times whose zones carry an offset compare as instants for `=`, `<`, `<=`, `>`, `>=` and ranges; a local (zone-less) value is ordered only against another local value (false otherwise, like dsntk's subtraction). `time + days and time duration` wraps around midnight.

`op: "parse"` success: `{ "ok": true, "tree": "<dsntk AstNode::tree() ascii, no color>", "elapsedMicros": n }`

`op: "info"` success: `{ "ok": true, "engine": "rheon", "engineCommit": "<40-hex repository commit>[-dirty]", "upstreamCommit": "<40-hex upstream dsntk-rs commit>", "numberBackend": "dashu", "version": "<rheon-feel-wasm crate version>" }`. `engineCommit` is the rheon repository commit the module was built from (`git rev-parse HEAD` at build time, `-dirty` appended when the work tree had uncommitted changes, `"unknown"` when built outside a git checkout without `RHEON_COMMIT` set), `upstreamCommit` the dsntk-rs commit the engine crates are based on (`[workspace.metadata.rheon]` in the root `Cargo.toml`), and `numberBackend` the feel-number backend compiled in (`rheon_feel_number::NUMBER_BACKEND`: `"dashu"`; `"intel"` only in an Intel build, which this repository does not make).

Failure (the instance stays healthy):

```jsonc
{ "ok": false, "error": { "kind": "request" | "syntax" | "context" | "evaluation" | "internal", "message": "string" } }
```

- `request`: malformed request JSON (including invalid UTF-8 and JSON strings with unpaired `\uD800`-style surrogate escapes, which the core can not represent) or unknown op.
- `syntax`: the FEEL expression or unary tests failed to parse, or are nested too deeply (§2). The message is dsntk's, without its `<ParserError>` prefix; dsntk gives no position.
- `context`: the input context is invalid (bad JSON shape, JSON nested too deeply, bad FEEL context literal, or a FEEL context nested too deeply / recursing too deeply).
- `evaluation`: evaluation was stopped: currently only when more than 200 function calls are nested (§2). FEEL usually yields null instead; that is a *success* with `isNull: true` and `nullReason`. Null reasons are meant to be readable (`time: invalid time literal '…'`, `log: argument must be a positive number`, `exp: result is not a finite number (decimal overflow)`); a number quoted in a reason is printed in scientific notation when its plain text would exceed 40 characters (`sublist: invalid position value: 9E+6144`).
- `internal`: a caught Rust panic (only possible on native builds; in wasm a panic traps, see §1) or a serialization failure.

## 4. JS runtime API (`packages/feel-runtime`, TypeScript, ESM)

```ts
export type Dialect = 'expression' | 'unaryTests';
/** Raw JSON text, spliced verbatim into the request JSON (keeps exact number digits). Only values made by rawJson() count (private Symbol.for brand). */
export interface RawJson { readonly rawJson: string; }
export interface RequestOptions { signal?: AbortSignal; }
export interface EvaluateRequest { dialect?: Dialect; expression: string; context?: Record<string, unknown> | string | RawJson; contextFormat?: 'json' | 'feel'; input?: unknown | RawJson; tzOffsetMinutes?: number; }
export interface FeelResult { display: string; feel: string; type: string; typed: unknown; isNull: boolean; nullReason?: string | null; }
export type EvaluateResponse =
  | { ok: true; result: FeelResult; warnings: { message: string }[]; elapsedMicros: number }
  | { ok: false; error: { kind: 'request'|'syntax'|'context'|'evaluation'|'internal'|'timeout'|'crash'|'cancelled'; message: string } };
export type ParseResponse = { ok: true; tree: string; elapsedMicros: number } | { ok: false; error: { kind: string; message: string } };
export interface ParseOptions { context?: Record<string, unknown> | string | RawJson; contextFormat?: 'json' | 'feel'; }

export interface FeelEngineOptions {
  wasmUrl?: string | URL;            // one of wasmUrl / wasmModule / wasmBytes is required
  wasmModule?: WebAssembly.Module;
  wasmBytes?: BufferSource;
  timeoutMs?: number;                // default 2000
  cancelGraceMs?: number;            // default 100: a cancelled running request may finish this long before its worker is replaced
  recycleMemoryBytes?: number;       // default 256 MiB: replace a worker whose wasm memory grew past this (after the request)
  startupTimeoutMs?: number;         // default 15000: a worker not ready by then counts as a failed start
  createWorker?: () => WorkerLike;   // default: new Worker(new URL('./worker.js', import.meta.url), {type:'module'})
}
export class FeelEngine {
  static create(opts: FeelEngineOptions): Promise<FeelEngine>; // compiles wasm once, spawns the worker, resolves when it is ready
  evaluate(req: EvaluateRequest, opts?: RequestOptions): Promise<EvaluateResponse>;
  parse(dialect: Dialect, expression: string, opts?: ParseOptions & RequestOptions): Promise<ParseResponse>;
  info(): Promise<unknown>;
  callJson(json: string, opts?: RequestOptions): Promise<any>;  // raw §2 request, sent as-is
  dispose(): void;                       // pending/later requests resolve {ok:false, error:{kind:'internal', message:'FeelEngine disposed'}}
  readonly isDisposed: boolean;          // after dispose(), or when the engine gave up restarting its worker
  readonly unavailableReason: string | null;
  readonly timeoutMs: number;
  readonly workerStarts: number;         // 1 + number of restarts (diagnostics)
}
// Also exported for non-worker hosts (Node, tests): a synchronous in-thread runner
export function instantiateFeel(source: BufferSource | WebAssembly.Module,
  env?: { tzOffsetMinutes?: number | null; onOutput?: (fd: 1 | 2, line: string) => void; recycleMemoryBytes?: number })
  : Promise<{ call(req: object | string): any; callJson(json: string): string; reset(): Promise<void>; readonly module: WebAssembly.Module }>;
export function createWasiShim(opts?): { imports; setMemory(m); recentStderr(): string[]; flush() }; // the 10 §1 imports
export function rawJson(text: string): RawJson;
```

Package exports: `.` (all of the above), `./worker` (the module worker script), `./wasi` (the shim).

Runtime guarantees:
- Requests are serialized: one at a time per engine, queued in FIFO order.
- **Cancellation:** a request whose `signal` aborts resolves `{ok:false, error:{kind:'cancelled'}}`: at once when it is still queued (it never reaches the worker); when it is running, after it finishes or, at most `cancelGraceMs` after it started, by terminating and replacing the worker. An already aborted signal resolves immediately.
- **Memory:** the worker reports its wasm memory size with every result; above `recycleMemoryBytes` the worker is replaced after that request (wasm memory never shrinks).
- **Restart failures:** a worker that fails before it is ready (script load error, `initError`, startup timeout) is retried after 100, 500 and 2000 ms. After 4 consecutive failures the engine gives up: all pending and later requests resolve `{kind:'internal', message:'FeelEngine unavailable: …'}`, `isDisposed` is `true` and `unavailableReason` says why (reload the page). A worker that becomes ready resets the count. During `create()` a failure rejects instead.
- **Timeout:** if a request runs past `timeoutMs`, the worker is `terminate()`d, the request resolves `{ok:false, error:{kind:'timeout'}}`, and a fresh worker is started for the next request. The compiled `WebAssembly.Module` is reused, so this is cheap.
- **Trap** (a panic or `unreachable`): resolves `{ok:false, error:{kind:'crash', message}}`, and the instance is re-created. When the trap is a failed allocation at the 512 MiB cap (Rust writes `memory allocation of N bytes failed` to stderr first), the message starts with `out of memory:` (`trapMessage()` in `instance.ts`), so hosts can present "the result does not fit" differently from a bug.
- **Exact numbers from JS:** a JS object/array passed as `context` or `input` is serialized with `JSON.stringify`, so its numbers are already f64. To keep the user's digits (e.g. text typed into the playground's JSON editor), pass `rawJson("<json text>")`: the runtime validates it parses as JSON (and, for `context` with `contextFormat: "json"`, that it is an object) and splices the text verbatim into the request JSON, never parsing its numbers. Only values created by `rawJson()` are raw (they carry a private `Symbol.for('@rheon/feel-runtime.rawJson')` brand); a plain object `{ rawJson: "..." }` is an ordinary context. The wasm request shape (§2) is unchanged.
- **Unpaired surrogates:** JS strings (and `JSON.parse`) allow unpaired UTF-16 surrogates, the core's JSON parser does not. The runtime rejects them before the wasm: `{kind:'syntax'}` in the expression, `{kind:'context'}` in `context`/`input` (raw characters or `\uD800`-style escapes).
- `tzOffsetMinutes` is filled in automatically from `new Date().getTimezoneOffset()` (sign inverted) unless the caller supplies it (`EvaluateRequest.tzOffsetMinutes`; for `instantiateFeel`, `env.tzOffsetMinutes`, `null` = don't send).
- `context` given as a string defaults to `contextFormat: 'feel'`; a JS object or `RawJson` defaults to `'json'`. Invalid `rawJson` (or a non-object `rawJson` context) resolves `{kind:'context'}` without reaching the wasm.
- `input` is only sent for `dialect: 'unaryTests'`.
- WASI shim: no environment variables, no preopens, `fd_write` to fd 1/2 forwarded line-by-line to `console.log`/`console.error` (or `onOutput`); the last stderr lines (e.g. a Rust panic message) are appended to `crash` messages. `proc_exit` throws (→ crash).
- A `crash` in `FeelEngine` terminates the worker and starts a fresh one (same compiled module). `instantiateFeel.call` re-instantiates synchronously.
- `instantiateFeel` is for **trusted input only**: it runs on the calling thread and nothing can interrupt it, so a runaway expression blocks that thread (e.g. a server's event loop). Untrusted input belongs in `FeelEngine` (or a Wasmtime host with fuel/epoch interruption). It requires the `_initialize` export (rejects otherwise) and re-creates its instance after a call that left memory above `env.recycleMemoryBytes` (default 256 MiB).
- Errors never reject the returned promises (except `FeelEngine.create`, which rejects if the wasm cannot be fetched/compiled/instantiated).

## 5. Repository layout

```
Cargo.toml               one workspace: the dsntk engine crates (repository root) + wasm/feel-wasm; `[workspace.metadata.rheon] upstream-commit`; the `wasm` build profile
<dsntk crates>/          common, feel, feel-parser, feel-evaluator, feel-number, feel-temporal, feel-regex, model, model-evaluator, dsntk (CLI), … unchanged layout of upstream dsntk-rs
scripts/bootstrap.sh     install the wasm32-wasip1 target, download wasmtime into .tools/ (gitignored)
scripts/env.sh           source to set wasmtime as the cargo runner for `cargo test --target wasm32-wasip1`
scripts/build-wasm.sh    build wasm/feel-wasm → dist/feel.wasm (+ size and interface report)
wasm/feel-wasm         Rust core implementing §1–3
packages/feel-runtime    JS/TS runtime implementing §4
apps/playground          Vite + Svelte 5 playground using feel-runtime
package.json             npm workspaces (packages/*, apps/*); scripts: bootstrap, build:wasm, build, dev, preview, test
```

- The playground imports `dist/feel.wasm` with `?url` (`apps/playground/src/engine.ts`), so `vite build` emits it as a hashed asset next to the bundle; the build fails early if `npm run build:wasm` has not run. The site is built with `base: './'` and works from any static host or sub-path.
- The fork's "wasm32 support" commit makes `FeelNumber → usize/isize` conversions that do not fit in 32 bits return a conversion error instead of panicking. Its "robustness" commit makes the position-taking built-ins (`sublist`, `substring`, `insert before`, `remove`, `list replace`) and list filters convert through 64-bit integers with wide arithmetic, so wasm32 returns the same results and null reasons as 64-bit native builds and no length can wrap. Covered by `positions_beyond_32_bits_are_null_not_panic` and `huge_positions_and_lengths_are_null_with_native_reasons`.
- `rust-toolchain.toml` pins rustc 1.95.0; `scripts/bootstrap.sh` verifies the sha256 sum of the wasmtime download. The engine crates are part of this repository (see README "Relationship to dsntk" and `docs/FORK.md`).
