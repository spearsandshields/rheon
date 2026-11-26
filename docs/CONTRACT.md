# rheon FEEL wasm contract

This is the single source of truth shared by the Rust core (`wasm/feel-wasm`), the JS runtime (`packages/feel-runtime`) and the playground (`apps/playground`). Change it here first.

## 1. Wasm module (`dist/feel.wasm`)

- Target: `wasm32-wasip1`, built as a **reactor** (`cdylib` linked with Rust's self-contained `crt1-reactor.o`, see `wasm/feel-wasm/build.rs`). `_initialize` **is** exported: the host calls it exactly once after instantiation (a second call traps), e.g. `wasi.initialize(instance)` or `instance.exports._initialize()`.
- Shadow stack: 10 MiB (stack-first layout). The parser is recursive, and the wasm frames also use the host's native stack (in a browser the JS engine's; a worker's is smaller than Node's main thread's). The parser stops at 640 expressions open at once, the core rejects ASTs deeper than 300 levels and stops evaluations with more than 200 nested function calls or a value nested more than 500 levels deep (§2, §3). The evaluator runs FEEL function calls, loops and nesting on the heap, so a FEEL recursion takes no native stack per call: every construct nested up to the AST limit, and a FEEL recursion of 200 nested calls whose function body is nested up to the AST limit, evaluate on each host measured here, Node's main thread and Wasmtime's default 512 KiB wasm stack included. A function that the engine calls back (the comparison of `sort`, the function of `list replace`) runs as a call of its own and takes native stack for each level of a recursion through it; a host gives the module the stack below for that. Running out of a stack is a trap ("crash": discard the instance). Measured with `npm run stack -- --report` (macOS arm64, Node 26, Wasmtime 49.0.3): the recursion of 200 calls reaches the AST limit (a body nested 289 levels) on Node's main thread, in Node workers with 2, 3 and 4 MiB stacks (Liftoff and TurboFan) and in Wasmtime with a wasm stack of 512 KiB to 16 MiB, and so does a recursion of 66 levels through a `sort` comparison (199 nested calls, the comparison's body nested 284 levels).
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
  "op": "evaluate" | "evaluateAll" | "evaluateEach" | "explain" | "parse" | "syntax" | "complete" | "hover" | "paths" | "outline" | "format" | "info",
  "dialect": "expression" | "unaryTests" | "simpleExpression" | "simpleExpressions" | "textualExpressions",   // default "expression"
  "expression": "string",                    // evaluate/parse
  "context": { /* JSON object */ },          // evaluate; optional, default {}
  "contextFormat": "json" | "feel",          // optional, default "json"; with "feel", `context` is a STRING holding a FEEL context literal
  "input": <JSON value>,                      // unaryTests only: the value tested (`?`). If absent, context["?"] is used
  "tzOffsetMinutes": -240,                    // optional: host local UTC offset in minutes, used by now()/today()
  "now": "2026-01-01T10:00:00Z",              // optional: the instant now()/today() read instead of the clock
  "language": "feel" | "b-feel",             // evaluate; default "feel"; "b-feel" is DMN 1.6 chapter 11
  "errorMode": "lenient" | "strict",         // evaluate; default "lenient" (DMN 1.6 §7.3.8)
  "maxSteps": 1000000                        // evaluate; optional: a budget of steps, a positive integer; no default
}
```

- A field that a request does not have (a misspelt `errorMode`, say) is not read, and the request is answered as without it. An evaluation says so in a warning that names the field: in `warnings` for `evaluate`, in `contextWarnings` for the fields of an `evaluateAll` request, and in the `warnings` of an entry for the fields of that entry (for `evaluateEach`, also for the fields of the request, which has no warnings of its own). An `evaluateAll` or `evaluateEach` request and their entries are read for the fields each has: an `evaluateAll` request `op`, `context`, `contextFormat`, `tzOffsetMinutes`, `now`, `language` and `evaluations`, an entry of `evaluations` `id`, `dialect`, `expression`, `input`, `errorMode` and `maxSteps`; an `evaluateEach` request `op`, `dialect`, `expression`, `contextFormat`, `tzOffsetMinutes`, `now`, `language`, `errorMode`, `maxSteps` and `entries`, an entry of `entries` `id`, `context` and `input`. A field written in the wrong place (`maxSteps` in an entry of `entries`, `errorMode` in an `evaluateAll` request) is not read there, and the warning says which fields that place has. The other operations have no warnings and say nothing.
- `language: "b-feel"` evaluates in B-FEEL (DMN 1.6 chapter 11): null only means missing data; where FEEL returns null for an error, B-FEEL returns `false` (boolean operators and functions; `true` for `!=`), the default of the function's result type (`0`, `""`, the epoch, `P0M`, `[]`, `(0..0)`), or converts mismatched arithmetic operands (`"The result is: " + 1` = `"The result is: 1"`, `1 + null` = `1`). The FEEL error behind such a result is listed among the `warnings`, not among the `diagnostics` errors: B-FEEL defines the result as normal (chapter 11, 11.1), so it is not counted and `errorMode: "strict"` accepts it. These are the errors of the operation whose answer B-FEEL defines: a comparison, `between` and `in` (with the tests of a list of tests), the arithmetic operator, a built-in call, and a domain of `some` or `every` that is no iteration context; an error of an operand that B-FEEL does not replace stays an error.
- `errorMode: "strict"`: when any error occurred during the evaluation (a null created with an error reason, see `diagnostics`; in B-FEEL not the errors behind its defaults, which are warnings), the response is an `evaluation` error whose message is `strict error mode: <first error>`, instead of a result. `"lenient"` returns the result and lists the errors.
- `maxSteps`: a budget for one evaluation. Every function call (of a user-defined, built-in or model function) and every item that a `for`, `some`, `every` or a filter visits is a step. The step after the last one of the budget stops the evaluation, and the response is an `evaluation` error with the message `evaluation took more than <maxSteps> steps; evaluation stopped`. Without the field an evaluation has no budget of steps; the limits of §1 still bound it. A value that is not a positive integer is a `request` error.

- `dialect` names the start rule of the grammar that the text is read as (DMN 1.7 Beta 1, 10.3.1.2 and 9.2), in every op that takes an expression:
  - `"expression"` (default): an expression; `"unaryTests"`: unary tests of the value `input`.
  - `"simpleExpression"`: a simple expression of S-FEEL (clause 9.2, rule 3), what the decision tables and invocations of conformance level 2 hold (clause 9.5): names and qualified names, numbers without exponent, strings, booleans, `date`, `time`, `duration`, `days and time duration` and `years and months duration` of one string literal, arithmetic and comparisons. Any other construct (parentheses, `null`, `@` literals, `if`, `for`, `and`, functions, lists, contexts, ...) is a `syntax` error that names it and its place, and the editor ops show it there. The text means what it means in FEEL (clause 9.4).
  - `"simpleExpressions"`: simple expressions separated by `,` (rule 4); `"textualExpressions"`: textual expressions separated by `,` (grammar rule 3; a list, context or function definition alone is no textual expression, in parentheses it is). Both evaluate to the list of the values, one item per expression.
- `parse` also accepts `context`/`contextFormat` (optional): the parser needs to know which multi-word names exist (`Grace's age + 1` only parses as a name if `Grace's age` is in the context). For `dialect: "unaryTests"`, `?` is always a known name.
- Unary tests are evaluated as dsntk's decision tables do: the tested value is bound to the name `?` in the scope and the result is `? in (<unary tests>)`: the `or` of the tests (DMN 1.6 Table 56). It is `true` when one test is true, `false` when every test is false, and null with the reason when no test is true and one has no answer, because the tested value is of another kind than the test (`"a"` tested by `< 10` or by `1, 2`). `not(<tests>)` is the negation, so it is null where the tests are. A decision table matches on `true` (clause 8.3.3), so null is no match. A test that is a boolean expression using `?` (a comparison, `and`, `or`, `between`, `in`, `instance of` or `not(...)`), e.g. `? > 5`, is satisfied when it is true (clause 7.3.2 d); any other test is compared with the tested value.
- If neither `input` nor `context["?"]` is given for unary tests, `?` is null and a warning is returned.
- The endpoint of a unary comparison is an expression and binds like the right side of a comparison: `> 0.1 + 0.2` tests `? > (0.1 + 0.2)` and `>= x + 1` tests `? >= (x + 1)`; `and`, `or`, a comparison and `between` end the endpoint (`<= 10 = (null..10]` compares the unary comparison with a range). `< d.year` compares with the year of `d` (DMN 1.6 §10.3.2.7, "Endpoints can be expressions"); a property of the unary value itself is read as `(< 10).end`.
- `tzOffsetMinutes`: integer in `-1080..=1080` (±18:00), otherwise a `request` error (every other integer, including `i64::MIN`). It is a fixed offset (no DST rules), applied to the "local" (zone-less) date-time computations of that request that need the host's offset — `now()`, `today()`, and the subtraction of a local and a zoned date-time — via a process-global override in rheon-feel-temporal (the fork's "local offset override" commit), reset after every request. Without it, local time is **UTC on every target**: chrono's `Local` is hard-wired to UTC on `wasm32-wasip1`, and native builds apply offset 0 too instead of the host zone, so results never depend on where the core runs. Comparisons of a local and a zoned value do not use it: the local one takes the offset of the other (DMN 1.7 Beta 1, Table 55).
- `now`: an ISO 8601 instant with its offset or `Z` (RFC 3339: `"2026-01-01T10:00:00Z"`, `"2026-01-01T12:00:00.5+02:00"`), otherwise a `request` error (a date, a local date and time, any other text or value). It pins the clock that `now()` and `today()` read for that request, seen at its `tzOffsetMinutes`, so an evaluation that uses them is repeatable (DMN 1.7 Beta 1, 10.3.4.11, which encourages a fixed clock for tests); without it they read the system clock. Requests that take `tzOffsetMinutes` take `now`, for all their evaluations; it never outlives its request.
- Limits: an expression (or FEEL context literal) whose AST is deeper than **300** levels is rejected (`syntax`, resp. `context`) before any recursive code runs after the parser (`expression is nested too deeply (N levels, the limit is 300)`). The parser itself is recursive and stops a text with more than **640** expressions open at once, parentheses included (`expression is nested too deeply (more than 640 levels of nesting)`): enough for every text whose AST is at most 300 levels deep, since an operand in parentheses opens two expressions for one level; an evaluation with more than **200** nested function calls (recursion) is stopped and reported as an `evaluation` error, never as a partial result; a call of a user function, of a built-in, and the call that `sort` and `list replace` make of the function they are given each count. So is an evaluation that produces a value nested more than **500** levels deep (`values are nested too deeply (more than 500 levels); evaluation stopped`): a scalar is 0 levels deep, a list or a context is one level deeper than the deepest value it holds (so an empty one is 1 level deep), and a function is one level deeper than its closure context; a value 500 levels deep is passed, bound and returned like any other. A loop that would deepen its results further, and `context put` with a key list that would set a value deeper than the limit, stop before they build the deeper value. A key list builds no context (DMN 1.7 Beta 1, 10.3.4.10): the first key of the path that the context does not have gets null, `context put({x: 1}, ["y", "a"], 2)` is `{x: 1, y: null}`; a path blocked by an entry that is not a context is null, whatever its length. So is an evaluation in which `matches`, `replace` or `split` meets a valid pattern that exceeds a limit of the regex engine (its backtracking budget or stack, the compiled size, the nesting depth): `matches: the pattern '^(a|a)*\1$' exceeds the regex engine's backtracking limit; evaluation stopped`. An invalid pattern is not a limit: its result is null (DMN 1.6 Table 75, note 2). So are invalid flags: a character other than `s`, `m`, `i`, `x` and `q` (XQuery 1.0 and XPath 2.0 F&O §7.6.1.1; `q` is the flag of F&O 3.1, kept as an extension), or flags that are not a string; `""` and null are no flags. `replace` is null for a replacement string that F&O §7.6.3 rejects: a `$` not followed by a digit and not escaped as `\$`, or a `\` that is not part of `\\` or `\$`; with `q` the replacement is used as is. `replace` and `split` are null for a pattern that matches the zero-length string (`""`, `x*`, `^`, `\s*`; F&O §7.6.3 and §7.6.4), and `split` of `""` is `[]`.
- Shared values: a list or a context is shared, not copied, when it is bound to a name, passed to a function, read from the input context, iterated, filtered or put into another list or context; a change (`append`, `context put`, a filter's result) builds a new value and leaves the shared one as it was. So a value can hold the same list or context on many paths (`(for i in 1..30 return if i = 1 then [1] else [partial[-1], partial[-1]])[-1]` holds 2²⁹ paths to one list) while its memory grows with its distinct lists and contexts only. The operations that compare or test values visit each shared list or context once per pair or per call: `=`, `!=`, `in`, `instance of`, `distinct values`, `index of`, `list contains`, `union`, `intersection`, and the parser's names of an input context take time linear in the distinct parts of the values (equality at most quadratic, one step per pair of distinct shared lists or contexts); `count` reads the length. An operation that writes the value out follows every path, so its output is exponential in the doubling: the result display, `string()`, `flatten` and `...` of a value doubled 30 times end at the 512 MiB memory cap (a trap, §1); there is no limit on the size of an output.
- Functions: a FEEL function is a lexical closure (DMN 1.6 §10.3.2.13.2). Its body reads its parameters, the names in scope where the function is written, the input context and the built-in functions, and nothing of the expression that invokes it: `{f: function() a, r: {a: 1, s: f()}.s}.r` is null. A name is read from the nearest scope around the function that has it (§10.3.2.11: the contexts of a scope are looked through from the first to the last): a parameter, a variable of a loop or the item of a filter that the function is written in before an entry of a context literal further out (`{x: "s", f: function(x) function() x, r: f(1)()}.r` is 1), and an entry of a context literal before a name further out. A path is read of that nearest name too: when its value has not the entry, the path is null, and a name further out that has the entry is not read (`{p: {c: 1}, r: (function(p) (function() p.c)())(7)}.r` is null, as it is without the inner function). Of a context literal it is written in, a function sees every entry (§10.3.2.11.1), also one that comes after it: the functions of one context call each other in any order and themselves (`{a: function(x) b(x) + 1, b: function(x) x * 2, r: a(1)}.r` is 3), and `{f: function() k, r: f(), k: 10}.r` is 10, the entries being evaluated in the order they need each other. A function that a function returns keeps what it saw where it was written: `{f: function(x) {x: 10, g: function(y) x + y}.g, h: f(1), r: h(5)}.r` is 15.

How JSON input becomes FEEL values (`contextFormat: "json"`):
- JSON number → FEEL number, parsed from the **exact source text** (serde_json `arbitrary_precision`). It never passes through f64.
- string → string, boolean → boolean, null → null, array → list, object → context. Keys become FEEL names, which may contain spaces or `'`, e.g. `"Grace's age"`. Leading/trailing whitespace of a key is trimmed (`" x "` is `x`); inner whitespace is kept as is in the key, which comes back as it was sent, and does not matter for reading it: a name is a list of tokens (DMN 1.6 clause 10.3.1.4), so the key `"Date - Time"` is read by `Date - Time`, `Date-Time` and `Date  -  Time`, and `"a  b"` by `a b`. Of two keys that are one name the key of the same text as the name is read, else the first in the order of the keys. `get value(m, "<key>")` reads the key of that text, and else the key that is that name.
- A context keeps the order its entries were written in: the keys of a JSON object in the order of the request text (a key written twice keeps its first place and has its last value), the entries of a context literal as they are written, whatever order they are evaluated in. A result shows them in that order (`display`, `feel`, `typed.components`, `get entries`); an entry that a built-in function adds comes last, one that it replaces keeps its place. Equality and the type of a context do not depend on the order.
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
  "warnings": [ { "code": "string", "message": "string" } ],  // the codes: below
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

Scale rounding (`round up/down/half up/half down`, `floor`, `ceiling` with a scale) rounds the number once, by the function's mode, to a multiple of 10^-scale (DMN 1.6 Table 77): every scale of -6111..6176 is valid for every number, a zero or a tiny one included (`round half up(0, 2)` = `0`, `round up(1e-6000, -6000)` = 1 followed by 6,000 zeros), a number with no digit below the scale is itself (`ceiling(0.25, 6145)` = `0.25`), and `round half down` goes to the nearest neighbour and only a tie toward zero (`round half down(2.56)` = `3`). A scale outside that range is null. Date-times and times whose zones carry an offset compare as instants for `=`, `<`, `<=`, `>`, `>=` and ranges; a local (zone-less) value takes the offset of the other operand (DMN 1.7 Beta 1, Table 55), so `@"10:00:00" = @"10:00:00+02:00"` is true. `=` compares whole seconds and the order compares the fraction of a second too (TCK 0068 time_005). `time + days and time duration` wraps around midnight.

`op: "evaluateAll"` evaluates several expressions over one context, which is read once:

```jsonc
// request
{
  "op": "evaluateAll",
  "context": { /* as for evaluate */ }, "contextFormat": "json", "tzOffsetMinutes": 0, "now": "2026-01-01T10:00:00Z", "language": "feel",   // for all evaluations
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

`op: "evaluateEach"` evaluates one expression over several contexts:

```jsonc
// request
{
  "op": "evaluateEach",
  "expression": "string", "dialect": "expression", "language": "feel", "errorMode": "lenient", "maxSteps": 1000000,   // for all entries
  "contextFormat": "json", "tzOffsetMinutes": 0, "now": "2026-01-01T10:00:00Z",
  "entries": [                                  // required; each entry: what differs between the evaluations
    { "id": 1,                                  // optional, a string or a number: repeated in the answer of the entry
      "context": { /* as for evaluate */ },     // optional
      "input": <JSON value> }                   // unary tests only: the tested value
  ]
}
// response
{ "ok": true, "results": [ /* one answer per entry of `entries`, in their order */ ], "elapsedMicros": 123 }
```

- Each answer is the response that `op: "evaluate"` gives for the expression with the `context` and the `input` of that entry and the other fields of the request: a success or a failure, with the `id` of the entry when it has one.
- The expression is parsed in the context of an entry, and that parse serves the entries after it whose contexts have the same names: the same keys, also in the contexts that are values of entries, down to 8 levels. For a context with other names it is parsed again, in that context, because the names in scope decide how a text reads (`unit price` is one name or two). A context that holds a list, a function, a range or a type is always parsed in. So the answers never depend on the other entries; only the time does.
- An entry that fails (a context that does not convert, a strict-mode error, its step budget, an entry that is no object) is a failure in its place, and the other entries are answered. An expression with a syntax error is a `syntax` failure in every place. A failure of the request itself (malformed JSON, `entries` missing or no array, `expression` missing, an invalid `dialect`, `language`, `errorMode`, `maxSteps`, `contextFormat` or `tzOffsetMinutes`) is the failure response below, without results.
- `contextFormat` is that of the request for the contexts of all entries. `maxSteps` bounds the work of each entry. The module has no limit on the number of entries. Nothing of the request is kept after the response. A trap (§1) ends the whole request.

`op: "parse"` success: `{ "ok": true, "tree": "<dsntk AstNode::tree() ascii, no color>", "elapsedMicros": n }`

### The editor operations

`syntax`, `complete` and `outline` serve an editor. `syntax` and `complete` answer for every
text: a text that is no expression has its errors in the answer, and a failure is only a bad
request or a context that does not convert. Places are offsets in UTF-16 code units (the unit
of JavaScript and .NET strings), `from` inclusive and `to` exclusive. They take `expression`,
`dialect`, and for the names in scope either `context`/`contextFormat` as `parse` does or
`outline` (below), not both. The module keeps nothing between calls.

`op: "syntax"`, with the optional field `"tree": true`:

```jsonc
{ "ok": true,
  "tokens": [["keyword", 0, 2], ["whitespace", 2, 3], ["name", 3, 17], …],   // [class, from, to]
  "diagnostics": [{ "code": "expected-expression", "message": "Expected an expression after `then`, but the expression ends here.", "from": 26, "to": 26 }],
  "truncated": false,
  "nodes": [["If", 0, 26], ["Comparison", 3, 21], …],                         // only with "tree": true: [kind, from, to]
  "elapsedMicros": n }
```

- The tokens are the whole text in order, white space and comments included. The classes are
  `whitespace`, `comment`, `keyword`, `name`, `number`, `string`, `constant` (`true`, `false`,
  `null`), `operator`, `punctuation`, `function` (the name of a date and time literal), `type`,
  `key` (of a context entry), `parameter`, `variable` (of `for`, `some`, `every`), `property`
  (the name after a dot) and `error` (a character that starts no token). A name is one token as
  the engine reads it, with its spaces. A new class is a change of this contract.
- `diagnostics` are the syntax errors in the order of their places, at most 100 (`truncated`
  says that the text has more), one per place. `code` is for programs and stable; `message` is
  for people and may change: it says what was expected and what stands there instead
  (``Expected `,` or `]` in the list, found `3`.``). The codes: `expected` (a certain token is missing), `expected-expression`,
  `expected-name`, `expected-type`, `unexpected` (a token that fits nowhere), `string-open`, `string-escape`,
  `comment-open`, `character`, `failed` (the parser gave up, as at its nesting limit; the rest of
  the text has tokens but no nodes). After an error the parser goes on as if the missing part
  were there. `diagnostics` is empty exactly when `parse` accepts the text.
- `nodes` are the parts of the expression, each before the nodes inside it; a consumer reads
  the nesting from the places. A missing part is a node `Error` of an empty place. The kinds
  are not a stable list yet.

`op: "syntax"` with `"check": true` also checks a text that parses, and says how grave each
diagnostic is:

```jsonc
{ "ok": true, "tokens": [ … ],
  "diagnostics": [
    { "code": "not-a-value", "severity": "warning", "from": 15, "to": 21,
      "message": "Expected one of the values of `loan.purpose` (\"REFI\", \"PURCHASE\"), found \"REFO\"." },
    { "code": "deprecated", "severity": "hint", "from": 26, "to": 40, "message": "`loan.old score` is deprecated: use score." } ],
  "checked": true, "truncated": false, "elapsedMicros": n }
```

- `severity` is `error`, `warning`, `information` or `hint`. A syntax error is an `error`. Without
  `check` an answer is what it was: its diagnostics are the syntax errors and have no `severity`.
- `checked`: whether the text was checked. A text with syntax errors is not, nor one whose
  analysis reached its limit; the answer then has the syntax errors only.
- What a check finds is certain from the text, the scope and the outline, never a guess, and none
  of it is an error: FEEL gives null for each.

  | code | severity | when |
  |---|---|---|
  | `unknown-name` | warning | a name that no construct of the text binds, the scope has no value of and no built-in function is. Only with a `context` or an `outline`: their names are all the names there are |
  | `unknown-entry` | warning | an entry after a dot of a context that has no such entry: a context of the scope (its entries are those the `context` or the `outline` has) or one that the text makes |
  | `deprecated` | hint | a name or an entry that an outline marks as deprecated is read; the reason is in the message |
  | `not-a-value` | warning | a literal compared (`=`, `!=`, `in`) with a name or an entry whose `values` an outline lists, and that is none of them; in unary tests, a test that is such a literal |
  | `always-null` | warning | an operator (`+ - * / **`, `< <= > >=`, `= !=`) whose operands are a number, a string or a boolean for certain (a literal, or a value of the scope of that type) of types it gives null for (`"a" + 1`, `1 < "a"`, `true = 1`). FEEL's answer: B-FEEL gives a default there |

- The diagnostics are in the order of their places.

`op: "complete"`, with the optional field `"cursor"` (a UTF-16 offset; default: the end of the text):

```jsonc
{ "ok": true,
  "to": 15,                              // the cursor
  "expects": "expression",               // what the grammar takes at the cursor
  "items": [{ "label": "Monthly Salary", "kind": "name", "from": 4, "type": null, "detail": null }],
  "truncated": false,
  "call": { … },                         // only inside the parentheses of a function that is known
  "elapsedMicros": n }
```

- An accepted item replaces the text from its `from` to `to` with its `label`. The items of one
  answer can have different `from`s: in `if a th` the lexer reads `a th` as one name, so names
  are offered for `a th` and keywords for `th`.
- `kind`: `value` (a value that an outline lists for the name that is compared here, below),
  `parameter` (a parameter of the invoked function that can be named here; it
  has `"insertText"`, the text to insert in place of the label: the name with its colon),
  `name` (a name of the context, or one the text declares before the cursor: an
  iteration variable, a parameter, `partial`, a key of a context being written, and in the
  predicate of a filter `item` and the entries of an item), `entry` (a member of the value
  before a dot: an entry of a context, a property of a date, a time, a duration or a range,
  and for a list the members of its items), `keyword`, `function` (a built-in function, or a
  name that an outline says is a function), `type`.
  The items are ordered by kind in this order and then by label; at most 200 (`truncated`).
  An item fits when it starts with what is typed, letters compared without case.
- `expects`: `expression`, `operator` (after a complete operand: `and`, `or`, `between`, `in`,
  `instance of`), `keyword:<word>` (a keyword the grammar needs here, such as `keyword:then`),
  `property` (after a dot), `type`, `name` (a new name is written here: nothing is offered),
  `parameter` (after a named parameter only a named one can follow), `comment` and `string`
  (the cursor is inside a comment, or inside a string that is not closed: nothing is offered).
- An item of a built-in function has its first signature as `detail`
  (`"substring(string, start position, length?): string"`), `"documentation"` (one line that says what
  the function gives) and the type of its result as `type`. A parameter has its type as `type`.
  A name and an entry have their type as `type` when the engine knows it, else null: the kind
  of the value without the entries of a context (`number`, `context`, `list<context>`,
  `range<date>`, `function`). The `detail` of these kinds is null.
- The value before a dot is any expression: the engine finds its type from the text and the
  context, without evaluating it (`for l in order.lines return l.` offers the entries of a
  line; `date(x).` offers `year`, `month`, …; `sort(lines, f)[1].` offers the entries of a
  line). A variable of an iteration has the type of an item of its domain, an entry of a
  context literal the type of its value, the result of a built-in function the type of its
  signature (for the functions that give back their list, such as `sort`, `append` and
  `reverse`, the type of that list). A parameter of a function written in the text, the result
  of a function of the host and a name without a value have no known type: nothing is offered
  after them. With an `outline` in place of the context the types of the names that are no
  contexts and no lists are known when the outline has them (below).
- `call` stands when the cursor is inside the parentheses of an invocation of a built-in
  function or of a function of an outline (the innermost one). The function of an outline is a
  name of the scope, or an entry at the end of a path (`lib.nz(`); a name after a dot is only
  ever that entry, not the built-in function of the same name:

  ```jsonc
  { "function": "substring",
    "documentation": "The characters from a position, as many as the length says or all that follow.",
    "argument": 1,                       // the index of the argument the cursor is in
    "named": [],                         // the names of the named parameters given before it
    "activeSignature": 0,                // the first signature that has a parameter for that argument, else 0
    "signatures": [{
      "label": "substring(string, start position, length?): string",
      "parameters": [{ "name": "string", "type": "string", "optional": false, "rest": false }, …],
      "result": "string",
      "named": true,                     // false: by position only (`append`, `min(1, 2, 3)`)
      "activeParameter": 1 }] }          // the parameter the argument binds to; absent when it binds to
                                         // none, or arguments are given by name
  ```

  A `rest` parameter stands for one or more arguments. The types are FEEL types as text, with
  `or` between alternatives (`date or date and time`). The names of the parameters are those
  the engine takes in an invocation with named parameters; where only named parameters were
  given so far, they are offered as items, without the ones already given and only of the
  signatures that have those.

`op: "outline"` takes `context`/`contextFormat` and answers `{ "ok": true, "outline": <outline>, "elapsedMicros": n }`.
An outline is `null` (a name without entries), `{"names": {<name>: <outline>, …}}` (a context)
or `{"items": <outline>}` (a list whose first item is a context). It holds the names of a
context without its values, so an editor builds it once and sends it with every keystroke
instead of the context; a consumer can also write one itself. A list is read by its first
item only.

A name that is no context can have its type in an outline: `{"type": "<FEEL type>"}`, the
type as FEEL writes it (`number`, `date`, `list<string>`, `range<date>`). With such an outline
`complete` shows the types and offers the properties of a date or a range as it does with the
context. The type is a statement of the consumer: the engine takes it as it is, also for a
name whose value would be null or an empty list. A text that is no FEEL type is a failure of
kind `context` that names the entry (`outline.order.total: expected a FEEL type (such as
number, date, list<string> or range<date>), found "numbr"`). `op: "outline"` writes the types
when the request has `"types": true` (default false: the answer is as without the option):
the type of each name that is no context and no list of contexts, when the value says it (not
for null; an empty list is a `list<Any>`).

An outline says as much as the consumer has to say; every part of the following is optional,
and an outline without it is read as before.

- **Types by name.** The outline of a request can have `"types": {<name>: <outline>, …}`, and
  a type text can name one of them, alone or inside another type (`"Customer"`,
  `"list<Loan>"`). The types can name each other and themselves (a loan has a customer, who
  has loans): the engine follows a name where the text reads it, so such an outline is as
  small as its types, however deep a text goes. `complete` shows a name of such a type with
  the name of the type. A name of `types` that is a type of FEEL (`number`, `list`), a type
  text that names no type, and types that are only names of each other are failures of kind
  `context` that name the entry.
- **What a name is for.** Beside `type`, `names`, `items` or `function`, an outline can have
  `"description": "<text>"` and `"deprecated": true | "<the reason>"`. `complete` gives the
  description as the `documentation` of the item, and `deprecated` as the outline has it; the
  deprecated items of a kind come after the others.
- **The values a name can have.** Beside a `type` that is a string, a number or a boolean
  (also a type of `types` that is one, and in its definition there, for every name of it):
  `"values": [<value> | {"value": <value>, "description": "<text>"}, …]`. Where the name is
  compared (`=`, `!=`, `in`), `complete` offers the values first, as items of the kind
  `value`: after the operator as they are written (`"REFI"`, `12`), and inside a string that
  is being written as its text (`loan.purpose = "RE` offers `REFI`). In the unary tests
  dialect the name `?` of the outline is the input: its values are offered where a test
  starts. The values are for the editor; an evaluation does not read them.
- **Functions.** `{"function": {"parameters": [{"name", "type", "description"}, …], "result":
  "<type>"}}`: a name that is a function, with the names of its parameters (a type and a
  description for each when they are known). `complete` offers it as a `function` with its
  signature as `detail`, and `call` hints its invocation as that of a built-in function, a
  parameter with its description as `documentation`. `op: "outline"` with `"types": true`
  writes this form for a function of a FEEL-format context: the names of its parameters and
  the types it declares.

```jsonc
{ "types": {
    "Loan":     { "names": { "amount": { "type": "number" },
                             "purpose": { "type": "string", "description": "Loan purpose" },
                             "customer": { "type": "Customer" } } },
    "Customer": { "names": { "loans": { "type": "list<Loan>" },
                             "old score": { "type": "number", "deprecated": "use score" } } } },
  "names": { "loan": { "type": "Loan" } } }
```

A field that an outline does not have is a failure of kind `context` that names the entry and
the field. The key of a name is read as the key of a context is (a key in another form than
FEEL reads a name, `"Net - Price"`, is the name its tokens are).

`op: "hover"` takes `expression`, `dialect`, the names in scope as `syntax` does, and `"at"`,
a place of the text. It answers what the name at that place is, or `"hover": null` where no
name stands:

```jsonc
{ "ok": true,
  "hover": { "from": 74, "to": 81,             // the place of the name
             "kind": "entry",                   // name | entry | function | variable | parameter | key
             "type": "string",                  // as an item of `complete` shows it, null when not known
             "documentation": "Loan purpose",   // of an outline, or of a built-in function
             "deprecated": "use kind",          // as the outline has it
             "signatures": [ … ] },             // of a function, as in `call`
  "elapsedMicros": n }
```

The name is typed as `complete` types the text up to its end, so a text with errors after
it is answered too. The answer is in parts, not a prepared text: the consumer shows it.

`op: "paths"` takes `expression`, `dialect` and the names in scope as `syntax` does, and
answers what the expression reads of those names, without evaluating it:

```jsonc
{ "ok": true,
  "paths": [
    { "path": ["net operating income"], "text": "net operating income", "whole": true,  "places": [[0, 20]] },
    { "path": ["loans"],                "text": "loans",                "whole": false, "places": [[27, 32]] },
    { "path": ["loans", [], "payment"], "text": "loans[].payment",      "whole": true,  "places": [[27, 59]] },
    { "path": ["loans", [], "status"],  "text": "loans[].status",       "whole": true,  "places": [[33, 39]] } ],
  "unknown": [],
  "complete": true,
  "elapsedMicros": n }
```

for `net operating income / sum(loans[status = "active"].payment)` with an outline that says
`loans` is a list of contexts with a `status` (without one, `status` is listed too: see below).

- A path starts at a name in scope. A segment is the name of an entry, or `[]` (an empty
  array) for the items of a list. A name is written as the context or the outline writes its
  key (without white space at its ends); `text` is the path for people.
- `whole: true`: the value at the path is read as a value (an operand, an argument of a
  function, the result), so all of it can be read, however deep it is. `whole: false`: only
  what the paths below it read, and that it is there.
- **Exact or wider, never narrower.** An evaluation with a context that holds only what the
  paths list gives what the evaluation with the whole context gives. To cut a context down:
  a name that no path starts with is left out; a value at a `whole` path is kept; of a context
  at another path the entries that paths name are kept, of a list its items, each cut down the
  same way (an item that no path lists becomes an empty context or null, so that the list
  keeps its length); a simple value at a listed path is kept as it is.
- What passes a value on without reading into it: a path, a filter, the variable of `for`,
  `some` and `every`, an entry of a context literal, the branches of `if`, an item of a list
  literal, the body of `for`, a parameter of a function that the text defines and invokes,
  `get value` with a key that is a string literal, and the built-in functions `reverse`,
  `sublist`, `remove`, `append`, `concatenate` and `insert before` with positional arguments.
  Everything else reads its operands whole.
- A name in the predicate of a filter is an entry of the item when the item has one, and it is
  read outside the filter too (the predicate is evaluated there first: a number selects an
  item). With a `context` or an `outline` the item is known to have the entry or not; without
  one, both are listed (`loans[].status` and `status`).
- Whether the items of a list have an entry is read from the items of the `context`, all of
  them: a name of a predicate is listed as an entry of the item when an item has it (the items
  of a list need not be alike). An `outline` says the type of every item.
- A function that a `context` holds (a context in FEEL format) is not in the text: when the
  expression reads it, every name of the scope is listed as read whole. A function that the
  text defines is followed into its body; when it invokes itself, what it gives is read whole.
- Without a `context` or an `outline`, the name of a built-in function is that function, not
  a name for the host to give.
- Which values are lists is read from the `context` or the `outline`. Without one, an entry
  of a name is written as an entry (`loans.amount`) also when the value is a list at run
  time, and `[]` is written where the text takes items (a filter, a loop). Read such paths as
  FEEL reads values: the items of a value that is no list are the value, and an entry of a
  list is the entry of each of its items.
- `places`: where the text reads the path, as `[from, to]` in UTF-16 units, in the order of
  the text.
- `unknown`: the names that nothing gives (no construct of the text binds them, the scope has
  no value of them, they are no built-in functions), each with its `places`. Only with a
  `context` or an `outline`; without one, every such name is a path, a name for the host to give.
- `complete: false`: the analysis reached its limit (200,000 nodes, the bodies of the functions
  of the text counted at each invocation); the expression can then read more than `paths` lists.
- For `dialect: "unaryTests"` the tested value is the path `?`, read whole.
- A text with syntax errors is the failure of kind `syntax` with `diagnostics` that `evaluate`
  answers. The answer does not depend on the values of a `context`, only on their shape (the
  entries that its contexts have, and which values are lists).

`op: "explain"` evaluates an expression as `evaluate` does and says how its value came about.
It takes the fields of `evaluate` (an expression only: `dialect` is `"expression"`) and

```jsonc
"expand": [ [from, to], { "at": [from, to], "within": [ … ], "iterations": [from, to] }, … ],  // optional
"maxRecords": 200,                                                                             // optional, 1 to 10000
"reads": true                                                                                  // optional
```

and answers what `evaluate` answers (`result`, `warnings`, `diagnostics`) with

```jsonc
"trace": [
  { "from": 0, "to": 27, "kind": "Addition", "value": { … },      // a node of the text and its value
    "name": "…",                                                   // of an entry of a context literal
    "iteration": 1, "iterations": 3,                               // see the loops below
    "within": [ { "at": [4, 57], "iteration": 1 }, { "at": [61, 68], "call": true } ],
    "nullFrom": { "at": [0, 23], "within": [ … ] },
    "nullOrigin": true, "nullCode": "missing-entry",
    "reads": ["loans[].amount", "rate"] } ],
"traceTruncated": false
```

- A record is a node of the text (its place in UTF-16 units and its `kind`, those of the nodes
  of `syntax`) with the value that the evaluation gave it, as `result` has a value. A value
  whose text is long (more than 4096 bytes, or more than 256 items, entries and values) is `{ "summarized": true, "type": "list" | "context" |
  "<type>", "isNull": false, "count": n }` (`count` for a list or a context). The records are
  in the order of the text, a node before the nodes inside it.
- By default the trace has the root of the text and its parts one level down: the operands of
  an operator, the entries of a context literal, the items of a list, the arguments of an
  invocation, the list and the predicate of a filter, the domain and the body of a loop, the
  branches of an `if` (the one evaluated). `expand` names nodes, by their places, whose parts
  are wanted too; an expanded node has its own record as well. The place of a record
  (`from`, `to`) is the place to name, so a trace is walked down without a `syntax` request.
  The engine keeps nothing
  between requests: each request evaluates again and records what it asks for, so an answer is
  as long as the request asks and not as the data is.
- A part that is evaluated once for each item (the body of a `for`, `some` or `every`, the
  predicate of a filter) has a record for each iteration, with `iteration` (from 0): for the
  first 20, or for `"iterations": [from, to)` of the entry of its loop. The record of a loop
  or a filter has `iterations`, how many it had.
- The parts of an invocation are its arguments and, when it invokes a function written in the
  text, the body of that function. A function that a built-in function invokes (the order of
  `sort`) is not invoked by an invocation of the text: its body has no record.
- `within` says where a record's node is evaluated, outermost first: the iteration of each
  loop around it (`{"at": <the loop>, "iteration": n}`) and the invocation that runs the body
  it is in (`{"at": <the invocation>, "call": true}`). A node that is evaluated more than
  once is named in `expand` with the `within` of its record: `{"at": [from, to], "within":
  […]}`. So a loop is entered one iteration at a time and a function that invokes itself one
  invocation at a time. An entry whose `within` does not lead to its node is the error
  `invalid-field`, and the message says the steps that do.
- A record whose value is null says where the null is from. `nullFrom` is the part that is
  null, as an entry of `expand`: for an `if` the branch taken, for an invocation the body it
  ran (else its first null argument), for a loop its domain or its first null iteration, else
  the first part that is null; a name that an earlier entry of a context literal around it
  gives its value names that entry, and so does `{…}.name`. `nullOrigin: true` says that the
  null is the node's own: its parts are not null, or it has none (a name that the context
  gives null or does not have, an operator or a function whose operands are not null, `null`
  written). A null is followed without being asked: a node that a `nullFrom` names is
  expanded as if `expand` named it, and so on until a record has `nullOrigin`, so one request
  says the whole way from a null to where it is from, and the trace has the parts of the nodes
  on that way: through the bodies of the functions invoked, however many invoke one another,
  and into the first iteration of a loop whose value is null, whichever it is. This costs one
  more evaluation of the text, whatever the length of the way, and only when a null is from a
  node that the request does not expand; it is an evaluation that also keeps the values on
  the ways of its nulls, which takes about twice the time of the evaluation itself. Both are
  one evaluation for `maxSteps`: following takes steps of the request's budget, never more,
  and when the budget does not cover it the answer is that of the first evaluation, without
  the way. Nodes that would make the trace longer than `maxRecords` are not expanded, and the
  trace is not cut for them: the way ends where the room does. Only the null that `nullFrom`
  names is followed: another null part says where it is from when it is expanded. A null
  record has neither field when its node is not expanded (its parts have no records) and
  when the trace is cut. The way is
  not followed through a value: a null that is an item of a list or an entry of a context
  given by the host, and a name that a loop, a filter or a parameter binds, end it.
- With `nullOrigin` stands `nullCode`, why the null is the node's own: `missing-entry` (a name
  that nothing gives, an entry that its context does not have, a property that its value does
  not have), `null-input` (a name or an entry that is given null: by the context of the
  request, an argument, an item of a loop), `operator` (an operator or a filter whose operands
  are not null), `function` (an invocation whose arguments are not null, or that ran no body
  of the text), `other` (`null` written, a name in the predicate of a filter, which an item
  can have, and what is none of these). The codes are stable; the finer reason is the text of
  `nullReason` of the value and of the `diagnostics`.
