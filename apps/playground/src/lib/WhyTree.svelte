<script lang="ts">
  import { tick } from 'svelte';
  import type { PartRow, WhyRow } from './why';

  interface Props {
    rows: WhyRow[];
    /** The editor's cursor while it has the focus: the innermost part under it is lit. */
    cursor: number | null;
    /** Opens or closes a part; opening one whose parts are not in the trace asks for them. */
    ontoggle: (row: PartRow) => void;
    /** Asks for the next iterations of a loop. */
    onmore: (row: Extract<WhyRow, { kind: 'more' }>) => void;
    /** Lights a part's place in the editor, or none. */
    onlight: (row: PartRow | null) => void;
    /** Selects a part's place in the editor. */
    onselect: (row: PartRow) => void;
  }

  let { rows, cursor, ontoggle, onmore, onlight, onselect }: Props = $props();

  let tree: HTMLDivElement;
  /** The row in the tab order (roving tab index). */
  let activeKey = $state<string | null>(null);
  const active = $derived(Math.max(0, rows.findIndex((r) => r.key === activeKey)));

  /** The innermost visible parts under the editor's cursor. */
  const atCursor = $derived.by(() => {
    if (cursor === null) return new Set<string>();
    const under = rows.filter((r): r is PartRow => r.kind === 'part' && r.from <= cursor && cursor <= r.to);
    const least = Math.min(...under.map((r) => r.to - r.from));
    return new Set(under.filter((r) => r.to - r.from === least).map((r) => r.key));
  });

  async function focusRow(index: number) {
    const row = rows[Math.max(0, Math.min(rows.length - 1, index))];
    if (!row) return;
    activeKey = row.key;
    await tick();
    tree.querySelector<HTMLElement>(`[data-key="${CSS.escape(row.key)}"]`)?.focus();
  }

  function onkeydown(e: KeyboardEvent, index: number) {
    const row = rows[index];
    const part = row.kind === 'part' ? row : null;
    const keys: Record<string, () => void> = {
      ArrowDown: () => focusRow(index + 1),
      ArrowUp: () => focusRow(index - 1),
      Home: () => focusRow(0),
      End: () => focusRow(rows.length - 1),
      ArrowRight: () => {
        if (part && !part.open && part.canExpand) ontoggle(part);
        else if (part?.open) focusRow(index + 1);
      },
      ArrowLeft: () => {
        if (part?.open) ontoggle(part);
        else if (row.parent >= 0) focusRow(row.parent);
      },
      Enter: () => (part ? onselect(part) : row.kind === 'more' && onmore(row)),
      ' ': () => (part ? part.canExpand && ontoggle(part) : row.kind === 'more' && onmore(row)),
    };
    const run = keys[e.key];
    if (!run) return;
    e.preventDefault();
    run();
  }

  /** The trail line of a row, as the left offsets of its pieces in rows of depth. */
  const at = (depth: number) => `calc(var(--pad) + ${depth} * var(--indent) + 6px)`;
</script>

