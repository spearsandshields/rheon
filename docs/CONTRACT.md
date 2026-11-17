# rheon FEEL wasm contract

This is the single source of truth shared by the Rust core (`wasm/feel-wasm`), the JS runtime (`packages/feel-runtime`) and the playground (`apps/playground`). Change it here first.

## 1. Wasm module (`dist/feel.wasm`)

- Target: `wasm32-wasip1`, built as a **reactor** (`cdylib` linked with Rust's self-contained `crt1-reactor.o`, see `wasm/feel-wasm/build.rs`). `_initialize` **is** exported: the host calls it exactly once after instantiation (a second call traps), e.g. `wasi.initialize(instance)` or `instance.exports._initialize()`.
- Shadow stack: 10 MiB (stack-first layout). The parser is recursive, and the wasm frames also use the host's native stack (in a browser the JS engine's; a worker's is smaller than Node's main thread's). The core therefore rejects ASTs deeper than 300 levels and stops evaluations with more than 200 nested function calls or a value nested more than 500 levels deep (§2, §3). The evaluator runs FEEL function calls, loops and nesting on the heap, so a FEEL recursion takes no native stack per call: every construct nested up to the AST limit, and a FEEL recursion of 200 nested calls whose function body is nested up to the AST limit, evaluate on each host measured here, Node's main thread and Wasmtime's default 512 KiB wasm stack included. A function that the engine calls back (the comparison of `sort`, the function of `list replace`) runs as a call of its own and takes native stack for each level of a recursion through it; a host gives the module the stack below for that. Running out of a stack is a trap ("crash": discard the instance). Measured with `npm run stack -- --report` (macOS arm64, Node 26, Wasmtime 49.0.3): the recursion of 200 calls reaches the AST limit (a body nested 289 levels) on Node's main thread, in Node workers with 2, 3 and 4 MiB stacks (Liftoff and TurboFan) and in Wasmtime with a wasm stack of 512 KiB to 16 MiB, and so does a recursion of 66 levels through a `sort` comparison (199 nested calls, the comparison's body nested 284 levels).
- Host stack: a host outside a JS engine gives the module at least 4 MiB of wasm stack (Wasmtime: `Config::max_wasm_stack`, on the command line `-W max-wasm-stack=4194304`; the default is 512 KiB) and calls it on a thread whose native stack is larger than that, because Wasmtime runs wasm on the calling thread's stack. JS hosts run untrusted requests in a worker, as `FeelEngine` does (Node's default worker stack is 4 MiB); a main thread reaches less (above).
- Linear memory starts at 11.4 MiB (182 pages: the 10 MiB stack and the data), grows on demand and is capped at **512 MiB** (`--max-memory`): a runaway allocation fails early (a trap) instead of growing the host to 4 GiB. Memory never shrinks, and an evaluation that builds ever longer strings grows it past its live data (freed blocks too small for the next string, docs/BACKLOG.md); hosts should recycle instances that grew large (the JS runtime does above 256 MiB).
- Both limits are checked by `scripts/build-wasm.sh`, which also requires the `_initialize` export; `scripts/stack-headroom.mjs` (`npm run stack`, run by CI) checks that five constructs nested to the AST depth limit and a value nested 500 levels deep (built through `partial`, compared, returned and rendered) evaluate, with a recursion of 200 calls whose body is nested to the AST limit, on Node's main thread, in a Node worker with a 4 MiB stack and in Wasmtime with its default 512 KiB and with a 4 MiB wasm stack, and that a recursion of 66 levels through a `sort` comparison whose body is nested to the AST limit evaluates in the worker and in Wasmtime with 4 MiB; `build.rs` fails the build when `crt1-reactor.o` is missing instead of silently producing a module without `_initialize`. `build.rs` also reads the engine version (the dsntk fork commit pinned in the root `Cargo.toml` and the upstream base commit) and fails when the pins are inconsistent.
- The engine is pure Rust (dsntk fork with its own decimal128 core on dashu-float): no C code is linked, so no C toolchain or sysroot is involved in the build.
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
  "op": "evaluate" | "evaluateAll" | "parse" | "info",
  "dialect": "expression" | "unaryTests",   // evaluate/parse; default "expression"
  "expression": "string",                    // evaluate/parse
  "context": { /* JSON object */ },          // evaluate; optional, default {}
  "contextFormat": "json" | "feel",          // optional, default "json"; with "feel", `context` is a STRING holding a FEEL context literal
  "input": <JSON value>,                      // unaryTests only: the value tested (`?`). If absent, context["?"] is used
  "tzOffsetMinutes": -240,                    // optional: host local UTC offset in minutes, used by now()/today()
  "language": "feel" | "b-feel",             // evaluate; default "feel"; "b-feel" is DMN 1.6 chapter 11
  "errorMode": "lenient" | "strict",         // evaluate; default "lenient" (DMN 1.6 §7.3.8)
  "maxSteps": 1000000                        // evaluate; optional: a budget of steps, a positive integer; no default
}
```

- `language: "b-feel"` evaluates in B-FEEL (DMN 1.6 chapter 11): null only means missing data; where FEEL returns null for an error, B-FEEL returns `false` (boolean operators and functions; `true` for `!=`), the default of the function's result type (`0`, `""`, the epoch, `P0M`, `[]`, `(0..0)`), or converts mismatched arithmetic operands (`"The result is: " + 1` = `"The result is: 1"`, `1 + null` = `1`). The FEEL errors behind these defaults still appear in `diagnostics`.
- `errorMode: "strict"`: when any error occurred during the evaluation (a null created with an error reason, see `diagnostics`), the response is an `evaluation` error whose message is `strict error mode: <first error>`, instead of a result. `"lenient"` returns the result and lists the errors.
- `maxSteps`: a budget for one evaluation. Every function call (of a user-defined, built-in or model function) and every item that a `for`, `some`, `every` or a filter visits is a step. The step after the last one of the budget stops the evaluation, and the response is an `evaluation` error with the message `evaluation took more than <maxSteps> steps; evaluation stopped`. Without the field an evaluation has no budget of steps; the limits of §1 still bound it. A value that is not a positive integer is a `request` error.

- `parse` also accepts `context`/`contextFormat` (optional): the parser needs to know which multi-word names exist (`Grace's age + 1` only parses as a name if `Grace's age` is in the context). For `dialect: "unaryTests"`, `?` is always a known name.
- Unary tests are evaluated as dsntk's decision tables do: the tested value is bound to the name `?` in the scope and the result is `? in (<unary tests>)`, i.e. a boolean (or null). Note (dsntk at the pinned commit): a boolean *expression* used as a test, e.g. `? > 5`, is **compared** with the input (`7 = true` → `false`) instead of being "satisfied when true" as in DMN 1.2+. Use `> 5` instead. The response carries a warning for every test that is a comparison/logical expression (or `not(...)`) using `?`.
- If neither `input` nor `context["?"]` is given for unary tests, `?` is null and a warning is returned.
- The endpoint of a unary comparison is an expression and binds like the right side of a comparison: `> 0.1 + 0.2` tests `? > (0.1 + 0.2)` and `>= x + 1` tests `? >= (x + 1)`; `and`, `or`, a comparison and `between` end the endpoint (`<= 10 = (null..10]` compares the unary comparison with a range). `< d.year` compares with the year of `d` (DMN 1.6 §10.3.2.7, "Endpoints can be expressions"); a property of the unary value itself is read as `(< 10).end`.
- `tzOffsetMinutes`: integer in `-1080..=1080` (±18:00), otherwise a `request` error (every other integer, including `i64::MIN`). It is a fixed offset (no DST rules), applied to every "local" (zone-less) date-time computation of that request — `now()`, `today()`, and comparisons/subtractions of local date-times — via a process-global override in rheon-feel-temporal (the fork's "local offset override" commit), reset after every request. Without it, local time is **UTC on every target**: chrono's `Local` is hard-wired to UTC on `wasm32-wasip1`, and native builds apply offset 0 too instead of the host zone, so results never depend on where the core runs.
- Limits: an expression (or FEEL context literal) whose AST is deeper than **300** levels is rejected (`syntax`, resp. `context`) before any recursive code runs; an evaluation with more than **200** nested function calls (recursion) is stopped and reported as an `evaluation` error, never as a partial result; a call of a user function, of a built-in, and the call that `sort` and `list replace` make of the function they are given each count. So is an evaluation that produces a value nested more than **500** levels deep (`values are nested too deeply (more than 500 levels); evaluation stopped`): a scalar is 0 levels deep, a list or a context is one level deeper than the deepest value it holds (so an empty one is 1 level deep), and a function is one level deeper than its closure context; a value 500 levels deep is passed, bound and returned like any other. A loop that would deepen its results further, and `context put` with a key list longer than the limit, stop before they build the deeper value; a key list whose path is blocked by an entry that is not a context builds nothing and is null, whatever its length. So is an evaluation in which `matches`, `replace` or `split` meets a valid pattern that exceeds a limit of the regex engine (its backtracking budget or stack, the compiled size, the nesting depth): `matches: the pattern '^(a|a)*\1$' exceeds the regex engine's backtracking limit; evaluation stopped`. An invalid pattern is not a limit: its result is null (DMN 1.6 Table 75, note 2). So are invalid flags: a character other than `s`, `m`, `i`, `x` and `q` (XQuery 1.0 and XPath 2.0 F&O §7.6.1.1; `q` is the flag of F&O 3.1, kept as an extension), or flags that are not a string; `""` and null are no flags. `replace` is null for a replacement string that F&O §7.6.3 rejects: a `$` not followed by a digit and not escaped as `\$`, or a `\` that is not part of `\\` or `\$`; with `q` the replacement is used as is. `replace` and `split` are null for a pattern that matches the zero-length string (`""`, `x*`, `^`, `\s*`; F&O §7.6.3 and §7.6.4), and `split` of `""` is `[]`.
- Shared values: a list or a context is shared, not copied, when it is bound to a name, passed to a function, read from the input context, iterated, filtered or put into another list or context; a change (`append`, `context put`, a filter's result) builds a new value and leaves the shared one as it was. So a value can hold the same list or context on many paths (`(for i in 1..30 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]` holds 2²⁹ paths to one list) while its memory grows with its distinct lists and contexts only. The operations that compare or test values visit each shared list or context once per pair or per call: `=`, `!=`, `in`, `instance of`, `distinct values`, `index of`, `list contains`, `union`, `intersection`, and the parser's names of an input context take time linear in the distinct parts of the values (equality at most quadratic, one step per pair of distinct shared lists or contexts); `count` reads the length. An operation that writes the value out follows every path, so its output is exponential in the doubling: the result display, `string()`, `flatten` and `...` of a value doubled 30 times end at the 512 MiB memory cap (a trap, §1); there is no limit on the size of an output.

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
    "nullReason": "string?" // when the null carries a reason, that text: an error, or a note on a null the specification defines for its input (a missing context entry), which is no error
  },
  "warnings": [ { "message": "string" } ],
  "diagnostics": { "count": 1, "messages": ["string"] },  // errors recorded during evaluate (null values created with an error reason; a note is not an error), first 64, consecutive repeats once
  "elapsedMicros": 123
}
```

Messages: every message of a response (`error.message`, `result.nullReason`, the `null(…)` of `result.feel`, `diagnostics.messages[]`, `warnings[].message`) holds at most 1,000 characters; a longer one keeps its first 999 and ends with `…`. A value, type, name or input text quoted in a message is quoted whole when its text is at most 120 characters, otherwise as its first 120 characters, `…` and its size when it has one (`N items`, `N entries`, `N characters`), and a number is quoted in scientific notation when its plain text would exceed 40 characters (`… was rounded to 2E-6176`).

`display` rules: numbers in their canonical text, which `feel`, `typed` and FEEL's `string()` also use: the exact value in plain digits, without an exponent and without trailing zeros after the decimal point, and `0` for every zero, which has no sign (`1.50` is `1.5`, `decimal(1, 2)` is `1`, `0 * 1e-6000` and `0 * -5` are `0`; DMN 1.6 §10.3.2.3.1: "There is no distinction between -0 and 0", "many different pairs have the same value"; Table 40 writes some results with their scale, `decimal(1, 2)` as `1.00` and `.10 * 30.00` as `3.0000`: the text shows the value only, and the values agree); strings JSON-quoted/escaped; `true`/`false`/`null`; temporal values, ranges and other non-JSON values as their FEEL text JSON-quoted (`"2026-10-16"`, `"P1DT2H"`, `"[1..10]"`), except a date and time with an IANA zone, which is ISO 8601 with its offset and the zone in brackets (`"2007-12-03T10:15:30+01:00[Europe/Paris]"`, DMN 1.6 Table 48; the offset is the zone's at that date and time; `feel` keeps the FEEL form `…@Europe/Paris`); when that offset has seconds, as local mean time before the standard zones does, it is written `±hh:mm:ss` (`"1900-01-01T12:00:00+00:09:21[Europe/Paris]"`), a deliberate departure from ISO 8601 and XML Schema Part 2 §3.2.7.3, which write an offset as `±hh:mm` only: the seconds keep the instant, as java.time's zoned text does, where whole minutes would name another one; functions as their quoted type (`"function<Any>->Any"`); lists/contexts as `[...]`/`{"key": value}` on one line when the line fits 80 columns, otherwise expanded with two-space indentation. `warnings` currently reports rounded/underflowed input numbers, missing unary-test input, boolean unary tests using `?`, and fractional seconds beyond nanoseconds that a time, date and time or days and time duration literal or text, or the seconds number of `time(h, m, s[, offset])`, lost when they were truncated (in the expression or a FEEL-format context; `fractional seconds of '10:00:00.1234567891' truncated to nanoseconds; dropped digits: '1'`, no warning when every dropped digit is 0), and the fraction of a month that a years and months duration times or divided by a number dropped when it was truncated toward zero (`fractional months of 'P10Y11M / 2.5' truncated to whole months; dropped fraction: '0.4'`, no warning for a whole number of months). The warnings of the context and the input come first (a FEEL-format context's among them), then those of the unary tests, then those of the evaluation, each in the order they occurred: a months warning of the expression can come before a seconds warning. A warning repeated in a row is kept once, and at most 64 of the evaluation. A warning is not an error: it does not count in `diagnostics` and strict mode does not stop at it.

Diagnostics come from the evaluation that is made: `and` and `or` evaluate their right operand only when the left one does not decide the result (DMN 1.6 Table 51: `false and x` is false and `true or x` is true whatever `x` is), so the errors and warnings of a right operand that cannot change the result are not recorded, strict mode does not stop on them, and a recursion guarded by `and` or `or` returns its value. The left operand is always evaluated. A filter `l[e]` evaluates `e` once in the enclosing scope (DMN 1.6 Table 69, row 1: "e1 is a list and e2 is an integer"): a number selects the item; anything else selects nothing, so the errors and warnings of that evaluation are not recorded, and `e` is evaluated for each item with the item in scope (`sum(items[amount > 100].amount)` records no error and passes strict mode). An abort in that evaluation, such as the call-depth limit, still stops the evaluation.

`typed` is dsntk's `ValueDto`; for values it has no DTO form (ranges, functions, built-in functions) all three fields are `null` (`{"simple": null, "components": null, "list": null}`).

Non-finite decimals: FEEL has no infinity/NaN. Any arithmetic or function result that overflows decimal128 (e.g. `9e6144 * 10`, `9e6144 + 9e6144`, `1 / 1e-6176`) is `null` with nullReason `result is not a finite number (decimal overflow)`.

Numbers are IEEE 754 decimal128 (34 significant digits, half-even rounding, exponent range -6176..=6111), computed by the repository's pure-Rust decimal core. Following DMN 1.6 §10.3.2.3.1, every operation is meant to give its exact result rounded once to 34 digits: `+ - * /`, `sqrt`, comparisons and parsing do, and `exp`, `ln`, `log` and `**` are **correctly rounded**, which native dsntk (Intel's BID128 library) is not always (1 ulp in roughly 11–16 % of inputs), so those results can differ from a native dsntk in the last digit. The number oracle (`tools/number-oracle/`) judges every number result of the module against these rules; the classes of results that still deviate are listed with their reasons in its `known-wrong.json`. An exponent outside -999,999,999..999,999,999 gives null for every base (DMN 1.6 Table 61: "value(e2) is a number in the range [-999,999,999..999,999,999]"; `1 ** 1e10` is null with the reason `exponentiation exponent is out of range -999999999..999999999`). `**` with an integer exponent (Table 61: "p is limited to 34 digits") is the exact power rounded once while it has at most 16,384 bits (about 4,900 digits; a negative exponent: one correctly rounded division), so its ties are exact (`5 ** 50`, `2 ** -50`); beyond that it is `exp(y ln x)` at 45 to 560 working digits, accepted when the computed digits are at least 10^7 units of the last working digit away from a half-way point: an inexact power is never a tie, and a result is correctly rounded unless the exact power lies within about 10^-520 (relatively) of a half-way point, which no known operand does.

Scale rounding (`round up/down/half up/half down`, `floor`, `ceiling` with a scale) rounds the number once, by the function's mode, to a multiple of 10^-scale (DMN 1.6 Table 77): every scale of -6111..6176 is valid for every number, a zero or a tiny one included (`round half up(0, 2)` = `0`, `round up(1e-6000, -6000)` = 1 followed by 6,000 zeros), a number with no digit below the scale is itself (`ceiling(0.25, 6145)` = `0.25`), and `round half down` goes to the nearest neighbour and only a tie toward zero (`round half down(2.56)` = `3`). A scale outside that range is null. Date-times and times whose zones carry an offset compare as instants for `=`, `<`, `<=`, `>`, `>=` and ranges; a local (zone-less) value is ordered only against another local value (false otherwise, like dsntk's subtraction). `time + days and time duration` wraps around midnight.

`op: "evaluateAll"` evaluates several expressions over one context, which is read once:

```jsonc
// request
{
  "op": "evaluateAll",
  "context": { /* as for evaluate */ }, "contextFormat": "json", "tzOffsetMinutes": 0, "language": "feel",   // for all evaluations
  "evaluations": [                              // required; each entry: the fields of one evaluation
    { "id": "total",                            // optional, a string or a number: repeated in the answer of the entry
      "expression": "string", "dialect": "expression", "input": <JSON value>, "errorMode": "lenient", "maxSteps": 1000000 }
  ]
}
// response
{
  "ok": true,
  "contextWarnings": [ { "message": "string" } ],  // the warnings of reading the context, once; empty when there are none
  "results": [ /* one answer per entry of `evaluations`, in their order */ ],
  "elapsedMicros": 123
}
```

- Each answer is the response that `op: "evaluate"` gives for that entry with the same context: a success (`ok`, `result`, `warnings`, `diagnostics`, `elapsedMicros` of that entry) or a failure (`ok: false`, `error`), with the `id` of the entry when it has one. Its `warnings` are those of the entry (its input, its unary tests, its evaluation); the warnings of the context stand in `contextWarnings`, not in the answers.
- An entry that fails (a syntax error, a strict-mode error, its step budget, a field with a value that is not allowed, an entry that is no object) is a failure in its place, and the other entries are answered. A failure of the request itself (malformed JSON, a context that does not convert, `evaluations` missing or no array, an invalid `language`, `contextFormat` or `tzOffsetMinutes`) is the failure response below, without results.
- The entries do not see each other: the tested value of unary tests (`?`) is bound for its entry only, and nothing an entry evaluates changes the context for the next. Nothing of the request is kept after the response.
- The module has no limit on the number of entries; `maxSteps` bounds the work of each. A trap (§1) ends the whole request.

`op: "parse"` success: `{ "ok": true, "tree": "<dsntk AstNode::tree() ascii, no color>", "elapsedMicros": n }`

`op: "info"` success: `{ "ok": true, "engine": "rheon", "engineCommit": "<40-hex repository commit>[-dirty]", "upstreamCommit": "<40-hex upstream dsntk-rs commit>", "numberBackend": "dashu", "version": "<rheon-feel-wasm crate version>" }`. `engineCommit` is the rheon repository commit the module was built from (`git rev-parse HEAD` at build time, `-dirty` appended when the work tree had uncommitted changes, `.cargo_vcs_info.json` in a crate packaged by cargo, `"unknown"` when built outside both without `RHEON_COMMIT` set), `upstreamCommit` the dsntk-rs commit the engine crates are based on (`[package.metadata.rheon]` in `wasm/feel-wasm/Cargo.toml`), and `numberBackend` the feel-number backend (`rheon_feel_number::NUMBER_BACKEND`, always `"dashu"`).

Failure (the instance stays healthy):

```jsonc
{ "ok": false, "error": { "kind": "request" | "syntax" | "context" | "evaluation" | "internal", "message": "string" } }
```

- `request`: malformed request JSON (including invalid UTF-8 and JSON strings with unpaired `\uD800`-style surrogate escapes, which the core can not represent) or unknown op. A request value quoted in the message is a preview when its JSON text is longer than 120 characters (`invalid 'dialect' "xxx…… (200 characters) (expected …)`).
- `syntax`: the FEEL expression or unary tests failed to parse, or are nested too deeply (§2). The message is dsntk's, without its `<ParserError>` prefix; dsntk gives no position. It quotes the input whole when it is at most 120 characters, otherwise as its first 120 characters, `…` and its length (`syntax error: 1+1+…+1+… (40002 characters)`).
- `context`: the input context is invalid (bad JSON shape, JSON nested too deeply, bad FEEL context literal, or a FEEL context nested too deeply, recursing too deeply, holding a value nested more than 500 levels deep or exceeding a regex engine limit).
- `evaluation`: evaluation was stopped by a limit (§2): more than 200 nested function calls, a value nested more than 500 levels deep, or a regex engine limit. FEEL usually yields null instead; that is a *success* with `isNull: true` and `nullReason`. Null reasons are meant to be readable (`time: invalid time literal '…'`, `log: argument must be a positive number`, `exp: result is not a finite number (decimal overflow)`); a value quoted in a reason is its text when that is at most 120 characters, otherwise a preview: its first 120 characters, `…` and its size when it has one (`equal err '[1, 2, 3, …, 32, … (1000 items)' =?= '1'`; sizes are `N items`, `N entries` and `N characters`); a number quoted in a reason is printed in scientific notation when its plain text would exceed 40 characters (`sublist: invalid position value: 9E+6144`). A null reason, and so each `diagnostics.messages` entry, holds at most 1,000 characters: a longer one keeps its first 999 and ends with `…`.
- `internal`: a caught Rust panic (only possible on native builds; in wasm a panic traps, see §1) or a serialization failure.

