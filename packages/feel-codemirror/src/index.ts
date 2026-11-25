/**
 * CodeMirror 6 support for FEEL from the rheon engine's own parser: the colours, the error
 * marks, the completions, the hint of the invoked function and what shows over a name are what
 * the engine answers for the text (`syntax`, `complete`, `hover`, docs/CONTRACT.md §3), so the editor reads a text as the
 * engine evaluates it.
 *
 *   const engine = await FeelEngine.create({ wasmUrl });
 *   new EditorView({ parent, extensions: [basicSetup, feelEditor({ engine, context })] });
 *
 * The engine is asked after every change of the text. An answer for a text that has changed
 * since is dropped; until the next answer arrives the colours move with the edits.
 */
import { autocompletion, snippetCompletion, type Completion, type CompletionContext, type CompletionResult } from '@codemirror/autocomplete';
import { linter, type Diagnostic } from '@codemirror/lint';
import { keymap } from '@codemirror/view';
import { Compartment, Facet, StateEffect, StateField, type EditorState, type Extension } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, hoverTooltip, showTooltip, type DecorationSet, type Tooltip, type ViewUpdate } from '@codemirror/view';
import type { CompleteResponse, CompletionCall, CompletionItem, EditorOptions, FormatResponse, FunctionSignature, Hover, HoverResponse, SyntaxResponse } from '@rheon/feel-runtime';

/** What answers the editor: a `FeelEngine` (a worker), or anything with these methods; the answers may be promises or values. */
export interface FeelSource {
  syntax(expression: string, opts: EditorOptions & { tree?: boolean }): Promise<SyntaxResponse> | SyntaxResponse;
  complete(expression: string, cursor: number, opts: EditorOptions): Promise<CompleteResponse> | CompleteResponse;
  /** Needed only for what shows over a name (`hover`). */
  hover?(expression: string, at: number, opts: EditorOptions): Promise<HoverResponse> | HoverResponse;
  /** Needed only for `formatDocument`. */
  format?(expression: string, opts: EditorOptions & { width?: number }): Promise<FormatResponse> | FormatResponse;
}

/** The names in scope and the dialect of the text: see `EditorOptions` of `@rheon/feel-runtime`. */
export type FeelScope = EditorOptions;

export interface FeelEditorConfig extends FeelScope {
  engine: FeelSource;
  /** The most syntax errors that are marked (default 20; 0 marks none). The engine reports up to 100. */
  maxDiagnostics?: number;
  /** Whether the completion offers templates of `if`, `for`, `some`, `every`, a function and a context (default true). */
  templates?: boolean;
  /** Whether the answers hold the nodes of the text (`syntaxOf(state).nodes`; default false). */
  tree?: boolean;
  /** Called with every answer that the editor takes. */
  onSyntax?: (syntax: FeelSyntax) => void;
  /** The most characters of a line that `formatDocument` aims at (default 80). */
  formatWidth?: number;
  /** Whether a hint shows the signatures of the built-in function whose parentheses the cursor is in (default true). */
  signatureHelp?: boolean;
  /** Whether the type of a name, and what the outline or the engine says of it, shows when the pointer rests on it (default true; needs an engine that answers `hover`). */
  hover?: boolean;
}

/** A `syntax` answer that succeeded. */
export type FeelSyntax = Extract<SyntaxResponse, { ok: true }>;

/** An in-thread engine (`instantiateFeel`) as a source: its `call` takes the requests as objects. */
export function instanceSource(instance: { call(request: object): unknown }): FeelSource {
  return {
    syntax: (expression, opts) => instance.call({ op: 'syntax', expression, ...opts }) as SyntaxResponse,
    complete: (expression, cursor, opts) => instance.call({ op: 'complete', expression, cursor, ...opts }) as CompleteResponse,
    hover: (expression, at, opts) => instance.call({ op: 'hover', expression, at, ...opts }) as HoverResponse,
    format: (expression, opts) => instance.call({ op: 'format', expression, ...opts }) as FormatResponse,
  };
}

