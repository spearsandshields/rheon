<!-- Produced 2026-10-20 by a 12-agent review (3 readers, 4 designs, 3 judges, synthesis, adversarial critic). Measurements and file references were verified against the fork at f70ac79 and the installed packages. -->

# Editor language support — decision and plan (amended)

## 1. Decision

dsntk becomes the only source of syntax validity: the squiggle, lint gutter and "Jump to error" come from a new error span on the fork, never from lezer error nodes. lang-feel/lezer-feel stay for colour, indent, completion and the positions tree, behind one adapter module and a differential corpus that fails when a known divergence stops reproducing. Phase 1 is the fork lexer work (keyword swallowing, `in` panic, token span); every option needs it. Replacing lezer-feel with a hand-written StreamLanguage is a gated follow-up (§5), not the default.

## 2. Why

- **One definition of valid, at the cheapest point.** Today lezer places the squiggle and dsntk decides. With the squiggle built from `op parse` (same context as evaluate) disagreement on validity is impossible; nothing else in the editor has to change. `@codemirror/lint` already remaps diagnostics through edits and drops stale async results (dist/index.js:152, :317-320, verified).
- **The independence constraint is already met.** The recorded constraint (2026-10-18) is about nikku's unlicensed feel-playground app; it explicitly keeps lezer-feel/lang-feel (MIT) with notices. Removing them is a preference, not a requirement.
- **Measured quality favours lezer for the editor.** On 82 valid rows lezer-feel is wrong on 3 (two upstream grammar gaps, one apostrophe heuristic); dsntk's tree changes shape with the context and is unusable for colouring. A hand tokenizer would be a third FEEL lexer that still disagrees with dsntk on name boundaries (`x + y`) and keyword whitespace rules, loses scope-aware colouring of in-expression names, the partial tree and contextual completion, for 26.6 kB (5.1 %) of bundle.
- **Smallest engine surface**: tier-1 span is ~120 lines in hand-written files (lexer.rs:197 `token_start`, parser.rs:216/220/113, new `ParseError`, old `parse_*` delegate), no Bison 3, no AstNode change, same edit site as the two owed fixes.
- README.md:201 and vite.config.ts:30 are wrong (lang-feel is 0.6 %); fix the text, not the dependency.

## 3. Plan

**Phase 1 — fork lexer (1.5 days; builds: `cargo test -p feel-parser`, dsntk CLI `--features tck` release, serialized behind the main session).** `~/dev/rheon` feel-parser:
1. `consume_name` (lexer.rs:745-769): cut unknown names at the 23 reserved words (incl. `instance`), exempt the 8 multi-word built-ins (`date and time`, `years and months duration`, `days and time duration`, `day of week/year`, `month of year`, `week of year`, `index of`). **Guard the cut**: skip it under `till_in`, when a `:` follows (existing checks) and under a new lexer-local `formal_parameters` flag set when the `function` keyword is returned and cleared at its matching `)` (verified today: `(function(net income for year) net income for year)(5)` = 5 and must stay so). Index-0 guard at :713. Tests: `for l in zz return l`, `function(p) for i in current(p).incomeItems return i.amount`, `(if c then i.amount else 0)`, `x instance of range<number>` with x unknown, `for in.x in [1] return x`, `date and time of birth`, `function(date of birth) date of birth`, `{ time of day: 1 }.time of day`. Re-run the TCK (baseline 3480/3511).
2. `token_start` after `read_input()` (lexer.rs:197), `token_span()`; `ParseError { message, span, found }` (char indices); parser.rs:216/220/113 fill it; `parse_expression_located` / `parse_unary_tests_located` in lib.rs. Tests: `1 + + 2` → (4,5) `"+"`; `{ a: 1, b: ` → EOF; astral char before the error.
3. Token recorder behind a cargo feature, exposed by the CLI; `scripts/gen-token-fixtures.sh` writes `apps/playground/test/fixtures/tokens.json` with the fork commit inside, so a fork bump invalidates it.

**Phase 2 — contract + runtime (0.5 day; feel-wasm native + wasm32 test builds).** `wasm/feel-wasm/src/engine.rs`: every `kind:"syntax"` error on `evaluate` and `parse` gets `span` (scalar indices), `utf16Span` and `found` **when the parser produced them**; depth-limit and runtime surrogate errors keep no span and CONTRACT.md §3 says so. `packages/feel-runtime` types (`FeelError.span?`) + tests. `scripts/gen-feel-names.mjs` reads bif.rs/bip.rs/lexer.rs → checked-in `feel-names.json` (functions, properties, keywords, type names, exemptions) for completion and tests.

