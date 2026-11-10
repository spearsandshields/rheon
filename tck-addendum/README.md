# tck-addendum

rheon's own tests for DMN 1.6 features (and a few additive DMN 1.7 beta functions) that the
official [DMN TCK](https://github.com/dmn-tck/tck) does not cover yet. They are byte-compatible
with the TCK: same directory layout, the TCK's `testCases.xsd` format, DMN 1.6 models.

- **Naming.** Folders are `rheon-NNNN-<feature>` (16xx: DMN 1.6, 17xx: DMN 1.7 beta) so they never
  collide with official folder numbers. Every file starts with a comment naming the OMG issue
  keys it covers.
- **Expected values** are derived from the formal DMN 1.6 text (or the 1.7 beta 1 text), cited in
  each test's `<description>`. Each folder was written and then adversarially re-checked against
  the spec; cases the spec does not settle were left out (see MANIFEST.md, "open questions").
- **Running.** `scripts/tck.sh` runs the official TCK and this tree through one server;
  `scripts/tck.sh --addendum-only` runs only these. Results are summarised against
  `docs/tck-baseline.txt`.
- **Dropping.** When the official TCK adds coverage for an issue, copy nothing: record the official
  folder in MANIFEST.md's "superseded by" column, delete ours, and rerun. If the official
  expectations differ from ours, the official ones win and the difference is a rheon bug.