- With `"reads": true` each record has `reads`: the names that the text of its node reads
  from around it, as paths (`debt.total`, `loans[].amount`; the notation of `paths`), in the
  order of the text. It says what the node can read, not what this evaluation read: both
  branches of an `if` are in it. A name that the node itself binds (the variable of its loop)
  is not; in a part of that loop the variable is a name read (`l.amount`).
- At most `maxRecords` records are kept (default 200); with more, `traceTruncated` is true
  and the evaluation and its `result` are as without the limit.
- A failure is the failure of `evaluate` for the same request (`syntax-error`, `limit-steps`,
  …). The records are read from the evaluation, which is the one of `evaluate`: the result,
  the warnings and the diagnostics are the same whatever is expanded.

`op: "format"` takes `expression`, `dialect`, the names in scope as `syntax` does, and the
optional field `"width"` (the most characters of a line, 1 to 1000; default 80). It answers
`{ "ok": true, "formatted": "<text>", "changed": true | false, "elapsedMicros": n }`.

- The layout depends only on the tokens of the text and the width, so formatting a formatted
  text changes nothing (`changed: false`). One space stands where the grammar reads better
  with one (around operators and keywords, after `,` and `:`), none inside brackets, around
  `.` and `..`, between a function and its `(`, or after a sign.
