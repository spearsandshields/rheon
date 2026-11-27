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
  import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, drawSelection, placeholder as cmPlaceholder } from '@codemirror/view';
  import { EditorState, Compartment, type Extension } from '@codemirror/state';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import { bracketMatching, indentOnInput, syntaxHighlighting, HighlightStyle } from '@codemirror/language';
  import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap } from '@codemirror/autocomplete';
  import { searchKeymap, highlightSelectionMatches } from '@codemirror/search';
  import { json } from '@codemirror/lang-json';
  import { tags as t } from '@lezer/highlight';
  import { feelEditor, formatDocument, setFeelScope, syntaxOf, type FeelSource } from '@rheon/feel-codemirror';
  import type { Dialect, Outline } from '../engine';

  interface Props {
    value: string;
    language: 'feel' | 'json';
    dialect?: Dialect;
    /** FEEL mode: the engine that colours the text, marks its errors and completes it. */
    engine?: FeelSource | null;
    /** FEEL mode: the names in scope, as the context or as an outline of it. */
    scope?: { context: Record<string, unknown> } | { outline: Outline };
    label: string;
    placeholder?: string;
    onchange: (value: string) => void;
    onsubmit?: () => void;
    /** FEEL mode: called with the engine's parse after edits and cursor moves. */
    onsyntax?: (info: SyntaxInfo) => void;
    /** id of an element describing the editor (keyboard help). */
    describedby?: string;
    /** The editor's keyboard shortcuts, shown when the pointer rests on it. */
    title?: string;
  }

  let { value, language, dialect = 'expression', engine = null, scope = { context: {} }, label, placeholder = '', onchange, onsubmit, onsyntax, describedby, title }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;
  const lang = new Compartment();

  // Colours come from the page's tokens, so the editor follows the theme. The JSON editor is
  // coloured by its grammar; the FEEL editor by the engine (the --feel-* properties).
  const highlight = HighlightStyle.define([
    { tag: [t.number, t.integer, t.float], color: 'var(--num)' },
    { tag: [t.string, t.special(t.string)], color: 'var(--str)' },
    { tag: [t.bool, t.null, t.atom], color: 'var(--kw)' },
    { tag: [t.propertyName, t.attributeName], color: 'var(--key)' },
    { tag: [t.bracket, t.paren, t.squareBracket, t.brace, t.punctuation, t.separator], color: 'var(--op)' },
    { tag: t.invalid, color: 'var(--err)' },
  ]);

  const theme = EditorView.theme({
    '&': {
      backgroundColor: 'transparent',
      color: 'var(--tx)',
      height: '100%',
      '--feel-keyword': 'var(--kw)',
      '--feel-number': 'var(--num)',
      '--feel-string': 'var(--str)',
      '--feel-constant': 'var(--kw)',
      '--feel-function': 'var(--fn)',
      '--feel-property': 'var(--key)',
      '--feel-name': 'var(--tx)',
      '--feel-comment': 'var(--tx3)',
      '--feel-operator': 'var(--op)',
      '--feel-punctuation': 'var(--op)',
      '--feel-error': 'var(--err)',
    },
    '&.cm-focused': { outline: 'none' },
    '.cm-scroller': { fontFamily: 'var(--mono)', fontWeight: '500' },
    '.cm-content': { caretColor: 'var(--tx)', padding: '12px 0 24px' },
    '.cm-cursor': { borderLeftColor: 'var(--tx)', borderLeftWidth: '2px' },
    '.cm-gutters': { backgroundColor: 'transparent', color: 'var(--tx3)', border: 'none', fontWeight: '400', fontSize: 'var(--fs-small)' },
    '.cm-lineNumbers .cm-gutterElement': { minWidth: '40px', padding: '0 16px 0 6px' },
    '.cm-activeLineGutter': { backgroundColor: 'transparent', color: 'var(--tx2)' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection': { backgroundColor: 'var(--sel) !important' },
    '.cm-matchingBracket, &.cm-focused .cm-matchingBracket': { backgroundColor: 'var(--hl)', outline: 'none' },
    '.cm-selectionMatch': { backgroundColor: 'var(--hl)' },
    '.cm-placeholder': { color: 'var(--tx3)' },
    '.cm-feel-keyword': { fontStyle: 'italic', fontWeight: '500' },
    '.cm-lintRange-error': { backgroundImage: 'none', textDecoration: 'underline wavy var(--err) 1.2px', textUnderlineOffset: '4px' },
    '.cm-lintRange-warning, .cm-lintRange-hint, .cm-lintRange-info': { backgroundImage: 'none', textDecoration: 'underline wavy var(--mk-line) 1.2px', textUnderlineOffset: '4px' },
    // tooltips: what shows over a name, the hint of a call, the messages of a mark, the completion
    '.cm-tooltip': { backgroundColor: 'var(--chrome)', color: 'var(--tx)', border: '1px solid var(--raised)', borderRadius: '6px', boxShadow: 'var(--shadow)', fontFamily: 'var(--sans)', fontWeight: '400', fontSize: 'var(--fs)' },
    '.cm-tooltip .cm-tooltip-arrow:before, .cm-tooltip .cm-tooltip-arrow:after': { borderTopColor: 'transparent', borderBottomColor: 'transparent' },
    '.cm-diagnostic': { padding: '6px 10px', borderLeft: '2px solid var(--err)', marginLeft: '0' },
    '.cm-diagnostic-warning, .cm-diagnostic-hint, .cm-diagnostic-info': { borderLeftColor: 'var(--mk-line)' },
    '.cm-diagnosticSource': { display: 'none' },
    '.cm-tooltip.cm-tooltip-autocomplete > ul': { fontFamily: 'var(--mono)', fontWeight: '500', fontSize: 'var(--fs-code-small)', padding: '3px', maxHeight: '16em', minWidth: '16em' },
    '.cm-tooltip.cm-tooltip-autocomplete > ul > li': { display: 'flex', alignItems: 'baseline', gap: '16px', padding: '2px 10px', lineHeight: '24px', borderRadius: '4px', color: 'var(--tx)' },
    '.cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]': { backgroundColor: 'var(--raised)', color: 'var(--tx)' },
    '.cm-completionLabel': { flex: '1 1 auto' },
    '.cm-completionMatchedText': { textDecoration: 'none', color: 'var(--fn)' },
    '.cm-completionDetail': { marginLeft: 'auto', fontStyle: 'normal', fontWeight: '400', color: 'var(--tx3)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis', maxWidth: '22em' },
    '.cm-tooltip.cm-completionInfo': { padding: '8px 12px', maxWidth: '22em', color: 'var(--tx2)', lineHeight: '1.45' },
    '.cm-feel-hover, .cm-feel-signature': { padding: '7px 11px', fontFamily: 'var(--mono)', fontWeight: '500', fontSize: 'var(--fs-code-small)', lineHeight: '1.6', maxWidth: '36em' },
    '.cm-feel-hover-documentation, .cm-feel-signature-summary': { fontFamily: 'var(--sans)', fontWeight: '400', fontSize: 'var(--fs)', color: 'var(--tx2)', opacity: '1', marginTop: '2px' },
    '.cm-feel-hover-deprecated': { fontFamily: 'var(--sans)', fontWeight: '400', fontSize: 'var(--fs)', fontStyle: 'normal', color: 'var(--mk)', opacity: '1', marginTop: '2px' },
    '.cm-feel-signature-active': { color: 'var(--fn)', textDecoration: 'underline', textUnderlineOffset: '3px' },
    '.cm-feel-signature-given': { opacity: '1', color: 'var(--tx3)' },
    // a mark's message already says that a name is deprecated
    '.cm-tooltip-hover:has(.cm-tooltip-lint) .cm-feel-hover-deprecated': { display: 'none' },
    '.cm-tooltip-section:not(:first-child)': { borderTop: '1px solid var(--raised)' },
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

  /** FEEL mode: gives the text the formatter's layout. */
  export function format() {
    if (view) formatDocument(view);
  }

  function languageExt(): Extension {
    if (language === 'json') return [json(), autocompletion()];
    // the engine reads the text: without it (still loading) the text is plain
    if (!engine) return [];
    return [
      feelEditor({ engine, dialect, ...scope, tree: true, check: true, onSyntax: () => view && onsyntax?.(analyse(view.state)) }),
      // beside the engine's completion: no icons, the type and the documentation say what an item is
      autocompletion({ icons: false }),
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

  // Dialect / names in scope changed → the editor asks the engine again.
  $effect(() => {
    const next = { dialect, ...scope };
    if (view && language === 'feel' && withEngine) view.dispatch({ effects: setFeelScope(next) });
  });
</script>

<div class="editor" class:small={language === 'json'} bind:this={host} {title}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    font-size: var(--fs-code);
    line-height: var(--lh-code);
  }
  .editor.small {
    font-size: var(--fs-code-small);
    line-height: var(--lh-code-small);
  }
  .editor :global(.cm-scroller) {
    line-height: inherit;
  }
  .editor :global(.cm-editor) {
    height: 100%;
  }
</style>