## 4. JS runtime API (`packages/feel-runtime`, TypeScript, ESM)

```ts
export type Dialect = 'expression' | 'unaryTests';
/** Raw JSON text, spliced verbatim into the request JSON (keeps exact number digits). Only values made by rawJson() count (private Symbol.for brand). */
export interface RawJson { readonly rawJson: string; }
export interface RequestOptions { signal?: AbortSignal; }
export interface EvaluateRequest { dialect?: Dialect; expression: string; context?: Record<string, unknown> | string | RawJson; contextFormat?: 'json' | 'feel'; input?: unknown | RawJson; tzOffsetMinutes?: number; language?: 'feel' | 'b-feel'; errorMode?: 'lenient' | 'strict'; maxSteps?: number; }
export interface FeelResult { display: string; feel: string; type: string; typed: unknown; isNull: boolean; nullReason?: string | null; }
export type EvaluateResponse =
  | { ok: true; result: FeelResult; warnings: { message: string }[]; diagnostics?: { count: number; messages: string[] }; elapsedMicros: number }
  | { ok: false; error: { kind: 'request'|'syntax'|'context'|'evaluation'|'internal'|'timeout'|'crash'|'cancelled'; message: string } };
export interface Evaluation { id?: string | number; dialect?: Dialect; expression: string; input?: unknown | RawJson; errorMode?: 'lenient' | 'strict'; maxSteps?: number; }
export interface EvaluateAllRequest { context?: Record<string, unknown> | string | RawJson; contextFormat?: 'json' | 'feel'; tzOffsetMinutes?: number; language?: 'feel' | 'b-feel'; evaluations: Evaluation[]; }
export type EvaluateAllResponse =
  | { ok: true; results: (EvaluateResponse & { id?: string | number })[]; contextWarnings: { message: string }[]; elapsedMicros: number }
  | { ok: false; error: { kind: string; message: string } };
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
  evaluateAll(req: EvaluateAllRequest, opts?: RequestOptions): Promise<EvaluateAllResponse>;  // §2 `evaluateAll`: one request, the context sent once
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
- **`evaluateAll`** is one request: it takes one place in the queue, `timeoutMs` and the `signal` are for all of its evaluations, and a timeout, a crash or a cancellation has no `results`. A `RawJson` value is spliced as the `context` and as the `input` of an entry. An entry that the runtime can not send (invalid `rawJson` input, an unpaired surrogate) is a failure in its place in `results`, with its `id`, like an entry that the module refuses; the other entries are sent. `instantiateFeel(...).call({ op: 'evaluateAll', ... })` does the same.
- WASI shim: no environment variables, no preopens, `fd_write` to fd 1/2 forwarded line-by-line to `console.log`/`console.error` (or `onOutput`); the last stderr lines (e.g. a Rust panic message) are appended to `crash` messages. `proc_exit` throws (→ crash).
- A `crash` in `FeelEngine` terminates the worker and starts a fresh one (same compiled module). `instantiateFeel.call` re-instantiates synchronously.
- `instantiateFeel` is for **trusted input only**: it runs on the calling thread and nothing can interrupt it, so a runaway expression blocks that thread (e.g. a server's event loop). Untrusted input belongs in `FeelEngine` (or a Wasmtime host with fuel/epoch interruption). It requires the `_initialize` export (rejects otherwise) and re-creates its instance after a call that left memory above `env.recycleMemoryBytes` (default 256 MiB).
- Errors never reject the returned promises (except `FeelEngine.create`, which rejects if the wasm cannot be fetched/compiled/instantiated).

## 5. Repository layout

```
Cargo.toml               one workspace: the engine crates (repository root) + wasm/feel-wasm; the `wasm` build profile
<dsntk crates>/          common, feel, feel-parser, feel-evaluator, feel-number, feel-temporal, feel-regex, model, model-evaluator, dsntk (CLI), … unchanged layout of upstream dsntk-rs
scripts/bootstrap.sh     install the wasm32-wasip1 target, download wasmtime into .tools/ (gitignored)
scripts/env.sh           source to set wasmtime as the cargo runner for `cargo test --target wasm32-wasip1`
scripts/build-wasm.sh    build wasm/feel-wasm → dist/feel.wasm (+ size and interface report)
scripts/stack-headroom.mjs  check that dist/feel.wasm evaluates constructs nested to the AST depth limit and a nested recursion in Node and Wasmtime (--report: the depths reached)
wasm/feel-wasm         Rust core implementing §1–3
wasm/feel-wasm/tests/corpus  requests that must never trap the module (checked natively and under Wasmtime): no-trap.json, single requests; edge-operands.json, expression templates over edge operands
packages/feel-runtime    JS/TS runtime implementing §4
apps/playground          Vite + Svelte 5 playground using feel-runtime
package.json             npm workspaces (packages/*, apps/*); scripts: bootstrap, build:wasm, build, dev, preview, test
```

- The playground imports `dist/feel.wasm` with `?url` (`apps/playground/src/engine.ts`), so `vite build` emits it as a hashed asset next to the bundle; the build fails early if `npm run build:wasm` has not run. The site is built with `base: './'` and works from any static host or sub-path.
- The fork's "wasm32 support" commit makes `FeelNumber → usize/isize` conversions that do not fit in 32 bits return a conversion error instead of panicking. Its "robustness" commit makes the position-taking built-ins (`sublist`, `substring`, `insert before`, `remove`, `list replace`) and list filters convert through 64-bit integers with wide arithmetic, so wasm32 returns the same results and null reasons as 64-bit native builds and no length can wrap. Covered by `positions_beyond_32_bits_are_null_not_panic` and `huge_positions_and_lengths_are_null_with_native_reasons`. The whole seconds of a days and time duration are an `i64` on every target too, so a zone offset of 2^32 seconds or more is out of range on wasm32 as natively (`time_offset_beyond_32_bits_is_null`).
- `rust-toolchain.toml` pins rustc 1.95.0; `scripts/bootstrap.sh` verifies the sha256 sum of the wasmtime download. The engine crates are part of this repository (see README "Relationship to dsntk" and `docs/FORK.md`).
