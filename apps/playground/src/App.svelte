<script lang="ts">
  import { onMount } from 'svelte';
  import type { SyntaxInfo } from './lib/CodeEditor.svelte';
  import TopBar, { type Theme } from './lib/TopBar.svelte';
  import ExpressionPane from './lib/ExpressionPane.svelte';
  import InputPane from './lib/InputPane.svelte';
  import ContextPane from './lib/ContextPane.svelte';
  import ResultPane from './lib/ResultPane.svelte';
  import InspectorPane from './lib/InspectorPane.svelte';
  import StatusStrip from './lib/StatusStrip.svelte';
  import { EXAMPLES, type Example } from './lib/examples';
  import { decodeState, encodeState, checkJson, contextObject, MAX_HASH_LENGTH, type ShareState } from './lib/share';
  import { mergeOutline } from './lib/outline';
  import { createEngine, rawJson, type Dialect, type EngineInfo, type EvaluateResponse, type Outline, type PlaygroundEngine } from './engine';

  const DEBOUNCE_MS = 250;
  const THEME_KEY = 'dsntk-playground-theme';
  const first = EXAMPLES[0];
  const DEFAULTS: ShareState = { dialect: first.dialect, expression: first.expression, context: first.context, input: '42', outline: first.outline ?? '', showTyped: false };

  let dialect = $state<Dialect>(DEFAULTS.dialect);
  let expression = $state(DEFAULTS.expression);
  let context = $state(DEFAULTS.context);
  let input = $state(DEFAULTS.input);
  let outline = $state(DEFAULTS.outline);
  let showTyped = $state(DEFAULTS.showTyped);
  /** false until the URL has been read (it may hold a compressed state) */
  let loaded = $state(false);
  let linkNotice = $state<string | null>(null);

  let engine = $state<PlaygroundEngine | null>(null);
  let info = $state<EngineInfo | null>(null);
  let engineError = $state<string | null>(null);
  let unavailable = $state<string | null>(null);
  let response = $state<EvaluateResponse | null>(null);
  let roundTripMs = $state<number | null>(null);
  let pending = $state(0);
  let syntax = $state<SyntaxInfo | null>(null);
  let expressionPane = $state<ExpressionPane>();
  let linkCopied = $state<'idle' | 'copied' | 'manual' | 'too-long'>('idle');
  let hashTooLong = $state(false);
  let theme = $state<Theme>('auto');

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
  // The names the FEEL editor knows: those of the context, or with a Types text the outline that
  // the engine last accepted for it, laid over the context's (lib/outline.ts). While the Types text
  // is being edited the last accepted outline stays, so the marks do not flicker.
  const outlineJsonError = $derived(outline.trim() ? checkJson(outline, true) : null);
  let editorOutline = $state<Outline | null>(null);
  let outlineCheck = $state<{ text: string; message: string } | null>(null);
  const outlineError = $derived(outlineJsonError ?? (outlineCheck?.text === outline ? outlineCheck.message : null));
  const scope = $derived(outline.trim() && editorOutline ? { outline: editorOutline } : { context: contextObject(context) });

  let outlineTimer: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    const [text, ctx, e, d, ctxValid] = [outline, context, engine, dialect, !contextError];
    clearTimeout(outlineTimer);
    if (!text.trim()) editorOutline = outlineCheck = null;
    if (!e || !text.trim() || outlineJsonError) return;
    outlineTimer = setTimeout(async () => {
      const fromContext = ctxValid ? await e.outline({ context: rawJson(ctx), contextFormat: 'json', types: true }) : null;
      const merged = mergeOutline(fromContext?.ok ? fromContext.outline : null, JSON.parse(text) as Outline);
      const res = await e.syntax('', { dialect: d, outline: merged });
      if (text !== outline || ctx !== context) return;
      if (res.ok) [editorOutline, outlineCheck] = [merged, null];
      else if (res.error.kind === 'context') [editorOutline, outlineCheck] = [null, { text, message: res.error.message }];
    }, DEBOUNCE_MS);
  });

  onMount(() => {
    try {
      const t = localStorage.getItem(THEME_KEY);
      if (t === 'light' || t === 'dark') theme = t;
    } catch {
      /* storage unavailable */
    }
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

  $effect(() => {
    const root = document.documentElement;
    if (theme === 'auto') delete root.dataset.theme;
    else root.dataset.theme = theme;
  });

  function cycleTheme() {
    theme = ({ auto: 'light', light: 'dark', dark: 'auto' } as const)[theme];
    try {
      if (theme === 'auto') localStorage.removeItem(THEME_KEY);
      else localStorage.setItem(THEME_KEY, theme);
    } catch {
      /* storage unavailable */
    }
  }

  function snapshot(): ShareState {
    return { dialect, expression, context, input, outline, showTyped };
  }

  function apply(s: ShareState) {
    dialect = s.dialect;
    expression = s.expression;
    context = s.context;
    input = s.input;
    outline = s.outline;
    showTyped = s.showTyped;
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
      return;
    }
    const abort = (evalAbort = new AbortController());
    pending++;
    const t0 = performance.now();
    try {
      const res = await engine.evaluate(
        {
          dialect,
          expression,
          context: rawJson(context),
          contextFormat: 'json',
          ...(dialect === 'unaryTests' ? { input: rawJson(input) } : {}),
        },
        { signal: abort.signal },
      );
      if (!abort.signal.aborted && !(!res.ok && res.error.kind === 'cancelled')) {
        response = res;
        roundTripMs = performance.now() - t0;
      }
    } catch (err) {
      if (!abort.signal.aborted) response = { ok: false, error: { kind: 'internal', code: 'internal', message: String(err) } };
    } finally {
      pending--;
      checkUnavailable();
    }
  }

  $effect(() => {
    // Track the inputs that affect evaluation.
    void [dialect, expression, context, input, engine, blocked];
    clearTimeout(evalTimer);
    evalTimer = setTimeout(evaluateNow, DEBOUNCE_MS);
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
</script>

<div class="app">
  <TopBar onpick={pickExample} oncopylink={copyLink} {linkCopied} {theme} oncycletheme={cycleTheme} />

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
          source={expression}
          onselect={select}
        />
      </div>
      <div class="slot o-inspector">
        <InspectorPane {engine} {dialect} {expression} context={contextError ? null : context} {syntax} onselect={select} ondone={checkUnavailable} />
      </div>
    </div>
  </main>

  <StatusStrip {info} {engineError} elapsedMicros={response?.ok ? response.elapsedMicros : null} {roundTripMs} />
</div>

<style>
  .app {
    height: 100vh;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .banner {
    flex: none;
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
    padding: 6px 16px;
    background: var(--chrome);
    color: var(--tx);
  }
  .banner.bad span {
    color: var(--err);
  }
  .panes {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 1px;
  }
  .col {
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .slot {
    display: flex;
    min-height: 0;
    min-width: 0;
  }
  .slot > :global(.pane) {
    flex: 1;
  }
  .o-expr {
    flex: 46 1 0;
  }
  .o-input {
    flex: none;
  }
  .o-context {
    flex: 54 1 0;
  }
  .o-result {
    flex: 0 1 auto;
    max-height: 55%;
  }
  .o-inspector {
    flex: 1 1 0;
  }

  /* phones and narrow windows: one column, the result right below what it is computed from */
  @media (max-width: 900px) {
    .app {
      height: auto;
      min-height: 100vh;
    }
    .panes {
      display: flex;
      flex-direction: column;
    }
    .col {
      display: contents;
    }
    .o-expr {
      order: 1;
      flex: none;
      height: 220px;
    }
    .o-input {
      order: 2;
    }
    .o-result {
      order: 3;
      max-height: none;
    }
    .o-context {
      order: 4;
      flex: none;
      height: 200px;
    }
    .o-inspector {
      order: 5;
      flex: none;
      min-height: 360px;
    }
  }
</style>