<div class="tree mono" role="tree" aria-label="The parts of the expression and their values" bind:this={tree} data-testid="why-tree">
  {#each rows as row, i (row.key)}
    {#if row.kind === 'part'}
      <div
        class="row"
        class:trail={row.trail}
        class:origin={row.origin}
        class:at-cursor={atCursor.has(row.key)}
        role="treeitem"
        aria-level={row.depth + 1}
        aria-expanded={row.canExpand ? row.open : undefined}
        aria-selected="false"
        aria-label="{row.item ? `${row.item}, ` : ''}{row.label}: {row.value}{row.note ? `, ${row.note}` : ''}"
        tabindex={i === active ? 0 : -1}
        data-key={row.key}
        style="--depth: {row.depth}"
        onkeydown={(e) => onkeydown(e, i)}
        onclick={() => {
          activeKey = row.key;
          onselect(row);
        }}
        onmouseenter={() => onlight(row)}
        onmouseleave={() => onlight(null)}
        onfocus={() => onlight(row)}
        onblur={() => onlight(null)}
      >
        {#if row.line?.through !== undefined}<span class="through" style="left: {at(row.line.through)}" aria-hidden="true"></span>{/if}
        {#if row.line?.elbow !== undefined}<span class="elbow" style="left: {at(row.line.elbow)}; width: calc({row.depth - row.line.elbow} * var(--indent) - 2px)" aria-hidden="true"></span>{/if}
        {#if row.line?.down}<span class="down" style="left: {at(row.depth)}" aria-hidden="true"></span>{/if}
        <span class="src">
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (the row takes the keys: arrows and Space) -->
          <span
            class="tog"
            class:can={row.canExpand}
            class:open={row.open}
            onclick={(e) => {
              e.stopPropagation();
              activeKey = row.key;
              if (row.canExpand) ontoggle(row);
            }}
          ></span>
          {#if row.item}<span class="item">{row.item}</span>{/if}
          <span class="label" title={row.label}>{row.label}</span>
        </span>
        <span class="value" class:null={row.isNull} title={row.value}>{row.value}</span>
        <span class="note" title={row.note}>{row.note ?? ''}</span>
      </div>
    {:else}
      <div
        class="row more"
        role="treeitem"
        aria-level={row.depth + 1}
        aria-selected="false"
        tabindex={i === active ? 0 : -1}
        data-key={row.key}
        style="--depth: {row.depth}"
        onkeydown={(e) => onkeydown(e, i)}
        onclick={() => onmore(row)}
      >
        {#if row.line?.through !== undefined}<span class="through" style="left: {at(row.line.through)}" aria-hidden="true"></span>{/if}
        <span class="src"><span class="tog"></span><span class="label">{row.label}</span></span>
      </div>
    {/if}
  {/each}
</div>

<style>
  .tree {
    --pad: 14px;
    --indent: 16px;
    display: grid;
    grid-template-columns: minmax(0, 1fr) fit-content(28%) fit-content(26%);
    column-gap: 16px;
    padding: 6px 0 24px;
    font-size: var(--fs-code-small);
  }
  .row {
    position: relative;
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
    align-items: center;
    min-height: 28px;
    padding-right: 16px;
    cursor: default;
  }
  .row:hover,
  .row.at-cursor {
    background: var(--hl);
  }
  .row:focus-visible {
    outline-offset: -2px;
    border-radius: 0;
  }
  .row.origin {
    background: var(--mk-row);
    box-shadow: inset 2px 0 0 var(--mk-line);
  }
  .src {
    display: flex;
    align-items: center;
    min-width: 0;
    padding-left: calc(var(--pad) + var(--depth) * var(--indent));
    white-space: nowrap;
    color: var(--tx2);
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row.origin .label {
    color: var(--mk);
    background: var(--mk-bg);
    border-radius: 3px;
    box-shadow: inset 0 -1.5px 0 var(--mk-line);
    padding: 0 1px;
  }
  .tog {
    position: relative;
    flex: none;
    width: 14px;
    height: 28px;
    margin-right: 6px;
  }
  .tog.can {
    cursor: pointer;
  }
  .tog.can::before {
    content: '';
    position: absolute;
    left: 3px;
    top: 10px;
    border: 4px solid transparent;
    border-left: 5px solid var(--tx3);
    border-right: 0;
    transform-origin: 2px 4px;
    transition: transform 0.12s;
  }
  .tog.can.open::before {
    transform: rotate(90deg);
  }
  .row.trail .tog.can::before {
    border-left-color: var(--mk-line);
  }
  .row.origin .tog::after {
    content: '';
    position: absolute;
    left: 3px;
    top: 11px;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--mk-line);
  }
  .row.origin .tog.can::after {
    display: none;
  }
  .item {
    margin-right: 10px;
    font: 400 var(--fs-small) var(--sans);
    color: var(--tx3);
  }
  .value {
    color: var(--tx);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 40ch;
  }
  .value.null {
    color: var(--tx2);
  }
  .row.trail .value {
    color: var(--mk);
  }
  .note {
    font: 400 var(--fs-small) var(--sans);
    color: var(--tx3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row.origin .note {
    color: var(--mk);
  }
  .more .label {
    font: 400 var(--fs) var(--sans);
    color: var(--tx2);
    text-decoration: underline;
    text-decoration-color: var(--tx3);
    text-underline-offset: 3px;
    cursor: pointer;
  }
  .through,
  .elbow,
  .down {
    position: absolute;
    pointer-events: none;
  }
  .through {
    top: 0;
    bottom: 0;
    width: 1.5px;
    background: var(--mk-line);
  }
  .elbow {
    top: 0;
    height: 14px;
    border-left: 1.5px solid var(--mk-line);
    border-bottom: 1.5px solid var(--mk-line);
    border-bottom-left-radius: 5px;
  }
  .down {
    top: 20px;
    bottom: 0;
    width: 1.5px;
    background: var(--mk-line);
  }
  /* phones: the note goes under the part */
  @media (max-width: 600px) {
    .tree {
      --pad: 10px;
      --indent: 12px;
      grid-template-columns: minmax(0, 1fr) fit-content(38%);
      column-gap: 12px;
    }
    .note {
      grid-column: 1 / -1;
      padding-left: calc(var(--pad) + var(--depth) * var(--indent) + 20px);
      white-space: normal;
      padding-bottom: 4px;
    }
    .note:empty {
      display: none;
    }
  }
</style>