- A part that does not fit the rest of its line is broken by its kind, and what is inside it is
  indented by two spaces: a list, a context, the arguments of an invocation, a filter and an
  expression in parentheses get an item per line between their brackets; `then`, `else`,
  `return` and `satisfies` start a line; every operator of a chain of one kind (`and`, `or`,
  comparisons, `+ -`, `* /`, `**`) starts a line. A part that can not be broken (a name, a
  string, a type) may run past the width.
- A context of two or more entries always has every entry on a line of its own, whatever its
  length; a context of one entry stays on its line when it fits.
- Names, numbers, strings, comments and parentheses are written as they stand in the input; a
  comment that runs to the end of its line keeps the line break after it.
- A text with syntax errors is the failure that `parse` answers, with its `diagnostics`.
- The formatted text is checked before it is answered: it parses to the same tree as the
  input. A text for which that does not hold comes back as it is, with `"reason"` (a string):
  its spacing decides how its names are read (a name in scope with an operator in it), or it
  is nested too deeply to be compared.

`op: "info"` success: `{ "ok": true, "engine": "rheon", "engineCommit": "<40-hex repository commit>[-dirty]", "upstreamCommit": "<40-hex upstream dsntk-rs commit>", "numberBackend": "dashu", "version": "<rheon-feel-wasm crate version>" }`. `engineCommit` is the rheon repository commit the module was built from (`git rev-parse HEAD` at build time, `-dirty` appended when the work tree had uncommitted changes, `.cargo_vcs_info.json` in a crate packaged by cargo, `"unknown"` when built outside both without `RHEON_COMMIT` set), `upstreamCommit` the dsntk-rs commit the engine crates are based on (`[package.metadata.rheon]` in `wasm/feel-wasm/Cargo.toml`), and `numberBackend` the feel-number backend (`rheon_feel_number::NUMBER_BACKEND`, always `"dashu"`).

