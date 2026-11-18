# rheon

**rheon** (Greek ῥέον, "that which flows", the root of *panta rhei*) is a Rust toolkit for
decision and process automation. Its engines are built for native code and for WebAssembly
alike; the FEEL engine already runs unchanged in the browser, in Node and in any WASI runtime.

The foundation is in place: a FEEL and DMN 1.6 engine with exact decimal arithmetic. On that
core, rheon is growing into a BPMN toolbox, the building blocks for running processes, and a
framework that embeds decisions and processes in applications.

rheon builds on [dsntk](https://github.com/dsntk/dsntk-rs) (DecisionToolkit, by Dariusz Depta),
the Rust FEEL/DMN engine, as a permanent fork that keeps upstream's crate layout. On top of it
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
  server, the DMN model evaluator, B-FEEL, lenient and strict error modes, and the official DMN
  TCK at 3,392 of 3,407 (14 of the 15 failures need Java). Features the official TCK does not
  test yet have TCK-format tests in [`tck-addendum/`](tck-addendum/MANIFEST.md), 312 of 312
  passing; `scripts/tck.sh` runs both.
  The DMN engine is not in the wasm module; the FEEL module offers B-FEEL and the error modes
  (`language`, `errorMode`, CONTRACT §2).

The edge-case comparison with other FEEL engines is in
[`docs/ENGINE-COMPARISON.md`](docs/ENGINE-COMPARISON.md); known bugs and limitations in
[`docs/BACKLOG.md`](docs/BACKLOG.md).

## Architecture

```
 ┌──────────────────────────── browser ─────────────────────────────────────────┐
 │                                                                               │
 │  apps/playground  (Vite + Svelte 5 + CodeMirror 6 / @rheon/feel-codemirror)   │
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
| `docs/` | `CONTRACT.md` (ABI and JSON contract), `FORK.md` (every engine change against upstream, verification numbers, how to take upstream fixes), `BACKLOG.md`, `ENGINE-COMPARISON.md` |
| `scripts/` | `bootstrap.sh`, `env.sh`, `build-wasm.sh`, `feel-abi.mjs`, `smoke.mjs`, `stack-headroom.mjs`, `third-party-notices.mjs` |
| `tools/` | The identity check of full responses against the previous release (`tools/identity/`), the number oracle (`tools/number-oracle/`) and the frozen FEEL corpus of the parser's corpus test (`tools/feel-corpus/`; see `tools/README.md`) |
| `Cargo.toml` | One workspace for all of the above; the `wasm` build profile |
| `dist/feel.wasm` | Build output of the core, shared by every host (browser, Node, any WASI runtime) |

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
| `npm run stack` | `scripts/stack-headroom.mjs`: five constructs nested to the AST depth limit, a value nested to the value depth limit (500) and a recursion of 200 calls with a body nested to the AST limit must evaluate on Node's main thread, in a 4 MiB Node worker and in Wasmtime with its default 512 KiB and with a 4 MiB wasm stack, and a recursion of 66 levels through a `sort` callback with a body nested to the AST limit in the worker and in Wasmtime with 4 MiB; `npm run stack -- --report` prints the depths each host reaches |
| `bash tools/identity/run.sh <reference.wasm> dist/feel.wasm` | Every committed request through the previous release (`tools/identity/fetch-baseline.sh` puts it in `.tools/baseline/`) and the build; the full responses must be identical except the differences registered in `tools/identity/registered/` (see `tools/identity/README.md`) |
| `npm run tier:fix`, `tier:group`, `tier:release` | The test tiers (see below) |
| `npm run test:identity` | `tools/identity/test.mjs` (`node --test`): the identity check's own behaviour on the previous release and the build (identical runs, unregistered and unreproduced differences, malformed ledgers, traps, `--record`, numbers kept as written) |
| `bash tools/number-oracle/run.sh [quick\|full]` | Every number result of `dist/feel.wasm` judged against its truth under DMN 1.6, computed in Python with `decimal`, exact rationals and mpmath (`PYTHON` names an interpreter with the C `decimal` module and mpmath 1.3.0, see `tools/number-oracle/README.md`); the known wrong classes are listed in `known-wrong.json`, and a listed class that no longer fails fails the run. CI runs the quick size |
| `bash scripts/tck.sh [--only <folder>]…` | The official DMN TCK with DecisionToolkit's patches and the tests of `tck-addendum/`, against the dsntk server of this tree with DecisionToolkit's runner; the three repositories are fetched into `.tck/` at the commits in `scripts/tck-pins.txt`, and the run exits 1 when a test fails or passes against `docs/tck-baseline.txt` (`node scripts/tck-summary.mjs .tck/out/tck_results.csv --update-baseline` rewrites it). `npm run test:tck-summary` checks the comparison on small result files |
| `scripts/publish-crates.sh` | Publishes the `rheon-*` crates that crates.io lacks at the workspace version, waiting out the new-crate rate limit (see `docs/FORK.md`, "Publishing") |
| `cargo test --workspace` | The engine's own suites (about 4,200 tests). A few upstream tests need things CI does not have and are skipped there: `external_functions` and `dmn_3_0076` (a Java external-function service), `compare_the_number_of_models` (a DMN TCK checkout at `../tck`), the server's `utils::tests::getting_*` (a checkout directory named `dsntk`); the tiers pass the same skips (`scripts/tier.mjs`) |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p rheon-feel-number …` | The API documentation of the ten published crates (the `-p` list is in `scripts/tier.mjs`, which runs it in the release tier), with every rustdoc warning, such as an intra-doc link that resolves to nothing, an error |
| `RHEON_FULL_TESTS=1 cargo test --release -p rheon-feel-number -p rheon-feel -p rheon-feel-parser -p rheon-feel-wasm` | The randomized and exhaustive tests of those crates at full size (by default they run a smaller number of cases; for `rheon-feel-wasm`, the edge-operand product with all its binary templates). `RHEON_PARSER_CORPUS=<absolute path of tools/feel-corpus>` with `-- --include-ignored` adds the parser's corpus test |

### Test tiers

`scripts/tier.mjs` runs the checks above in tiers and ends with a table of every step (build or
run, seconds, result), the build and run totals, the elapsed time and the verdict. Steps run
side by side, up to `--jobs` at once (default: half the cores, at most 4): a step starts when
the steps it needs have passed, and steps that share a cargo build directory, the identity
cache or the runtime package run one after the other. The tests run through
`scripts/run-tests.mjs`, which builds them as `cargo test` does and runs the test programs side
by side. The budget limits the run time summed over the steps; compile steps (cargo check, test
builds, the wasm build, clippy, the documentation) are timed but not judged. Each step's output
is in `.tools/tier/<tier>/`.

| Command | Runs | Budget (run time) |
|---|---|---|
| `npm run tier:fix -- [-p <crate>]… [--test <filter>] [--base <rev>]` | After every change: `cargo fmt --all --check`, `cargo check` of the product crates, the fix's own test (`--test`), the tests of the touched crates (`-p`, or the crates owning the files that differ from the merge base with `main`), the wasm build and the identity check on the committed sets; a changed ledger file adds the Rust copies of the TCK tests and runs each changed release-tier ledger file, and a changed tool its own tests. Stops at the first failure | 2 min |
| `npm run tier:group -- [--only rust\|module\|carried]` | After a group of changes, in three sets (CI runs each as a job): `rust`, the fix tier's steps for every product crate (all their tests with CI's skips), clippy with warnings denied, the tier runner's and the TCK summary's tests; `module`, the wasm build, the identity check, the identity tool's tests, the number oracle's quick run, the contract tests on wasm32, `test:runtime`, `smoke`, `stack` and `build:playground`; `carried`, the tests and clippy of the crates carried from dsntk and `scripts/tck.sh`, which runs against their server. Runs every step and reports every failure | 6 min |
| `npm run tier:release -- [--without-large-sets]` | Before a release: the group tier, then the identity check with `--release` and with the large sets (`IDENTITY_LARGE_SETS`), each with `--verify-cache`, the oracle's full run (in shards, `ORACLE_JOBS`), the full-size randomized tests and the parser corpus, `npm run stack -- --report` and `cargo doc` of the published crates with warnings denied. An input kept outside the repository is required unless its `--without` flag names it, and then the step is listed as NOT RUN | 30 min |
| `npm run tier:speed` | Before a release, on a quiet machine: the speed suite (`RHEON_BENCH`, the directory of `bench.mjs`: the previous release and the build back to back, engine mode, results beside the logs). It is no step of the release tier: measured right after other steps the machine is warm, and the build, measured second, reads slower than it is | none |

