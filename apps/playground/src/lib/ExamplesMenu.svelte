<script lang="ts">
  import { tick } from 'svelte';
  import { EXAMPLES, type Example } from './examples';

  let { onpick }: { onpick: (ex: Example) => void } = $props();
  let open = $state(false);
  let root: HTMLDivElement;
  let trigger: HTMLButtonElement;
  let list = $state<HTMLUListElement>();

  const items = () => Array.from(list?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? []);

  async function openMenu(focus: 'first' | 'last' = 'first') {
    open = true;
    await tick();
    const all = items();
    (focus === 'first' ? all[0] : all[all.length - 1])?.focus();
  }

  function close(returnFocus: boolean) {
    open = false;
    if (returnFocus) trigger.focus();
  }

  function pick(ex: Example) {
    close(true);
    onpick(ex);
  }

  function onTriggerKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      void openMenu(e.key === 'ArrowDown' ? 'first' : 'last');
    }
  }

  // WAI-ARIA menu pattern: arrows / Home / End move between items, Escape closes and returns focus.
  function onMenuKey(e: KeyboardEvent) {
    const all = items();
    const index = all.indexOf(document.activeElement as HTMLButtonElement);
    const move = (i: number) => {
      e.preventDefault();
      all[(i + all.length) % all.length]?.focus();
    };
    switch (e.key) {
      case 'ArrowDown':
        return move(index + 1);
      case 'ArrowUp':
        return move(index - 1);
      case 'Home':
        return move(0);
      case 'End':
        return move(all.length - 1);
      case 'Escape':
        e.preventDefault();
        e.stopPropagation();
        return close(true);
      case 'Tab':
        open = false;
    }
  }

  function onWindowClick(e: MouseEvent) {
    if (open && !root.contains(e.target as Node)) open = false;
  }
</script>

<svelte:window onclick={onWindowClick} onkeydown={(e) => e.key === 'Escape' && open && close(true)} />

<div class="menu" bind:this={root}>
  <button
    class="trigger"
    bind:this={trigger}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls="examples-menu"
    onclick={() => (open ? close(false) : openMenu())}
    onkeydown={onTriggerKey}
  >
    <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><path d="M2 3.5h12M2 8h12M2 12.5h8" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" fill="none" /></svg>
    Examples
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 3.5l3 3 3-3" stroke="currentColor" stroke-width="1.5" fill="none" stroke-linecap="round" /></svg>
  </button>
  {#if open}
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <ul class="list" role="menu" id="examples-menu" aria-label="Examples" bind:this={list} onkeydown={onMenuKey}>
      {#each EXAMPLES as ex (ex.id)}
        <li role="none">
          <button role="menuitem" tabindex="-1" data-example={ex.id} onclick={() => pick(ex)}>
            <span class="title">
              {ex.title}
              {#if ex.dialect === 'unaryTests'}<span class="tag">unary</span>{/if}
              {#if ex.id === 'runaway'}<span class="tag warn">timeout</span>{/if}
            </span>
            <span class="blurb">{ex.blurb}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .menu {
    position: relative;
  }
  .trigger {
    min-height: 32px;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    height: 32px;
    padding: 0 12px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface);
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
  }
  .trigger:hover {
    border-color: var(--border-strong);
  }
  .list {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 20;
    width: min(340px, calc(100vw - 32px));
    max-height: min(70vh, 560px);
    overflow-y: auto;
    margin: 0;
    padding: 6px;
    list-style: none;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.16);
  }
  @media (max-width: 520px) {
    .trigger {
      height: 40px;
    }
    /* The trigger sits at the left edge on phones: open towards the right. */
    .list {
      left: 0;
      right: auto;
    }
  }
  .list button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 1px;
    width: 100%;
    padding: 8px 10px;
    border: 0;
    border-radius: 8px;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .list button:hover,
  .list button:focus-visible {
    background: var(--accent-soft);
    outline: none;
  }
  .title {
    font-size: 13.5px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .blurb {
    font-size: 12.5px;
    color: var(--muted);
  }
  .tag {
    font-size: 10.5px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent);
  }
  .tag.warn {
    background: var(--timeout-soft);
    color: var(--timeout);
  }
</style>
