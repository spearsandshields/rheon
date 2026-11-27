<script lang="ts">
  import CodeEditor, { type SyntaxInfo } from './CodeEditor.svelte';
  import type { Dialect, Outline, PlaygroundEngine } from '../engine';

  interface Props {
    dialect: Dialect;
    ondialect: (d: Dialect) => void;
    expression: string;
    onchange: (value: string) => void;
    engine: PlaygroundEngine | null;
    /** The names in scope of the FEEL editor: the context, or the outline of the Types tab. */
    scope: { context: Record<string, unknown> } | { outline: Outline };
    onsubmit: () => void;
    onsyntax: (info: SyntaxInfo) => void;
  }

  let { dialect, ondialect, expression, onchange, engine, scope, onsubmit, onsyntax }: Props = $props();
  let editor = $state<CodeEditor>();

  /** Selects a range of the text and scrolls it into view. */
  export function select(from: number, to: number) {
    editor?.select(from, to);
  }

  const DIALECTS: { id: Dialect; label: string }[] = [
    { id: 'expression', label: 'Expression' },
    { id: 'unaryTests', label: 'Unary tests' },
  ];
</script>

<section class="pane" aria-label="Expression">
  <header class="phead">
    <div class="dialects" role="radiogroup" aria-label="Dialect">
      {#each DIALECTS as d (d.id)}
        <button role="radio" aria-checked={dialect === d.id} class:on={dialect === d.id} onclick={() => ondialect(d.id)} data-testid="dialect-{d.id === 'expression' ? 'expression' : 'unary'}">{d.label}</button>
      {/each}
    </div>
    <span class="spacer"></span>
    <button class="act" onclick={() => editor?.format()} disabled={!engine} title="Format the text: one layout, long formulas broken into indented lines (Shift+Alt+F)" data-testid="format">Format</button>
  </header>
  <div class="pbody well" data-testid="feel-editor">
    <CodeEditor
      bind:this={editor}
      value={expression}
      language="feel"
      {dialect}
      {engine}
      {scope}
      label="FEEL text"
      describedby="feel-help"
      title="Ctrl+Enter or Cmd+Enter evaluates at once. Tab indents: press Esc, then Tab, to leave the editor."
      placeholder={dialect === 'unaryTests' ? '> 10, [1..5], "gold"' : 'sum([1, 2, 3]) / 3'}
      {onchange}
      {onsubmit}
      {onsyntax}
    />
  </div>
  <p class="sr-only" id="feel-help">
    The result updates as you type; Control+Enter or Command+Enter evaluates at once. Tab indents: press Escape, then Tab, to leave the editor.
  </p>
</section>

<style>
  .dialects {
    display: flex;
    gap: 2px;
  }
  .dialects button {
    padding: 5px 8px;
    border-radius: var(--radius);
    color: var(--tx3);
  }
  .dialects button:hover {
    color: var(--tx2);
  }
  .dialects button.on {
    color: var(--tx);
    font-weight: 500;
  }
  @media (max-width: 900px) {
    .dialects button {
      min-height: 40px;
      padding: 0 10px;
    }
  }
</style>
