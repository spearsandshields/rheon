<script lang="ts">
  import type { EvaluateResponse } from '../engine';
  import { fmtMicros } from './share';

  interface Props {
    response: EvaluateResponse | null;
    busy: boolean;
    /** Why evaluation is blocked (e.g. invalid context JSON), if it is. */
    blocked: string | null;
    roundTripMs: number | null;
    timeoutMs: number;
    showTyped: boolean;
    ontoggletyped: () => void;
    /** Set when the lezer parser located a syntax error: offers "Jump to error". */
    onjumptoerror?: (() => void) | null;
  }

  let { response, busy, blocked, roundTripMs, timeoutMs, showTyped, ontoggletyped, onjumptoerror = null }: Props = $props();
  let copied = $state(false);
  let showAll = $state(false);
  let showAllTyped = $state(false);
  let feelExpanded = $state(false);

  /** Results longer than this are cut in the page (copy still copies everything). */
  const DISPLAY_LIMIT = 20_000;
  const TYPED_LIMIT = 50_000;

  // a new result starts collapsed again
  $effect(() => {
    void response;
    showAll = false;
    showAllTyped = false;
    feelExpanded = false;
  });

  /** A crash whose message the runtime marked as a failed allocation at the 512 MiB memory cap. */
  const isOutOfMemory = (error: { kind: string; message: string }) => error.kind === 'crash' && error.message.startsWith('out of memory');

  const ERROR_TITLES: Record<string, string> = {
    syntax: 'Syntax error',
    context: 'Invalid context',
    evaluation: 'Evaluation stopped',
    request: 'Bad request',
    internal: 'Internal engine error',
    crash: 'Engine crashed — engine restarted',
  };

  function typeFamily(type: string): string {
    if (type === 'Null') return 'null';
    if (type.startsWith('list')) return 'list';
    if (type.startsWith('context')) return 'context';
    if (type === 'number' || type === 'string' || type === 'boolean') return type;
    if (/date|time|duration/.test(type)) return 'temporal';
    return 'other';
  }

  /** Text shown for the result: temporal scalars unquoted (their FEEL text), so they don't look like strings. */
  function shownText(r: { display: string; feel: string; type: string }): string {
    return typeFamily(r.type) === 'temporal' && r.display.startsWith('"') ? r.feel : r.display;
  }

  /** dsntk reads `a + b` as ONE name when `a` and `b` are unknown (FEEL names may contain + - * / and spaces). */
  function nameHint(reason: string | null | undefined): string | null {
    const key = reason?.match(/context has no value for key '(.+?)'/)?.[1];
    if (!key || !/[+\-*/ ]/.test(key)) return null;
    return `FEEL names may contain spaces and + - * /, so dsntk read "${key}" as a single name. If you meant an operation on separate values, define each of them in the context.`;
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

  // Short text for screen readers: announced once per finished evaluation, not the whole panel.
  const status = $derived.by(() => {
    if (busy || blocked || !response) return '';
    if (!response.ok) {
      if (response.error.kind === 'timeout') return 'Evaluation timed out';
      if (isOutOfMemory(response.error)) return 'Out of memory';
      return `${ERROR_TITLES[response.error.kind] ?? 'Error'}`;
    }
    const r = response.result;
    const text = shownText(r);
    return `Result, ${r.type}: ${text.length > 100 ? text.slice(0, 100) + '…' : text}`;
  });
</script>

<section class="card output" aria-busy={busy} aria-labelledby="result-title">
  <p class="sr-only" aria-live="polite" aria-atomic="true">{status}</p>
  <header class="card-head">
    <h2 id="result-title">Result</h2>
    {#if response?.ok}
      <span class="badge t-{typeFamily(response.result.type)}" title="FEEL type" data-testid="type-badge">{response.result.type}</span>
      {#if response.warnings.length || (response.result.isNull && response.result.nullReason)}
        {@const problems = response.warnings.length + (response.result.isNull && response.result.nullReason ? 1 : 0)}
        <span class="problems" title="null reasons and warnings below" data-testid="problem-count">{problems} {problems === 1 ? 'problem' : 'problems'}</span>
      {/if}
    {/if}
    <span class="spacer"></span>
    {#if busy}
      <span class="busy" data-testid="busy"><span class="spinner" aria-hidden="true"></span>evaluating</span>
    {:else if response?.ok}
      <span class="meta" title="engine time / round trip">
        {fmtMicros(response.elapsedMicros)}{#if roundTripMs !== null}<span class="sep">·</span>{roundTripMs.toFixed(0)} ms total{/if}
      </span>
    {/if}
    {#if response?.ok}
      {@const text = shownText(response.result)}
      <button class="icon-btn" onclick={() => copy(text)} title="Copy the result text">{copied ? 'Copied' : 'Copy'}</button>
    {/if}
  </header>

  <div class="body" class:stale={busy}>
    {#if blocked}
      <div class="notice neutral">{blocked}</div>
    {:else if !response}
      <div class="notice neutral">Type a FEEL expression to evaluate it.</div>
    {:else if !response.ok}
      {#if response.error.kind === 'timeout'}
        <div class="alert timeout" data-testid="error-timeout">
          <strong>Evaluation timed out after {timeoutMs / 1000}s — engine restarted</strong>
          <p>The engine exceeded its time budget, so its worker was terminated and a fresh one started.</p>
        </div>
      {:else if isOutOfMemory(response.error)}
        <div class="alert crash" data-testid="error-oom">
          <strong>Out of memory — engine restarted</strong>
          <p>The result (or an intermediate value) did not fit in the engine's 512 MiB of memory, so its worker was replaced. Try a smaller list or range.</p>
          <pre class="mono">{response.error.message}</pre>
        </div>
      {:else}
        <div class="alert {response.error.kind === 'crash' ? 'crash' : 'error'}" data-testid="error-{response.error.kind}">
          <div class="alert-head">
            <strong>{ERROR_TITLES[response.error.kind] ?? response.error.kind}</strong>
            {#if response.error.kind === 'syntax' && onjumptoerror}
              <button class="link-btn" onclick={onjumptoerror} data-testid="jump-to-error">Jump to error</button>
            {/if}
          </div>
          <pre class="mono">{response.error.message}</pre>
        </div>
      {/if}
    {:else}
      {@const r = response.result}
      {@const text = shownText(r)}
      <pre class="value mono" class:null={r.isNull} data-testid="result-display">{showAll || text.length <= DISPLAY_LIMIT ? text : text.slice(0, DISPLAY_LIMIT) + '\n…'}</pre>
      {#if text.length > DISPLAY_LIMIT}
        <div class="truncated">
          Showing {DISPLAY_LIMIT.toLocaleString()} of {text.length.toLocaleString()} characters.
          <button class="link-btn" onclick={() => (showAll = !showAll)} data-testid="show-all">{showAll ? 'Show less' : 'Show all'}</button>
        </div>
      {/if}
      {#if typeFamily(r.type) === 'context'}
        <p class="note">dsntk keeps context entries sorted by name, so they may come back in a different order than written.</p>
      {/if}
      {#if r.isNull && r.nullReason}
        {@const hint = nameHint(r.nullReason)}
        <div class="alert null-reason" data-testid="null-reason">
          <strong>null because</strong>
          <span>{r.nullReason}</span>
          {#if hint}<p class="hint-text" data-testid="name-hint">{hint}</p>{/if}
        </div>
      {/if}
      {#each response.warnings as w, i (i)}
        <div class="alert warning"><strong>Warning</strong> <span>{w.message}</span></div>
      {/each}
      <div class="foot">
        <button class="link-btn" aria-expanded={showTyped} onclick={ontoggletyped} data-testid="toggle-typed">
          {showTyped ? 'Hide' : 'Show'} typed value (DTO)
        </button>
        <button
          class="feel mono"
          class:expanded={feelExpanded}
          title={r.feel.length > 2000 ? r.feel.slice(0, 2000) + '…' : r.feel}
          aria-label="dsntk FEEL text form (click to {feelExpanded ? 'collapse' : 'expand'})"
          aria-expanded={feelExpanded}
          onclick={() => (feelExpanded = !feelExpanded)}
          data-testid="feel-text">{r.feel}</button
        >
      </div>
      {#if showTyped}
        {@const typed = JSON.stringify(r.typed, null, 2)}
        <pre class="typed mono" data-testid="typed">{showAllTyped || typed.length <= TYPED_LIMIT ? typed : typed.slice(0, TYPED_LIMIT) + '\n…'}</pre>
        {#if typed.length > TYPED_LIMIT}
          <div class="truncated">
            Showing {TYPED_LIMIT.toLocaleString()} of {typed.length.toLocaleString()} characters.
            <button class="link-btn" onclick={() => (showAllTyped = !showAllTyped)}>{showAllTyped ? 'Show less' : 'Show all'}</button>
          </div>
        {/if}
      {/if}
    {/if}
  </div>
</section>

<style>
  .output .body {
    padding: 14px 16px 16px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: opacity 0.15s;
    min-width: 0;
  }
  .stale {
    opacity: 0.55;
  }
  .badge {
    font-family: var(--font-mono);
    font-size: 12px;
    padding: 1px 8px;
    border-radius: 999px;
    border: 1px solid currentColor;
    max-width: 55%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .t-number { color: var(--t-number); }
  .t-string { color: var(--t-string); }
  .t-boolean { color: var(--t-boolean); }
  .t-temporal { color: var(--t-temporal); }
  .t-list { color: var(--t-list); }
  .t-context { color: var(--t-context); }
  .t-null, .t-other { color: var(--t-null); }
  .meta {
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .sep {
    margin: 0 5px;
    color: var(--muted-2);
  }
  .busy {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--accent);
  }
  .spinner {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    border: 2px solid var(--accent-soft);
    border-top-color: var(--accent);
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  .value {
    margin: 0;
    padding: 14px 16px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    font-size: 15px;
    line-height: 1.55;
    white-space: pre;
    max-height: 46vh;
    overflow: auto;
  }
  .value.null {
    color: var(--muted);
  }
  .notice {
    color: var(--muted);
    font-size: 14px;
    padding: 6px 0;
  }
  .alert {
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 13.5px;
    border: 1px solid transparent;
    overflow-wrap: anywhere;
  }
  .alert strong {
    margin-right: 6px;
  }
  .alert p {
    margin: 4px 0 0;
  }
  .alert pre {
    margin: 6px 0 0;
    white-space: pre-wrap;
    font-size: 13px;
  }
  .alert.error,
  .alert.crash {
    background: var(--danger-soft);
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 25%, transparent);
  }
  .alert.crash {
    border-style: dashed;
  }
  .alert.timeout {
    background: var(--timeout-soft);
    color: var(--timeout);
    border-color: color-mix(in srgb, var(--timeout) 25%, transparent);
  }
  .alert.timeout p {
    color: var(--text);
    opacity: 0.8;
  }
  .alert.null-reason {
    background: var(--surface-2);
    border-color: var(--border);
    color: var(--text);
  }
  .alert.null-reason strong {
    color: var(--muted);
    font-weight: 600;
  }
  .alert.warning {
    background: var(--warn-soft);
    color: var(--warn);
  }
  .foot {
    display: flex;
    align-items: baseline;
    gap: 12px;
    min-width: 0;
  }
  .feel {
    font-size: 12px;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
    border: 0;
    background: none;
    padding: 0;
    text-align: left;
    cursor: pointer;
  }
  .feel.expanded {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    max-height: 30vh;
    overflow: auto;
  }
  /* touch target on phones, like the other controls (app.css: ~40px) */
  @media (max-width: 520px) {
    .feel {
      min-height: 40px;
      display: flex;
      align-items: center;
    }
  }
  .foot {
    flex-wrap: wrap;
  }
  .problems {
    font-size: 11.5px;
    font-weight: 600;
    padding: 1px 8px;
    border-radius: 999px;
    background: var(--warn-soft);
    color: var(--warn);
    white-space: nowrap;
  }
  .alert-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
  }
  .truncated,
  .note {
    margin: 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .hint-text {
    color: var(--muted);
    font-size: 12.5px;
  }
  .typed {
    margin: 0;
    padding: 12px 14px;
    font-size: 12.5px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: 10px;
    max-height: 40vh;
    overflow: auto;
  }
</style>