**Phase 3 — playground (1 day).**
- New `apps/playground/src/lib/feel-language.ts` is the only importer of lang-feel/lezer-feel/@lezer/* and of lint/completion code (grep guard in tests). It exports `feelLanguage({dialect, contextNames})`, `feelLinter(getEngine)` and `syntaxInfo(state)`.
- `feelLinter`: async `linter()` source, delay 250, with its own AbortController (abort the previous parse on each run); calls `engine.parse(dialect, doc, {context})`, and when the context JSON is invalid parses **without** a context instead of giving up. Emits one diagnostic from `utf16Span` (EOF widened onto the previous token with the existing code), message `unexpected "<found>"`; returns [] while the engine is loading or when the error carries no span. `needsRefresh` re-runs it when dialect or context change. The lezer `syntaxLinter` is deleted; lezer error nodes feed only the tree view.
- Completion: keep lang-feel's snippets/keywords; add name + built-in completion from `feel-names.json` and contextNames via `languageData.autocomplete` (gated with `ifNotIn`). Fix the contextNames comment (CodeEditor.svelte:44).
- Tree panel: keep, relabel "lezer-feel tree (positions; not the engine's parse)"; "Jump to error" now uses the engine span (OutputPanel.svelte:14-15, :139-141; App.svelte:77, :152-154).
- `apps/playground/test/differential.test.mjs` (node:test, like feel-runtime): the 101-row corpus + the fork's 304 `accept()` inputs; asserts engine span position, the lezer-vs-dsntk allowlist (#97, #80, apostrophe) reproduce-or-fail, and the 9 Name-collapse rows as documented engine behaviour.

**Phase 4 — text and backlog (0.5 day).** README/vite comments; notices unchanged; BACKLOG: close tier 1 + two fixes, replace the 2026-10-20 note with this decision and the gate below.

**Total: ~3.5 days**, fork steps serialised behind the main session's build; Phase 3 can start against a stub span immediately.

## 4. What users lose vs today, and mitigation

| Loss | Mitigation |
|---|---|
| Squiggle async (~250 ms + worker hop), one error at a time; waits behind a slow evaluate (≤ 2 s + restart) | Lint timer fires before the evaluate debounce, so parse is usually queued first; previous diagnostics stay remapped |
| No squiggle before the wasm loads | Context/engine status already shown |
| `x + y` with unknown names shows no error (engine truth: one Name) | "Undefined-name hint" via `found`/nullReason moves up the backlog |
| lezer's two grammar gaps still colour/tree wrongly (`range<T>`, nested date and time) | Allowlisted in the corpus; send upstream PRs (small grammar edits) |

## 5. Gated option: StreamLanguage replacement (4-5 days, decided separately)

Trigger only for a stated reason: a dependency policy that excludes lezer-feel, an upstream regression the corpus catches, or a second host needing an engine-driven highlighter. Prerequisites now satisfied by this plan: adapter module, corpus, generated name tables, engine span. Scope if triggered: StreamLanguage with scope tracking for in-expression names (keys, for-variables, parameters), a sync lexical `linter()` pass (StreamLanguage emits no diagnostics), completion rebuilt with `snippetCompletion`, token names aligned with `ifNotIn`, Tokens debug list replacing the positions tree, removal of lang-feel/lezer-feel/@lezer/lr/@lezer/common (keep @lezer/highlight: imported directly).

## 6. Rejected

- **Engine tokenize op**: re-derives parser flags from previous tokens, honest mode collapses `a and b`; revisit only with a second host.
- **Own lezer grammar from feel.y**: 23 mid-rule lexer-steering actions have no lezer form; it would reproduce dsntk quirks, not editor quality.
- **Replacing lezer now**: see §1-2; the gain is 5 % bundle and one fewer MIT notice, the cost is features and a third lexer.

## 7. Reopen if

- The owner states a dependency reason to drop lezer-feel → §5.
- Users demand a positioned tree from the engine → LALR span tree (2-4 days).
- The fork adopts operator-cutting for unknown names (changes evaluation semantics; not proposed).

## 8. Open questions for the owner

1. Confirm the independence constraint is the app, not the MIT libraries (memory says so); if not, §5 becomes Phase 5.
2. Ship `span` + `utf16Span` (recommended) or `span` only?
3. node:test beside feel-runtime (recommended) or vitest for the playground?