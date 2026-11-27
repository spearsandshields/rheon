<script lang="ts" generics="T extends string">
  /** Tabs of a pane (WAI-ARIA tabs): arrows, Home and End move between them, the selected one is in the tab order. */
  interface Props {
    tabs: { id: T; label: string }[];
    selected: T;
    onselect: (id: T) => void;
    /** Prefix of the ids of the tabs and their panels: `<id>-tab-<tab>`, `<id>-panel-<tab>`. */
    id: string;
    label: string;
  }

  let { tabs, selected, onselect, id, label }: Props = $props();
  let bar: HTMLDivElement;

  function onkeydown(e: KeyboardEvent) {
    const index = tabs.findIndex((t) => t.id === selected);
    const to = { ArrowRight: index + 1, ArrowLeft: index - 1, Home: 0, End: tabs.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    const next = tabs[(to + tabs.length) % tabs.length];
    onselect(next.id);
    bar.querySelector<HTMLButtonElement>(`#${id}-tab-${next.id}`)?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label={label} bind:this={bar} tabindex="-1" {onkeydown}>
  {#each tabs as tab (tab.id)}
    <button
      role="tab"
      id="{id}-tab-{tab.id}"
      aria-selected={tab.id === selected}
      aria-controls="{id}-panel-{tab.id}"
      tabindex={tab.id === selected ? 0 : -1}
      class:on={tab.id === selected}
      onclick={() => onselect(tab.id)}
      data-testid="tab-{tab.id}">{tab.label}</button
    >
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 2px;
  }
  .tabs:focus {
    outline: none;
  }
  button {
    padding: 5px 8px;
    border-radius: var(--radius);
    color: var(--tx3);
    white-space: nowrap;
  }
  button:hover {
    color: var(--tx2);
  }
  button.on {
    color: var(--tx);
    font-weight: 500;
  }
  @media (max-width: 900px) {
    button {
      min-height: 40px;
      padding: 0 10px;
    }
  }
</style>
