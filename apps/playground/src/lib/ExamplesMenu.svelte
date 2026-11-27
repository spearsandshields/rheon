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
    class="act"
    bind:this={trigger}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls="examples-menu"
    onclick={() => (open ? close(false) : openMenu())}
    onkeydown={onTriggerKey}
    data-testid="examples"
  >
    Examples
    <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M2 3.5l3 3 3-3" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" /></svg>
  </button>
  {#if open}
    <!-- svelte-ignore a11y_interactive_supports_focus -->
    <ul class="list" role="menu" id="examples-menu" aria-label="Examples" bind:this={list} onkeydown={onMenuKey}>
      {#each EXAMPLES as ex (ex.id)}
        <li role="none">
          <button role="menuitem" tabindex="-1" data-example={ex.id} onclick={() => pick(ex)}>
            <span class="title">{ex.title}</span>
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
  .list {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    z-index: 20;
    width: min(340px, calc(100vw - 24px));
    max-height: min(70vh, 560px);
    overflow-y: auto;
    margin: 0;
    padding: 4px;
    list-style: none;
    background: var(--chrome);
    border: 1px solid var(--raised);
    border-radius: 6px;
    box-shadow: var(--shadow);
  }
  @media (max-width: 520px) {
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
    padding: 7px 10px;
    border-radius: 4px;
    text-align: left;
  }
  .list button:hover,
  .list button:focus-visible {
    background: var(--raised);
    outline: none;
  }
  .title {
    font-weight: 500;
  }
  .blurb {
    color: var(--tx2);
  }
</style>