const setSyntax = StateEffect.define<FeelSyntax | null>();
const scopeOf = Facet.define<FeelScope, FeelScope>({ combine: (values) => values[0] ?? {} });
const formatterOf = Facet.define<{ engine: FeelSource; width?: number }, { engine: FeelSource; width?: number } | null>({ combine: (values) => values[0] ?? null });

interface Held {
  syntax: FeelSyntax | null;
  marks: DecorationSet;
}

function marksOf(syntax: FeelSyntax | null, length: number): DecorationSet {
  const ranges = [];
  for (const [kind, from, to] of syntax?.tokens ?? []) {
    if (kind !== 'whitespace' && from < to && to <= length) ranges.push(Decoration.mark({ class: `cm-feel-${kind}` }).range(from, to));
  }
  return Decoration.set(ranges, true);
}

const held = StateField.define<Held>({
  create: () => ({ syntax: null, marks: Decoration.none }),
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setSyntax)) return { syntax: effect.value, marks: marksOf(effect.value, tr.newDoc.length) };
    }
    // until the engine answers for the new text, the colours move with the edits
    return tr.docChanged ? { syntax: value.syntax, marks: value.marks.map(tr.changes) } : value;
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.marks),
});

/** The last answer of the engine that the editor took, or null before the first. After an edit it is the answer for the text before the edit until the next one arrives. */
export function syntaxOf(state: EditorState): FeelSyntax | null {
  return state.field(held, false)?.syntax ?? null;
}

/** Colours through CSS custom properties (`--feel-keyword` and so on), with defaults for a light and a dark editor. */
const colours = EditorView.baseTheme({
  '.cm-feel-keyword': { color: 'var(--feel-keyword, #8a2be2)', fontWeight: '600' },
  '.cm-feel-number': { color: 'var(--feel-number, #1d4ed8)' },
  '.cm-feel-string': { color: 'var(--feel-string, #17803d)' },
  '.cm-feel-constant': { color: 'var(--feel-constant, #b45309)' },
  '.cm-feel-function, .cm-feel-type': { color: 'var(--feel-function, #0e7490)' },
  '.cm-feel-property, .cm-feel-key, .cm-feel-parameter': { color: 'var(--feel-property, #a3366b)' },
  '.cm-feel-name, .cm-feel-variable': { color: 'var(--feel-name, inherit)' },
  '.cm-feel-comment': { color: 'var(--feel-comment, #746c5f)', fontStyle: 'italic' },
  '.cm-feel-operator': { color: 'var(--feel-operator, #6a645a)' },
  '.cm-feel-punctuation': { color: 'var(--feel-punctuation, #8a8377)' },
  '.cm-feel-error': { color: 'var(--feel-error, #c0392b)' },
  '.cm-feel-signature': { padding: '4px 8px', fontFamily: 'monospace', fontSize: '0.9em', maxWidth: '40em' },
  '.cm-feel-signature-active': { textDecoration: 'underline' },
  '.cm-feel-signature-given': { opacity: '0.6' },
  '.cm-feel-signature-summary': { fontFamily: 'sans-serif', opacity: '0.8', marginTop: '2px' },
  '.cm-feel-hover': { padding: '4px 8px', fontFamily: 'monospace', fontSize: '0.9em', maxWidth: '40em' },
  '.cm-feel-hover-documentation, .cm-feel-hover-deprecated': { fontFamily: 'sans-serif', opacity: '0.8', marginTop: '2px' },
  '.cm-feel-hover-deprecated': { fontStyle: 'italic' },
  '&dark .cm-feel-keyword': { color: 'var(--feel-keyword, #c792ea)' },
  '&dark .cm-feel-number': { color: 'var(--feel-number, #8fb0ff)' },
  '&dark .cm-feel-string': { color: 'var(--feel-string, #a5e07b)' },
  '&dark .cm-feel-constant': { color: 'var(--feel-constant, #f0b45a)' },
  '&dark .cm-feel-function, &dark .cm-feel-type': { color: 'var(--feel-function, #5cd0e6)' },
  '&dark .cm-feel-property, &dark .cm-feel-key, &dark .cm-feel-parameter': { color: 'var(--feel-property, #ff8fbf)' },
  '&dark .cm-feel-comment': { color: 'var(--feel-comment, #8f8b83)' },
  '&dark .cm-feel-operator': { color: 'var(--feel-operator, #9b978f)' },
  '&dark .cm-feel-punctuation': { color: 'var(--feel-punctuation, #7d7a73)' },
});