Failure (the instance stays healthy):

```jsonc
{ "ok": false, "error": { "kind": "request" | "syntax" | "context" | "evaluation" | "internal", "code": "string", "message": "string" } }
```

`kind` says which part failed, `code` which failure of that kind it is. A code is for a program:
it is one of the list below and stays what it is; a message is for people and can change. A new
code is a change of this contract.

| kind | code | when |
|---|---|---|
| `request` | `malformed-json` | the request is no JSON |
| | `not-utf8` | the request is no UTF-8 |
| | `missing-field` | a field that the op needs is not there (`op`, `expression`, `evaluations`, `entries`) |
| | `invalid-field` | a field has a value that is not allowed (also: the request, or an entry of it, is no object) |
| | `conflicting-fields` | two fields that exclude each other (`context` and `outline`) |
| | `unknown-op` | `op` is none of the ops |
| `syntax` | `syntax-error` | the text does not parse (with `diagnostics`) |
| | `expression-too-deep` | the text is nested too deeply |
| `context` | `invalid-context` | the context or the input is not what its format takes, or holds a value that has no FEEL value |
| | `context-too-deep` | the JSON of the context or the input is nested too deeply |
| | `invalid-outline` | an outline has a field it has not, or a type that is none |
| | `outline-too-deep` | an outline is nested too deeply |
| `evaluation` | `limit-steps` | more steps than `maxSteps` |
| | `limit-calls` | more than 200 nested function invocations |
| | `limit-depth` | a value nested more than 500 levels deep |
| | `limit-engine` | a limit of an engine below FEEL (a regular expression) |
| | `strict-error` | `errorMode: "strict"`, and the evaluation had an error |
| `internal` | `panic`, `serialization`, `internal` | a caught panic, a response that could not be written, another failure of the engine |

