<script lang="ts" module>
  /** One node of the lezer-feel syntax tree (pre-order), with its source range. */
  export interface LezerNode {
    name: string;
    from: number;
    to: number;
    depth: number;
    error: boolean;
  }

  /** What the FEEL editor reports about its lezer-feel parse (positions, partial tree on errors). */
  export interface SyntaxInfo {
    nodes: LezerNode[];
    /** true when the tree was cut at MAX_NODES */
    truncated: boolean;
    errors: { from: number; to: number; message: string }[];
    /** selection head (the cursor) and anchor */
    cursor: number;
    anchor: number;
  }

  const MAX_NODES = 3000;
</script>

<script lang="ts">
  import { onMount } from 'svelte';
  import { EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter, drawSelection, placeholder as cmPlaceholder } from '@codemirror/view';
  import { EditorState, Compartment, type Extension } from '@codemirror/state';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle, ensureSyntaxTree } from '@codemirror/language';
  import { linter, lintGutter, forceLinting, type Diagnostic } from '@codemirror/lint';
  import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap } from '@codemirror/autocomplete';
  import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { json } from '@codemirror/lang-json';
  import { feel } from 'lang-feel';
  import { tags as t } from '@lezer/highlight';
  import type { Dialect } from '../engine';
  import { RHEON_PARSER, configureRheonSyntax, loadRheonSyntax, refreshRheonSyntax, rheonSyntaxExtension, rheonSyntaxOf } from './rheonSyntax';


  interface Props {
    value: string;
    language: 'feel' | 'json';
    dialect?: Dialect;
    /** Names offered as completions in FEEL mode. */
    contextNames?: string[];
    label: string;
    placeholder?: string;
    onchange: (value: string) => void;
    onsubmit?: () => void;
    /** FEEL mode: called with the lezer-feel parse after edits and cursor moves. */
    onsyntax?: (info: SyntaxInfo) => void;
    /** id of an element describing the editor (keyboard help). */
    describedby?: string;
  }

  let { value, language, dialect = 'expression', contextNames = [], label, placeholder = '', onchange, onsubmit, onsyntax, describedby }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  const lang = new Compartment();

  // Colors come from CSS custom properties, so the editor follows the page theme.
  const highlight = HighlightStyle.define([
    { tag: [t.keyword, t.controlKeyword, t.operatorKeyword, t.definitionKeyword], color: 'var(--syn-keyword)', fontWeight: '600' },
    { tag: [t.number, t.integer, t.float], color: 'var(--syn-number)' },
    { tag: [t.string, t.special(t.string)], color: 'var(--syn-string)' },
    { tag: [t.bool, t.null, t.atom], color: 'var(--syn-atom)' },
    { tag: [t.function(t.variableName), t.function(t.propertyName)], color: 'var(--syn-fn)' },
    { tag: [t.propertyName, t.attributeName], color: 'var(--syn-prop)' },
    { tag: [t.variableName], color: 'var(--syn-var)' },
    { tag: [t.comment, t.lineComment, t.blockComment], color: 'var(--syn-comment)', fontStyle: 'italic' },
    { tag: [t.operator, t.compareOperator, t.arithmeticOperator, t.logicOperator], color: 'var(--syn-op)' },
    { tag: [t.bracket, t.paren, t.squareBracket, t.brace, t.punctuation, t.separator], color: 'var(--syn-punct)' },
    { tag: t.invalid, color: 'var(--danger)' },
  ]);

  const theme = EditorView.theme({
    '&': { backgroundColor: 'transparent', color: 'var(--text)', fontSize: '14px', height: '100%' },
    '&.cm-focused': { outline: 'none' },
    '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.6' },
    '.cm-content': { caretColor: 'var(--accent)', padding: '10px 0' },
    '.cm-cursor': { borderLeftColor: 'var(--accent)', borderLeftWidth: '2px' },
    '.cm-gutters': { backgroundColor: 'transparent', color: 'var(--muted-2)', border: 'none', paddingLeft: '4px' },
    '.cm-activeLine': { backgroundColor: 'var(--active-line)' },
    '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--muted)' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': { backgroundColor: 'var(--selection) !important' },
    '.cm-matchingBracket': { backgroundColor: 'var(--match)', outline: 'none' },
    '.cm-placeholder': { color: 'var(--muted-2)' },
    '.cm-tooltip': { backgroundColor: 'var(--surface-2)', border: '1px solid var(--border)', borderRadius: '8px', overflow: 'hidden' },
    '.cm-tooltip-autocomplete > ul > li[aria-selected]': { backgroundColor: 'var(--accent)', color: 'var(--on-accent)' },
    '.cm-selectionMatch': { backgroundColor: 'var(--match)' },
    '.cm-lintRange-error': { backgroundImage: 'none', textDecoration: 'underline wavy var(--danger)', textUnderlineOffset: '3px' },
    '.cm-diagnostic-error': { borderLeftColor: 'var(--danger)' },
    '.cm-gutter-lint': { width: '1.1em' },
  });

  function errorMessage(doc: string, from: number): string {
    if (from >= doc.length) return 'unexpected end of input';
    const token = doc.slice(from).match(/^\s*(\S{1,12})/)?.[1] ?? doc[from];
    return `unexpected ${JSON.stringify(token)}`;
  }

  /** lezer-feel parse of the document: flattened tree with ranges, and its error nodes. */
  function analyse(state: EditorState): SyntaxInfo {
    if (RHEON_PARSER && language === 'feel') return analyseRheon(state);
    const tree = ensureSyntaxTree(state, state.doc.length, 200);
    const doc = state.doc.toString();
    const nodes: LezerNode[] = [];
    const errors: SyntaxInfo['errors'] = [];
    let truncated = false;
    if (tree) {
      let depth = 0;
      tree.iterate({
        enter(node) {
          const error = node.type.isError;
          if (error) {
            let from = Math.min(node.from, doc.length);
            let to = Math.max(node.to, Math.min(from + 1, doc.length));
            if (to === from) {
              // An error at the end of the input is zero-width, which CodeMirror shows only as a tiny
              // point marker: underline the token before it instead (`{ a: 1, b: ` → `b:`).
              const before = doc.slice(0, from).match(/(\S+)\s*$/);
              if (before) {
                to = from - (before[0].length - before[1].length);
                from = to - before[1].length;
              }
            }
            errors.push({ from, to, message: errorMessage(doc, Math.min(node.from, doc.length)) });
          }
          if (node.type.isTop && depth === 0) {
            depth++;
            return;
          }
          if (nodes.length >= MAX_NODES) {
            truncated = true;
            return false;
          }
          nodes.push({ name: error ? 'ERROR' : node.name, from: node.from, to: node.to, depth: depth - 1, error });
          depth++;
        },
        leave() {
          depth--;
        },
      });
    }
    return { nodes, truncated, errors, cursor: state.selection.main.head, anchor: state.selection.main.anchor };
  }

  /** The engine's own parse of the document (`?parser=rheon`): its nodes with their depth, and its errors. */
  function analyseRheon(state: EditorState): SyntaxInfo {
    const syntax = rheonSyntaxOf(state);
    const doc = state.doc.toString();
    const nodes: LezerNode[] = [];
    const open: number[] = [];
    for (const [name, from, to] of (syntax?.nodes ?? []).slice(0, MAX_NODES)) {
      while (open.length && open[open.length - 1] < to) open.pop();
      if (open.length && open[open.length - 1] === to && from === to) open.pop();
      nodes.push({ name: name === 'Error' ? 'ERROR' : name, from, to, depth: open.length, error: name === 'Error' });
      open.push(to);
    }
    const errors = (syntax?.diagnostics ?? []).map((d) => {
      let { from, to } = d;
      if (to === from) {
        // an error at the end of the input: underline the token before it
        const before = doc.slice(0, from).match(/(\S+)\s*$/);
        if (before) {
          to = from - (before[0].length - before[1].length);
          from = to - before[1].length;
        }
      }
      return { from, to, message: d.message };
    });
    return { nodes, truncated: (syntax?.nodes.length ?? 0) > MAX_NODES, errors, cursor: state.selection.main.head, anchor: state.selection.main.anchor };
  }

  const syntaxLinter = linter(
    (v): Diagnostic[] =>
      analyse(v.state).errors.slice(0, 20).map((e) => ({
        from: e.from,
        to: e.to,
        severity: 'error',
        source: RHEON_PARSER ? 'rheon' : 'lezer-feel',
        message: RHEON_PARSER ? e.message : `Syntax error: ${e.message} (position ${e.from})`,
      })),
    { delay: 250 },
  );

  /** Selects a source range in the editor and scrolls it into view. */
  export function select(from: number, to: number) {
    if (!view) return;
    const len = view.state.doc.length;
    view.dispatch({ selection: { anchor: Math.min(from, len), head: Math.min(to, len) }, scrollIntoView: true });
    view.focus();
  }

  function languageExt(): Extension {
    if (language === 'json') return json();
    // the engine's own parser colours the text: no grammar of the editor
    if (RHEON_PARSER) return [];
    const context: Record<string, null> = {};
    for (const n of contextNames) context[n] = null;
    return feel({ dialect, context });
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          highlightActiveLineGutter(),
          highlightActiveLine(),
          history(),
          drawSelection(),
          indentOnInput(),
          bracketMatching(),
          closeBrackets(),
          autocompletion(),
          highlightSelectionMatches(),
          syntaxHighlighting(highlight),
          theme,
          EditorView.lineWrapping,
          cmPlaceholder(placeholder),
          EditorView.contentAttributes.of(describedby ? { 'aria-label': label, 'aria-describedby': describedby } : { 'aria-label': label }),
          ...(language === 'feel' ? [syntaxLinter, lintGutter()] : []),
          ...(language === 'feel' && RHEON_PARSER ? [rheonSyntaxExtension] : []),
          keymap.of([
            { key: 'Mod-Enter', run: () => (onsubmit?.(), true) },
            ...closeBracketsKeymap,
            ...defaultKeymap,
            ...searchKeymap,
            ...historyKeymap,
            ...completionKeymap,
            indentWithTab,
          ]),
          lang.of(languageExt()),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) onchange(u.state.doc.toString());
            const refreshed = u.transactions.some((tr) => tr.effects.some((effect) => effect.is(refreshRheonSyntax)));
            if (onsyntax && (u.docChanged || u.selectionSet || refreshed || syntaxChanged(u))) onsyntax(analyse(u.state));
          }),
        ],
      }),
    });
    if (onsyntax && view) onsyntax(analyse(view.state));
    if (RHEON_PARSER && language === 'feel') {
      configureRheonSyntax(dialect, contextNames);
      void loadRheonSyntax().then(() => {
        view?.dispatch({ effects: refreshRheonSyntax.of(null) });
        if (view) forceLinting(view);
      });
    }
    return () => view?.destroy();
  });

  // the parse can complete after the edit that triggered it
  function syntaxChanged(u: { startState: EditorState; state: EditorState }): boolean {
    return ensureSyntaxTree(u.startState, 0, 0) !== ensureSyntaxTree(u.state, 0, 0);
  }

  // External value changes (examples, URL hash) → replace the document.
  $effect(() => {
    const v = value;
    if (view && v !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } });
    }
  });

  // Dialect / context names changed → reconfigure the language.
  $effect(() => {
    void dialect;
    void contextNames;
    if (RHEON_PARSER && language === 'feel') configureRheonSyntax(dialect, contextNames);
    view?.dispatch({ effects: [lang.reconfigure(languageExt()), refreshRheonSyntax.of(null)] });
    if (view && language === 'feel') {
      forceLinting(view);
      onsyntax?.(analyse(view.state));
    }
  });
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
  }
  .editor :global(.cm-editor) {
    height: 100%;
  }
</style>
