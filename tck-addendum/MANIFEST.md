# tck-addendum manifest

| folder | tests | DMN | issues | spec | status | superseded by |
|---|---|---|---|---|---|---|
| rheon-1601-round-single-argument | 26 | 1.6 | DMN16-93 | §10.3.4.5 Table 77 | passing | |
| rheon-1602-number-single-argument | 20 | 1.6 | DMN16-208, DMN16-105 | §10.3.4.1 Table 73, rule 35 | passing | |
| rheon-1603-temporal-value-property | 47 | 1.6 | DMN16-11 | §10.3.2.3.4–8, Tables 66–67 | passing | |
| rheon-1604-singleton-list-iteration | 30 | 1.6 | DMN16-94 | §10.3.2.9.4, boxed iterators | see baseline | |
| rheon-1605-descendant-operator | 19 | 1.6 | DMN16-72 | rule 68 | not implemented | |
| rheon-1606-b-feel-operators | 28 | 1.6 | DMN16-91, DMN16-71 | §11.2 | not implemented | |
| rheon-1607-b-feel-functions | 67 | 1.6 | DMN16-91 | §11.3–§11.8 | not implemented | |
| rheon-1608-b-feel-arithmetic | 31 | 1.6 | DMN16-91 | §11.9–§11.10 | not implemented | |
| rheon-1701-trim-function | 16 | 1.7 b1 | DMN17-96 | §10.3.4.3, rule 61 | passing | |
| rheon-1702-intersection-function | 17 | 1.7 b1 | DMN17-98 | §10.3.4.4 | passing | |
| rheon-1703-insert-after-function | 20 | 1.7 b1 | DMN17-100 | §10.3.4.4 | passing | |

## Open questions (left untested on purpose)

The spec does not settle these; rheon's current choice is noted.

- **value of a time with a non-zero offset**: local or UTC midnight. rheon uses the UTC form, which
  keeps `value` consistent with time subtraction across offsets.
- **value of a local date and time** (no offset or zone): `valuedt` is defined only for values
  with a UTC form. rheon returns null.
- **Epoch** for `valuedt`: not fixed in §10.3.2.3.6; tests use 1970-01-01T00:00:00Z, the epoch
  §11.5 names.
- **Singleton conversion for textual `for`/`some`/`every`**: 1.6 states it only for the boxed
  iterators. rheon already converts in both forms.
- **Descendant operator**: order of matches across branches, lists of contexts, contexts inside
  lists.
- **number() footnote 1** cites "grammar rule 37" (`digits`), evidently meaning rule 35
  (numeric literal); rheon validates against rule 35.
- **intersection** with one list, or with non-list arguments.
