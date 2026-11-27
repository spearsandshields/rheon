<script lang="ts">
  import { onMount } from 'svelte';
  import type { SyntaxInfo } from './lib/CodeEditor.svelte';
  import TopBar from './lib/TopBar.svelte';
  import ExpressionPane from './lib/ExpressionPane.svelte';
  import InputPane from './lib/InputPane.svelte';
  import ContextPane from './lib/ContextPane.svelte';
  import ResultPane from './lib/ResultPane.svelte';
  import InspectorPane, { type InspectorTab } from './lib/InspectorPane.svelte';
  import StatusStrip from './lib/StatusStrip.svelte';
  import { EXAMPLES, type Example } from './lib/examples';
  import { decodeState, encodeState, checkJson, contextObject, MAX_HASH_LENGTH, type ShareState } from './lib/share';
  import { TypesCheck } from './lib/outline.svelte';
  import { nullOrigin, rows } from './lib/why';
  import { EXACT_ENOUGH_MICROS, meanTime, type Timing } from './lib/timing';
  import { WhyState } from './lib/whyState.svelte';
  import type { EditorMark } from './lib/marks';
  import { createEngine, rawJson, type Dialect, type EngineInfo, type EvaluateResponse, type ExplainResponse, type PlaygroundEngine, type ReadPath } from './engine';

  const DEBOUNCE_MS = 250;
  /** The most records an explanation holds: enough for a few loops opened at once. */
  const MAX_RECORDS = 500;
  const first = EXAMPLES[0];
  const DEFAULTS: ShareState = { dialect: first.dialect, expression: first.expression, context: first.context, input: '42', outline: first.outline ?? '', showTyped: false, inspector: 'why' };

  let dialect = $state<Dialect>(DEFAULTS.dialect);
  let expression = $state(DEFAULTS.expression);
  let context = $state(DEFAULTS.context);
  let input = $state(DEFAULTS.input);
  let outline = $state(DEFAULTS.outline);
  let showTyped = $state(DEFAULTS.showTyped);
  let inspector = $state<InspectorTab>(DEFAULTS.inspector);
  /** false until the URL has been read (it may hold a compressed state) */
  let loaded = $state(false);
  let linkNotice = $state<string | null>(null);

  let engine = $state<PlaygroundEngine | null>(null);
  let info = $state<EngineInfo | null>(null);
  let engineError = $state<string | null>(null);
  let unavailable = $state<string | null>(null);
  let response = $state<EvaluateResponse | ExplainResponse | null>(null);
  /** The text that `response` is the answer for. */
  let answered = $state('');
  const why = new WhyState();
  /** What the expression reads of the context (`paths`); null when the context or the expression is not valid. */
  let reads = $state<ReadPath[] | null>(null);
  let roundTripMs = $state<number | null>(null);
  let timing = $state<Timing | null>(null);
  let pending = $state(0);
  let syntax = $state<SyntaxInfo | null>(null);
  let expressionPane = $state<ExpressionPane>();
  let linkCopied = $state<'idle' | 'copied' | 'manual' | 'too-long'>('idle');
  let hashTooLong = $state(false);

  const contextError = $derived(checkJson(context, true));
  const inputError = $derived(dialect === 'unaryTests' ? (input.trim() ? checkJson(input, false) : 'Expected a JSON value for ?, such as 42 or "gold".') : null);
  const blocked = $derived(
    !engine
      ? (engineError ?? 'Loading the engine')
      : unavailable
        ? null
        : contextError
          ? 'The context is not valid JSON. Fix it to evaluate.'
          : inputError
            ? 'The input value ? is not valid JSON. Fix it to evaluate.'
            : !expression.trim()
              ? dialect === 'unaryTests'
                ? 'Type unary tests, such as > 10, [1..5] or "gold".'
                : 'Type a FEEL expression to evaluate it.'
              : null,
  );
  // The names the FEEL editor knows: the context's, or with a Types text the outline that the
  // engine accepted for it, laid over the context's names.
  const types = new TypesCheck(() => ({ text: outline, context: contextError ? null : context, dialect, engine }));
  const outlineError = $derived(types.errorOf(outline));
  const scope = $derived(outline.trim() && types.outline ? { outline: types.outline } : { context: contextObject(context) });

  onMount(() => {
    let disposed = false;
    void decodeState(location.hash, DEFAULTS).then(({ state, notice }) => {
      apply(state);
      linkNotice = notice;
      loaded = true;
    });
    createEngine()
      .then(async (e) => {
        if (disposed) return e.dispose();
        info = await e.info();
        engine = e;
      })
      .catch((err: unknown) => (engineError = `The engine did not load: ${String(err)}`));
    const onHash = async () => {
      if (location.hash === (await encodeState(snapshot()))) return;
      const { state, notice } = await decodeState(location.hash, DEFAULTS);
      apply(state);
      linkNotice = notice;
    };
    window.addEventListener('hashchange', onHash);
    window.addEventListener('popstate', onHash);
    return () => {
      disposed = true;
      window.removeEventListener('hashchange', onHash);
      window.removeEventListener('popstate', onHash);
      engine?.dispose();
    };
  });

  function snapshot(): ShareState {
    return { dialect, expression, context, input, outline, showTyped, inspector };
  }

  function apply(s: ShareState) {
    dialect = s.dialect;
    expression = s.expression;
    context = s.context;
    input = s.input;
    outline = s.outline;
    showTyped = s.showTyped;
    inspector = s.inspector;
  }

  async function pickExample(ex: Example) {
    // a history entry per example pick, so Back returns to what was there before
    history.pushState(null, '', await encodeState(snapshot()));
    apply({ ...snapshot(), dialect: ex.dialect, expression: ex.expression, context: ex.context, input: ex.input ?? input, outline: ex.outline ?? '' });
    linkNotice = null;
  }

  /** The engine gave up restarting its worker: stop evaluating, offer a reload. */
  function checkUnavailable() {
    if (engine?.unavailableReason) unavailable = engine.unavailableReason;
  }

  // ── live evaluation ────────────────────────────────────────────────
  // Only the newest request matters: a superseded one is cancelled, which also terminates a
  // runaway evaluation instead of letting it run into the timeout.
  let evalTimer: ReturnType<typeof setTimeout> | undefined;
  let evalAbort: AbortController | null = null;

  async function evaluateNow() {
    clearTimeout(evalTimer);
    evalAbort?.abort();
    evalAbort = null;
    if (!engine || blocked || unavailable) {
      response = null;
      reads = null;
      return;
    }
    const abort = (evalAbort = new AbortController());
    pending++;
    const t0 = performance.now();
    const text = expression;
    // what the text reads of the context: asked first, it does not evaluate and answers at once
    void engine.paths(text, { dialect, context: rawJson(context), contextFormat: 'json', signal: abort.signal }).then((res) => {
      if (!abort.signal.aborted && !(!res.ok && res.error.kind === 'cancelled')) reads = res.ok ? res.paths : null;
    });
    try {
      // An expression is explained: the answer of evaluate, with how its value came about.
      const ask = () =>
        dialect === 'unaryTests'
          ? engine!.evaluate({ dialect, expression: text, context: rawJson(context), contextFormat: 'json', input: rawJson(input) }, { signal: abort.signal })
          : engine!.explain({ expression: text, context: rawJson(context), contextFormat: 'json', expand: why.expand, maxRecords: MAX_RECORDS }, { signal: abort.signal });
      let res = await ask();
      // a new context can leave an opened iteration without its item: ask again with nothing opened
      if (!res.ok && res.error.code === 'invalid-field' && why.expand.length && !abort.signal.aborted) {
        why.reset();
        res = await ask();
      }
      if (!abort.signal.aborted && !(!res.ok && res.error.kind === 'cancelled')) {
        response = res;
        answered = text;
        roundTripMs = performance.now() - t0;
        if (res.ok) void time(res.elapsedMicros, text, abort.signal);
        else timing = null;
      }
    } catch (err) {
      if (!abort.signal.aborted) response = { ok: false, error: { kind: 'internal', code: 'internal', message: String(err) } };
    } finally {
      pending--;
      checkUnavailable();
    }
  }

  /**
   * The time of the evaluation for the status strip. A reading of a fast one is the step of the
   * browser's clock, so a fast evaluation is repeated (plain `evaluate`, one at a time, until the
   * next request cancels it) and the mean is shown; until then the strip keeps what it showed.
   */
  async function time(reading: number, text: string, signal: AbortSignal) {
    if (reading >= EXACT_ENOUGH_MICROS) {
      timing = { micros: reading, runs: 1 };
      return;
    }
    const request = { dialect, expression: text, context: rawJson(context), contextFormat: 'json' as const, ...(dialect === 'unaryTests' ? { input: rawJson(input) } : {}) };
    const mean = await meanTime(async () => {
      if (signal.aborted || !engine) return null;
      const res = await engine.evaluate(request, { signal }).catch(() => null);
      return res?.ok && !signal.aborted ? res.elapsedMicros : null;
    });
    if (!signal.aborted) timing = mean ?? { micros: reading, runs: 1 };
  }

  $effect(() => {
    // Track the inputs that affect evaluation.
    void [dialect, expression, context, input, engine, blocked];
    clearTimeout(evalTimer);
    evalTimer = setTimeout(evaluateNow, DEBOUNCE_MS);
  });

  // Another text has other places: what was opened in the Why tree does not apply to it.
  $effect(() => {
    void [dialect, expression];
    why.reset();
  });

  // ── the Why tree, and its marks in the editor ─────────────────────
  const trace = $derived(response?.ok && 'trace' in response ? response : null);
  const whyRows = $derived(trace ? rows(trace.trace, answered, why.open) : []);
  const origin = $derived(trace ? nullOrigin(trace.trace, answered) : null);
  // Places are those of the answered text; while the text is newer, nothing is marked.
  const marks = $derived.by((): EditorMark[] => {
    if (answered !== expression) return [];
    const out: EditorMark[] = [];
    if (origin) out.push({ from: origin.from, to: origin.to, class: 'cm-origin', tip: { title: 'The null of the result is from here', text: origin.reason } });
    if (why.lit) out.push({ from: why.lit.from, to: why.lit.to, class: 'cm-lit' });
    return out;
  });

  // ── URL hash (shareable state) ─────────────────────────────────────
  let hashTimer: ReturnType<typeof setTimeout> | undefined;
  let hashSeq = 0;
  $effect(() => {
    const state = snapshot();
    if (!loaded) return;
    clearTimeout(hashTimer);
    const id = ++hashSeq;
    hashTimer = setTimeout(async () => {
      const hash = await encodeState(state);
      if (id !== hashSeq) return;
      // Very large states (e.g. a big context) are not mirrored into the URL: such links break.
      hashTooLong = hash.length > MAX_HASH_LENGTH;
      if (!hashTooLong && location.hash !== hash) history.replaceState(null, '', location.pathname + hash);
    }, 300);
  });

  async function copyLink() {
    const hash = await encodeState(snapshot());
    if (hash.length > MAX_HASH_LENGTH) {
      linkCopied = 'too-long';
    } else {
      history.replaceState(null, '', location.pathname + hash);
      const url = location.origin + location.pathname + hash;
      try {
        await navigator.clipboard.writeText(url);
        linkCopied = 'copied';
      } catch {
        // Clipboard blocked (permissions, insecure context): the URL bar holds the link.
        linkCopied = 'manual';
      }
    }
    setTimeout(() => (linkCopied = 'idle'), 2500);
  }

  const select = (from: number, to: number) => expressionPane?.select(from, to);
  const selectAnswered = (from: number, to: number) => answered === expression && select(from, to);
