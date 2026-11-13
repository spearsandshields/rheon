# rheon

**rheon** (Greek ῥέον, "that which flows", the root of *panta rhei*) is a Rust decision and
process stack in the making: the FEEL expression language and DMN decisions today, BPMN later,
with every engine also compiled to WebAssembly so that the same module runs in the browser, in
Node and in .NET.

It is a fork of [dsntk](https://github.com/dsntk/dsntk-rs) (DecisionToolkit, by Dariusz Depta),
the Rust FEEL/DMN engine, kept in upstream's layout so that upstream can be merged. On top of it
rheon adds a pure-Rust decimal128 number core, a set of engine fixes (listed in
[`docs/FORK.md`](docs/FORK.md)), the `wasm32-wasip1` build of the FEEL engine, a TypeScript
runtime and a browser playground.

rheon is an independent project. It is not affiliated with, endorsed by or supported by
DecisionToolkit, Engos Software or the Object Management Group. "DMN", "FEEL" and "BPMN" are
trademarks of the OMG.

## What works today

- **FEEL engine in wasm** (`dist/feel.wasm`, ~3.6 MiB raw, ~650 KiB brotli): expressions and
  unary tests, exact 34-digit decimals (IEEE 754 decimal128 semantics in pure Rust on
  [dashu-float](https://crates.io/crates/dashu-float), so `0.1 + 0.2` is `0.3` and `1/3` is
  `0.3333333333333333333333333333333333`), typed results, "null because …" reasons, a JSON ABI
  (`feel_alloc` / `feel_call` / `feel_free`) documented in [`docs/CONTRACT.md`](docs/CONTRACT.md).
- **JS/TS runtime** `@rheon/feel-runtime`: a Web Worker host with a 10-function WASI shim,
  evaluation timeout, crash recovery and exact numbers across the boundary, plus a synchronous
  `instantiateFeel` for trusted input. See its [README](packages/feel-runtime/README.md).
- **Playground** `apps/playground` (Vite + Svelte 5 + CodeMirror 6): expression and unary-test
  dialects, a JSON context editor that keeps every digit, the engine's syntax tree, examples,
  shareable links, light and dark themes, a phone layout. Everything runs in the browser; the
  build is a static site. Live demo: <https://spearsandshields.github.io/rheon/> (GitHub Pages, deployed
  by CI from `main`).
- **DMN 1.6** (the formal OMG version, September 2026) in the native engine: the `dsntk` CLI and
  server, the DMN model evaluator, B-FEEL, lenient and strict error modes, and the DMN TCK at
  3,392 of 3,407 (the rest need Java). Features the official TCK does not test yet have
  TCK-format tests in [`tck-addendum/`](tck-addendum/MANIFEST.md); `scripts/tck.sh` runs both.
  The DMN engine is not in wasm yet (next milestone); the FEEL module already offers B-FEEL and
  the error modes (`language`, `errorMode`, CONTRACT §2).

The edge-case comparison with feelin and feel-scala is in
[`docs/ENGINE-COMPARISON.md`](docs/ENGINE-COMPARISON.md); deferred work and open questions in
[`docs/BACKLOG.md`](docs/BACKLOG.md).

## Architecture

```
 ┌──────────────────────────── browser ─────────────────────────────────────────┐
 │                                                                               │
 │  apps/playground  (Vite + Svelte 5 + CodeMirror 6 / lang-feel)                │
 │     │  evaluate / parse / info   (CONTRACT §4: FeelEngine)                     │
 │     ▼                                                                         │
 │  packages/feel-runtime  ── main thread: fetch + compile feel.wasm once,       │
 │     │                      FIFO queue, timeout, crash → restart,              │
 │     │                      fills in tzOffsetMinutes, splices {rawJson} as is  │
 │     │ postMessage(WebAssembly.Module, request JSON)                           │
 │     ▼                                                                         │
 │  Web Worker (module)  ── 10-function WASI shim (no node:wasi, no fs)          │
 │     │  feel_alloc / feel_call / feel_free   (CONTRACT §1–3, JSON in/out)      │
 │     ▼                                                                         │
 │  dist/feel.wasm  ◄── wasm/feel-wasm (Rust, cdylib reactor)                    │
 │                         └─ the dsntk crates of this repository: parser,       │
 │                            evaluator, temporal, feel-number (`dashu` backend) │
 └───────────────────────────────────────────────────────────────────────────────┘
   timeout → worker.terminate() + new worker (same compiled Module, so cheap)
```

## Repository layout

| Path | What it is |
|---|---|
| `feel`, `feel-number`, `feel-parser`, `feel-evaluator`, `feel-temporal`, `feel-regex`, `model`, `model-evaluator` | The engine crates rheon has changed, named `rheon-*` and published on crates.io (directories keep upstream's names) |
| `evaluator`, `workspace`, `gendoc`, `server`, `dsntk`, `examples` | The dsntk crates carried with manifest-only changes, still named `dsntk-*` and marked `publish = false`. The untouched `dsntk-common`, `dsntk-macros`, `dsntk-feel-grammar` and `dsntk-recognizer` are not in the tree: they come from crates.io at upstream's 0.3.0 |
| `wasm/feel-wasm` | `rheon-feel-wasm`: JSON request → engine → JSON response, built as a wasm32-wasip1 reactor (CONTRACT §1–3) |
| `packages/feel-runtime` | `@rheon/feel-runtime`, the TypeScript runtime (CONTRACT §4) |
| `apps/playground` | `@rheon/playground`, the web app |
| `docs/` | `CONTRACT.md` (ABI and JSON contract), `FORK.md` (every engine change against upstream, verification numbers, how to merge upstream), `BACKLOG.md`, `EDITOR-PLAN.md`, `ENGINE-COMPARISON.md` |
| `scripts/` | `bootstrap.sh`, `env.sh`, `build-wasm.sh`, `smoke.mjs`, `stack-headroom.mjs`, `third-party-notices.mjs` |
| `tools/` | dsntk-side harnesses: the differential number fuzzer and the FEEL corpus runner (see `tools/README.md`) |
| `Cargo.toml` | One workspace for all of the above; the `wasm` build profile |
| `dist/feel.wasm` | Build output of the core, shared by every host (browser, Node, later .NET) |

## Get it

- **npm**: `npm install @rheon/feel-runtime` ships `feel.wasm` together with the TypeScript
  runtime (Web Worker with timeouts, WASI shim, Node runner). See
  [`packages/feel-runtime/README.md`](packages/feel-runtime/README.md).
- **crates.io**: the engine crates (`rheon-feel`, `rheon-feel-parser`, `rheon-feel-evaluator`,
  `rheon-model-evaluator`, …) for Rust programs, and `rheon-feel-wasm`, the source of the module.
- **GitHub Releases**: every `vX.Y.Z` tag gets `feel.wasm`, its sha256, the npm tarball and the
  playground build attached by `.github/workflows/release.yml`.

## Quickstart

Requirements: Rust via `rustup` (`rust-toolchain.toml` pins 1.95.0 and the `wasm32-wasip1`
target), Node ≥ 20 with npm, `curl` (for wasmtime), macOS or Linux (x86_64 or arm64). The whole
build is pure Rust + Node: no C toolchain, no Java.

```sh
npm install            # npm workspaces: packages/* and apps/*
npm run bootstrap      # installs the wasm target and .tools/wasmtime
npm run build          # build:wasm → build:runtime → build:playground
npm run preview        # serve apps/playground/dist (vite preview)
```

For development:

```sh
npm run build:wasm     # only needed again after changing Rust code
npm run dev            # builds the runtime, then starts the Vite dev server
```

| Script | Does |
|---|---|
| `npm run bootstrap` | `./scripts/bootstrap.sh`: `rustup target add wasm32-wasip1`, downloads wasmtime (sha256-checked) into `.tools/` |
| `npm run build:wasm` | `./scripts/build-wasm.sh`: `cargo build --profile wasm --target wasm32-wasip1 -p rheon-feel-wasm`, then `dist/feel.wasm`, a size report, and a check of the imports, exports, stack and memory cap against CONTRACT §1 |
| `npm run build` | wasm + runtime (`tsc`) + playground (`svelte-check` + `vite build`, with `third-party-notices.txt` regenerated) |
| `npm run dev` | Playground dev server (arguments after `--` go to Vite) |
| `npm test` | `cargo test -p rheon-feel-wasm` native + the same under wasmtime (`--target wasm32-wasip1`) + runtime tests (`node --test`) |
| `npm run smoke` | Node smoke test of `dist/feel.wasm` (node:wasi) |
| `npm run stack` | `scripts/stack-headroom.mjs`: requests nested to the AST and call-depth limits must evaluate in Node (main thread and a 4 MiB worker) and in Wasmtime with a 4 MiB wasm stack; `npm run stack -- --report` prints the depths each host reaches |
| `scripts/publish-crates.sh` | Publishes the `rheon-*` crates that crates.io lacks at the workspace version, waiting out the new-crate rate limit (see `docs/FORK.md`, "Publishing") |
| `cargo test --workspace` | The engine's own suites (about 4,200 tests). A few upstream tests need things CI does not have and are skipped there: `external_functions` and `dmn_3_0076` (a Java external-function service), `compare_the_number_of_models` (a DMN TCK checkout at `../tck`), the server's `utils::tests::getting_*` (a checkout directory named `dsntk`); see `.github/workflows/ci.yml` |
| `RHEON_FULL_TESTS=1 cargo test --release -p rheon-feel-number -p rheon-feel -p rheon-feel-parser` | The randomized and exhaustive tests of those crates at full size (by default they run a smaller number of cases). `RHEON_PARSER_CORPUS=<absolute path of tools/evaluator-corpus/corpora>` with `-- --include-ignored` adds the parser's corpus test |

The production build in `apps/playground/dist/` uses a relative base (`./`), so you can host it on
any static file server and under any sub-path. `feel.wasm` is a hashed asset in `assets/`. Serve
it as `application/wasm` so the browser can compile it while it downloads.

The `info` op of the module reports `engineCommit`, the repository commit it was built from
(`-dirty` when the work tree had uncommitted changes), and `upstreamCommit`, the dsntk-rs commit
the engine crates are based on; the playground shows the short SHA in its header.

## Relationship to dsntk

rheon is a permanent fork: the engine is developed here, in its own direction, and
contributing the changes back to dsntk is not a goal. The dsntk crates sit at the repository root
in upstream's directory layout, and `upstream` is a git remote
(`git remote add upstream https://github.com/dsntk/dsntk-rs.git`), so individual upstream fixes
can still be taken by cherry-pick (`docs/FORK.md` has the procedure).
[`docs/FORK.md`](docs/FORK.md) lists every engine change with its verification numbers. In
short, rheon's engine differs from upstream dsntk in:

- a **pure-Rust decimal128 core** (`feel-number`), with the `dashu` backend as default and
  Intel's BID128 library behind the `intel` feature (needs a C toolchain; not used here);
  correctly rounded `exp`, `ln`, `log` and fractional `**`, validated against an mpmath oracle
  (0 wrong results in about 27 M checks);
- **wasm32 support** (Java external functions become a null with a reason; 32-bit conversions);
- **robustness**: every user-reachable panic found in review is a FEEL null or an error, a call
  depth limit, an iterative AST drop, linear `for` loops and list literals (upstream was quadratic);
- **engine fixes** found by review and by the DMN TCK: scale rounding, lost-step `for` ranges,
  midnight wrap of `time + duration`, cross-offset temporal ordering, a cubic lexer path, the full
  FEEL scale range, back-references in `matches`, readable null reasons.

DMN TCK (patched suite, 3,511 tests): rheon passes 3,480, upstream at the base commit 3,479;
the remaining failures are Java external functions (14), decision services (2), implicit
conversions (12) and multiple imports (3), all in the DMN layer.

Crate naming: a crate whose source rheon has changed is renamed `rheon-*` (today
`rheon-feel`, `-feel-number`, `-feel-parser`, `-feel-evaluator`, `-feel-temporal`,
`-feel-regex`, `-model` and `-model-evaluator`) and published on crates.io, so a Rust project
can depend on `rheon-feel-evaluator = "0.1"` directly. Upstream crates rheon has not changed are
used from crates.io at dsntk's 0.3.0 release (`dsntk-common`, `-macros`, `-feel-grammar`,
`-recognizer`). The dsntk crates carried with manifest-only changes (`dsntk-evaluator`,
`-workspace`, `-gendoc`, `-server`, `-examples` and the `dsntk` CLI) stay in the tree under their
`dsntk-*` names and are never published from here. Directories keep upstream's names either way.

## Known limitations

- **Java external functions are not supported.** They need a JVM, and on wasm32 they evaluate to
  null with a reason.
- **Rounding of `exp`, `ln`, `log` and fractional `**`.** These are correctly rounded to 34
  digits (the exact result, rounded once, half-even). Upstream dsntk with the Intel library is
  1 ulp off for roughly 11–16 % of inputs and occasionally more, so the last digit of such results
  can differ from upstream dsntk; e.g. `12.2384283 ** 4.25` is `41959.85737359436186095331070746801`
  here (the exact value continues …7468005517…, so …801 is the correctly rounded 34-digit
  result), while Intel gives …800. The exact operations (`+ - * /`, `sqrt`, rounding
  functions, comparisons, formatting) are bit-identical to Intel.
- **Integer-exponent `**` is Intel-identical, not correctly rounded.** `x ** n` with an
  integer `n` uses Intel's binary exponentiation with 34-digit rounding after every
  multiplication, so the error grows with |n|: `1.1 ** 100000` is 294 ulp from the exact value
  rounded to 34 digits, `1.1 ** -100000` 853 ulp, `1.0001 ** 1000000` about 89,000 ulp, and
  `2 ** -20000` and `0.5 ** 20000` (the same value) differ in their last digits. Small exponents
  on short operands are exact (`3 ** 50`, `2 ** 100`). Only a non-integer exponent takes the
  correctly rounded `exp(y ln x)` path.
- **Size.** `feel.wasm` is about 3.6 MiB raw (3.8 MB), about 995 KiB gzip and 650 KiB brotli
  (`npm run build:wasm` prints the exact figures). The `wasm` profile builds at opt-level 3; at
  opt-level z the same code is about 3.0 MiB raw and 540 KiB brotli and runs at about half the
  speed. With the Intel C library (and opt-level z) the module was 3.7 MB / 1.6 MB / 1.3 MB.
  The build is reproducible: `scripts/build-wasm.sh` remaps `CARGO_HOME` and the workspace
  path out of the module (the only machine-specific bytes were panic-location strings), so the
  same toolchain, lock file and commit give a byte-identical `feel.wasm`
  from any directory (verified by rebuilding from a copy at another path). A plain
  `cargo build` without the script still works but embeds the builder's paths. The JS bundle is
  about 530 kB (180 kB gzip), mostly CodeMirror and lang-feel's completion docs.
- **Long loops** were quadratic in upstream dsntk (`for i in 1..100000` took tens of seconds);
  this repository makes them linear (`count(for i in 1..1000000 return i)` takes about 0.11 s
  in wasm). Expressions that are still too slow hit the 2 s timeout; the worker is terminated
  and restarted. In the playground a superseded evaluation is cancelled (its worker is replaced
  after a 100 ms grace period), so typing never queues up behind a runaway one.
- **Nesting and recursion limits.** Expressions nested deeper than 300 AST levels are a `syntax`
  error, and more than 200 nested function calls (e.g. runaway recursion) stop the evaluation with
  an `evaluation` error (CONTRACT §2). Both are sized for a browser worker's stack. A recursion
  whose function body is itself deeply nested can still exhaust the stack below 200 calls;
  that is reported as `crash` and the worker is replaced. Measured (`npm run stack -- --report`):
  190 nested calls run with a body nested 64 levels in a 4 MiB Node worker and in Wasmtime with a
  4 MiB wasm stack, 14 to 17 levels on Node's main thread, 8 levels with Wasmtime's default
  512 KiB wasm stack (CONTRACT §1).
- **Memory.** Linear memory is capped at 512 MiB, so a runaway allocation traps early (`crash`).
  A list element costs about 130 B while the list grows (a 64-byte value plus the spare
  capacity of the list; `count(for i in 1..1000000 return i)` grows memory by 128 MiB) and a
  one-entry context about 1 KB, so `for i in 1..2500000 return i` (after about 0.3 s) or a
  million one-entry contexts (after about 0.9 s) hit the cap before the 2 s budget;
  the runtime recognises Rust's "memory allocation of … failed" in that trap and the message
  starts with `out of memory:` (the playground shows "Out of memory — engine restarted" instead
  of a crash). Wasm memory never shrinks, so the runtime replaces a worker whose instance grew
  past 256 MiB. `instantiateFeel` (and any host without fuel or epoch interruption) cannot stop
  a long evaluation such as `for i in 1..1e30 return i`; use `FeelEngine` or a Wasmtime
  deadline for untrusted input.
- **Panics.** Any remaining panic inside dsntk traps the instance (`panic = "abort"`; integer
  overflow checks are on in release builds, so an overflow is a panic, never a wrong answer). The
  runtime recovers and adds the panic message to the error.
- **Time zones.** `tzOffsetMinutes` is one fixed offset per request, the browser's current
  offset. There are no DST rules for other dates. Without it, local time is UTC on every target.
- **Temporal comparisons.** Date-times and times with offsets or zones compare as instants
  (`@"…T10:00:00+01:00" < @"…T10:00:00Z"` is true, `@"10:00:00+02:00" = @"08:00:00Z"` is true;
  `is()` stays zone-strict). A zone-less (local) value is only ordered against another local
  value: `<`, `>`, `<=`, `>=` between a local and a zoned date-time are false, like dsntk's
  subtraction of the two (null), although `=` applies the fixed offset. `max`/`min` accept only
  numbers and strings in dsntk (`max(dt1, dt2)` is null); use `sort(…)[1]` or `if a < b then …`.
- **Unary-test semantics.** dsntk evaluates `? in (<tests>)`. A boolean
  expression test such as `? > 5` is compared with the input instead of being treated as
  "satisfied when true"; the core returns a warning for such tests. The usual forms `> 5`,
  `[1..10]` and `"a", "b"` work. An endpoint that is an arithmetic expression needs
  parentheses: dsntk parses `> 0.1 + 0.2` as `(? > 0.1) + 0.2` (a typed null, "invalid argument
  type … actual type is boolean"), while `> (0.1 + 0.2)` and `>= (x + 1)` work. See CONTRACT §2.
- **Parsing needs context.** Names with spaces or operators (`Grace's age`, `a+b`) are only split
  correctly when they are in the context, so the playground sends the context to `parse` too.
- **Syntax errors have no position in dsntk.** The playground locates them with the lezer-feel
  parser (squiggle, gutter marker, "Jump to error", and a lezer tree with positions); lezer-feel
  and dsntk can disagree on edge cases. `docs/EDITOR-PLAN.md` is the plan to make the engine the
  single source of syntax validity.
- **Context entries are sorted.** dsntk keeps contexts in a `BTreeMap`, so results list keys in
  name order.
- **Only Chromium was tested** in a real browser.

## Next steps: .NET host

The same `dist/feel.wasm` is meant to run unchanged on .NET through the
[Wasmtime NuGet package](https://www.nuget.org/packages/Wasmtime):

Do not copy `instantiateFeel` for a server: it runs on the calling thread with no way to interrupt
it, so it is for trusted input only.

1. `Engine` with `Config.WithFuelConsumption(true)` and/or `WithEpochInterruption(true)`. Fuel or
   epoch deadlines replace the browser's worker-terminate timeout, so a runaway loop traps the
   `Store` instead of hanging a thread. Also set `Store.SetLimits(memorySize: …)` to cap memory.
   Give wasm at least 4 MiB of stack (Wasmtime's `max_wasm_stack`, set on the `Config`; the
   default 512 KiB lets a deeply nested recursion trap early, CONTRACT §1) and call it on a
   thread whose native stack is larger than that (for example a `Thread` created with an 8 MiB
   `maxStackSize`).
2. Compile the `Module` once and cache it (or serialize it with `Module.Serialize`). Create one
   `Store` + `Instance` per evaluation, or a pool of them, because instances are single-threaded.
   After any trap (fuel, epoch, panic or stack overflow), throw the instance away, as the JS
   runtime does.
3. Provide WASI with `Linker.DefineWasi()` + `WasiConfiguration` (no preopens, no env), or with
   the same 10 functions the JS shim implements (CONTRACT §1; the current module imports six of
   them).
4. Call `_initialize` once, then `feel_alloc` → write the UTF-8 request → `feel_call` → unpack
   `(ptr << 32) | len` → read → `feel_free`. Exactly as in `packages/feel-runtime/src/instance.ts`.
5. Send `tzOffsetMinutes` from `TimeZoneInfo.Local.GetUtcOffset(DateTime.Now)`. Keep decimals as
   strings, or parse the `typed` DTO: .NET `decimal` holds only 28–29 digits, fewer than
   decimal128's 34.
6. Port the contract tests (`crates/feel-wasm/tests/contract.rs`) to xUnit against the same
   request/response JSON.

dotnet was not installed in the environment this repo was built in, so none of this has been run
yet.

## License

rheon is licensed under either of the Apache License 2.0 ([`LICENSE`](LICENSE)) or the MIT
license ([`LICENSE-MIT`](LICENSE-MIT)), at your option, the same terms as dsntk, whose copyright
notices are retained ([`NOTICE`](NOTICE)). The dashu crates are MIT OR Apache-2.0; CodeMirror,
lang-feel and lezer-feel are MIT. The playground ships the full notices of everything in its
bundle and in `feel.wasm` as `third-party-notices.txt`.
