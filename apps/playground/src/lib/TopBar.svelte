<script lang="ts">
  import { onMount } from 'svelte';
  import ExamplesMenu from './ExamplesMenu.svelte';
  import type { Example } from './examples';

  type Theme = 'auto' | 'light' | 'dark';

  interface Props {
    onpick: (ex: Example) => void;
    oncopylink: () => void;
    linkCopied: 'idle' | 'copied' | 'manual' | 'too-long';
  }

  let { onpick, oncopylink, linkCopied }: Props = $props();

  // The reader's theme, kept in this browser; "auto" follows the system.
  const THEME_KEY = 'dsntk-playground-theme';
  let theme = $state<Theme>('auto');

  onMount(() => {
    try {
      const t = localStorage.getItem(THEME_KEY);
      if (t === 'light' || t === 'dark') theme = t;
    } catch {
      /* storage unavailable */
    }
  });

  $effect(() => {
    const root = document.documentElement;
    if (theme === 'auto') delete root.dataset.theme;
    else root.dataset.theme = theme;
  });

  function cycleTheme() {
    theme = NEXT[theme];
    try {
      if (theme === 'auto') localStorage.removeItem(THEME_KEY);
      else localStorage.setItem(THEME_KEY, theme);
    } catch {
      /* storage unavailable */
    }
  }

  const NEXT: Record<Theme, Theme> = { auto: 'light', light: 'dark', dark: 'auto' };
  const NAMES: Record<Theme, string> = { auto: 'the system theme', light: 'the light theme', dark: 'the dark theme' };
  const LINK_TEXT = { idle: 'Copy link', copied: 'Link copied', manual: 'The link is in the address bar', 'too-long': 'Too large for a link' };
</script>

<header class="bar">
  <h1 class="brand">rheon <span>FEEL Playground</span></h1>
  <span class="spacer"></span>
  <nav class="actions" aria-label="Page">
    <ExamplesMenu {onpick} />
    <button class="act" onclick={oncopylink} data-testid="copy-link" aria-live="polite">{LINK_TEXT[linkCopied]}</button>
    <button class="act icon" onclick={cycleTheme} title="Showing {NAMES[theme]}; switch to {NAMES[NEXT[theme]]}" aria-label="Showing {NAMES[theme]}. Switch to {NAMES[NEXT[theme]]}." data-testid="theme">
      {#if theme === 'dark'}
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><path d="M13 9.5A5.5 5.5 0 016.5 3a5.5 5.5 0 106.5 6.5z" fill="currentColor" /></svg>
      {:else if theme === 'light'}
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="3" fill="currentColor" /><path d="M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.4 1.4M11.6 11.6L13 13M3 13l1.4-1.4M11.6 4.4L13 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
      {:else}
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="5.5" stroke="currentColor" stroke-width="1.5" fill="none" /><path d="M8 2.5a5.5 5.5 0 010 11z" fill="currentColor" /></svg>
      {/if}
    </button>
    <a class="act" href="https://github.com/spearsandshields/rheon" target="_blank" rel="noopener">GitHub</a>
  </nav>
</header>

<style>
  .bar {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    height: 40px;
    padding: 0 10px 0 16px;
    background: var(--chrome);
  }
  .brand {
    font-size: var(--fs);
    font-weight: 600;
    white-space: nowrap;
  }
  .brand span {
    margin-left: 4px;
    font-weight: 400;
    color: var(--tx2);
  }
  .spacer {
    flex: 1;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .icon {
    width: 28px;
    padding: 0;
    justify-content: center;
  }
  @media (max-width: 900px) {
    .bar {
      height: auto;
      flex-wrap: wrap;
      padding: 4px 4px 4px 14px;
    }
    .icon {
      width: 40px;
    }
  }
  @media (max-width: 520px) {
    .actions {
      width: 100%;
      margin-left: -12px;
    }
  }
</style>
