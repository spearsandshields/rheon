<script lang="ts">
  import CodeEditor from './CodeEditor.svelte';

  interface Props {
    context: string;
    onchange: (value: string) => void;
    /** What is wrong with the context JSON, if anything. */
    error: string | null;
    onsubmit: () => void;
  }

  let { context, onchange, error, onsubmit }: Props = $props();
</script>

<section class="pane" aria-labelledby="context-title">
  <header class="phead">
    <h2 class="plabel" id="context-title">Context</h2>
  </header>
  <div class="pbody well" data-testid="context-editor">
    <CodeEditor value={context} language="json" label="Context JSON" describedby={error ? 'context-error' : undefined} placeholder={'{ "x": 1 }'} {onchange} {onsubmit} />
  </div>
  {#if error}<p class="error" id="context-error" data-testid="context-error">{error}</p>{/if}
</section>

<style>
  .error {
    flex: none;
    padding: 8px 14px;
    color: var(--err);
    background: var(--well);
    overflow-wrap: anywhere;
  }
</style>
