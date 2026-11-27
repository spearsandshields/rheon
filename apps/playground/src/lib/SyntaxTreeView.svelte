<script lang="ts">
  import { rawJson, type Dialect, type ParseResponse, type PlaygroundEngine } from '../engine';
  import { fmtTime } from './timing';
  import type { SyntaxInfo } from './CodeEditor.svelte';

  interface Props {
    engine: PlaygroundEngine | null;
    dialect: Dialect;
    expression: string;
    /** The context JSON when it is valid: the parser needs its names to split multi-word names. */
    context: string | null;
    /** The FEEL editor's parse of its text, with the cursor. */
    syntax: SyntaxInfo | null;
    onselect: (from: number, to: number) => void;
    /** Called after each request: the engine may have given up restarting its worker. */
    ondone: () => void;
  }

  let { engine, dialect, expression, context, syntax, onselect, ondone }: Props = $props();

  const DEBOUNCE_MS = 250;
  let view = $state<'tree' | 'positions'>('tree');
  let tree = $state<ParseResponse | null>(null);
  let busy = $state(false);
  let copied = $state(false);

  // The tree is asked for only while this view is shown.
  let timer: ReturnType<typeof setTimeout> | undefined;
  let abort: AbortController | null = null;
  $effect(() => {
    const request = { engine, dialect, expression, context };
    clearTimeout(timer);
    timer = setTimeout(() => parse(request), DEBOUNCE_MS);
    return () => clearTimeout(timer);
  });
  $effect(() => () => abort?.abort());

  async function parse(r: { engine: PlaygroundEngine | null; dialect: Dialect; expression: string; context: string | null }) {
    abort?.abort();
    abort = null;
    busy = false;
    if (!r.engine) return;
    if (!r.expression.trim()) {
      tree = null;
      return;
    }
    const mine = (abort = new AbortController());
    busy = true;
    try {
      const res = await r.engine.parse(r.dialect, r.expression, {
        ...(r.context === null ? {} : { context: rawJson(r.context), contextFormat: 'json' as const }),
        signal: mine.signal,
      });
      if (!mine.signal.aborted && !(!res.ok && res.error.kind === 'cancelled')) tree = res;
    } finally {
      if (abort === mine) {
        busy = false;
        abort = null;
      }
      ondone();
    }
  }

  const ERROR_TITLES: Record<string, string> = {
    syntax: 'Syntax error',
    context: 'The context is not valid',
    timeout: 'Parsing took too long, so the engine was restarted',
    crash: 'The engine crashed and was restarted',
    internal: 'Internal engine error',
    request: 'Bad request',
  };

  /** Plain-text form of the positions tree, for copying. */
  function positionsText(info: SyntaxInfo): string {
    return info.nodes.map((n) => `${'  '.repeat(n.depth)}${n.name}  ${n.from}–${n.to}`).join('\n');
  }

  async function copy() {
    const text = view === 'positions' ? (syntax ? positionsText(syntax) : '') : tree?.ok ? tree.tree : '';
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch {
      /* clipboard unavailable */
    }
  }

  /** Index of the node of the positions tree to light: the one matching the selection, else the innermost one under the cursor. */
  const cursorNode = $derived.by(() => {
    if (!syntax) return -1;
    const { nodes, cursor, anchor } = syntax;
    if (cursor !== anchor) {
      const from = Math.min(cursor, anchor);
      const to = Math.max(cursor, anchor);
      let exact = -1; // the deepest node with exactly this range
      nodes.forEach((n, i) => {
        if (n.from === from && n.to === to) exact = i;
      });
      if (exact >= 0) return exact;
    }
    let best = -1;
    nodes.forEach((n, i) => {
      const inside = n.from <= cursor && (cursor < n.to || (cursor === n.to && n.from === n.to));
      if (inside && (best < 0 || n.depth >= nodes[best].depth)) best = i;
    });
    return best;
  });

  const canCopy = $derived((view === 'tree' && tree?.ok) || (view === 'positions' && !!syntax?.nodes.length));
</script>