The budgets hold on a developer machine; with `CI` set the times are reported without a budget.
Measured on an Apple M2 (4 performance and 4 efficiency cores) with Node 26, warm builds, the
engine after 0.7.0: the group tier about 39 s elapsed (128 s summed over its steps; all 8,627
product tests 14 s), the release tier about 7.6 minutes elapsed with the speed suite still in it
(the oracle's full run 3.9 min in shards beside the full-size tests, 3 min; the speed suite 2.8 min).
Before the steps ran side by side the group tier took about 2.8 minutes and the release tier
about 13.4.
`RHEON_WASM_TARGET_DIR` gives the wasm steps a target directory of their own. `npm run
test:tier` checks the runner on step tables of its own.

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

- a **pure-Rust decimal128 core** (`feel-number`, on dashu-float; no C library); correctly
  rounded `exp`, `ln`, `log` and fractional `**`, and every number result judged by the number
  oracle against DMN 1.6 (`tools/number-oracle/`);
- **wasm32 support** (Java external functions become a null with a reason; 32-bit conversions);
- **robustness**: every user-reachable panic found in review is a FEEL null or an error, a call
  depth limit, an iterative AST drop, linear `for` loops and list literals (upstream was quadratic);
- **engine fixes** found by review and by the DMN TCK: scale rounding, lost-step `for` ranges,
  midnight wrap of `time + duration`, cross-offset temporal ordering, a cubic lexer path, the full
  FEEL scale range, back-references in `matches`, readable null reasons, scope frames left behind
  by early returns, exponential subtraction reasons, a trap on an iteration variable that starts
  with `in`, the named parameter of a range literal, quadratic list grammar actions, date ranges
  that end at the first or last supported date;
- **performance**: names in scope found through an index instead of an exponential set of keys,
  loops and filters that move their contexts, shared function definitions, a lexer and parser
  without per-token copies, shared strings, and the wasm module built at opt-level 3
  (`docs/FORK.md` 39-54).

DMN TCK (`scripts/tck.sh`, the official suite with DecisionToolkit's patches): rheon passes
3,392 of 3,407; the 15 failures are Java external functions (14) and one decision-service case,
both in the DMN layer (`docs/tck-baseline.txt` lists them). The 312 tests of `tck-addendum/`
all pass.

Crate naming: a crate whose source rheon has changed is renamed `rheon-*` (today
`rheon-feel`, `-feel-number`, `-feel-parser`, `-feel-evaluator`, `-feel-temporal`,
`-feel-regex`, `-model` and `-model-evaluator`) and published on crates.io, so a Rust project
can depend on `rheon-feel-evaluator = "0.3"` directly. Upstream crates rheon has not changed are
used from crates.io at dsntk's 0.3.0 release (`dsntk-common`, `-macros`, `-feel-grammar`,
`-recognizer`). The dsntk crates carried with manifest-only changes (`dsntk-evaluator`,
`-workspace`, `-gendoc`, `-server`, `-examples` and the `dsntk` CLI) stay in the tree under their
`dsntk-*` names and are never published from here. Directories keep upstream's names either way.

## Known limitations

- **Java external functions are not supported.** They need a JVM, and on wasm32 they evaluate to
  null with a reason.
- **Rounding of `exp`, `ln`, `log` and `**`.** These are correctly rounded to 34
  digits (the exact result, rounded once, half-even). Upstream dsntk with the Intel library is
  1 ulp off for roughly 11–16 % of inputs and occasionally more, so the last digit of such results
  can differ from upstream dsntk; e.g. `12.2384283 ** 4.25` is `41959.85737359436186095331070746801`
  here (the exact value continues …7468005517…, so …801 is the correctly rounded 34-digit
  result), while Intel gives …800. The number oracle (`tools/number-oracle/`) judges every
  number result against DMN 1.6; the classes that still deviate are listed in its
  `known-wrong.json`.
- **Integer-exponent `**`.** `x ** n` with an integer `n` is the exact power rounded once while
  it has at most about 4,900 digits (its reciprocal one correctly rounded division), and the
  correctly rounded `exp(y ln x)` beyond: `1.05 ** 360` is
  `42476396.40868002039560415895318542`, and `2 ** -20000 = 0.5 ** 20000`. Upstream dsntk
  multiplies with a rounding after every product, so its last digits differ, by 294 ulp for
  `1.1 ** 100000` and about 89,000 ulp for `1.0001 ** 1000000`. A large exponent beyond the
  exact power costs about 0.4 ms per evaluation.
- **Size.** `feel.wasm` is about 3.6 MiB raw (3.8 MB), about 995 KiB gzip and 650 KiB brotli
  (`npm run build:wasm` prints the exact figures). The `wasm` profile builds at opt-level 3; at
  opt-level z the same code is about 3.0 MiB raw and 540 KiB brotli and runs at about half the
  speed. With the Intel C library (and opt-level z) the module was 3.7 MB / 1.6 MB / 1.3 MB.
  The build is reproducible: `scripts/build-wasm.sh` remaps `CARGO_HOME` and the workspace
  path out of the module (the only machine-specific bytes were panic-location strings), so the
  same toolchain, lock file and commit give a byte-identical `feel.wasm`
  from any directory (verified by rebuilding from a copy at another path). A plain
  `cargo build` without the script still works but embeds the builder's paths. The JS bundle is
  mostly CodeMirror.
- **Long loops** were quadratic in upstream dsntk (`for i in 1..100000` took tens of seconds);
  this repository makes them linear (`count(for i in 1..1000000 return i)` takes about 0.11 s
  in wasm). Expressions that are still too slow hit the 2 s timeout; the worker is terminated
  and restarted. In the playground a superseded evaluation is cancelled (its worker is replaced
  after a 100 ms grace period), so typing never queues up behind a runaway one.
- **Nesting and recursion limits.** Expressions nested deeper than 300 AST levels are a `syntax`
  error, and more than 200 nested function calls (e.g. runaway recursion) stop the evaluation with
  an `evaluation` error (CONTRACT §2). FEEL calls take no native stack, so 200 nested calls run
  with a body nested to the AST limit on Node's main thread, in a worker and with Wasmtime's
  default 512 KiB wasm stack; a recursion through a callback of the engine (the comparison of
  `sort`) takes native stack per level, and a host that runs out of it reports `crash` and the
  worker is replaced. Measured (`npm run stack -- --report`): 66 levels through a `sort`
  comparison with a body nested to the AST limit run on every host measured (CONTRACT §1).
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
- **Unary-test semantics.** The engine evaluates `? in (<tests>)`. A boolean
  expression test such as `? > 5` is satisfied when it is true. The usual forms `> 5`,
  `[1..10]` and `"a", "b"` work; a tested value of another kind than the tests gives null. The endpoint of a comparison
  takes the arithmetic after it: `> 0.1 + 0.2` tests `? > (0.1 + 0.2)`. See CONTRACT §2.
- **Parsing needs context.** Names with spaces or operators (`Grace's age`, `a+b`) are only split
  correctly when they are in the context, so the playground sends the context to `parse` too.
- **The editor reads a text as the engine does.** The playground's FEEL editor takes its
  colours, its error marks (squiggle, gutter marker, "Jump to error"), its completions and the
  tree with positions from the engine's own parser (`syntax` and `complete`, docs/CONTRACT.md
  §3), through `@rheon/feel-codemirror`. An `evaluate` syntax error itself carries no position.
