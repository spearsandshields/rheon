# The dsntk engine crates in rheon: changes against upstream

rheon is a fork of <https://github.com/dsntk/dsntk-rs> (the DecisionToolkit
FEEL/DMN engine, Copyright (c) 2015-2026 Dariusz Depta, MIT OR Apache-2.0).
rheon is a permanent fork: the engine is developed here and contributing
changes back to dsntk is not a goal. The dsntk crates sit at the repository
root in upstream's directory layout, so individual upstream fixes can still be
taken by cherry-pick (see "Taking upstream changes" below). This file lists
every engine change in the order it was made. Until 2026-10-28 these commits lived on a `wasm` branch of a local
clone, consumed as a git dependency by a separate `dsntk-wasm` repository; that
repository's content now lives here (`wasm/`, `packages/`, `apps/`, `docs/`,
`scripts/`) and the branch became `main`.

Rules:

- Engine changes are small, self-contained commits.
- `fastnum-backend` is an unpublished side branch with the experimental fastnum
  number backend (see below); it is not needed for wasm.
- The `rheon-*` crates are published on crates.io (`cargo publish --workspace`, see
  "Publishing" below). The four upstream crates rheon never changed (`dsntk-common`,
  `-macros`, `-feel-grammar`, `-recognizer`) left the tree on 2026-11-09 and are used from
  crates.io at dsntk 0.3.0, pinned `=0.3.0` in the root manifest; the dsntk crates with
  manifest-only changes stay in the tree with `publish = false`.
- Crate naming (2026-10-28): a crate whose source rheon has changed is
  renamed `rheon-*` (package name and Rust path; the directory keeps
  upstream's name). A crate carried as upstream ships it, or with manifest-only
  changes such as the number-backend feature forwarding, keeps its `dsntk-*`
  name until it is changed. Renamed so far: `rheon-feel`, `-feel-number`,
  `-feel-parser`, `-feel-evaluator`, `-feel-temporal`, `-feel-regex`,
  `-model-evaluator` and, since 2026-11-09, `-model` (it imports `rheon-feel`
  types, so upstream's `dsntk-model` cannot replace it). `scripts/upstream-rename.pl` holds the mapping; extend
  its crate list when another crate is renamed.

## Base

- Upstream commit: `c76aee02187183b3659a9051f3ebb283630066f7`
  ("Prepared next development version.", workspace version `0.3.1-dev`,
  `git describe`: `v0.1.0-47-gc76aee0`).
- This is the commit recorded as `upstream-commit` in `wasm/feel-wasm/Cargo.toml` (`[package.metadata.rheon]`).
  Commits 1-4 below are the four patches `dsntk-wasm/patches/dsntk/0001..0004`
  applied in order, one commit each. After commit 4 the tree is
  byte-identical to `dsntk-wasm/third_party/dsntk` (verified with
  `diff -r`, excluding `target/`, `.git/` and `.bootstrap-stamp`).

## Engine commits (oldest first)

| # | SHA | Title | Origin |
|---|-----|-------|--------|
| 1 | `e1fc50dc2a3fd9b941712ee0e76c01a51bf74b99` | Build on wasm32: gate Java external functions, 32-bit isize/usize conversions | `0001-wasm32-support.patch` |
| 2 | `b1a04ca6aaec7a93d57ca4141587e7f2c0b5eb4f` | feel-temporal: process-wide local UTC offset override for hosts without a time zone | `0002-local-offset-override.patch` |
| 3 | `ec2ced32fff2b0fdc72a8173385774cbb654a567` | Robustness: no user-reachable panics, non-finite numbers are null, call depth limit, iterative AST drop | `0003-robustness.patch` |
| 4 | `55948221d3e11df370366696d3d52431820d7c9f` | Linear `for` loops and list literal parsing (were quadratic) | `0004-linear-for-and-list-parsing.patch` |
| 5 | `8c56f2c83c5fb28d3cdd5d8faa10444ffa4d4996` | Add docs/FORK.md | fork only |
| 6 | `1fd27a4` | feel-number: shared decimal128 core for pure-Rust backends | dashu spike |
| 7 | `dde684a` | feel-number: dashu-float backend, feature `dashu` (default) \| `intel` | dashu spike |
| 8 | `c75b0da` | Forward the number backend features through the whole workspace | fork |
| 9 | `ecf9feb` | Tests: per-backend expectations where Intel's rounding error is encoded, dashu regressions | fork |
| 10 | `e6b67e8` | feel-number: track the sign of NaN like Intel (Debug-visible only) | fork |
| 11 | `6af4e49` | tools: differential fuzz and FEEL corpus harnesses (Intel vs dashu) | dashu spike |
| 12 | `0bfbc9b` | feel-number: `NUMBER_BACKEND` constant naming the compiled-in backend | fork |
| 13 | `3fe6406` | tools: vendor the patched dfp-number-sys used as the Intel reference | fork only |
| 14 | `8542135` | feel-number: scale rounding keeps the scale when the rounded coefficient is ±1 | upstream bug |
| 15 | `7c16676` | feel-evaluator: stop a numeric `for` range when the step is lost to rounding | upstream bug |
| 16 | `6f08eab` | feel-temporal: time ± duration wraps at midnight; order times and date-times across offsets as instants | upstream bug |
| 17 | `cb5997f` | feel-parser: bound the name-prefix search in the lexer (path chains were cubic) | upstream bug |
| 18 | `58bbc18` | feel-evaluator: readable null reasons for time/exp/log/duration; huge numbers in messages in scientific notation | fork |
| 19 | `f619d8a` | feel-temporal: `is()` on times stays zone-strict now that `=` compares instants | with 16 |
| 20 | `3dbd70e` | model-evaluator: expectations for the new null reasons of time(), exp(), log() and duration() | with 18 |
| 21 | `4b5ca51` | feel-number: scale-based rounding accepts the full FEEL scale range -6111..=6176 | TCK 1141-1144 |
| 22 | `69e81cc` | feel-regex: back-references in `matches` patterns (fancy-regex) | TCK 1111 |
| 23 | `b3b616b` | model: accept the DMN 1.6 namespace | DMN 1.6 |
| 24 | `f893c57` | feel: single-argument rounding and number(from); trim, intersection, insert after (1.7 beta) | DMN 1.6 |
| 25 | `f3259a0` | feel: `value` property of temporal values; filters on a non-list; number() text must be a numeric literal; tck-addendum | DMN 1.6 |
| 26 | `18bcd98` | feel: truncate fractional list positions before the sign check (remove panicked) | upstream bug |
| 27 | `f9b8111` | feel: date and time(date, time, timezone) | DMN 1.6 |
| 28 | `4aa566d` | model: load ONNX functions; compatibility tests follow the 1.6 changes | DMN 1.6 |
| 29 | `62bcf99` | feel-grammar back in the tree as rheon-feel-grammar (tables reproduce byte for byte) | fork |
| 30 | `18f0fd1` | feel: descendant operator `e...name` (grammar, lexer, AST, evaluator) | DMN 1.6 |
| 31 | `070be1a` | model: implicit conversions, TCK 1157 12/12 | TCK 1157 |
| 32 | `b986c79` | model: multiple imports (per-model import names), TCK 1160 4/4; boxed for over a single value | TCK 1160 |
| 33 | `e56f08c` | feel: B-FEEL; all()/any() follow the deciding item | DMN 1.6 |
| 34 | `6baa444` | feel: diagnostics and lenient/strict error modes; wasm `language`, `errorMode`, `diagnostics` | DMN 1.6 |
| 35 | `5e04541` | feel: equality with null no longer depends on the operand order | upstream bug |
| 36 | `94ec326` | feel: null equality is one rule; a non-conformant argument ends the invocation | upstream bug |
| 37 | `f65f252` | feel: one invocation for every function call (Table 64) | DMN 1.6 |
| 38 | `a1d5a62` | feel: `in` treats every null alike | upstream bug |
| 39 | `d39b7d6` | feel-parser: names found in an index over the borrowed input contexts; composite names searched, not flattened | upstream bug |
| 40 | `d39b7d6` | feel: the type of a list of contexts without building the item types | fork |
| 41 | `d39b7d6` | feel-evaluator: every evaluator that pushes a context restores the scope depth on return | upstream bug |
| 42 | `d39b7d6` | feel-evaluator: FEEL loops and filters move their contexts instead of cloning them | fork |
| 43 | `d39b7d6` | feel-number: decimal128 comparison, digit count and packing without 128-bit division | fork |
| 44 | `d39b7d6` | feel: function values share their definition (`Arc<FunctionDefinition>`) | fork |
| 45 | `d39b7d6` | feel-wasm: request input converted by value | fork |
| 46 | `d39b7d6` | feel-evaluator: subtraction null reasons from the evaluated operands (2^n - 1 diagnostics become n) | upstream bug |
| 47 | `d39b7d6` | wasm profile at opt-level 3 with a 10 MiB shadow stack | fork |
| 48 | `22afefd` | feel-parser: an iteration variable that starts with `in` is a syntax error, not a trap | upstream bug |
| 49 | `075af71` | feel-parser: a range literal's named temporal parameter keeps its name | upstream bug |
| 50 | `039128f` | feel-parser: list grammar actions append and complete each list once (the tails were quadratic) | upstream bug |
| 51 | `35f53e6`..`4aca592` except `cb925d0` | feel-parser: lexer and parser without per-token copies | fork |
| 52 | `cb925d0` | feel-parser: the composite-name search without recursion | fork |
| 53 | `6dcbbba` | feel-evaluator: a date range ending at the first or last supported date restarts at its start | upstream bug |
| 54 | `ff98977` | feel: strings share their text (`FeelString`) | fork |
| 55 | `5c2789f` | feel-wasm: test that no corpus request traps the module | fork |
| 56 | `cf5349d` | tools: identity check of full responses against the previous release | fork |
| 57 | `149beef` | ci: check response identity against the previous release | fork |
| 58 | `637cc8c` | tools: frozen temporal request corpus | fork |
| 59 | `77e25bf` | temporal: durations hold a symmetric range | upstream bug |
| 60 | `b4a7710` | temporal: a years and months literal is invalid when any component does not fit | upstream bug |
| 61 | `ae63142` | temporal: date and date-time arithmetic takes one signed step | upstream bug |
| 62 | `97decb9` | temporal: date plus or minus a days and time duration follows Table 58 | DMN 1.6 |
| 63 | `061ad26` | temporal: whole seconds of a duration are an i64 on every target | fork |
| 64 | `c6a7a69` | feel-evaluator: sort cannot panic on any predicate | upstream bug |
| 65 | `5fdaa0d` | feel-evaluator: one abort channel, reset and read by every entry point | fork |
| 66 | `8c4eee8` | feel: Display of lists, contexts and types writes as it goes | fork |
| 67 | `f1f09d6` | feel: one bounded renderer for message arguments and a cap per message | fork |
| 68 | `1f7c11e` | feel-evaluator: values in null reasons are previews | fork |
| 69 | `94eab15` | feel-parser: a syntax error quotes a preview of the input | fork |
| 70 | `032af5b` | model-evaluator: values in DMN messages are previews | fork |
| 71 | `30c2d9e` | feel-wasm: request echoes are previews and every message is capped | fork |
| 72 | `c7027b4` | regex: rheon-feel-regex tells engine limits from invalid patterns | fork |
| 73 | `a499385` | feel-evaluator: a regex engine limit aborts the evaluation | fork |
| 74 | `a27959d` | temporal: read fractional seconds as an exact integer | upstream bug |
| 75 | `2b9f135` | temporal: keep every time of day below 24:00:00 | upstream bug |
| 76 | `9ea1bf6` | feel-evaluator: build date and time(date, time, zone) from its parts | with 27 |
| 77 | `b67ace8` | diagnostics: report fractional seconds truncated to nanoseconds as warnings | fork |
| 78 | `e3af54d` | feel-evaluator: warn when time() seconds carry more than nine fractional digits | fork |
| 79 | `6c91a51` | model: bound how deeply the elements of a model nest | upstream bug |
| 80 | `e5a60d7` | model-evaluator: item definitions that form a cycle are a build error naming the cycle | upstream bug |
| 81 | `6512eef` | model-evaluator: an item definition type beyond the size or depth bound is a build error | upstream bug |
| 82 | `b25e023` | model-evaluator: decision-service function values hold a weak model reference | upstream bug |
| 83 | `078c1cd` | feel-wasm: display measures the inline width before rendering | fork |
| 84 | `3577ebe` | feel-wasm: no combination of edge operands traps the module | fork |
| 85 | `c4efc4f` | tools: automated checks of the identity tool | fork |
| 86 | `5247f8b` | scripts: one module session for the Node tools | fork |
| 87 | `4c63a17` | tools: the identity check answers the reference per source | fork |
| 88 | `5aeaa6c` | tools: cache the reference answers of the identity check | fork |
| 89 | `52f3aae` | tools: find the commit behind an identity difference | fork |
| 90 | `75f0541` | tools: a number oracle that judges every number result | fork |
| 91 | `0c50018` | ci: run the number oracle | fork |
| 92 | `063a27d` | tools: the number oracle's edge cases are an identity source | fork |
| 93 | `3ba44fd` | tools: frozen regex request corpus | fork |
| 94 | `397762a` | tools: remove the Intel differential tools | fork |
| 95 | `1d2087d` | feel-number: one number backend, no backend features | fork |
| 96 | `1526888` | feel-number: a number has no negative zero | upstream bug |
| 97 | `8fbf3e6` | feel-number: one canonical text for every number | fork |
| 98 | `a0a3bee` | feel-number: NaN carries no sign | fork |
| 99 | `c5bcbba` | feel-number: exp and ln results need no cohort rule | fork |
| 100 | `63e021c` | feel-number: round_to rounds to a scale in one step | upstream bug |
| 101 | `cb193b6` | feel-evaluator: the rounding built-ins share round_to | fork |
| 102 | `01bde71` | feel-number: new() scales without a clamp | upstream bug |
| 103 | `913b90a` | feel-number: exact modulo | upstream bug |
| 104 | `321f364` | feel-number: integer powers are correctly rounded | upstream bug |
| 105 | `c118fec` | feel-evaluator: an exponent outside Table 61's range gives null | upstream bug |
| 106 | `5083e22` | feel-temporal: a time offset prints its sign from its value | upstream bug |
| 107 | `55541f6` | feel-evaluator: time() takes every offset within ±14:00 and none with a fraction of a second | upstream bug |
| 108 | `e2ed204` | feel-temporal: the minutes of an offset literal are below 60 | upstream bug |
| 109 | `1ad19fc` | feel-temporal: an offset literal has no seconds | DMN 1.6 |
| 110 | `59fbd66` | feel-wasm: the offset of a zoned date and time is displayed with its seconds | fork |
| 111 | `43648ef` | feel-evaluator: second and seconds are exact decimals | upstream bug |
| 112 | `37c8fc2` | feel-temporal: the years and months of a duration are magnitudes | upstream bug |
| 113 | `4efa7f0` | feel-temporal: years and months duration(from, to) has one rule for both signs | upstream bug |
| 114 | `20efaa2` | feel-evaluator: the value of a time in a zone without a known offset is null | upstream bug |
| 115 | `4cda64c` | feel-evaluator: a years and months duration that drops a fraction of a month warns | fork |
| 116 | `1a77981` | feel-evaluator: pin the nulls that are no errors | DMN 1.6 |
| 117 | `384fc80` | feel: a missing context entry is a defined null with a note | DMN 1.6 |
| 118 | `26f2cdb` | feel-evaluator: stddev of fewer than two numbers is a defined null | DMN 1.6 |
| 119 | `1279ceb` | model-evaluator: a decision table without a matching rule is a defined null | DMN 1.6 |
| 120 | `70f3936` | model-evaluator: a missing input variable is a defined null whatever its type | DMN 1.6 |
| 121 | `5ed694d` | feel-evaluator: if takes the else branch unless the condition is true | upstream bug |
| 122 | `0cae1b8` | feel-evaluator: != reports like = | upstream bug |
| 123 | `0a1d4c5` | feel-evaluator: a null operand of an arithmetic operator is an error | upstream bug |
| 124 | `3adac0a` | feel-evaluator: a unary comparison compares with the value of its path endpoint | upstream bug |
| 125 | `4da3dcf` | model-evaluator: one collection evaluator for every simple type | fork |
| 126 | `c9202e8` | model-evaluator: an item definition that references another keeps its allowed values | upstream bug |
| 127 | `3069fdb` | model-evaluator: allowed values are tested against each element of a collection | upstream bug |
| 128 | `08f630e` | feel-evaluator: an iteration context sees the variables bound before it | upstream bug |
| 129 | `f846d99` | feel-evaluator: a repeated iteration variable gives null | DMN 1.6 |
| 130 | `7377b32` | regex: matches and replace read their flags by one rule | upstream bug |
| 131 | `38c6bf4` | regex: a replacement string follows the F&O rules | upstream bug |
| 132 | `a4d650d` | regex: replace and split reject a pattern that matches the zero-length string | upstream bug |
| 133 | `0590e61` | ci: document the published crates with warnings denied | fork |
| 134 | `f5c2713` | ci: run on Node 24 | fork |
| 135 | `7a8c066` | benches: remove the benchmarks that need a nightly toolchain | fork |
| 136 | `2dd5fde` | tools/identity: a ledger entry can state a rule | fork |
| 137 | `d421dcc` | tools/identity: request sources and a ledger for the release tier | fork |
| 138 | `c9023e9` | scripts: the TCK run uses pinned inputs and fails on a changed result | fork |
| 139 | `8d22825` | scripts: the three test tiers as commands with measured times and budgets | fork |
| 140 | `5d1944e` | ci: the group tier on pull requests, the release tier before a release | fork |
| 141 | `ec5dc38` | tools/identity: the Node requirement and the times against the current baseline | fork |
| 142 | `0b09875` | tools/identity: requests on values that are used again after they were passed on | fork |
| 143 | `234ac81` | model-evaluator: pin what a boxed context writes into an iteration or an item frame | fork |
| 144 | `0583407` | feel-evaluator: the ordering of sort takes a predicate that can stop | fork |
| 145 | `c6252c8` | tools/identity: the cache test holds when the differences trap the reference | fork |
| 146 | `3475456` | feel-evaluator: an abort stops every loop | fork |
| 147 | `0991817` | feel: a context has its own read API and no longer derefs to its map | fork |
| 148 | `7b58453` | feel: remove the named-parameter and positional-parameters values that nothing builds | fork |
| 149 | `41e9a2a` | feel: assign_entry returns the value it replaces | fork |
| 150 | `eedf7f8` | feel: a list value is a type of its own | fork |
| 151 | `c108d47` | feel: lists and contexts keep their nesting depth | fork |
| 152 | `ab97b62` | feel: values nest at most 500 levels within an evaluation | fork |
| 153 | `d969f14` | scripts: the stack check nests a value to the depth limit | fork |
| 154 | `a9d6506` | feel: lists share their items | fork |
| 155 | `cf2af1f` | feel-evaluator: equality visits each pair of shared values once | fork |
| 156 | `449ea2a` | feel-evaluator: the list built-ins that compare items share one equality walk per call | fork |
| 157 | `69cb975` | feel: instance of is decided on the value, without building its type | fork |
| 158 | `d3ce115` | feel: the parameters and the result type of a function definition are read through accessors | fork |
| 159 | `120b9eb` | feel: context values are shared | fork |
| 160 | `725faab` | feel: a scope frame is an owned context or a shared one | fork |
| 161 | `3b8615a` | feel: a function definition shares its closure context | fork |
| 162 | `d19c7fd` | feel-parser: the name index visits a shared context once | fork |
| 163 | `034fe04` | feel-parser: the item names of a list are read from its items | fork |
| 164 | `85d098e` | tools/identity: requests for filters by index, logic operators, callbacks, B-FEEL arithmetic and context literals | fork |
| 165 | `3fa2ee9` | feel-evaluator: one iteration path for FEEL and boxed expressions | fork |
| 166 | `ab7412b` | feel-evaluator: one filter path for FEEL and boxed expressions | fork |
| 167 | `3fbae27` | docs: shared values, the nesting limit and their measured effect | fork |
| 168 | `7f5baaf` | feel: the conformance of a value to a type visits a shared list or context once | fork |
| 169 | `3572bf4` | feel-evaluator: and / or evaluate the right operand only when the left does not decide | DMN 1.6 |
| 170 | `ccb89c4` | feel-evaluator: the item at an index of a filter is one function | fork |
| 171 | `33fb6ad` | feel-evaluator: a filter expression is evaluated once in the enclosing scope, and a number selects | DMN 1.6 |
| 172 | `05d5f49` | feel-evaluator: sort and list replace call their function in one scope | fork |
| 173 | `1436889` | feel-evaluator: the function called by sort and list replace is a counted call | fork |
| 174 | `1e23fe8` | feel-evaluator: arithmetic operators are named kernels behind one entry | fork |
| 175 | `947ff20` | tools/identity: a ledger outcome can be the digest of a long response | fork |
| 176 | `047ee58` | tools/identity: the depth probes of the large sets that 0.5.0 answers are committed | fork |
| 177 | `40c7ec5` | tools/identity: the large-set run leaves out two chains by name | fork |
| 178 | `ef0081f` | tools/identity: the order test compares differences without their source | fork |
| 179 | `d1eab1d` | tools: the identity check compares with 0.6.0 | fork |
| 180 | `8a34c4c` | feel-evaluator: value kernels by name | fork |
| 181 | `b7ddea6` | feel: a tentative mark without a closure; the deep search is a loop | fork |
| 182 | `fd820d6` | tools/identity: request sets for scoping, functions, order, types, cells and names | fork |
| 183 | `d78c977` | feel-evaluator: the evaluator is chosen in one place; feel-wasm: backends harness | fork |
| 184 | `6c40e04` | feel-wasm tests: random evaluable expressions with shrinking | fork |
| 185 | `f3de4a6` | model-evaluator tests: backend twins | fork |
| 186 | `f2da3e8` | scripts: tier steps for the backends harness and twins; stack report probes | fork |
| 187 | `682c4f1` | feel-evaluator: name resolution for the flat evaluator | fork |
| 188 | `5828c52` | feel-evaluator: the flat program, its run loop and the seam, with literals | fork |
| 189 | `59520de` | feel-evaluator: the flat evaluator runs names, qualified names and paths | fork |
| 190 | `48f989e` | feel-evaluator: the flat evaluator runs arithmetic, comparisons and between | fork |
| 191 | `e34aa74` | feel-evaluator: the flat evaluator runs logic, if, lists, ranges, in and unary tests | fork |
| 192 | `129cf87` | feel-evaluator: the flat evaluator runs invocations of built-ins and host functions | fork |
| 193 | `2b5c622` | feel-evaluator: the flat evaluator runs instance of and type expressions | fork |
| 194 | `33131b6` | feel-evaluator: the flat evaluator builds and runs small expressions with fewer allocations | fork |
| 195 | `89f3b40` | feel-wasm tests: the backends harness sends each request once | fork |
| 196 | `972980f` | feel-wasm tests: one helper runs a test body on a thread with a big stack | fork |
| 197 | `5da8aec` | model-evaluator tests: the twins compare whole diagnostics and note what was lowered | fork |
| 198 | `624781b` | feel-wasm tests: the backends harness compares responses without recursion | fork |
| 199 | `a55c369` | feel-evaluator: the flat evaluator runs context literals | fork |
| 200 | `23be293` | feel-evaluator: the flat evaluator runs for, some and every | fork |
| 201 | `7875acf` | feel-evaluator: the flat evaluator runs filters | fork |
| 202 | `adf572a` | feel-evaluator tests: allocation caps for loops and filters | fork |
| 203 | `2e714ac` | feel-evaluator: host invocations inside binders reuse one scope per region | fork |
| 204 | `2130045` | feel-evaluator: the resolution reports where a free name is read | fork |
| 205 | `ec17200` | feel-wasm tests: the sample of the backends harness has a refused request | fork |
| 206 | `a82f5ff` | feel-evaluator: the flat evaluator runs function definitions and their invocations | fork |
| 207 | `8b90d06` | feel-evaluator: a body finds a free name its caller has read at the caller's closure | fork |
| 208 | `00c72e6` | feel-evaluator: invocations of the flat evaluator allocate no arguments | fork |
| 209 | `6d47106` | scripts, docs: the stack check asserts FEEL recursion at the AST limit | fork |
| 210 | `aef8331` | feel-evaluator: an interval of integers is counted as integers | fork |
| 211 | `63ffdcf` | feel-evaluator: the root expression reads a free name of the host once | fork |
| 212 | `51b8221` | feel-evaluator: the scopes kept for functions of the host are one per layout | fork |
| 213 | `66beec7` | feel-evaluator: tests of what a function of the host sees; a body of another program gets its own scope | fork |
| 214 | `a8b20ed` | feel-runtime tests: the runaway evaluation that is cancelled keeps little memory | fork |
| 215 | `c2043b9` | tools/identity: a recursion that overflowed the stack of the host is registered | fork |
| 216 | `cfb2027` | feel-evaluator tests: the answers of the evaluator of closures are recorded | fork |
| 217 | `4567d9f` | feel-evaluator: the flat evaluator is the only evaluator of expressions | fork |
| 218 | `0bdf6b5` | feel-evaluator: the evaluator of closures is deleted | fork |
| 219 | `56b24c9` | feel-evaluator tests: the end of a time range is found by adding a duration | fork |
| 220 | `8cea6e1` | feel-evaluator: the recorded answers of the tests are not part of the package | fork |
| 221 | `360df97` | feel, feel-evaluator, feel-wasm: an evaluation can have a budget of steps | fork |
| 222 | `5c052cb` | feel-parser: a name that is not in scope ends before a keyword | fork |
| 223 | `d01efb7` | feel-parser: a string literal or a block comment that is not closed is a syntax error | fork |
| 224 | `6d233e3` | feel-parser: the name tokens after an iteration variable `item` compare each part with `in` once | fork |
| 225 | `d3e6945` | feel, feel-wasm: a JSON input context of more than 11 entries is built in one pass | fork |
| 226 | `d5bfeab` | feel-parser: the name index compares the items of a list only when a lookup depends on it | fork |
| 227 | `f1407dc` | feel-wasm: the JSON of context and input is read straight into values | fork |
| 228 | `795c84b` | feel-wasm: the response of an evaluation is written straight from the result | fork |
| 229 | `544f67f` | feel, feel-wasm: the records of an input list share their names and the order of their keys | fork |
| 230 | `cd5e6d8` | feel-number: the digits of a number text are read as a number, not collected as text | fork |
| 231 | `5f3494c` | feel-parser: a name is looked up among the pushed contexts in one step | fork |
| 232 | `c00944b` | feel-parser: a candidate name is a piece of its chain, looked up only when a name has its length | fork |

(The commits are on `main`, after the base commit; `git rev-parse <sha>` gives a full SHA.)

What each commit does:

1. **wasm32 support.** `reqwest` becomes a
   `cfg(not(target_arch = "wasm32"))` dependency of `feel-evaluator`; on
   wasm32 the Java external-function evaluator is replaced by a stub that
   returns a FEEL null ("external Java functions are not supported on
   wasm32"). The `isize`/`usize` conversions in `feel-number` no longer
   `compile_error!` on 32-bit targets; out-of-range values are a conversion
   error (FEEL null) instead of a panic. Java external functions are
   unsupported on wasm by decision; this is the final shape.
2. **Local offset override.** `feel-temporal` gains
   `set_local_offset_override`, `local_offset_override` and `local_now`.
   `chrono::Local` is always UTC on WASI, so an embedding host sets a fixed
   offset (dsntk-wasm sets it from the request's `tzOffsetMinutes`). Without
   an override nothing changes. This patch is kept by decision.
3. **Robustness.** Every user-reachable panic found in review becomes a FEEL
   null or an error: non-finite decimal results (`FeelNumber::is_finite`,
   `Display` never panics on Inf/NaN), `time()` with an out-of-range offset,
   unpaired `\u` surrogate escapes, huge positions/lengths (positions go
   through 64-bit integers so wasm32 and 64-bit agree), overflowing duration
   arithmetic. Adds `MAX_CALL_DEPTH` for nested function calls,
   `AstNode::depth`/`for_each_child`/`is_leaf`, an iterative `Drop` for
   `AstNode`, `from_str_reporting_rounding`, and a linear `Value::type_of`
   for lists (was exponential in nesting depth). The CLI `evaluator` crate
   gets a two-line fix so it compiles with the iterative drop.
4. **Linear loops.** `ForExpressionEvaluator` moved the `partial` list in and
   out of the iteration context instead of cloning it every iteration;
   `action_list_tail` appends and `action_list` reverses once instead of
   inserting at the front. `count(for i in 1..100000 return i)` went from
   over 30 s to 34 ms natively.
5. This file.
6. **Shared decimal128 core** (`feel-number/src/decimal128.rs`). The FEEL
   number with decimal128 semantics (34 digits, half-even, exponent
   -6176..=6111, cohorts), stored as a `Copy` triple (sign, `u128`
   coefficient, `i32` exponent) plus Inf/NaN. Exact operations are computed
   in the core (`u128`, or a small `U256` for wide sums, products, square
   roots and the cohort decision): `+ - *`, `sqrt` (exact integer root with
   remainder as sticky digit), rounding to integral, quantize, scalbn,
   comparisons, parsing (`bid128_from_string` rules), Intel-compatible
   `Debug`/`Display`, conversions, the integer-exponent `pow` (Intel's binary
   exponentiation, bit-identical) and the binary128 cohort emulation of
   `exp`/`ln`. The inexact operations go through a private trait `Backend`
   (`div`, `exp`, `ln` and one round of `exp(y ln x)` at a working
   precision); the backend returns the correctly rounded value with the side
   the exact result lies on, the core turns that into sticky digits and
   rounds exactly once (no double rounding in the subnormal range), and runs
   the precision-escalating Ziv loop (45/60/90/140 digits) for fractional
   powers, rounding exact ties half-even.
7. **dashu backend** (`feel-number/src/backend_dashu.rs`): dashu-float's
   non-panicking `Context` API at precision 34 (and at the working
   precisions for `pow`); dashu's own `sqrt`/`powf`/`powi` are not used
   (wrong rounding when the remainder equals the root, mis-certified near
   ties, non-terminating exact ties). Features in `feel-number`: `dashu`
   (default, pure Rust, no C toolchain) and `intel` (dfp-number-sys,
   `dfp_number.rs` untouched); exactly one must be on.
8. **Feature forwarding.** Every crate that depends on feel-number
   (feel-temporal, feel, feel-parser, feel-evaluator, model, model-evaluator,
   evaluator, workspace, gendoc, server, dsntk) has `dashu` (default) and
   `intel` features forwarding to its dsntk dependencies, and the workspace
   dependency table declares those crates with `default-features = false`.
   One selection covers the whole build graph; the fastnum/dashu spikes
   could only select the backend for the five-crate chain.
9. **Tests.** `feel-number/tests/pow.rs` `test_pow_002` (`12.2384283 ** 4.25`)
   and `model-evaluator` `dmn_3_0100::_0278` (TCK 0100
   `exponent_lhs_number_exp_rhs_number_005`, `5 ** 2.55`) encoded Intel's
   result, 1 ulp below the correctly rounded value; they are now per backend.
   Plus the spike's regression tests (scalbn clamping, exp/ln cohorts, pow
   with bases within 1e-33 of one, `(-1)^y` cohorts) and the core's own unit
   tests (256-bit helper against schoolbook string arithmetic, add/mul/sqrt
   against exact results, guards, subnormal single rounding, sqrt half-way,
   pow ties, scalbn padding, NaN sign).
10. **NaN sign.** `V::NaN` carries a sign (negation flips, `abs` clears, a
    NaN operand propagates its sign, invalid operations give `+NaN`) so that
    `Debug` prints `-NaN` exactly like `bid128_to_string`; `Display` and FEEL
    are unchanged (`NaN` / null).
11. **Tools** (`tools/`, see `tools/README.md`): the op-by-op differential
    fuzzer with the mpmath/decimal oracle and the FEEL corpus harness. Both
    were removed in item 94; the number oracle of item 90 judges the number
    core, and the corpus is kept in `tools/feel-corpus/`.
12. **`NUMBER_BACKEND`.** `rheon_feel_number::NUMBER_BACKEND` is `"dashu"` or
    `"intel"`, selected by the same feature as the backend itself; dsntk-wasm
    reports it as `numberBackend` in its `info` op. Since item 95 it is always
    `"dashu"`.
13. **Intel reference for `tools/`**: the patched `dfp-number-sys` crate the
    `tools/` harnesses build the Intel side with. It was vendored in full
    (17 MB, mostly Intel's C library); it is now `tools/dfp-number-sys.patch`
    on top of the crates.io 0.2.14 release, fetched by
    `tools/fetch-dfp-number-sys.sh`. The patch and the script were removed
    with the tools in item 94.
14. **Scale rounding.** `unscale()` (both backends) returned `±1` without
    scaling back whenever the rounded integral was `±1`, so `round up(0.1, 1)`
    was `1`, `floor(0.19, 1)` was `1`, `round up(10, -1)` was `1`. The
    upstream tests `round up(±5.5, -6111)` = `±1` and `floor(-1.56, -6111)` =
    `-1` pinned the bug; they now expect `±1E+6111`.
15. **Lost-step `for` ranges.** `for i in 1e34..1e34 return i` never ended
    (`1e34 + 1 = 1e34` at 34 digits, so the counter never advanced and the
    list grew until the memory cap). The number-interval iterator stops when
    the next value equals the current one; the result is `[1e34]`.
16. **Temporal fixes.** `time + days and time duration` wraps around midnight
    with the nanosecond carry (`23:59:59 + PT1S = 00:00:00`; it was
    `24:00:00`, and the duration's fraction was dropped). Times and date-times
    whose zones have an offset are ordered as instants in UTC
    (`2026-10-17T10:00:00+01:00 < 2026-10-17T10:00:00Z`,
    `10:00:00+02:00 = 08:00:00Z`); ordering across different offsets used to
    be always false. A local value stays comparable only with another local
    value (as for subtraction), so `@"…T23:59:59Z" >= @"…T00:00:00"` is still
    false (upstream test `comparison_ge::_0030`).
17. **Lexer.** `.` is an additional name symbol, so after each `Dot` the lexer
    scanned the rest of an `a.a.a…` chain as one candidate name and tried
    every prefix (O(n³) for n steps; 400 steps took 2 s). Prefixes longer
    than any name the scope can match are skipped; the fallback name is
    unchanged. The bound counts the composite keys only for a candidate
    with a part that trims to empty, and the built-in property names only
    right after a dot (39).
18. **Messages.** `time("10:00:00+15:00")` said `time_1`, `exp(14150)` said
    `exp`, `duration("P")` said `duration`, `log(0)` said nothing; they now
    say what is wrong, and type errors use the `invalid argument type` form.
    `sublist([1,2,3], 9e6144)` embedded 6,145 digits in its reason; numbers
    longer than 40 characters are formatted as `9E+6144` in messages. (Other
    null reasons still quoted such numbers in full until commit 68 made the
    values in null reasons previews.)
19. **`is()` on times.** `FeelTime::is` compares the fields and the zone as
    written (TCK 0103 `is(@"20:00:50+00:00", @"21:00:50+01:00")` is false),
    since `==` now compares instants.
20. **TCK expectations** for the reason texts changed by commit 18.
21. **Scale range of the rounding functions.** `round up(5.5, 6176)` was null
    because the scaled intermediate `n * 10^scale` must fit decimal128
    (adjusted exponent <= 6144), which was enforced as if it were the scale
    limit. `check_scale` (both backends) now rejects only scales outside the
    FEEL range -6111..=6176 and returns a value unchanged when it has no digit
    below 10^-scale; `decimal()` accepts 6176. TCK 1141-1144 `017_a` pass
    unpatched (dsntk-tck-patches had rewritten them to 6144).
22. **Back-references in `matches`.** `feel-regex` compiles `matches` patterns
    with `fancy-regex` (back-references, look-around, a backtracking limit
    that turns pathological patterns into a null instead of a hang; since
    commit 73 a pattern beyond an engine limit stops the evaluation with an
    error);
    `replace`/`split` keep `regex`. With the `x` flag, whitespace inside
    `\p{...}` is stripped before compiling. TCK 1111 `K2-MatchesFunc-17`
    passes unpatched.

Commits 23-38 are described in their commit messages. Commits 39-54 make parsing and
evaluation faster and fix the upstream bugs found on the way; apart from the fixes marked
"upstream bug", every response is the same as 0.3.0's (measured below).

39. **Name index.** For every parse 0.3.0 converted the whole evaluation scope into a parsing
    scope (a deep copy of every input context), and for every name the lexer read it built
    the set of all flattened keys in scope, the composite keys `k1 . k2 . … . kn` of nested
    contexts included. That set grows exponentially with the nesting depth: a path
    `k0.k1.….k99` into a 100-level context ran into the 512 MiB cap. `ParsingScope<'a>` now
    borrows the input contexts (`FeelScope::frames`) and indexes each one on first use
    (`FrameIndex`: one walk with an explicit stack over the nested contexts, the context types
    and the item type of a list of contexts whose items all conform to it). A name that
    contains the composite separator ` . ` is found by a search over the states (node, offset
    in the name), each visited at most once. The lexer's prefix bound (17) is computed without
    the set: the composite keys count only for a candidate with a part that trims to empty
    (only U+1680 trims away, and only such a part can put ` . ` into a candidate), the
    built-in property names (14 bytes at most, derived from `bip::BUILT_IN_PROPERTY_NAMES`,
    `5954a8f`) only right after a dot. One function decides which values carry nested names,
    for the index, the composite search and the contexts pushed while parsing (`4a6a585`); list
    item types are keyed by address, so `ParsingScope` is `Send` as at 0.3.0 (`74620b7`).
40. **List types.** `Value::type_of` of a list built the type of every item to find the common
    item type of a list of contexts. A context is now checked against a context type entry by
    entry, without building its type (`121517c`: one rule for the type of a list). The types are
    the same; randomized tests compare the rule with 0.3.0's at every nesting level.
41. **Scope depth.** The filter (`only number or boolean indexes`) and the context literal (an
    entry that is a null) returned early without popping the context they had pushed, so the
    caller popped the wrong one: `for i in 1..3 return [1,2]["x"]` was `[null]` (two results
    lost), and in `{r: for i in 1..3 return [1,2]["x"], s: i}` the loop variable was still
    visible after the loop (`s: 3`). Every evaluator that pushes a context (filter, context
    literal, invocation, B-FEEL operands, iterations) records the depth and restores it on every
    return (`FeelScope::{depth, truncate, pop_frame}`): three nulls, and `s: null`.
42. **Loops and filters move their contexts.** FEEL `for`, `some` and `every` cloned the
    iteration context into the scope for every iteration, and the filter cloned every item.
    They now move the context into the scope and back (`partial` is assigned in place,
    `FeelContext::assign_entry`). The filter moves each item: a context with an `item` entry
    becomes the frame itself, another context is copied once for its entries, and the frame
    `{item: …}` is reused from item to item. The iteration advances with `next()` alone, and
    `some` and `every` stop iterating at the deciding item (0.3.0 stopped evaluating there but
    ran the iteration to its end; results and diagnostics are the same). If an evaluation
    removed the iteration or item context, which no evaluator does, the loop, filter, `some` or
    `every` returns null with an internal-error reason instead of a wrong result, after the
    scope is restored (`910a535`); `ForExpressionEvaluator::evaluate` returns that `Value`.
    The model evaluator's boxed iterators and filters keep copying (`new()`), because a boxed
    context writes its entries into the context on the top of the scope (docs/BACKLOG.md); the
    FEEL builders use `for_feel()`. `count(for i in 1..10000 return i)` natively: 132
    allocations (+17 reallocations) per call, 40,199 (+26) at 0.3.0.
43. **Decimal fast paths.** `digit_count` comes from the bit length (times 1233/4096, then one
    comparison with a power of ten) instead of 128-bit divisions. Two finite numbers with equal
    exponents are compared by their coefficients; otherwise only the coefficient with the
    larger exponent is scaled (`checked_mul`; an overflow means that side is larger).
    `round_pack_u128` returns at once for a coefficient below 10^34 with the exponent in range.
    Exact: unit tests compare each path with the general algorithm on every edge value and on
    random values (`feel-number/src/decimal128/tests.rs`), and `tools/difftest/run.sh 100000` (removed in item 94)
    (3,900,001 cases) on this code gives the same mismatch set, cohorts and oracle verdicts
    against Intel as 0.3.0's core. The production code of `decimal128.rs` is unchanged since
    `d39b7d6`.
44. **Shared function definitions.** `Value::FunctionDefinition` holds an
    `Arc<FunctionDefinition>` (named fields, `FunctionDefinition::new`, `From<FunctionDefinition>
    for Value`) instead of six fields, so a name lookup or an argument copies a pointer. The
    invocation borrows the definition, and `closed_over` copies it (`Arc::make_mut`) only when
    the closure has entries to add. Arguments and results are coerced by value
    (`Value::try_into_coerced`, `into_coerced`, with the checks of `try_coerced`, `6e5b59d`).
    Without the copy of each argument the allocator placed strings differently, and a recursion
    that doubles a string 26 times hit the 512 MiB cap; commit 54 removes that (shared strings).
    `size_of::<Value>()` is 64 B natively (128 B at 0.3.0) and 64 B on wasm32 (as at 0.3.0:
    there the 48-byte `FeelNumber` sets it). `FunctionDefinition` derives `Debug` (`359a14a`),
    so the `Debug` text of a function value differs from 0.3.0's.
45. **Input by value.** The request's `context` and `input` are converted by value: strings
    and keys are moved (a key that trimming leaves unchanged becomes the `Name` without a
    copy), numbers are parsed from the JSON number's own text (`arbitrary_precision`), entries
    are inserted with `FeelContext::insert`, and the rest of the request is dropped before
    parsing. The path of a value in warnings and errors (`a.b[0]`) is written only when a
    message needs it, without recursion (`8dcba2b`). The texts and order of the warnings and
    errors are 0.3.0's (golden tests in `wasm/feel-wasm/tests/contract.rs`).
46. **Subtraction reasons.** The null reason of a failing subtraction evaluated both operands
    again to print them, so a chain of failing subtractions was exponential:
    `{a: "x", r: a - 1 - … - 1}.r` with 16 subtractions recorded 65,535 diagnostics
    (2^n - 1). The reason is formatted from the operands already evaluated: 16 diagnostics, with
    the same texts.
47. **Build profile.** `[profile.wasm]` builds at `opt-level = 3` (was `"z"`); its stack frames
    are larger, so the shadow stack is 10 MiB (was 8 MiB, `wasm/feel-wasm/build.rs`).
    `scripts/stack-headroom.mjs` (`npm run stack`, run by CI, `4fffdf0`) checks that five
    constructs nested to the AST depth limit evaluate on Node's main thread, in a Node worker
    with a 4 MiB stack and in Wasmtime with a 4 MiB wasm stack, and that a recursion of 190
    calls whose body is nested 60 levels (the depth 0.3.0 reaches in the TurboFan worker and in
    Wasmtime, measured on macOS arm64) evaluates in the worker and in Wasmtime. Measured on
    the final module (macOS arm64, Node 26, Wasmtime 49.0.3):
    - engine time: an opt-level z build of the same code takes 1.85x as long (geomean of 24
      requests, two alternating runs);
    - size: 3,786,626 B raw, 1,018,652 B gzip, 667,524 B brotli (opt-level z: 3,160,691 /
      840,786 / 552,386; 0.3.0: 3,129,141 / 830,762 / 545,186);
    - initial linear memory: 11.38 MiB (0.3.0: 9.38 MiB);
    - the deepest body (nesting levels) with which `npm run stack -- --report`'s recursion of
      190 calls still evaluates: Node worker with a 4 MiB stack 64 with either V8 compiler
      (0.3.0: 49 Liftoff, 60 TurboFan), Wasmtime with a 4 MiB wasm stack 64 (0.3.0: 60),
      Wasmtime's default 512 KiB 8 (0.3.0: 6), Node's main thread 17 Liftoff, 14 TurboFan
      (0.3.0: 10, 14). At 64 the 10 MiB shadow stack runs out, not the host's native stack.
48. **`in` at the start of an iteration variable.** Where the grammar expects an iteration
    variable, the lexer cuts the name before its first `in` part. When the first part was `in`
    the cut underflowed (`index - 1`): `for in+1 in [1] return 1`, `for in[1] return 1` and
    `some in.x in [1] satisfies true` panicked natively and trapped the wasm instance. Such an
    `in` is now the keyword `in` (a name never starts with a literal terminal symbol,
    DMN 1.6 §10.3.1.4), so these are syntax errors; `for in1 in [1] return in1` is `[1]` as
    at 0.3.0. After a variable named `item` the mode stays set (docs/BACKLOG.md), so
    `for item in[1] return item` (a trap at 0.3.0) is `[1]`.
49. **Range literal with a named parameter.** The grammar action for the named temporal
    parameter of a range literal read the colon's slot, so
    `range("[date(from: \"2012-01-01\")..date(\"2013-01-01\")]")` built `date` with an unnamed
    string and was null ("invalid range literal"). `feel.y` models the parameter like a named
    parameter of an invocation (tables regenerated with Bison 3.8.2): the range
    `[2012-01-01..2013-01-01]`.
50. **List grammar actions.** The nine list-tail actions inserted each item at the front
    (`insert(0)`, quadratic in the length of the list; commit 4 fixed only the list literal).
    Every list is now built by appending and completed once by its consumer, through one pair
    of helpers (`f8957b4`).
51. **Lexer and parser without per-token copies.** Each step is exact (the token streams, ASTs
    and errors are compared on exhaustive and randomized inputs):
    - token values move into the AST; the parser's stacks start with room for 16 states,
      16 values and 8 nodes;
    - each name chain is scanned once per parse, and its parts are byte ranges into the input,
      not strings;
    - a candidate without an empty part skips the composite search, and probing the contexts
      pushed while parsing allocates nothing;
    - the candidate bound uses only the names that share the token's first word;
    - the input index is built on first use, and a one-part name needs no lookup unless the
      scope decides its token;
    - string literals without escapes and runs of digits take one exact allocation;
    - the keyword window (12 characters) is read only after a keyword's initial;
    - the lexer reads the input `&str` in place (0.3.0 copied it into a `Vec<char>`, 4 bytes
      per character).

    Memory, measured natively on the final code against the code of commit 39: `x1 + x2 + … +
    x400` with the 400 names in the context takes 2,953 allocations per call (164,153), and its
    peak live memory is 172,541 B (147,801 B), because the parts of every name chain are kept
    for the whole parse (32 B per part natively, 16 B on wasm32). The parse of `1 + 2` peaks at
    2,338 B (1,338 B), the stacks reserved up front.
52. **Composite search without recursion.** The first composite lookup converted every input
    context into a parsing context with a recursive conversion (stack depth = nesting depth).
    The search runs on the index's nodes with an explicit stack, and one implementation serves
    the input contexts and the contexts pushed while parsing (`1e1f677`). A composite name
    2,000 levels down parses in a 256 KiB thread. Requests to feel.wasm cannot nest that deep
    (JSON contexts stop at about 125 levels, FEEL contexts at the AST limit), so this is
    robustness for direct callers of the library.
53. **Date range at the limit.** A date range that ends at the last supported date
    (262142-12-31, going up) or the first (-262143-01-01, going down) repeated its end date when
    an outer iteration context advanced, instead of restarting at its start: the iteration
    contexts are a cross product, inner to outer (DMN 1.6 §10.3.2.14).
54. **Shared strings.** `Value::String` holds a `FeelString`, a newtype over `Arc<String>`, so a
    name lookup, a string literal or a copied list item copies a pointer, not the text. The one
    operation that changes a string, `+` (FEEL and B-FEEL), appends in place when its left
    operand is not shared and otherwise allocates the result once at its final length. Every
    output, message and `Debug` text is unchanged. With commit 44's arguments moved instead of
    copied, `s + s` had left freed operand copies behind as holes too small for the next, longer
    string; now no operand is copied. A recursion that doubles a string argument 23 times grows
    linear memory by 32.06 MiB, 2.00x the 16 MiB result (0.3.0: 48.06 MiB, 3.00x), and 26
    doublings complete at a peak of 267.44 MiB (0.3.0: 393.44 MiB). Each string now carries a
    40-byte shared header: filtering 10,000 contexts (`list-filter-count`) makes 40,000 fewer
    allocations and peaks 3.3% higher natively (22.46 MB against 21.75 MB before this commit;
    51.6 MB at 0.3.0). Speed against the build before this commit: 1.006x on the geomean of
    the 24 requests.

Speed, measured 2026-11-13 on the final module (wasm, Node 26 on macOS arm64, median wall time
of a `feel_call` on a warm instance): 5.31x 0.3.0's speed on the geomean of 24 requests and
5.10x on 84 parse-heavy requests. For example, a formula over a 1,000-entry input context
7.13 ms → 0.46 ms; a 200-entry context literal with a 1,000-entry input context 188 ms →
1.36 ms; `x1 + … + x400` 55.0 ms → 0.29 ms; the 100-segment path `a.a.….a` 356 ms → 0.36 ms;
a 5,000-entry context literal 19.2 ms → 6.2 ms; `count(for i in 1..10000 return i)` 7.58 ms →
1.11 ms; filtering 10,000 contexts 38.3 ms → 11.1 ms. Full responses (result, type, warnings,
diagnostics, errors) of 1,174 requests (evaluate requests and a parse request made from each)
equal 0.3.0's except 52 registered differences, all from commits 39, 41, 46, 48, 49 and 53.

Commits 55-84 (0.5.0) make the engine robust at its limits: the inputs that trapped the module
through a panic no longer do (an input that exhausts the module's memory still traps), every
message and evaluation is bounded, temporal values are exact at their edges and in their
fractions, and model loading is bounded. Each change of a wasm answer against 0.4.0 that the
identity check finds is registered with both answers and counted in its item, except the 544
number previews of the edge-operand product (item 84), instances of the registered message rule
that are not listed one by one; native-only changes are stated in their items with both answers.
The totals are under "Verification" below.

55. **No-trap corpus.** `wasm/feel-wasm/tests/corpus/no-trap.json` is a JSON array of module
    requests, exactly as a host sends them, and the contract test `corpus_requests_never_trap`
    sends each one through `feel_wasm::call`, natively and under Wasmtime. A request passes when
    the module answers it with JSON whose `ok` is a boolean and whose error, if any, is not
    `internal` (a caught panic), and when the next request, a function call (`abs(-1) + 1`),
    still answers `2`, so no call-depth or abort state lingers. Natively a panic fails the test
    with the request named; under Wasmtime a trap aborts the test binary, and the request is the
    last `no-trap:` line on stderr. Values are not asserted here: the corpus is also a request
    source of the identity check (commit 56), which pins every answer. The 66 initial requests
    are inputs at the engine's limits that 0.4.0 answers: temporal values at the edges of the
    supported range with checked operations, month-end arithmetic, regex patterns at the engine
    limits (catastrophic backtracking, `\w{10000}`, 65 nested groups), expressions at and beyond
    the 300-level nesting limit (as `evaluate` and `parse`, and as a FEEL context), recursion at
    and beyond the 200-call limit, and `sort` with consistent predicates on 0 to 10,000 items. An
    input that traps the module is added here in the commit that fixes it and stays as a
    regression seed. The test runs on the same 512 MiB native thread as
    `nesting_and_recursion_limits`, through one shared helper, because unoptimized native test
    builds use much bigger stack frames than the release module and a native stack overflow
    aborts the test binary instead of failing one test. Basis: no input may panic the engine
    (CONTRACT §1: on wasm a panic aborts the instance). No API change.

56. **Identity check against the previous release.** `tools/identity/` holds the check that every
    full response of a candidate `feel.wasm` equals the previous release's, except the differences
    registered with their basis. `run.sh <reference.wasm> <candidate.wasm>` sends every committed
    request source (so far the no-trap corpus of commit 55, each `evaluate` request also as
    `parse`) through both modules, each request byte for byte as written and every number read
    losslessly from its source text, and compares the whole responses, without `elapsedMicros`,
    `engineCommit` and the `version` of `info`; a trap is the response `{"trap": true}`. The
    ledger `registered/<change>.json` lists, per change that alters answers, each request with its
    `why` and the `old` and `new` answer, as the result text or the whole response. A difference
    that is not registered fails the run, and so does a registered entry that does not reproduce
    on both sides, a request registered twice, or a malformed ledger, so the ledger cannot drift
    from the code. `--record` drafts the entries of a change from a run. `baseline.json` names the
    release (`v0.4.0`) and the sha256 of its `feel.wasm` asset, and `fetch-baseline.sh` downloads
    that asset and fails closed on a different sum. Checked on 0.4.0: the module against itself
    exits 0; a registered entry with a wrong `new`, and one that does not differ, exit 1; 0.3.0
    against 0.4.0 exits 1 with its differences listed, and 0 once they are recorded and passed as a
    ledger; a malformed ledger, a duplicate registration, a mixed or invalid `old`/`new` and a
    request that calls `now()` exit 2. Basis: answers stay identical to the previous release except
    the changes registered with both answers. No API change.
57. **The identity check in CI.** The `wasm` job runs `tools/identity/fetch-baseline.sh` and
    `tools/identity/run.sh .tools/baseline/feel.wasm dist/feel.wasm` right after
    `npm run build:wasm`, with `GH_TOKEN` from the workflow's token for the release download. A
    pull request that changes an answer without registering it fails there. Checked locally with
    the same commands: a build whose call-depth message differs by one word fails the step with
    the four requests of the corpus that quote it; the unchanged build passes it; a download with
    a different sha256 is deleted and fails the step, and so does a failed download. No API
    change.
58. **Frozen temporal corpus.** `tools/identity/requests/temporal.json` holds 440 temporal requests
    and no expected answers: the previous release answers them, and every change of an answer is
    registered with the change that makes it. It covers fractional seconds of 1 to 9 digits (all
    zeros, a final 1, all nines, fractions whose binary product is known to be off, exact ones) and
    of more than 9 digits, in each literal form (`time`, `date and time` and `duration` texts and
    `@"…"` literals); the exact constructor paths (`time(h, m, s)` with fractional and computed
    seconds, `date and time(date, time[, zone])`) and carries at 24:00 and at the end of a year;
    the range edges of dates, date-times and both duration kinds, year 0 and years below 1000;
    daylight saving gaps and overlaps (Europe/Paris, America/New_York, the 30-minute shift of
    Australia/Lord_Howe, the skipped day of Pacific/Apia, a date beyond the zone rules' horizon);
    zoned against zone-less comparisons; and offsets at and beyond ±18:00, with seconds and with
    invalid minutes. With the evaluate requests also sent as parse requests and the no-trap corpus,
    the identity check sends 1,008 requests, all answered without a trap and identical between the
    build and 0.4.0. No API change.
59. **Durations hold a symmetric range.** A years and months duration holds -(2^63 - 1) to
    2^63 - 1 months, a days and time duration -(2^63 - 1) to 2^63 - 1 nanoseconds. Both types
    accepted -2^63 before, whose absolute value does not exist in an `i64`: its `Display`, `abs`
    and components overflowed, which trapped the module (`duration("P1M") *
    -9223372036854775808`, its `string`, `abs` and negation, a date or date and time plus or minus
    it, the `.days`, `.hours`, `.minutes`, `.seconds` of the days and time twin), or the saturating
    absolute value printed it one nanosecond off (`duration("PT0.000000001S") *
    -9223372036854775808`, a date and time difference of exactly -2^63 ns, sums and quotients that
    land there). Every constructor from a raw count now checks the range, so `abs` and negation
    are total, and a product, quotient, sum or difference of -2^63 is null with the reason of
    every other out-of-range result. The `value` of a date and time is computed from the instant
    difference in nanoseconds itself, not through a duration, so
    `date and time("1677-09-21T00:12:43.145224192Z").value` stays the exact `-9223372036.854775808`
    (DMN 1.6 §10.3.2.3.5: valuedt is the number of seconds since the epoch). The inputs that
    trapped are in the no-trap corpus; 34 registered differences, each with both answers, 10 of
    them combinations of the edge-operand product of item 84 (sums, differences and products of
    exactly -2^63 nanoseconds or months). Basis: no input may panic the engine (CONTRACT §1). API
    breaks (rheon-feel-temporal): `FeelYearsAndMonthsDuration::from_ym` and `from_m` and
    `FeelDaysAndTimeDuration::from_n` and `from_sn` return `Option<Self>`;
    `FeelDaysAndTimeDuration::from_s` takes an `i32`; the `ops::Add` and `ops::Sub` impls of both
    durations are removed (use `checked_add` and `checked_sub`), `checked_neg` is removed and
    `ops::Neg` is total; both derive `Default` (zero); `&FeelDate - &FeelDate` returns
    `Option<FeelYearsAndMonthsDuration>`; `FeelDateTime::nanos_since` is added.
60. **A years and months literal is invalid when any component does not fit.** The parser
    accepted a literal when one of its components was valid and silently left out a component
    that overflowed: `duration("P768614336404564650Y8M")` was `P768614336404564650Y`,
    `duration("P99999999999999999999Y3M")` was `P3M` and `duration("P1Y99999999999999999999M")`
    was `P1Y`. A literal is now valid when it has a component and every component it has fits the
    duration range, so these are null with `duration: invalid duration literal: '…'` (an `@"…"`
    literal with `invalid @ literal: …`). Basis: DMN 1.6 §10.3.2.3.8, a years and months duration
    is its years and months, valueymd = 12 × years + months; the literal must not mean a
    different duration. 6 registered differences. No API change.
61. **Date and date and time arithmetic takes one signed step.** A date or a date and time plus
    or minus a years and months duration moves the date by the signed number of months in one
    function, and a date and time plus or minus a days and time duration moves the instant by the
    signed number of nanoseconds with a checked step. Before, the sign was handled by `+` and `-`
    calling each other with the absolute value, and subtracting a days and time duration used
    chrono's `-=`, which panics outside the supported instants:
    `date and time("-262143-01-01T00:00:00Z") - duration("P1D")` and
    `date and time("-262143-01-01T00:00:00Z") + duration("-P1D")` trapped the module. They are null
    now, with the reasons of the same results after the last supported instant. The month step
    keeps the day of the month and takes the last day of a shorter month, as before: DMN 1.6 §9.4
    gives these operations the semantics of op:add-yearMonthDuration-to-date and
    op:subtract-yearMonthDuration-from-date (XML Schema Part 2, Appendix E), while Table 58's
    formula `date(…, e1.day)` has no value for such a day. Nothing in rheon-feel-temporal calls a
    chrono operation that can panic. Basis: no input may panic the engine (CONTRACT §1); Table 58,
    `valuedt⁻¹(valuedt(e1) ± valuedtd(e2))`. 8 registered differences, 5 of them combinations of
    the edge-operand product of item 84 (the first supported instant minus a duration). API break
    (rheon-feel-temporal): `FeelDate::add_months(u32)` and `FeelDate::sub_months(u32)` are removed;
    the signed month step is internal to the crate.
62. **Date plus or minus a days and time duration follows Table 58.** DMN 1.6 Table 58 defines it
    as `date(valuedt⁻¹(valuedt(e1) ± valuedtd(e2)))`, with the date taken as the date and time at
    UTC midnight (§10.3.2.3.5): the result is the date of that instant, so the signed nanoseconds
    are rounded down to whole days in one step. Before, whole days were counted from the unsigned
    components and, going back, one more day was taken only for a remainder of at least one
    second, so a sub-second remainder was ignored: `date("2020-01-02") - duration("PT0.5S")` was
    `2020-01-02` and is `2020-01-01` now, `date("2020-01-02") + duration("-PT0.000000001S")` is
    `2020-01-01` and `date("2020-01-02") - duration("P1DT0.5S")` is `2019-12-31`. Every other
    result is unchanged. Basis: DMN 1.6 Table 58; owner decision to follow it for the sub-second
    remainder. 12 registered differences, 9 of them combinations of the edge-operand product of
    item 84: a date minus one nanosecond is the day before (6), and from the first supported date
    it is out of range and null (3). No API change.
63. **Whole seconds of a duration are an `i64` on every target.**
    `FeelDaysAndTimeDuration::as_seconds` returned an `isize`, 32 bits on wasm32, so `time(h, m,
    s, offset)` read an offset of 2^32 seconds or more modulo 2^32 there:
    `time(10, 0, 0, duration("PT4294967296S"))` was `10:00:00Z` on wasm and null natively. Every
    target now answers null, `time: offset is out of range: …`, as 64-bit native builds did; the
    positions and lengths of the built-ins already go through 64-bit integers for the same
    reason (CONTRACT §5). Basis: results never depend on where the core runs. 4 registered
    differences (wasm only). API break (rheon-feel-temporal): `FeelDaysAndTimeDuration::as_seconds`
    returns `i64` (was `isize`).
64. **`sort` cannot panic on any predicate.** `sort` passed the user's `precedes` to the standard
    library's stable sort, which may panic when the comparison is not a total order (documented
    since Rust 1.81), and does on 1.95: `sort(for i in 1..33 return modulo(i, 7), function(x, y)
    modulo(x * 7, 5) < modulo(y * 3, 5))` trapped the module. DMN 1.6 §10.3.4.9 (Table 81) states
    no condition on `precedes`, so `sort` now owns its ordering (`feel-evaluator/src/bifs/sorting.rs`,
    no recursion, no `unsafe`, positions instead of moved values): up to 20 items by insertion,
    with the same calls of `precedes` as before; longer lists in runs (a run in which each item
    precedes the one before it is reversed, a run in which none does is kept, shorter runs are
    extended to 20 items by binary insertion), merged level by level, the right item first only
    when it precedes the left one, with galloping searches once one side wins 7 times in a row.
    For every predicate the result is a permutation fixed by the list and the answers of
    `precedes`; for a strict weak order it is the stable sort, so every such result is
    unchanged. An answer other than `true` still means "does not precede". Calls of `precedes`
    for 10,000 items against the library sort: pseudo-random 0.82-0.87x, ordered and reversed
    equal (9,999), a sorted list with a random tail 0.59x, seven distinct values 1.33x. Speed on
    the wasm module (median of 15 runs, Node, against 0.4.0): `count(sort(for i in 1..10000
    return modulo(i * 7919, 10007), function(x, y) x < y))` 44.2 ms → 39.3 ms (0.89x the time),
    the ordered list 1.03x, the reversed list 1.01x, 10,000 items of seven distinct values
    19.7 ms → 24.2 ms (1.23x), 1,000 items 0.90x, 20 items 1.00x. Basis: no input may panic the
    engine (CONTRACT §1). 3 registered differences: the trapping input, and two predicates that
    are not strict orders on 200 and 1,000 items, whose permutation differs. No API change.
65. **One abort channel, owned by one evaluation.** The call-depth limit was a sticky thread-local
    flag that only the wasm layer reset and read. `ModelEvaluator::evaluate_invocable` reset it
    and never read it, so a DMN evaluation that hit the limit served its cut-short value as the
    result (`null(function call depth limit (200) exceeded)`), and `evaluate_context`,
    `evaluate` and the other FEEL entry points neither reset nor read it: after one runaway
    request body, every function call of the native server's next body on that worker thread
    was null. Now `rheon_feel::diagnostics` holds one evaluation state (the call depth and the
    abort that stopped the evaluation, first reason kept) that is clear outside an evaluation
    and cleared by RAII when the evaluation ends, also on a panic. Every entry point opens or
    joins an evaluation (`Session::run` in rheon-feel-evaluator) and reports an abort instead of a
    value: `evaluate` and `ModelEvaluator::evaluate_invocable` return an `Outcome`,
    `evaluate_context` and the range-literal entry points return `Err` (`<EvaluationAborted>
    function calls are nested too deeply (more than 200 levels, e.g. runaway recursion);
    evaluation stopped`), and the server, the TCK handler and the CLI report it as an error. A
    prepared evaluator and a decision table evaluator run as one evaluation: inside a session
    they join it, outside one a stopped evaluation is null with the abort's text, so no partial
    value reaches an enclosing expression (`{f: function(n) f(n + 1), r: f(1)}.r = null` was
    `true`) and no hit policy answers after a limit stopped one of the table's entries (a FIRST
    table whose first rule's input entry recursed without end answered its second rule's
    output). A refused call returns a bare null that is never observed. Basis: DMN 1.6 §7.3.8
    (the evaluation reports diagnostic information; a limit is not a FEEL result), CONTRACT §3
    (a stopped evaluation is reported as an `evaluation` error, never as a partial result), and
    the rule that no state outlives one request. The wasm responses are identical, the call-depth
    `evaluation` and `context` errors included; the changed answers are native only: the
    runaway context body and the next body on the same thread, the DMN invocable, the CLI,
    and a prepared evaluator and a decision table evaluator outside a session. API breaks:
    rheon-feel adds `diagnostics::{Abort, evaluation, one_evaluation, abort, enter_call, Call}`
    and `From<Abort> for DsntkError`, with `Abort` and its variants `#[non_exhaustive]`
    (`Abort::limit` builds the engine-limit variant); rheon-feel-evaluator removes
    `reset_call_depth` and `call_depth_exceeded`, adds `Session` and `Outcome`, `evaluate` returns
    `Outcome<Value>`, `evaluate_context`, `evaluate_context_node`, `evaluate_range_literal` and
    `evaluate_range_literal_node` return `Err` on an abort, and `prepare` and
    `prepare_with_dialect` evaluators turn an abort into a null with its text;
    `evaluate_invocation` joins a running session, and outside one it is an evaluation of its
    own, whose stop is null with the abort's text (was `null(function call depth limit (200)
    exceeded)`, with the flag left set for the next caller on the thread);
    rheon-model-evaluator: `evaluate_invocable` returns `Outcome<Value>` and
    `evaluate_invocable_with_mode` returns `Outcome<(Value, Diagnostics)>`, and the evaluator of
    `build_decision_table_evaluator` turns an abort into a null with its text; unpublished:
    `dsntk-evaluator` re-exports `Session` and `Outcome`, `Workspaces::evaluate` returns `Err` on
    an abort.
66. **Display of lists, contexts and types writes as it goes.** The `Display` of a list (and of an
    expression list), of a context and of a context or function type built the text of every
    element, joined them into one `String` and then wrote it, and a range or a null with a
    reason built its parts the same way. A writer that stops early (a bounded message) therefore
    still paid for the whole text: a 100,000-item list offered all 688,895 bytes to a writer that
    wanted 200. Each element is now written straight to the formatter, separated by `, ` in one
    helper, so formatting stops at the first refused write. Every text is byte for byte the same.
    Basis: messages must be bounded in time and memory, not only in length. No API change.
67. **One bounded renderer for message arguments and a cap per message.** A null reason was
    `format!` of whatever its arguments printed, so a reason that quoted a long list, context or
    pattern held all of it: `(for i in 1..1000 return i) = 1` gave a 4,913-character reason, and
    1,000 comparisons of a 100,000-item list (`{big: for i in 1..100000 return i, r: for j in
    1..1000 return big = j}.r`) built about 690 MB of reasons and trapped the module at its 512 MiB
    cap. The new module `rheon_feel::message` has the one renderer and the one funnel: `arg` quotes
    a value, type, context, name or text whole when its text is at most 120 characters and as a
    preview otherwise (its first 120 characters, `…` and its size: `N items`, `N entries`, `N
    characters`), and a number whose plain text is longer than 40 characters in scientific notation;
    `compose` builds a message of at most 1,000 characters, a longer one keeping its first 999 and
    `…`, and stops formatting there. `value_null!` builds every reason through `compose`, so every
    reason is bounded by construction, even at a site that quotes a value without `arg`, and builds
    it out of line, in `values::null_with_reason`, which is cold and never inlined: a reason's
    arguments and formatting take no space in the stack frame of the code that returns the null,
    often an evaluator that recursion nests deeply. The recursion of the stack check
    (`scripts/stack-headroom.mjs --report`, a body nested d levels in 190 calls) reaches at least
    the depths of 0.4.0 on every host (0.4.0 → this build, macOS arm64): the Node main thread 17 →
    18 with Liftoff and 14 → 14 with TurboFan, the 2 MiB worker 37 → 40 and 31 → 31, the 3 MiB
    worker 60 → 64 and 50 → 50, the 4 MiB worker 64 → 89 and 64 → 69, Wasmtime with 512 KiB, 1, 2
    and 4 MiB 9, 23, 52, 64 → 10, 25, 57, 98; a level of `+` takes 227 bytes of the Liftoff stack,
    243 before. The type of a value is quoted as `values::TypeOf`, which writes the text of
    `type_of()` from the value itself: a context type entry by entry, a list type from its first
    item's type built with at most 1,000 type nodes (`list<…>` when it would hold more), declared
    types as they are; `type_of()` itself is the same body with an unlimited budget. The
    `MessageArg` impls live with their types. With streaming `Display` (item 66), a message
    allocates a bounded amount, never a whole value or type. This commit caps; the sites quote
    previews from the next ones on. 5 answers change: four reasons over 1,000 characters (a list
    compared with a number, in lenient and strict mode, a missing path entry of a long context, an
    invalid pattern of 5,001 characters) are cut, and the evaluation of 1,000 such reasons, which
    trapped, answers; `scripts/smoke.mjs` checks it on a fresh instance (22 MiB of growth; 0.4.0
    traps at 500 MiB). Item 68 gives four of them previews and registers them with those answers, so
    1 registered difference is this item's: the reason of the invalid pattern, which stays cut.
    Speed against 0.4.0: the suite geomean 1.00x, 1,000 error nulls 1.00x. Basis: messages name the
    value by a short preview and its size, at most 1,000 characters each (owner decision); CONTRACT
    §3 (numbers over 40 characters in scientific notation). API breaks (rheon-feel): `message::{arg,
    Arg, compose, MessageArg, Size}`, `values::TypeOf` and `values::null_with_reason` are added,
    with `MessageArg` for `TypeOf` and for the existing `Value`, `FeelType`, `FeelNumber`, `Name`,
    `FeelContext`, `Bip`, `str`, `String` and `serde_json::Value`; `value_null!` caps its message at
    1,000 characters and builds it in a closure that it passes to `values::null_with_reason`, so
    the macro's arguments are evaluated inside that closure and can no longer use `?`, `return` or
    `break`.
68. **Values in null reasons are previews.** Every null reason of the evaluator and of rheon-feel
    quotes its values, contexts, names, texts and numbers through `arg`, and the type of a value
    through `TypeOf`: the 199 `invalid_argument_type!` sites pass the value instead of its
    `type_of()`, and so do the other type messages (operators, `between`, `string join`,
    `sublist`, `time`, the iteration and range errors). A message therefore never builds the
    type of a whole context: `abs(c)` for a 10,000-entry context allocated the 10,000-entry type
    (about 1.1 MB) for its reason and now allocates at most 64 KiB more than reading `c`. The
    reason of `(for i in 1..1000 return i) = 1` is `equal err '[1, 2, 3, …, 32, … (1000 items)'
    =?= '1'` (the first 120 characters of the list, then its size), a missing entry of a long
    context quotes the context's preview with `… (1 entry)`, and a number whose plain text is
    longer than 40 characters is in scientific notation in every reason, as CONTRACT §3 already
    said for `sublist` and `substring`: `1e50 / "a"` gives `[division] incompatible types: 1E+50
    / "a"` (and two pinned tests, a duration divided by 1e-50, now read `… / 1E-50`). A short
    argument (a number, a boolean, a temporal value, a string of at most 118 bytes) is written
    straight, with no preview to watch it, and a long one streams its first 120 characters, so
    quoting allocates nothing. Speed against 0.4.0: the suite geomean 1.00x; 1,000 reasons that
    each quote two values and their types 0.93x (each operand now passes the renderer), 1,000
    reasons quoting a list or a context 1.06x and 1.16x. 5 registered differences: four answers
    of item 67, now with previews and registered here with them (the reasons of a list compared
    with a number in lenient and strict mode and of a missing path entry, and the evaluation of
    1,000 such reasons), and `1e50 / "a"`; the reason of a 5,001-character invalid pattern stays
    cut and registered with item 67, since the regex library quotes it. Basis: messages name the
    value by a short preview and its size (owner decision); CONTRACT §3. No API change beyond
    rheon-feel's `MessageArg::is_short` (with a default) and `MessageArg` for references and
    `FeelString`.
69. **A syntax error quotes a preview of the input.** The parser's syntax error echoed the whole
    input, so a 40,002-character expression with a trailing operator gave a 40,016-character
    `syntax` error, for `evaluate` and `parse` alike. `err_syntax_error` and
    `err_not_a_feel_name` now quote the input through `arg`: whole up to 120 characters, else its
    first 120 characters, `…` and `(40002 characters)`. Every shorter input keeps its full echo.
    2 registered differences (the evaluate and the parse request). Basis: messages name the input
    by a short preview and its size (owner decision); CONTRACT §3. No API change.
70. **Values in DMN messages are previews.** The item-definition, variable, boxed-expression and
    invocable messages of rheon-model-evaluator quoted whole values and built whole types: an
    input of the wrong type quoted all of it twice, as `expected type 'string', actual type is
    '<its whole type>' in value '<its whole text>'`. They now quote through `arg` and `TypeOf`, so
    `{Customer Name: for i in 1..1000 return i}` against a string item definition gives
    `expected type 'string', actual type is 'list<number>' in value '[1, 2, 3, …, 32, … (1000
    items)'`, and a 10,000-entry context gives the previews of its type and its text, each
    ending `… (10000 entries)`. The strict-mode text of `evaluate_invocable_with_mode` is built by
    `compose`, at most 1,000 characters. Native only (DMN is not in the wasm module); the TCK
    results are identical. Basis: messages name the value by a short preview and its size
    (owner decision). No API change.
71. **Request echoes are previews and every message is capped.** The wasm layer quoted request
    values whole: an invalid `dialect`, `language`, `errorMode`, `contextFormat`, `op` or
    `tzOffsetMinutes` was echoed in full, a JSON number's text and the path of a value in the
    input messages too, and a rounded input number in its plain digits
    (`a[0]: number 1.5e-6176 is too small for a 34-digit decimal and was rounded to 0.<6175
    zeros>2`). They now quote through `arg` (`invalid 'dialect' "xxx…… (200 characters) (expected
    "expression" or "unaryTests")`, `… was rounded to 2E-6176`); the path of an input value is
    written as it goes, never built as a text, and a long one is a preview with its size in
    characters in every message; the FEEL-context error quotes its value and `TypeOf`, and every
    `error.message`, warning and strict-mode text goes through `compose`, so no message of a
    response exceeds 1,000 characters (CONTRACT §3 states the rule). 11 registered differences: the
    echoes of a long `dialect` and `contextFormat` (each for evaluate and parse), `op` and a
    1,000-digit `tzOffsetMinutes`, the empty-key error under a 2,000-character key (evaluate and
    parse) and the number warning under it, a 5,002-character number and the rounded `1.5e-6176`;
    the identity check sends these numbers as written. Basis: messages name the value by a short
    preview and its size (owner decision); CONTRACT §3. No API change.
72. **rheon-feel-regex tells engine limits from invalid patterns.** The regex engines give up on
    some valid patterns: fancy-regex's backtracking budget and stack, the compiled-size limit of
    the automata, and the nesting limits of the parsers (64 levels in fancy-regex, 250 in
    regex-syntax). Every such failure was reported as an invalid pattern. The crate now has a
    typed `RegexError`: `Invalid` (an invalid pattern or flags, XQuery F&O FORX0001-FORX0004,
    whose FEEL result is null) or `LimitExceeded(Limit)` with `Limit::{Backtracking,
    BacktrackingStack, CompiledSize, Nesting}`. A pattern is called invalid only when the engine
    finds it invalid within its limits; one the engine cannot read within them is a limit:
    fancy-regex's recursion error, a build error with a size limit or a nest-limit syntax error,
    and every run-time failure are limits; the `regex` crate's `CompiledTooBig` is one, and on
    its syntax-error path the pattern is parsed again with the engine's defaults to tell a
    nesting limit apart. So a pattern that nests past fancy-regex's limit and is invalid further
    on (300 `(` and then `[`) is a nesting limit in `matches`, whose parser stops at the limit,
    and invalid in `replace` and `split`, whose parser reads the whole pattern before it checks
    the nesting. `replace` and `split` move into the crate with the replacement-group rewriting,
    the `q` quoting and the fallback to the pattern without flags when the flagged pattern is
    invalid, as they were; a limit on the flagged pattern is reported at once instead of falling
    back, since the flags apply to the pattern (XQuery F&O 3.1 §5.6.3, `fn:replace` takes the
    flags of `fn:matches`): `replace("x", "k{100000}", "y", "i")`, where only the flagged pattern
    exceeds the compiled-size limit (case folding gives `k` the Kelvin sign as a third form), was
    the answer of the pattern without flags, `"x"`. That answer change is registered with item 73,
    which gives it its final form. The built-ins keep their argument checks and messages, and
    every other FEEL answer is unchanged in this commit (the identity check includes the limit
    inputs). API breaks (rheon-feel-regex): `RegexError`, `Limit`, `replace` and `split` are added,
    and `regex` and `regex-syntax` become dependencies (both already in the build graph);
    rheon-feel-evaluator no longer depends on `regex`.
73. **A regex engine limit aborts the evaluation.** `matches`, `replace` and `split` turned an
    engine limit into the null of an invalid pattern, which B-FEEL then replaced by `false`, `""`
    or `[]`: `matches("aaaa…!", "^(a|a)*\1$")` (40 `a`) was null with `invalid regex pattern`,
    the same `or true` in B-FEEL was `true`, `split("a", "\w{10000}")` in B-FEEL was `[]`, and a
    DMN decision silently did not match. An implementation limit is not an XQuery F&O error, so
    DMN 1.6 Table 75 note 2's null does not apply, and chapter 11's defaults replace FEEL nulls,
    which an abort is not. A `LimitExceeded` from rheon-feel-regex now stops the evaluation
    through the abort channel (item 65) with `<function>: the pattern '<preview>' exceeds the
    regex engine's <limit>; evaluation stopped`; an invalid pattern keeps its null and text. The
    wasm answer is an `evaluation` error, a `context` error inside a FEEL-format context, and DMN
    reports the abort (`Outcome`, the server's `errors`). The backtracking limit of item 22
    therefore ends the evaluation as an error. CONTRACT §2 and §3 list the new cause. 18
    registered differences: the compiled-size, nesting and backtracking limits of the three
    functions, with and without flags, in FEEL and in B-FEEL, among them two inputs of the
    no-trap corpus; `replace("x", "k{100000}", "y", "i")`, where only the flagged pattern exceeds
    the limit (item 72), was `"x"`; and `matches` of 300 `(` and then `[`, invalid beyond the
    nesting limit, was the null of an invalid pattern. Basis: DMN 1.6 §7.3.8 and Table 75 note 2;
    CONTRACT §3 (a stopped evaluation is reported, never a partial result). API break
    (rheon-feel-regex): `is_match` and `is_match_with_flags` return `Result<bool, RegexError>`.
74. **Exact fractional seconds.** The time, date and time and days and time duration parsers read
    the fraction digits as a binary double and truncated the product with 10^9, which is one
    nanosecond low for many decimal fractions: of all fractions with 4, 5 and 6 digits, 167 of
    10^4, 1,710 of 10^5 and 17,066 of 10^6 (none with 1 to 3 digits), so `time("10:00:00.0157")`
    was `10:00:00.015699999`, `duration("PT0.000015839S")` was `PT0.000015838S` and
    `time("10:00:00.0157").value` was `36000.015699999`. A fraction that the double rounds to 1
    gave 10^9 nanoseconds: `time("10:00:00.99999999999999999")` showed the malformed
    `10:00:00.`, `…T23:59:59.99999999999999999Z` showed `…T23:59:59.Z` and
    `duration("PT1.99999999999999999S")` was `PT2S`. The digits are now read as an integer in one
    function shared by the three parsers: the first nine are the nanoseconds, the digits beyond
    them are truncated, as before. `date and time(date, time, zone)`, which formats its parts and
    parses the text again, is exact with it. Tests: every fraction of up to six digits and 100,000
    of seven to nine digits against the integer of their digits. Basis: DMN 1.6 §10.3.2.3.4,
    §10.3.2.3.6 and §10.3.2.3.7, a time, a date and time and a days and time duration are values of
    the XML Schema time, dateTime and dayTimeDuration datatypes, whose seconds are decimals. 1,508
    registered differences, each the exact digits of a fraction the double got wrong. No API
    change.
75. **Every time of day is below 24:00:00.** The time constructors checked the hour, minute and
    second but not the nanoseconds, and 24:00:00 was accepted with any fraction:
    `time("24:00:00.5")` and `@"24:00:00.5"` were `00:00:00.5`, `date and time("2020-01-01T24:00:00.5Z")`
    was `2020-01-02T00:00:00.5Z`, and `FeelTime::local_opt(10, 0, 0, 1_000_000_000)` built a time
    whose text was `10:00:00.`. A time of day now has an hour below 24, a minute and a second below
    60 and nanoseconds below 10^9, or is 24:00:00 with a zero fraction, which is midnight; one
    constructor checks it and every other one goes through it, `utc_opt` included, which kept an
    hour of 24 as it was. So these three are null with the reasons of other invalid literals,
    while `24:00:00`, `24:00:00.000` and `2020-12-31T24:00:00Z` (2021-01-01T00:00:00Z) are
    unchanged. The clock time of `now()` folds a leap second, which chrono gives as nanoseconds
    from 10^9 on, into the last nanosecond of the second before. Basis: XML Schema Part 2 §3.2.7,
    "'24' is permitted if the minutes and seconds represented are zero"; DMN 1.6 §10.3.2.3.4, a
    time is a value of the XML Schema time datatype. 3 registered differences. API breaks
    (rheon-feel-temporal): the unchecked `FeelTime::utc`, `FeelTime::local` and `FeelTime::offset`
    are removed and `FeelTime::midnight(FeelZone)` is added; `FeelTime::{utc_opt, local_opt,
    offset_opt, zone_opt}` and `FeelDateTime::{utc, local, offset}` return `None` for nanoseconds
    of 10^9 or more and for 24:00:00 with a fraction, and `utc_opt` turns 24:00:00 into midnight;
    the time and date and time parsers reject 24:00:00 with a fraction.
76. **`date and time(date, time, zone)` is built from its parts.** It formatted the date, the time
    and the zone as a literal and parsed that text again, so a date whose year has fewer than four
    digits, which `FeelDate` prints as `0999` or `-005` and the literal pattern does not accept,
    failed as `[core::date and time] invalid timezone 'Z'`. It now reads the zone designator alone
    with the rules of a literal's zone suffix (`FeelZone::from_designator`: `Z`, an offset within
    ±14:00, `@` and a zone name the zone database knows; never empty) and builds the date and time
    from the date and the time of day in that zone. Every input the literal could write gives the
    same answer, its failures the same `invalid timezone` reason; `date and time(date(999,12,31),
    time(23,59,0), "Z")` is now `0999-12-31T23:59:00Z`, the value `date and time(d, t)` already
    gave with a zoned time, and year 0 and the negative years above -1000 likewise, as
    `date(0,1,1)` gives them. Basis: DMN 1.6 §10.3.2.3.6, "The year must be in the range
    [-999,999,999..999,999,999]"; §10.3.4.1 Table 73. 4 registered differences (years 999, 0 and
    -5). API break (rheon-feel-temporal): `FeelZone::from_designator` is added.
77. **Fractional seconds truncated to nanoseconds are reported as warnings.** A time, date and
    time or days and time duration holds nanoseconds, so the digits beyond the ninth were dropped
    without a word: `time("10:00:00.1234567891")` is `10:00:00.123456789`. They are still
    truncated, and the evaluation now says so: the diagnostics collection of `rheon_feel` keeps a
    list of warnings beside its errors, filled by `diagnostics::warn` while a collection is active,
    with the same rules as the error messages (a repeat in a row once, at most 64, restored when a
    collection ends, also on a panic) but neither counted nor read by strict mode, since a
    truncation is no semantic error (DMN 1.6 §7.3.8 makes semantic errors the reason to report and
    return null). The three temporal parsers report the digits they dropped
    (`parse_reporting_truncation`, which `FromStr` and `TryFrom` call), and every producer quotes
    the literal or text through one message, `fractional seconds of '10:00:00.1234567891' truncated
    to nanoseconds; dropped digits: '1'`, built with `arg`, so a 100,000-digit fraction gives a
    message of a few hundred characters: an `@"…"` literal (parsed once when the expression is
    built, warning on each evaluation), `time`, `date and time` and `duration` of a string, and the
    `xsd:time`, `xsd:dateTime` and `xsd:duration` conversions of typed input. Only digits that hold
    a value warn: `10:00:00.1234567890` does not. The wasm module appends the evaluation's warnings
    to the response's `warnings` after those of the input and the unary tests, including the
    warnings of a FEEL-format context; `diagnostics.count` and strict mode are unchanged, and an
    evaluation stopped by a limit still answers its error without warnings. Native callers read
    them from `Diagnostics::warnings`, also from `ModelEvaluator::evaluate_invocable_with_mode`.
    CONTRACT §3 lists the warning. Basis: the owner rule that more than nine fraction digits are
    truncated to nanoseconds with a warning naming the literal and the dropped digits, never
    rounded or rejected; CONTRACT §3. 23 registered differences (the warnings of literals and texts
    with more than nine digits, in an expression, a loop and a FEEL-format context), and the 7
    entries of item 74 with more than nine digits now also hold their warning. API breaks
    (rheon-feel): `Diagnostics` has a private `warnings` field with the accessor `warnings()`, so
    its struct literals no longer compile outside the crate; `diagnostics::warn`,
    `values::truncated_fraction_warning` and `values::warn_truncated_fraction` are added;
    (rheon-feel-temporal) `FeelTime::parse_reporting_truncation`,
    `FeelDateTime::parse_reporting_truncation` and `FeelDaysAndTimeDuration::parse_reporting_truncation`
    are added.
78. **`time()` seconds with more than nine fraction digits warn.** `time(hour, minute, second[,
    offset])` takes the nanoseconds of an exact decimal seconds number and truncated the digits
    beyond the ninth silently, while the same truncation of a literal warns (item 77). It now
    warns with the same message, naming the seconds number as `arg` quotes it and the digits it
    lost: `time(10, 0, 0.1234567891)` gives `fractional seconds of '0.1234567891' truncated to
    nanoseconds; dropped digits: '1'`, and a computed number warns too (`time(10, 0, 10/3)`, 24
    dropped digits). The value is unchanged; no warning when every dropped digit is 0 or when the
    call gives no time (an offset out of range). The split of the seconds into nanoseconds and
    dropped digits is one function for both arities. Basis: the owner rule that a truncation to
    nanoseconds warns, for literals and `time(h, m, s)` alike, and only when a digit other than 0
    is dropped; CONTRACT §3. 2 registered differences. No API change.
79. **The elements of a model nest at most 100 levels deep.** The XML parser (roxmltree's tokenizer
    recurses once per element), the schema validation, the model parser and the evaluator builders
    all follow the element tree recursively, and nothing bounded it: a model with 100,000 nested
    `itemComponent`s or boxed contexts overflowed the stack inside the XML parser and aborted the
    process. `rheon_model::parse` now reads the text once, without recursion, before the XML parser
    runs, the way the tokenizer reads it (start, end and empty-element tags change the depth;
    comments, CDATA sections, processing instructions, quoted attribute values and text do not;
    where the tokenizer stops with an error, so does the check), and rejects a document whose
    elements nest deeper than 100 levels, the root element being level 1, with `<ModelParserError>
    elements are nested more than 100 levels deep`. Every later recursion over the elements is
    bounded by it. The bound is the stack proof: a model nested to it is parsed, built and evaluated
    on a 2 MiB thread of the unoptimized test build (nested boxed lists and boxed contexts; tests at
    the bound and one past it). The measured ceiling on that thread is 124 levels (aarch64, rustc
    1.95.0): the XML parser alone overflows it from 125 levels on, and parsing, building and
    evaluating a model all pass at 124. The bound keeps a margin of about a quarter below that
    ceiling, rather than the 256 levels asked for or the measured 124, because CI runs the at-bound
    tests on x86_64 Linux, whose ceiling could not be measured; the deepest model of the repository
    and the TCK nests 12 levels. Old and new (native only; the wasm module parses no models): a
    model nested 101 to 124 levels deep parsed and now gives the error; one nested deeper aborted
    the process and now gives the error; a malformed document nested more than 100 levels deep
    reports the nesting, not the XML error found later in the text. Basis: the owner rule that
    robustness comes first (a model file must not abort the host); the owner decision to keep the
    bound at 100 with that margin; DMN 1.6 defines no nesting limit, so the bound is an
    implementation limit, reported as an error. API break (rheon-model): `parse` rejects a document
    whose elements nest more than 100 levels deep.
80. **Item definitions that form a cycle are a build error naming the cycle.** The type, value
    and context evaluators of item definitions follow their type references at every use,
    recursively, and the only guard was a cycle validator in `rheon_model::parse` with four
    defects: it added a second node for a reference placed before its definition, so a cycle of
    two or more definitions (`tA{x: tB}`, `tB{y: tA}`) was never closed; it had no edges for
    function items (`tF` with `outputTypeRef="tF"`); it checked one file, while the evaluators
    resolve over all models of a build (two models with one namespace); and its search recursed,
    so a long chain of definitions overflowed in the validator itself. Each of these aborted the
    process with a stack overflow. The validator is removed, and one check runs at the start of
    the build, before anything follows a reference, over the item definitions of all models: one
    node per key (the last definition added under it, as in the evaluators), edges from the
    references the evaluators follow (the type reference, the components' type references at any
    depth, a function item's parameter and output types; the rule for function items is now one
    function, `referenced_item_definition`, shared with the type evaluator), and an iterative
    depth-first search that names the first cycle it closes:
    `<ItemDefinitionError> item definitions form a cycle: tA -> tB -> tA`, each name quoted by
    `arg` and the message capped at 1,000 characters, so a ring of 10,000 definitions is named
    in a bounded message. A reference into an imported model is not followed by the evaluators
    and has no edge. Old and new (native only; the wasm module builds no models): the cycles above,
    a 3-cycle through type references, a function taking its own type, a cycle through a nested
    component, two models with one namespace and a 10,000-definition ring aborted the process and
    are now that error; `DMN_N_0088` and `1001.dmn` (a self-referencing `iTreeNode`) failed
    `parse` with `<ModelValidatorError> cyclic dependency between item definitions`, so a
    workspace skipped only that file, and now parse and fail the build with
    `item definitions form a cycle: iTreeNode -> iTreeNode`, so their workspace fails to deploy
    like any other build error. Basis: DMN 1.6 §7.3.3 (a type reference names an item definition
    or a built-in type; no recursive types are defined, so recursive item definitions stay
    rejected); the owner rule that robustness comes first. API breaks: (rheon-model) `parse` no
    longer rejects item definition cycles, the `<ModelValidatorError>` kind is gone and the
    `petgraph` dependency is removed; (rheon-model-evaluator) `ModelEvaluator::new` returns the
    new `<ItemDefinitionError>` for a cycle.
81. **An item definition type beyond the size or depth bound is a build error.** The type
    evaluator builds a definition's type as a tree and expands a referenced definition at each
    use, so 30 definitions that each use the next one twice built a type of 2^31 nodes (the
    process ran out of memory) and a chain of 100,000 definitions, each a context of the next,
    overflowed the stack while it was built. The graph check of item 80 now also computes, in
    the order its search finishes the definitions and without building any type, the number of
    nodes and the depth of each definition's type, by the rules the type evaluator builds by: a
    built-in type is one node; a context is one node and its entries, a list one node and its
    item type, a function one node, its parameter types and its result type; a referenced
    definition counts its whole type at each use; a reference to no definition counts one node.
    Depth: a built-in type is 1, a context, list or function one more than its deepest part. A
    type of more than 100,000 nodes is `<ItemDefinitionError> item definition 't16' expands to more
    than 100000 type nodes`, one nested more than 256 levels deep `item definition 't1' nests
    types more than 256 levels deep`, naming the first definition finished that exceeds a bound.
    Tests check that the counts are those of the built types, and that a type of exactly 100,000
    nodes and one exactly 256 levels deep build, and that an input of each is checked, evaluated
    and tested with `instance of` on the default 2 MiB thread of the unoptimized test build (the
    deep one overflows that thread from about 600 levels on, aarch64, rustc 1.95.0). The largest
    type of the repository and the TCK models has 51 nodes and the deepest 6 levels. Old and new
    (native only): 17 levels of doubling (a type of 131,071 nodes) built and now give the node
    error for `t16`, and 30 levels exhausted memory and now give it at once; a chain of 257
    definitions built and now gives the depth error for `t1`; a chain of 100,000 aborted the
    process and now gives it for `t99744`. What remains: the bounds hold per definition, so a type
    is still expanded at each use and the global type context, cloned for each decision
    evaluation, grows with the models times the node bound; sharing named types is later work.
    Basis: the owner limits for item definition types (100,000 nodes, 256 levels), each with its
    stack proof; DMN 1.6 defines no type size limit, so the bounds are implementation limits,
    reported as build errors. API break (rheon-model-evaluator): `ModelEvaluator::new` returns
    `<ItemDefinitionError>` for a type over 100,000 nodes or 256 levels.
82. **Decision-service function values hold a weak model reference.** Each decision service is
    also a function value, stored in the model evaluator, and its body held an `Arc` of that
    model evaluator, so every model with a decision service referred to itself and was never
    freed: a host that reloads models (the server, a workspace) kept every version. The body now
    holds a `Weak` reference and upgrades it for each call; during an evaluation the caller holds
    the model, so the upgrade succeeds and every answer is unchanged (TCK and the compatibility
    tests of 0085-0088, 0092 and 1130). The invariant, in the module and `FunctionBody` docs: a
    value never holds a strong handle to the state that holds it. Old and new (native only): a
    model built from `3_0085.dmn` stayed alive after its last `Arc` was dropped and is now freed;
    a decision service value called after its model was dropped evaluated the service with the
    leaked model and is now `null(decision service decisionService_001 cannot be called after its
    model was unloaded)`. The remaining step, the invocation taking the model from the call's
    session instead of from the value, is later work (BACKLOG). Basis: DMN 1.6 §10.4 ("A
    decision service is semantically equivalent to a FEEL function"); the owner rule that memory
    never accumulates beyond a request. API break (behaviour, rheon-model-evaluator): a
    decision-service function value called after its model was dropped is null with that reason.
83. **The wasm display measures the width of a line before rendering it.** `display` writes a list
    or context on one line when that line fits 80 columns, else expanded. It built the one-line
    text of every container to compare its length and kept it while it rendered the children, so
    a result nested d levels deep around a string of s bytes held about d × s bytes at once: a
    2 MiB string inside 280 lists filled the 512 MiB memory cap and trapped the module. Now one
    one-line writer writes either into the output or into a measure that counts the bytes and
    stops once they pass the width left on the line (strings and keys are counted as serde_json
    escapes them, without a copy, and a string longer than that width stops the measure at
    once), and each node is written once. The output is byte-identical: `indent + width <= 80`
    holds exactly when `width <= 80 - indent`, and past indent 80 no container fits, as before;
    tests pin the 80/81-byte boundary at indent 0, 2 and 78, a width counted in bytes, the
    escapes of strings and keys at the boundary, empty containers and nesting past indent 80.
    `scripts/smoke.mjs` evaluates the 280-level probe on a fresh instance: the 0.4.0 module
    traps after growing by 498 MiB, the new one answers with the expanded display and grows by
    26.5 MiB (bound 32 MiB). Speed against 0.4.0 (engine benchmarks): geomean 1.00x, larger
    results 1.05-1.10x (`ctx200-*`), every workload at 0.96x or above. 1 registered difference:
    the probe, which trapped and now answers. What remains: a scalar other than a string (a
    range with long string endpoints) is still rendered to text when it is measured, at most
    once per enclosing level within 80 bytes of it. Basis: CONTRACT §3, the `display` rule;
    the owner rule that robustness comes first. No API change.
84. **No combination of edge operands traps the module.**
    `wasm/feel-wasm/tests/corpus/edge-operands.json` is a deterministic product of expression
    templates over 38 operands at the engine's edges: numbers at the limits of the 64-bit and 32-bit
    integers and of decimal128 (`1E+6144`, `-9.999999999999999999999999999999999E+6144`, `1E-6176`),
    both duration kinds at the ends of their range and at one unit, dates and date and times at the
    ends of the supported range and in a daylight saving gap and overlap, times at the ends of the
    day, null, `""`, `[]` and `{}`. The contract test `edge_operand_products_never_trap` applies
    each of the 15 unary templates (the value, `string`, `abs`, negation, the duration and date
    properties, `.value`, `= itself`, `floor`, `decimal`) to every operand and `+`, `-` and `*` to
    every ordered pair, 4,902 requests, with the checks of the no-trap corpus (commit 55) after each
    one; with `RHEON_FULL_TESTS` set (natively only, Wasmtime passes no environment) it adds `/`,
    `<`, `=`, `modulo`, `max` and `**`, 13,566 requests. It is the seed set of combinations for a
    later fuzzer, where the no-trap corpus holds single requests. Against the engine before commit
    59 the test fails natively on its first trapping request, `duration("-PT0.000000001S") + date
    and time("-262143-01-01T00:00:00Z")` (an `internal` error, ``DateTime - TimeDelta`
    overflowed``), and aborts the test binary under Wasmtime with that request as the last
    `no-trap:` line; on this tree it passes in both, and the full product passes under Wasmtime too
    (checked with the switch forced on). Run time: 0.8 s native debug, 1.0 s under Wasmtime; full
    size 0.24 s native release, 2.3 s native debug and 2.9 s under Wasmtime. Answers are not
    asserted, and the product is not a source of the identity check: its full size's 13,528 distinct
    requests through 0.4.0 and this build differ in 568 answers. The 24 that are not message changes
    are registered one by one, with both answers, in the ledger of the change that makes them: 7
    sums, differences and products of exactly -2^63 nanoseconds, which 0.4.0 printed 1 ns off, and 3
    of exactly -2^63 months, which trapped, are null (item 59); 5 date and times at the first
    supported instant minus a duration, which trapped, are null (item 61); 6 dates minus one
    nanosecond are the day before, and 3 at the first supported date are null (item 62, Table 58).
    The other 544 are null reasons that quote `1E+6144`,
    `-9.999999999999999999999999999999999E+6144` or `1E-6176` in scientific notation where 0.4.0
    wrote up to 6,177 plain digits: instances of the registered message rule (item 68, CONTRACT §3),
    not listed one by one. Basis: no input may panic the engine (CONTRACT §1: on wasm a panic aborts
    the instance). No API change.

Commits 85-133 make answers follow DMN 1.6 where 0.4.0 gave a wrong one without a report: numbers
(one backend, one canonical text, exact rounding, modulo and integer powers), time offsets and the
temporal properties, the nulls that the specification defines, `if`, `!=`, the null operands of
arithmetic, unary comparisons with a path endpoint, allowed values of item definitions, iteration
contexts, and the flags, replacement strings and zero-length patterns of the regex built-ins. Tools
come first: the identity check answers and caches the reference per source, a number oracle judges
every number result against exact arithmetic, and frozen request corpora pin the number edges and
the regex built-ins; the last commit makes CI document the published crates with rustdoc's warnings
denied. Each change of a wasm answer against 0.4.0 that the identity check finds is registered with
both answers and counted in its item; changes that only DMN or the Rust API show are stated in
their items with both answers. The totals are under "Verification" below.

85. **Automated checks of the identity tool.** `tools/identity/test.mjs` (`npm run test:identity`,
    run in CI after the identity step) checks `equal.mjs` on two real modules, the previous release
    and the build (`IDENTITY_REFERENCE`, `IDENTITY_CANDIDATE`; a missing module fails the tests): a
    module against itself exits 0 and leaves a `today()` request and its parse twin out; a
    request that traps both sides is `{"trap": true}` on both and the next request runs on a fresh
    instance; requests of the committed ledger with a 40-digit integer and `1.5e-6176` added to
    their context differ between the modules and exit 1, `--record` drafts them with those numbers
    as written, and the draft passed as a ledger exits 0; the same draft exits 1 when the candidate
    does not make the change and when the reference does not answer `old`; an entry with a wrong
    `old` or `new` exits 1; a context number beyond a double is sent as written; two requests that
    differ only beyond a double's precision are two requests; thirteen kinds of malformed ledger,
    a request registered twice and six usage errors exit 2 with their message. The checks that need
    a difference between the modules skip with a message when the two modules answer every request
    of the committed ledger alike. The checks found one defect: a JSON number where the ledger
    expects an entry or a response object was taken for an object, so `"old": 2` was accepted as a
    response that never matched (exit 1 instead of 2) and a number entry was reported as an
    unknown field `rawJSON`; both are input errors now, through one predicate for "a JSON object"
    that the canonical form uses too. A planted defect in the tool (the `old` check skipped, lossy
    JSON, the clock exclusion off) fails the checks. No API change.
86. **One module session for the Node tools.** `scripts/feel-abi.mjs` holds the raw ABI once:
    `callText` returns the response text, decoded as strict UTF-8 (bytes that are not UTF-8 throw
    `MalformedResponse`), `call` parses it, and `session(module)` answers request texts one after
    another and, after a trap, resolves to `{ trap }` with the error and continues on a fresh
    instance. The identity check (`tools/identity/equal.mjs`) and the Node host of the stack check
    (`scripts/stack-headroom.mjs`) used their own copies of that loop; both use the session now.
    The smoke test's `call` now rejects a response that is not UTF-8 instead of decoding it with
    replacement characters, and the stack check fails on a response that is not JSON instead of
    reporting it as a trap. Checked: the identity tool's checks (item 85) pass before and after,
    the identity run prints the same lines, `npm run smoke` and `npm run stack` pass, and the
    stack report is the same with the old and the new scripts. No API change.
87. **The identity check answers the reference per source.** A unit is one source file (its
    requests, then the parse twins of its `evaluate` requests, without the clock requests) or one
    ledger file (the `request` of each entry). The reference answers each unit in order on a fresh
    instance, so its answers to a unit are a function of the module and the unit, which a cache of
    them needs; a request that several units list takes the answer of the first. The candidate
    answers the work list on one instance as before, and the work list, the comparison, the
    output and `--record` are unchanged. Responses are held and compared as their canonical text
    (sorted keys, every number as written), parsed again only where a result text or a draft
    needs the object. A request in several units is now evaluated once per unit by the
    reference: on the committed sources 2,627 requests as before, with 50 module panic messages on
    stderr instead of 30 and the same stdout; the time is unchanged (8.1 s and 8.2 s). Checked: a
    new test runs two sources in both orders and gets the same summary and the same failure lines
    (it passes before the change too: it pins the property that the reference keeps no state
    between requests); the identity tool's checks pass; a `--record` draft of 40 registered
    requests is byte-identical to the one the tool wrote before. No API change.
88. **The identity check caches the reference's answers.** `equal.mjs --cache <dir>` keeps the
    answers of the reference to each unit (item 87) in
    `<dir>/<reference sha256>/<16 hex of the unit path's sha256>-<name>.jsonl`, a header, one line
    per request (the sha256 of the request text and the canonical answer, or a trap) and a trailer
    with the sha256 of every byte before it. A file is used only when the digest is right, the
    header equals the run's (format, reference, unit bytes, unit kind, `--parse-too`, the sha256 of
    `equal.mjs` and `scripts/feel-abi.mjs`, Node's version, architecture and `execArgv`) and the
    request hashes are the unit's in order; otherwise the unit is evaluated and the file written
    again through a rename, and an existing file is reported as rejected with the reason. A trap
    depends on the host, so every cached trap is evaluated again on a fresh instance, and one that
    answers rejects the file. A unit with a response that is not JSON is not cached. A new
    reference directory removes the directories of other references (only directories named by a
    sha256), so there is at most one file per unit path; deleting the cache is always safe. The
    candidate is never cached. `--verify-cache` evaluates the reference in full, fails with
    `CACHE MISMATCH` on any cached answer that differs and rewrites the files. The summary line
    ends with `reference: <n> answers from the cache, <m> evaluated`. `run.sh` passes `--cache
    .tools/identity-cache`; CI restores no cache, so its run is uncached. Before the change the
    four new tests fail with exit 2 (`unknown option --cache`). They check that no cache, a cold
    cache and a warm cache give the same exit status, the same output apart from those counts and
    the same `--record` draft, for identical modules, a trapping request, a trap entry, entries
    with a wrong `old` or `new`, unregistered differences and their draft replayed; that a flipped
    byte, a cached trap the module answers, another `--parse-too`, an edited source and a file that
    is not a cache file are rejected and evaluated; that another reference gets its own directory
    and removes the old one; and that a planted wrong answer with a valid digest misleads a cached
    run until `--verify-cache` reports it, exits 1 and rewrites the file. Measured on the
    committed sources (2,627 requests, 30 of them trapping 0.4.0): 8.4 s cold, 7.5 s warm, 8.4 s
    with `--verify-cache`; the warm run answers 2,701 reference answers from the cache and evaluates
    the 50 trapping ones again, which take 5.6 s, two memory exhaustions of 0.4.0 most of it. No
    API change.
89. **Finding the commit behind an identity difference.** `tools/identity/bisect.sh
    <reference.wasm> <entries.json>` is the step of `git bisect run`: it builds `dist/feel.wasm` of
    the checked-out commit and runs `equal.mjs` with the entries (a `--record` draft, or a part of
    it) as the only ledger and no source, with the reference's answers cached. Every entry
    reproduced is bad (exit 1), any other outcome good (exit 0); a failed build or an input error
    of `equal.mjs` skips the commit (125), and a usage error ends the bisect (128). It runs the
    `equal.mjs` next to it and builds the work tree it runs in, so a copy kept outside the tree
    bisects commits older than the script; a third argument checks a given module without a
    build. Before the script exists the new test fails (bash exits 127); it checks that entries
    recorded from the reference against the candidate are bad with the candidate, good with the
    reference, skipped with a module that cannot be loaded, and that a usage error exits 128.
    `tools/identity/README.md` gains "While developing": the committed sources after every build,
    the larger sets once per group of changes, `--record` then bisect on a failure,
    `--verify-cache` before a release, with the measured times (against 0.4.0: committed sources
    8.4 s uncached and 7.5 s cached; with the sets kept outside the repository, 187,677 requests,
    7.0 min uncached, 6.5 min cached and 7.1 min verifying; 0.4.0's trapping requests, evaluated
    again on every run, take most of the reference's time). Checked with a real bisect between
    0.4.0 (good) and the tree before commit 85 (bad), 31 commits, with the entries of
    `registered/sort-own-order.json` and a copy of the script outside the tree: five steps, each a
    wasm build, ended at `c6a7a69` (item 64), the change that registered them. No API change.
90. **A number oracle that judges every number result.** `tools/number-oracle/run.sh [quick|full]
    [module.wasm]` sends requests to a `feel.wasm` and judges each number result against its truth
    under DMN 1.6, computed in Python (`decimal` with a decimal128 context, exact rationals, and
    mpmath 1.3.0 pinned by hash in `requirements.txt`): every operation applied to the exact values
    of its operands and rounded once to 34 digits, half even, with gradual underflow; null for no
    value (§10.3.2.3.1); the scale functions by their rounding modes within -6111..6176, a scale
    without its fractional part (Table 77, footnotes 1 and 2); `modulo` by its formula over exact
    values; `odd` and `even` by the parity of an integer, false for any other number; `**`
    correctly rounded, with exponents within ±999,999,999 (Table 61); `number()` by footnote 1 of
    Table 73; and one canonical text per value (plain digits, no trailing zeros, no `-0`) in
    `display`, `feel`, `typed` and `string()`.
    `exp`, `log` and fractional `**` are decided by an interval rule (mpmath at rising precision
    with guard digits for the condition, both ends of the enclosure rounded once, a rounding
    boundary such as a tie decided exactly), and `exp` and `log` must also agree with Python's
    correctly rounded functions. 188 families, 2,522 fixed edge vectors in literal form, and a
    random part per family sent as JSON numbers in the context, 200 cases each in a quick run and
    20,000 in a full one. `known-wrong.json` lists the classes (family glob and verdict) the engine
    is known to get wrong, each with its reason; any other wrong answer fails the run, and so does
    an entry that no failing edge vector matches, so a change that repairs a class must delete its
    entry. `scripts/feel-answers.mjs` answers a file of requests with a module, one response line
    per request. The oracle replaces the differential tools as the judge of the number core: they
    compared the two backends and, for the scale functions, re-ran the engine's own algorithm, so
    they could not see a defect both backends share. Before the truths, `oracle.py --self-test`
    (Table 77's examples, the conversion examples of Table 73 and §10.3.2.3.1, Table 40's values,
    the domain rules, and second formulations of `modulo`, integer powers and `sqrt`) fails on
    every planted wrong truth but one that agrees on every case (integer powers through Python's
    own `power`). With an empty exemption list the quick run against the build fails: 4,354 of
    35,924 cases in 142 classes, all of them known defects of the number core: a zero printed with
    a minus sign (430 cases), trailing zeros of the scale (592), the rounding functions with a
    scale (null for a zero or a tiny number, 1,922; wrong values for subnormals, near the scale
    limits and in `round half down`, 671), `modulo` beyond 34-digit quotients (248), integer
    powers that are not correctly rounded (408), exponents outside Table 61's range that are
    evaluated (60), and `string()` results that carry those texts (23). The listed classes cover
    them; the full run, 2,896,576 cases in 5.3 min, has no other wrong answer, no trap and no
    undecided case. No API change.
91. **CI runs the number oracle.** Job `wasm` runs the quick size of the oracle (item 90) on the
    build after the identity check: a virtual environment of the runner's `python3`, mpmath
    installed with `pip install --require-hashes` from `tools/number-oracle/requirements.txt`, and
    `run.sh quick` with `PYTHON` pointing into it. A planted exemption for a healthy family
    (`sqrt/perfect`, `value`) makes the same command fail as a stale entry (exit 1); without it
    the command passes, 35,924 cases in 6.4 s. The install from PyPI was not run here (no network
    on the development machine); there the environment held the same mpmath 1.3.0 files, checked
    against the wheel's `RECORD`. No API change.
92. **The number oracle's edge vectors are an identity source.** `cases.py --edges` prints the
    oracle's fixed edge vectors (2,535 requests in literal form, among them the examples of Table
    77 and Table 40 and the edge vectors that back each class of `known-wrong.json`) as a JSON array,
    and `tools/identity/requests/numbers.json` is that output, committed. The oracle's `run.sh`
    fails (exit 1) when the file is not the generator's output, so a number answer that changes on
    an edge is a registered difference, and every registered new answer is one the oracle judges.
    The oracle now sends the edge vectors of every family before the random parts, so a random
    case can never displace an edge vector of a later family and the vectors that keep an
    exemption alive are the same in every size. Against 0.4.0 one edge differs, by a rule
    registered before: the warning on a context number rounded to a subnormal quotes `2E-6176` in
    scientific notation (the 40-character rule of CONTRACT §3). Its entry is in
    `registered/number-edges-message-previews.json`; a check outside the repository confirmed that
    old and new are equal once the message texts are removed. The committed sources grow from
    2,627 to 7,359 requests (with the parse twins); the run takes 8.4 s uncached and 7.5 s cached,
    as before, since the time is the trapping requests of 0.4.0. Checked: one edited request in
    the committed file fails the oracle's run, which passed before the check; the identity tool's
    checks pass. No API change.
93. **A frozen regex request corpus.** `tools/identity/requests/regex.json` holds every distinct
    `matches`, `replace` and `split` expression that was harvested from the feel-evaluator unit
    tests (120) and from the DMN models under `examples/` and `bbt/` (77), one of them with its FEEL
    context, and 27 more requests on what the regex built-ins must settle under XQuery 1.0 and
    XPath 2.0 Functions and Operators: unknown, repeated, padded and non-string flags, `q` beside
    other flags, `x` and `#`, replacement strings with `$0`, `$10`, a lone `$`, `$$`, `${1}`, `\$`,
    `\\` and `$name`, and patterns that match the empty string in `replace` and `split`
    (224 requests). They hold no expected answers; the previous release answers them, and they
    are identical in the build. The harvested corpora they come from are removed with the
    differential tools (item 94), so the expressions are kept here, frozen. The W3C test suite of
    the regular expressions is not imported. No API change.
94. **The Intel differential tools are removed.** `tools/difftest` (the op-by-op fuzzer of the
    number core, item 11) and `tools/evaluator-corpus` (the FEEL corpus through the evaluator)
    built the number crate twice, once with Intel's BID128 library, and judged where the two
    backends disagreed; `tools/fetch-dfp-number-sys.sh` and `tools/dfp-number-sys.patch` fetched
    and patched that library for them (item 13). The number oracle (item 90) judges every number
    result of the module against DMN 1.6 instead, without a second implementation. Every
    operation of the fuzzer that a FEEL expression reaches has a family in the oracle (parsing,
    `+ - * /`, negation, `abs`, `sqrt`, `exp`, `ln` and `log`, the four kinds of `**`, `modulo`,
    `decimal` and the six rounding functions, `odd` and `even`, comparisons, formatting, and chains
    of operations through nested expressions); the rest (`%`, the assign
    operators, `trunc`, `frac`, `square`, the quantum rounding, the integer comparisons and
    conversions, `new`, the constructors from integers and the constants) are internal to the
    number core and keep their unit tests. The four FEEL corpora of the second tool (17,217
    expressions) are kept as data in `tools/feel-corpus/`, because the parser's corpus test reads
    them (`RHEON_PARSER_CORPUS`). The `tools/` ignore list keeps `__pycache__/` only, and the
    root one loses the fetched library's directory. The `intel` feature of the number crate is not
    part of this change. No API change.
95. **One number backend, no backend features.** DMN 1.6 §10.3.2.3.1 bases FEEL numbers on
    IEEE 754-2008 decimal128; the repository builds one implementation of it, the pure-Rust core
    on dashu-float, and the second one, Intel's BID128 library through `dfp-number-sys` behind
    the `intel` feature, is gone with its binding `feel-number/src/dfp_number.rs` (843 lines).
    `rheon-feel-number` has no features: the three dashu crates are plain dependencies and
    `dfp-number-sys` leaves the workspace table and `Cargo.lock`. The `default`, `dashu` and
    `intel` features that forwarded the choice go from `rheon-feel`, `rheon-feel-temporal`,
    `rheon-feel-parser` (which keeps `parsing-tables`), `rheon-feel-evaluator`, `rheon-model`,
    `rheon-model-evaluator` and the unpublished members (`dsntk-evaluator`, `dsntk-workspace`,
    `dsntk-gendoc`, `dsntk-server` and `dsntk`, the last two keep `tck`), and the workspace table
    no longer declares them with `default-features = false`. `NUMBER_BACKEND` is the constant
    `"dashu"`, so `info.numberBackend` is unchanged. The tests that held a second expectation
    for an Intel build (`feel-number/tests/pow.rs`, TCK 0100 `_0278`) keep the correctly rounded
    one. CONTRACT.md §3, the README and the engine comparison no longer promise bit-identity
    with Intel's library: they state the single rounding of §10.3.2.3.1 and point to the number
    oracle and its list of known deviations; the integer `**` caveat stays. Test first: `cargo
    check --workspace --all-features` failed on the `compile_error!` that forbade both backends
    at once; it passes now. Identity: no difference. Rust API break: the Cargo features `intel`
    and `dashu` (and `default`) of `rheon-feel-number`, `rheon-feel`, `rheon-feel-temporal`,
    `rheon-feel-parser`, `rheon-feel-evaluator`, `rheon-model` and `rheon-model-evaluator` are
    gone, so a dependency written with `features = ["dashu"]` or `features = ["intel"]` no longer
    resolves; `default-features = false` is harmless.
96. **A number has no negative zero.** DMN 1.6 §10.3.2.3.1: "There is no distinction between -0
    and 0." The number core kept the IEEE 754 sign of a zero, so `-0`, `0 * -5`, `0 / -5`,
    `round half up(-0.4)`, `round down(-0.5)`, `floor(-0.0)`, `sqrt(-0.0)` and `string(-0)`
    printed `-0` (or `-0.0`, `-0.00`, …), and `string(-0) = "0"` was false. Now the one
    constructor of `FeelNumber` (`FeelNumber::of`) clears the sign of a zero; the arithmetic on the
    unpacked value keeps the IEEE signs inside one operation, and every result passes through that
    constructor, so no zero with a sign is ever stored. The point fix that cleared the sign in
    `ceiling` without a scale is gone. Test first: `feel-number/tests/minus_zero.rs` (a parsed,
    negated, multiplied, divided, rounded, square-rooted and raised zero prints `0` and debugs as
    `+0E…`) and the contract test `a_zero_has_no_sign` (`display`, `feel`, `typed`, `string()`)
    failed on the parent with `-0`. Identity: 101 registered differences in
    `registered/no-negative-zero.json`, all on the number oracle's edge vectors: 99 texts of a zero
    that lose the minus sign, `string(-0) = "0"` (false to true) and `string length(string(-0))`
    (2 to 1). The oracle's exemptions for a negative zero and for `string(-0)` are deleted. Rust
    behaviour break: a `FeelNumber` zero has no sign: `"-0".parse()`, `-FeelNumber::zero()` and
    `0 * -5` give `+0` (Debug `+0E+0`), so `1 / -0` is `+Inf` with the operators of the crate.
97. **One canonical text for every number.** DMN 1.6 §10.3.2.3.1 represents a number as a pair
    `(p,s)` and notes that "many different pairs have the same value. For example, value(1,0) =
    value(10,1) = value(100,2)"; Table 73 has `string(1.1) = "1.1"`. The owner's decision is
    canonical values in every text form. It departs from the example texts of Table 40, which
    writes some results with their scale (`decimal(1, 2)` as `1.00`, `.10 * 30.00` as `3.0000`,
    `1.0*10**3` as `1000.0`), and keeps their values. A `FeelNumber` carried a flag beside its value: a
    literal, a JSON number, `decimal`, `abs` and the scale functions printed the trailing zeros of
    their cohort (`1.50`, `decimal(1, 2)` as `1.00`, `abs(exp(1e-34))` as 34 digits), the
    arithmetic trimmed them except in a zero (`1.10 - 1.10` printed `0.0`, `0 * 1e-10`
    `0.000000000`). The flag is gone (`FeelNumber` is the value alone, 32 bytes instead of 48),
    and `Display` writes one text per value: plain digits without an exponent, no trailing zeros
    after the decimal point, `0` for every zero. The cohort stays in the value, where only `Debug`
    shows it, and no rule depends on it. `Display` honours width, fill, alignment and the sign
    flag and ignores a precision (`{:.2}` truncated the digits instead of rounding them, and no
    caller used it). `display`, `feel`, `typed`, `string()`, JSON output and the numbers quoted in
    messages all go through it; CONTRACT §3 states the rule. Test first: the new cases of
    `feel-number/tests/to_string.rs` and the contract test `a_number_has_one_text` (with the
    zeros of `zero_with_huge_exponent_is_displayed_as_0`) failed on the parent with the padded
    texts. Pinned texts updated: `feel-number` (`to_string`, `exp`, `pow`, `round` and the unit
    tests), feel-evaluator (`string({a: 1.0, b: 2.0})`, a reason quoting `0.0`, the text of an
    interval `(1.0..5.0)`), model-evaluator (TCK 0014 `rate`, 0058 `1000000.00`, 1100 `0.0`, and
    three decision tables with `Discount: 0.10`), and the bench copies of the three TCK tests,
    checked by grep since no gate compiles the benches. The TCK runner compares decimals by value,
    so no TCK result changes. Identity: 78 registered differences in
    `registered/canonical-number-text.json`, all on the number oracle's edge vectors: 75 texts
    that lose their trailing zeros, `string(1.50) = "1.5"` (false to true) and two string lengths;
    27 entries of `registered/no-negative-zero.json` (item 96), whose new text was a zero with
    trailing zeros, are amended in place to `0`. The oracle's exemptions for texts with trailing
    zeros and for the `string()` results that carried them are deleted. Rust API break:
    `FeelNumber`'s `Display` (and so `to_string()` and `Jsonify`) prints the canonical text and
    ignores a precision; `FeelNumber` is 32 bytes.
98. **NaN carries no sign.** FEEL has no NaN (DMN 1.6 §10.3.2.3.1: "There is no value for
    notANumber, positiveInfinity, or negativeInfinity. Use null instead."); the number core's NaN
    only marks an invalid operation until the evaluators turn it into null. It carried a sign
    that followed Intel's library (flipped by negation, cleared by `abs`, kept when propagated,
    and the subtraction did not invert it), visible only in `Debug` (`+NaN`, `-NaN`). The sign is
    gone with its special cases in negation, subtraction, `abs` and the power function; `Debug`
    prints `NaN`. Test first: the unit test `nan_has_no_sign` failed on the parent with `+NaN`;
    the test that pinned the Intel signs is deleted. Identity: no difference (a NaN never reaches
    a response). Rust behaviour break: `Debug` of a NaN `FeelNumber` is `NaN`.
99. **`exp` and `ln` results need no cohort rule.** The cohort (exponent) of a correctly rounded
    `exp` or `ln` result followed an emulation of Intel's binary128 to decimal128 conversion
    (`exp(1E-40)` as `1E+0`, `exp(1E-34)` with 34 digits): the result was evaluated a second time
    at 45 digits to decide it, and a fractional power was padded to 34 digits. Since item 97 no
    text shows a cohort, and no rule of the number core depends on it, so the emulation is gone:
    `binary128_cohort`, `wide_digits`, `full_precision` and the 45-digit precision, the second
    evaluation of every `exp` and `ln`, the precision argument of the backend's `exp` and `ln`
    (always 34 digits now), and the three `U256` helpers only the emulation used. A result is
    the value the backend rounded once. Test first: none can fail, since no response shows a
    cohort; the tests that pinned the cohorts (`exp_ln_cohorts_follow_intel`,
    `tests/exp.rs`, `tests/ln.rs`) are value tests now (value and canonical text, at the former
    cohort cases) and passed on the parent and after the change. Identity: no difference; the
    number oracle's `exp`, `log` and fractional power families have no new wrong answer. No API
    change; `Debug` shows other cohorts for these results.
100. **Rounding to a scale is one step.** DMN 1.6 Table 77: `floor`, `ceiling`, `round up`,
    `round down`, `round half up` and `round half down` "Return n with given scale and rounding
    mode", and footnote 1: "Scale is in the range [−6111..6176]". The number core scaled the
    number to an integer first (`n * 10^scale`, with the scale clamped to -6176..6144 like Intel's
    `bid128_scalbn`), rounded that intermediate and scaled it back; `round half down` subtracted
    one tenth of the last place kept and then rounded half up. So `round half down(2.56)` was 2,
    `round half down(1.1251, 2)` 1.12 and `round half down(-2.51)` -2; `ceiling(0.25, 6145)` was
    0.025 and `floor(1e-10, 6150)` 0.0000000000000001 (the clamp); and every number whose
    intermediate left the exponent range was null with "invalid scale": a zero at any non-zero
    scale (`round half up(0, 2)`, `floor(0, 1)`, `ceiling(0, -1)`), a tiny number at a coarse
    scale (`floor(1e-200, -6000)`, `round up(1e-6000, -6000)`). Now one private step,
    `round_at`, rounds a value to a multiple of `10^exponent` on its coefficient: a value with no
    digit below that position is itself (never padded), otherwise the dropped digits are compared
    with half a unit once and the kept coefficient is incremented by the mode; the result always
    fits. `FeelNumber::round_to(scale, mode)` checks the scale range (the only error) and calls
    it; the six methods and `round` (`decimal`) delegate to it, and rounding to an integral value
    is the same step at exponent 0. The modes are one enum, `RoundingMode` (`HalfEven`,
    `HalfUp`, `HalfDown`, `Up`, `Down`, `Ceiling`, `Floor`), which replaces the private `Rnd`;
    `check_scale`, `settled_at`, `unscale`, `quantize_v`, `ilogb_v` and the scaled intermediate go.
    Test first: `rounding_to_a_scale_is_one_step` (`feel-number/tests/round.rs`), a table of 341
    numbers and scales with the result of every mode computed by Python's `decimal`, failed on
    the parent with 392 wrong results (the half-down values, the clamped scale and the spurious
    errors); the unit tests that pinned the old values are updated (`round half down` of 2.51,
    -2.51, 1.151, a number at scale 6145, a NaN at a scale, the clamped scale of `round`).
    Identity: 643 registered differences in `registered/rounding-to-a-scale.json`, all on the
    number oracle's edge vectors of the six functions: 558 nulls that are now numbers, 85 wrong
    values corrected; `decimal` is unchanged. The oracle's twelve exemptions of the six
    functions are deleted, and its quick run judges every rounding result right. Rust
    behaviour break: the six rounding methods of `FeelNumber` give these values, accept a zero
    and a tiny number at every scale of the range, and `round` with a scale outside it is NaN
    (was a value at the clamped scale).
101. **The rounding built-ins share `round_to`.** `floor`, `ceiling`, `round up`, `round down`,
    `round half up` and `round half down` of feel-evaluator went through six copies of the same
    argument checks, each calling its own method of `FeelNumber`; now one private helper takes
    the number, the scale, the `RoundingMode` and the built-in's two names and calls
    `FeelNumber::round_to` (item 100), keeping every reason text byte for byte (`[core::round
    down] invalid argument type, …`, `[core::round_up] invalid scale: …`, `[core::floor]
    <FeelNumberError> invalid scale, …`). `decimal` calls `round_to(scale, HalfEven)` after
    truncating the scale and keeps its reason `scale is out of range`. `round_to` and
    `RoundingMode` become public with these callers (feel-evaluator now depends on
    `rheon-feel-number` directly); the seven methods they replace are removed. The helper builds
    its type reasons with `invalid_argument_type!`, which, like the module form of `value_null!`
    behind it, takes the function name as any expression, not only as a literal. The tests of
    `feel-number` and the two rounding benches call `round_to`. Test first: none new; the
    built-in tests, the rounding table of item 100 and the identity run are the proof.
    Identity: no difference. Rust API break: `FeelNumber::ceiling`, `floor`, `round_up`,
    `round_down`, `round_half_up`, `round_half_down` and `round` are removed; use
    `round_to(scale, RoundingMode::…)`, which returns `Err` only for a scale outside
    `-6111..=6176` (`round(&scale)` took the scale as a number and gave NaN there).
102. **`FeelNumber::new` scales without a clamp.** `new(n, s)` multiplied `n` by `10^-s` through
    a scaling step that clamped the power to -6176..6144 like Intel's `bid128_scalbn`, so
    `new(890, 6177)` was `890E-6176` (ten times too large) and `new(1, -6177)` was `1E+6144`
    instead of an overflow. Since item 100 nothing else used that step; it goes, and `new` packs
    `n · 10^-s` with the one rounding of the core (half-even, gradual underflow, infinity beyond
    the range). Test first: `test_debug_004` and the unit test of the padding near the overflow
    edge failed on the parent with the clamped values (`+890E-6176`, `1E+6144` for `new(1,
    -6145)`). Identity: no difference (no FEEL path passes such a scale). Rust behaviour break:
    `new(n, s)` with `s` beyond the range of the exponent is the rounded value or infinite
    (`new(890, 6177)` is `89E-6176`, `new(1, i32::MAX)` is 0, `new(1, i32::MIN)` is infinite).
103. **`modulo` is exact.** DMN 1.6 Table 77: "modulo (dividend, divisor) = dividend -
    divisor*floor (dividend/divisor)", and "In case either dividend or divisor is negative, the
    result has the same sign of the divisor"; the owner's decision is the formula over the exact
    values, rounded once. The built-in computed it in 34-digit steps (a division, a floor, a
    product and a difference, each rounded), so a quotient beyond 34 digits or near an integer
    lost the remainder: `modulo(1e40, 3)` was 1000000, `modulo(1e40, 7)` 0, `modulo(5e33, 0.7)` 0,
    `modulo(1 - 2e-34, 1/3)` was -1E-34 (the wrong sign), `modulo(-1E-6176, 1E+6000)` a negative
    number, and `modulo(1e6000, 1e-6000)` and `modulo(9e6144, 1e-6176)` were null (the quotient
    overflowed). Now `FeelNumber::modulo` takes the truncated remainder `|a| mod |b|` exactly on
    the coefficients (with `ea >= eb` the power `10^(ea-eb)` is taken modulo the divisor's
    coefficient by binary exponentiation with exact 256-bit products, new `U256::rem_u128`; with
    `ea < eb` it is a `u128` remainder, or `|a|` itself when `|b|` is the larger), keeps it with
    the divisor's sign when the signs agree, and otherwise subtracts it from `|b|`, the only
    rounding. `None` means a zero divisor; the built-in keeps its reason `[core::modulo] division
    by zero`. Test first: `the_remainder_is_exact` of the built-in's tests (with `modulo(1e40, 3)`
    1000000 on the parent) failed; the Table 77 examples are tests of the built-in and of
    `FeelNumber::modulo` (`feel-number/tests/modulo.rs`, which replaces the tests of `%`); the
    contract test that listed `modulo(9e6144, 1e-6176)` and `modulo(5, 1e-6176)` among the nulls
    of an overflow now asserts 0 for both. Identity: 13 registered differences in
    `registered/exact-modulo.json`, all on the number oracle's edge vectors; the two requests of
    the contract test are new edge vectors of its `modulo/huge-quotient` family, so they are in
    the committed source. The oracle's two `modulo` exemptions are deleted, and its quick run
    judges every `modulo` result right. Rust API break: `impl Rem` and `impl RemAssign` for
    `FeelNumber` (`%` was the same 34-digit-step floor modulo) are removed; use
    `FeelNumber::modulo`, which returns `None` for a zero divisor.
104. **Integer powers are correctly rounded.** DMN 1.6 Table 61: `e1 ** e2` is "(p,s) such that
    value(p,s) = value(e1)^value(e2) + ε", "p is limited to 34 digits", "ε is rounding error",
    with the one rounding of §10.3.2.3.1; the owner's decision is correctly rounded. An integer
    exponent within `i32` was a binary exponentiation that rounded every product to 34 digits,
    so the error grew with the exponent: `1.05 ** 360` ended in …541 instead of …542, `(1 +
    0.05/12) ** 360` in …413408 instead of …41355, `1.1 ** 100000` was 294 ulp off,
    `1.0001 ** 1000000` about 89,000 ulp, `2 ** -20000 = 0.5 ** 20000` was false and `3 ** (4 **
    5)` 1 ulp off. Now `pow_integer` takes the base `c0 · 10^e0` without trailing zeros and `n =
    |y|`: (1) `y > 0` and `c0^n` within 256 bits (`U256::checked_pow`, square and multiply with
    checked products, no allocation): that exact power, rounded once; (2) `y < 0` and `c0` a
    power of two or of five: `1/c0` is the exact `5^k · 10^-k` or `2^k · 10^-k`, raised the same
    way, so ties such as `2 ** -50` are exact; (3) otherwise while `c0^n` has at most 16,384
    bits (about 4,900 digits): the exact power from the backend (`Backend::int_pow`,
    `UBig::pow`), or for `y < 0` one correctly rounded division `1 / c0^n`, moved to the
    exponent `±e0 · n` and rounded once into the range; (4) beyond that the certified
    `exp(y ln x)` loop with the integer as exponent. An inexact integer power is never a tie, so
    that loop no longer falls back to the tie assumption for it: it goes on from 140 to 280 and
    560 working digits, and a result still undecided there (within about 10^-520 of a half-way
    point, relatively; no such operand is known) is rounded as its computed digits say;
    CONTRACT §3 states this bound. The `(-1)^y` emulation of Intel's cohort for an exponent
    outside `i32` goes with the binary exponentiation; `|x| = 1` gives ±1 by the parity of `y`
    on the exact path. Test first: `integer_powers_are_correctly_rounded`
    (`feel-number/tests/pow.rs`, the exact powers rounded once, ties included) failed on the
    parent with 12 wrong powers; the cohort test of `(-1)^y` is a value test now. Pinned values
    updated: `3 ** (4 ** 5)` in feel-evaluator, and the payment amounts of TCK 0008, 0009, 0003,
    0005, 0014, 0040 and 0041 in model-evaluator with their bench copies (checked by grep; each
    new value is the one the number oracle computes for the same formula); the TCK runner
    compares decimals with an epsilon, and those folders, 0075 and 0100 pass as before.
    Identity: 16 registered differences in `registered/integer-powers.json`, all on the number
    oracle's edge vectors (`(1 + 0.05/12) ** 360` is …41355, not …413550: the value ends in a
    zero, which the canonical text drops). The oracle's exemption for wrong integer-power values
    is deleted; its quick run judges every power right but the exponents outside Table 61's
    range. Speed, per evaluation against 0.4.0 (`feel.wasm`, a random 34-digit `x`): `x ** 2`
    1.0×, `x ** -3` 0.9×, `x ** 12` 1.35×, `x ** 100` 7.8× (0.14 ms, an exact power of 3,400
    digits), `x ** 1000000` and `1.0001 ** 1000000` 8.4× (0.43 ms, the certified loop; it was a
    binary exponentiation of 20 rounded products). Rust behaviour break: `FeelNumber::pow` with
    an integer exponent gives the correctly rounded value; `Debug` of an integer power shows the
    cohort of the exact power of the base without its trailing zeros (`10 ** 2` is `+1E+2`, was
    `+100E+0`; `1.50 ** 2` is `+225E-2`, was `+22500E-4`), and `(-1).pow(y)` for a huge `y` has
    the cohort of 1.
105. **An exponent outside Table 61's range gives null.** DMN 1.6 Table 61: in `e1 ** e2`,
    "value(e2) is a number in the range [-999,999,999..999,999,999]"; the owner's decision is
    null outside it. The evaluator computed every power: `1 ** 1e10`, `(-1) ** 1e10`,
    `1 ** 1000000000` and `1 ** 1000000000.5` were 1, `0.5 ** 1e10` and `0 ** 1000000000` 0, and
    `2 ** 1e10` null as an overflow. Now `**` checks the exponent first, for every base and every
    exponent value, integer or not, before the shortcuts `1 ** y` and `x ** 0` of the number
    core: outside the range it is null with the reason `exponentiation exponent is out of range
    -999999999..999999999`. `FeelNumber::pow` keeps its signature and has no domain of its own;
    `0 ** 0` stays 1 and an underflow stays 0 (`10 ** -999999999`). Test first:
    `an_exponent_outside_the_range_gives_null` failed on the parent with `1 ** 1e10` = 1; the
    test of `4 ** 1E+40` takes the new reason, and `4 ** 999999999` keeps the overflow reason.
    Identity: 12 registered differences in `registered/exponent-domain.json` (whole responses),
    all on the number oracle's edge vectors; the entry `1.000000001 ** 1000000000` of
    `registered/integer-powers.json` (item 104) is amended in place to the null, its `why`
    naming both rules. The oracle's last two exemptions are deleted: its list is empty, and its
    quick run judges every case right. No API change.
106. **A time offset prints its sign from its value.** DMN 1.6 §10.3.2.3.4: "A time in the
    semantic domain is a value of the XML Schema time datatype"; XML Schema Part 2 §3.2.7.3, the
    lexical representation of a timezone: "'+' indicates a nonnegative duration, '-' indicates a
    nonpositive duration". The text of an offset took its sign from the whole hours (`offset /
    3600`), which are 0 below one hour, so `time("10:00:00-00:30")` printed `10:00:00+00:30` and
    `time(10, 0, 0, duration("-PT30S"))` printed `10:00:00+00:00:30`: a text that names another
    value. An offset now has one type, `FeelOffset` (new file `feel-temporal/src/feel_offset.rs`):
    whole seconds within ±14:00, built by one range check, with one printer that writes `-` for a
    negative count and `+` otherwise (zero is `+00:00`, as before), then `hh:mm` of the
    magnitude and `:ss` when its seconds are not zero. `FeelZone::Offset` holds it, and `impl
    From<FeelOffset> for FeelZone` is the one place where the zero offset becomes UTC; offset
    literals and `TryFrom<i32>` go through it (`TryFrom<i32>` keeps its range in this item).
    Test first: `an_offset_prints_its_own_sign` (feel-evaluator, `bif_time.rs`) failed on the
    parent with `10:00:00+00:30`; `feel_offset.rs` has the printer's and the range's unit tests.
    Identity: 8 registered differences in `registered/offset-sign-from-value.json` (the `string`
    of a time and of a date and time with an offset below one hour, from a literal, `@`, `time(h,
    m, s, offset)` and `date and time(date, time, zone)`), with their requests and a control
    (`time offset`, unchanged) added to `requests/temporal.json`. Rust API break
    (rheon-feel-temporal): `FeelZone::Offset` holds a `FeelOffset` (was `i32`); `FeelOffset` is
    added with `ZERO` and `Display`; the `Display` of `FeelZone::Offset` changes for negative
    offsets below one hour.
107. **`time()` takes every offset within ±14:00 and none with a fraction of a second.** DMN 1.6
    Table 67, `time offset`: "The time offset duration must be in the interval
    [duration("-PT14H")..duration("PT14H")] as per the XML Schema Part 2 dateTime datatype"; XML
    Schema Part 2 §3.2.7.3: "Timezones are durations with (integer-valued) hour and minute
    properties". `time(hour, minute, second, offset)` accepted -11:59..+12:00 only (the range of
    XML Schema's recoverable timezone of a date, not of a time offset), so `time(10, 0, 0,
    duration("PT13H"))` was null, while literals took ±14:00; and it truncated the offset to
    whole seconds, so `duration("PT0.5S")` gave a UTC time and `duration("PT1H0.5S")` +01:00.
    Now `FeelOffset::from_duration` is the offset of a duration: whole seconds, then the one
    range check of `FeelOffset`; `time_4` builds the time in `FeelZone::from` of it, and any
    other duration gives null with the reason `time: offset is out of range: …` as before. Whole
    seconds stay accepted (the TCK's `time(11, 59, 45, duration("PT2H45M55S"))` is
    `11:59:45+02:45:55`). `FeelTime::offset_opt` and `FeelDateTime::offset` go through the same
    check. `TryFrom<i32> for FeelZone` and `err_invalid_time_zone_offset` are removed; the doc of
    `FeelZone::Offset` and the comment on `tzOffsetMinutes` (its ±18:00 is the host offset a
    request accepts, not a FEEL limit) are corrected. Test first:
    `an_offset_is_a_whole_number_of_seconds_within_14_hours` (feel-evaluator, `bif_time.rs`)
    failed on the parent with null for `duration("PT13H")`. Identity: 9 registered differences in
    `registered/time-offset-range.json` (whole responses: six offsets between ±12:00 and ±14:00
    that were null are times, and three with a fraction of a second that were times are null),
    with their requests and the boundary cases that keep their answers (`PT14H1S`, `-PT14H1S`,
    `PT14H0.000000001S`, the TCK's `PT2H45M55S` pair) added to `requests/temporal.json`. Rust API
    break (rheon-feel-temporal): `TryFrom<i32> for FeelZone` removed (`From<FeelOffset>`
    replaces it); `FeelOffset::from_duration` added; `FeelTime::offset_opt` and
    `FeelDateTime::offset` accept offsets within ±14:00 (were -11:59..+12:00).
108. **The minutes of an offset literal are below 60.** DMN 1.6 §10.3.2.3.4: a time is "a value
    of the XML Schema time datatype"; XML Schema Part 2 §3.2.7.3: timezones have "the minute
    magnitude limited to at most 59". The offset of a literal was the sum `3600 h + 60 m`
    unchecked, so `+05:99` was `+06:39`, `+13:60` was `+14:00` and `-00:60` was `-01:00`. Now
    `FeelZone::from_captures` gives `None` for minutes of 60 or more, so a time, date and time
    or `@` literal with such an offset is invalid (null with its literal reason), and so is the
    zone `"+05:99"` of `date and time(date, time, zone)` (`FeelZone::from_designator`). Test
    first: `test_from_captures` and `zone_designators` (feel-temporal) and
    `the_minutes_of_an_offset_literal_are_below_60` (feel-evaluator) failed on the parent with
    `Some(Offset(FeelOffset(23940)))` and `10:00:00+06:39`. Identity: 8 registered differences
    in `registered/offset-literal-minutes.json` (whole responses), four on requests of
    `requests/temporal.json`; the other requests and two controls (`-01:59`, `+05:59`) are added
    to it. Rust behaviour break (rheon-feel-temporal): `FeelTime`, `FeelDateTime` and
    `FeelZone::from_designator` reject an offset with minutes of 60 or more.
109. **An offset literal has no seconds.** DMN 1.6 §10.3.2.3.4: a time is "a value of the XML
    Schema time datatype"; XML Schema Part 2 §3.2.7.3: "The lexical representation of a timezone
    is a string of the form: (('+' | '-') hh ':' mm) | 'Z'". The offset pattern of time, date
    and time and `@` literals and of the zone of `date and time(date, time, zone)` took an
    optional `:ss`, so `time("10:00:00+05:30:15")` was a time with that offset and
    `@"2020-01-01T10:00:00+01:00:00"` one with +01:00. Now `OFFSET_PATTERN` is `±hh:mm` and
    `FeelZone::from_captures` has no seconds branch: such a literal or zone is invalid and gives
    null with its reason. An offset with seconds remains a value: `time(h, m, s, offset)` takes
    whole seconds (item 107), and the TCK requires `string(time(11, 59, 45,
    duration("PT2H45M55S")))` to be `"11:59:45+02:45:55"`; that text does not read back as a
    literal. Test first: `an_offset_literal_has_no_seconds` (feel-temporal and feel-evaluator)
    failed on the parent with `Some(Offset(FeelOffset(18001)))` and the time
    `10:00:00+05:30:15`. Tests changed with it: `test_from_captures`, `zone_designators`, the
    zone table of `date_and_time_from_date_time_and_zone_takes_the_zone_suffix_rules` (`+01:00:30`
    is invalid), `bif_date_time::_0038` (null, and the same value written `+05:01`),
    `bif_is::_0047`-`_0049` (written with `±hh:mm` offsets), and the non-compliant model N_0079,
    `_0013` and `_0016` with their bench copies (null; the model expects the text with offset
    seconds). Identity: 8 registered differences in `registered/offset-literal-seconds.json`
    (whole responses), four on requests of `requests/temporal.json`; the other four are added
    to it. Rust behaviour break (rheon-feel-temporal): `FeelTime`, `FeelDateTime` and
    `FeelZone::from_designator` reject an offset written with seconds.
110. **The offset of a zoned date and time is displayed with its seconds.** DMN 1.6 Table 48: a
    date and time with an IANA zone maps to an ISO 8601 text "suffixed by the IANA timezone id in
    rectangular brackets, e.g. 2007-12-03T10:15:30+01:00[Europe/Paris]". The wasm `display` built
    that offset with its own sign and minute arithmetic and dropped its seconds, so
    `@"1900-01-01T12:00:00@Europe/Paris"` showed `+00:09[Europe/Paris]` for the zone's +00:09:21
    (local mean time before the standard zones): the text of another instant. Now
    `zoned_date_time` (`wasm/feel-wasm/src/display.rs`) writes the offset the zone database
    resolves with `FeelOffset::write_seconds`, the one offset printer (item 106), which takes a
    plain count of seconds, since such an offset can have seconds and can lie beyond ±14:00; its
    own arithmetic is deleted. The seconds depart on purpose from the form Table 48 asks for:
    ISO 8601 writes an offset with hours and minutes only, as XML Schema Part 2 §3.2.7.3 does
    ("The lexical representation of a timezone is a string of the form: (('+' | '-') hh ':' mm)
    | 'Z'", the clause item 109 applies to literals); `±hh:mm:ss` keeps the instant, as
    java.time's zoned text does, where whole minutes would name another one. CONTRACT §3 states
    it. Test first:
    `zoned_date_time_display_keeps_the_seconds_of_the_offset` (`wasm/feel-wasm/tests/contract.rs`)
    failed on the parent with `+00:09[Europe/Paris]`. Identity: 3 registered differences in
    `registered/zoned-display-offset-seconds.json` (Paris 1900 as `date and time()` and `@`, New
    York 1880 `-04:56:02`), with their requests and two controls (Kolkata `+05:30`, the `time
    offset` of Paris 1900) added to `requests/temporal.json`. Rust API (rheon-feel-temporal):
    `FeelOffset::write_seconds` added.
111. **`second` and `seconds` are exact decimals.** DMN 1.6 Table 67: `second` is "The second of
    the minute as a decimal in the interval [0..60)", and `seconds` of a days and time duration
    "must return a decimal in the interval [0..60)". Both read the whole seconds only, so
    `time("10:00:30.5").second` was 30 and `duration("PT1.5S").seconds` 1. Now they come from
    `temporal_value::second` (feel-evaluator, `builders.rs`), which builds the exact decimal of the
    whole seconds and the nanoseconds with the same step as `value`; `seconds` stays a magnitude
    (`duration("-PT1.5S").seconds` is 1.5), like `days`, `hours` and `minutes`. The `Display` of a
    days and time duration takes its nanoseconds from the new `get_nanoseconds`. Test first:
    `second_and_seconds_are_exact_decimals` (feel-evaluator, `properties.rs`) failed on the parent
    with 30 for `time("10:00:30.5").second`; the pinned `seconds` of
    `duration("-PT9223372036.854775807S")` is 16.854775807 (was 16). Identity: 13 registered
    differences in `registered/exact-decimal-seconds.json`, five of them requests of
    `requests/temporal.json` already, with the other eight and three controls (whole seconds,
    `date("2020-01-01").second`) added to it. Rust API (rheon-feel-temporal):
    `FeelDaysAndTimeDuration::get_nanoseconds` and `FeelDateTime::nanos` added.
112. **The years and months of a duration are magnitudes.** DMN 1.6 Table 67: `years` is "The
    normalized years component of a years and months duration value as an integer", and `months`
    "must return an integer in the interval [0.. 11]"; the owner's decision is that every
    duration component is a magnitude. `years` and `months` were signed (`duration("-P1Y2M")` had
    -1 and -2), while `days`, `hours`, `minutes` and `seconds` of a days and time duration are
    magnitudes. Now `FeelYearsAndMonthsDuration::years` and `months` split the magnitude of the
    months count (`-P1Y2M` has 1 and 2), `is_negative` carries the sign, and the `Display` of the
    duration uses them instead of its own split. `value` keeps the signed months count. Test
    first: `the_components_are_magnitudes` (feel-temporal, every count in -30..=30: `months` in
    `0..12`, `12 * years + months` the magnitude) and `years_and_months_are_magnitudes`
    (feel-evaluator, `properties.rs`) failed on the parent with -6 months for -30 and -1 for
    `duration("-P1Y2M").years`. Identity: 7 registered differences in
    `registered/duration-component-magnitudes.json`, with their requests and six controls
    (positive durations, a negative one's zero component, `value` and the text) added to
    `requests/temporal.json`. Rust API break (rheon-feel-temporal):
    `FeelYearsAndMonthsDuration::years` and `months` return `u64` magnitudes (were signed `i64`).
113. **`years and months duration(from, to)` has one rule for both signs.** DMN 1.6 Table 73:
    "return years and months duration between from and to", with `years and months
    duration(date("2011-12-22"), date("2013-08-24")) = duration("P1Y8M")`. `&FeelDate - &FeelDate`
    chose its branch by the year alone, so a negative span inside one year took the day rule of a
    positive one: from 2020-05-15 to 2020-03-01 was `-P3M`, while from 2020-03-01 to 2020-05-15
    is `P2M`. Now it orders the two dates: the difference of their months, one fewer when the day
    of the later date is below the day of the earlier one, negated when `self` is the earlier
    date, so `a - b` is `-(b - a)`. Positive spans and negative spans across years keep their
    answers; date and time arguments still compare their dates alone. Test first:
    `a_negative_span_inside_one_year` and
    `the_months_between_two_dates_have_one_rule_for_both_signs` (feel-temporal: over a grid of two
    years, every month and the days 1, 15, 28, 31, antisymmetry, and for an earlier date whose day
    every month has, `a + (b - a) <= b < a + (b - a) + P1M`) and
    `the_duration_between_two_dates_has_one_rule_for_both_signs` (feel-evaluator) failed on the
    parent with `-P3M`. At a month end the day rule and the clamped addition differ
    (2021-01-31 plus `P1M` is 2021-02-28, while from 2021-01-31 to 2021-02-28 is `P0M`), so the
    grid's addition test leaves out the days 29 to 31 of the earlier date. Identity: 6 registered
    differences in `registered/years-and-months-between.json`, with their requests and six
    controls (the positive twins, a span across years, equal dates, the example of Table 73 and
    its reverse) added to `requests/temporal.json`. Rust behaviour break (rheon-feel-temporal):
    `&FeelDate - &FeelDate` gives new results for negative spans inside one year.
114. **The value of a time in a zone without a known offset is null.** DMN 1.6 §10.3.2.3.4: "If
    no time offset is specified, the time is interpreted as a local time of day", and `valuet` is
    the number of seconds since midnight; a time in a zone is valued in its UTC form. `of_time`
    (feel-evaluator, `temporal_value`) took a zone whose offset it could not resolve as UTC
    (`feel_time_offset().unwrap_or(0)`), so such a time had the value of another instant. Now a
    local time keeps its clock time, a time with an offset or a resolved zone its UTC form, and a
    time in a zone without a known offset gives null with the reason `value: the time zone of the
    time has no known offset`. A time in a zone resolves its offset on the current date, so the
    case is not reproducible through an expression (a zone name is checked when it is read; a
    time in a gap of daylight saving time has no offset only on such a day). Test first:
    `the_value_of_a_time_needs_its_offset`, a unit test inside `temporal_value` (local, UTC,
    `+01:00`, `Etc/GMT-1` and an unknown zone name), failed on the parent with 36000.5 for the
    unknown zone. Identity: no difference; four controls (a local, a UTC, a zoned and an offset
    time with a fraction) are added to `requests/temporal.json`. No API change.
115. **A years and months duration that drops a fraction of a month warns.** DMN 1.6 Table 60:
    a years and months duration times a number is `valueymd⁻¹(valueymd(e1) * value(e2))`, and
    divided by a number "If value(e2)=0, the result is null. Else the result is
    valueymd⁻¹(valueymd(e1) / value(e2))"; §10.3.2.3.8: "The value of a years and months duration
    can be expressed as a number of months", and the text is silent on a product or a quotient
    with a fraction of a month. The DMN TCK (compliance-level-3, 0100-arithmetic) expects it
    truncated toward zero: `@"P10Y11M" / 2.5` is `P4Y4M`, `@"P10Y11M" / -2.5` is `-P4Y4M` and
    `-2.5 * @"P1Y11M"` is `-P4Y9M`. The answers keep that truncation, as 0.4.0 gave
    them, and by the owner's decision the evaluation now says what it dropped: the three
    years and months duration sites of `build_mul` and `build_div` (feel-evaluator,
    `builders.rs`) call `temporal_value::ym_duration`, the one conversion of a number of months
    to a duration, which truncates and records the warning `fractional months of 'P10Y11M / 2.5'
    truncated to whole months; dropped fraction: '0.4'` through the warning channel of the
    truncated fractional seconds. A whole number of months, as FEEL computes it in 34 digits
    (`duration("P1Y") * (1/3)` is `P4M`), does not warn. A warning is no error: `diagnostics`
    and strict mode are unchanged. CONTRACT §3 lists the warning. Test first:
    `a_years_and_months_product_warns_when_it_drops_a_fraction_of_a_month`
    (`multiplication.rs`) and `a_years_and_months_quotient_warns_when_it_drops_a_fraction_of_a_month`
    (`division.rs`) failed on the parent with no warning. Identity: 13 registered differences in
    `registered/whole-months-truncation-warnings.json` (whole responses: the same result with
    the new warning), with their requests and fourteen controls (whole results, zero, a product
    out of range, a division by zero, days and time durations) added to `requests/temporal.json`.
    No API change.
116. **The nulls that the specification defines are pinned as no errors.** DMN 1.6 §7.3.8: the
    evaluation of a semantic error "MUST report or log diagnostic information and SHALL return
    null", while some inputs have null as their defined result: Table 51, before which "Because
    these are total functions, the input can be true, false, or otherwise", gives null for `and`
    and `or` with an operand that is no boolean unless the other operand decides (`false and x`
    is false, `true or x` is true); §10.3.4.4 has `sum([]) = null` and `mean([]) =
    null`; Table 82 has `get value({key1 : "value 1"}, "unexistent-key") = null`. These already
    answer a bare null with no error, and now tests hold them so: `te_errors` (feel-evaluator,
    `tests/mod.rs`, beside `te_warnings`, which shares its evaluation) asserts the value, the
    counted errors and their messages, and the `and`, `or`, `sum`, `mean`, `get value` and
    `[{a: 1}].b` tests assert none; `defined_nulls_are_no_errors`
    (`wasm/feel-wasm/tests/contract.rs`) asserts a count of 0 and the same result in strict mode;
    `a_missing_input_variable_of_a_built_in_type_is_a_null_without_an_error`
    (model-evaluator, `input_data.rs`) asserts a bare null and a count of 0. Tests only: they pass
    on the parent. `requests/nulls.json` is a new committed request source (71 requests) for
    these and the neighbouring nulls and errors (a missing context entry, `stddev` of fewer than
    two numbers, `if` with a condition that is not true, `!=` on kinds that `=` cannot compare, a
    null operand of `+`), lenient and strict, FEEL and B-FEEL. Identity: no difference. No API
    change.
117. **A missing context entry is a defined null with a note.** DMN 1.6 §10.3.2.6: "Selecting a
    value by key from context m in the semantic domain is denoted as m.key1 or get value(m,
    "key1")", and Table 82 defines `get value({key1 : "value 1"}, "unexistent-key") = null`;
    §7.3.8 makes a semantic error "report or log diagnostic information". `{a: 1}.b` gave the
    null with the reason `build_path: no entry b in context: {a: 1}`, counted as an error, so
    strict mode failed on it, while `get value({a: 1}, "b")` is a bare null without one. Now a
    reason is an error or a note: `rheon_feel::diagnostics::Severity` (`Error`, `Note`,
    non-exhaustive) says which, `record(severity, message)` records an error and returns at once
    for a note, and `null_with_reason(severity, reason)` builds the message once for both.
    `value_null!(note: …)` gives the null with a note, every other form an error null, and
    `value_null!()` is unchanged. The path's missing entry (feel-evaluator, `build_path`) is a
    note: the null keeps its text (`result.nullReason`, the `null(…)` of `result.feel` and the
    Rust `Display` are unchanged), it is not counted or listed in `diagnostics`, and strict mode
    does not stop at it. The module docs of `diagnostics` classify every null as a value, a
    defined null (bare or with a note), an error null or an abort. CONTRACT §3 and the
    `Diagnostics` type of `@rheon/feel-runtime` say that a note is no error. Test first:
    `a_missing_context_entry_is_a_null_with_a_note` (feel-evaluator, `path.rs`) failed on the
    parent with the message listed, and `a_missing_context_entry_is_no_error`
    (`wasm/feel-wasm/tests/contract.rs`) with a count of 1; `a_note_is_no_error` and
    `a_note_with_arguments_is_capped_like_any_message` (feel, `diagnostics.rs`) test the
    mechanism. Identity: 6 registered differences in `registered/missing-context-entry.json`
    (lenient, strict and B-FEEL `{a: 1}.b`, a nested path, and a path beside a division by zero,
    whose division is still counted), all requests of `requests/nulls.json`, three of them added
    in this commit; the entry of `{a: for i in 1..1000 return i}.b` in
    `registered/message-previews.json` is amended in place (its count is now 0). Rust API break
    (rheon-feel): `diagnostics::Severity` added; `diagnostics::record` and
    `values::null_with_reason` take the severity first; `value_null!` gains the `note:` forms;
    `Diagnostics::count` and `messages` leave out notes.
118. **`stddev` of fewer than two numbers is a defined null.** DMN 1.6 §10.3.4.4, stddev: "If
    the list is empty or if the list contains only one element, the function returns null", with
    `stddev( [ 47 ] ) = null` and `stddev( 47 ) = null`; its parameter is a "list of number".
    `stddev` (feel-evaluator, `bifs/core.rs`) tested the count first, with a counted error, so
    `stddev([47])` failed strict mode and `stddev(["a"])` was reported as too short; the
    positional form gave a number or a boolean as its one argument the error `[positional::stddev]
    invalid argument type, expected list`. Now the items are checked to be numbers first (an
    error null, `stddev: expected number, actual type is … with value …`), and then fewer than
    two numbers is the null with the note `stddev: minimum two input arguments expected`, not
    counted and not failing strict mode; one positional argument that is no list is a list of
    one item, as for `sum`, `mean` and `median`. Test first:
    `stddev_of_fewer_than_two_numbers_is_a_null_with_a_note` (`bif_stddev.rs`) failed on the
    parent with the message counted, and `stddev_of_fewer_than_two_numbers_is_no_error`
    (`wasm/feel-wasm/tests/contract.rs`) with a count of 1. The pins of `stddev(true)`
    (`bif_stddev.rs`) and of TCK 0063 `stddev(null)` and `stddev(47)` (`dmn_3_0063.rs`, test and
    bench) take the new texts. Identity: 15 registered differences in
    `registered/stddev-fewer-than-two-numbers.json` (lenient, strict and B-FEEL), all requests of
    `requests/nulls.json`, four of them added in this commit. No API change.
119. **A decision table without a matching rule is a defined null.** DMN 1.6 §10.3.2.10: "A
    decision table may have no rule hit for a set of input values. In this case, the result is
    given by the default output value, or null if no default output value is specified." The
    decision table evaluator (model-evaluator, `decision_table.rs`) gave that null with the
    counted reason `no rules matched, no output value defined`, so
    `evaluate_invocable_with_mode(…, ErrorMode::Strict)` turned it into the strict null. Now the
    reason is a note (item 117): the value keeps its text, the count is 0, and strict mode
    returns the same value. Test first: `no_matching_rule_is_a_null_with_a_note`
    (model-evaluator, `no_default_output_values.rs`) failed on the parent with a count of 1.
    Identity: no difference (the wasm module evaluates no decision tables). Rust behaviour break
    (rheon-model-evaluator): `evaluate_invocable_with_mode` counts no error for a decision table
    without a matching rule, and in strict mode returns its value.
120. **A missing input variable is a defined null whatever its type.** An input variable that
    is missing from the input context is missing data, and a null the specification defines for
    its input is no error (DMN 1.6 §7.3.8 covers semantic errors; the owner's list of defined
    nulls names the missing input variable). A variable of a built-in type gave a bare null with
    no error, while one whose type is an item definition (model-evaluator, `variable.rs`) gave
    the counted reason `no name … in context …`, so strict mode failed on it. Now that reason is
    a note (item 117): the value keeps its text, the count is 0. A structured input that lacks a
    component stays a counted error: how structured inputs are bound is not changed here. Test
    first: `a_missing_input_variable_of_an_item_definition_type_is_a_null_with_a_note`
    (model-evaluator, `input_data.rs`, model `tEmploymentStatus` with the context `{}`) failed on
    the parent with a count of 1. The same conversion reads the input decisions of a decision
    service invoked as a whole from the input context, so an input decision of an
    item-definition type that the context does not supply is the same missing data:
    `a_missing_input_decision_of_an_item_definition_type_is_a_null_with_a_note`
    (model-evaluator, `tests/various/dmn_0007`) failed on the parent with a count of 1.
    Identity: no difference (the wasm module has no input data).
    Rust behaviour break (rheon-model-evaluator): `evaluate_invocable_with_mode` counts no error
    for a missing input variable of an item-definition type, nor for such an input decision of
    a decision service that the context does not supply, and in strict mode no longer fails
    on it.
121. **`if` takes the else branch unless the condition is true.** DMN 1.6 Table 50 rule 45:
    `if e1 then e2 else e3` is "if FEEL(e1) is true then FEEL(e2) else FEEL(e3)", and "A
    conditional if a then b else c is equal to b if a is true, and equal to c otherwise." The
    `if` evaluator (feel-evaluator, `build_if`) took the else branch for `false` and null only,
    and gave a condition of another kind the counted error `condition in 'if' expression is not
    a boolean value`, so `if 9 then 1 else 2` was null and failed strict mode. Now every
    condition that is not `true` takes the else branch, with no error: `if 9 then 1 else 2`,
    `if "x" then 1 else 2`, `if [true] then 1 else 2` and `if {} then 1 else 2` are 2. The boxed
    conditional is unchanged: DMN 1.6 §10.2.1.8 says its "if" part "MUST resolve to a boolean",
    and TCK 1150 expects null for one that does not. Test first:
    `a_condition_that_is_not_true_takes_the_else_branch` (feel-evaluator, `if_expression.rs`)
    and `if_takes_the_else_branch_unless_the_condition_is_true`
    (`wasm/feel-wasm/tests/contract.rs`) failed on the parent with the null; the pin of
    `if N then 1 else 2` with `N = 9` (`if_expression.rs`) is now 2. Identity: 8 registered
    differences in `registered/if-else-unless-true.json` (the four conditions, lenient and
    strict), all requests of `requests/nulls.json`. No API change.
122. **`!=` reports like `=`.** DMN 1.6 §10.3.2.15: "By definition, FEEL(e1 != e2) is FEEL(not(e
    1= e2))". `=` on operands it cannot compare (`"a" = 1`) gives null with the counted reason
    `equal err '"a"' =?= '1'`, but `!=` (feel-evaluator, `build_nq`) gave a bare null with no
    reason, so strict mode stopped on `"a" = 1` and not on `"a" != 1`. Now one private function,
    `equal`, holds the body of `=`; `!=` negates its boolean and passes its null on, with the
    same reason, the same count and the same strict failure. In B-FEEL `"a" != 1` is still
    `true`, and the error behind it is listed, as for `=`. Test first:
    `not_equal_on_kinds_that_equal_cannot_compare_reports_like_equal` (feel-evaluator,
    `comparison_ne.rs`, every pair of kinds whose `=` the equality tests pin as `equal err`) and
    `not_equal_reports_like_equal` (`wasm/feel-wasm/tests/contract.rs`, lenient, strict and
    B-FEEL) failed on the parent with the bare null. The BACKLOG entry on `!=` is removed.
    Identity: 5 registered differences in `registered/not-equal-reports-like-equal.json`
    (lenient, strict, B-FEEL, B-FEEL strict, and a date against a number), all requests of
    `requests/nulls.json`. No API change.
123. **A null operand of an arithmetic operator is an error.** DMN 1.6, before Table 57 and
    before Table 59: "if input values are not of the listed types, the result is null"; Table 61
    takes "type(e1) is number. value(e2) is a number"; Table 63 defines `-e` as "e*-1". The text
    gives the value, null, and does not say whether a null operand is an error, so the official
    TCK decides: it marks every arithmetic operation with a null operand `errorResult="true"`
    ("invalid arithmetic combination", compliance-level-3/0100-arithmetic, the
    `error_when_{add,subtract,multiply,divide,exponent}_*` cases with a null on either side,
    `10 + null` and `null + 10` among them), and TCK 0085 decision 002 with a null input ("because
    of the string + null operation"). The operators decided it each in their own way: the
    addition (feel-evaluator, `build_add`) passed a null on when it was the left operand or stood
    beside a number (`1 + null`, `null + "a"`, no error) and otherwise counted an error with one
    of several reasons (`expected string as a second argument in addition`, `[builders::add]
    invalid argument type, …`); the multiplication passed a null left operand on (`null * 1`, no
    error); the subtraction and the division counted `[subtraction] incompatible types: 1 - null`
    and `[division] incompatible types: …`, the exponentiation `exponentiation base is not a
    number`, the negation `unexpected type in arithmetic negation: Null`. Now one private rule,
    `Arithmetic::null_operand`, decides for all six operators: with a null operand on either side
    the value is null with the counted reason `[<operator>] incompatible types: <operation>`, the
    style of the subtraction's and the division's type errors, which `Arithmetic::incompatible`
    writes with the operands as evaluated (`[addition] incompatible types: 1 + null`,
    `[negation] incompatible types: -null`), and strict mode fails on it; a failing operand
    counts its own error before it (`"a" + 1/0` counts the division's and the addition's). The
    binary operators go through one combinator, `arithmetic`, which evaluates both operands and
    hands them to `operate`, and the incompatible-types errors of the subtraction, the
    multiplication and the division come from `Arithmetic::incompatible` too, with their texts
    unchanged. `operate` holds the null rule and the operator and is never inlined: the
    combinator's frame stays on the stack while a nested operand evaluates, and with `operate`
    inlined the stack report's recursion reached 17 levels on Node's main thread with Liftoff,
    one fewer than the parent; out of line it reaches 22 there (parent 18), 46, 75 and 104 in
    workers of 2, 3 and 4 MiB (40, 64, 89), 24, 51, 83 and 114 with TurboFan (13, 29, 47, 65),
    and 289, the parser's limit, in Wasmtime with a 16 MiB stack (120); the smaller Wasmtime
    stacks reach the parent's 10, 26, 57 and 120. B-FEEL converts the null operand (§11.9-§11.11)
    and gives a value (`1 + null` and `2 ** null` are 1, `null - 6` is -6, `null * 1` is 0,
    `"a" + null` is `"a"`), and the FEEL error behind it stays listed, as for its other
    conversions (CONTRACT §2), so B-FEEL in strict mode fails on it, as it did for the
    subtraction (`null - 6`). Test first: `a_null_operand_of_an_addition_is_an_error`,
    `…_of_a_multiplication_…`, `…_of_an_exponentiation_…` and `…_of_a_negation_…`
    (feel-evaluator, one per operator's test file, the binary ones through `te_null_operands`:
    every kind on either side of a null, and a failing operand) and
    `a_null_operand_of_an_arithmetic_operator_is_an_error` (`wasm/feel-wasm/tests/contract.rs`,
    lenient, strict and B-FEEL) failed on the parent; `…_of_a_subtraction_…` and
    `…_of_a_division_…` passed there, as those operators already counted the error with this
    text. Pins that took the old answers: TCK 0100 (72 tests: 25 additions, 22 multiplications,
    25 exponentiations), TCK 0085 decisions 002 with a null input (three), TCK 2-0105 (three,
    which expect null and mark no error) and TCK 2-0003 (the allowed-values null of the input),
    each in `src/tests/compatibility` and in its bench copy; in feel-evaluator `null + @"P1DT2H"`,
    `null * @"P1Y"`, a date plus an out-of-range product and its negation, two function bodies
    that add a missing name, and a missing context entry plus 1 (`path.rs`); in model-evaluator
    the decision of `various/dmn_0007` that doubles a missing input decision, whose note stays
    uncounted while the multiplication counts its error; and `bbt/tests/cli/edm/1006_ERR`. The
    TCK runner of this repository compares values, which stay null, so its results are
    unchanged. The diagnostics module docs no longer count a null operand passed on among the
    defined nulls. Identity: 85 registered differences in `registered/arithmetic-null-operand.json`
    (30 FEEL, 16 FEEL strict, 23 B-FEEL, 16 B-FEEL strict), all requests of `requests/nulls.json`,
    which gains 146 requests (each binary operator with a null on either side of a number, of a
    string, of a null and of a failing operand, and the negation of null, of a failing operand and
    of a missing context entry, in the four modes): the operations whose null operand 0.4.0 passed
    on (`1 + null`, `null + 1`, `null * 1`, …) count an error now, the others keep their count
    with the new reason, and the subtraction and the division answer as in 0.4.0. Five entries of
    `registered/durations-symmetric-range.json` (the negation of an out-of-range product, and a
    date, a date and time or a time plus one) and the entry `({a: {b: 1}}.a.c) + 1` of
    `registered/missing-context-entry.json` are amended in place (the product's or the note's null
    and the operator's error). No API change.
124. **A unary comparison compares with the value of its path endpoint.** DMN 1.6 §10.3.2.7: a
    range is "a comparison operator and a single endpoint (grammar rule 7.a.)", and "Endpoints
    can be expressions (grammar rule 16)"; the grammar parses `< x.p` as the unary comparison of
    the path `x.p`. The path evaluator (feel-evaluator, `build_path`) looked at its parent node:
    under a unary comparison it wrapped a left value that is neither a context nor a list in the
    unary value first, read the property from that, and returned it as `Value::Transparent`,
    which the unary builders unwrapped. So `< x.p` was `(< x).p`: `< d.year` with `d` a date was
    null with `no such property in unary less: year`, and `> r.start` with `r` a range was `r`
    itself, so unary tests such as `< applicant.dob.year` never matched. Now a path is
    evaluated the same way wherever it stands, and the unary builders wrap whatever their
    endpoint gives: `< d.year` is `UnaryLess(2020)`, and in unary tests 2000 satisfies it, 20
    satisfies `> r.start`. A path whose property does not exist is a null endpoint, as `< null`
    is. A property of the unary value itself is read with parentheses, `(< 10).end`, the form
    TCK 0074 uses. The builder no longer keeps a stack of parent nodes, which had no other
    reader, and its lifetime parameter goes with it. Test first:
    `a_unary_comparison_compares_with_the_value_of_its_path_endpoint` (feel-evaluator,
    `properties.rs`, and `wasm/feel-wasm/tests/contract.rs` in the unary-tests dialect) failed
    on the parent with `false`. The 16 property tests of `>1.start` and the like
    (`properties.rs`) take the parenthesised form `(>1).start`, which keeps their answers; the
    three `Value::Transparent` assertions (feel, `tests/values.rs`) are gone. CONTRACT §2 says
    that path endpoints need no parentheses. Identity: 14 registered differences in
    `registered/unary-comparison-path-endpoints.json`, all requests of the new committed source
    `requests/unary-tests.json` (20 requests: path endpoints in unary tests and as expressions,
    and the parenthesised forms, which do not change). Rust API break (rheon-feel):
    `Value::Transparent` is removed.
125. **One collection evaluator for every simple type.** An item definition that is a collection
    of a simple type (model-evaluator, `item_definition.rs`) had ten copies of one closure, one
    per FEEL type, that differed only in the test of an element and the type named in the
    message. They are one closure now, built from the element test and that name, so a rule for
    the elements is written once. The messages stay as they were, including the two that differ
    from the rest: a collection of nulls says `expected null` for a value that is no list, and a
    collection of `Any` takes every element. A test pins them:
    `a_collection_of_a_simple_type_names_what_it_expected` (`item_definition.rs`), which passes on
    the parent as well. The compatibility tests of the model evaluator pass unchanged. Identity:
    no difference (the wasm module has no item definitions). No API change.
126. **An item definition that references another keeps its allowed values.** DMN 1.6 §7.3.3:
    "An ItemDefinition element may restrict the values that are allowed from typeRef, using the
    allowedValues attribute. allowedValues is an instance of UnaryTests that constrains the
    domain of the typeRef." The evaluator of an item definition whose typeRef names another item
    definition (model-evaluator, `item_definition.rs`) built its allowed values and dropped them,
    so `tSmall` (typeRef `tBase`, a number, allowedValues `[0..10]`) took 11. Now the value the
    referenced definition gives is tested against the allowed values; a null it gives keeps its
    reason, as the simple types pass a null through. Test first:
    `an_item_definition_that_references_another_keeps_its_allowed_values` (`item_definition.rs`,
    the new example model `item_definition/0701.dmn`) failed on the parent with 11 for 11. TCK
    0109, 0110, 0113 and 0119 are unchanged. Identity: no difference (the wasm module has no
    item definitions). Rust behaviour break (rheon-model-evaluator): an input whose item
    definition references another with allowed values is null with `value '…' is not allowed`
    when the value is outside them.
127. **Allowed values are tested against each element of a collection.** DMN 1.6 §7.3.3: "In
    cases where the isCollection attribute of an ItemDefinition is true, each element of the
    collection must satisfy the UnaryTests of the allowedValues, i.e. the allowedValues are
    projected onto the collection elements"; Table 26: "In case of a collection, it is projected
    on the collection elements." The three collection evaluators (model-evaluator,
    `item_definition.rs`: of a simple type, of a referenced type, of a type with components)
    tested the whole list, so a collection of strings with allowedValues `"A", "B"` rejected
    `["A"]` and `[]` with `value '["A"]' is not allowed`. Now `check_each_allowed_value` tests
    each element and names the first one that fails (`["A", "C"]` is null with `value '"C"' is
    not allowed`); `[]` is `[]`. An element that the referenced definition of a collection
    rejects (a value that is not null, made null) stays in the list with its null and reason, as
    item 126 does for a single value; every other element is tested, a null element included, so
    `["A", null]` is null with `value 'null' is not allowed`, as it is for a collection of a
    simple type (there with `expected string`). Test first:
    `allowed_values_are_tested_against_each_element_of_a_collection` (`item_definition.rs`, the
    new example model `item_definition/0702.dmn`) failed on the parent with `value '["A"]' is
    not allowed`. TCK 0109, 0110, 0113 and 0119 are unchanged. Identity: no difference (the wasm
    module has no item definitions). Rust behaviour break (rheon-model-evaluator): an input
    typed by a collection with allowed values takes a list whose elements each satisfy them,
    including the empty list, and a rejected list names its first failing element.
128. **An iteration context sees the variables bound before it.** DMN 1.6 §10.3.2.14: "Examples
    of valid iteration contexts are: ... x..x+10", and "A for loop expression will iterate over
    each element in the iteration context, binding the element to the corresponding variable";
    §10.3.2.15 binds the variables of `some` and `every` to the elements of the Cartesian
    product of their contexts, and "When the Cartesian product is empty, the some ... satisfies
    quantifier returns false and the every ... satisfies quantifier returns true". The DMN TCK
    (compliance-level-3, 0084-feel-for-loops, "for loop with second iterator depending on first
    iterator") expects `for x in [[1,2],[3,4]], y in x return y` to be `[1,2,3,4]`. The loops
    (feel-evaluator, `iterations.rs`) evaluated every context once, before the loop, with one
    special case for a context that is the bare name of an earlier variable, which did not skip
    an empty list: `for x in [[1,2],[],[3]], y in x return y` was `[1, 2, 2, 3]`, `for i in 1..3,
    j in 1..i return j` was null, and `y in x.items` and the contexts of `some` and `every` saw
    no earlier variable. Now one walk binds the variables of `for`, `some` and `every` in order:
    the domain of the first is evaluated in the scope of the expression, the domain of each later
    one in every iteration of the earlier ones with them bound, and a domain without items
    contributes no iteration. The walk keeps one open sequence per variable, so it has no
    recursion; a variable whose sequence ended is unbound, and the implicit `partial` of a `for`
    is unbound while a domain is evaluated, so a domain sees the variables before it only.
    Numbers and dates of an interval are still produced one at a time, with the rule that a step
    lost to rounding ends a number interval. The boxed `for`, `some` and `every`
    (model-evaluator, `boxed_expressions.rs`) use the same walk with their one variable and a
    copied iteration context. So a later context is evaluated once per iteration of the earlier
    ones: its errors are counted once per iteration (the message is listed once), and after an
    empty context it is not evaluated at all. A repeated variable is bound by the later context.
    Test first: `an_iteration_context_sees_the_variables_bound_before_it` and
    `a_later_iteration_context_is_evaluated_in_every_iteration_of_the_earlier_ones`
    (`for_expression.rs`) and `a_quantified_context_sees_the_variables_bound_before_it`
    (`some_expression.rs`, `every_expression.rs`) failed on the parent with `[1, 2, 2, 3]`, a
    count of 1, `false` and null. `tests/iterations.rs` is rewritten against `Iteration` and
    `Sequence`; its dependent case with an empty list now has no iteration for it. The BACKLOG
    entry on an empty list in a `for` with a dependent variable is removed. TCK 0003, 0016,
    0033, 0084, 1152, 1153 and 1154 pass as before. Identity: 21 registered differences in
    `registered/iteration-contexts-see-earlier-variables.json`, all requests of the new committed
    source `requests/iteration.json` (47 requests: dependent, independent and single contexts,
    errors in later contexts, empty contexts, `partial`, repeated variables, `some` and `every`).
    Speed (the median of the wasm engine, Apple M2, against the module before this change):
    single-variable loops of 10,000 iterations 0.92 to 1.02, independent two-variable loops of
    10,000 iterations 0.80 to 0.97, the dependent loop `for x in xs, y in x` over 100 lists of
    100 numbers 0.20 (the value of each earlier variable was copied again in every iteration);
    against 0.4.0 0.79 to 0.93 and 0.18. Rust API break (rheon-feel-evaluator):
    `ForExpressionEvaluator`, `SomeExpressionEvaluator` and `EveryExpressionEvaluator` are
    removed; `Iteration` (`boxed`, `collect`, `some`, `every`), `Sequence` (`of`, `empty`,
    `interval`) and `Domain` are added.
129. **A repeated iteration variable gives null.** DMN 1.6 §10.3.2.14 binds each element "to the
    corresponding variable", and §10.3.2.15 binds "keys n1, n2, etc." to the elements of the
    Cartesian product; the text is silent on a variable named twice. With the contexts bound in
    order (item 128) the later binding hides the earlier one, so `for i in [1,2], i in [3] return
    i` was `[3, 3]`, where 0.4.0 kept the earlier binding and gave `[1, 2]`. Keeping either
    binding silently answers a question the expression does not ask, so now the builder checks
    the names of the iteration contexts of `for`, `some` and `every` (feel-evaluator,
    `builders.rs`, `iteration`), and a name given twice builds an evaluator of null with the
    counted reason `duplicate iteration variable 'i'`. The boxed expressions have one variable.
    Test first: `a_repeated_iteration_variable_gives_null` (`for_expression.rs`) failed on the
    parent with `[3, 3]`. Identity: the three requests of a repeated variable in
    `registered/iteration-contexts-see-earlier-variables.json` are amended in place (`for`,
    `some`, `every`: null with the reason). No API change.
130. **`matches` and `replace` read their flags by one rule.** DMN 1.6 Table 75 note 2:
    "pattern, replacement, and flags SHALL conform to the syntax and constraints specified in
    clause 7.6 of XQuery 1.0 and XPath 2.0 Functions and Operators. Note that where XPath
    specifies an error result, FEEL specifies a null result." That edition's §7.6.1.1: "If there
    are characters present that are not defined here as flags, then an error is raised
    [err:FORX0001]"; "Letters may appear in any order and may be repeated"; and `x`: "whitespace
    characters (#x9, #xA, #xD and #x20) in the regular expression are removed prior to matching
    with one exception: whitespace characters within character class expressions (charClassExpr)
    are not removed". `matches` trimmed its flags and rejected `""` (`empty regex flags`), passed
    `x` to the engine, where `#` starts a comment and whitespace inside a class is dropped too
    (three string rewrites patched part of that), and applied `q` after the block-name rewrite;
    `replace` ignored letters that are no flags, dropped `q` beside `s`, `m` or `x`, escaped every
    character under `q` (so `b` became the word boundary `\b`), ignored flags that are not a
    string, and compiled the pattern without its flags when the flagged pattern was invalid. Now
    one private `Flags` (feel-regex, `flags.rs`) reads the flags of both: every character must be
    `s`, `m`, `i`, `x` or `q`, `""` is no flags, nothing is trimmed, and there is no fallback.
    `q` is not defined by the edition DMN 1.6 cites; it is the flag of XPath and XQuery F&O 3.1
    §5.6.2, kept as an extension: "all characters in the regular expression are treated as
    representing themselves, not as metacharacters", and "If it is used together with the m, s,
    or x flag, that flag has no effect". So under `q` the pattern is quoted (only the engine's
    special characters are escaped) and only `i` applies. `x` is applied by a scanner, never by
    the engine: whitespace outside character classes is removed, also after a backslash, so
    `"hello\ sworld"` reads `hello\sworld` as the clause's example expects, and `#` is an ordinary
    character. An invalid pattern's reason quotes the pattern as written, not the engine's form
    with its `(?i)` group. In the evaluator one `matches` replaces `matches_2` and `matches_3`;
    null flags are no flags for both built-ins (TCK 1111 `fn-null-flags`), other flags that are
    not a string are null (`replace: flags must be a string`), and one helper turns a
    `RegexError` into the result of all three regex built-ins. Test first: the flag tests of
    `bif_matches.rs` and `bif_replace.rs` failed on the parent (`matches("input", "pattern", "")`
    was null with `empty regex flags`; `replace("abc","[A-Z]","#","all unknown but i")` was
    `"###"`). TCK 1111 `K-MatchesFunc-5` (flags `" "`) stays null, now with `invalid regex flag:
    ' '`. 25 registered differences (`registered/regex-flags.json`), among them
    `replace("abc","b","x","q")`, which was `"xabcx"`, and `replace("a#b","#","-","x")`, which was
    `"-a-#-b-"`; `requests/regex.json` gains the 23 requests of the new tests that it did not
    hold. API breaks (rheon-feel-regex): `is_match` takes the flags (`""` for none) and
    `is_match_with_flags` is removed; `replace` takes `flags: &str` instead of `Option<&str>`;
    `RegexError::Invalid` is renamed `InvalidPattern`, and `RegexError::InvalidArgument` is added.
131. **A replacement string follows the F&O rules.** XQuery 1.0 and XPath 2.0 F&O §7.6.3, where
    S is the number of parenthesized sub-expressions and "N is the decimal number formed by taking
    all the digits that consecutively follow the $ character": N=0 is the whole match, 1<=N<=S
    the group (empty when it did not match), "If S<N<=9, then the variable is replaced by the
    zero-length string", and "Otherwise (if N>S and N>9), the last digit of N is taken to be a
    literal character ... and the rules are reapplied"; "A literal "$" symbol must be written as
    "\$"", "A literal "\" symbol must be written as "\\"", and FORX0004 (a null in FEEL) is raised
    for a `$` "that is not immediately followed by a digit 0-9 and not immediately preceded by a
    "\"", and for a `\` "that is not part of a "\\" pair, unless it is immediately followed by a
    "$" character". `replace` rewrote `$N` to the engine's `${N}` with a process-wide compiled
    regex and handed the rest to the engine's own syntax: `$0` named no group (`[$0x]` gave
    `[]`), `$10` with one group named group 10, `$`, `$$` and `$a` were accepted (`$$` as one
    `$`), `${1}` was the engine's named-group form, and `\$` and `\\` stayed two characters.
    Now a private `Replacement` (feel-regex, `replacement.rs`) reads the string once, after the
    pattern is compiled so that S is known, into literal text and group numbers, and is the
    engine's replacer for every match; an invalid string is null whether or not the pattern
    matches, with `invalid replacement string: a '$' that is not escaped must be followed by a
    digit` or `... a '\' must be followed by '\' or '$'`. The string is read from left to right,
    so in `\\$1` the pair comes first and `$1` is a group, and `\\$` ends in a lone `$`. The
    group digits are read with saturating arithmetic, so a long run of digits cannot overflow.
    With `q`, the F&O 3.1 extension of item 130, "the replacement string is used as is" (§5.6.4).
    The process-wide `REPLACEMENT_GROUP` is deleted. The forms of the TCK (`[1=$1][2=$2]`,
    `a$1$1`, `$1c$2`, `($1) $2-$3`, `[$0]`) keep their answers; TCK 1109 passes 28 of 28. Test
    first: the replacement tests of `bif_replace.rs` failed on the parent (`replace("abc","b","$")`
    was `"a$c"`, `replace("abc","b","\\$")` was `"a\$c"`, `replace("abc","b","[$0x]")` was
    `"a[]c"`). 15 registered differences (`registered/regex-replacement.json`), among them
    `replace("abcde","(a)(b)(c)(d)(e)","$23")`, the clause's own example, which was `""` and is
    `"b3"`; the entry `replace("abc","b","$1","q")` of item 130 is amended in place from `"ac"`
    to `"a$1c"`; `requests/regex.json` gains the 9 requests of the new tests that it did not hold.
    API break (behaviour, rheon-feel-regex): `replace` reads the replacement by these rules and
    returns `RegexError::InvalidArgument` for an invalid one.
132. **`replace` and `split` reject a pattern that matches the zero-length string.** XQuery 1.0
    and XPath 2.0 F&O §7.6.3: "An error is raised [err:FORX0003] if the pattern matches a
    zero-length string, that is, if the expression fn:matches("", $pattern, $flags) returns true.
    It is not an error, however, if a captured substring is zero-length." §7.6.4 (`fn:tokenize`,
    which DMN 1.6's `split` is without flags) raises the same error, and "If $input is the empty
    sequence, or if $input is the zero-length string, the result is the empty sequence"; a
    separator at the start or the end, and two adjacent separators, give zero-length strings.
    `replace` and `split` iterated over the engine's empty matches: `replace("abc","x*","-")` was
    `"-a-b-c-"`, `replace("abc","^","-")` was `"-abc"`, `split("abc","")` was
    `["", "a", "b", "c", ""]`, `split("a,b",",*")` was `["", "a", "b", ""]`, and `split("", ";")`
    was `[""]`. Now the one compile of the two built-ins (feel-regex, `impl_regex.rs`, `compile`)
    tests the compiled pattern, flags included, against `""` once and returns
    `RegexError::InvalidArgument` with `the pattern matches the zero-length string`, a null in
    FEEL; `split` of `""` is `[]` (owner decision), and every other input keeps the engine's
    split, which already gives tokenize's zero-length strings (`split("1,15,,24,50,",",")` is
    `["1", "15", "", "24", "50", ""]`, the clause's example). The pattern is checked before the
    input, as an invalid pattern is, so `split("","x*")` is null, not `[]`; an engine limit and an
    invalid pattern come before the zero-length test. Test first: the zero-length tests of
    `bif_replace.rs` and `bif_split.rs` failed on the parent (`"-a-b-c-"`, `[""]`, `["", "a",
    "b", "c", ""]`). TCK 0067 and 1109 use no such pattern and pass. Every entry of
    `registered/regex-engine-limits.json` still reproduces (`split("a", <100 nested groups>)` is
    `["", ""]`). 11 registered differences (`registered/regex-zero-length-matches.json`);
    `requests/regex.json` gains the 13 requests of the new tests that it did not hold. API break
    (behaviour, rheon-feel-regex): `replace` and `split` return `RegexError::InvalidArgument` for
    a pattern that matches the zero-length string, and `split` returns no item for an empty
    input.
133. **CI documents the published crates with warnings denied.** Job `engine` runs `cargo doc
    --no-deps` on the ten published crates with `RUSTDOCFLAGS=-D warnings`, so an intra-doc link
    that resolves to nothing, or any other rustdoc warning, fails the run; the README's script
    table names the step. The module docs of rheon-feel-wasm linked its three ABI exports, which
    exist only on wasm32, and now write them as code, so that crate documents without a warning
    too. The step was checked against a tree whose rheon-feel-number docs still quoted DMN 1.6's
    "pairs [that] have the same value" with bare brackets (item 97 escapes them): it failed there
    with `unresolved link to 'that'` (exit 101), and it passes here. No API change.
134. **CI runs on Node 24.** The identity check answers every registered request with the
    previous release's module. One of them,
    `{big: for i in 1..100000 return i, r: for j in 1..1000 return big = j}.r`, makes the 0.4.0
    module exhaust its memory while it builds its null reasons (item 68 bounds them). Node 24 and
    Node 26 report that as a trap of the module, which the ledger registers. Node 22.23 crashes
    itself with a segmentation fault instead (6 of 6 runs on macOS arm64, and on the ubuntu
    runner), so the check ended with status 139. The `ci` and `release` workflows now set up
    Node 24, and `tools/identity/README.md` states the requirement. A request that exhausts the
    memory of this module (a list doubled 30 times) ends as a trap on Node 22 as well. For
    hosts: a 0.4.0 module on Node 22 can take the whole process down on such a request, not only
    its worker. No API change.
Commits 135-177 make the engine robust on values that hold the same list or context on many paths,
and faster on loops, filters and calls, with tools first. The identity check gains rule entries, the
release tier's request sources and ledger and a cache test, the TCK run pinned inputs, and the three
test tiers become commands with budgets that CI runs (135-145); an abort stops every loop (146). The
value rework gives contexts and lists a type and a read API of their own, keeps their nesting depth
and bounds it at 500 levels, shares lists, contexts, scope frames and closures, and makes every walk
that compares, tests, types or indexes a value visit a shared list or context once, with request
sets, one iteration path and one filter path for FEEL and boxed expressions on the way (147-168).
Then `and` and `or` evaluate the right operand only when the left does not decide, a filter
evaluates its expression once in the enclosing scope, `sort` and `list replace` call their function
in one scope as a counted call, and the arithmetic operators are named kernels (169-174); the last
commits commit the depth probes of the large request sets that 0.5.0 answers (175-177). Each change
of a wasm answer against 0.5.0 that the identity check finds is registered with both answers or
explained by a rule entry, and counted in its item; changes that only DMN or the Rust API show are
stated in their items. The totals are under "Verification" below.

135. **The benchmarks that need a nightly toolchain are removed.** Five crates had a `benches/`
    directory (model-evaluator 156 files, feel-number 11, feel-parser 6, feel-evaluator 4, feel 3;
    36,510 lines), and every file needed `#![feature(test)]`. `rust-toolchain.toml` pins a stable
    compiler, so no gate and no CI step compiled them, and `cargo check --workspace --all-targets`
    failed on the first of them (E0554). The model-evaluator ones were a second copy of the TCK
    tests in `src/tests/compatibility`: 3,591 `#[bench]` against 3,594 `#[test]`, kept in step by
    hand, and 16 of the 151 copied files already held other string literals (expected answers and
    input contexts) than their tests, 177 in all. The answers stay pinned by the tests, and the
    speed of the engine is measured on the wasm module. Removed with them: the Taskfile tasks
    `bench`, `benchmarks-dry` and `benchmarks`, the `benchmarks.txt` ignore line, and the two
    model-evaluator tests that only printed the number of tests beside the number of benches (so
    the workspace runs 2 tests fewer), with the `antex` and `walkdir` dev-dependencies they alone
    used; the table at the top of the compatibility tests loses its benches column. A benchmark
    of the TCK models, when one is built, runs on the stable toolchain and states no expected
    answer: it takes the models and inputs from the pinned TCK checkout of `scripts/tck.sh` and
    times the model build (parsing and `ModelEvaluator::new`) and each test case's evaluation with
    its own timing loop, one row per model, so nothing in it can drift from the tests. No API
    change.
136. **A ledger entry of the identity check can state a rule.** A change that removes the same
    kind of diagnostic from many requests needed one ledger entry per request. A ledger file may
    now also hold rule entries (`tools/identity/README.md`, "Rule entries"): a name, the basis, an
    optional pattern the request's expression must match, the paths where the two responses may
    differ, each with its relation (`gains` or `loses` elements that match a pattern, `decreases`
    or `increases` as exact integers, `rewrite` by a pattern and its replacement), and examples in
    the whole-response form. A difference of a request with no entry of its own is explained by
    the first rule, in ledger order, whose relations hold at its paths and outside of which the
    two responses are identical; a trap is never explained. Each example is run like an entry
    (the reference answers `old`, the candidate `new`) and must be an instance of its rule, so a
    rule cannot go stale, and an example's request is registered at most once like any other.
    The summary counts the explained differences per rule, `--explained <out.json>` lists them
    for review, and `--record` drafts only what nothing explains. The relations are pure
    functions in `rules.mjs`, tested on literal responses (`npm run test:identity`, 18 tests, the
    6 that need two modules that differ skipped while the ledger is empty), and the lossless JSON
    helpers move to `json.mjs`; the cache format is 2 and its key covers the three files. Checked
    by hand with the 0.4.0 module as the reference and 0.5.0 as the candidate on
    `requests/nulls.json`: a rule for a dropped `no entry` error (count down, messages lose
    `no entry`) with one example explained 3 of the 120 differences, the other 116 stayed
    differences, and the same rule stated as an increase was refused (exit 2, `diagnostics.count:
    1 to 0 is not an increase`). No API change.
137. **The identity check has request sources and a ledger for the release tier.** Some requests
    cost the 0.5.0 module seconds each before it exhausts its memory. The two large request sets
    kept outside the repository were run once against 0.5.0 (7,845 requests with the committed
    ones, 480 s, peak 3.5 GB): 22 trap both sides, 19 of them on memory exhaustion after more than
    a second each and 3 at once. The 17 of those 19 that are short texts are committed as
    `tools/identity/release/requests/memory-exhausting.json`: 15 that double a list or a context
    30 times through `partial` and then count, compare, search, render or walk it (1.2 to 1.7 s
    each), and 2 that nest a list or a context 100,000 levels deep by a loop (158 s and 35 s); the
    other 2 are 2 MB texts (100,000 context entries that each nest the one before, 1.2 s each)
    and stay in the large set. `tools/identity/run.sh` reads `release/requests/` and the ledger
    `release/registered/` only with `--release`; otherwise `equal.mjs --exclude` (new) leaves
    those requests, with their parse twins, out of every other source and counts them in the
    summary, so no run after a build evaluates them, while a ledger entry is always run.
    `run.sh --large` adds the sets that `IDENTITY_LARGE_SETS` lists (colon-separated), and the
    npm scripts `identity` and `identity:large` run the check with and without them. A change
    that alters the answer of a release-tier request registers it in `release/registered/`; at a
    release that directory is emptied with `registered/`. `run.sh --release` against 0.5.0: 7,534
    requests, the 17 trap both modules, 0 differences, 427 s, peak 3.7 GB; the run without it is
    unchanged (7,502 requests, 0 left out). No API change.
138. **The TCK run uses pinned inputs and fails on a changed result.** `scripts/tck.sh` cloned the
    official TCK, DecisionToolkit's patches and its runner at the head of their default branches,
    and printed the comparison with `docs/tck-baseline.txt` but always exited 0, so it could not
    serve as a gate and its inputs could move under it. Now `scripts/tck-pins.txt` names the
    commit of each (tck `5cd912f`, dsntk-tck-patches `8dfc54e`, dsntk-test-runner `3276110`, the
    inputs of the recorded baseline); a clone already at its pin is used offline, any other is
    fetched at the pin, and `--refresh` is gone (moving a pin is a commit with the baseline it
    gives). `scripts/tck-summary.mjs` sets the exit status: 1 for a newly failing or newly passing
    test, a baseline test with no row, or a result file without rows, 2 for a usage error; with
    `--only` or `--addendum-only` the script passes `--partial`, which compares only the tests that
    ran. `--baseline <file>` names another baseline, and its checks are
    `npm run test:tck-summary` (7 tests, failing on the previous summary, which took no options
    and exited 0). The runner is built into its own target directory and the server is found
    through `cargo metadata`, so the run works with any `CARGO_TARGET_DIR`. Run here: official
    3,392 of 3,407, addendum 312 of 312, 0 newly failing, 0 newly passing, 0 not run, exit 0, 86 s
    with the server's release build. No API change.
139. **The three test tiers are commands with measured times and budgets.** `scripts/tier.mjs`
    (`npm run tier:fix`, `tier:group`, `tier:release`; the README's "Test tiers") runs each tier as
    a list of steps and ends with a table of every step (build or run, seconds, result), the build
    and run totals, the budget and the verdict. The fix tier runs format, compilation of every
    target, the fix's own test, the tests of the touched crates (named with `-p` or derived from
    the files that differ from the merge base with `main` through `cargo metadata`), the wasm build
    and the identity check, adds the Rust copies of the TCK tests and each release-tier ledger
    file when a ledger changes and a tool's own tests when the tool changes, and stops at the
    first failure. The group tier runs the whole workspace and every check of the module and the
    official TCK; the release tier adds the large and slow checks, and requires the inputs kept
    outside the repository unless a `--without` flag lists the step as NOT RUN. Test steps are
    split into their `--no-run` build and their run, and the budget (fix 2 min, group 6 min,
    release 30 min) judges the run total only; with `CI` set the times are reported without a
    budget. `npm run test:tier` checks the runner on step tables of its own (a failing step,
    stopping, a skipped dependant, the budget, a missing outside input, usage errors). The
    toolchain gains the `clippy` component, so clippy is the pinned one everywhere.
    `scripts/build-wasm.sh` and `scripts/tck.sh` find cargo's target directory through the new
    `scripts/cargo-target-dir.sh` instead of assuming `./target`, and `RHEON_WASM_TARGET_DIR` gives
    the tiers' wasm steps a target directory of their own. The fix tier of this change: 3.0 s of
    run time, 25.9 s of builds. No API change.
140. **CI runs the group tier on pull requests, and the release tier before a release.** The two
    jobs `engine` and `wasm` of `ci.yml` become one job `group` that sets up the toolchain, Node
    24, the wasm tools and the oracle's Python environment and runs `npm run tier:group`, then
    uploads the tier's logs, `feel.wasm` and the playground; the Pages deploy needs it. The group
    tier adds clippy with warnings denied, the tools' own tests and the official TCK at its pinned
    commits to what CI ran. `release.yml` gains a job `tier` that runs `npm run tier:release --
    --without-large-sets --without-speed-suite` on a tag and on a manual dispatch, and the
    publishing job needs it, so a failing tier builds and publishes nothing; the large request sets
    and the speed suite stay outside the repository, so that run lists them as NOT RUN and a
    release is fully checked only by the release tier on the reference machine. `cargo doc` of the
    published crates (item 133) moves from every push to the release tier. On CI the tiers report
    their times without a budget until five runs give one (1.5 times the slowest, the job timeout
    twice that); until then the jobs time out at 60 and 120 minutes. The documents that named the
    CI jobs name the tiers. The workflows were not run here: they are seen on the first push. No
    API change.
141. **The identity check's Node requirement and times are stated against 0.5.0.** The identity
    tool's README said to use Node 24 or later because one registered request made the 0.4.0
    module crash Node 22; with 0.5.0 as the reference no committed request does, so it now says
    that Node 21 or later is enough against 0.5.0 and that Node 24 or later is still needed for a
    reference of 0.4.0 or older (a bisect of older commits, a comparison with an old release).
    `engines.node` stays `>=21`, and CI stays on Node 24, the version its times are measured on, as
    the workflow comments now say. The table of times is measured again against 0.5.0 (Apple M2,
    Node 26): the committed sources 7,502 requests, 1.0 s uncached and 0.6 s cached (8.4 s and 7.5 s
    against 0.4.0); with the large sets 7,813 requests, 38 s uncached and 22 s cached; the release
    tier's `--release --verify-cache` 7,534 requests in 427 s and `--large --verify-cache` in 39 s,
    0 cache mismatches, peaks 4.1 GB and 2.2 GB. The README gives the times of the tiers from their
    first runs: the group tier 62.1 s of run time and 15.4 s of warm builds, every step passing;
    the release tier 1,029 s of run time and 51 s of builds, every step passing except the speed
    suite, whose `bench.mjs` (outside the repository) loaded a comparison engine that is not
    installed here; with that load made optional it ran in 54 s, a geomean of 1.00x between the
    release module and the build of the same engine. Running the committed sets on Node 22
    against 0.5.0 was not checked here (no Node 22 is installed). No API change.
142. **Requests and tests on values that are used again after they were passed on.** The value
    rework changes how lists, contexts and frames are stored and shared, and its risk is a write
    through one holder of a value that shows through another. `tools/identity/requests/values.json`
    (105 requests) uses a list, a context or a string again after a built-in, an operator, a
    filter, a loop, a call or a closure derived a new value from it (`append`, `context put` with
    a key and a key list, `sort`, `reverse`, `list replace`, `insert before`, `remove`, `distinct
    values`, `union`, `concatenate`, `flatten`, `sublist`, `context merge`, `get entries`, `+` on
    strings), with items with and without an entry `item` and the input context aliased in both
    context formats; it nests a value 100 and 499 levels deep through each channel that builds one
    (`partial`, a recursion around list literals, a chain of context literals, ranges, closures,
    `context put` with a key list) and walks it with `count`, `=`, `instance of`, `string`,
    `flatten` and `...`; and it doubles a list and a context 10 times through `partial`, so their
    items share the value before them. 0.5.0 answers every one of them (0.8 s for the set). A list
    and a context nested 499 levels deep through `partial`, and their walks, cost 0.5.0 0.7 s and
    5.6 s each, because it copies `partial` in every iteration, so these 12 requests are
    `tools/identity/release/requests/values-deep.json` (44 s on each side). The same expressions,
    except those 12, are tests in `feel-evaluator/src/tests/values_used_again.rs` with the answers
    they have (the value text and the number of errors); the deep values are compared with their
    text built by the test. Identity against 0.5.0: 7,710 requests in 2.7 s uncached and 1.4 s
    cached; with the large sets 7,905; with `--release` 7,766 in 513 s (peak 3.5 GB); 0
    differences. No API change.
143. **What a boxed context writes into an iteration or an item frame is pinned.** A boxed context
    writes each named entry into the context on the top of the scope. In the body of a boxed
    `for` that is the iteration context, as the match of a boxed filter the context of the item;
    no FEEL expression reaches this path, so the identity check cannot see it. The model of
    `model-evaluator/src/tests/various/dmn_0008` pins today's answers: an entry written in one
    iteration or for one item is not seen by the next (`[1, 2, 3]`; `[{a: 2}, {a: 3}]`), nor by
    the entries before it (`[[0, 10], [0, 20], [0, 30]]`; all three items); an entry named
    `partial` replaces the results of the iterations before it, by the list it holds (`[3, 1]`
    for `[x]` over `[1, 2, 3]`) or by no results when it is not a list (`[3]`); an entry named
    `item` is read by the entries after it and leaves the items as they were (`[2, 3]`); and an
    entry named as an entry of a context item is not read by the entries after it (`[]` for
    `a: a * 10` then `a > 15`). A change to how frames are stored or shared that alters one of
    these answers states it here. Tests only; no API change.
144. **The ordering of `sort` takes a predicate that can stop.** `order_by`
    (`feel-evaluator/src/bifs/sorting.rs`) took a predicate that always answered, so a sort ran to
    its end whatever the predicate's calls did. The predicate now answers `Some(precedes)` or
    `None`, and at the first `None` the ordering returns `None` without calling it again, from the
    insertion of a short list, the runs of a long one and its merges alike (the internal helpers
    pass the stop on with `?`). `sort` answers `Some` for every call, so nothing it returns changes;
    a `None` would give null. The unit test stops the predicate at its first call, at a call during
    the runs and at a call during the merges, for 2 to 5,000 items, and checks that the predicate
    was called exactly that many times; on the parent it does not compile, because the predicate
    cannot stop there. Identity step; no API change (the function is private to the crate).
145. **The identity tool's cache test holds when the recorded differences trap the reference.**
    `npm run test:identity` checks that a run with a warm cache answers as one without, on
    requests of the committed ledger that the two modules answer differently. It asserted that
    some of those answers came from the cache, but a trap is never cached (it is evaluated again
    on every run), so a ledger whose entries all trap the reference failed the test although the
    cache worked as specified. The test now asserts the exact counts: every answer of the cold
    run comes from the cache except the traps, which are evaluated again. With the ledger empty
    the test is skipped as before. No API change.
146. **An abort stops every loop.** An evaluation stopped by a limit (the call depth, a regex engine
    limit) refused every further call, but a loop went on through its items: a `for` over 10,000,000
    items held a null for each and exhausted the module's memory (a trap where the evaluation error
    was due), an `every` over 10,000,000,000 combinations ran for minutes, and the work after the
    abort also recorded errors no one reads. `rheon_feel::diagnostics::stopped()` (new, public; its
    callers are in rheon-feel-evaluator) tells whether the running evaluation was stopped, from the
    same state the call guard reads, and is `false` outside an evaluation. The walk of `for`, `some`
    and `every` (`Iteration::walk`, also the boxed forms) reads it before each further item, a
    filter before each item and before it evaluates its expression once more for an index, `sort`
    before each comparison (its ordering then ends, item 144) and `list replace` after each call of
    its function; each then ends with a null, which is never observed, because the abort is the
    outcome. An evaluation that ends answers as before. Two answers change, registered in
    `tools/identity/registered/abort-stops-loops.json` with their requests in
    `wasm/feel-wasm/tests/corpus/no-trap.json`: `for i in 1..10000000 return f(i)` and `for i in
    1..100, j in 1..100000 return f(j)` with a runaway `f` trapped 0.5.0 after 0.4 s and are now the
    call-depth `evaluation` error (DMN 1.6 §7.3.8: the evaluation "MUST report or log diagnostic
    information"; CONTRACT §2). The corpus also gains the same loops, filters, `sort` and `list
    replace` over 100,000 items, which 0.5.0 already answered with that error, and the contract test
    asserts the error for each kind of loop. Natively, `diagnostics::collect` around an aborted
    evaluation now sees the errors recorded up to the abort only: in the tests, a loop over 1,000
    items records as many as the same loop over its first item (2 against 1,001 for `for i in
    1..1000 return [f(i), 1 + null]`), and `list replace` no longer records "matching function must
    return boolean value" for the refused call. Measured (wasm, Node 26): the traced `for i in
    1..1000000 return f(i)` 107.5 ms on 0.5.0 and 0.4 ms now (a second call on one instance); `every
    i in l, j in l satisfies f(j) = null` over two lists of 100,000 items 397 s on 0.5.0 and 20 ms
    now (most of it building the list); `sort` and `list replace` over 100,000 items 37 ms and 40 ms
    on 0.5.0, 11 ms each now. The speed suite against 0.5.0 in two alternating rounds: geomean
    1.005x and 1.003x over 39 workloads, 1.001x and 1.002x over the 86 parse workloads, no workload
    below 0.97x in both rounds (`loop-count-10k` 1.012x and 1.003x, `list-filter-count` 1.009x and
    0.995x, `sort-10k` 1.004x and 1.006x, `list-replace-10k` 1.006x and 1.001x, `fib15` 1.051x and
    1.002x). Allocations, reallocations and peak live bytes per call are those of 0.5.0 for every
    workload (native counting harness); the stack report and the string doubling's peak memory (n =
    17 to 26) are identical. Rust API: the added function `rheon_feel::diagnostics::stopped`.
147. **A context has its own read API and no longer derefs to its map.** `FeelContext`
    dereferenced to the `BTreeMap` that holds its entries, so every caller could read the map
    directly and the way a context stores its entries was part of the API. The context now reads
    through its own methods: `iter` (an `ExactSizeIterator`, in the order of the names),
    `keys_from(name)` (the names from the first one not less than `name` on, which
    the parser's bound on the keys whose first word is a given word reads), and `contains_entry`,
    which takes a `Name` or its text. The parser, `context` and `=` read the context through
    them. Identity step: no answer changes. Rust API (`rheon-feel`): `impl Deref for FeelContext`
    is removed (use the methods above, `get_entry`, `len` and `is_empty`); `FeelContext::iter`
    returns `impl ExactSizeIterator<Item = (&Name, &Value)>` instead of
    `std::collections::btree_map::Iter`; `contains_entry` is generic over `Q` with `Name: Borrow<Q>`
    (a `&Name` argument compiles as before); `keys_from` is added.
148. **The named-parameter and positional-parameters values that nothing builds are removed.**
    `Value::NamedParameter` and `Value::PositionalParameters` came from upstream, but no code builds
    either value (the evaluator binds an invocation's arguments from the syntax tree), so they were
    matched only to display them and to give their type `Any`. Identity step: no answer changes.
    Rust API (`rheon-feel`): the variants `Value::NamedParameter` and `Value::PositionalParameters`
    are removed (`Value::NamedParameters` stays).
149. **`assign_entry` returns the value it replaces.** `FeelContext::get_entry_mut` handed out a
    `&mut Value` into a context, a write that the context cannot see; the next commits keep the
    nesting depth of a context, which such a write would bypass. `FeelContext::assign_entry` now
    returns the value it replaces (`None` for a new entry), and the two callers of
    `get_entry_mut` take their value back through it: `for` takes `partial` out of the iteration
    context, and a filter takes the item out of its `{item: …}` context, by assigning a null
    placeholder, as they did by hand. When the item context no longer has `item` the filter gives
    the lost-context null as before. Identity step: no answer changes. Rust API (`rheon-feel`):
    `FeelContext::get_entry_mut` is removed; `FeelContext::assign_entry` returns `Option<Value>`.
150. **A list value is a type of its own.** `Value::List` held a `Vec<Value>`, so every holder of a
    list could change its items in place, and how a list stores its items was part of the API. It
    now holds `FeelList` (`rheon_feel::values::FeelList`), which reads its items as a slice
    (`Deref<Target = [Value]>`), changes only through its own `push`, and gives its
    items up with `into_vec`; it is built from a `Vec<Value>` without a copy (`From`), from an
    iterator (`FromIterator`), iterates by value and by reference, compares its items and prints as
    the slice of items in its debug form, so `Value::List` debug-prints as before. There is no
    `&mut` access to the items. The built-ins that edit a list (`insert before`, `insert after`,
    `remove`, `reverse`, `list replace`) take a copy of the items with `to_vec`, as they cloned the
    list before, and build the new list from it; `for` keeps its results as a `FeelList`; a filter
    and a loop over a list take its items out with `into_vec`, as they moved the `Vec` out before.
    Identity step: no answer changes. Rust API (`rheon-feel`): `Value::List` holds `FeelList`
    instead of `Vec<Value>` (build one with `vec.into()` or `FeelList::from(vec)`, read it as a
    slice, edit it through `into_vec`); `FeelList` is added; `values_to_string`,
    `values_to_feel_string` and `values_to_jsonify` take `&[Value]` instead of `&Vec<Value>` (a
    `&Vec<Value>` argument compiles as before). `Values` stays `Vec<Value>`, the type of the other
    variants that hold several values.
151. **Lists and contexts keep their nesting depth.** A limit on how deeply values nest needs the
    depth of a value where it is produced, and a walk over the whole value there would make every
    loop quadratic. A list now keeps the nesting depth of its deepest item: a push raises it in
    constant time and a list built from its items computes it in the same pass (a list has no other
    way to change). A context remembers its depth once it was read, until an entry changes: every
    method that changes an entry (`set_entry`, `insert`, `assign_entry`, `remove_entry`, `set_null`,
    `zip`, `overwrite`, `move_entry`, `create_entries`, `create_entry`, `apply_entries`) goes
    through one accessor that forgets it, so a write costs one store, and a context that is only
    written, such as the iteration context of a loop, never computes it; the next read looks at each
    entry once. The remembered depth is no cache: it is derived from the entries alone, it is exact
    whenever it is set (it goes from not known to the depth on a read, and back on a change), a
    clone copies it with the identical entries, and two reads of one context, through any handles
    and on any threads, store the same number. A context kept exact on every write instead, as a
    list is, made `count(for i in 1..10000 return i)` 0.70x as fast as 0.5.0 (three writes of the
    iteration context per item), and 0.88x with the depth of a scalar read inline. The depth of a
    scalar is 0; a list or a context is one level deeper than the deepest value it holds, so an
    empty one is 1 level deep; a function is one level deeper than its closure context, so 2 when it
    captures nothing; a range, an interval end, a unary test and the other values that hold values
    are one level deeper than what they hold. Nothing reads it yet. The tests in
    `feel/src/tests/depth.rs` change random contexts 60,000 times through every method that writes
    an entry, build random lists by push, from a vector and from an iterator, and compare each depth
    with a walk of the whole value, the changes that make a value shallower included. Equality,
    cloning and the debug forms are unchanged; `Value` stays `Send + Sync` (the remembered depth is
    an atomic). Identity step: no answer changes; no API change (the depth is read inside
    `rheon-feel` only).
152. **Values nest at most 500 levels within an evaluation.** Nothing bounded how deeply values
    nest, and every consumer of a value (its drop, display, `string`, `=`, the response JSON, the
    parser's index of an input context) walks it recursively, so a value deep enough ran out of the
    stack: a recursion or a loop that wraps a value 9,500 times, and `context put` with a list of
    100,000 keys, trapped 0.5.0. A value 501 levels deep now stops the evaluation with
    `Abort::ValueDepth` ("values are nested too deeply (more than 500 levels); evaluation stopped"),
    an `evaluation` error, or a `context` error for a FEEL-format input context (CONTRACT §1 to §3).
    Every consumer on every host walks at least 1,193 levels (measured on 0.5.0), twice the limit
    and more. The depth is that of item 151, and the limit is read where an expression produces a
    value, by `rheon_feel::diagnostics::bound_depth`: the result of every invocation (`guarded`, so
    every built-in and user function, FEEL and boxed), a list, range and context literal (each
    entry, and the result), the results of a `for` after each item (their growth through `partial`;
    `bound_list_depth` reads the depth the list keeps), a function definition (its closure context),
    a filter of a value that is no list (the list that holds it is a level deeper), and in
    `rheon-model-evaluator` the result of every boxed expression (`build_coerced_result_evaluator`,
    which a boxed list, context or relation passes) and each entry of a boxed context after its
    coercion (a collection type puts a value that is no list into one). It is not read where a value
    is bound: a scope frame is a context, and a frame that binds a value 500 levels deep to a
    parameter, a loop variable, `item` or `partial` is 501 levels deep, which is allowed. `context
    put` with a key list refuses a list of more keys than the value may still nest
    (`FeelContext::apply_entries`) before it builds a context, so 100,000 keys stop at once; a key
    list whose path is blocked by an entry that is not a context builds nothing and gives null
    whatever its length, as on 0.5.0 (the existing part of the path is read first, without
    recursion). Inside an evaluation the null that takes the place of the deeper value is never
    observed, because the abort is the outcome; outside one it carries the abort's text. Test first:
    `feel-evaluator/tests/value_depth.rs` checks each channel at 500 levels (the depth of the
    answer) and at 501 (the abort, and a clean next evaluation), a 500-level value bound and
    returned through each kind of frame, and loops and a key list that would nest 100,000 levels; on
    the parent the 501 cases answer values 501 and 502 levels deep.
    `model-evaluator/src/tests/various/dmn_0009` does the same for a boxed list, a boxed context and
    a boxed context entry of a collection type, and fails without the boxed checks; the contract
    test asserts both error kinds with their texts. Answers change, each registered with both
    answers: `tools/identity/requests/value-depth.json` (54 requests, every channel at 500 and 501
    levels, the 500 ones, two blocked key paths and an `instance of` at 500 levels identical, and
    the seven 501-level depth probes of the large sets of item 176) and the two `context put`
    requests with 100,000 keys in `no-trap.json` in `tools/identity/registered/value-depth.json` (26
    entries); the four requests of `tools/identity/release/requests/value-depth-deep.json` (also in
    `no-trap.json`), which trapped 0.5.0 after 0.6 to 1.3 s, and the two loops of
    `memory-exhausting.json` that nest a list and a context 100,000 levels deep, which trapped it
    after 158 s and 35 s, in `tools/identity/release/registered/value-depth.json`. Basis: an
    implementation limit, reported under DMN 1.6 §7.3.8 (the evaluation "MUST report or log
    diagnostic information"), at the limit the owner approved. No committed request of an earlier
    source and no official TCK test nests a value beyond 500 levels: their answers are unchanged.
    Measured (wasm, Node 26, Apple M2) on the module of this commit and the next against 0.5.0, two
    alternating rounds: geomean 0.996x and 0.988x over the 39 workloads of the speed suite, 0.997x
    and 1.000x over the 86 parse workloads; below 0.97x in both rounds `loop-count-1k` (0.964x,
    0.959x), `bif-loop-10k` (0.963x, 0.952x) and `fib15` (0.955x, 0.939x), the cost of items 149 to
    152 together (about 1 to 2 % each on a tight loop, measured per item: moving `partial` out
    through `assign_entry`, the list type, the depth bookkeeping, and the check of every call's
    result); reading the depth of a loop's results by value instead of from the list cost `count(for
    i in 1..10000 return i)` 8 %, and this commit reads the field the list keeps. Allocations and
    reallocations per call are those of 0.5.0 for all 39 suite and 86 parse workloads; the peak of
    live bytes grows by 8 to 2,048 B (a context is 8 bytes larger natively for its remembered depth;
    `list-filter-count` 22,459,853 to 22,459,885 B). The string doubling's peak memory (n = 17 to
    26), 2,000 calls (0 B) and the stack report are those of 0.5.0. The walkers of a value built
    natively (drop, display, `string`, JSON, the response's type, `=`, `instance of`, `flatten`,
    `...`, the parser's index) reach at least 1,265 levels on every host, as on 0.5.0. Rust API: the
    added variant `rheon_feel::diagnostics::Abort::ValueDepth` and the added functions
    `rheon_feel::diagnostics::bound_depth` and `bound_list_depth`; `FeelContext::apply_entries`
    returns an error (and stops a running evaluation) for a path that would nest the value more than
    500 levels deep.
153. **The stack check nests a value to the depth limit.** `npm run stack` evaluated five
    constructs nested to the AST depth limit, but no value nested to the value depth limit of item
    152, so a change that makes the frames of a walk over a value larger could take a host's stack
    below 500 levels unseen. `scripts/stack-headroom.mjs` now also evaluates on every host it
    checks a value nested 500 levels deep: a `for` builds it through `partial` (its results 500
    levels deep, `partial` held in a scope frame one level deeper), and it is passed to a function,
    compared with itself, returned, rendered and dropped. On this module it passes on Node's main
    thread and in a 4 MiB worker with Liftoff and with TurboFan and in Wasmtime with a 4 MiB wasm
    stack (7.0 s for the whole check), as on 0.5.0; with the depth set to 501 every host reports the
    construct as failed, where the limit stops the evaluation. Tooling; no API change.
154. **Lists share their items.** A list held its items in a vector of its own, so every copy of
    a list (a name read, an argument passed, an item taken out of a list, `partial` in a loop)
    copied all its items and theirs: `count(items)` over 10,000 contexts read by name copied the
    10,000 contexts, and a list that a loop doubles 30 times through `partial`
    (`if i = 1 then [1] else [partial[-1], partial[-1]]`) held 2^29 copies of its first item and
    exhausted the module's memory. `FeelList` now holds its items behind a reference count: a copy
    shares them, the empty list allocates nothing, and the items are never changed while they are
    shared (DMN 1.6 §10.3.2.5: "Lists are immutable and may be nested"). `push` appends in place
    when the list holds its items alone and to a copy of them otherwise; `into_vec` moves the
    items out of a list that holds them alone and clones them otherwise (a shallow copy: each
    item clone shares what the item holds); taking the items by value moves them out or clones
    them one at a time as they are taken, so a loop over a list read by name copies no vector of
    items first, and a loop domain that is a single value is held as such. The depth of item 151
    is kept as before. Tests first: `feel/src/tests/lists_shared.rs` (a copy shares the items; a
    push, `into_vec` and taking the items of a copy leave the other copy unchanged; the items of a
    list that holds them alone are moved out; the empty list; a list of contexts of strings copied
    into 8 threads and changed in each leaves the original unchanged), failing on the parent where
    a copy does not share; and in `feel-evaluator/tests/allocations.rs`, `count(items)` and
    `{f: function(l) count(l), r: f(items)}.r` over 10,000 numbers and 10,000 contexts read by
    name cost at most 8 allocations and under 4 KiB more than over one item (0 allocations and
    0 B more; on the parent 639,936 B more for `count(items)` over numbers, the copy of the
    items). Answers change: 7 requests of
    `tools/identity/release/requests/memory-exhausting.json`, which exhausted the memory of 0.5.0
    after 1.2 to 1.7 s, now answer (`tools/identity/release/registered/shared-lists.json`):
    `count` of the doubled loop (30, in 8 ms), `=` of two doubled lists and of one with itself
    (true), `instance of list<Any>` (true), `distinct values` (1 item), `index of` ([1, 2]) and
    `union` (1 item). On this module the last six take 5.1 to 21.7 s each, because equality and
    the type of a list walk a shared item again on every path that reaches it (2^28 to 2^29
    visits). Rendering the doubled list, `string` and `flatten` of it, whose results are
    exponential in size, and the 5 requests that double a context still exhaust the memory. No
    other committed request and no official TCK test changes its answer. Measured (wasm, Node
    26, Apple M2) against the module of item 153, 2 rounds: `count(items)` over 10,000 contexts
    1.197x (8.1 to 6.8 ms), over 10,000 numbers 1.082x, `{f: function(l) count(l), r: f(items)}.r`
    1.430x, `count(for x in items return x.amount)` 1.015x, `count(for i in 1..10000 return i)`
    1.019x, `sum(items[amount > 100].amount)` 1.036x, `count(items[status = "open"])` 1.023x.
    Rust API (`rheon-feel`): `FeelList`'s `IntoIterator` by value yields the added
    `rheon_feel::values::ListItems` instead of `std::vec::IntoIter<Value>`; `FeelList` and
    `ListItems` are `Send + Sync`.
155. **Equality compares each pair of shared lists once.** FEEL equality (`=`, `!=`, `in`, and the
    list built-ins that compare items) walked both values along every path, so after item 154 a
    value that holds the same list on many paths took time exponential in its size: `x = x` for
    a list doubled 30 times through `partial` took 10.9 s, 2^29 comparisons of its first item,
    and doubled 60 times it would not end. The walk (`feel-evaluator/src/equality.rs`, moved out
    of `builders.rs` unchanged otherwise) now remembers, for one comparison, the answer for each
    pair of lists it compared where either list shares its items (`FeelList::is_shared`), keyed
    by the addresses of their items (`FeelList::address`), and answers such a pair again from
    memory. The memory is created when the first shared pair is met and dropped with the
    comparison; it is never iterated. It is exact: a pair is reached again only through two paths
    that meet in a list held twice, which is shared, and the borrowed values keep every address
    in use for the whole comparison; the two values compared are never looked up, and a list
    that holds its items alone costs one read of its reference count. There is no shortcut for a
    list compared with itself, because equality is not reflexive (a list of functions is not
    equal to itself). Tests first, in `feel-evaluator/src/tests/shared_values.rs`: lists doubled
    60 times compared with themselves, with a list doubled alike, and with one that differs in
    its last item, and lists of functions doubled 60 times (false, as before) answer at once,
    while on the parent the test was still running when it was stopped after 180 s; and equality
    on 200 random sets of values that share their lists answers as on the same values with
    nothing shared. Identity step: no answer changes. On this module the requests of
    `release/registered/shared-lists.json` with `=`, `distinct values`, `index of` and `union`
    take 7 to 9 ms (5.1 to 14.4 s before); `instance of` still takes 21.8 s. Rust API
    (`rheon-feel`): the added `FeelList::address` and `FeelList::is_shared`.
156. **The list built-ins that compare items share one equality walk per call.** `distinct
    values`, `index of`, `list contains`, `union`, `intersection` and `in` with a list compare
    many pairs in one call, each with an equality walk of its own (item 155), so a pair of shared
    lists below the items was compared again in every comparison that reached it: over 2,000
    copies of a list doubled 60 times, each comparison walked the 59 levels again. Each call now
    passes one walk through all its comparisons, so such a pair is compared once per call. The
    items and the match themselves are compared as the two values of a comparison are, without
    a lookup, so a scan over lists that are not shared below their items creates no memory. Test:
    in `feel-evaluator/src/tests/shared_values.rs`, these built-ins and `in` answer on 100 random
    sets of values that share their lists as on the same values with nothing shared. Identity
    step: no answer changes. Measured (wasm, Node 26, Apple M2, 2 rounds) against the module of
    item 155, with `d` a list doubled 60 times: `count(distinct values(for i in 1..2000 return d))`
    11.1 to 0.53 ms, `count(index of(l, d))` over 2,000 copies of `d` 11.1 to 0.54 ms,
    `count(union(l, l))` 21.7 to 0.60 ms; scans over lists that share nothing below their items
    unchanged (`count(distinct values(for i in 1..2000 return [i, i]))` 26.5 and 25.9 ms,
    `index of` and `in` over 2,000 one-item lists 0.34 and 0.31 ms, `list contains` over 5,000
    numbers 0.56 ms on both). No API change.
157. **`instance of` is decided on the value, without building its type.** `instance of` built the
    type of a list, a context, a range or a function (`Value::type_of`) and related it to the
    type asked for. The type of a list is a list of the type of its first item when every other
    item's type conforms to it, so building it typed each item, and a value that holds the same
    list on many paths was typed once per path: `instance of list<Any>` over a list doubled 30
    times through `partial` took 21.8 s after item 154, and doubled 60 times it would not end.
    `Value::has_type`, `Value::has_type_of` and `Value::type_instance_of`
    (`feel/src/values/type_relations.rs`) answer what `type_of() == t`,
    `type_of() == other.type_of()` and `type_of().instance_of(t)` answer, reading the value only
    as far as the relation needs (a list against `list<Any>` is not read at all, a context only
    in the entries of the type asked for). Within one relation, whether the items of a list
    conform to its first is decided once for a list that nests more than two levels (a shallower
    list costs a pass over its items when it is reached again), and the answer for a pair of
    nodes is remembered when either is a list that shares its items, keyed by the addresses of
    the borrowed nodes; the memory lives for the relation, is created on first use and is never
    iterated. `instance of` uses them for lists, contexts, ranges and functions and for a right
    operand that is not a type; Table 62 is not changed (the answers are those of the built
    types). Tests first: `feel/src/tests/type_relations.rs` compares the three relations with
    the built types for 300 random sets of values of every kind that share their lists (scalars,
    lists, contexts, ranges, functions, types, interval ends, unary tests) against the types of
    those values and random types, and relates lists doubled 60 times, one of them with a
    different last item, to their types at once; in `feel-evaluator/src/tests/shared_values.rs`,
    `instance of` over a list doubled 60 times answers at once, while on the parent that test was
    still running when it was stopped after 180 s. Identity step: no answer changes. The request
    `instance of list<Any>` of `release/registered/shared-lists.json` takes 7.6 ms (21.8 s
    before). Measured (wasm, Node 26, Apple M2, 2 rounds) against the module of item 156:
    `instance of` over lists that share nothing is slower, `l instance of list<list<number>>` for
    100 lists of two numbers 3.38 to 6.44 ms per 1,000 (1.9x the time; 11.0 ms when the items of
    every list were decided once through the memory, which is now kept for deeper lists only),
    with items of two kinds 3.78 to 5.89 ms; `instance of` over a context 4.83 to 4.79 ms per
    10,000, a range 2.22 to 2.28 ms, a number 1.24 ms on both. Rust API (`rheon-feel`): the added
    `Value::has_type`, `Value::has_type_of` and `Value::type_instance_of`.
    Measured on the module of items 154 to 157 together (wasm, Node 26, Apple M2, two
    alternating rounds): against the module of item 153, geomean 1.012x and 1.016x over the 39
    workloads of the speed suite, none below 0.97x in either round; `count(items)` over 10,000
    contexts 1.194x and 1.188x, over 10,000 numbers 1.062x and 1.070x,
    `{f: function(l) count(l), r: f(items)}.r` 1.434x; against 0.5.0, geomean 1.009x and 1.006x
    over the suite and 0.998x and 0.992x over the 86 parse workloads; below 0.97x in both rounds
    `count(for i in 1..10000 return abs(i))` (0.952x, 0.965x) and the 100 by 100 nested loop
    (0.965x, 0.947x), against item 153's module at least 0.990x and 0.977x, and of the parse set a
    list literal nested 297 levels (0.924x; 0.955x of item 153's module, one allocation more per
    list) and a flat context literal of 5,000 entries (0.964x, 0.970x; 1.00x of item 153's
    module). Allocations per call (native): one more per non-empty list built (the box that holds
    its items), fewer where a list was copied (`count(items)` over 10,000 contexts 210,105 to
    160,105, `{f: function(l) count(l), r: f(items)}.r` 260,207 to 160,206); the native peak of
    `count(items[status = "open"])` 22,459,853 B on 0.5.0, 22,459,925 B here (one box more than
    item 153's module); `count(items)` over 10,000 contexts 22,447,220 to 12,191,876 B. The string
    doubling's peak memory (n = 17 to 26, 267.38 MiB at 26) and the stack report are those of
    0.5.0. The walkers of a value built natively reach at least 1,265 levels on every host, as
    on 0.5.0 (equality at least 2,140 levels, 3,750 before; dropping a list at least 4,609, 6,656
    before); `context put` refuses a key list beyond 500 levels by design. `run.sh --release`:
    7,886 requests, 34 registered differences, 0 others, 529 s, peak RSS 3.1 GB.
158. **The parameters and the result type of a function definition are read through accessors.**
    `FunctionDefinition` exposed its formal parameters and its return type as public fields, so any
    holder of a function definition could change what an invocation binds and coerces to. They are
    now private and read through `parameters()` (the names with their types, in order) and
    `result_type()`; the invocation, `sort`, the callback check of `list replace`, the business
    knowledge model's inputs, `instance of` and the display read them there. `closure` and
    `closure_ctx` stay public: a function returned by a body is closed over by writing them.
    Identity step: no answer changes. Rust API (`rheon-feel`): the fields
    `FunctionDefinition::parameters` and `FunctionDefinition::result_type` are private; read them
    with the added `FunctionDefinition::parameters()` (`&[(Name, FeelType)]`) and
    `FunctionDefinition::result_type()` (`&FeelType`); build a definition with
    `FunctionDefinition::new` as before.
159. **Context values are shared.** A context value held its context by value, so every copy of
    it (a name read, an argument passed, an entry taken out, `partial` in a loop) copied all its
    entries and theirs: `data.entry0` over a context of 10,000 entries copied the 10,000 entries
    (11,546 allocations and 1,644,148 B more than over one entry), and a context that a loop
    doubles 30 times through `partial` (`{a: partial[-1], b: partial[-1]}`) held 2^29 copies of
    its first context and exhausted the module's memory. `Value::Context` now holds an
    `Arc<FeelContext>`: a copy shares the context, and a context is never changed while it is
    shared (`context put` "Returns a new context that includes the new entry", DMN 1.6 Table 82).
    A change takes the context out of its value (`FeelContext::try_from` moves it out of a value
    that holds it alone and copies it otherwise) or copies it through `Arc::make_mut`; a copy is
    shallow, its entries share what they hold. The changes along a path (`create_entries`,
    `create_entry`, `apply_entries`, `move_entry`, and the merge of a decision's imports) copy
    only the shared contexts on the path; the merge takes a nested context out of its target
    first, so a context the target holds alone is merged in place (a unit test in
    `model-evaluator/src/decision.rs` keeps its address and leaves a copy held elsewhere
    unchanged). `context put` copies the entries of its context only, and the
    path step, `get entries`, `context merge` of one context and the binding of a context to a
    name are reference counts. The walks of items 155 to 157 treat a shared context as they treat
    a shared list: equality remembers the answer for a pair of contexts of one size where either
    is shared, keyed by the addresses of the contexts, and the type relations of `instance of`
    remember a pair with a shared context on either side; a pair is reached again only through
    two paths that meet in a list or a context held twice, which is shared, so each pair is
    decided once. A filter of a list of contexts still gives each item's entries a frame of
    their own, now a shallow copy of the item. A value is quoted in a message
    through its `Arc` as before. Tests first: `feel/src/tests/contexts_shared.rs` applies every
    method that changes a context (`set_entry`, `insert`, `assign_entry`, `remove_entry`,
    `set_null`, `zip`, `overwrite`, `move_entry`, `create_entry`, `create_entries`,
    `apply_entries`) to a copy at nesting 1, 2 and 3 and checks that the original equals a
    snapshot, that the copy changed, that the copy owns the contexts on the path and shares the
    contexts off it; that a path of 1 to 3 names copies the contexts on the path only; and that a
    context value copied into 8 threads and changed in each leaves the original unchanged; on the
    parent the original stays unchanged but a copy shares nothing. In
    `feel-evaluator/src/tests/shared_values.rs`, `=`, `distinct values`, `index of`,
    `list contains`, `union`, `in` and `instance of` over contexts doubled 60 times answer at once;
    with this commit's contexts but without the context arms of the walks, each of the three tests
    was still running when it was stopped after 60 s. In `feel-evaluator/tests/allocations.rs`,
    `data.entry0`, `get value(data, "entry0")`, `{f: function(c) c.entry0, r: f(data)}.r` and
    `{d: data, r: d.entry0}.r` over a context of 10,000 entries cost at most 8 allocations and
    under 4 KiB more than over one entry. Answers change: 4 requests of
    `tools/identity/release/requests/memory-exhausting.json`, which exhausted the memory of 0.5.0
    (the three over a context doubled 30 times after 1.4 to 1.5 s), now answer
    (`tools/identity/release/registered/shared-contexts.json`): `=` of the doubled context with
    itself (true, in under 1 ms), the invocation of a function whose parameter declares
    `context<a: …>` nested 40 levels deep with a context doubled 40 times (0: a context type
    names the entries it follows, so the check follows one path), and the parse of `x.a.a` and of
    `{r: x.a}` with the doubled context as the FEEL input context (their syntax trees), which take
    52 s and 37 s on this module, because the parser's index of the input context visits a context
    again on every path that reaches it. Rendering the doubled context and its descendants `...a`,
    whose results are exponential in size, still exhaust the memory. No other committed request
    and no official TCK test changes its answer. Rust API (`rheon-feel`): `Value::Context` holds
    `Arc<FeelContext>` instead of `FeelContext` (build one with `ctx.into()` or `Value::from(ctx)`;
    match `Value::Context(ctx)` and read through the `Arc`; take the context out with
    `FeelContext::try_from(value)` or `Arc::unwrap_or_clone`); `rheon_feel::message::MessageArg`
    is implemented for `Arc<T>` where `T: MessageArg`. `Value` stays `Send + Sync`.
160. **A scope frame is an owned context or a shared one.** The stack of a `FeelScope` held its
    contexts by value, so a context value pushed as a frame was copied first: a filter over a
    list of contexts copied the entries of each item to put them in scope. A frame is now a
    `ScopeFrame`: a context the scope owns (the parameters of a call, the entries of a context
    literal, the variables of a loop) or a shared context value pushed without a copy
    (`FeelScope::push_shared`). A frame reads as its context (`Deref<Target = FeelContext>`); a
    write into a shared frame (`set_value`, `set_name`) copies the context first, so it never
    shows through the value it was pushed from. The one write into a frame that another evaluator
    pushed is that of a boxed context, which writes each entry into the frame on the top of the
    scope; on a shared item frame it now writes into a copy that is dropped with the frame, as it
    wrote into the item's copy before. A filter pushes a context item shared in both of its modes;
    in the mode for FEEL predicates, a context item with an entry `item` is kept as it was pushed,
    and only the depth of the stack is checked after its predicate (no FEEL predicate replaces a
    frame, and a boxed filter uses the other mode). The parser reads the frames through `Deref`.
    Tests: `feel/src/tests/scope.rs` pushes a shared context, writes into its frame, and checks
    that the shared context is unchanged and that popping copies out only a context that is still
    shared; the pins of item 143 (`model-evaluator/src/tests/various/dmn_0008`) pass unchanged.
    Identity step: no answer changes. Rust API (`rheon-feel`): `ScopeFrame` is added (`Deref` to
    its context, `into_context`, `From<FeelContext>`, `Send + Sync`); `FeelScope::push_shared` is
    added; `FeelScope::frames` returns `Ref<[ScopeFrame]>` instead of `Ref<[FeelContext]>`;
    `FeelScope::pop_frame` returns `Option<ScopeFrame>` instead of `Option<FeelContext>`;
    `pop`, `peek` and `contexts` return their contexts as before, a shared one copied.
161. **A function definition shares its closure context.** A call put the closure context of
    the function in scope as a copy, so every call copied the values the function closes over:
    1,000 more calls of a function that closes over 50 names of a context literal cost 58,001
    allocations more, against 2,001 for a function that closes over one name. The closure
    context is now shared (`FunctionDefinition::closure_ctx: Arc<FeelContext>`): a call pushes it
    as a shared frame (item 160), a copy of the definition shares it, and closing over the names
    a function returned by a body uses (`closed_over`) writes into a copy of the definition and
    of its closure context when they are shared. `FunctionDefinition::new` takes the closure
    context as before and holds it alone. Test first: in `feel-evaluator/tests/allocations.rs`,
    1,000 more calls of a function that closes over 50 names cost at most 8 allocations more
    than over one name (on the parent 58,001 against 2,001). Identity step: no answer changes.
    Rust API (`rheon-feel`): `FunctionDefinition::closure_ctx` is an `Arc<FeelContext>` instead
    of a `FeelContext` (change it through `Arc::make_mut`).
162. **The parser's index of an input context reads a shared context once.** The index of the
    names of an input context (`feel-parser/src/index.rs`, `Keys::new`) followed every path, so
    after item 159 an input context that holds the same context on many paths was indexed once per
    path: the parse of `x.a.a` and of `{r: x.a}` with `x` a context doubled 30 times through
    `partial` took 52 s and 37 s. A context held on several paths (its reference count is above
    one) is now read again only when it is reached with a longer composite-name prefix than
    before: its keys are the same on every path, so with a prefix no longer it adds no key and no
    longer composite name, and neither do the contexts below it, whose prefixes are shorter then
    too; the longest prefix of each shared context is kept by its address for the one index, and
    the memory is created when the first shared context is met. A context held on one path is
    read as often as the value that holds it, as before. The composite-name search already
    visited each state (node, offset) once, and a shared context has one address on all its
    paths, so it needs no change; the comments that said nodes are never shared now say why a
    shared node is exact. Tests first: in `feel-parser/src/tests/names_index.rs`, the parse of
    `x.a.leaf + 1` with `x` a context doubled 30 and 60 times reads at most depth + 2 contexts
    (counted by the work counter `index contexts`), while on the parent the test was still
    running when it was stopped after 60 s; composite names into a context held under three keys
    equal the flattened-key search (a pin, as on the parent); and the random comparison of the
    name lookup with the flattened-key search now also builds contexts held under two keys.
    Identity step: no answer changes, since the answers of the two parse requests were
    registered with item 159. The requests of
    `tools/identity/release/registered/shared-contexts.json` now take 8 ms, 1.6 ms and 0.4 ms on
    this module, and the ledger file 4.8 s (94.1 s before, the reference's traps included). Whether
    a list or a context is shared, and the address a walk remembers it by, are one rule in
    rheon-feel, read by this index, by the type relations (item 157) and by FEEL equality (item
    155), which each wrote it out before. Rust API: `Value::is_shared` and `Value::node_address`
    are added.
163. **The parser reads the names of a list's items from its items.** The names nested in a list of
    an input context are those of the list's item type when that is a context type, and the parser
    built the type of the list (`Value::type_of`) to find them, which typed every item and followed
    every path of a shared value: with `l` a list of two contexts doubled 30 times, the parse of
    `l.a.leaf + 1` exhausted the memory (the test process was killed). The item type of a list is
    the type of its first item when the types of the other items conform to it, so the names are now
    read from the first item: a context (its keys and the names nested in its values, by the rule of
    every other context of an input context), or a context type held as a value, a context type
    entry, a formal parameter or an interval end of one; the conformance of the items is decided by
    the type relations of item 157 without building a type (`FeelList::items_conform_to_first`, the
    decision `instance of` makes for a list), a pair with a shared list or context once. The index
    keeps the node of each list for the composite-name search, as it kept the list's item type. A
    context item may be held by its list alone, and such a list may be held on several paths (in
    `{a: l, b: l}` the item of `l` is held once, by `l`), so the index remembers the longest prefix
    of a shared list as it does for a shared context (item 162), and admits the item by the list:
    without that, a list of one context around two copies of the list before it, doubled 30 times,
    was indexed once per path (the test of it was still running when it was stopped after 60 s); it
    now reads at most depth + 2 contexts at 30 and 60 levels. The conversion of an input context
    into a parsing context reads a list the same way. One case differs from the built type and
    follows the rule of the other contexts instead: an entry of a list's first item that holds an
    internal value of a type (a context type, a context type entry, a formal parameter or an
    interval end, which no request and no DMN input can produce) carries no names, as the same entry
    outside a list never did. Tests first: in `feel-parser/src/tests/names_index.rs`, the parse of
    `l.a.leaf + 1` with `l` a list of two contexts doubled 30 and 60 times reads at most depth + 2
    contexts, while on the parent the test process was killed after it exhausted the memory; and the
    names of lists of contexts, of type values and of contexts that nest contexts, types and lists
    equal the item types that `Value::type_of` builds. The frozen oracle tests of the parser, which
    type the lists as before, and the parse twins of every committed request answer as before. The
    type of a list itself (`Value::type_of`, and the type a message quotes) is decided the same way:
    whether the items after the first conform to it is decided by
    `FeelList::items_conform_to_first`, and only the first item's type is built. It built the first
    item's type and then built or checked the type of every other item, so the type of a list that
    holds the same list on many paths followed every path (2^40 type builds for a list doubled 40
    times): the test of it, `feel/src/tests/message.rs`, was stopped after 60 s on the parent, and
    now the type of a list doubled 60 times is written at once. A message quotes `list<Any>` where a
    later item's type, larger than the 1,000 type nodes a message may build, made it quote
    `list<…>`, as for `1 + [[1], [{e0: 0, …, e999: 0}]]`
    (`tools/identity/registered/list-type.json`). Answers change: the message of that addition
    quotes `list<Any>`, and the addition `1 + l` with `l` a list doubled 40 times, which exhausted
    the memory of 0.5.0, is null with the type of the list in its error
    (`tools/identity/release/registered/shared-lists.json`). Rust API (`rheon-feel`): the added
    `FeelList::items_conform_to_first`. Measured on the module of items 158 to 163 together (wasm,
    Node 26, Apple M2, two alternating rounds): against the module of item 157, geomean 1.080x and
    1.070x over the 39 workloads of the speed suite and 1.057x and 1.065x over the 86 parse
    workloads, none below 0.97x in either round; `sum(items[amount > 100].amount)` 1.301x and
    1.301x, `count(items[status = "open"])` 1.323x and 1.336x, `count(for x in items return
    x.amount)` 1.225x, `{b: context put(data, "p", 2), r: [count(get entries(data)), b.p]}.r`
    1.371x, `count(for i in 1..count(items) return items[i].amount)` over 1,000 contexts 4.315x and
    4.260x; of the parse set the path `a.a.a…` of 100 names 5.271x, of 100 distinct names 5.020x, of
    25 names 1.778x, a context literal nested 149 levels 2.768x. Against 0.5.0: geomean 1.082x and
    1.084x over the suite and 1.056x and 1.055x over the parse set; path of 100 names 5.256x and
    5.222x, of 100 distinct names 4.972x and 4.969x, the context literal nested 149 levels 2.752x
    and 2.740x, the two filters 1.282x to 1.299x, the input of 10,000 contexts parsed for `1 + 2`
    0.974x and 0.971x, the inputs of 1,000 and 5,000 entries at least 0.970x (`fctx-temporal-200`
    0.976x and 0.970x, `fctx-5000` 0.982x and 0.974x), `fib15` 0.982x and 0.971x; below 0.97x in
    both rounds only a list literal nested 297 levels (0.925x and 0.924x) and a flat context literal
    of 5,000 entries (0.968x and 0.968x), as after item 157. Allocations per call (native): one more
    per context value built and per function definition (`count(items)` over 10,000 contexts 160,105
    to 170,105, `fib15` 6,190 to 6,192, `ctx200-0f` 16,929 to 16,930), fewer where a context was
    copied (the two filters 260,134 to 170,134 and 260,144 to 170,144, `count(for x in items return
    x.amount)` 260,154 to 170,143, `count(for i in 1..count(items) return items[i].amount)`
    10,025,160 to 21,160, the path of 100 names 10,789 to 988, the context literal nested 149 levels
    30,037 to 8,134); reallocations equal for every workload. The native peak of `count(items[status
    = "open"])` is 12,798,943 B (22,459,853 B on 0.5.0, 22,459,925 B after item 157). The string
    doubling's peak memory (n = 17 to 26, 267.38 MiB at 26) and the stack report are those of 0.5.0.
    The walkers of a value built natively reach at least 1,265 levels on every host, as on 0.5.0
    (dropping a context at least 3,156, 8,562 before; equality of lists at least 2,062, 2,140
    before); `context put` refuses a key list beyond 500 levels by design. `run.sh --release`: 7,890
    requests, 37 registered differences, 0 others, 482 s, peak RSS 3.9 GB.
164. **Request sets for filters, logic operators, callbacks, B-FEEL arithmetic and context
    literals.** The paths of filters by index and by item, of `and` and `or`, of the functions
    that `sort` and `list replace` call, of the B-FEEL arithmetic and of context literals had few
    requests in the committed sources, so a change to them could alter an answer that no identity
    run compares. Five sources of `tools/identity/requests/` now hold them, with their answers
    taken from 0.5.0 as for every source: `filters.json` (149 requests: filters over item entries
    and items, lenient and strict; the index branch with `0`, `-0`, indexes in and out of range
    from both ends, non-integers, indexes beyond 2⁶⁴ and values that are no number, on an empty
    and a non-empty list; context items with and without an entry `item`, nested filters, filters
    of a value that is no list, in loops and over `partial`), `logic.json` (118: an operand with an
    error, a warning or no boolean on either side, lenient, strict and B-FEEL, and recursions
    guarded by `and` or `or`), `callbacks.json` (51: callbacks that are function definitions,
    named functions, built-ins or no function, of the wrong arity, with errors or no boolean,
    closing over names, and recursing through the callback 66 and 80 levels deep),
    `bfeel-arithmetic.json` (2,235: `+`, `-`, `*`, `/` and `**` on every pair of twelve operands of
    every kind, B-FEEL lenient and strict and FEEL, and the numeric edges) and
    `context-literals.json` (47: repeated keys, entries that read earlier and later entries,
    nested literals, function entries, 200 entries). No request of them traps 0.5.0 or takes it
    more than 9 ms. The filter `[{g: f, v: 5}][if g instance of number then 1 else g(1)]` is a
    syntax error on 0.5.0, as is every `if c instance of number then …`; the set holds it beside
    the same filter with the condition in parentheses, which recurses until the call limit stops
    it. Identity against 0.5.0 with the module of item 163: 12,916 requests (7,826 before), 0
    differences beyond the registered ones; 4.5 s uncached, 2.8 s cached; with the large sets
    13,109 requests (the nine known depth probes differ, as before); with `--release` 12,980
    requests, 37 registered differences, 0 others, 528 s, peak RSS 4.9 GB. Tooling only; no API
    change.
165. **One iteration path for FEEL and boxed expressions.** A `for`, `some` or `every` expression
    had two ways to put its iteration context in scope: a FEEL expression moved the context into the
    scope and back in every iteration, because a FEEL body never writes into the context on the top
    of the scope, and a boxed one copied it in every iteration, because a boxed context as its body
    does. The iteration context is now one shared context for the whole walk (`Arc<FeelContext>`),
    pushed as a shared frame (item 160) in every iteration and for every domain, and changed between
    iterations only, when the walk holds it alone, so without a copy. What a body writes into the
    frame goes into a copy that is dropped with the frame, as the copy of the boxed path was;
    `partial` is moved into the context before the body and read back from the frame the body ran
    in, so an entry `partial` that a boxed body writes replaces the results as before (the pins of
    item 143 pass unchanged). A frame that is still the iteration context (its address is that of
    the shared context) was not written, and the results are then moved out of the context without a
    further lookup: reading them from the frame first cost loops 5 to 9 % (`count(for i in 1..10000
    return i)` 0.917x of the module of item 163). The walk allocates the shared context once per
    evaluation of the expression. `Iteration::for_feel` and the mode it chose are removed; FEEL
    expressions build their iteration with a crate-private constructor, and `Iteration::boxed` is
    unchanged. Test first: in `feel-evaluator/src/tests/for_expression.rs`, a body that reads an
    entry and writes it into the frame sees no value in every iteration, with one variable and with
    two, and a body that writes `partial` replaces the results so far; on the parent the FEEL
    iteration answered `[null, 7]`, the write reaching the next iteration. Identity step: no answer
    changes. No public API change.
166. **One filter path for FEEL and boxed expressions.** A filter had two ways to put an item in
    scope: for a FEEL predicate it moved each item into the scope and back, reusing one special
    context `{item: value}` for all items, and for the match of a boxed filter it put a copy of
    each item into a new special context. Both took the items out of the list first
    (`into_vec`), which copied every item of a list that another holder shares, such as a list
    read by name. The filter now reads the items in place and clones the items it keeps (a list
    or a context item is shared, not copied); a context item is pushed shared, as before, and the
    special context is one shared context for all items, pushed shared and changed between items
    only. What a predicate writes into the frame on the top of the scope therefore goes into a
    copy dropped with the frame, as the boxed path's copy was, and the pins of item 143 pass
    unchanged. The lost-context checks of the FEEL path now hold for every filter: a predicate
    that pops the item contexts, or replaces the special context with one without an entry
    `item`, gives the internal-error null (no FEEL predicate and no boxed match does either).
    The index branch reads the item at the index from the list in place. Tests first: in
    `feel-evaluator/src/tests/filter.rs`, a predicate that reads an entry and writes it into the
    frame does not see it for the next item, for numbers, context items and context items with
    an entry `item` (on the parent the FEEL filter kept `[1]` of `[1, 2]`, the write reaching the
    second item); in `feel-evaluator/tests/allocations.rs`, `count(items[item = null]) +
    count(items)` over 10,000 numbers or contexts costs at most 8 allocations and under 4 KiB
    more than over one item (on the parent 649,939 bytes more, the copy of the items). The test
    that compared the two modes now compares the filter over a shared list and over a list built
    anew. Identity step: no answer changes. Rust API (`rheon-feel-evaluator`):
    `FilterExpressionEvaluator::for_feel` is removed; `FilterExpressionEvaluator` is a unit
    struct that only `new` and `Default` build.
167. **Shared values, the nesting limit and their measured effect.** `docs/CONTRACT.md` states
    what sharing means for a consumer: lists and contexts are shared when they are bound, passed,
    read from the input, iterated, filtered or put into another value; the operations that
    compare or test values visit each shared list or context once, so they take time linear in
    the distinct parts of a value (equality at most quadratic); and writing such a value out
    follows every path, so a value doubled 30 times still ends at the memory cap when it is
    displayed, rendered by `string()`, flattened or spread. `docs/BACKLOG.md` loses "values are
    cloned on lookup", which items 154 to 166 removed, and gains the derived Rust `==` that the
    ANY and PRIORITY hit policies of native decision tables use (a tree walk), the list built-ins
    that copy the items of their argument (`count(append(for i in 1..10000 return i, 0))`
    allocates 641,162 bytes more than the loop alone, a copy of the 10,000 items), and why loops
    and filters push their contexts shared (the writes of boxed contexts). Docs only; the API
    breaks of items 147 to 166 are listed in their items. Measured on the module of items 164 to
    166 (wasm, Node 26, Apple M2, two alternating rounds; a first run against the module of item
    163 had a build running beside its first round and was measured again): against the module of
    item 163, geomean 1.001x and 1.009x over the 39 workloads of the speed suite, none below 0.97x
    in both rounds; `count(for i in 1..count(items) return items[i].amount)` over 1,000 contexts
    1.237x and 1.234x (the index filter no longer copies the list), the two filters of item 163
    1.015x to 1.026x, the loops 0.982x to 1.012x except one round each of `count(for i in
    1..10000 return i > 0 and i < 20000)` (0.910x; 1.007x in the other round, 0.977x and 1.136x
    in the first run) and of the B-FEEL `count(for i in 1..10000 return "n" + i)` (0.950x;
    1.014x); the parse set 1.020x and 1.001x over 86 workloads. Against 0.5.0, the value
    rework as a whole (items 147 to 166): geomean 1.086x and 1.086x over the suite and 1.054x and
    1.063x over the parse set; `count(for i in 1..count(items) return items[i].amount)` over
    1,000 contexts 5.456x and 5.521x; `sum(items[amount > 100].amount)` 1.317x and
    `count(items[status = "open"])` 1.313x; `f(items)` passing 10,000 items 1.409x;
    `count(for x in items return x.amount)` 1.229x; `count(items)` over 10,000 contexts 1.167x
    and 1.175x; the path `a.a.a…` of 100 names 5.237x and 5.282x, of 100 distinct names 4.948x and
    4.972x, a context literal nested 149 levels 2.732x and 2.742x; the input of 10,000 contexts
    parsed for `1 + 2` 0.979x and 0.967x (the owner's floor is 0.88x); below 0.97x in both rounds
    only a list literal nested 297 levels (0.923x and 0.928x, one box per list since item 154);
    `fib15` 0.973x and 0.967x, `count(for i in 1..10000 return abs(i))` 0.964x and 0.977x.
    Allocations per call (native) against the module of item 163: one more per loop evaluation
    (the shared iteration context: `count(for i in 1..10000 return i)` 131 to 132, the nested
    loop of 100 by 100 1,066 to 1,167), the index loop over 1,000 contexts 21,160 to 20,161 and
    68,123,055 to 3,171,095 bytes, the two filters equal in count with 10 and 11 more
    reallocations (the kept items grow one by one instead of in the list taken apart); every
    other workload equal in allocations and reallocations. The native peak of
    `count(items[status = "open"])` is 12,671,845 B (22,459,853 B on 0.5.0). The string
    doubling's peak memory (n = 17 to 26, 267.38 MiB at 26, growth 2.00x the result) equals that
    of 0.5.0. Module size 3,761,500 B (3,737,388 B on 0.5.0, 3,763,638 B after item 163).
168. **The conformance of a value to a type visits a shared list or context once.**
    `Value::is_conformant`, the conformance of a value to a type (the check of a typed parameter, of
    the type a boxed context, a decision or a decision service declares, and of the implicit
    conversions), is a relation of the value, not of its type, and walked the value along every
    path: a list that a loop doubles 40 times through `partial`, passed to a function whose
    parameter declares its 40-level type, made 2^40 checks, and so did a list that holds the same
    context twice at each of 40 levels (`[c, c]` with `c = {a: l}` around the list before it). A
    context type names the entries it follows, so a context doubled 40 times and checked against
    `context<a: …>` nested 40 levels was checked along one path. It is now walked by `Conformance`
    in `feel/src/values/type_relations.rs`, which remembers the answer for a shared list or context
    and a type node by their addresses, for one check (a node held once is reached again only
    through a shared node above it); the answers are those of the walk before. Tests first:
    `feel/src/tests/type_relations.rs` checks lists doubled 60 times and lists that hold the same
    context twice at each of 60 levels against their types; each was stopped after 60 s on the
    parent, the second also with the answers remembered for shared lists only. Answers change: the
    invocations over a list doubled 40 times and over a list that holds the same context twice at
    each of 40 levels, which exhausted the memory of 0.5.0, answer 0
    (`tools/identity/release/registered/shared-lists.json` and `shared-contexts.json`). No API
    change.
169. **`and` and `or` evaluate the right operand only when the left one does not decide.** Both
    operators evaluated both operands and then applied DMN 1.6 Table 51, so the right operand ran
    even where its value cannot change the result: its errors and warnings were recorded
    (`false and (1 + "a")` was false with one error, and failed in strict mode), and a recursion
    guarded by `or` or `and` (`{f: function(n) n <= 0 or f(n - 1), r: f(3)}.r`) called itself on
    until the 200-call limit stopped the evaluation. The right operand is now evaluated only when
    the left one is not `false` (`and`) or not `true` (`or`); the values of Table 51 are
    unchanged for every pair of operands, and the left operand is always evaluated
    (`(1 + "a") and false` is false with its error, `true and (1 + "a")` null with it). The B-FEEL
    dialect shares the operators. Tests first: `feel-evaluator/src/tests/conjunction.rs` and
    `disjunction.rs` (the nine rows of Table 51 for each operator; no error and no warning
    recorded for a right operand that cannot change the result, the errors of the left operand
    kept, the guarded recursions end), and `wasm/feel-wasm/tests/contract.rs` (the same requests
    in lenient and strict mode and in B-FEEL), failing on the parent where the right operand's
    error is counted. Answers change (`tools/identity/registered/logic-right-operand.json`,
    `docs/CONTRACT.md` section 3): 16 requests of the committed sets change their result or
    their error (strict mode passes where the right operand's error stopped it; the guarded
    recursions answer true or false instead of the call-depth error), and 22 lose only the
    errors or the warnings of the right operand, explained by two rule entries. The official TCK
    is unchanged (its `and` and `or` tests, 0064 and 0065, compare values). No API change.
170. **The item at an index of a filter is one function.** The index branch of a filter
    (`feel-evaluator/src/filters.rs`) wrote the bounds check twice, once for a positive index
    and once for a negative one. It is now `item_at`: the integer check, the empty list, the
    magnitude beyond 2⁶⁴ and the bounds are checked once, in the order and with the texts of
    before. Identity step: the tests of `feel-evaluator/src/tests/filter.rs` and the index
    requests of `tools/identity/requests/filters.json` (0, a non-integer, beyond 2⁶⁴, on an empty
    and a non-empty list) answer as before. No API change.
171. **A filter expression is evaluated once in the enclosing scope, and a number selects.** A
    filter `l[e]` over a list evaluated `e` for every item with the item in scope, then once more in
    the enclosing scope, and only then looked whether that gave a number. So an index was evaluated
    once per item (`count(for i in 1..count(items) return items[i].amount)` was quadratic in the
    list), every error of both passes was recorded (`sum(items[amount > 100].amount)` had two errors
    for the name `amount` in the enclosing scope and failed in strict mode), and a type error of the
    per-item pass hid an index that the enclosing scope defines. DMN 1.6 Table 69 maps row 1 ("e1 is
    a list and e2 is an integer") in the enclosing scope and the boolean rows in the scope with the
    item. The filter now evaluates `e` once in the enclosing scope first: a number selects the item
    at once (`item_at`, item 170) and its errors stay; anything else selects nothing, so what that
    evaluation recorded is removed and `e` is evaluated for each item as before. An abort in it is
    never removed: the evaluation stops. The removal is `rheon_feel::diagnostics::tentative`, which
    returns a mark of what the running collection held (`Tentative`; dropping it keeps the records,
    `Tentative::discard` sets the count back and cuts the messages and the warnings to their marked
    lengths, exact at the cap and after a repeat of the message before the mark, and nothing when no
    collection runs). The evaluation in the enclosing scope runs in an empty frame of its own, so
    what a boxed context as the match of a boxed filter writes into the frame on the top
    (model-evaluator) is dropped with it: on the parent that evaluation came after the items and
    wrote into the enclosing frame, where a later entry of an enclosing boxed context read it (a new
    pin of `model-evaluator/src/tests/various/dmn_0008`: `seen` of the match was 1 there and is now
    null with "context has no value for key 'seen'"; the other pins of that model, an entry reading
    a later entry among them, answer as before). Tests first: `feel/src/diagnostics.rs` (a discarded
    mark, at the cap, after a repeat, with a nested collection, outside a collection; a kept mark;
    an abort survives), and `feel-evaluator/src/tests/filter.rs` (an index evaluator that counts its
    runs runs once for `list[2]` over 1,000 items, 1,001 times on the parent; the requests below),
    and `wasm/feel-wasm/tests/contract.rs` (strict mode), failing on the parent; the pins of frames
    that a caller's evaluator removes or writes are kept, with the evaluation in the enclosing scope
    added. Answers change (`tools/identity/registered/filter-probe.json`, `docs/CONTRACT.md` section
    3, the backlog entry on the outer evaluation removed): 14 requests of the committed sets change
    their result or their error (strict mode passes on filters over item entries and on `[][1 +
    "a"]`; `{a: 2, r: [{a: "x"}, {a: "y"}][a]}.r` is `{a: "y"}` instead of the null of a string
    index; an index the enclosing scope defines selects where the items' scope ran a runaway
    recursion, `5` instead of the call-depth error; and one change for the worse, where the
    expression is a type error for the items and a runaway recursion in the enclosing scope, `{f:
    function(n) f(n + 1), r: [{g: "s"}][if g = "s" then "x" else f(1)]}.r`, is now the call-depth
    error instead of null), and 59 lose only errors or warnings of the enclosing-scope evaluation,
    explained by one rule entry (`[1, 2, 3][1 + "a"]` has 3 errors instead of 4, `[10, 20,
    30][count([1 + "a"])]` 1 instead of 4; the error counts of five rows of
    `feel-evaluator/src/tests/values_used_again.rs` fall the same way). The official TCK has no
    filter whose expression is a name or a path, and its boxed filter tests 1151 004 and 005 keep
    their null. Rust API (`rheon-feel`): the added `diagnostics::tentative` and
    `diagnostics::Tentative`. Measured (wasm, Node 26, Apple M2, the median of warm calls on one
    instance): `count(for i in 1..count(items) return items[i].amount)` over 1,000 contexts takes
    0.938 ms and over 10,000 contexts 8.65 ms, 9.2 times as long, where the module of item 170 took
    57.3 ms and 5,591 ms (one cold call each, as the module of item 167: 56.7 ms and 5,548 ms; warm,
    56.7 ms and 5,561 ms, 98 times as long), so the index loop is linear in the list.
172. **`sort` and `list replace` call their function in one scope.** Each call of the ordering
    function of `sort` and of the matching function of `list replace` was given a new empty
    scope, whose stack of frames was allocated again for every call. One empty scope now serves
    every call of one `sort` or `list replace`: a call pushes the function's closure and its
    parameters and truncates the stack back to its depth when it returns (`apply_body`), so each
    call starts from the empty scope as before. It stays an empty scope, not the caller's, so
    what the function's free names find is unchanged. Identity step: the requests of
    `tools/identity/requests/callbacks.json` and the aliasing requests of `values.json` answer as
    before. No API change. Measured (wasm, Node 26, Apple M2, three alternating rounds against
    the module of item 171): `count(sort(for i in 1..10000 return modulo(i * 7919, 10007),
    function(a, b) a < b))` 32.07 to 27.72 ms (1.157x), `count(list replace(for i in 1..10000
    return i, function(item, newItem) item > 5000, 0))` 3.43 to 3.08 ms (1.115x).
173. **The function called by `sort` and `list replace` is a counted call.** The ordering function
    of `sort` and the matching function of `list replace` were invoked without the call guard, so
    they did not count toward the 200 nested function calls: a recursion through a callback was 2
    calls deep per level (the function and the built-in) and ran 99 levels before the limit, on the
    native stack of every callback frame besides. Both are now invoked through `call`
    (`feel-evaluator/src/invocation.rs`), the guard around `invoke` that `evaluate_invocation` uses
    too, so a recursion through a callback is 3 calls deep per level: levels 1 to 66 answer as
    before, 67 to 99 are now the call-depth error, and 100 and more stop in both. The invocation
    nodes keep entering the guard themselves, before they evaluate the function and the arguments.
    After an abort a callback is not reached at all, because the call of the built-in is refused
    first, and the loops of both built-ins already stopped at the abort. Test first:
    `feel-evaluator/tests/evaluators.rs`, a recursion through each callback 66 levels deep answers
    and 80 levels deep stops at the call-depth limit, while on the parent 80 levels answered.
    Answers change (`tools/identity/registered/callback-call-depth.json`, `docs/CONTRACT.md` section
    2, the backlog entries on the comparator and on call depth counting): the two requests of
    `callbacks.json` 80 levels deep are now the call-depth error instead of `true`. The official
    TCK's `sort` and `list replace` tests (0013-sort, 1155-list-replace-function) use shallow
    function literals and business knowledge models. No API change. Measured (wasm, Node 26, Apple
    M2, against the module of item 172, three runs of three or four alternating rounds): the
    10,000-item `sort` of item 172 0.934x, 0.945x and 0.936x (27.5 to 29.4 ms), its `list replace`
    0.960x, 0.957x and 0.956x, under the floor of 0.97x that applied to this step. The cost is the
    guard on each call of the ordering function, one per comparison: the count of nested calls is
    entered and left, the abort is checked, and the call's result is checked for a non-finite number
    and for its nesting depth. The commit is kept as a robustness fix, the figure reported. Against
    0.5.0 at the end of items 168 to 173 (two alternating rounds) the `sort` stands at 1.099x and
    1.075x and the `list replace` at 1.055x and 1.049x: the shared scope of item 172 gains more than
    the guard costs.
174. **Arithmetic operators are named kernels behind one entry.** Each of the five binary
    arithmetic operators and the negation was a closure in its builder, wrapped by `operate` (the
    null-operand rule) and, for all but `**`, by a second evaluator closure (`guard_finite`).
    B-FEEL could not call them on two values, so for every B-FEEL arithmetic node it built a
    second FEEL evaluator of `⟦lhs⟧ op ⟦rhs⟧` (`bfeel::feel_operator_evaluator`) and ran it on a
    context of two entries pushed for each evaluation, once more for each converted pair of
    operands. The operators are now named functions of two values (`add`, `subtract`,
    `multiply`, `divide`, `exponentiate`, and `negate` of one), free of scopes and evaluators,
    and `Arithmetic::apply` is the one entry: the null-operand rule, the kernel, and the finite
    check for the operators that had it (`**` reports a non-finite power itself). The FEEL
    nodes and B-FEEL call it on the operand values; `guard_finite`, `bfeel::Operator`, the operand
    names and nodes, `feel_operator_evaluator` and `bfeel::apply` are gone, and the frame of an
    operator's evaluator still holds only its two operands (`apply` stays out of line). Identity
    step: the same kernels on the same operand values record the same errors in the same order,
    and the B-FEEL attempt still records its FEEL error first; the requests of
    `tools/identity/requests/bfeel-arithmetic.json` (2,235: every operator on each pair of
    operand kinds, nulls and mismatches, lenient and strict) and the tests of
    `feel-evaluator/src/tests/bfeel.rs` answer as before. The stack report (`npm run stack --
    --report`, a body of nested additions in 190 nested calls) reaches about twice as deep, as
    each level of the body runs one evaluator closure fewer: the Node worker with 4 MiB 104 to 209
    levels (Liftoff) and 114 to 210 (TurboFan), Node's main thread 22 to 45 and 24 to 46,
    Wasmtime with 4 MiB 120 to 220 and with 512 KiB 10 to 19 (`docs/CONTRACT.md` section 1 and
    the comment of `scripts/stack-headroom.mjs` state the new figures; theirs were older than
    0.5.0). No API change. Measured (wasm, Node 26, Apple M2, two alternating rounds) on the
    module of this commit against that of item 167: `sum(for i in 1..10000 return i * 2 + 1)`
    in B-FEEL 2.001x and 2.006x, `count(for i in 1..10000 return "n" + i)` 2.002x and 2.115x.
    Items 168 to 173 together, against the module of item 167: geomean 1.220x and 1.233x over
    the 39 workloads of the speed suite and 1.009x and 1.013x over the 86 parse workloads, none
    below 0.97x in both rounds; `count(for i in 1..10000 return i < 0 and (i * i + 1) / 3 > 5)`
    7.291x and 7.413x, the index loop of item 171 over 1,000 contexts 64.9x and 63.5x, the
    `sort` of item 172 1.096x and 1.114x, its `list replace` 1.058x and 1.070x, `fib15` 1.071x
    and 1.054x. Against 0.5.0: geomean 1.336x and 1.331x over the suite, none below 0.97x in
    both rounds (`loop-count-1k` 1.011x and 0.988x, `bif-loop-10k` 0.974x and 0.990x, `fib15`
    1.036x and 1.032x, `loop-nested-100x100` 0.984x and 0.986x), and 1.067x and 1.070x over the
    parse set, below 0.97x in both rounds only the list literal nested 297 levels (0.931x and
    0.924x, item 154). The two filters of item 163 answer with fewer errors (item 171) at 1.308x
    to 1.335x. Allocations per call (native) against the module of item 167: fewer for 20 of
    the 39 suite workloads and 49 of the 86 parse workloads, more for none, the others equal; the
    `and` loop above 39,139 to 174, the `sort` 625,358 allocations and 121,052 reallocations to
    504,336 and 31, the `list replace` 50,212 and 10,035 to 40,213 and 36, the two B-FEEL loops
    60,165 to 153 and 120,154 to 60,148, the index loop 20,161 to 17,161. The string doubling to
    n = 26 grows the memory exactly as with the module of item 167 (267.38 MiB at 26, twice the
    string). The depth margin, the deepest nesting each walker of a value reaches before the
    stack of the host runs out (five hosts: Node's main thread and a 4 MiB worker, each with
    Liftoff and TurboFan, and Wasmtime with 4 MiB), is unchanged for every walker of a list or a
    context nested in one line, at least 1,265 levels on every host; on a list that holds the
    same list twice at every level, or that list beside a list of one item, `instance of
    list<boolean>` and the type of the value (as a response states it) reach 1,452 levels, the
    check of a typed parameter 2,218 and equality 2,062, all above the limit of 500.
175. **A ledger outcome of the identity check can be the digest of a long response.** A ledger
    entry held its old and new answers as result texts or whole responses, so an answer that
    renders a value nested 500 levels deep (several megabytes for a list grown through `partial`)
    could be registered only by changing the request. `old` and `new` may now also be
    `{"sha256": …}`, the sha256 of the response's canonical text without the fields that identify
    the build (the text the cache of reference answers stores); the response is compared whole,
    as in the response form. `--record` writes a response longer than 65,536 characters in this
    form (`tools/identity/README.md`). Test: `npm run test:identity`, a recorded 501-level list
    whose old answer is a digest reproduces, and another digest does not; a malformed digest is
    an input error. Tooling only; no API change.
176. **The depth probes of the large sets that 0.5.0 answers are committed requests.** Nine depth
    probes 501 levels deep in the request sets kept outside the repository answer the value-depth
    error of item 152, so the large-set run listed them as unregistered differences. The seven
    that 0.5.0 answers are now in `tools/identity/requests/value-depth.json`, as they are written
    there: a list and a context grown through `partial`, a recursion around list literals that
    nests 501 levels, chains of 501 context literals, of ranges and of closures, and `context
    put` with 501 keys. Their entries in `tools/identity/registered/value-depth.json` hold the old
    answer whole for the range and the closure chains and as a digest (item 175) for the five
    whose rendering is longer than 65,536 characters, and the value-depth error as the new one;
    the basis is that of item 152, the owner's limit of 500 levels. Identity against 0.5.0: the
    committed sources are 12,932 requests, 63 registered differences reproduced, 0 others.
    Tooling only; no API change.
177. **The large-set run leaves out two chains by name.** The two other depth probes of the large
    sets, a context literal of 100,000 entries that each hold the entry before (2 MB) and one of
    50,000 closures that each return the closure before (1.3 MB), stop with the value-depth error
    where 0.5.0 exhausts its memory and traps; they are too large to commit with a ledger entry,
    so the large-set run listed them as unregistered differences. `run.sh --large` now leaves
    them out by name: `tools/identity/large-left-out.json` names each with the sha256 of its
    canonical text and the reason, `equal.mjs --leave-out` drops the named requests from every
    source and counts them, and a name that no source holds is an input error, so the list
    cannot go stale unseen. Their 501-level forms are registered (item 176). Test: `npm run
    test:identity` (a named request is left out and counted, a stale or malformed name is an
    input error, and `run.sh` passes the list only with `--large`). The large-set run against
    0.5.0: 13,109 requests, 2 left out by name, 63 registered differences reproduced, 0 others,
    34 s uncached (peak RSS 2.5 GB). Tooling only; no API change.
Commits 179-221 replace the evaluator of expressions. The identity check moves to 0.6.0 and the
tools of the replacement come first: the value kernels as named functions, request sets and pins for
scoping, functions, order, types, cells and names, a seam where the evaluator is chosen, a harness
that compares two evaluators on every request, random expressions and twins of the model tests
(179-186). The flat evaluator is then built beside the evaluator of closures, one kind of expression
at a time: name resolution, the program and its run loop, names and paths, operators, logic,
invocations, types, context literals, walks, filters, function definitions and calls on the heap,
with the comparison showing no difference at each step (187-209). Intervals of integers are counted
as integers, the root expression reads a free name of the host once, the scopes kept for functions
of the host are bounded, and tests compare what a function of the host sees (210-213). After two
test and ledger commits (214, 215) the answers of the evaluator of closures are recorded, the flat
evaluator becomes the only one and the evaluator of closures is deleted (216-218); two small commits
follow (219, 220), and an evaluation can have a budget of steps (221). No answer of a parsed
expression changes against 0.6.0 but one registered request (215); a tree built by hand of a shape
that the parser never produces gets one defined error (217). Boxed expressions of DMN models (boxed
for, some, every, filter, context) are composed by the DMN layer from evaluators as before; they
share the sequence, item and outcome kernels with the run loop and are not lowered to programs. The
totals are under "Verification" below.

Commits 222-226 are five changes of the lexer, the parser's name index and the input conversion.
Two change answers, each registered with its basis: a name that is not in scope ends before a
keyword that can follow an operand, so `for s in x return 1` and `x and y` have their keywords when
`x` is no name in scope (222), and a string literal or a block comment that the input does not close
is a syntax error instead of the end of the input (223). Three change no answer: the name tokens
after an iteration variable `item` no longer search their chain again and again (224), a JSON input
context of more than 11 entries is built in one pass (225), and the name index compares the items
of a list only when a lookup depends on it (226).

Commits 227-232 are the boundary of the wasm module and two inputs of the parser that cost out of
proportion. No answer changes. The JSON of `context` and `input` is read straight into values by a
strict reader, with the tree of JSON values as the answer to everything the reader refuses (227);
the response of an evaluation is written straight from the result, byte for byte as before (228);
the records of an input list share their names and the order of their keys (229); and the digits
of a number are read as a number (230). In the parser, a name is looked up among the pushed
contexts in one step (231), and a candidate name is a piece of the flattened text of its chain,
looked up only when a name in scope has its length (232).

179. **The identity check compares with 0.6.0.** The baseline moves to the v0.6.0 release module,
    and the ledgers of registered differences against 0.5.0 are emptied: 0.6.0 answers every
    committed request as registered. The 61 entries and 3 rule entries of `registered/` and the 21
    entries of `release/registered/` stay in git history; their requests stay in the request
    sources. The release-tier requests that 0.6.0 answers in well under a second move out of the
    release tier: `values-deep.json` (12, at most 4 ms each) to `requests/`, the 17 of
    `memory-exhausting.json` that 0.6.0 answers (at most 9 ms each) to `requests/doubled-values.json`,
    and the 4 of `value-depth-deep.json` (at most 17 ms each) only stay in
    `wasm/feel-wasm/tests/corpus/no-trap.json`, which already holds them. The 5 that exhaust the
    memory of 0.6.0 stay in `release/requests/memory-exhausting.json`. The two chains that
    `large-left-out.json` named answer the value-depth error on both sides now, so the list is
    empty and the large-set run compares them. Identity of the 0.6.0 build against the 0.6.0
    module: the committed sources are 12,997 requests, with `--release` 13,007 (5 trapping the
    reference), with the large sets 13,176 (2 trapping the reference); 0 differences in each, and
    0 cache mismatches with `--verify-cache`. Tooling only; no API change.
180. **The value kernels of the evaluator are named functions.** The closures that the evaluator
    builder made for an expression computed its value from the evaluated operands inside their own
    bodies, so no other evaluator could reach that computation without copying it. It is now in
    named functions in `feel-evaluator/src/kernels/`, which read no scope and evaluate nothing:
    the arithmetic (moved whole, with `finite`), the comparisons and `between`, `in` against every
    right-hand form, `instance of`, the types built from evaluated parts (list, range, context and
    function types, formal parameters), paths, properties and descendants, the value of a name or
    a qualified name that no scope holds, the number and `@` literals, list and range literals,
    the entries of context literals, the value of a function definition and of an external
    mapping. The builder's closures evaluate their parts in the same order and call them. The
    message of an invalid operand of addition names the module of its call site's file, so the
    macro gains the form `invalid_argument_type!(module "builders", …)`, which addition now uses:
    the texts stay `[builders::add] …`. The binding of arguments converts them in place
    (`coerce_arguments`), and the closure of a returned function takes the search of its names as
    a function. `bind` and `closed_over` are not inlined into the application of a body, so their
    frames are not on the stack while the body runs: a nested call takes no more stack than in
    0.6.0 (`npm run stack -- --report`: every host reaches the depth of 0.6.0 or one more). Test first: a kernel test that `add` of a date and a boolean still says
    `[builders::add]`; the evaluator's tests and the TCK copies unchanged. Identity step: no answer
    changes. Rust API: none (crate-private).
181. **A tentative mark is taken without a closure, and the deep search of a context is a loop.**
    `diagnostics::tentative(f)` ran `f` and returned its result with the mark of what the running
    collection held before it, so only code that evaluates through a closure could take one.
    `diagnostics::tentative_mark()` takes the mark alone; the evaluation that follows it is the
    tentative one, and the filter's evaluation in the enclosing scope takes it so. The deep search
    of a context (`FeelContext::search_deep`, under `search_entry` and the scope's qualified
    search) called itself once per name; it now takes one step per name in a loop, so the search
    itself is not bounded by the host stack. The other walks of a qualified name still call
    themselves once per name: `FeelContext::create_entries` (under `create_entry`, which writes
    the names a function closes over into its closure context) and `contains_deep`. Tests: the tests of the mark
    take it alone, as before around the same records; the deep search stops at a value that is no
    context and follows a context nested 500 levels deep. Identity step: no answer changes. Rust
    API: `rheon_feel::diagnostics::tentative` is removed; `rheon_feel::diagnostics::tentative_mark`
    is added.
182. **Request sets and native pins for scoping, functions, order, types, cells and names.** Six
    request sets in `tools/identity/requests/` cover what an evaluator must reproduce besides the
    values: `scoping.json` (145 requests), `functions.json` (177), `order.json` (165),
    `types.json` (116), `cells.json` (161) and `names.json` (105), described in
    `tools/identity/README.md`. Among them: the hidden `partial` of a `for` stays out of sight in
    later iteration contexts, also when a variable is named `partial` and when a called function
    reads it (`for partial in [1, 2], y in [partial] return y` is `[null, null]` with two errors);
    nested calls 198 and 201 deep through `abs(abs(f(n - 1)))`, within the AST depth of a request;
    a callback that reads an outer name through a function entry, so that its scope is empty. None
    traps 0.6.0, and each answers in at most 7 ms on it. `feel-evaluator/src/tests/scoping.rs`
    pins 33 of them natively, evaluated as a request evaluates them, with their values, messages
    and counts. Identity against 0.6.0: 0 differences by construction. Tests only; no API change.
183. **The evaluator of an expression is chosen in one place, and a harness compares two.**
    `prepare`, `prepare_with_dialect` and `evaluate` build through one private function, which
    asks the flat evaluator (`feel-evaluator/src/flat.rs`) to lower the expression and builds the
    closures of the whole expression when it refuses; it refuses every expression yet, so every
    expression is built as before. The cargo feature `backends` of rheon-feel-evaluator, enabled
    only by the dev-dependencies of rheon-feel-wasm, adds the hidden module `testing`:
    `with_backend` chooses on the thread, inside a closure, between the evaluator of closures, the
    flat evaluator with that fallback, the flat evaluator alone (a refused expression is an error
    null naming the refusal) and a mutant that negates boolean results; `take_refusals` reads the
    refusals. `wasm/feel-wasm/tests/backends.rs` sends every request of the identity request
    sets, of the ledgers, of `no-trap.json` and of the edge-operand product (the product moved
    into `tests/corpus/product.rs`, shared with the contract test) through the evaluator of
    closures and through the flat evaluator alone, and compares the responses whole; a refused
    request is counted by the kind of its node. `RHEON_PARSER_CORPUS` adds the FEEL corpus and
    `RHEON_FULL_TESTS` the release-tier requests, without those that exhaust the module's memory
    (natively no cap stops them; the identity check compares them in the module). The comparison
    is `#[ignore]`d, for the group tier. Test: `harness_detects_a_difference` (two equal
    evaluators give no difference; the mutant gives the three boolean answers of five). Since the
    flat evaluator refuses every expression yet, only requests that build no expression are
    compared. Identity step: no answer changes. Rust API: none
    without the feature; with it, `rheon_feel_evaluator::testing::{Backend, Refusal,
    with_backend, take_refusals}` (hidden, for tests).
184. **Random evaluable expressions for the backends harness, with shrinking.**
    `wasm/feel-wasm/tests/backends/gen.rs` builds a request from a seed (SplitMix64, the same on
    every host): an expression directed by the grammar and by the scope, so that most names refer
    to a binder that exists where they are written (iteration variables, `partial`, parameters,
    context entries, `item` and the entries of filtered items, names of an input context), over
    arithmetic, comparisons, logic, `if`, `between`, `in` with unary tests, `instance of`, `for`
    with one and two iteration contexts over ranges and lists, `partial`, `some` and `every`,
    filters by a predicate and by an index (0, negative, not an integer, not a number), context
    literals with repeated keys, forward references and entries named `item`, `partial` and
    `abs`, function definitions called positionally, by name, with the wrong arity and with typed
    parameters, functions whose bodies read a name that only their caller binds where it calls
    them (a loop variable, `partial`, the item of a filter or its entry, a later entry),
    recursion of 3 to 250 calls, `sort` and `list replace` callbacks, unary-test
    requests, B-FEEL, strict mode and a JSON input context. The harness sends 2,000 of them after
    the committed requests (50,000 with `RHEON_FULL_TESTS`, one with `RHEON_BACKENDS_SEED`); a
    request whose responses differ is shrunk, by replacing nodes with literals or with their
    children while the difference stays, and reported with its seed. Tests: the same seed gives
    the same request; at least 95% of 2,000 parse (measured 98%); 2,000 cover every construct;
    shrinking a request that the mutant answers differently keeps the difference and gives a
    shorter request. A difference that a request shows only after the requests before it fails
    the step too. Measured: the 2,000 random requests are all refused, since the flat evaluator
    lowers nothing yet. Tests only; no API change.
185. **The model tests can compare every evaluation with a twin.** With `RHEON_BACKEND_TWINS`
    set, `build_model_evaluator(s)` of the model-evaluator tests builds each model twice, by the
    flat evaluator (with its fallback) and by the evaluator of closures, and `assert_decision`,
    `assert_business_knowledge_model` and `assert_decision_service` evaluate both: the outcome
    texts and the diagnostics (count, messages and warnings) must be equal, besides the pinned
    text. With `RHEON_BACKEND_COVERAGE` naming a file, they add a line to it for each expression
    the flat evaluator lowered or refused (by the kind of the node) and for each evaluation, with
    or without a twin. A model
    is held weakly beside its twin, so the tests of a model's lifetime see it dropped. The feature
    `backends` of rheon-feel-evaluator is enabled from the dev-dependencies of
    rheon-model-evaluator; it now records refusals only inside `with_backend`, so other tests
    record nothing. Test: `a_model_has_a_twin`. Measured: the model-evaluator tests (3,722) in
    3.97 s alone and 4.35 s with the twins. Tests only; no API change.
186. **The tiers run the comparison of the two evaluators, and the stack report measures two more
    recursions.** The group tier runs the backends harness (`backends`: `cargo test -p
    rheon-feel-wasm --test backends -- --ignored`, with the FEEL corpus) and sets
    `RHEON_BACKEND_TWINS` for the model tests and the TCK copies; the release tier runs the harness
    at full size (`backends-full`, release build, `RHEON_FULL_TESTS`). The fix tier is unchanged.
    `npm run test:tier` checks the three steps. `npm run stack -- --report` also prints, on every
    host, the deepest body of a recursion of 200 nested calls (the call depth limit) and of a
    recursion of 66 levels through a `sort` callback (199 nested calls), the native stack of a
    host re-entry; the checks without `--report` are unchanged. Measured on the 0.6.0 module
    (macOS arm64, Node 26, Wasmtime 49.0.3), the deepest body of 200 calls: Node main thread 43
    (Liftoff and TurboFan), 4 MiB worker 199 and 200, Wasmtime 18 (512 KiB), 210 (4 MiB) and the
    AST limit (16 MiB); of 66 levels through `sort`: main thread 111 and 157, Wasmtime 55 (512 KiB),
    138 (1 MiB), and the AST limit (284) from 2 MiB and in the 3 and 4 MiB workers. Tooling only;
    no API change.
187. **Name resolution for the flat evaluator.** `feel-evaluator/src/flat/resolve.rs` (the module
    `flat` became a directory) decides from the parser's tree alone, with no scope and no input,
    what every name of an expression reads and where every binder is kept. The scopes of the
    evaluator of closures become static regions: a region is one scope at a position of the code
    (the first entries of a context literal's or an iteration's layout, the item of a filter, the
    closure of a function), and a name reads the slot of a binder of its own body, `partial` (the
    results so far of a `for`), the item of each filter around it first, or else a free name, read
    at run time from outside the body. A context entry is seen by the entries after it only and a
    literal stops at a repeated key; the first domain of an iteration is outside it and each later
    domain sees the variables before it, and in a `for` no later domain sees `partial`, a variable
    of that name included, as the walk removes it; parameters of one name share a slot; the types
    of parameters and an external mapping are read where the function is defined; a qualified name
    of one segment reads as a name (without a built-in), of several it is searched along the chain
    at run time. Only the shapes that the evaluator of closures binds bind names; other shapes are
    kept unbound. Indices are `u32` (no new limit). The free names, with where they are read (root,
    function body, filter predicate, callee), are a crate-private report. Nothing calls the pass
    yet. Tests: `flat::tests::names` (20 rules, on the names each expression reads and on the
    dump of the resolved tree). Identity step: no answer changes. Rust API: none.
188. **The flat evaluator runs literals.** The flat evaluator is now the evaluator of every
    expression it lowers: `prepare`, `prepare_with_dialect` and `evaluate` resolve the expression,
    lower it to a program (`flat/program.rs`: instructions of at most 16 bytes over numbered
    slots, with tables of constants and messages, checked for indices out of range in builds with
    debug assertions) and run it in one loop (`flat/vm.rs`), one run per evaluation, in the
    evaluation's session and diagnostics. A node that is not lowered refuses the whole expression,
    which is then built as closures, as before. Lowered: number, string, boolean, null and `@`
    literals; a literal whose evaluation records nothing is a constant, a truncated `@` literal
    records its warning and a literal without a value its error at every evaluation. Values are
    computed into temporaries that their one reader moves out. The backends harness also counts
    the compared requests in which an expression was lowered (`testing::take_lowered`). Tests:
    `flat::tests::machine` (programs built by hand: verification, the order of a run's errors and
    warnings), `flat::tests::constructs` (each literal against the evaluator of closures, value
    and diagnostics). Measured (harness, debug): 59,069 requests, 29,595 compared, 366 lowered,
    0 differences; random 4,000, 224 lowered, 0 differences. Identity step: no answer changes.
    Rust API: none without the feature `backends`; with it, `testing::take_lowered` (hidden).
189. **The flat evaluator runs names, qualified names, paths and descendants.** A free name reads
    the scope of the host (`FeelScope::get_value`), else the built-in function of that name, else
    the error null `context has no value for key`, built at every read; a qualified name is
    searched in the scope of the host (`FeelScope::search`), else the error null `no value for
    qualified name`; a path and a descendant apply the kernels `path` and `descendants` to their
    evaluated base, and one whose name node is no name is the builder's error at every evaluation
    without evaluating the base. Tests: `flat::tests::constructs` (names from the input and
    built-ins, shadowed built-ins, `?`; qualified names of 0 to 3 segments over five inputs;
    paths over contexts, lists, dates, durations and strings, descendants, malformed paths).
    Measured (harness, debug): 717 lowered of 29,946 compared, random 262, 0 differences.
    Identity step: no answer changes. Rust API: none.
190. **The flat evaluator runs arithmetic, comparisons and `between`, FEEL and B-FEEL.** `+ - *
    / **` apply `Arithmetic::apply` (B-FEEL: `bfeel::arithmetic`), `-e` `Arithmetic::negation`,
    `= != < <= > >=` the kernels `equal`, `not_equal` and `compare_*`, and `between` the kernel of
    that name on its three operands, all evaluated first, in order; in B-FEEL a null of a
    comparison or of `between` becomes `false` (`true` for `!=`), as `bfeel::boolean_result` makes
    it. Tests: `flat::tests::constructs` (every operator on every pair of 12 operands of every
    kind, in both dialects; negation; `between` on 80 triples; nested operators). Measured
    (harness, debug): 8,036 lowered of 37,265 compared, random 440, 0 differences. Identity step:
    no answer changes. Rust API: none.
191. **The flat evaluator runs logic, `if`, lists, ranges, `in` and unary tests.** `and` and `or`
    evaluate the right operand only when the left one does not decide (Table 51; a jump past the
    right operand's code), `if` takes the then branch only when the condition is `true`; list
    literals are built item by item (`bound_depth` on the list, as `list_of` does), comma lists
    and negated comma lists of unary tests likewise without the check; range literals and their
    ends, the unary tests `= != < <= > >=`, `-` and parenthesised expressions give the values the
    builder gives; `in` applies `in_operator` and `not in` (a hand-built `Out`) evaluates the whole
    `in`, then its left operand again; in B-FEEL `and`, `or` and `in` never give null. The
    program check now also rejects a jump that is not forward into the code and lists that are
    not opened and closed in nesting order on every path. Tests: `flat::tests::constructs`
    (`and`/`or` on 36 pairs and `if` on 6 conditions in both dialects, lists and ranges, `in`
    with 8 right-hand forms on 6 values, `not in`, unary tests on 5 inputs), `flat::tests::machine`
    (unstructured code). Measured (harness, debug): 9,509 lowered of 38,738 compared, random 744,
    0 differences. Identity step: no answer changes. Rust API: none.
192. **The flat evaluator runs invocations of built-in functions and of functions of the host.**
    Every invocation node is entered before its function and arguments are evaluated
    (`diagnostics::enter_call`, the call depth limit: a refused invocation evaluates nothing and is
    null) and left after it (`finite`, `bound_depth`, then the guard is dropped), as the builder's
    guard does; the function and the arguments, in the order written, go to `invoke` with the
    program's dialect, positionally or by name, so built-in functions, functions of the host's
    scope (DMN business knowledge models and decision services, functions of the input) and
    values that are no function give what they give today. A malformed invocation is the
    builder's error inside the invocation. A run that ends early leaves the invocations it
    entered innermost first. The program check rejects a leave without an enter, an open
    invocation at the return and an enter whose refusal does not continue past its leave. Tests:
    `flat::tests::constructs` (22 invocations of built-ins in both dialects, positional and named,
    wrong arity, non-functions; 199 to 250 nested invocations, with the invocations after the
    abort; functions of the input, a `sort` callback, typed parameters; malformed invocations),
    `flat::tests::machine` (enter and leave). Measured (harness, debug): 26,352 lowered of 55,581
    compared, random 900, 0 differences. Identity step: no answer changes. Rust API: none.
193. **The flat evaluator runs `instance of` and type expressions.** A type is evaluated at every
    evaluation, as the builder evaluates it: a named type of the grammar is a constant, a type name
    is a qualified-name search, `list<t>` and `range<t>` apply `list_type` and `range_type`, a
    context type adds its entries (`context_type_entry`, `add_context_type_entry`) in order and
    stops at the first that is no entry, without evaluating the rest, and a function type applies
    `function_type` to its parameter types and result type; `instance of` applies `instance_of`
    (not wrapped in B-FEEL). Every construct without binders is now lowered: what the flat
    evaluator still refuses are context literals, iterations, filters and function definitions.
    Tests: `flat::tests::constructs` (11 values against 19 types in both dialects, malformed
    types). Measured (harness, debug): 26,588 lowered of 55,817 compared, random 926, 0
    differences; refused `Context` 2,408, `FunctionDefinition` 311, `Filter` 297, `For` 213,
    `Every` 12 and 11 of kinds the report did not name then (it named five kinds). Identity
    step: no answer changes. Rust API: none.
194. **The flat evaluator builds and runs a small expression with fewer allocations.** Every
    expression is built once per request in the module, so the build is part of each request's
    time. The resolution pass sizes its node tables from the size of the tree; a few names are
    found by comparing them with each, more in a hash map (the standard one, whose hasher is keyed
    at random, so no input makes its lookups slow), and the tables of names and free names are
    allocated only when there are names; the root expression's proto is not in a vector; the
    program keeps its tables as they were built and takes the resolved names (only when the
    expression is lowered) instead of interning them again, and paths read their entry names
    from the resolution; a run whose frame has at most 4 slots keeps it on the host's stack.
    Measured (native, release, the build and one run of an evaluation, evaluator of closures /
    flat evaluator): `a` 126 / 352 ns (470 before this change), `1 + 2` 307 / 515 (595),
    `(a * b + c) / d` 858 / 1,180 (1,299), `abs(1)` 416 / 774 (845); one run alone: `a` 53 /
    74 ns (96), `1 + 2` 60 / 89 (110).
    Allocations per request (native, `feel_wasm::call`, 0.6.0 engine / this one): `tiny` 92 /
    95 (97 before), `names-10f` 248 / 233. Speed suite (V8, build and run per request, two
    rounds, against 0.6.0): `tiny` 0.95x and 0.94x, `names-10f` 0.96x and 0.96x (0.81x before),
    `decimal-arith` 0.95x and 0.96x, `ut-gt` 0.92x and 0.93x: the build of a small expression
    still costs more than the builder's. Identity step: no answer changes. Rust API: none.
195. **The backends harness sends each request once, and leaves out the requests that exhaust
    memory.** The parse twin of an evaluate request builds no evaluator from a JSON input, so it
    can not differ, and the identity check already sends it; the counts of the harness are the
    requests whose expressions both evaluators built. With `RHEON_FULL_TESTS` the requests of
    the release tier that exhaust the module's memory are left out: natively no memory cap stops
    them. Tooling only: no answer changes. Rust API: none.
196. **One helper runs a test body on a thread with a big stack.** The contract test and the
    backends harness share `tests/shared/big_stack.rs` instead of a copy each. Tooling only.
    Rust API: none.
197. **The twins compare whole diagnostics and note what was lowered.** The twins of the model
    tests compare the whole diagnostics of an evaluation, its warnings included. With
    `RHEON_BACKEND_COVERAGE` naming a file, the model tests note in it every expression that the
    flat evaluator lowered, every one it refused (by the kind of the node) and every evaluation,
    with a twin or without one, so a run with twins shows how much flat code it compared.
    Tooling only. Rust API: `rheon_feel_evaluator::testing::take_lowered` is new (doc-hidden,
    behind the `backends` feature).
198. **The backends harness compares responses without recursion.** It compares the texts of the
    two responses without the members that name the build or the time, read without recursion,
    so a response that holds a value nested hundreds of levels deep is compared too; parsed as
    JSON it would stop at the JSON parser's recursion limit. Tooling only. Rust API: none.
199. **The flat evaluator runs context literals.** A context literal's entries are binders: each
    has a slot of the frame, before the temporaries, and its value is evaluated into it and nested
    no deeper than values may nest (`BoundDepth`); the entries after it read it from the slot, by
    a copy, with no instruction of their own. `Context` builds the context of the entries in
    order, moving their values out of the slots. At a repeated key the value of its entry is
    evaluated, then the key's error (`duplicated context entry key: …`, the kernel's text, now one
    function) is the value, and the slots of the entries before it are cleared. A context with an
    entry that is no entry with a key (only a tree built by hand has one) is refused. The program
    keeps the layouts and the regions of name resolution, for two cold paths: a function of the
    host invoked inside a context literal (a DMN body, an external function, a function of a FEEL
    input context) gets the scope that the evaluator of closures has there, the context of the
    entries visible at the invocation pushed on the host's scope for the call (an empty one is left
    out: it changes no read); and a qualified name of several segments is searched in the entries
    visible at its position, a scope at a time and going on below where it fails, then in the
    host's scope. Tests: `flat::tests::constructs` (entries that read earlier, later and outer
    entries, shadowed built-ins, repeated keys, B-FEEL, a function of the host that reads the
    entries before its invocation, qualified names over nested contexts, an irregular entry
    refused). Measured (harness, debug): 28,138 of 29,540 requests compared, 28,001 lowered, 0
    differences; random 1,097 compared, 1,076 lowered, 0 differences; model tests with twins
    3,723 passed, 4,987 expressions lowered, 3,635 evaluations compared with a twin. Identity
    step: no answer changes. Rust API: none.
200. **The flat evaluator runs `for`, `some` and `every`.** An iteration is a walk, the record of
    its nesting depth: the domain of the first variable is evaluated before it opens
    (`LoopOpen`), and `LevelPush` adds a domain (an interval, a range literal of a `for`, or the
    items of a value, as `Sequence` makes them; a domain that fails ends the walk with its null).
    `Step` takes the walk's next step as the walk of the closures does: unless the evaluation
    was stopped, the next value of the innermost variable that has one, into the variable's
    slot, followed by the domain of the next variable or by the body; a variable whose domain
    has no value left is cleared. `Visit` takes the body's value: a `for` adds it to its results
    and bounds their depth, a `some` or an `every` ends at a value that decides, and at one that
    is no boolean with null. `LoopClose` gives the value and clears the variables. `partial` is
    the walk's results (`LoadPartial`); a variable named `partial` is hidden from the domains
    after it, as the walk of the closures removes that name before each of them, and loses to the
    results in the body. A variable named twice is the builder's error (its text, now one
    function), and so are the malformed `some` and `every` of a tree built by hand. A function of
    the host invoked inside an iteration gets the variables (and `partial`) that have values
    there; qualified names search them. The program check verifies that every walk is the record
    of its depth and that its steps lie inside it. Tests: `flat::tests::constructs` (intervals,
    ranges, dates, lists, single values and null as domains, failing domains, dependent domains,
    `partial` and its depth bound, variables named twice, `partial` hidden in later domains, the
    quantifiers' deciding values, functions of the host that read the variables and `partial`,
    qualified names, malformed iterations, FEEL and B-FEEL); `model-evaluator` `host_calls` (a
    business knowledge model whose body reads the variables, `partial` and earlier entries of its
    invocation). Measured (harness, debug): 28,345 of 29,540 requests compared, 28,208 lowered, 0
    differences; random 1,270 compared, 1,249 lowered, 0 differences; model tests with twins
    3,723 passed, 5,026 expressions lowered. Identity step: no answer changes. Rust API: none.
201. **The flat evaluator runs filters.** A filter's items are a record of its nesting depth.
    `FilterOpen` takes the value of the list: a null is the filter's value at once, without the
    predicate; a value that is no list is filtered as the list of it, and the value is then bounded
    a level deeper. The predicate is first evaluated in the scope around the filter, after a
    tentative mark of the diagnostics: `FilterStep` selects the item at a number's index
    (`item_at`, unless the evaluation was stopped), and on any other value discards what the
    predicate recorded and visits the items, evaluating the same code of the predicate for each:
    the evaluation stopped ends the filter with null before each item and before its value, an
    item is kept when the predicate is `true`, and a value that is no boolean, number or null
    ends the filter with its null (`outcome`). A name in a predicate reads, while the filter
    visits its items, first the item: `item` is the item, or its entry `item` when it is a
    context that has one (`ItemSelf`), and another name the item's entry of that name
    (`ProbeItem`), innermost filter first; then what the name reads around the filter. A function
    of the host invoked in a predicate gets the item's entries and `{item: …}` as the scope of the
    closures has them, and a qualified name searches them. Only function definitions are refused
    now. Tests: `flat::tests::constructs` (indices in and out of range, not integers, empty
    lists, booleans, nulls and other values as predicates, non-lists, context items with and
    without an entry `item`, nested filters, `partial[-1]`, predicates reading context entries,
    loop variables and the input, functions of the host that read the item, qualified names,
    FEEL and B-FEEL). Measured (harness, debug): 28,735 of 29,540 requests compared, 28,598
    lowered, 0 differences, refused only `FunctionDefinition` 805; random 1,527 compared, 1,506
    lowered, 0 differences; model tests with twins 3,724 passed, 5,086 expressions lowered,
    refused only 173 function definitions. Identity step: no answer changes. Rust API: none.
202. **Allocation caps for the walks and filters of the flat evaluator.** `feel-evaluator/tests/
    allocations.rs` measures what 10,000 iterations or items cost more than one: `for`, `some` and
    `every` over intervals and lists, `partial[-1]`, and filters over numbers and contexts (by
    `item`, by an entry and by `item.n`) allocate only for their results, 12 allocations of
    2,096,640 bytes as the vector of 10,000 values doubles (capped at those and a tenth); two
    variables cost one allocation more; reading `partial` costs no copy of the results (an
    iteration that passes it to a built-in allocates 2 allocations and 72 bytes, for the
    invocation). Tests only; no API change.
203. **A function of the host invoked inside binders gets one reused scope per region.** An
    invocation of a function of the host inside a context literal, an iteration or a filter pushes
    the scopes that the evaluator of closures has there; each was a context built for the call.
    Now each region keeps one context for the run, created at its first invocation: its names are
    the region's, its values are assigned for each invocation without copying the names, and set
    back to null after it, so no value (`partial`'s results above all, which would then be copied
    as they grow) is held between invocations. The regions of the invocation are kept in one
    vector of the run. Test: `allocations.rs` (10,000 invocations in a walk cost 3 allocations and
    about 1 KiB each, no copy of the results). Measured (native, release, a business knowledge
    model invoked in a `for` over 10,000 numbers, minimum of 30 rounds): 4.38 ms with a context
    built per call, 3.41 ms now (0.6.0 engine: 2.84 ms). Identity step: no answer changes. Rust
    API: none.
204. **The resolution reports where a free name is read.** For every free name the resolution
    keeps where it is read: in the root expression or in a body, in the predicate of a filter,
    as the function of an invocation, and for a qualified name the read of its first segment.
    The report is private to the crate; it is what a layer that resolves names before an
    evaluation would read. Identity step: no answer changes. Rust API: none.
205. **The sample of the backends harness has a request that the flat evaluator refuses.** The
    harness's own test counts one refusal by kind and node, and the test of the size of an
    instruction is gone (the program's verification covers the encoding). Tooling only. Rust
    API: none.
206. **The flat evaluator runs function definitions and their invocations, on the heap.** A
    function definition evaluates its formal parameters (a constant when every type is a named type
    of the grammar; otherwise each parameter's type in order, up to the first that fails, as the
    builder's fold), then the mapping of an external body (even when a parameter failed), then
    makes the function with the kernel of the builder (`function_definition`): its closure holds
    the value of each name it closes over, searched where it is defined, a scope at a time down to
    the host's. The body of FEEL is a proto of the program, lowered after the root expression (one
    body after the other, so no body is lowered inside another), and the function's body is
    `FunctionBody::Feel` with the program and the index of the proto (the program is the code of
    its bodies, so a definition allocates no code).
    An invocation of such a function is an activation of the body in the same run: the arguments
    are bound as `invoke` binds them (`bind`'s parts: the number of positional arguments, named
    arguments in the order of the parameters, then each converted to its parameter's type, the same
    texts; positional ones in their temporaries, named ones in one vector of the run), its slots
    are a window above those of its caller, which waits in a frame record on the heap, and its
    `Return` makes a function that the body gives close over the names it uses as they are there
    (`closed_over`) and converts the value to the result type. So FEEL recursion, nesting and
    loops take no stack of the host. A body reads its parameters and binders from slots and every
    other name as the evaluator of closures reads it in the scope of the call: its closure, then
    the scopes of its caller where the caller invoked it (its binders, the item of a filter, its
    closure), and so on down to the host's scope, then the built-in function of that name; the
    value of such a name is kept in a cell of the activation once read (the chain below an
    activation does not change while it runs), and a name that nothing has is null with its error
    at every read. Qualified names are searched along the same chain. A function of the host
    invoked in a body gets the scopes of the whole chain pushed on the host's scope; the function
    of another program (a decision that gives a function) is an activation too, which runs that
    program's code. The host's invocations of such a function (`sort` and `list replace`, DMN
    bodies, `evaluate_invocation`) run its body as a run of its own (`invoke`'s one new arm), with
    the closure and the parameters as `apply_body` has them. Tests: `flat::tests::constructs`
    (definitions, captures, a body that reads its caller's loop variable, `partial`, item and
    later entries, a returned function that closes over a later entry, typed and named
    parameters, external bodies, recursion to 250 calls, mutual recursion, callbacks of `sort`,
    searches in the callers' scopes), `feel::tests::function` (`Feel` compares and prints like
    `LiteralExpression`). Measured (harness, debug): every request of the committed sources is
    compared and nothing is refused, 29,540 requests, 29,403 lowered, 0 differences; random 2,000
    compared, 1,979 lowered, 0 differences; model tests with twins 3,724 passed, 5,259
    expressions lowered and none refused, 3,641 evaluations compared with a twin. Speed suite (V8,
    two rounds, against 0.6.0): `closure-call-10k` 1.07x and 1.08x, `typed-call-10k` 1.20x and
    1.20x, `named-call-10k` 1.09x and 1.11x, `fn-in-ctx-1k` 1.14x and 1.16x; `fib20` 0.78x and
    0.79x, `mutual-rec-150` 0.60x and 0.60x, `deep-rec-190x40` 0.68x and 0.69x, where a free name
    is found the first time in each activation by a walk through every caller. Identity step: no
    answer changes. Rust API: `rheon_feel::FeelCode` added (code that holds numbered bodies);
    `FunctionBody` gains the variant `Feel(Arc<dyn FeelCode>, u32)` (an exhaustive `match` on
    `FunctionBody` breaks).
207. **A body finds a free name its caller has read at the caller's closure.** The walk for a
    free name of a body goes through every activation below it, so recursion read its callee's
    name in time proportional to its depth. Now, where the walk reaches the closure of a waiting
    activation of the same program that has read the same name, the value in that activation's
    cell (what it found, or the built-in function of that name) is the rest of the walk: that
    activation read it from its closure down, and the chain below an activation does not change
    while it waits. The scopes above its closure (its binders and items where it invoked the
    body) are still searched first. Each proto keeps the names of its cells, by name. Tests:
    `flat::tests::constructs` (a caller that rebinds the name around the invocation, an item, a
    parameter of the same name, a built-in function, a missing name, recursion). Measured (speed
    suite, V8, two rounds, against the build before this change): `fib20` 1.24x and 1.24x,
    `fib15` 1.19x and 1.17x, `mutual-rec-150` 2.26x and 2.27x, `deep-rec-190x40` 1.48x and
    1.49x, the other call workloads 0.97x to 1.02x; against 0.6.0: `fib20` 0.98x and 0.98x,
    `mutual-rec-150` 1.34x and 1.35x, `deep-rec-190x40` 1.00x and 1.01x. Identity step: no
    answer changes. Rust API: none.
208. **Invocations of the flat evaluator allocate no arguments.** In FEEL a built-in function
    invoked with positional arguments reads them in their temporaries (the binder of the built-in
    takes a slice), which are then cleared: the vector of arguments is gone. Named arguments of a
    function of FEEL are put in the order of the parameters straight from their temporaries, and
    those of a function of the host and a built-in function into the one vector the invocation
    takes. Tests: `allocations.rs` (10,000 invocations of a function of FEEL, positional, named,
    typed or recursive, allocate nothing but the walk's results: 12 allocations and 2,096,640
    bytes more than one invocation; 10,000 of `abs` allocate 10,011 times, the value `abs` makes
    and the results). Identity step: no answer changes. Rust API: none.
209. **The stack check asserts FEEL recursion at the AST limit on every host.** FEEL calls run
    on the heap, so `scripts/stack-headroom.mjs` now asserts that a recursion of 200 calls (the
    call depth limit) whose body is nested to the AST limit evaluates on every host it runs:
    Node's main thread and a 4 MiB worker, with Liftoff and with TurboFan, and Wasmtime with its
    default 512 KiB wasm stack (newly checked) and with 4 MiB; and that a recursion of 66 levels
    through a `sort` comparison (199 nested calls) whose body is nested to the AST limit
    evaluates in the worker and in Wasmtime with 4 MiB, the hosts the contract requires for a
    callback, which takes native stack per level. The recursion of 190 calls and its floors of 60
    levels are gone. CONTRACT §1, the README and the runtime's README say that FEEL recursion
    takes no native stack, with the measured figures. Measured (`--report`, macOS arm64, Node 26,
    Wasmtime 49.0.3): every host, Node's main thread and Wasmtime with 512 KiB included, reaches
    the AST limit with both recursions (a body nested 289 levels in the recursion, 284 in the
    comparison); 0.6.0 reaches 43 levels with 200 calls on Node's main thread, 199 (Liftoff) and
    200 (TurboFan) in the 4 MiB worker, 18 and 210 in Wasmtime with 512 KiB and 4 MiB (so it
    fails the recursion of the new check on every host), and through `sort` 111 (Liftoff) on the
    main thread and 55 in Wasmtime with 512 KiB. No
    committed request traps 0.6.0 for want of stack, so the identity step shows no change.
    Rust API: none.
210. **An interval of integers is counted as integers.** The numbers of `a..b` and `[a..b]`
    whose ends are integers held without a scale are produced by counting an `i64` and
    converting each count, instead of adding one to a number and comparing it with the end at
    every step. Each count converts to the number that the addition gives, in the same cohort;
    an end with a scale or a fraction, a zero with a sign and an end outside the `i64` range
    step as numbers as before. Tests: `tests/iterations.rs` (both ways compared item by item, at
    the ends of the integer range as well). Measured (wasm, Node 26, against 0.6.0, two rounds,
    with the items up to this one): the counting loop over 10,000 numbers 1.01 ms to 366 us
    (2.75x), nested loops 2.6x to 3.0x. Identity step: no answer changes. Rust API:
    `FeelNumber::unscaled_i64` is new.
211. **The root expression reads a free name of the host once.** A free name of the root
    expression has a cell, as a free name of a body has: its first read looks the name up in the
    scope of the host, or finds the built-in function of that name, and later reads give that
    value. The scope of the host has the name or not for the whole run (a function of the host
    writes only into the scopes it pushes), so the answers are the same; a name that nothing has
    stays an error recorded at every read. Before, every read looked the name up by its text,
    and a built-in function was found from its name as a string each time. A conditional jump
    tests its operand where it is. Measured: natively, one input name read in a loop of 10,000
    iterations over a context of 1,000 entries 1,709 us to 988 us; on the speed suite a built-in
    function called in a loop 2.1x, short-circuit logic 1.9x against 0.6.0. Identity step: no
    answer changes. Rust API: none.
212. **The scopes kept for functions of the host are one per layout.** A function of the host
    that an expression invokes gets the binders of the invocation's position in a scope that the
    run keeps to use again. The run kept one such scope for each region, and every entry of a
    context literal is a region of its own, so a literal of n entries that each invoke a
    function of the host kept n scopes with n(n-1)/2 entries until the run ended. The run keeps
    one scope for each layout: a region that sees more entries than the one before extends it,
    one that sees fewer starts it again. Test: `allocations.rs` (400 entries allocated
    13,359,551 bytes where 200 allocated 3,428,607; now about twice as much). Identity step: no
    answer changes. Rust API: none.
213. **Tests of what a function of the host sees; a body of another program gets a scope of its
    own.** `flat/tests/host.rs` invokes a function whose body is the host's and gives what its
    scope holds for a set of names: from the entries of a context literal, from the domains and
    the body of a walk (with a variable named `partial`), from the predicate of a filter, from
    recursive calls and from a function of another expression (24 cases). The last case
    panicked: the scope kept for a layout was looked up by the layout's index before the layout
    was known to be one of the program that the run starts with. Such a body gets a scope of its
    own, which is not kept. No wasm request reaches the case (it needs a function of the host
    and a function of another expression). Rust API: none.
214. **The runaway evaluation that a runtime test cancels keeps little memory.** The test
    cancels a running evaluation after 200 ms. Its expression collected 100,000,000 numbers,
    which a walk over integers does fast enough to fill the 512 MiB of the instance before that,
    so the request ended as a crash. The expression is a walk of walks that keeps at most
    100,000 numbers at a time. Tooling only. Rust API: none.
215. **A recursion that overflowed the stack of the host is registered.** A function that wraps
    the value of each call in 250 lists, called 190 levels deep, overflowed the stack of Node's
    main thread in 0.6.0, and the instance trapped. Calls of FEEL functions do not use the stack
    of the host, so the evaluation reaches the limit of 500 levels of nesting and stops with the
    value-depth error. The request is in `tools/identity/release/requests/deep-recursion.json`
    with its entry in `release/registered/host-stack.json`: the one registered difference of
    this series against 0.6.0. Rust API: none.
216. **The answers of the evaluator of closures are recorded.** The tests of `flat/tests/`
    compared the two evaluators. The answers of the evaluator of closures for their 4,775 cases
    (value and diagnostics; 259 of them trees built by hand) are recorded in
    `flat/tests/expected/`, and the tests compare the flat evaluator with the record; a missing
    or unequal record fails. Tooling only. Rust API: none.
217. **The flat evaluator is the only evaluator of expressions.** The seam no longer falls back
    to the evaluator of closures. A tree of a shape that the parser never produces, which the
    lowering does not take, evaluates to null with one error at every evaluation, `malformed
    expression tree: unexpected <Kind> node`, where the kind is the name of the node that
    stopped the lowering and never its content. Every expression that the parser produces
    answers as before: the records of item 216 and the identity check are unchanged.
    **Behaviour change for trees built by hand only:** four crate tests of malformed trees
    changed their expected error (a context whose entry or key is not of its shape, a function
    definition without formal parameters, an empty comma list), from the error of the part to
    this one. Rust API: the behaviour above for `prepare`, `prepare_with_dialect` and `evaluate`
    on malformed trees.
218. **The evaluator of closures is deleted.** `builders.rs`, the `backends` feature with
    `rheon_feel_evaluator::testing`, the backends harness with its generator, the twins of the
    model tests and their tier steps are gone (2,349 lines). What the DMN layer uses for boxed
    expressions stays (`Iteration`, `Domain`, `Sequence`, the boxed filter, `invoke`,
    `closed_over`, `ClosureBuilder`): boxed `for`, `some`, `every` and filters compose
    evaluators of the host, and share the sequence, the item and the outcome kernels with the
    run loop. The error texts that name `builders` stay byte for byte. Checks: the group tier,
    with the identity check at 0 differences. Rust API: the doc-hidden module
    `rheon_feel_evaluator::testing` and the feature `backends` are removed.
219. **The end of a time range in a test is found by adding a duration.** The helper that checks
    that `now()` lies within a second of a time built the end of its range by adding to the
    seconds of the time, so it failed whenever it ran in the last second of a minute. Tooling
    only. Rust API: none.
220. **The recorded answers of the tests are not part of the package.**
    `feel-evaluator/Cargo.toml` excludes `src/flat/tests/expected/` (1.3 MB). Rust API: none.
221. **An evaluation can have a budget of steps.** An evaluate request can carry `maxSteps`, a
    positive integer: every function call (of a user-defined, built-in or model function) and
    every item that a `for`, `some`, `every` or a filter visits is a step, and the step after
    the last one of the budget stops the evaluation with the `evaluation` error `evaluation took
    more than <n> steps; evaluation stopped`; the instance stays healthy. Without the field an
    evaluation has no budget and answers as before; a value that is not a positive integer is a
    `request` error, read after `errorMode`. The budget is part of the evaluation state beside
    the call depth, so nested runs and callbacks share it. Tests: `contract.rs` (loops, filters,
    quantifiers, recursion, callbacks, built-in calls; the exact count of `count(for i in 1..10
    return i)`: 12 steps; the field's values and its place in the order of request errors),
    `engine.test.mjs` (through a worker, which is kept). A runtime test of the timeout walks
    100,000,000 items instead of 10,000,000. Additive to the wasm request and to
    `@rheon/feel-runtime` (`EvaluateRequest.maxSteps`). Rust API:
    `diagnostics::evaluation_within`, `diagnostics::step`, `Abort::Steps` and
    `Session::with_step_budget` are new.
222. **A name that is not in scope ends before a keyword.** When no name in scope starts at a
    name token, the name took every word up to the next punctuation, keywords included: `for s in
    x return 1`, `if x then y else z` and `some s in x satisfies s.z` were syntax errors when `x`
    was no name in scope, and `x and y` was one unknown name. Such a name now ends before the
    first of its words that is a keyword which can follow an operand (`and`, `between`, `else`,
    `in`, `instance`, `or`, `return`, `satisfies`, `then`), written as a word of its own: white
    space before it, and white space or the end of the input after it, as where the keyword
    starts a token. The rest is read as the following tokens, as after a name in scope (DMN 1.6
    §10.3.1.6 resolves a keyword inside a name by the names in scope, and here no name in scope
    has it). Unchanged: a name in scope is matched first, so it holds its keywords (`tax
    return`); the built-in names `date and time`, `years and months duration` and `days and time
    duration` are read whole; the other keywords never follow an operand, so `price list` stays
    one name; `opt-in rate` and `x and(y)` have no keyword as a word of its own. **Behaviour
    change beyond the fix:** a name that is not in scope and has one of the nine keywords as a
    word (`Is in range`) was null with an error and is another expression now, often a syntax
    error. The frozen reference lexer of the differential tests has the same rule, written in
    its own terms: as a registered difference it would leave the tokens after every such name
    uncompared. Tests: `expr::name` (the tree without the names in scope equals the tree with
    them; names in scope with keywords; the words that stay), `expr::various` (a filter of four
    conjunctions parses without its names in scope). Identity: the 10 requests of the committed
    sets whose answers change and those of 72 new requests of `names.json`, each with its parse
    twin, 104 entries in `registered/unknown-name-ends-before-keyword.json`; every changed
    answer of the 10 equals what 0.6.0 answers for the form with the name in parentheses. The
    official TCK is unchanged, test by test. `docs/BACKLOG.md` states what remains (a name that
    is not in scope still takes the symbols and other words after it). Rust API: none; the
    tokens and trees of `rheon-feel-parser` change as above.
223. **A string literal or a block comment that is not closed is a syntax error.** The lexer
    read the end of the input where the text ended inside a string literal or a block comment,
    and the parser accepted what came before: `1 + 2 "never closed` was 3 and `1 /* never
    closed` was 1. The token at the quote or at the `/*` is now an undefined token, so the input
    is a syntax error, as a string literal with a line break already was (DMN 1.6 §10.3.1.2:
    grammar rule 33 ends a string literal with its quote, and a block comment is `/* ... */`). A
    line comment still ends with the input. The differential tests register the difference with
    the reference: the same tokens, then the undefined token where the reference reads the end
    of the input. Tests: `expr::invalid` (21 texts under every start rule; closed forms).
    Identity: the new request set `syntax.json` (57 requests) and the 38 entries of
    `registered/unclosed-string-or-comment.json` (10 requests, lenient and strict, with their
    parse twins, and a FEEL-format input context). Rust API: none.
224. **The name tokens after an iteration variable `item` compare each part with `in` once.**
    The mode of the iteration variable (the name ends before a part `in`) stays set after a
    variable `item`, which is read before the mode is looked at (`docs/BACKLOG.md`,
    *Iteration-variable mode outlives `item`*). Every later name token then searched the rest of
    its chain for a part `in`: 1,000,000 comparisons for a chain of 1,000 names. The lexer
    remembers the last search of the chain, and a name token inside its range takes the answer.
    The tokens are the same, also where the mode shows. Tests:
    `parts_compared_with_in_grow_linearly_with_the_chain` (a work counter),
    `names_after_an_item_variable_equal_reference`. Measured (wasm, V8, best of 5, `for item in
    [1] return a + a + ...`): 1,000 terms 2.28 ms to 0.94 ms, 4,000 terms 30.4 ms to 1.64 ms,
    16,000 terms 463 ms to 6.2 ms, as with any other variable name. No answer changes. Rust API:
    none.
225. **A JSON input context of more than 11 entries is built in one pass.** Every entry of a
    JSON object was inserted into its context with a search of the map, though the entries come
    in the order of their keys, which is the order of the names unless a key was trimmed.
    `FeelContext` can be collected from entries: those in ascending order of their names,
    without a repeated name, build the map in one pass, and from the first entry out of that
    order on each is inserted, so an entry with the name of an earlier one replaces it as
    before. The input conversion collects an object of more than 11 entries this way; a smaller
    one, whose entries are one node of the map, is inserted as before, which is cheaper there.
    Of two keys that are one name after trimming, the later one in key order still gives the
    entry. Tests: `test_context_collected_from_entries` (equal to inserting, in every order),
    `context_keys_are_trimmed_but_inner_whitespace_is_kept` (both orders, nested contexts, a
    context of 44 keys). Measured (wasm, V8, whole requests): a context of 100 names 0.032 ms to
    0.028 ms, 1,000 names 0.310 ms to 0.247 ms, 5,000 names 1.69 ms to 1.31 ms, 100,000 names
    39.8 ms to 29.7 ms; rows of 24 keys 0.94x the time, of 64 keys 0.87x, of 128 keys 0.82x, of
    1 to 16 keys within 3% either way. The memory of the instance after a context of 100,000
    names is 33.3 MiB (30.8 MiB before: the entries are held once more while the map is built).
    No answer changes. Rust API: `impl FromIterator<(Name, Value)> for FeelContext` is new.
226. **The name index compares the items of a list only when a lookup depends on it.** The keys
    of the items of a list are names in scope when the items conform to the first one, and the
    index of an input context decided that for every list when it was built, which reads every
    item of every list; a parse that looked up one name of two parts paid for it. The index is
    now built without the comparison: the keys of the first item of a list of two or more items
    are held as not decided. It answers every lookup that these keys do not decide: a name it
    does not hold is no name in scope, a name held without such a list on the way is one, and
    its lengths are upper bounds of the lengths of the names in scope (a larger bound admits
    more candidate names, and each is looked up). A lookup of a name that only such lists hold
    builds the index with every list decided, which answers from then on; a composite name found
    through such a list is searched again there. The lexer does not look up the last candidate
    of a name token, its first part alone, when a keyword that ends a name follows it (item
    222): the token is this part whether it is a name in scope or not, so the list of a `for`
    and the operands of `and` decide no list (`for s in m.lines return s.qty`); a built-in
    property after a dot and a built-in type name where a type is read are still recognised.
    Tests: `lists_are_decided_when_a_lookup_needs_them` (a work counter),
    `names_of_list_items_equal_reference` (lists of items of one and of several types), and the
    random texts and contexts of the differential tests. Measured (wasm, V8, best of 60 whole
    requests): `sum(items.amount)` over 10,000 rows 8.85 ms to 7.76 ms; a `for` over the same
    rows 9.09 ms to 7.92 ms; a nested `for` over 200 lists of 50 rows 6.32 ms to 5.09 ms; a
    filter by a key of two words, which decides its list, 9.18 ms and 9.18 ms. No answer
    changes. Rust API: none.
227. **The JSON of context and input is read straight into values.** A request was read into a
    tree of JSON values and the tree converted into FEEL values: two walks, and the input held
    twice. A strict reader of the request text (`wasm/feel-wasm/src/json.rs`) walks the top-level
    object once and reads the JSON of `context` (an object) and of `input` (an object or an
    array) into values where it stands; the other fields are read as before. The reader answers
    only what it is sure of: it takes a text only when it is JSON as serde_json accepts it (the
    same grammar, escapes and nesting limit), and whatever it refuses is answered by the tree as
    before: a text that is no request, a top-level key written with an escape, an object with
    serde_json's private number key, a key with white space at an end, an empty key, a number
    that does not convert, and an object whose keys are not ascending when one of its values
    has a warning. So every error and its text are unchanged, of two entries with one key the
    later one counts, the warnings of an object come in the order of its keys, and a warning
    quotes a number as serde_json hands it on. Tests: random objects, a third of them damaged,
    read exactly as through the tree or refused (20,000; 400,000 at full size); random and
    damaged texts taken only when serde_json reads them as an object; the nesting limit; the
    scan of a string eight bytes at a time; a contract test that answers 37 requests both
    ways. Measured (wasm, V8, whole requests): `1` with 5,000 input names 1.31 ms to 0.61 ms,
    with 10,000 small contexts 6.87 ms to 5.47 ms, with 160 records of 200 fields in nested
    lists 7.63 ms to 4.16 ms; slower: a 1 MiB string without escapes 0.37 ms to 0.41 ms. The
    four changes that would let the tree go (exact keys, a repeated key as an error, warnings
    in the order of the text, own texts for malformed JSON) are not made. Rust API: none.
228. **The response of an evaluation is written straight from the result.** The result was
    walked four times, copied into a tree of its typed form and that into a tree of JSON
    values, and the response written from the tree: about 36 allocations for every number of a
    list. The response of a successful evaluation is now written as text
    (`wasm/feel-wasm/src/output.rs`), the typed form straight from the value and the `feel` and
    `type` texts through a writer that escapes what is written to it; byte for byte what
    serde_json wrote for the tree. Tests: the result of 30 expressions equals the text of its
    tree, which stays in the tests as the reference; strings are escaped as serde_json escapes
    them; a contract test reads 13 responses back and writes them again. Measured: a list of
    10,000 numbers returned 13.7 ms to 5.3 ms, 10,000 small contexts 79.5 ms to 37.3 ms, with
    the memory of the instance growing by 19.3 MiB instead of 84.4 MiB; `1 + 2` 5.2 us to
    3.8 us; slower: a 1 MiB string of which every fifth character is escaped, echoed, 14.5 ms
    to 17.0 ms. Rust API: none.
229. **The records of an input list share their names and the order of their keys.** The reader
    remembers, for each depth, the keys of the object it read last: their names, whether they
    are ascending, and the order that makes them so; the next object at that depth with the
    same keys in the same order takes the names and the order, for one comparison per key. A
    `Name` holds its text shared, so the name of a key is one text for all records. Tests: the
    differential tests of the reader, and lists of records whose key orders repeat, change,
    differ in number and repeat a key. Measured: `1` with 10,000 rows of 8 keys 9.9 ms to
    7.9 ms; a nested input of 3,400 records with 129,000 numbers 21.0 ms to 16.5 ms; small
    expressions 0.97x to 1.03x. Rust API: `Name` holds shared text (`From<Name> for String`
    copies); `FeelContext::from_ascending_entries` is new.
230. **The digits of a number text are read as a number.** The parser of a number text collected
    its digits in a string that the rounding read again. The coefficient is read as a number
    while it has at most 38 digits after its leading zeros and handed to the rounding that
    takes a number; a longer one is rounded from its digits as before. Test:
    `parse_equals_parse_by_text` compares the parser with the one it replaces on 55 chosen and
    50,000 random texts (2,000,000 at full size). Measured: `1` with a list of 40,000 numbers
    2.98 ms to 2.41 ms; the nested input of item 229 16.6 ms to 14.3 ms. Rust API: none.
231. **A name is looked up among the pushed contexts in one step.** Every binder pushes a
    context, and a name of two or more parts was looked up in each pushed context in turn: n
    nested binders with such names cost n² steps. The scope counts, for each key of the pushed
    contexts themselves, the contexts that have it; with more than 16 pushed contexts the
    bound of the candidates by their first word is the bound of all their keys. Tests:
    `pushed_contexts_walked_grow_linearly_with_the_nesting` (a work counter, and the tokens of
    binders nested 1 to 40 deep against the reference). Measured (op parse):
    `for v in a b return ` 2,000 times around `a b + v` 34.7 ms to 2.5 ms. The depth limit is
    still checked after the parse. Rust API: none.
232. **A candidate name is a piece of its chain, looked up only when a name has its length.** A
    name token tried every prefix of its chain of name parts and wrote the flattened text of
    each anew: with a long key in scope that shares its first word with the text, a text of n
    parts cost n³ byte steps. The flattened text of a regular chain is written once, a
    candidate is a piece of it, and a candidate of a length that no name in scope has is not
    written or looked up (`ParsingScope::may_contain_length`; the index of an input context
    keeps the lengths of its keys). Tests: the differential tests, in which every candidate
    piece is also compared with the flattening it replaces. Measured (the key `a.a...a.b` and
    `a` in the context, the text `a.a...a`): 250 parts 164 ms to 0.74 ms, 500 parts 1,279 ms
    to 2.5 ms, 1,000 parts (4 KB) 9.5 ms; the cost is still a step per candidate (2,000 parts
    37 ms). Rust API: none.
233. **`evaluateAll` evaluates several expressions over one context.** A host that evaluates
    several expressions over the same context sent the context with each, and reading a large
    context is nearly the whole request. `op: "evaluateAll"` takes the context once and a list
    `evaluations` (each with the fields of one evaluation and an optional `id`); the response
    has `results`, one answer per entry in their order, each what `evaluate` answers for that
    entry, and `contextWarnings` once. An entry that fails is a failure in its place; the
    entries do not see each other; nothing is kept after the request (docs/CONTRACT.md).
    `evaluate` is unchanged. Tests: 20 evaluations over three contexts answered as the single
    requests answer them. Measured (a context of 1.6 MB, medians of 30): one request each
    14.6 ms; 10 entries in one request 15.2 ms. Rust API: none (the wasm boundary only).
234. **The letter of an exponent belongs to a number only before exponent digits.** `1e` and
    `1e+` were the number 1, and `1e + 2` was 3. The letter is part of the number only when
    digits follow it, after an optional sign (DMN 1.6 grammar rule 35); otherwise the number
    ends before the letter: `1e` is a syntax error, `if true then 1else 2` a conditional.
    Identity ledger: 28 entries. Rust API: none.
235. **A name that is not in scope ends before an operator symbol.** With no `x` in scope
    `x - 1` was the one name `x-1` and `x.y` the name `x.y`, each null. Such a name ends before
    `.`, `/`, `-`, `+` and `*`, as before a keyword that can follow an operand; the apostrophe
    stays in the name. A name in scope is matched first, also one written with symbols side by
    side. A name that is being declared (`parse_name`: `Approved/Declined`, `Profit and loss`)
    is the whole text (`Parser::with_whole_names`). Identity ledger: 104 entries; a name that
    is not in scope and has an operator symbol is the expression now (docs/BACKLOG.md). The
    official TCK answers are unchanged. Rust API: none (`parse_name` keeps its signature).
236. **The endpoint of a unary comparison takes the arithmetic after it.** The cell `> x + 1`
    was `(? > x) + 1`, which is null, so a rule with such a cell never matched. The six unary
    comparisons have the precedence of `in` (DMN 1.6 grammar rules 7 and 51); `and`, `or`, a
    comparison and `between` end the endpoint as before. Tables regenerated with Bison 3.8.2,
    the same tokens. Identity ledger: 44 entries. Rust API: none.
237. **A range literal takes a negative number as an endpoint and has an endpoint.**
    `range("[-1..1]")` was null, and `(..)` was a range of nothing. An endpoint is a number
    with an optional minus (grammar rule 67 with the sign of rule 35), and a range literal has
    at least one endpoint (rule 66). Identity ledger: 20 entries. Rust API: none.
238. **The tests of the front end compare with recorded outcomes.** The tests compared every
    input with a frozen copy of an older lexer and parser and a list of allowed differences,
    which grew with every fix. The outcomes are recorded instead (`feel-parser/src/tests/golden`:
    a digest per case at the default size, one per group at the full size, and 110 chosen
    expressions in a readable file with their tokens and trees); the copy (3,900 lines) is
    deleted. A fix that changes an answer rewrites the files in its commit
    (`RHEON_GOLDEN=update`, at both sizes) and says which cases changed. Rust API: none.
239. **`list`, `range` and `context` are names outside a type.** Before a `<` the three words
    were keywords everywhere: `list < 2` and `{list: 1}.list < 2` were syntax errors. They are
    keywords only where a type is expected, and a type is expected for the one token after the
    parser asks; before, the request stood until a built-in type name used it, so a `date` or
    `number` long after `x instance of T` was read as a type. A function type asks right after
    `function <` (tables regenerated, the same tokens); no action that changes the lexer is
    reduced after a lookahead any more. Identity ledger: 36 entries. Rust API: none.
240. **The `and` of a `between` is the parser's choice, and a dot asks for one token.** After
    `between` the next `and` anywhere was the between's, so `x between (if a and b then 1 else
    2) and 5` was a syntax error. The lexer reads every `and` as a conjunction; while a between
    is open the parser asks its tables whether it would shift the between's `and` in its state
    (`Parser::takes`: the reductions are followed beside the stack, no action runs) and reads
    the token as that one where it would. And a built-in property name is looked for in the
    one token after a dot: before, until one was found, so `years and months duration` as a
    parameter type after a path was the name `years`. Identity ledger: 24 entries; `a.b + start
    included` with `start` in scope was null with a warning and is a syntax error, as any name
    in scope followed by a word. Rust API: none.
241. **A built-in name that is longer than the name in scope is the name.** With a variable
    `string` in scope `string length(string)` was a syntax error; with `date` in scope `date
    and time("...")` was a conjunction. After the lookup of the names in scope, the prefixes of
    up to four words are compared with the built-in names: a function name with more words
    than the name in scope wins (one with a keyword in it only before the `(` of its call),
    and where a type is expected a built-in type name wins also with as many words. A variable
    with the same name as a built-in function still stands for the variable. Identity ledger:
    34 entries. Rust API: none.
242. **`evaluateAll` in the npm runtime.** `FeelEngine.evaluateAll(req, opts)` and
    `instantiateFeel(...).call({ op: 'evaluateAll', ... })` send the request of item 233: the
    context once (a `rawJson` context spliced verbatim), `tzOffsetMinutes` filled in as for
    `evaluate`, and each entry with its own fields (a `rawJson` input spliced; the input sent
    for unary tests only). It is one request in the queue, under one timeout and one signal.
    An entry that the runtime can not send (invalid raw JSON, an unpaired surrogate) is a
    failure in its place in `results`, with its `id`, and the other entries are sent. The
    existing methods are unchanged. Tests: every answer equals the answer of `evaluate` for
    the entry; raw digits; failures in place; a failure of the request; a real worker.
    Rust API: none.
243. **The records of an input list hold their names in common.** Each object of the input was
    a context with a map of its own; for a list of records with the same keys a third of the
    reading built and freed these maps. A context has a second form for its entries: a list
    of names, ascending and each once, held in common by several contexts, and the values of
    this one. Every reader sees the entries of a map, and the first change gives the context
    a map of its own. The reader keeps only the values of an object while its keys are those
    of the object before it at its depth. Tests: a context of shared names against the map of
    the same entries, for every read and every change. No answer changes. Measured (wasm):
    a 1.58 MB input of 3,400 records 13.9 to 11.3 ms. Rust API:
    `FeelContext::from_shared_names` (new).
244. **A plain number of the input is read in one pass.** A number was passed three times (its
    end, a search for an exponent, its digits into 128 bits). A number without an exponent and
    with at most 18 digits after its leading zeros is read in the pass that finds its end,
    into 64 bits, with the digits and the scale of its text; every other number goes the way
    it went. The parser of a number text reads its first 19 digits in 64 bits. Tests: 100,000
    number texts against the conversion of their texts. No answer changes. Measured (wasm):
    the same input 11.3 to 8.1 ms; the ordinary speed suite 1.07x, input-heavy requests 1.4x
    to 1.95x. Rust API: none.
245. **A record of an input list is read when it is first used.** A request paid for reading
    all of its input, also when the expression used a small part. An object that is an item
    of an array, of at least 128 bytes, is passed without reading it into values, and is a
    context that reads it when something first uses it (`FeelContext::deferred`, the trait
    `DeferredEntries`); at every depth, so the records inside a record that is read stay unread
    until they are used. The pass validates the object and takes only one that the reader
    reads without a warning and without refusing it; any other is read at once. So the values,
    the warnings, the errors and their order are unchanged. Tests: whatever the pass takes is
    read to the same end, without a warning, into a value of the depth the pass gave; lists of
    records against the conversion through the tree; a context that reads on first use against
    the context of its source. Measured (wasm): an input of 1.58 MB, expression reading 2
    numbers 8.3 to 2.8 ms, reading one field of 160 records 8.3 to 4.9 ms; a field of every
    record read 1.67 to 2.20 ms (the pass is paid on top); the ordinary speed suite 0.99x
    (three workloads that build or read many contexts 0.94x to 0.95x), 1.50x against 0.6.0.
    Rust API: `FeelContext::deferred`,
    `FeelContext::is_built`, `DeferredEntries` (new).
246. **FEEL functions are lexical closures.** The body of a function read the names it did not
    bind through the scopes of whoever invoked it, and a function that a body returned closed
    over the names as they were where the body ended (both kept from dsntk): `{f: function() a,
    r: {a: 1, s: f()}.s}.r` was 1, and `{f: function(x) {x: 10, g: function(y) x + y}.g, h:
    f(1), r: h(5)}.r` was 6. A body now reads its parameters, the names in scope where the
    function is written, the scope of the host and the built-in functions (§10.3.2.13.2). Of
    the context literal it is written in it sees every entry (§10.3.2.11.1: all the entries
    of a context are in scope for an entry's expression), also a later one once that has its
    value, so the functions of one context call each other in any order and themselves: each
    evaluation of such a literal has a `Home` of its entries (rheon-feel), which the functions
    written in it hold. A home can hold a value that holds the home (a function of the
    literal), so the homes of an evaluation are emptied when the evaluation ends, which frees
    them; a function that outlives its evaluation keeps what it closed over and no longer sees
    the entries after it. The bodies of boxed functions of DMN
    models keep the old behaviour (docs/BACKLOG.md). Tests: the pins of tests/scoping.rs (19
    new cases, 9 changed), 29 records of the flat evaluator's tests; a test of the runtime
    that the memory of an instance does not grow over 300 such evaluations. Identity ledger: 58
    entries (`lexical-closures.json`), among them helper functions of a context used by
    `sort`, which were null and work. The official TCK answers are unchanged. Rust API:
    `Home` (new), `FunctionDefinition::home` and `FunctionDefinition::at_home` (new);
    `FunctionDefinition` compares and prints as before.
247. **The identity check compares with 0.7.0.** The baseline moves to the v0.7.0 release module,
    and the ledgers of registered differences against 0.6.0 are removed (490 entries in 11 files
    of `registered/`, one file of `release/registered/`): 0.7.0 answers every committed request
    as registered. Git history keeps the ledgers; their requests stay in the request sources.
    Identity of the 0.7.0 build against the 0.7.0 module: the committed sources are 15,450
    requests, with `--release` 15,462 (5 trapping the reference), no difference.

248. **The parser is written by hand.** `feel-parser/src/parser.rs` is a recursive-descent parser
    with one loop for the operators, in place of the driver of the LALR tables that Bison
    generated from `feel-grammar/src/feel.y` (`lalr.rs`, the table driver and `build.rs` are
    deleted; the token types moved to `token.rs` with their numbers). It reads the tokens of
    the same lexer and tells it what it expects at the places where the generated parser did
    so through its actions; a token is read when the parser first looks at it. For every text,
    start rule and scope it reads the same tokens and gives the same tree or the same error:
    proven before the switch on about 61 million generated cases with both parsers side by
    side, and after it by the recorded outcomes of the parser tests, which did not change.
    One registered difference: the parser takes native stack for each level of nesting and
    stops at 640 expressions open at once (`expression is nested too deeply (more than 640
    levels of nesting)`). A text nested deeper than that was rejected before by the depth
    check after the parser with the number of levels in the message; more than 639 nested
    parentheses around one operand were accepted. Texts whose tree is at most 300 levels deep
    and long chains without nesting answer as before. The ledger is
    `tools/identity/registered/parser-nesting.json`. Also new, and used by nothing yet:
    `parse_tolerant`, which never fails and returns the tokens, the nodes with their places
    and every syntax error of a text (the base of the editor operations); the wasm module has
    a prototype operation `syntax` behind the feature `editor-prototype`, outside the
    contract. `rheon-feel-grammar` is no build dependency any more and is not published
    (`publish = false`): it stays as the reference grammar. The `trace` argument of the entry
    points is accepted and ignored. In a debug build of this workspace the parser crate is
    compiled with `opt-level = 1`, so that the nesting limit fits the stack of a test thread.

249. **Editor operations.** The module answers three new operations (docs/CONTRACT.md §3):
    `syntax` (every token of a text with its class, every syntax error with a place and a
    code, and on request the nodes with their places), `complete` (what can stand at a cursor:
    names of the context and of the text, entries after a dot, keywords by position, built-in
    functions, types) and `outline` (the names of a context without its values, which the other
    two take in place of the context). They read the tolerant parse of the hand-written parser
    and answer for every text; places are UTF-16 code units. `@rheon/feel-runtime` has
    `FeelEngine.syntax`, `complete` and `outline` with their types. The prototype feature
    `editor-prototype` is gone. One registered difference: the message of an unknown `op` names
    the new operations (`tools/identity/registered/editor-operations.json`, 2 entries). Not in
    this version: the type and the signature of an item (`type` and `detail` are null), named
    parameters of built-in functions, and entries after a dot for anything but names that lead
    into the context.

250. **`@rheon/feel-codemirror`, and the playground on it.** A new npm package holds the
    CodeMirror 6 support for FEEL: it asks the engine (`syntax`, `complete`) after every change
    and shows its answers as colours, error marks and completions, with templates of `if`,
    `for`, `some`, `every`, a function and a context where an expression can start; an answer
    for a text that has changed since is dropped. The playground's FEEL editor uses it, and
    its tree with positions is the engine's; `lang-feel`, `lezer-feel` and the lezer runtime
    are no dependencies of the playground any more, and their notices leave
    `third-party-notices.txt`. The release workflow packs the package beside the runtime's.
    The package had no tests of its own at this point (item 255 adds them): the playground's build and a check in a browser
    exercise it.

251. **A syntax error says where and what.** The error of `evaluate`, `evaluateAll` and
    `parse` for a text that does not parse holds `diagnostics`, the errors of the text with
    their places and codes as `syntax` answers them; `message` is unchanged. The playground's
    result panel lists them. Registered as a rule: `error.diagnostics` is gained
    (`tools/identity/registered/syntax-error-places.json`). The messages of the syntax errors
    say what was expected and what stands there ("Expected `then` after the condition of `if`,
    but the expression ends here."); the playground shows each as its line and column, a link
    that selects the place.

252. **A formatter.** `feel-parser/src/format.rs` gives a text one layout from the tolerant
    parse: spaces by the roles of the tokens, and a part that does not fit its line broken by
    its kind with what is inside it indented (docs/CONTRACT.md §3, `op: "format"`). The layout
    does not depend on the layout of the input, so it is stable; names, comments and
    parentheses stay as written. The result is checked: it must parse to the same tree as the
    input, otherwise the text comes back unchanged. On the 17,217 expressions of the corpus at
    two widths every text is formatted and stable. The module has `op: "format"`,
    `@rheon/feel-runtime` `FeelEngine.format`, `@rheon/feel-codemirror` the command
    `formatDocument` (Shift-Alt-F), the playground a Format button. The message of an unknown
    `op` names it (the entries of `editor-operations.json` are recorded again).

253. **The identity check compares with 0.8.1.** The baseline moves to the v0.8.1 release module
    (the tag v0.8.0 has no release), and the ledgers of registered differences against 0.7.0 are
    removed (`parser-nesting.json`, `editor-operations.json` and the rule
    `syntax-error-places.json`): 0.8.1 answers every committed request as registered. Git
    history keeps the ledgers. Identity of this build against the 0.8.1 module: 15,450 requests,
    with `--release` 15,462 (5 trapping the reference), no difference.

254. **The signatures of the built-in functions.** `feel/src/signatures.rs` lists for each of the
    89 built-in functions its signatures (the names and types of the parameters, optional and
    rest parameters, the type of the result, whether the parameters can be named) and one line
    that says what it gives. The evaluator does not read the table; a test of the evaluator
    invokes every signature by position and by name with values of the types and checks that
    the function takes it. `op: "complete"` uses it (docs/CONTRACT.md §3): an item of a function
    has `detail`, `info` and `type`; inside the parentheses of a built-in function the answer
    has `call` (the signatures and the index of the argument); where a parameter can be named,
    the names are items of the kind `parameter` with `insert`; `expects` can be `parameter`.
    Additions only: no answer of 0.8.1 changes except that the items of functions have values
    where they had null. `@rheon/feel-runtime` has the types; `@rheon/feel-codemirror` shows
    the signature in the completion list and a hint at the cursor (`signatureHelp`).
    Found on the way, not changed: a built-in function reports a named parameter of a wrong
    type as "parameter not found", and `< 10` evaluates as an expression to a value that
    displays as `UnaryLess(10)`.

255. **Tests of `@rheon/feel-codemirror`.** `packages/feel-codemirror/test/editor.test.mjs` runs
    the extension in jsdom (a development dependency) with the engine in the same thread: the
    classes of the tokens, edits and dropped answers, error marks and `maxDiagnostics`, `tree`,
    completion of names, functions, parameters and templates, `setFeelScope`, `formatDocument`
    and the hint. The group tier runs them as the step `test-codemirror`.

256. **`e1 in e2` with a single value is `e1 = e2`.** DMN 1.6 Table 56 (rule 49.c) defines `in`
    with one value on the right as equality. The `in` operator answered false where `=` answers
    null because the two values are of different kinds: `"sad" in true` was false and is null
    (in strict error mode it is the error of `=`), as `"sad" = true` is. Values of one kind,
    null on either side and B-FEEL (where `in` is always true or false) answer as before.
    A list of tests (`x in (true, false)` and the unary tests dialect) follows in item 260. New request set
    `tools/identity/requests/membership.json` (23 requests); 8 registered differences (one of them B-FEEL, where the answer stays false and the diagnostic of `=` is reported) in
    `tools/identity/registered/in-single-value.json`. Tracker row EVA-25.

257. **A parameter that is given is not reported as not found.** The ten built-in functions
    that take one list by the name `list` (`all`, `any`, `max`, `mean`, `median`, `min`,
    `mode`, `product`, `stddev`, `sum`) answered "parameter 'list' not found" when the
    parameter was given with a value that is no list (`sum(list: 1)`, `sum(list: null)`). The
    null now says `[core::sum] invalid argument type, expected list, actual type is number`; a
    parameter of another name is still not found. The messages of `substring` named `sublist`
    in five places and name `substring`. The answers stay null; only the reasons change. A test
    gives every named parameter of every signature a value of six other kinds and checks that
    none is reported as not found. New request set
    `tools/identity/requests/parameter-messages.json` (40 requests); 31 registered differences
    in `tools/identity/registered/parameter-messages.json`.

258. **A comparison with one endpoint is a range.** DMN 1.6 clause 10.3.2.7 makes `< 10` a
    value of the range type (grammar rule 7.a, Table 42). As an expression it was shown by the
    name of its variant (`UnaryLess(10)`, also by `string()` and inside lists and contexts),
    had the type `boolean`, and `instance of` answered null for it. It is shown as it is
    written (`< 10`, `>= 2020-01-01`, `= 5`, `!= 5`), has the type `range<number>` (the type of
    its endpoint), `instance of` answers for it, and it conforms to a parameter of a range
    type. How unary tests match, `in`, the properties and equality are not changed. Not
    changed and still open: `range("(..10)") = (< 10)` is null where the spec makes them the
    same range, and `.start` of `< 10` is null with a message. New request set
    `tools/identity/requests/unary-values.json` (63 requests); 62 registered differences in
    `tools/identity/registered/unary-values.json`, 7 of them on requests of older sets.

259. **The identity check has requests of the editor operations.** The committed request sets
    held no `syntax`, `complete`, `outline` or `format` request, so a change of their answers
    was caught only by the contract tests. `tools/identity/requests/editor.json` has 260: whole
    and broken texts as `syntax` (with and without the tree) and `format` (two widths), both
    dialects, 50 `complete` requests at the places the grammar distinguishes, `outline` of six
    contexts, an outline in place of the context, and the request errors. Against 0.8.1 the 24
    `complete` answers that item 254 changed are registered in
    `tools/identity/registered/completion-signatures.json` (answers longer than 4 KiB by their
    digest); the other 236 are identical. Found on the way, not changed: inside a line comment
    `complete` offers what an expression takes. Tracker row EDT-13.

260. **Unary tests answer true, false or null; a test that uses `?` is its own answer.** Two
    rules of the specification, made together because the first lays the second bare.
    (a) DMN 1.6 Table 56 defines a list of tests as the `or` of its tests, and clause 8.3.3
    says an input satisfies an entry when `FEEL(e in (t)) = true`, and a negated entry when it
    is `false`: three answers. The `in` operator answered false when no test was true, also
    when a test could not be compared with the tested value (`"a"` tested by `1, 2` or by
    `< 10`), and `not(...)` answered true there, so an entry `not(1, 2)` matched the input
    `"a"`. Now the answer is true when one test is true, false when all are false, and
    otherwise the null of the first test without an answer, with its reason; `not(...)` is null
    where the tests are. This is `x in (a, b)` in expressions and the unary tests dialect. A
    list value (`x in [1, 2]`, a list among the tests) is searched as before, and B-FEEL, where
    a comparison is true or false, answers as before (kernel `InTwoValued`). In strict error
    mode a value of another kind than the tests is now an error for values as it already was
    for comparisons and ranges. A decision table matches on true, so null is no match.
    (b) Clause 7.3.2 d and the last row of Table 56: a test that is a boolean expression using
    `?` is satisfied when it is true. The engine compared the boolean with the tested value
    (`? > 5` with 7 was `7 = true`, false), documented with a warning. Now a test that uses `?`
    and gives a boolean is that answer (`Value::TestAnswer`, kernel `TestAnswer`); when it gives
    null and is a boolean expression by its form (a comparison, `and`, `or`, `between`, `in`,
    `instance of`, an invocation) the answer is that null; any other value is compared with the
    tested value as before (`(? * 0) + 7`), and so is `?` alone. The warning is gone.
    Official TCK: unchanged (3,392 of 3,407, the same 15 as the baseline). New request set
    `tools/identity/requests/unary-test-answers.json` (275 requests); 168 registered differences
    in `tools/identity/registered/unary-test-answers.json` (82 for (a), 86 for (b)).
    Found on the way, not changed: a decision table of a DMN model does not bind `?` for its
    input entries (only the allowed values of an item definition do), so `? > 5` in a model's
    table is still not satisfied. Tracker row EVA-25.

261. **The comparisons are one left-associative level.** Grammar rule 49 lists `=`, `!=`, `<`,
    `<=`, `>`, `>=`, `between` and `in` as one production, and rule 4 gives the comparison one
    place in the order of precedence. The parser had three levels inherited from the generated
    tables: the six operators (not associative, so `1 < 2 < 3` and `a = b = true` were syntax
    errors), `between` above them and `in` above that (right associative). Now a comparison
    takes the comparison on its left as its operand and an arithmetic expression on its right:
    `a = b = true` is `(a = b) = true`, `1 < 2 < 3` is `(1 < 2) < 3` (null: a boolean is not
    compared with a number), `a = b in c` is `(a = b) in c` (it was `a = (b in c)`),
    `a = x between 1 and 2` is `(a = x) between 1 and 2`, `a in b in c` is `(a in b) in c`,
    and the endpoint of a unary comparison ends before `in` as before the other comparisons
    (item 236). A comparison written first is unchanged (`x in c = true`,
    `x between 1 and 2 = true`). The diagnostic code `comparison` of `syntax` is gone with the
    error. A long chain of comparisons is no nesting; the depth limit of the tree applies to
    it. Tracker row PAR-20.

262. **A `[` after the start endpoint of an interval that starts with `]` is a filter.**
    `]a[1]..b]` was a syntax error: after the `]` that opens an interval the generated tables
    read every `[` as the bracket that closes one, also after the start endpoint, where no
    interval can end. The start endpoint is now read as in `[a[1]..b]` and `(a[1]..b)`; after
    the end endpoint a `[` closes the interval as before (`]a..b[`). Identity ledger: 54
    entries, all in the new request set `interval-filters.json`.

263. **A comparison with one endpoint equals the range whose other endpoint is undefined.**
    DMN 1.6 clause 10.3.2.7 and Table 42: `< 10` is the range with the end 10, not included,
    and an undefined start; the range function's row says `range("(..2)")` is `< 2`. Equality
    had one arm per form: `range("(..10)") = (< 10)` was null, `(< 10) = range("(..10)")` was
    false, `(< 10) = (1..10)` was true and `(< 10) = (<= 10)` was null. Now the four
    comparisons and the intervals are compared as ranges: the same flags, and the same
    endpoints, where an undefined endpoint equals only an undefined one. An endpoint written
    as `null` is a value, so `(< 10) = (null..10)` stays false (TCK 0068, range_006 to 009).
    The undefined endpoint is null without an error (`(< 10).start`, `(> 1).end`; it was
    "no such property"), and a range with an undefined endpoint has the type of its other
    endpoint (`range("(..10)")` is a `range<number>`, it was `range<Any>` and no instance of
    `range<number>`). So `index of`, `list contains` and `distinct values` find such a range
    in either form. Found, not changed: the range functions (`includes`, `overlaps`,
    `coincides`, ...) take no range with an undefined endpoint in either form. Identity
    ledger: 49 entries. Tracker row EVA-29.

264. **Completion offers nothing inside a comment or a string that is not closed.** With the
    cursor inside a line comment, a block comment without its `*/` or a string without its
    closing quote, `complete` offered what the grammar takes after the text before it (the
    names, keywords and functions of an expression, or the operators). It now answers
    `expects: "comment"` or `"string"` with no items; `call` is still told, so the signature
    of the function around a string argument stays. Identity ledger: 17 entries. Tracker row
    EDT-14.

265. **A word that is a keyword is the keyword whatever follows it.** DMN 1.6 clause 10.3.1.4:
    "A name start (grammar rule 26) SHALL NOT be a literal terminal symbol." The lexer read
    `and`, `between`, `else`, `every`, `external`, `for`, `if`, `in`, `instance`, `of`, `or`,
    `return`, `satisfies`, `some` and `then` as keywords only before white space, and `true`,
    `false` and `null` only before some characters; before anything else the word started a
    name. Now the word is the keyword when no name character follows it, and a longer word
    (`iffy`, `in1`, `order`) is a name as before. So `x in[1, 2]`, `x in(1, 2)`,
    `if(a)then(1)else(2)`, `for x in[1] return(x)` and `a and(b)` parse (they were syntax
    errors). And a name that starts with a keyword is none, also where the old lexer took
    it: a name in scope (`in-progress`, `and/or`, `if-flag`), a key (`{in: 1}`, `{true: 1}`;
    the string key `{"in": 1}` stays), a parameter (`function(in) 1`, `f(in: 1)`) and a member
    (`x.and.y`; `x.in` before white space already was an error). Such an entry is read with
    `get value(x, "in")`. `null()` and `true()` invoke the literal (null, as before, with
    another reason). Identity: 6 of the 16,876 earlier requests differ (`null(1)`); the new
    request set `keywords-and-item.json` registers the rest. The official TCK is unchanged.
    Tracker row PAR-17.

266. **The word `item` is a name like any other.** The lexer returned the name `item` whenever a
    name started with that word (inherited, for filters), so a name that starts with it was
    never read: with `item count` in the context `item count + 1` was a syntax error, `item-id`
    was `item - id`, and `{item x: 1}`, `for item count in ...` and `function(item x) ...` were
    syntax errors. It also left the lexer in the mode of the iteration variable after
    `for item in`, so `in+1` in the body was read as a name. A filter needs no special case:
    `item` alone ends before an operator, a keyword or a bracket as any name that is not in
    scope does. Part of tracker row NAM-5.

267. **The entries of a context literal are evaluated in the order they need each other.**
    DMN 1.6 says both "may reference keys from previous context entries" (clause 10.3.2.6) and
    "All of the entries of m are in-scope for e, but the depends on graph SHALL be acyclic.
    [...] Simply evaluate the context entries in depends on order" (clause 10.3.2.11.1, and
    Table 70). The owner decided for the second on 2026-11-18. An entry saw only the entries
    written before it, so `{a: b + 1, b: 1}.a` was null, or read a `b` outside the literal.
    Now:
    - an entry is evaluated after the entries its value reads, and otherwise in the order
      written: `{a: b + 1, b: 1}.a` is 2, also where a `b` outside the literal exists (that
      answer changes from the outer `b` to the entry);
    - an entry does not read its own key (clause 10.3.2.6, TCK 1146 and 0085): `{x: x + 1}`
      reads the `x` outside the literal, as before;
    - a function definition needs nothing to be made, and what its body reads is needed when
      it is invoked: `{i: f(), f: function() a, a: 1}.i` is 1, and function entries invoke
      each other and themselves as before;
    - entries that need each other in a circle have no value: `{a: b, b: a}` gives null for
      both, each with an error that names the entry (`a` read the outer `b` before);
    - a name that a construct inside the value binds (an iteration variable, a parameter, a
      key of a literal inside) is not the entry;
    - a key of several tokens is a name in scope in the entries written before it too
      (`{x: a-b + 1, a-b: 2}.x` is 3), found by one pass over the literal before its entries
      are parsed, and never in its own value (`{a-b: a-b}` subtracts).
    The order is found from the syntax tree before the names are resolved
    (`flat/depends.rs`); the layout of the literal binds its entries in that order, so an entry
    sees exactly the entries evaluated before it. A consequence: `{r: abs(-1), abs: 1}` reads
    the entry `abs`, not the built-in function. Boxed contexts of DMN models are not changed
    here. Identity: 13 of the earlier requests differ; with the new request set
    `context-entry-order.json` they are registered. Tracker row NAM-4.

268. **A name is matched against the keys of the scopes it stands in; nested keys are names
    only for a member of a value.** Every key at any depth of the input, and every key of a
    context literal from its entry to the end of the text, was a name in scope everywhere
    (inherited): with the input `{x: {"a-b": 1}, a: 5, b: 2}` the text `a-b` was read as the
    name `a-b`, which has no value there (null), and in `[{a-b: 1}, a-b]` the second `a-b` was
    read as the key of the literal before it. Now (the first half of tracker row NAM-6):
    - a name that stands alone is matched against the keys of the contexts in scope
      themselves: the input's own keys, the variables and parameters of the constructs around
      it, and the keys of the context literals it stands in. `a-b` above is 3;
    - a member of a value is matched against the keys at any depth too, and against the keys
      of the context literals of the text: the name after a dot (`x.a-b`, `{a-b: 1}.a-b`) and
      the names in the predicate of a filter, where the entries of an item are in scope
      (`l[a-b = 1]`);
    - the key of a context literal is a name of the literal's own scope: in the entries of
      the literal (item 267) and in what stands inside them, not after or beside the literal.
    The lookups of the member names are the ones that were there (the tests of the index run
    with every name as a member name, and their recorded outcomes are unchanged). None of the
    17,268 earlier identity requests differs, and the official TCK is unchanged; the new
    request set `member-names.json` registers the rule. Also fixed here: the scan for the keys
    of a literal (item 267) started one byte after the start of the token, not after the
    brace, so with white space before the `{` it found no keys, and it panicked when that
    white space was a character of several bytes (found by the full-size parser tests).

Verification after commits 21-22 (native, dashu): feel-number tests on both
backends, feel-regex, feel-evaluator 3376 passed / 13 failed (Java RPC
tests) / 3 ignored, model-evaluator compatibility tests (all but TCK 0076,
Java). Unpatched TCK subset: 289/298 (remaining: 0036 typeRefs, 1115/1117
dates beyond chrono's year range). Patched TCK: unchanged, 3377/3407 result
rows (same 30 non-successes as before: 0076, 0085 015, 1157, 1160).

Verification after commits 14-20 (native, dashu): feel-number 21 unit +
228 integration tests pass; feel-temporal 57 (lib) + the `tests/` suite,
feel-parser 461+; feel-evaluator 3370 passed, 13 failed (the
`external_functions` tests, which need the Java RPC service), 3 ignored;
model-evaluator 3651 passed, 18 failed (all TCK 0076, Java external
functions), 20 ignored. Clippy is clean for the touched crates.

### Known, intended differences between the dashu core and the Intel backend

History: the Intel backend was removed in item 95; these were the differences while both
existed.

- `exp`, `ln`, FEEL `log`, `pow` with a non-integer exponent or an exponent
  outside `i32`: the core is correctly rounded, Intel (binary128 inside) is
  1 ulp off in 11-16% of random inputs, 2-9 ulp in about 0.3%, rarely more.
- Scaling to the overflow edge: Intel's `bid128_scalbn` pads the coefficient
  only to about 33 digits before reporting overflow, the core to 34, so
  `floor(999999999999999E+6116, 14)` is finite here and null (overflow) on
  Intel.
- `ilogb + scale` outside `i32` (a zero with a negative scale, e.g.
  `ceiling(0, -1)`, or a scale near the `i32` limits): Intel overflows the
  addition (panic with overflow checks, as in dsntk-wasm release builds;
  wrapping otherwise, which then reports the invalid scale with the wrapped
  number); the core computes it in `i64` and reports the invalid scale with
  the clamped value. Both are null in FEEL, the reason text differs in the
  quoted number. `FeelNumber::new(n, i32::MIN)` likewise saturates instead
  of overflowing `-s`.
- The cohort (trailing zeros, visible only through `Debug` and `abs()`) of
  an `exp`/`ln` result that is an integer or a short dyadic fraction follows
  an emulation of Intel's binary128 conversion; it differs in about 1.5e-5
  of such results (`exp(74.971090)`, `exp(log(2.5))`, `log(exp(7))`).

## Verification (native, cargo 1.95.0 / rustc 1.95.0)

Commits 227-232 (2026-11-16), on the tree of commit 232, by the release tier (`npm run
tier:release`, 814 s of run time and 57 s of builds, budget 1,800 s; 32 of 32 steps): workspace
8,903 passed, 0 failed, 24 ignored (CI's skips: the Rust TCK copies 3,555, the model evaluator 168,
the other crates 5,180); clippy on the workspace without warnings, also on every one of commits 227-232 on its own, each
of which passes `cargo check --workspace --all-targets`; the API documentation builds with
rustdoc's warnings denied; feel.wasm tests 93 native and 94 under Wasmtime; runtime 34; the
identity tool's tests pass; smoke passes, with its two memory checks growing by 12.0 MiB and
22.8 MiB (bound 32 MiB) and the 26 string doublings by 256.0 MiB (at most 281.6); the randomized
and exhaustive tests at full size 1,051 passed and the parser corpus 512; the playground builds.
The number oracle judges every case right with no exemption: 35,939 cases quick and 2,896,591 at
full size, in 188 families. DMN TCK via `scripts/tck.sh`: official 3,392 of 3,407, tck-addendum
312 of 312, and the result of every test equal to that on the tree of commit 221. Full responses
against the 0.6.0 release module, every cached reference answer equal to a live one: the committed
sources 14,976 requests, the release sources 14,988, the two sets kept outside the repository
15,153; the registered differences are those of items 215, 222 and 223 and no other, none
unregistered in any run. Speed against 0.6.0 (the speed suite of 58 workloads, wasm, Node 26,
Apple M2, two alternating rounds): geomean 1.44x and 1.43x (1.20x on the tree of commit 226).
`1 + 2` 4.8 us to 3.9 us (1.25x), a formula over 1,000 input names 453 us to 237 us (1.92x), over
5,000 names 1.98x, a `for` over an input list of 10,000 contexts 1.76x, a filter over it 1.40x,
counting 10,000 numbers in a loop 2.58x. Slower than 0.6.0 in both rounds: a recursion through
`sort` callbacks, 0.92x and 0.94x. The boundary workloads (15 requests whose cost is the reading
of the input or the writing of the result, kept outside the repository) against the tree of
commit 226: a list of 10,000 numbers returned 2.6x, 10,000 input contexts echoed 2.3x, `1` with
5,000 input names 2.2x. The module is 3,904,418 bytes (commit 226: 3,849,528; +1.4%). Commits 227
to 230 passed the group tier one by one; commits 231 and 232 the fix tier and the group tier on
the tree of 232. Not run: CI, Node 24 and Node 22.

Commits 222-226 (2026-11-16), on the code of commit 226 (two corrections of documentation were
folded into commits 222 and 223 after the run), by the release tier (`npm run tier:release`, 803 s
of run time and 48 s of builds, budget 1,800 s; 32 of 32 steps): workspace 8,891 passed, 0 failed,
24 ignored (CI's skips: the Rust TCK copies 3,555, the model evaluator 168, the other crates 5,168);
clippy on the workspace without warnings, also on every one of commits 222-226 on its own, each of
which passes `cargo check --workspace --all-targets`; the API documentation builds with rustdoc's
warnings denied; feel.wasm tests 83 native and 84 under Wasmtime; runtime 34; the identity tool's
tests pass; smoke passes, with its two memory checks growing by 12.0 MiB and 27.3 MiB (bound 32
MiB) and the 26 string doublings by 256.0 MiB (at most 281.6); the randomized and exhaustive tests
at full size 1,039 passed and the parser corpus 511; the playground builds. The number oracle judges
every case right with no exemption: 35,939 cases quick and 2,896,591 at full size, in 188 families.
DMN TCK via `scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, and the result of
every test equal to that on the tree of commit 221. Full responses against the 0.6.0 release
module, every cached reference answer equal to a live one: the committed sources 14,976 requests,
142 registered differences reproduced (items 222 and 223: 104 and 38); with the release sources
14,988 requests, 143 reproduced (with item 215), 6 trapping 0.6.0; with the two sets kept outside
the repository 15,153 requests, 142 reproduced; none unregistered in any run. Speed against 0.6.0
(the speed suite of 58 workloads, wasm, Node 26, Apple M2, two alternating rounds): geomean 1.20x
and 1.20x (1.18x on the tree of commit 221). Against that tree, in both rounds: a formula over
1,000 input names 1.00x to 1.16x, over 5,000 names 1.00x to 1.20x, a `for` over an input list of
10,000 contexts 1.04x to 1.19x, a deep path with 1,000 input names 1.01x to 1.17x, a filter that
calls a function 1.01x to 1.13x, `context put` on the input context 1.01x to 1.12x; no workload
is slower than on that tree beyond the difference between two rounds (0.03x). The module is
3,849,528 bytes (commit 221: 3,830,095; +0.5%). Not run: CI, Node 24 and Node 22.

Commits 179-221 (2026-11-16), on the tree of commit 221, by the release tier (`npm run
tier:release`, 904 s of run time and 95 s of builds, budget 1,800 s; 32 of 32 steps): workspace
8,881 passed, 0 failed, 24 ignored (CI's skips: the Rust TCK copies 3,555, the model evaluator 168,
the other crates 5,158); clippy on the workspace without warnings, also on every one of commits
179-221 on its own, each of which passes `cargo check --workspace --all-targets`; the API
documentation builds with rustdoc's warnings denied; feel.wasm tests 83 native and 84 under
Wasmtime; runtime 34; the identity tool's tests pass; smoke passes, with its two memory checks
growing by 12.0 MiB and 27.3 MiB (bound 32 MiB) and the 26 string doublings by 256.0 MiB (at most
281.6); the randomized and exhaustive tests at full size 1,029 passed and the parser corpus 502; the
playground builds. The number oracle judges every case right with no exemption: 35,939 cases quick
and 2,896,591 at full size, in 188 families. DMN TCK via `scripts/tck.sh`: official 3,392 of 3,407,
tck-addendum 312 of 312. Full responses against the 0.6.0 release module, every cached reference
answer equal to a live one: the committed sources 14,718 requests with no difference; with the
release sources 14,730 requests, 1 registered difference reproduced (item 215), 6 trapping 0.6.0;
with the two sets kept outside the repository 14,895 requests; none unregistered in any run. Before
the evaluator of closures was deleted, on the tree of commit 215, the two evaluators were compared
in one process: 38,204 requests and 50,000 random expressions without a difference and without an
expression that the flat evaluator did not take, and the 3,724 model tests with a twin for every
evaluation. The stack report: a recursion of 200 calls whose body is nested to the limit of the
parser (depth 289) is answered on every host measured, and so is the recursion through `sort` (depth
284): Node's main thread and workers of 2, 3 and 4 MiB with Liftoff and TurboFan, and Wasmtime with
512 KiB, 1, 2, 4 and 16 MiB; 0.6.0 reached depth 45 on Node's main thread and 19 under Wasmtime with
512 KiB. Speed against 0.6.0 (the speed suite of 58 workloads, wasm, Node 26, Apple M2, two
alternating rounds): geomean 1.18x and 1.18x. Loops: counting 10,000 numbers 1.02 ms to 390 us
(2.6x), summing them 2.0x, nested loops 2.5x to 2.8x, a built-in function in a loop 2.1x, `instance
of` in a loop 2.2x, a context built in a loop 1.9x, short-circuit logic 1.9x. Calls of functions in
a loop 1.3x to 1.5x; `fib(20)` 1.09x; sort 1.14x. Filters and quantifiers over an input list 0.98x
to 1.01x (their requests are mostly the reading of the input). Slower in both rounds: small
expressions that are built for one evaluation (tiny 0.93x, unary tests 0.88x to 0.93x, a date
addition 0.88x, a comparison of two date and times 0.88x to 0.91x, decimal arithmetic 0.92x, a
formula over ten names 0.93x to 0.95x, a deep path 0.90x to 0.92x, a string concatenation 0.91x to
0.93x: about 0.3 us to 0.6 us each) and a recursion through `sort` callbacks (0.87x). The first
request of a new instance takes about 1 ms longer on V8 (5.5 ms against 4.5 ms, measured on the tree
of commit 203). The module is 3,830,095 bytes (0.6.0: 3,759,555; +1.9%). Not run: CI, Node 24 and
Node 22.

Commits 135-177 (2026-11-16), on the tree of commit 177, by the release tier (`npm run
tier:release`, 1,320 s of run time and 77 s of builds, budget 1,800 s): workspace 8,798 passed, 0
failed, 24 ignored (CI's skips: the Rust TCK copies 3,555, the model evaluator 167, the other crates
5,076); clippy on the workspace without warnings, also on every one of commits 135-177 on its own,
each of which passes `cargo check --workspace --all-targets`; the API documentation builds with
rustdoc's warnings denied; feel.wasm tests 81 native and 82 under Wasmtime; runtime 33; the identity
tool's tests 22; smoke passes, with its two memory checks growing by 12.0 MiB and 26.5 MiB (bound 32
MiB) and the 26 string doublings by 256.0 MiB (at most 281.6); the randomized and exhaustive tests
at full size 1,024 passed and the parser corpus 502; the playground builds. The number oracle judges
every case right with no exemption: 35,939 cases quick and 2,896,591 at full size, in 188 families.
DMN TCK via `scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, every per-test
result identical to 0.5.0's. Full responses against the 0.5.0 release module, each evaluate request
also sent as parse, every cached reference answer equal to a live one (`--verify-cache`): the
committed sources 12,934 requests, 64 registered differences reproduced and 80 explained by the
three rules of items 169 and 171 (59, 16 and 5); with the release sources 13,007 requests, 85
registered differences reproduced, 30 of them trapping 0.5.0; with the two sets kept outside the
repository 13,111 requests, 2 of them left out by name (item 177); none unregistered in any run. The
stack report, against 0.5.0: the recursion reaches depth 45 and 46 on Node's main thread with
Liftoff and TurboFan (22 and 24), 94, 152 and 209 in workers of 2, 3 and 4 MiB with Liftoff (46, 75,
104) and 95, 153 and 210 with TurboFan (51, 83, 114), and 19, 48, 105, 220 and 289 under Wasmtime
with 512 KiB, 1, 2, 4 and 16 MiB (10, 26, 57, 120, 289); the parse op's nesting limits and the value
depth limit of 500 are unchanged. The string doubling's peak memory (n = 17 to 26, 267.38 MiB at 26,
growth 2.00x the result) equals that of 0.5.0. Speed against 0.5.0 (the speed suite, one round,
wasm, Node 26, Apple M2): geomean 1.32x over 39 workloads, every result equal; the index loop over
1,000 contexts 351x, `and` / `or` that stop early 7.31x, the B-FEEL loops 1.99x and 1.97x, `sort`
1.10x, `list replace` 1.06x; below 0.97x only `tiny` (0.96x, 5.1 µs against 4.9 µs). Module size
3,759,555 B (3,737,388 B on 0.5.0). The publish dry run needs the crates.io index and was not run.

Commits 85-133 (2026-11-15), on the tree of commit 133: workspace 8,724 passed, 0 failed, 24
ignored (CI's skips), the same with `RHEON_FULL_TESTS=1`, and the README's full-size run
(`RHEON_FULL_TESTS=1`, release, the four crates) 989 passed; clippy on the workspace without
warnings; the API documentation of the ten published crates builds with rustdoc's warnings denied
(item 133); feel.wasm tests 79 native (8 unit, 71 contract) and 80 under Wasmtime; runtime 33;
smoke passes, with its two memory checks growing by 16.6 MiB and 26.5 MiB (bound 32 MiB); the stack
check passes, and its report's recursion depth is at least that of the start of these commits on
every host: Node's main thread 22 with Liftoff and 24 with TurboFan (start 18 and 14, 0.4.0 17 and
14), workers of 2, 3 and 4 MiB 46, 75 and 104 with Liftoff (40, 64, 89) and 51, 83 and 114 with
TurboFan (31, 50, 69), Wasmtime with 512 KiB, 1, 2, 4 and 16 MiB 10, 26, 57, 120 and 289 (10, 25,
57, 98, 98); the playground builds. The number oracle (item 90) judges every case right with no
exemption: 35,939 cases quick and 2,896,591 at full size, in 188 families. DMN TCK via
`scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, every per-test result identical
to 0.4.0's. Full responses against the 0.4.0 release module, each request sent as written: 194,079
requests (the committed sources, each evaluate request also as parse, the ledger, and the request
sets kept outside the repository), 2,789 registered differences reproduced (1,144 of them from
commits 85-133, counted per change in their items), none unregistered, and every cached reference
answer equal to a live one (`--verify-cache`). 52 of the requests trap 0.4.0: 30 of them answer now
and are registered; the other 22 trap the build as well, as they did at the start of these commits:
loops that double a shared list 30 times (`for i in 1..30 return ... [partial[-1], partial[-1]]`),
which exhaust the module's memory. Against the module of commit 129 the same requests differ only
in the 51 registered answers of items 130-132. Speed against 0.4.0 (engine benchmarks, 24
workloads): geomean 1.02x, every result equal, from 0.92x (`fib15`, 572 to 619 µs) to 1.10x
(`loop-count-1k`); against the start of these commits 1.03x, from 0.98x to 1.08x; 11 regex
workloads against 0.4.0: geomean 1.03x, from 1.00x to 1.23x, every result equal except the one
whose replacement string the F&O rules read differently. The publish dry run needs the crates.io
index and was not run.

Commits 55-84 (2026-11-14), on the tree of commit 84: workspace 8,639 passed, 0 failed, 24 ignored
(CI's skips); clippy without warnings; feel.wasm tests 69 native (8 unit, 61 contract) and 70 under
Wasmtime (8, 62); runtime 33; smoke passes, with its two memory checks growing by 22.1 MiB and 26.5
MiB (bound 32 MiB); the stack check passes, and its report reaches at least 0.4.0's depth on every
host (item 67); the playground builds; the randomized and exhaustive tests at full size
(`RHEON_FULL_TESTS=1`, release, with the parser corpus, now with `rheon-feel-wasm`) 971 passed. DMN
TCK via `scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, every per-test result
identical to 0.4.0's. Full responses against the 0.4.0 release module, each request sent as
written: 187,677 requests (the committed sources, each evaluate request also as parse, the ledger,
and request sets kept outside the repository: the fraction sweep, the duration, message and regex
probes and the performance workloads), 1,645 registered differences reproduced (counted per change
in items 59-83), none unregistered; 52 of the requests trap 0.4.0: 30 of them answer in the build,
and the other 22, loops that double a shared list 30 times, exhaust the module's memory there as
well. The full-size edge-operand product of item 84 (28,110 requests with its parse twins), which
is not an identity source, differs in 568 answers: its 24 answer changes that are not message
changes are among the registered ones, and its other 544 answers differ only in null reasons that
quote a number in scientific notation, instances of the registered message rule (item 68) that are
not listed one by one. Speed against 0.4.0 (engine benchmarks, 24 workloads): geomean 1.01x, every
result equal, from 0.94x to 1.11x.

Commits 39-53 (2026-11-13): workspace 8,488 passed, 0 failed, 24 ignored (CI's skips); clippy
without warnings; feel.wasm tests 56 native and 57 under Wasmtime; runtime 33; the randomized
and exhaustive tests at full size (`RHEON_FULL_TESTS=1`, release, with the parser corpus) 867
passed, with no unregistered difference against the frozen reference front end; DMN TCK via
`scripts/tck.sh`: official 3,392 of 3,407, tck-addendum 312 of 312, no result changed against
`docs/tck-baseline.txt`.

Commit 54 (2026-11-13): workspace 8,500 passed, 0 failed (CI's skips); feel.wasm tests native
and under Wasmtime, runtime 33 and smoke pass; full responses of 1,382 requests identical to
the build before it; DMN TCK per-test results unchanged; the stack check passes, every depth at
least as deep as before (Wasmtime with 512 KiB, 1 MiB, 2 MiB: 9, 23, 52 nested levels, was 8,
21, 48).

DMN 1.6 work (commits 23-34, 2026-11-10): workspace 8,383 passed, 0 failed (CI skips);
feel.wasm contract tests 53 under wasmtime; runtime 33; DMN TCK via `scripts/tck.sh`
(patched, DecisionToolkit runner): official 3,392 of 3,407 (0076 Java 14, 0085 runner 1), was
3,377; tck-addendum 312 of 312 runnable cases (error-result cases are skipped by the runner and
pinned by Rust tests). Failing tests: `docs/tck-baseline.txt`.

Earlier numbers (the `intel` and `dashu` features below were removed in item 95):

Default features (`dashu`), at commit 10:

```sh
cd ~/dev/rheon
cargo build --workspace                       # no dfp-number-sys in `cargo tree`
cargo test -p rheon-feel-number               # 21 unit + 207 integration tests
cargo test -p rheon-feel-temporal -p rheon-feel-parser -p rheon-model
cargo test -p rheon-feel-evaluator --lib -- --skip external_functions
cargo test -p rheon-model-evaluator
cargo clippy -p rheon-feel-number --lib --tests
```

| Crate | dashu (default) | intel (`--no-default-features --features intel`) |
|-------|-----------------|-----------------------------------------------|
| rheon-feel-number | 228 passed, 0 failed (21 unit, 207 integration) | 207 passed, 0 failed |
| rheon-feel-temporal | 79 passed | 79 passed |
| rheon-feel-parser | 461 passed | 461 passed |
| rheon-model | 45 passed | 45 passed |
| rheon-feel-evaluator | 3371 passed, 13 failed, 3 ignored | 3370 passed, 14 failed, 3 ignored |
| rheon-model-evaluator | 3650 passed, 19 failed, 20 ignored | 3650 passed, 19 failed, 20 ignored |

The 13 feel-evaluator failures are `tests::external_functions::*` (Java RPC
server at `127.0.0.1:22023`, same on both); the 14th on intel was
`tests::bifs::bif_now::_0001` (`now()` crossing a second boundary, flaky).
The 19 model-evaluator failures are `dmn_3_0076::*` (18, Java external
functions) and `dmn_3_0103::_0017` (`time_006`, depends on the current
Australia/Melbourne DST offset), identical on both backends; before commit 9
dashu also failed `dmn_3_0100::_0278` (`5 ** 2.55`, Intel's 1-ulp value).

The Intel runs need the patched `dfp-number-sys` (the crates.io version has
FFI signature bugs), injected without committing a machine-specific path:

```sh
cargo test --no-default-features --features intel \
  --config "patch.crates-io.dfp-number-sys.path=\"$(tools/fetch-dfp-number-sys.sh)\"" \
  -p rheon-feel-number
git checkout Cargo.lock          # --config patch.crates-io rewrites the lock file
```

`cargo check --workspace --no-default-features --features intel` (with the
same `--config`) succeeds and `cargo tree` then shows no dashu crate.

### Differential results (`tools/`, commit 10)

The two tools were removed in item 94; the number oracle (item 90) judges the number core since.

- `tools/difftest/run.sh 200000`: 7,800,001 cases (200k per op, 39 ops), 58.8 s
  wall (263 s CPU), 184,687 Display mismatches, all judged by the oracle:
  0 `INTEL correct, DASHU WRONG`; exp/ln/log/pow and transcendental chains
  `dashu correct, intel wrong` (<=1 ulp: 122,747; 2-9 ulp: 1,668; >=10 ulp:
  10); scale ops and `new` at the overflow edge `dashu correct, intel wrong
  (Inf/PANIC vs finite)` (1,485); the Err-vs-panic rows of the `ilogb +
  scale` overflow (about 1.9% of each scale op, judged "both correct");
  31,062 `round_quantize` rows outside the quantum range (no oracle, Intel
  panics in `bid128_scalbn` of the quantum, the core reports NaN -> null) and
  the chain steps without a recorded operand. 6 cohort-only (Debug)
  differences (2 `exp`, 4 chains through `ln`), the known emulation limit.
  Bit-identical (0 mismatches): parse, `+ - * / %`, assign ops, neg, abs,
  trunc, frac, sqrt, square, pow with an `i32` exponent, FEEL modulo,
  `decimal`, comparisons, predicates, formatting, the ten integer
  conversions, `From<int>`, constants, and the 200k random exact-op chains.
- `tools/evaluator-corpus/run.sh`: 17,217 FEEL expressions. `corpus` (3313):
  1 mismatch, `now()`. `gen` (8636): 115 Display mismatches, 112 judged dashu
  correct and 3 `ceiling(0, -1)` variants (both null, reason text) + 2
  Debug-only cohorts. `extra` (1090): 44 mismatches: 20 exp/log/pow where
  dashu is correctly rounded (one of them, `(1+1e-33) ** (-1e34+1)`, needs
  300 mpmath digits: truth `...05508826375`, dashu `...088`, Intel `...089`),
  6 x `floor(999999999999999E+6116, 14)` & co. (Intel overflow quirk, null vs
  finite), 18 zero-with-negative-scale rows (both null, reason text). `dmn`
  (4178): 1 mismatch, `5 ** 2.55` (dashu correct). 0 panics on either side.
  Per corpus 0.02-0.5 s per backend.

The same tree, as `dsntk-wasm/third_party/dsntk`, also passes dsntk-wasm's
`cargo test --target wasm32-wasip1` under wasmtime for commits 1-4; the
dashu commits have not been run on wasm32 from this repository yet (the
dashu spike built feel-evaluator with `dashu` for `wasm32-wasip1` and
`wasm32-unknown-unknown` without a C toolchain).

## The `fastnum-backend` branch

`fastnum-backend` = commits 1-12 of the table above as they were on 2026-10-16 (before the history
rewrite, so with other hashes) + one commit adding
`feel-number/src/backend_fastnum.rs` behind feature `fastnum` (fastnum's
D256 division, the spike's exp/ln series, `pow` as `exp(y ln x)` at a fixed
77 digits, every result rounded once by the core from its digit string),
forwarded through the same manifests. It is experimental: not correctly
rounded in the strict sense (a 77-digit result within its own error of a
34-digit tie can round to the wrong neighbour), and the exact tie
`16 ^ -12.5` lands on whichever side the error falls on. Verified:
`cargo test -p rheon-feel-number --no-default-features --features fastnum`
228 passed, clippy clean, `cargo check --workspace --no-default-features
--features fastnum` succeeds. `dashu` stays the default.

## Taking upstream changes

Since the rename, upstream commits that touch the renamed crates no longer
apply as they are: their manifests and `use rheon_feel…` paths name the old
crates. Take upstream fixes one at a time, rewriting the patch through the
rename script before applying it:

```sh
git fetch upstream
git format-patch -1 --stdout <upstream sha> | perl scripts/upstream-rename.pl | git am -3
```

`upstream-rename.pl` rewrites `dsntk-<crate>` and `dsntk_<crate>` for every
renamed crate and leaves the others alone. A patch that touches `common`, `macros`,
`feel-grammar` or `recognizer` has no target here, because those crates are used from
crates.io: either wait for the next dsntk release and bump the `=0.3.0` pins in the root
manifest, or bring the crate back into the tree as `rheon-<crate>` and add it to the
rename script. Resolve the remaining conflicts by hand, run the
verification below, and record the cherry-pick in the table above. Whole
upstream releases are not merged any more.

## History rewrite of 2026-11-11

The 73 rheon commits from 2026-10-16 on were rewritten with `git filter-repo` to remove the
maintainer's home directory from four documentation commits and to give one commit the same
noreply author as all the others. Trees are unchanged; upstream's signed commits and the
`dsntk/*` tags are untouched. The commit hashes cited in the docs were updated in the same
push; crates.io still records the pre-rewrite hash of 0.2.0 (`3209ccd`, now `a91330d`).

## Publishing to crates.io

The ten `rheon-*` crates share the workspace version (`[workspace.package] version`,
rheon's own line; `0.1.0` was the first release, `0.2.0` added `rheon-feel-grammar`, `0.3.0` changed the API of
`rheon-feel` and `rheon-feel-evaluator`, `0.4.0` changes the API of `rheon-feel`, `rheon-feel-parser`
and `rheon-feel-evaluator`, `0.5.0` that of `rheon-feel`, `rheon-feel-temporal`, `rheon-feel-regex`,
`rheon-feel-evaluator`, `rheon-model` and `rheon-model-evaluator`, see below) and pin each other with `=<version>` in
the root manifest. Every other workspace member is `publish = false`, so one command
publishes the set in dependency order:

```sh
cargo login                          # once: a crates.io token with the publish-new and publish-update scopes
cargo publish --workspace --dry-run  # packages and verify-builds everything, uploads nothing
cargo publish --workspace
```

crates.io rate-limits the creation of new crate names (a burst of five, then one every ten
minutes; the 429 response names the time to retry). A release that introduces new crates
therefore stops part-way, and `--workspace` then refuses to continue because the first
crates "already exist". `scripts/publish-crates.sh` handles both: it asks crates.io which
crates still miss the workspace version, publishes only those, and sleeps until the named
retry time. The first release (0.1.0, 2026-11-09) took about 35 minutes this way. For a release: bump the workspace version and the `=` pins in the root manifest
together, run the verification above, commit, tag `vX.Y.Z`, publish.

`rheon-feel-wasm` is the tenth published crate: its build script takes the commit from
`.cargo_vcs_info.json` inside a packaged crate and the upstream commit from its own manifest.
The built module reaches users through the npm package `@rheon/feel-runtime` (which ships
`feel.wasm`) and the GitHub Release that `.github/workflows/release.yml` creates for every
`vX.Y.Z` tag (`feel.wasm`, its sha256, the npm tarball and the playground build).

The API changes of 0.4.0 (commits 39-54):

- Changed:
  - `rheon-feel`: `Value::FunctionDefinition` holds an `Arc<FunctionDefinition>` instead of six
    fields. `Name` implements `Borrow<str>`, so `map.contains_key(&"d".into())` on a map keyed
    by `Name` no longer infers its type (write `&Name::from("d")`). The `Debug` text of a
    function value is the derived `Debug` of `FunctionDefinition`. `Value::String` holds a
    `FeelString` (a shared `Arc<String>`) instead of a `String`: a pattern that binds the
    `String` by value no longer compiles (`into_string()` returns it, without a copy when the
    text is not shared); `value_string!` accepts anything that converts into `FeelString`.
  - `rheon-feel-parser`: `ParsingScope<'a>` has a lifetime parameter (invariant) and borrows
    the input contexts; `From<&FeelScope>` is removed, so outside the crate a scope is built
    with `Default`; `pop()` returns `()`; `flattened()` is removed. `ParsingScope` is no longer
    `UnwindSafe`: it holds references to the input contexts, and `FeelContext` is not
    `RefUnwindSafe`, so a closure that uses a scope needs `AssertUnwindSafe` for
    `catch_unwind`. It is `Send` and not `Sync`, as at 0.3.0.
  - `rheon-feel-evaluator`: `ForExpressionEvaluator::evaluate` returns `Value` (was `Values`).
    `FilterExpressionEvaluator` has a private field, so it is built with `new()`, `default()`
    or `for_feel()`, not as `FilterExpressionEvaluator {}`.
- Added: `FunctionDefinition` (with `new` and `From<FunctionDefinition> for Value`); `FeelString`
  (with `From<FeelString> for Value`);
  `FeelScope::{frames, depth, truncate, pop_frame}`; `FeelContext::{insert, assign_entry,
  get_entry_mut}`; `Value::{try_into_coerced, into_coerced}`; `Name::from_parts`;
  `bip::BUILT_IN_PROPERTY_NAMES`; `for_feel()` on `ForExpressionEvaluator`,
  `SomeExpressionEvaluator`, `EveryExpressionEvaluator` and `FilterExpressionEvaluator`.
- Kept: `FeelScope::contexts()`. The value types of `rheon-feel`, `AstNode`, `ClosureBuilder`,
  `FilterExpressionEvaluator`, `Dialect` and `ModelEvaluator` are `Send` and `Sync`, and
  `FeelScope` and `ParsingScope` are `Send`, as at 0.3.0; each crate's `send_sync` test checks
  this at compile time.

The API changes of 0.5.0 (commits 59-83; the item of each change is in parentheses):

- Changed:
  - `rheon-feel-temporal`: `FeelYearsAndMonthsDuration::from_ym` and `from_m` and
    `FeelDaysAndTimeDuration::from_n` and `from_sn` return `Option<Self>`, and
    `FeelDaysAndTimeDuration::from_s` takes an `i32` (59). `ops::Neg` of both durations is total,
    and `&FeelDate - &FeelDate` returns `Option<FeelYearsAndMonthsDuration>` (59).
    `FeelDaysAndTimeDuration::as_seconds` returns `i64` (was `isize`) (63).
    `FeelTime::{utc_opt, local_opt, offset_opt, zone_opt}` and `FeelDateTime::{utc, local, offset}`
    return `None` for nanoseconds of 10^9 or more and for 24:00:00 with a fraction, `utc_opt` turns
    24:00:00 into midnight, and the time and date and time parsers reject 24:00:00 with a
    fraction (75).
  - `rheon-feel`: `value_null!` caps its message at 1,000 characters and builds it in a closure
    that it passes to `values::null_with_reason`, so its arguments are evaluated inside that
    closure and can no longer use `?`, `return` or `break` (67). `Diagnostics` has a
    private `warnings` field, read with `warnings()`, so its struct literals no longer compile
    outside the crate (77).
  - `rheon-feel-evaluator`: `evaluate` returns `Outcome<Value>`; `evaluate_context`,
    `evaluate_context_node`, `evaluate_range_literal` and `evaluate_range_literal_node` return
    `Err` on an abort; the evaluators of `prepare` and `prepare_with_dialect` turn an abort into a
    null with its text; `evaluate_invocation` joins a running session, and outside one a stopped
    invocation is null with the abort's text (was `null(function call depth limit (200)
    exceeded)`, with the flag left set for the next caller on the thread) (65). The crate no
    longer depends on `regex` (72).
  - `rheon-feel-regex`: `is_match` and `is_match_with_flags` return `Result<bool, RegexError>`
    (73); `regex` and `regex-syntax` become dependencies (72).
  - `rheon-model`: `parse` rejects a document whose elements nest more than 100 levels deep (79);
    it no longer rejects item definition cycles, the `<ModelValidatorError>` kind is gone, and
    the `petgraph` dependency is removed (80).
  - `rheon-model-evaluator`: `ModelEvaluator::evaluate_invocable` returns `Outcome<Value>` and
    `evaluate_invocable_with_mode` returns `Outcome<(Value, Diagnostics)>`, and the evaluator of
    `build_decision_table_evaluator` runs as one evaluation, turning an abort into a null with its
    text (65). A decision-service function value called after its model was dropped is null with
    that reason (82). `ModelEvaluator::new` returns `<ItemDefinitionError>` for item definitions
    that form a cycle (80) and for a type over 100,000 nodes or 256 levels (81).
- Removed:
  - `rheon-feel-temporal`: the `ops::Add` and `ops::Sub` impls of both durations (use
    `checked_add` and `checked_sub`) and their `checked_neg` (59); `FeelDate::add_months(u32)` and
    `FeelDate::sub_months(u32)` (61); `FeelTime::{utc, local, offset}` (75).
  - `rheon-feel-evaluator`: `reset_call_depth` and `call_depth_exceeded` (65).
- Added:
  - `rheon-feel-temporal`: `Default` (zero) for both durations and `FeelDateTime::nanos_since`
    (59); `FeelTime::midnight(FeelZone)` (75); `FeelZone::from_designator` (76);
    `parse_reporting_truncation` on `FeelTime`, `FeelDateTime` and `FeelDaysAndTimeDuration` (77).
  - `rheon-feel`: `diagnostics::{Abort, evaluation, one_evaluation, abort, enter_call, Call}` and
    `From<Abort> for DsntkError`, with `Abort` and its variants `#[non_exhaustive]` and
    `Abort::limit` building the engine-limit variant (65); `message::{arg, Arg, compose,
    MessageArg, Size}`, `values::TypeOf` with its `MessageArg` impl, `values::null_with_reason`
    (the cold function behind `value_null!`), and `MessageArg` for
    `Value`, `FeelType`, `FeelNumber`, `Name`, `FeelContext`, `Bip`, `str`, `String` and
    `serde_json::Value` (67); `MessageArg::is_short` (with a default) and `MessageArg` for
    references and `FeelString` (68); `diagnostics::warn`,
    `values::truncated_fraction_warning` and `values::warn_truncated_fraction` (77).
  - `rheon-feel-evaluator`: `Session` and `Outcome` (65).
  - `rheon-feel-regex`: `RegexError`, `Limit`, `replace` and `split` (72).
- Unpublished members: `dsntk-evaluator` re-exports `Session` and `Outcome`, and its `evaluate`
  returns `Outcome`; `dsntk-workspace`'s `Workspaces::evaluate` returns `Err` on an abort (65).
- Kept: `rheon-feel-number`, `rheon-feel-grammar`, `rheon-feel-parser` and `rheon-feel-wasm` have
  no API change. The `Send` and `Sync` properties that each crate's `send_sync` test checks are
  those of 0.4.0. The wasm module's ABI and its request and response JSON are unchanged; what
  changes there are answers, registered with both answers as "Verification" states.

The API changes of commits 85-133 (the item of each change is in parentheses):

- Changed:
  - Every crate that forwarded a number backend: the Cargo features `intel` and `dashu` (and
    `default`) of `rheon-feel-number`, `rheon-feel`, `rheon-feel-temporal`, `rheon-feel-parser`,
    `rheon-feel-evaluator`, `rheon-model` and `rheon-model-evaluator` are gone, so a dependency
    written with `features = ["dashu"]` or `features = ["intel"]` no longer resolves;
    `default-features = false` is harmless (95).
  - `rheon-feel-number`: `FeelNumber`'s `Display` (and so `to_string()` and `Jsonify`) prints the
    canonical text and ignores a precision, and `FeelNumber` is 32 bytes (97). Behaviour: a zero
    has no sign (96); `Debug` of a NaN is `NaN` (98); `new(n, s)` with `s` beyond the exponent's
    range is the rounded value or infinite (102); `pow` with an integer
    exponent is correctly rounded, and `Debug` of an integer power shows the cohort of the exact
    power (104).
  - `rheon-feel-temporal`: `FeelZone::Offset` holds a `FeelOffset` (was `i32`), and its `Display`
    changes for negative offsets below one hour (106); `FeelTime::offset_opt` and
    `FeelDateTime::offset` accept offsets within ±14:00 (were -11:59..+12:00) (107);
    `FeelYearsAndMonthsDuration::years` and `months` return `u64` magnitudes (were signed `i64`)
    (112). Behaviour: `FeelTime`, `FeelDateTime` and `FeelZone::from_designator` reject an offset
    with minutes of 60 or more (108) or written with seconds (109); `&FeelDate - &FeelDate` gives
    new results for negative spans inside one year (113).
  - `rheon-feel`: the module form of `value_null!` takes the function name as any expression,
    not only as a literal (101); `diagnostics::record` and `values::null_with_reason` take a
    `Severity` first, and `Diagnostics::count` and `messages` leave out notes (117).
  - `rheon-feel-regex`: `is_match` takes the flags (`""` for none) and `replace` takes
    `flags: &str` instead of `Option<&str>`; `RegexError::Invalid` is renamed `InvalidPattern`
    (130). Behaviour: invalid flags (130), an invalid replacement string (131) and a pattern of
    `replace` or `split` that matches the zero-length string (132) are
    `RegexError::InvalidArgument`; `split` of `""` has no item (132).
  - `rheon-model-evaluator`, behaviour: `evaluate_invocable_with_mode` counts no error for a
    decision table without a matching rule (119) or a missing input variable of an
    item-definition type (120), and in strict mode returns their value; an input whose item
    definition references another with allowed values is checked against them (126), and an
    input typed by a collection with allowed values is checked per element (127).
- Removed:
  - `rheon-feel-number`: `FeelNumber::ceiling`, `floor`, `round_up`, `round_down`,
    `round_half_up`, `round_half_down` and `round` (use `round_to`) (101); `impl Rem` and
    `impl RemAssign` for `FeelNumber` (use `FeelNumber::modulo`) (103).
  - `rheon-feel-temporal`: `TryFrom<i32> for FeelZone` (`From<FeelOffset>` replaces it) (107).
  - `rheon-feel`: `Value::Transparent` (124).
  - `rheon-feel-evaluator`: `ForExpressionEvaluator`, `SomeExpressionEvaluator` and
    `EveryExpressionEvaluator` (128).
  - `rheon-feel-regex`: `is_match_with_flags` (130).
- Added:
  - `rheon-feel-number`: `FeelNumber::round_to` and `RoundingMode` (100, public with their callers
    in 101), `FeelNumber::modulo` (103).
  - `rheon-feel-temporal`: `FeelOffset` with `ZERO` and `Display` (106), `FeelOffset::from_duration`
    and `From<FeelOffset> for FeelZone` (107), `FeelOffset::write_seconds` (110);
    `FeelDaysAndTimeDuration::get_nanoseconds` and `FeelDateTime::nanos` (111).
  - `rheon-feel`: `diagnostics::Severity` and the `note:` forms of `value_null!` (117).
  - `rheon-feel-evaluator`: `Iteration` (`boxed`, `collect`, `some`, `every`), `Sequence` (`of`,
    `empty`, `interval`) and `Domain` (128).
  - `rheon-feel-regex`: `RegexError::InvalidArgument` (130).
- Kept: `rheon-feel-grammar` and `rheon-feel-wasm` have no API change, and `rheon-feel-parser`
  and `rheon-model` change only by the removed features (95). The wasm module's ABI and its
  request and response JSON are unchanged; what changes there are answers, registered with both
  answers as "Verification" states.

The API changes of commits 227-232 (the item of each change is in parentheses):

- Changed:
  - `rheon-feel`: `Name` holds its text shared, so a clone is no copy and `From<Name> for String`
    copies the text (229).
- Added:
  - `rheon-feel`: `FeelContext::from_ascending_entries` (229).
- The wasm ABI, its request and response JSON and every answer are unchanged.

The API changes of commits 222-226 (the item of each change is in parentheses):

- Changed:
  - `rheon-feel-parser`: behaviour: a name that is not in scope ends before a keyword that can
    follow an operand (222); a string literal or a block comment that is not closed is a syntax
    error (223).
- Added:
  - `rheon-feel`: `impl FromIterator<(Name, Value)> for FeelContext` (225).
- The wasm ABI and its request and response JSON are unchanged; the answers that change (222,
  223) are registered with both answers as "Verification" states.

The API changes of commits 179-221 (the item of each change is in parentheses):

- Changed:
  - `rheon-feel-evaluator`: behaviour: a tree built by hand of a shape that the parser never
    produces evaluates to null with the error `malformed expression tree: unexpected <Kind> node`
    (217); the numbers of an interval of integers are counted as integers, with the same items (210).
- Removed:
  - `rheon-feel`: `diagnostics::tentative` (181).
- Added:
  - `rheon-feel`: `diagnostics::tentative_mark` (181); the trait `FeelCode` and the variant
    `FunctionBody::Feel`, which holds the code of a body and the index of the body in it (206);
    `diagnostics::evaluation_within`, `diagnostics::step` and `Abort::Steps` (221).
  - `rheon-feel-number`: `FeelNumber::unscaled_i64` (210).
  - `rheon-feel-evaluator`: `Session::with_step_budget` (221).
- Added and removed within the series: the feature `backends` and the doc-hidden module
  `rheon_feel_evaluator::testing` (183, 218).
- wasm request, additive: `maxSteps` in evaluate requests, and `EvaluateRequest.maxSteps` in
  `@rheon/feel-runtime` (221). The wasm ABI and every response are unchanged.

The API changes of commits 135-177 (the item of each change is in parentheses):

- Changed:
  - `rheon-feel`: `FeelContext::iter` returns `impl ExactSizeIterator<Item = (&Name, &Value)>`
    and `contains_entry` is generic over `Q` with `Name: Borrow<Q>` (147);
    `FeelContext::assign_entry` returns `Option<Value>` (149); `Value::List` holds `FeelList` instead of `Vec<Value>`, and
    `values_to_string`, `values_to_feel_string` and `values_to_jsonify` take `&[Value]` (150);
    `FeelList`'s `IntoIterator` by value yields `ListItems` (154); the fields
    `FunctionDefinition::parameters` and `result_type` are private (158); `Value::Context` holds
    `Arc<FeelContext>` (159); `FeelScope::frames` returns `Ref<[ScopeFrame]>` and
    `FeelScope::pop_frame` `Option<ScopeFrame>` (160); `FunctionDefinition::closure_ctx` is an
    `Arc<FeelContext>` (161). Behaviour: a value nested more than 500 levels deep stops the
    evaluation with `Abort::ValueDepth`, and `FeelContext::apply_entries` returns an error for a
    path that would nest it deeper (152).
  - `rheon-feel-evaluator`: `FilterExpressionEvaluator` is a unit struct that only `new` and
    `Default` build (166). Behaviour: an abort stops every loop (146); `and` and `or` evaluate the
    right operand only when the left does not decide (169); a filter evaluates its expression once
    in the enclosing scope first (171); the function that `sort` and `list replace` call is a
    counted call in one scope (172, 173).
- Removed:
  - `rheon-feel`: `impl Deref for FeelContext` (147); `Value::NamedParameter` and
    `Value::PositionalParameters` (148); `FeelContext::get_entry_mut` (149).
  - `rheon-feel-evaluator`: `FilterExpressionEvaluator::for_feel` (166).
- Added:
  - `rheon-feel`: `diagnostics::stopped` (146); `FeelContext::keys_from` (147); `FeelList` (150);
    `Abort::ValueDepth`, `diagnostics::bound_depth` and `bound_list_depth` (152);
    `values::ListItems` (154); `FeelList::address` and `FeelList::is_shared` (155);
    `Value::has_type`, `Value::has_type_of` and `Value::type_instance_of` (157);
    `FunctionDefinition::parameters()` and `result_type()` (158); `MessageArg` for `Arc<T>` (159);
    `ScopeFrame` and `FeelScope::push_shared` (160); `Value::is_shared` and `Value::node_address`
    (162); `FeelList::items_conform_to_first` (163); `diagnostics::tentative` and
    `diagnostics::Tentative` (171).
- Kept: the other crates have no API change. `Value`, `FeelList`, `ListItems` and `ScopeFrame`
  are `Send + Sync`. The wasm module's ABI and its request and response JSON are unchanged; what
  changes there are answers, registered with both answers as "Verification" states.

### The pre-rename procedure (historical)

Until 2026-10-28 the crates kept upstream's names and the plan was to merge or
rebase onto upstream releases; the procedure is kept for reference.


```sh
cd ~/dev/rheon
git fetch origin
git checkout main && git merge --ff-only origin/main     # keep main a mirror
git checkout wasm
git rebase main                                         # replay the commits
git rebase wasm fastnum-backend                         # optional side branch
```

`upstream` is <https://github.com/DecisionToolkit/dsntk> (the crates.io name is
`dsntk`; the old <https://github.com/dsntk/dsntk-rs> URL redirects there).
Upstream has not moved since `c76aee0`, so today the rebase is a no-op.
Replaying the branch onto a moved base (tried against the head of upstream
PR DecisionToolkit/dsntk#112, 27 commits ahead) first conflicts in commit 3
(`ec2ced3`, Robustness) in `feel-evaluator/src/filters.rs` and
`feel-parser/src/parser.rs` (one hunk each); commit 4 overlaps PR #112
itself, and commit 8 touches every manifest. Resolve conflicts per commit
(each one is small and self-contained) and compile every replayed commit:

```sh
git rebase -x 'CARGO_BUILD_JOBS=4 cargo check -p rheon-feel-evaluator' upstream/main
```

Then:

1. Rerun the verification above (the suites, the identity check
   `tools/identity/run.sh` and the number oracle `tools/number-oracle/run.sh full`).
2. Rebuild and re-verify the wasm side in this repository: `npm test`,
   `npm run build:wasm`, `npm run smoke`, `npm run build` (the module's
   `engineCommit` follows HEAD automatically), and update `upstream-commit`
   in `wasm/feel-wasm/Cargo.toml` (`[package.metadata.rheon]`).

Since 2026-10-28 the engine commits live on `main` of this repository together
with the wasm, runtime and playground code, and the seven changed crates carry
`rheon-*` names, so this rebase procedure no longer applies as written; the
cherry-pick procedure above replaces it.

If a commit was merged upstream, `git rebase` drops it automatically when
the patch is identical; otherwise drop it by hand with `git rebase -i`.

## Layout note

This is the full upstream workspace (CLI, server, model evaluator, ...) so
that `cargo test` of any crate works unchanged. dsntk-wasm only needs
`common`, `macros`, `feel`, `feel-number`, `feel-temporal`, `feel-regex`,
`feel-grammar`, `feel-parser` and `feel-evaluator`; with the default
features none of them needs a C toolchain any more.
