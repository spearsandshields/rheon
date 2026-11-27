<script lang="ts" module>
  export type InspectorTab = 'why' | 'tree';
</script>

<script lang="ts">
  import SyntaxTreeView from './SyntaxTreeView.svelte';
  import TabBar from './TabBar.svelte';
  import WhyTree from './WhyTree.svelte';
  import type { SyntaxInfo } from './CodeEditor.svelte';
  import type { PartRow, WhyRow } from './why';
  import type { Dialect, EvaluateResponse, PlaygroundEngine } from '../engine';

  interface Props {
    tab: InspectorTab;
    ontab: (tab: InspectorTab) => void;
    engine: PlaygroundEngine | null;
    dialect: Dialect;
    expression: string;
    /** The context JSON when it is valid. */
    context: string | null;
    syntax: SyntaxInfo | null;
    /** Why the expression is not evaluated, if it is not. */
    blocked: string | null;
    response: EvaluateResponse | null;
    rows: WhyRow[];
    truncated: boolean;
    maxRecords: number;
    ontoggle: (row: PartRow) => void;
    onmore: (row: Extract<WhyRow, { kind: 'more' }>) => void;
    onlight: (row: PartRow | null) => void;
    onselect: (from: number, to: number) => void;
    ondone: () => void;
  }

  let { tab, ontab, engine, dialect, expression, context, syntax, blocked, response, rows, truncated, maxRecords, ontoggle, onmore, onlight, onselect, ondone }: Props = $props();

  const TABS: { id: InspectorTab; label: string }[] = [
    { id: 'why', label: 'Why' },
    { id: 'tree', label: 'Syntax tree' },
  ];

  /** What the Why tab says when it has no parts to show. */
  const whyEmpty = $derived.by(() => {
    if (dialect === 'unaryTests') return 'Unary tests are not explained yet. Switch to Expression to see how a value comes about.';
    if (blocked) return blocked;
    if (!response) return 'Evaluating.';
    if (!response.ok) {
      if (response.error.kind === 'syntax') return 'Fix the syntax error to see the parts of the expression and their values.';
      return 'The evaluation did not finish, so it has no parts to show.';
    }
    return rows.length ? null : 'The engine recorded no parts of this expression.';
  });
</script>

<section class="pane" aria-label="Inspector" data-testid="inspector">
  <header class="phead">
    <TabBar tabs={TABS} selected={tab} onselect={ontab} id="insp" label="Inspector" />
  </header>
  <div class="pbody" id="insp-panel-{tab}" role="tabpanel" aria-labelledby="insp-tab-{tab}">
    {#if tab === 'why'}
      {#if whyEmpty}
        <p class="empty" data-testid="why-empty">{whyEmpty}</p>
      {:else}
        <WhyTree {rows} cursor={syntax?.focused ? syntax.cursor : null} {ontoggle} {onmore} {onlight} onselect={(row) => onselect(row.from, row.to)} />
        {#if truncated}
          <p class="empty">The trace holds its first {maxRecords} parts; the parts after them are not shown.</p>
        {/if}
      {/if}
    {:else}
      <SyntaxTreeView {engine} {dialect} {expression} {context} {syntax} {onselect} {ondone} />
    {/if}
  </div>
</section>