/** The templates: each is offered where an expression can start and what is typed starts its first word. */
const TEMPLATES: [string, string][] = [
  ['if … then … else …', 'if ${condition} then ${value} else ${other}'],
  ['for … in … return …', 'for ${item} in ${list} return ${value}'],
  ['some … in … satisfies …', 'some ${item} in ${list} satisfies ${condition}'],
  ['every … in … satisfies …', 'every ${item} in ${list} satisfies ${condition}'],
  ['function(…) …', 'function(${parameters}) ${body}'],
  ['{ key: value }', '{ ${key}: ${value} }'],
];

function completionOf(item: CompletionItem): Completion {
  const types: Record<CompletionItem['kind'], string> = { value: 'enum', parameter: 'property', name: 'variable', entry: 'property', keyword: 'keyword', function: 'function', type: 'type' };
  return {
    label: item.label,
    type: types[item.kind],
    detail: item.detail ?? item.type ?? undefined,
    // a deprecated name says so, with its reason, and comes after the others
    info: item.deprecated === undefined ? item.documentation : [item.documentation, item.deprecated === true ? 'Deprecated.' : `Deprecated: ${item.deprecated}`].filter(Boolean).join(' '),
    boost: item.deprecated === undefined ? undefined : -50,
    // an item replaces the text from its own start: the items of one answer can start at different places
    apply: (view, _completion, _from, to) => {
      const insert = item.kind === 'function' ? `${item.label}()` : (item.insertText ?? item.label);
      const cursor = item.from + insert.length - (item.kind === 'function' ? 1 : 0);
      view.dispatch({ changes: { from: item.from, to, insert }, selection: { anchor: cursor }, userEvent: 'input.complete' });
    },
  };
}

const setCall = StateEffect.define<Tooltip | null>();

/** The hint of the invoked function, as a tooltip at the cursor. */
const hint = StateField.define<Tooltip | null>({
  create: () => null,
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setCall)) return effect.value;
    }
    // the hint of a text or a cursor that has changed goes; the next answer brings it back
    return tr.docChanged || tr.selection ? null : value;
  },
  provide: (field) => showTooltip.from(field),
});

/** The index of the parameter of the signature that the argument of the call is (the engine says it), or -1. */
function activeParameter(signature: FunctionSignature): number {
  return signature.activeParameter ?? -1;
}

/** The hint of a call: its signatures, the parameter of the cursor marked, and what the function gives. */
export function signatureHint(call: CompletionCall): HTMLElement {
  const dom = document.createElement('div');
  dom.className = 'cm-feel-signature';
  // the signatures that have a parameter for the argument, or all when none has
  const fitting = call.signatures.filter((signature) => call.named.length > 0 || call.argument === 0 || activeParameter(signature) >= 0);
  for (const signature of fitting.length ? fitting : call.signatures) {
    const line = dom.appendChild(document.createElement('div'));
    line.className = 'cm-feel-signature-line';
    line.append(`${call.function}(`);
    const active = activeParameter(signature);
    signature.parameters.forEach((parameter, index) => {
      if (index > 0) line.append(', ');
      const part = line.appendChild(document.createElement(index === active ? 'strong' : 'span'));
      part.textContent = `${parameter.name}${parameter.rest ? '...' : parameter.optional ? '?' : ''}: ${parameter.type}`;
      if (index === active || call.named.includes(parameter.name)) part.className = index === active ? 'cm-feel-signature-active' : 'cm-feel-signature-given';
    });
    line.append(`): ${signature.result}`);
  }
  const summary = dom.appendChild(document.createElement('div'));
  summary.className = 'cm-feel-signature-summary';
  summary.textContent = call.documentation ?? '';
  return dom;
}

/**
 * What shows over a name: the name with its type, or the signatures of a function, then what
 * the outline or the engine says of it and whether it is deprecated. `name` is the name as the
 * text has it.
 */
