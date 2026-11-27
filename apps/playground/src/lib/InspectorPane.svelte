<script lang="ts">
  import SyntaxTreeView from './SyntaxTreeView.svelte';
  import type { SyntaxInfo } from './CodeEditor.svelte';
  import type { Dialect, PlaygroundEngine } from '../engine';

  interface Props {
    engine: PlaygroundEngine | null;
    dialect: Dialect;
    expression: string;
    /** The context JSON when it is valid. */
    context: string | null;
    syntax: SyntaxInfo | null;
    onselect: (from: number, to: number) => void;
    ondone: () => void;
  }

  let { engine, dialect, expression, context, syntax, onselect, ondone }: Props = $props();
</script>

<section class="pane" aria-labelledby="inspector-title" data-testid="inspector">
  <header class="phead">
    <h2 class="plabel" id="inspector-title">Syntax tree</h2>
  </header>
  <div class="pbody">
    <SyntaxTreeView {engine} {dialect} {expression} {context} {syntax} {onselect} {ondone} />
  </div>
</section>