The codes of the warnings (`warnings[].code`, `contextWarnings[].code`):

| code | when |
|---|---|
| `unknown-field` | a field that the request, or an entry of it, does not have: it is not read |
| `number-rounded` | a number of more than 34 significant digits, or too small, was rounded (in the context, the input or the text) |
| `fraction-truncated` | fraction digits of seconds beyond nanoseconds were dropped |
| `date-zone-dropped` | a date string had a zone, which a date does not hold |
| `months-truncated` | a fraction of a month was dropped in the arithmetic of a years and months duration |
| `bfeel-default` | B-FEEL gave its default where FEEL has an error: the error is the message |
| `no-input` | unary tests without an input value: null is tested |

A null result has no code: `nullReason` is its text.

- `request`: malformed request JSON (including invalid UTF-8 and JSON strings with unpaired `\uD800`-style surrogate escapes, which the core can not represent) or unknown op. A request value quoted in the message is a preview when its JSON text is longer than 120 characters (`invalid 'dialect' "xxx…… (200 characters) (expected …)`).
- `syntax`: the FEEL expression or unary tests failed to parse, or are nested too deeply (§2). When they failed to parse, the error also holds `diagnostics`: where and what the errors of the text are, as `op: "syntax"` answers them (`[{"code", "message", "from", "to"}, …]`, places in UTF-16 code units). The message is dsntk's, without its `<ParserError>` prefix; dsntk gives no position. It quotes the input whole when it is at most 120 characters, otherwise as its first 120 characters, `…` and its length (`syntax error: 1+1+…+1+… (40002 characters)`).
- `context`: the input context is invalid (bad JSON shape, JSON nested too deeply, bad FEEL context literal, or a FEEL context nested too deeply, recursing too deeply, holding a value nested more than 500 levels deep or exceeding a regex engine limit).
- `evaluation`: evaluation was stopped by a limit (§2): more than 200 nested function calls, a value nested more than 500 levels deep, or a regex engine limit. FEEL usually yields null instead; that is a *success* with `isNull: true` and `nullReason`. Null reasons are meant to be readable (`time: invalid time literal '…'`, `log: argument must be a positive number`, `exp: result is not a finite number (decimal overflow)`); a value quoted in a reason is its text when that is at most 120 characters, otherwise a preview: its first 120 characters, `…` and its size when it has one (`equal err '[1, 2, 3, …, 32, … (1000 items)' =?= '1'`; sizes are `N items`, `N entries` and `N characters`); a number quoted in a reason is printed in scientific notation when its plain text would exceed 40 characters (`sublist: invalid position value: 9E+6144`). A null reason, and so each `diagnostics.messages` entry, holds at most 1,000 characters: a longer one keeps its first 999 and ends with `…`.
- `internal`: a caught Rust panic (only possible on native builds; in wasm a panic traps, see §1) or a serialization failure.

