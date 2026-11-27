<script lang="ts">
  import type { EvaluateResponse } from '../engine';

  interface Props {
    response: EvaluateResponse | null;
    busy: boolean;
    /** Why evaluation is blocked (e.g. invalid context JSON), if it is. */
    blocked: string | null;
    timeoutMs: number;
    showTyped: boolean;
    ontoggletyped: () => void;
    /** The expression the response is about: the reasons of a syntax error quote it. */
    source: string;
    /** Selects a range of the expression in its editor: each reason of a syntax error is a link to its place. */
    onselect: (from: number, to: number) => void;
  }

  let { response, busy, blocked, timeoutMs, showTyped, ontoggletyped, source, onselect }: Props = $props();

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

  let copied = $state(false);
  let showAll = $state(false);
  let showAllTyped = $state(false);

  /** Results longer than this are cut in the page (copy still copies everything). */
  const DISPLAY_LIMIT = 20_000;
  const TYPED_LIMIT = 50_000;
  /** A value this short and on one line is shown large. */
  const LARGE_UP_TO = 32;

  // a new result starts collapsed again
  $effect(() => {
    void response;
    showAll = false;
    showAllTyped = false;
  });

  /** A crash whose message the runtime marked as a failed allocation at the 512 MiB memory cap. */
  const isOutOfMemory = (error: { kind: string; message: string }) => error.kind === 'crash' && error.message.startsWith('out of memory');

  const ERROR_TITLES: Record<string, string> = {
    syntax: 'Syntax error',
    context: 'The context is not valid',
    evaluation: 'Evaluation stopped',
    request: 'Bad request',
    internal: 'Internal engine error',
    crash: 'The engine crashed and was restarted',
  };

  const isTemporal = (type: string) => /date|time|duration/.test(type) && !type.startsWith('list') && !type.startsWith('context');

  /** Text shown for the result: temporal scalars unquoted (their FEEL text), so they don't look like strings. */
  function shownText(r: { display: string; feel: string; type: string }): string {
    return isTemporal(r.type) && r.display.startsWith('"') ? r.feel : r.display;
  }

  /** The engine reads `a + b` as ONE name when `a` and `b` are unknown (FEEL names may contain + - * / and spaces). */
  function nameHint(reason: string | null | undefined): string | null {
    const key = reason?.match(/context has no value for key '(.+?)'/)?.[1];
    if (!key || !/[+\-*/ ]/.test(key)) return null;
    return `FEEL names may contain spaces and + - * /, so the engine read "${key}" as a single name. If you meant an operation on separate values, define each of them in the context.`;
  }

  async function copy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      copied = true;
      setTimeout(() => (copied = false), 1200);
    } catch {
      /* clipboard unavailable */
    }
  }

  // Short text for screen readers: announced once per finished evaluation, not the whole pane.
  const status = $derived.by(() => {
    if (busy || blocked || !response) return '';
    if (!response.ok) {
      if (response.error.kind === 'timeout') return 'Evaluation timed out';
      if (isOutOfMemory(response.error)) return 'Out of memory';
      return ERROR_TITLES[response.error.kind] ?? 'Error';
    }
    const r = response.result;
    const text = shownText(r);
    return `Result, ${r.type}: ${text.length > 100 ? text.slice(0, 100) + '…' : text}`;
  });
</script>