{#snippet positionsTree(info: SyntaxInfo)}
  <ul class="positions" data-testid="positions-tree">
    {#each info.nodes as n, i (i)}
      <li style="padding-left: {n.depth * 14}px">
        <button class="node" class:err={n.error} class:at-cursor={i === cursorNode} onclick={() => onselect(n.from, n.to)} title="Select {n.from}–{n.to} in the editor"
          ><span>{n.name}</span><span class="range">{n.from}–{n.to}</span></button
        >
      </li>
    {/each}
    {#if info.truncated}<li class="quiet">The tree is cut at {info.nodes.length} nodes.</li>{/if}
  </ul>
{/snippet}

<div class="bar">
  <div class="views" role="radiogroup" aria-label="Tree view">
    <button role="radio" aria-checked={view === 'tree'} class:on={view === 'tree'} onclick={() => (view = 'tree')} data-testid="tree-view-tree" title="The tree the engine evaluates">Tree</button>
    <button role="radio" aria-checked={view === 'positions'} class:on={view === 'positions'} onclick={() => (view = 'positions')} data-testid="tree-view-positions" title="The parts of the text as the engine reads them, with their places; select one to select it in the editor">Positions</button>
  </div>
  <span class="spacer"></span>
  {#if view === 'tree' && busy}<span class="quiet">Parsing</span>{/if}
  {#if view === 'tree' && !busy && tree?.ok}<span class="quiet">{fmtTime(tree.elapsedMicros)}</span>{/if}
  {#if canCopy}<button class="act" onclick={copy} title="Copy the tree as text" data-testid="copy-tree">{copied ? 'Copied' : 'Copy'}</button>{/if}
</div>
<div class="body">
  {#if view === 'tree'}
    {#if !expression.trim()}
      <p class="quiet">Type an expression to see its syntax tree.</p>
    {:else if !tree}
      <p class="quiet">Parsing</p>
    {:else if tree.ok}
      <pre class="ast mono" data-testid="tree">{tree.tree}</pre>
    {:else}
      <div class="error" data-testid="tree-error-{tree.error.kind}">
        <h3>{ERROR_TITLES[tree.error.kind] ?? tree.error.kind}</h3>
        <p>{tree.error.message}</p>
      </div>
      {#if tree.error.kind === 'syntax' && syntax?.nodes.length}
        <p class="quiet note">What the engine reads of the text up to and around the error:</p>
        {@render positionsTree(syntax)}
      {/if}
    {/if}
  {:else if !syntax?.nodes.length}
    <p class="quiet">Type an expression to see its parts.</p>
  {:else}
    {@render positionsTree(syntax)}
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 4px 14px;
  }
  .views {
    display: inline-flex;
    gap: 2px;
    padding: 2px;
    border-radius: 6px;
    background: var(--well);
  }
  .views button {
    padding: 2px 10px;
    border-radius: 4px;
    color: var(--tx3);
  }
  .views button.on {
    background: var(--raised);
    color: var(--tx);
  }
  .spacer {
    flex: 1;
  }
  .quiet {
    color: var(--tx2);
  }
  .body {
    padding: 6px 16px 20px;
  }
  .ast {
    margin: 0;
    font-size: var(--fs-code-small);
    line-height: 1.2; /* keeps the box-drawing connectors continuous */
    color: var(--tx2);
  }
  .error h3 {
    margin: 0;
    font-size: var(--fs);
    font-weight: 600;
    color: var(--err);
  }
  .error p {
    color: var(--tx2);
    overflow-wrap: anywhere;
  }
  .note {
    margin: 12px 0 4px;
  }
  .positions {
    list-style: none;
    margin: 0;
    padding: 0;
    font: 500 var(--fs-code-small) / 1.6 var(--mono);
  }
  .node {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 14px;
    width: 100%;
    padding: 0 6px;
    border-radius: 4px;
    text-align: left;
    color: var(--tx);
  }
  .node:hover {
    background: var(--hl);
  }
  .node.at-cursor {
    background: var(--hl);
    box-shadow: inset 2px 0 0 var(--tx3);
  }
  .node.err {
    color: var(--err);
  }
  .range {
    color: var(--tx3);
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  @media (max-width: 900px) {
    .views button {
      min-height: 36px;
    }
  }
</style>