## 4. JS runtime API (`packages/feel-runtime`, TypeScript, ESM)

```ts
export type Dialect = 'expression' | 'unaryTests' | 'simpleExpression' | 'simpleExpressions' | 'textualExpressions';
/** Raw JSON text, spliced verbatim into the request JSON (keeps exact number digits). Only values made by rawJson() count (private Symbol.for brand). */
export interface RawJson { readonly rawJson: string; }
export interface RequestOptions { signal?: AbortSignal; }
export interface EvaluateRequest { dialect?: Dialect; expression: string; context?: Record<string, unknown> | string | RawJson; contextFormat?: 'json' | 'feel'; input?: unknown | RawJson; tzOffsetMinutes?: number; now?: string; language?: 'feel' | 'b-feel'; errorMode?: 'lenient' | 'strict'; maxSteps?: number; }
export interface FeelResult { display: string; feel: string; type: string; typed: unknown; isNull: boolean; nullReason?: string | null; }
export type EvaluateResponse =
  | { ok: true; result: FeelResult; warnings: { code: string; message: string }[]; diagnostics?: { count: number; messages: string[] }; elapsedMicros: number }
  | { ok: false; error: { kind: 'request'|'syntax'|'context'|'evaluation'|'internal'|'timeout'|'crash'|'cancelled'; message: string } };
export interface Evaluation { id?: string | number; dialect?: Dialect; expression: string; input?: unknown | RawJson; errorMode?: 'lenient' | 'strict'; maxSteps?: number; }
export interface EvaluateAllRequest { context?: Record<string, unknown> | string | RawJson; contextFormat?: 'json' | 'feel'; tzOffsetMinutes?: number; now?: string; language?: 'feel' | 'b-feel'; evaluations: Evaluation[]; }
export type EvaluateAllResponse =
  | { ok: true; results: (EvaluateResponse & { id?: string | number })[]; contextWarnings: { code: string; message: string }[]; elapsedMicros: number }
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
  evaluateEach(req: EvaluateEachRequest, opts?: RequestOptions): Promise<EvaluateEachResponse>;  // §2 `evaluateEach`: one request, the expression parsed once for contexts of the same names
  explain(req: ExplainRequest, opts?: RequestOptions): Promise<ExplainResponse>;  // §3 `explain`: the answer of `evaluate` with `trace` and `traceTruncated`; `ExplainRequest` is `EvaluateRequest` without `dialect` and `input`, with `expand` and `maxRecords`
  parse(dialect: Dialect, expression: string, opts?: ParseOptions & RequestOptions): Promise<ParseResponse>;
  // the editor operations (§3): `opts` names the dialect and either the context or its outline
  syntax(expression: string, opts?: EditorOptions & { tree?: boolean } & RequestOptions): Promise<SyntaxResponse>;
  complete(expression: string, cursor: number, opts?: EditorOptions & RequestOptions): Promise<CompleteResponse>;
  outline(opts?: ParseOptions & RequestOptions): Promise<OutlineResponse>;
  format(expression: string, opts?: EditorOptions & { width?: number } & RequestOptions): Promise<FormatResponse>;
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
- A field of a request object that the runtime does not know (a misspelt `errormode`) is sent with the value `null` where the runtime builds the request: `FeelEngine`'s `evaluate`, `evaluateAll` and `evaluateEach`, and `instantiateFeel().call` for `evaluateAll` and `evaluateEach`, with their entries. The module names it in a warning, as for a request sent as JSON text; its value is not sent (it may be large, or hold what JSON can not). `call` with any other request sends the object as it is, as JSON.
- **`evaluateEach`** is one request in the same way: one place in the queue, one timeout and one `signal` for all entries, and no `results` after a timeout, a crash or a cancellation. A `RawJson` value is spliced as the `context` and as the `input` of an entry, and an entry that the runtime can not send is a failure in its place. `contextFormat` is `'feel'` by default when the first context is a string.
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