- **Context entries are sorted.** dsntk keeps contexts in a `BTreeMap`, so results list keys in
  name order.
- **Only Chromium was tested** in a real browser.

## Hosts outside a JS engine

`dist/feel.wasm` runs in any WASI preview1 runtime that provides the imports of CONTRACT §1
(`npm run stack` runs it in Wasmtime):

- Give wasm at least 4 MiB of stack (Wasmtime: `max_wasm_stack` on the `Config`,
  `-W max-wasm-stack=4194304` on the command line; the default 512 KiB lets a deeply nested
  recursion trap early, CONTRACT §1), and call it on a thread whose native stack is larger than
  that.
- Bound every request with fuel or epoch interruption and cap the store's memory: nothing else
  stops a runaway loop on the calling thread. `instantiateFeel` has neither and is for trusted
  input only.
- An instance is single-threaded: use one per thread or a pool, and discard an instance after any
  trap (fuel, epoch, panic or stack overflow).
- Capture the module's stderr (fd 2) to tell the traps apart: a failed allocation at the 512 MiB
  cap writes `memory allocation of N bytes failed` before it traps, so the request needed more
  memory than the cap allows, while a panic writes its message (CONTRACT §1, §4).
- Call `_initialize` once, then `feel_alloc` → write the UTF-8 request → `feel_call` → unpack
  `(ptr << 32) | len` → read → `feel_free`, as `packages/feel-runtime/src/instance.ts` does.
- Send `tzOffsetMinutes` with each request, and keep decimals as strings or read the `typed`
  result: decimal128 has 34 digits, more than most host decimal types.

## License

rheon is licensed under either of the Apache License 2.0 ([`LICENSE`](LICENSE)) or the MIT
license ([`LICENSE-MIT`](LICENSE-MIT)), at your option, the same terms as dsntk, whose copyright
notices are retained ([`NOTICE`](NOTICE)). The dashu crates are MIT OR Apache-2.0; CodeMirror
is MIT. The playground ships the full notices of everything in its
bundle and in `feel.wasm` as `third-party-notices.txt`.
