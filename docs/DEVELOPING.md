# Developing rheon

How the repository is laid out, how to build it, and how it is tested. For what rheon is and how
to use it, see the [README](../README.md).

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

## Build

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
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p rheon-feel-number …` | The API documentation of the published crates (the `-p` list is in `scripts/tier.mjs`, which runs it in the release tier), with every rustdoc warning, such as an intra-doc link that resolves to nothing, an error |
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

## Relationship to dsntk, in detail

rheon is a permanent fork: the engine is developed here, in its own direction, and
contributing the changes back to dsntk is not a goal. The dsntk crates sit at the repository root
in upstream's directory layout, and `upstream` is a git remote
(`git remote add upstream https://github.com/dsntk/dsntk-rs.git`), so individual upstream fixes
can still be taken by cherry-pick (`docs/FORK.md` has the procedure).
[`docs/FORK.md`](FORK.md) lists every engine change with its verification numbers. In
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
can depend on `rheon-feel-evaluator = "0.9"` directly. Upstream crates rheon has not changed are
used from crates.io at dsntk's 0.3.0 release (`dsntk-common`, `-macros`, `-recognizer`). The dsntk crates carried with manifest-only changes (`dsntk-evaluator`,
`-workspace`, `-gendoc`, `-server`, `-examples` and the `dsntk` CLI) stay in the tree under their
`dsntk-*` names and are never published from here. Directories keep upstream's names either way.