export function hoverInfo(hover: Hover, name: string): HTMLElement {
  const dom = document.createElement('div');
  dom.className = 'cm-feel-hover';
  if (hover.signatures?.length) {
    for (const signature of hover.signatures) {
      const line = dom.appendChild(document.createElement('div'));
      line.className = 'cm-feel-hover-line';
      const parameters = signature.parameters.map((parameter) => `${parameter.name}${parameter.rest ? '...' : parameter.optional ? '?' : ''}: ${parameter.type}`);
      line.textContent = `${name}(${parameters.join(', ')}): ${signature.result}`;
    }
  } else {
    const line = dom.appendChild(document.createElement('div'));
    line.className = 'cm-feel-hover-line';
    line.textContent = hover.type ? `${name}: ${hover.type}` : name;
  }
  if (hover.documentation) {
    const documentation = dom.appendChild(document.createElement('div'));
    documentation.className = 'cm-feel-hover-documentation';
    documentation.textContent = hover.documentation;
  }
  if (hover.deprecated !== undefined) {
    const deprecated = dom.appendChild(document.createElement('div'));
    deprecated.className = 'cm-feel-hover-deprecated';
    deprecated.textContent = hover.deprecated === true ? 'Deprecated.' : `Deprecated: ${hover.deprecated}`;
  }
  return dom;
}

/**
 * The tooltip for the place `pos` of the editor: what the engine answers for the name there
 * (`hover`), or null where no name stands, the engine does not answer `hover`, or the text has
 * changed since it was asked.
 */
export async function hoverAt(engine: FeelSource, view: EditorView, pos: number): Promise<Tooltip | null> {
  if (!engine.hover) return null;
  const text = view.state.doc.toString();
  const answer = await engine.hover(text, pos, view.state.facet(scopeOf));
  if (!answer.ok || !answer.hover || view.state.doc.toString() !== text) return null;
  const found = answer.hover;
  return { pos: found.from, end: found.to, above: true, create: () => ({ dom: hoverInfo(found, text.slice(found.from, found.to)) }) };
}

/**
 * The extension: highlighting, error marks and completion from the engine. Change the names in
 * scope or the dialect later with `setFeelScope`.
 */