<section class="pane result" aria-busy={busy} aria-labelledby="result-title" data-testid="result">
  <p class="sr-only" aria-live="polite" aria-atomic="true">{status}</p>
  <header class="phead">
    <h2 class="plabel" id="result-title">Result</h2>
    <span class="spacer"></span>
    {#if response?.ok}
      {@const text = shownText(response.result)}
      <button class="act" onclick={() => copy(text)} title="Copy the result text">{copied ? 'Copied' : 'Copy'}</button>
    {/if}
  </header>

  <div class="body" class:stale={busy}>
    {#if blocked}
      <p class="quiet">{blocked}</p>
    {:else if !response}
      <p class="quiet">Type a FEEL expression to evaluate it.</p>
    {:else if !response.ok}
      {#if response.error.kind === 'timeout'}
        <div class="failure" data-testid="error-timeout">
          <h3>Stopped after {timeoutMs / 1000} s</h3>
          <p>The evaluation ran past its time budget, so the engine was restarted. Try a smaller list or range.</p>
        </div>
      {:else if isOutOfMemory(response.error)}
        <div class="failure" data-testid="error-oom">
          <h3>Out of memory</h3>
          <p>The result, or a value on the way to it, did not fit in the engine's 512 MiB, so the engine was restarted. Try a smaller list or range.</p>
          <pre class="mono">{response.error.message}</pre>
        </div>
      {:else}
        <div class="failure error" data-testid="error-{response.error.kind}">
          <h3>{ERROR_TITLES[response.error.kind] ?? response.error.kind}</h3>
          {#if response.error.kind === 'syntax' && response.error.diagnostics?.length}
            <ul class="reasons" data-testid="syntax-details">
              {#each response.error.diagnostics.slice(0, 5) as detail (detail.from + detail.code)}
                {@const place = placeOf(detail)}
                <li>
                  <button class="link" onclick={() => onselect(place.from, place.to)} title="Select this place in the editor" data-testid="syntax-detail">{place.label}</button>
                  <span data-testid="syntax-detail-message">{detail.message}</span>
                </li>
              {/each}
              {#if response.error.diagnostics.length > 5}<li>and {response.error.diagnostics.length - 5} more</li>{/if}
            </ul>
          {:else}
            <pre class="mono">{response.error.message}</pre>
          {/if}
        </div>
      {/if}
    {:else}
      {@const r = response.result}
      {@const text = shownText(r)}
      {@const large = text.length <= LARGE_UP_TO && !text.includes('\n')}
      <div class="value" class:large>
        <pre class="mono" class:null={r.isNull} data-testid="result-display">{showAll || text.length <= DISPLAY_LIMIT ? text : text.slice(0, DISPLAY_LIMIT) + '\n…'}</pre>
        {#if large}<span class="type mono" title="FEEL type" data-testid="type-badge">{r.type}</span>{/if}
      </div>
      {#if !large}<p class="type mono" title="FEEL type" data-testid="type-badge">{r.type}</p>{/if}
      {#if text.length > DISPLAY_LIMIT}
        <p class="quiet">
          Showing {DISPLAY_LIMIT.toLocaleString()} of {text.length.toLocaleString()} characters.
          <button class="link" onclick={() => (showAll = !showAll)} data-testid="show-all">{showAll ? 'Show less' : 'Show all'}</button>
        </p>
      {/if}
      {#if r.isNull && r.nullReason}
        {@const hint = nameHint(r.nullReason)}
        <p class="reason" data-testid="null-reason">null because {r.nullReason}</p>
        {#if hint}<p class="quiet" data-testid="name-hint">{hint}</p>{/if}
      {/if}
      {#each response.warnings as w, i (i)}
        <p class="warning">{w.message}</p>
      {/each}
      <button class="disclosure" aria-expanded={showTyped} onclick={ontoggletyped} data-testid="toggle-typed">
        <svg width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M3.5 2l3 3-3 3" stroke="currentColor" stroke-width="1.4" fill="none" stroke-linecap="round" /></svg>
        Typed value (DTO)
      </button>
      {#if showTyped}
        {@const typed = JSON.stringify(r.typed, null, 2)}
        <pre class="typed mono" data-testid="typed">{showAllTyped || typed.length <= TYPED_LIMIT ? typed : typed.slice(0, TYPED_LIMIT) + '\n…'}</pre>
        {#if typed.length > TYPED_LIMIT}
          <p class="quiet">
            Showing {TYPED_LIMIT.toLocaleString()} of {typed.length.toLocaleString()} characters.
            <button class="link" onclick={() => (showAllTyped = !showAllTyped)}>{showAllTyped ? 'Show less' : 'Show all'}</button>
          </p>
        {/if}
      {/if}
    {/if}
  </div>
</section>

<style>
  .result {
    overflow: auto;
  }
  .body {
    padding: 4px 22px 18px 14px;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    min-width: 0;
    transition: opacity 0.15s;
  }
  /* an evaluation that takes a moment dims the old answer; a quick one does not flicker */
  .stale {
    opacity: 0.5;
    transition-delay: 0.2s;
  }
  .value {
    max-width: 100%;
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .value pre {
    margin: 0;
    font-size: 14px;
    line-height: 1.6;
    white-space: pre;
    overflow: auto;
    max-height: 46vh;
    max-width: 100%;
    font-variant-numeric: tabular-nums;
  }
  .value.large pre {
    font-size: var(--fs-value);
    line-height: 1.35;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .null {
    color: var(--mk);
  }
  .type {
    font-weight: 400;
    color: var(--tx2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
  }
  .quiet,
  .reason {
    color: var(--tx2);
    overflow-wrap: anywhere;
  }
  .warning {
    display: flex;
    align-items: baseline;
    gap: 8px;
    color: var(--tx);
    overflow-wrap: anywhere;
  }
  .warning::before {
    content: '';
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--mk-line);
    transform: translateY(-1px);
  }
  .failure {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding-top: 6px;
    min-width: 0;
    max-width: 100%;
  }
  .failure h3 {
    margin: 0;
    font-size: var(--fs);
    font-weight: 600;
  }
  .failure.error h3 {
    color: var(--err);
  }
  .failure p {
    color: var(--tx2);
  }
  .failure pre {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--tx2);
  }
  .reasons {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .reasons li {
    overflow-wrap: anywhere;
  }
  .reasons .link {
    margin-right: 6px;
    white-space: nowrap;
  }
  .disclosure {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    color: var(--tx3);
  }
  .disclosure:hover {
    color: var(--tx2);
  }
  .disclosure svg {
    transition: transform 0.12s;
  }
  .disclosure[aria-expanded='true'] svg {
    transform: rotate(90deg);
  }
  .typed {
    margin: 0;
    padding: 10px 12px;
    font-size: var(--fs-small);
    background: var(--well);
    border-radius: var(--radius);
    max-height: 40vh;
    max-width: 100%;
    overflow: auto;
  }
  @media (max-width: 900px) {
    .disclosure {
      min-height: 40px;
    }
  }
</style>
