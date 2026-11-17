<script lang="ts" module>
  /** One node of the engine's parse of the text (each before the nodes inside it), with its source range. */
  export interface SyntaxNode {
    name: string;
    from: number;
    to: number;
    depth: number;
    error: boolean;
  }

  /** What the FEEL editor reports about the engine's parse of its text (positions, partial tree on errors). */
  export interface SyntaxInfo {
    nodes: SyntaxNode[];
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
  import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { lintGutter } from '@codemirror/lint';
  import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap } from '@codemirror/autocomplete';
  import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { json } from '@codemirror/lang-json';
  import { tags as t } from '@lezer/highlight';
  import { feelEditor, setFeelScope, syntaxOf, type FeelSource } from '@rheon/feel-codemirror';
  import type { Dialect } from '../engine';

  interface Props {
    value: string;
    language: 'feel' | 'json';
    dialect?: Dialect;
    /** FEEL mode: the engine that colours the text, marks its errors and completes it. */
    engine?: FeelSource | null;
    /** FEEL mode: the context whose names are in scope. */
    scope?: Record<string, unknown>;
    label: string;
    placeholder?: string;
    onchange: (value: string) => void;
    onsubmit?: () => void;
    /** FEEL mode: called with the engine's parse after edits and cursor moves. */
    onsyntax?: (info: SyntaxInfo) => void;
    /** id of an element describing the editor (keyboard help). */
    describedby?: string;
  }

  let { value, language, dialect = 'expression', engine = null, scope = {}, label, placeholder = '', onchange, onsubmit, onsyntax, describedby }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  const lang = new Compartment();

  // Colors come from CSS custom properties, so the editor follows the page theme. The JSON
  // editor is coloured by its grammar; the FEEL editor by the engine (the --feel-* properties).
  const highlight = HighlightStyle.define([
    { tag: [t.number, t.integer, t.float], color: 'var(--syn-number)' },
    { tag: [t.string, t.special(t.string)], color: 'var(--syn-string)' },
    { tag: [t.bool, t.null, t.atom], color: 'var(--syn-atom)' },
    { tag: [t.propertyName, t.attributeName], color: 'var(--syn-prop)' },
    { tag: [t.bracket, t.paren, t.squareBracket, t.brace, t.punctuation, t.separator], color: 'var(--syn-punct)' },
    { tag: t.invalid, color: 'var(--danger)' },
  ]);

  const theme = EditorView.theme({
    '&': {
      backgroundColor: 'transparent',
      color: 'var(--text)',
      fontSize: '14px',
      height: '100%',
      '--feel-keyword': 'var(--syn-keyword)',
      '--feel-number': 'var(--syn-number)',
      '--feel-string': 'var(--syn-string)',
      '--feel-constant': 'var(--syn-atom)',
      '--feel-function': 'var(--syn-fn)',
      '--feel-property': 'var(--syn-prop)',
      '--feel-name': 'var(--syn-var)',
      '--feel-comment': 'var(--syn-comment)',
      '--feel-operator': 'var(--syn-op)',
      '--feel-punctuation': 'var(--syn-punct)',
      '--feel-error': 'var(--danger)',
    },
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

  /** The engine's parse of the document: its nodes with their depth, and its errors. */
  function analyse(state: EditorState): SyntaxInfo {
    const syntax = syntaxOf(state);
    const doc = state.doc.toString();
    const nodes: SyntaxNode[] = [];
    // the ends of the nodes that are open; a node lies inside the ones that end at or after it
    const open: number[] = [];
    for (const [name, from, to] of (syntax?.nodes ?? []).slice(0, MAX_NODES)) {
      while (open.length && open[open.length - 1] < to) open.pop();
      while (open.length && open[open.length - 1] <= from && from < to) open.pop();
      nodes.push({ name: name === 'Error' ? 'ERROR' : name, from, to, depth: open.length, error: name === 'Error' });
      open.push(to);
    }
    const errors = (syntax?.diagnostics ?? []).map((d) => {
      let { from, to } = d;
      if (to === from) {
        // an error at the end of the input has no width: point at the token before it
        const before = doc.slice(0, from).match(/(\S+)\s*$/);
        if (before) {
          to = from - (before[0].length - before[1].length);
          from = to - before[1].length;
        }
      }
      return { from, to, message: d.message };
    });
    return { nodes, truncated: (syntax?.nodes?.length ?? 0) > MAX_NODES, errors, cursor: state.selection.main.head, anchor: state.selection.main.anchor };
  }

  /** Selects a source range in the editor and scrolls it into view. */
  export function select(from: number, to: number) {
    if (!view) return;
    const len = view.state.doc.length;
    view.dispatch({ selection: { anchor: Math.min(from, len), head: Math.min(to, len) }, scrollIntoView: true });
    view.focus();
  }

  function languageExt(): Extension {
    if (language === 'json') return [json(), autocompletion()];
    // the engine reads the text: without it (still loading) the text is plain
    if (!engine) return [];
    return [
      feelEditor({ engine, dialect, context: scope, tree: true, onSyntax: () => view && onsyntax?.(analyse(view.state)) }),
      lintGutter(),
    ];
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
          highlightSelectionMatches(),
          syntaxHighlighting(highlight),
          theme,
          EditorView.lineWrapping,
          cmPlaceholder(placeholder),
          EditorView.contentAttributes.of(describedby ? { 'aria-label': label, 'aria-describedby': describedby } : { 'aria-label': label }),
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
            if (onsyntax && language === 'feel' && (u.docChanged || u.selectionSet)) onsyntax(analyse(u.state));
          }),
        ],
      }),
    });
    if (onsyntax && language === 'feel') onsyntax(analyse(view.state));
    return () => view?.destroy();
  });

  // External value changes (examples, URL hash) → replace the document.
  $effect(() => {
    const v = value;
    if (view && v !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } });
    }
  });

  // The engine arrived → the FEEL editor gets it.
  let withEngine: FeelSource | null = null;
  $effect(() => {
    if (view && language === 'feel' && engine !== withEngine) {
      withEngine = engine;
      view.dispatch({ effects: lang.reconfigure(languageExt()) });
    }
  });

  // Dialect / context changed → the editor asks the engine again.
  $effect(() => {
    const next = { dialect, context: scope };
    if (view && language === 'feel' && withEngine) view.dispatch({ effects: setFeelScope(next) });
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