export function feelEditor(config: FeelEditorConfig): Extension {
  const { engine, maxDiagnostics = 20, templates = true, tree = false, onSyntax, signatureHelp = true, hover = true } = config;
  const scope: FeelScope = { dialect: config.dialect, context: config.context, contextFormat: config.contextFormat, outline: config.outline };

  // asks the engine after every change of the text or of the scope
  const asker = ViewPlugin.fromClass(
    class {
      private asked = 0;
      constructor(readonly view: EditorView) {
        this.ask();
      }
      update(update: ViewUpdate) {
        if (update.docChanged || update.startState.facet(scopeOf) !== update.state.facet(scopeOf)) this.ask();
      }
      private ask() {
        const id = ++this.asked;
        const text = this.view.state.doc.toString();
        const take = (answer: SyntaxResponse) => {
          // only the answer to the last question, for the text as it still is
          if (id !== this.asked || this.view.state.doc.toString() !== text) return;
          const syntax = answer.ok ? answer : null;
          this.view.dispatch({ effects: setSyntax.of(syntax) });
          if (syntax) onSyntax?.(syntax);
        };
        // a dispatch is not allowed inside an update: the answer is taken afterwards
        Promise.resolve()
          .then(() => engine.syntax(text, { ...this.view.state.facet(scopeOf), tree }))
          .then(take, () => undefined);
      }
    },
  );

  // asks the engine for the invoked function at the cursor after every change of the text or the cursor
  const hinter = ViewPlugin.fromClass(
    class {
      private asked = 0;
      constructor(readonly view: EditorView) {
        this.ask();
      }
      update(update: ViewUpdate) {
        if (update.docChanged || update.selectionSet || update.startState.facet(scopeOf) !== update.state.facet(scopeOf)) this.ask();
      }
      private ask() {
        const id = ++this.asked;
        const { state } = this.view;
        const selection = state.selection.main;
        if (!selection.empty) return;
        const text = state.doc.toString();
        const take = (answer: CompleteResponse) => {
          const now = this.view.state;
          if (id !== this.asked || now.doc.toString() !== text || now.selection.main.head !== selection.head) return;
          const call = answer.ok ? answer.call : undefined;
          if (!call && now.field(hint) === null) return;
          this.view.dispatch({ effects: setCall.of(call ? { pos: selection.head, above: true, create: () => ({ dom: signatureHint(call) }) } : null) });
        };
        Promise.resolve()
          .then(() => engine.complete(text, selection.head, state.facet(scopeOf)))
          .then(take, () => undefined);
      }
    },
  );

  const errors = linter(
    (view): Diagnostic[] => {
      const length = view.state.doc.length;
      return (syntaxOf(view.state)?.diagnostics ?? []).slice(0, maxDiagnostics).map((d) => {
        let from = Math.min(d.from, length);
        let to = Math.min(d.to, length);
        if (from === to && from > 0) {
          // an error at an empty place (a missing part, the end of the text): mark the character before it
          from -= 1;
        } else if (from === to && to < length) {
          to += 1;
        }
        return { from, to, severity: 'error', source: 'rheon', message: d.message };
      });
    },
    { delay: 0, needsRefresh: (update) => update.transactions.some((tr) => tr.effects.some((effect) => effect.is(setSyntax))) },
  );

  const complete = async (context: CompletionContext): Promise<CompletionResult | null> => {
    const text = context.state.doc.toString();
    const answer = await engine.complete(text, context.pos, context.state.facet(scopeOf));
    if (context.aborted || !answer.ok) return null;
    const from = answer.items.reduce((least, item) => Math.min(least, item.from), context.pos);
    if (answer.items.length === 0 && !context.explicit) return null;
    // nothing is offered for an empty word unless asked for, except after a dot and where a parameter is named or values are listed
    const values = answer.items.some((item) => item.kind === 'value');
    if (from === context.pos && !context.explicit && answer.expects !== 'property' && answer.expects !== 'parameter' && !values) return null;
    const options = answer.items.map(completionOf);
    if (templates && answer.expects === 'expression') {
      const typed = text.slice(from, context.pos).toLowerCase();
      for (const [label, template] of TEMPLATES) {
        if (typed !== '' && template.toLowerCase().startsWith(typed)) options.push(snippetCompletion(template, { label, type: 'text', boost: -1 }));
      }
    }
    return options.length ? { from, to: context.pos, options, filter: false } : null;
  };

  return [
    scopeCompartment.of(scopeOf.of(scope)),
    formatterOf.of({ engine, width: config.formatWidth }),
    held,
    asker,
    colours,
    errors,
    autocompletion({ override: [complete] }),
    keymap.of([{ key: 'Shift-Alt-f', run: formatDocument }]),
    signatureHelp ? [hint, hinter] : [],
    hover && engine.hover ? hoverTooltip((view, pos) => hoverAt(engine, view, pos).catch(() => null)) : [],
  ];
}

const scopeCompartment = new Compartment();

/** The effect that gives an editor other names in scope or another dialect: `view.dispatch({ effects: setFeelScope(scope) })`. */
export function setFeelScope(scope: FeelScope): StateEffect<unknown> {
  return scopeCompartment.reconfigure(scopeOf.of(scope));
}

/**
 * A command: replaces the text of the editor with its formatted form (the engine's `format`).
 * A text with syntax errors, or one that already has the layout, stays as it is. Bound to
 * Shift-Alt-F by `feelEditor`. Returns false when the editor has no engine that formats.
 */
export function formatDocument(view: EditorView): boolean {
  const formatter = view.state.facet(formatterOf);
  if (!formatter?.engine.format) return false;
  const text = view.state.doc.toString();
  Promise.resolve()
    .then(() => formatter.engine.format!(text, { ...view.state.facet(scopeOf), width: formatter.width }))
    .then(
      (answer) => {
        // only for the text that was sent
        if (!answer.ok || !answer.changed || view.state.doc.toString() !== text) return;
        const cursor = Math.min(view.state.selection.main.head, answer.formatted.length);
        view.dispatch({ changes: { from: 0, to: text.length, insert: answer.formatted }, selection: { anchor: cursor }, userEvent: 'input.format' });
      },
      () => undefined,
    );
  return true;
}
