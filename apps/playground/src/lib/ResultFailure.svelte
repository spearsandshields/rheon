<script lang="ts" module>
  import type { FeelError } from '@rheon/feel-runtime';

  /** A crash whose message the runtime marked as a failed allocation at the 512 MiB memory cap. */
  const isOutOfMemory = (error: { kind: string; message: string }) => error.kind === 'crash' && error.message.startsWith('out of memory');

  const TITLES: Record<string, string> = {
    syntax: 'Syntax error',
    context: 'The context is not valid',
    evaluation: 'Evaluation stopped',
    request: 'Bad request',
    internal: 'Internal engine error',
    crash: 'The engine crashed and was restarted',
  };

  /** The heading of a failure, also what a screen reader is told. */
  export function failureTitle(error: { kind: string; message: string }, timeoutMs: number): string {
    if (error.kind === 'timeout') return `Stopped after ${timeoutMs / 1000} s`;
    if (isOutOfMemory(error)) return 'Out of memory';
    return TITLES[error.kind] ?? error.kind;
  }
</script>

<script lang="ts">
  interface Props {
    error: FeelError;
    timeoutMs: number;
    /** The expression the failure is about: the reasons of a syntax error point into it. */
    source: string;
    onselect: (from: number, to: number) => void;
  }

  let { error, timeoutMs, source, onselect }: Props = $props();

  /**
   * Where a reason of a syntax error points, as a range to select and the text there: a reason
   * about a missing part has an empty place, and points at the token before it.
   */
  function placeOf(detail: { from: number; to: number }): { from: number; to: number; label: string } {
    let { from, to } = detail;
    if (from === to) {
      const before = source.slice(0, from).match(/(\S+)\s*$/);
      if (before) {
        to = from - (before[0].length - before[1].length);
        from = to - before[1].length;
      }
    }
    // the place as a person reads it: line and column of where the error is, both from 1
    const before = source.slice(0, detail.from);
    const line = before.split('\n').length;
    const column = detail.from - (before.lastIndexOf('\n') + 1) + 1;
    return { from, to, label: `Line ${line}, column ${column}` };
  }
</script>

{#snippet message(text: string)}
  <!-- the engine quotes code in backticks: `,` or `]` -->
  {#each text.split(/`([^`]*)`/) as part, i (i)}{#if i % 2}<code class="mono">{part}</code>{:else}{part}{/if}{/each}
{/snippet}

<div class="failure" class:error={error.kind !== 'timeout' && !isOutOfMemory(error)} data-testid={isOutOfMemory(error) ? 'error-oom' : `error-${error.kind}`}>
  <h3>{failureTitle(error, timeoutMs)}</h3>
  {#if error.kind === 'timeout'}
    <p>The evaluation ran past its time budget, so the engine was restarted. Try a smaller list or range.</p>
  {:else if isOutOfMemory(error)}
    <p>The result, or a value on the way to it, did not fit in the engine's 512 MiB, so the engine was restarted. Try a smaller list or range.</p>
    <pre class="mono">{error.message}</pre>
  {:else if error.kind === 'syntax' && error.diagnostics?.length}
    <ul class="reasons" data-testid="syntax-details">
      {#each error.diagnostics.slice(0, 5) as detail (detail.from + detail.code)}
        {@const place = placeOf(detail)}
        <li>
          <button class="link" onclick={() => onselect(place.from, place.to)} title="Select this place in the editor" data-testid="syntax-detail">{place.label}</button>
          <span data-testid="syntax-detail-message">{@render message(detail.message)}</span>
        </li>
      {/each}
      {#if error.diagnostics.length > 5}<li>and {error.diagnostics.length - 5} more</li>{/if}
    </ul>
  {:else}
    <pre class="mono">{error.message}</pre>
  {/if}
</div>

<style>
  .failure {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 6px;
    min-width: 0;
    max-width: 100%;
  }
  h3 {
    margin: 0;
    font-size: var(--fs);
    font-weight: 600;
  }
  .error h3 {
    color: var(--err);
  }
  p,
  pre {
    margin: 0;
    color: var(--tx2);
  }
  pre {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
  .reasons {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  li {
    overflow-wrap: anywhere;
  }
  .link {
    margin-right: 6px;
    white-space: nowrap;
  }
  code {
    font-size: 0.95em;
    color: var(--tx);
  }
</style>
