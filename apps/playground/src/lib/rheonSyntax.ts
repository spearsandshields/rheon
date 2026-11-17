/**
 * Prototype: highlighting, error marks and the positions tree from the engine's own parser
 * (`op: "syntax"`), switched on with `?parser=rheon`. The operation exists only in a module
 * built with the feature `hand-parser`; with any other module the editor shows plain text.
 *
 * The engine runs in this thread here: one call per edit, a few microseconds for a formula.
 */
import { instantiateFeel, type Dialect, type FeelInstance } from '@rheon/feel-runtime';
import { StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, type DecorationSet } from '@codemirror/view';
import feelWasmUrl from '../../../../dist/feel.wasm?url';

/** Whether the page was opened with `?parser=rheon`. */
export const RHEON_PARSER = typeof location !== 'undefined' && new URLSearchParams(location.search).get('parser') === 'rheon';

/** The answer of `op: "syntax"`: places are offsets in UTF-16 code units. */
export interface RheonSyntax {
  tokens: [string, number, number][];
  nodes: [string, number, number][];
  diagnostics: { code: string; message: string; from: number; to: number }[];
  elapsedMicros: number;
}

let instance: FeelInstance | null = null;
let loading: Promise<void> | null = null;
const config: { dialect: Dialect; names: string[] } = { dialect: 'expression', names: [] };

/** Loads the engine for the syntax calls (once). */
export function loadRheonSyntax(): Promise<void> {
  loading ??= fetch(feelWasmUrl)
    .then((response) => response.arrayBuffer())
    .then((bytes) => instantiateFeel(bytes))
    .then((created) => {
      instance = created;
    });
  return loading;
}

/** Sets the dialect and the names in scope of the next syntax calls. */
export function configureRheonSyntax(dialect: Dialect, names: string[]): void {
  config.dialect = dialect;
  config.names = names;
}

function syntaxOf(expression: string): RheonSyntax | null {
  if (!instance) return null;
  const context = Object.fromEntries(config.names.map((name) => [name, null]));
  const response = instance.call({ op: 'syntax', expression, dialect: config.dialect, context });
  return response && response.ok ? (response as RheonSyntax) : null;
}

/** Asks the editor to read the syntax again (the engine is loaded, or the scope changed). */
export const refreshRheonSyntax = StateEffect.define<null>();

interface Held {
  syntax: RheonSyntax | null;
  marks: DecorationSet;
}

function hold(text: string): Held {
  const syntax = syntaxOf(text);
  const ranges = [];
  for (const [kind, from, to] of syntax?.tokens ?? []) {
    if (kind !== 'whitespace' && from < to && to <= text.length) ranges.push(Decoration.mark({ class: `rs-${kind}` }).range(from, to));
  }
  return { syntax, marks: Decoration.set(ranges, true) };
}

const field = StateField.define<Held>({
  create: (state) => hold(state.doc.toString()),
  update(held, tr) {
    if (tr.docChanged || tr.effects.some((effect) => effect.is(refreshRheonSyntax))) return hold(tr.newDoc.toString());
    return held;
  },
  provide: (f) => EditorView.decorations.from(f, (held) => held.marks),
});

const colors = EditorView.baseTheme({
  '.rs-keyword': { color: 'var(--syn-keyword)', fontWeight: '600' },
  '.rs-number': { color: 'var(--syn-number)' },
  '.rs-string': { color: 'var(--syn-string)' },
  '.rs-constant': { color: 'var(--syn-atom)' },
  '.rs-function, .rs-type': { color: 'var(--syn-fn)' },
  '.rs-property, .rs-key, .rs-parameter': { color: 'var(--syn-prop)' },
  '.rs-name, .rs-variable': { color: 'var(--syn-var)' },
  '.rs-comment': { color: 'var(--syn-comment)', fontStyle: 'italic' },
  '.rs-operator': { color: 'var(--syn-op)' },
  '.rs-punctuation': { color: 'var(--syn-punct)' },
  '.rs-error': { color: 'var(--danger)' },
});

/** The editor extension: the tokens of the engine as colours. */
export const rheonSyntaxExtension: Extension = [field, colors];

/** The syntax the editor holds for its text, or null before the engine is loaded. */
export function rheonSyntaxOf(state: EditorState): RheonSyntax | null {
  return state.field(field, false)?.syntax ?? null;
}