</script>

<div class="app">
  <TopBar onpick={pickExample} oncopylink={copyLink} {linkCopied} />

  {#if unavailable}
    <div class="banner bad" role="alert" data-testid="engine-unavailable">
      <span>The engine stopped working: its worker failed to restart ({unavailable}).</span>
      <button class="act" onclick={() => location.reload()}>Reload the page</button>
    </div>
  {/if}
  {#if linkNotice}
    <div class="banner" data-testid="link-notice">
      <span>{linkNotice}</span>
      <button class="act" onclick={() => (linkNotice = null)}>Dismiss</button>
    </div>
  {/if}
  {#if hashTooLong}
    <div class="banner" data-testid="hash-too-long">
      <span>This state is too large to keep in the address bar, so the address is not updated while it is this big.</span>
    </div>
  {/if}

  <main class="panes">
    <div class="col">
      <div class="slot o-expr">
        <ExpressionPane
          bind:this={expressionPane}
          {dialect}
          ondialect={(d) => (dialect = d)}
          {expression}
          onchange={(v) => (expression = v)}
          {engine}
          {scope}
          onsubmit={evaluateNow}
          onsyntax={(s) => (syntax = s)}
          {marks}
        />
      </div>
      {#if dialect === 'unaryTests'}
        <div class="slot o-input"><InputPane {input} oninput={(v) => (input = v)} error={inputError} /></div>
      {/if}
      <div class="slot o-context">
        <ContextPane
          {context}
          oncontext={(v) => (context = v)}
          {contextError}
          {outline}
          onoutline={(v) => (outline = v)}
          {outlineError}
          {reads}
          onsubmit={evaluateNow}
        />
      </div>
    </div>
    <div class="col">
      <div class="slot o-result">
        <ResultPane
          {response}
          busy={pending > 0}
          {blocked}
          timeoutMs={engine?.timeoutMs ?? 2000}
          {showTyped}
          ontoggletyped={() => (showTyped = !showTyped)}
          source={answered}
          onselect={selectAnswered}
          {origin}
        />
      </div>
      <div class="slot o-inspector">
        <InspectorPane
          tab={inspector}
          ontab={(t) => (inspector = t)}
          {engine}
          {dialect}
          {expression}
          context={contextError ? null : context}
          {syntax}
          {blocked}
          {response}
          rows={whyRows}
          truncated={!!trace?.traceTruncated}
          maxRecords={MAX_RECORDS}
          ontoggle={(row) => why.toggle(row) && evaluateNow()}
          onmore={(row) => {
            why.more(row);
            evaluateNow();
          }}
          onlight={(row) => {
            why.lit = row;
          }}
          onselect={selectAnswered}
          ondone={checkUnavailable}
        />
      </div>
    </div>
  </main>

  <StatusStrip {info} {engineError} {timing} {roundTripMs} />
</div>
