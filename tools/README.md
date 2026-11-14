# Tools

- `identity/`: the identity check of full responses of `feel.wasm` against the previous release,
  with the ledger of registered differences (see `identity/README.md`). CI runs it after every
  wasm build.
- `number-oracle/`: every number result of `feel.wasm` judged against its truth under DMN 1.6,
  computed in Python with `decimal`, exact rationals and mpmath (see `number-oracle/README.md`).
  CI runs its quick size.
- `feel-corpus/`: FEEL expressions as JSON lines `{"src", "ctx", "expr"}`, frozen: `corpus.jsonl`
  (3,313, harvested from the feel-evaluator unit tests and `bbt/`), `dmn.jsonl` (4,178, the
  literal expressions of the DMN models under `examples/` and `bbt/`), `gen.jsonl` (8,636,
  generated numeric edge cases) and `extra.jsonl` (1,090, numeric expressions around `exp`,
  `log`, `sqrt`, `**` and `/`). The parser's corpus test reads them:

  ```sh
  RHEON_PARSER_CORPUS="$PWD/tools/feel-corpus" cargo test --release -p rheon-feel-parser corpus_equals_reference -- --include-ignored
  ```
