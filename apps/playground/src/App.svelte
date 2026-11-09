<script lang="ts">
  import { onMount } from 'svelte';
  import CodeEditor, { type SyntaxInfo } from './lib/CodeEditor.svelte';
  import OutputPanel from './lib/OutputPanel.svelte';
  import ExamplesMenu from './lib/ExamplesMenu.svelte';
  import { EXAMPLES, type Example } from './lib/examples';
  import { decodeState, encodeState, checkJson, topLevelKeys, fmtMicros, MAX_HASH_LENGTH, type ShareState } from './lib/share';
  import { createEngine, rawJson, type Dialect, type EngineInfo, type EvaluateResponse, type ParseResponse, type PlaygroundEngine } from './engine';

  const DEBOUNCE_MS = 250;
  const first = EXAMPLES[0];
  // The syntax tree starts open on desktop widths, closed on phones.
  const wide = typeof matchMedia === 'function' && matchMedia('(min-width: 901px)').matches;
  const DEFAULTS: ShareState = {
    dialect: first.dialect,
    expression: first.expression,
    context: first.context,
    input: '42',
    showTree: wide,
    showTyped: false,
  };

  let dialect = $state<Dialect>(DEFAULTS.dialect);
  let expression = $state(DEFAULTS.expression);
  let context = $state(DEFAULTS.context);
  let input = $state(DEFAULTS.input);
  let showTree = $state(DEFAULTS.showTree);
  let showTyped = $state(DEFAULTS.showTyped);
  /** false until the URL has been read (it may hold a compressed state) */
  let loaded = $state(false);
  let linkNotice = $state<string | null>(null);

  let engine = $state<PlaygroundEngine | null>(null);
  let info = $state<EngineInfo | null>(null);
  /** Tooltip of the engine name: the exact build (fork commit, upstream base, number backend). */
  function engineTitle(i: EngineInfo): string {
    const parts = [`rheon commit ${i.engineCommit ?? '?'}`];
    if (i.upstreamCommit) parts.push(`upstream dsntk-rs ${i.upstreamCommit}`);
    parts.push(`number backend ${i.numberBackend}${i.version ? `, feel-wasm ${i.version}` : ''}`);
    return parts.join('\n');
  }
  let engineError = $state<string | null>(null);
  let unavailable = $state<string | null>(null);
  let response = $state<EvaluateResponse | null>(null);
  let roundTripMs = $state<number | null>(null);
  let pending = $state(0);
  let tree = $state<ParseResponse | null>(null);
  let treeBusy = $state(false);
  let treeView = $state<'dsntk' | 'lezer'>('dsntk');
  let treeCopied = $state(false);
  let syntax = $state<SyntaxInfo | null>(null);
  let feelEditor = $state<CodeEditor>();
  let linkCopied = $state<'idle' | 'copied' | 'manual' | 'too-long'>('idle');
  let hashTooLong = $state(false);
  let theme = $state<'auto' | 'light' | 'dark'>('auto');

  const contextError = $derived(checkJson(context, true));
  const inputError = $derived(dialect === 'unaryTests' ? (input.trim() ? checkJson(input, false) : 'Enter a JSON value for ?') : null);
  const blocked = $derived(
    !engine
      ? (engineError ?? 'Loading engine…')
      : unavailable
        ? null
        : contextError
          ? 'Fix the context JSON to evaluate.'
          : inputError
            ? 'Fix the input value (?) to evaluate.'
            : !expression.trim()
              ? dialect === 'unaryTests'
                ? 'Type unary tests, e.g. > 10, [1..5], "gold".'
                : 'Type a FEEL expression to evaluate it.'
              : null,
  );
  // Stable list identity: only changes when the set of names changes.
  const namesKey = $derived(topLevelKeys(context).join('\u0000'));
  const contextNames = $derived(namesKey ? namesKey.split('\u0000') : []);
  const firstSyntaxError = $derived(syntax?.errors[0] ?? null);

  onMount(() => {
    try {
      const t = localStorage.getItem('dsntk-playground-theme');
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
      .catch((err: unknown) => (engineError = `Engine failed to load: ${String(err)}`));
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

  const NEXT_THEME = { auto: 'light', light: 'dark', dark: 'auto' } as const;
  function cycleTheme() {
    theme = NEXT_THEME[theme];
    try {
      if (theme === 'auto') localStorage.removeItem('dsntk-playground-theme');
      else localStorage.setItem('dsntk-playground-theme', theme);
    } catch {
      /* storage unavailable */
    }
  }

  function snapshot(): ShareState {
    return { dialect, expression, context, input, showTree, showTyped };
  }

  function apply(s: ShareState) {
    dialect = s.dialect;
    expression = s.expression;
    context = s.context;
    input = s.input;
    showTree = s.showTree;
    showTyped = s.showTyped;
  }

  async function pickExample(ex: Example) {
    // a history entry per example pick, so Back returns to what was there before
    history.pushState(null, '', await encodeState(snapshot()));
    apply({ ...snapshot(), dialect: ex.dialect, expression: ex.expression, context: ex.context, input: ex.input ?? input });
    linkNotice = null;
  }

  function jumpToError() {
    if (firstSyntaxError) feelEditor?.select(firstSyntaxError.from, firstSyntaxError.to);
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
      if (!abort.signal.aborted) response = { ok: false, error: { kind: 'internal', message: String(err) } };
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

  // ── syntax tree (only while the panel is open) ─────────────────────
  let parseTimer: ReturnType<typeof setTimeout> | undefined;
  let parseAbort: AbortController | null = null;
  $effect(() => {
    void [dialect, expression, context, engine, showTree];
    clearTimeout(parseTimer);
    if (!showTree || !engine) return;
    parseTimer = setTimeout(parseNow, DEBOUNCE_MS);
  });

  async function parseNow() {
    parseAbort?.abort();
    parseAbort = null;
    treeBusy = false;
    if (!engine || unavailable) return;
    if (!expression.trim()) {
      tree = null;
      return;
    }
    const abort = (parseAbort = new AbortController());
    treeBusy = true;
    try {
      // Pass the context (when valid) so multi-word names such as `Grace's age` split correctly.
      const res = await engine.parse(dialect, expression, {
        ...(contextError ? {} : { context: rawJson(context), contextFormat: 'json' as const }),
        signal: abort.signal,
      });
      if (!abort.signal.aborted && !(!res.ok && res.error.kind === 'cancelled')) tree = res;
    } finally {
      if (parseAbort === abort) {
        treeBusy = false;
        parseAbort = null;
      }
      checkUnavailable();
    }
  }

  const TREE_ERROR_TITLES: Record<string, string> = {
    syntax: 'Syntax error',
    context: 'Invalid context',
    timeout: 'Parsing timed out — engine restarted',
    crash: 'Engine crashed — engine restarted',
    internal: 'Internal engine error',
    request: 'Bad request',
  };

  /** Plain-text form of the lezer tree, for copying. */
  function lezerText(info: SyntaxInfo): string {
    return info.nodes.map((n) => `${'  '.repeat(n.depth)}${n.name}  ${n.from}–${n.to}`).join('\n');
  }

  async function copyTree() {
    const text = treeView === 'lezer' ? (syntax ? lezerText(syntax) : '') : tree?.ok ? tree.tree : '';
    try {
      await navigator.clipboard.writeText(text);
      treeCopied = true;
      setTimeout(() => (treeCopied = false), 1200);
    } catch {
      /* clipboard unavailable */
    }
  }

  /** Index of the lezer node to highlight: the one matching the selection, else the innermost one under the cursor. */
  const cursorNode = $derived.by(() => {
    if (!syntax) return -1;
    const { nodes, cursor, anchor } = syntax;
    if (cursor !== anchor) {
      const from = Math.min(cursor, anchor);
      const to = Math.max(cursor, anchor);
      let exact = -1; // the deepest node with exactly this range
      nodes.forEach((n, i) => {
        if (n.from === from && n.to === to) exact = i;
      });
      if (exact >= 0) return exact;
    }
    let best = -1;
    nodes.forEach((n, i) => {
      const inside = n.from <= cursor && (cursor < n.to || (cursor === n.to && n.from === n.to));
      if (inside && (best < 0 || n.depth >= nodes[best].depth)) best = i;
    });
    return best;
  });

  // ── URL hash (shareable state) ─────────────────────────────────────
  let hashTimer: ReturnType<typeof setTimeout> | undefined;
  let hashSeq = 0;
  $effect(() => {
    const state = { dialect, expression, context, input, showTree, showTyped };
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
</script>

{#snippet lezerTree(info: SyntaxInfo)}
  <ul class="ltree mono" data-testid="lezer-tree">
    {#each info.nodes as n, i (i)}
      <li style="padding-left: {n.depth * 14}px">
        <button
          class="lnode"
          class:err={n.error}
          class:at-cursor={i === cursorNode}
          onclick={() => feelEditor?.select(n.from, n.to)}
          title="select {n.from}–{n.to} in the editor"
        ><span class="nname">{n.name}</span><span class="range">{n.from}–{n.to}</span></button>
      </li>
    {/each}
    {#if info.truncated}<li class="sub">… (tree cut at {info.nodes.length} nodes)</li>{/if}
  </ul>
{/snippet}

<div class="app">
  <header class="top">
    <div class="brand">
      <div class="logo" aria-hidden="true">ƒ</div>
      <div>
        <h1>rheon FEEL Playground</h1>
        <p class="engine-line" data-testid="engine-info">
          {#if info}
            engine <b title={engineTitle(info)}>{info.engine}{info.engineCommit ? `@${info.engineCommit.slice(0, 7)}` : ''}</b><span class="dot" aria-hidden="true">·</span>number backend
            <b>{info.numberBackend}</b><span class="dot dot-exact" aria-hidden="true">·</span><span class="exact">exact 34-digit decimals</span>
          {:else if engineError}
            <span class="engine-failed">engine failed to load</span>
          {:else}
            loading engine…
          {/if}
        </p>
      </div>
    </div>
    <nav class="actions">
      <ExamplesMenu onpick={pickExample} />
      <button class="btn" onclick={copyLink} data-testid="copy-link">
        <svg width="14" height="14" viewBox="0 0 16 16" aria-hidden="true"><path d="M6.5 9.5l3-3M7 4.5l1-1a2.8 2.8 0 014 4l-1 1M9 11.5l-1 1a2.8 2.8 0 01-4-4l1-1" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" /></svg>
        {linkCopied === 'copied' ? 'Copied!' : linkCopied === 'manual' ? 'Link is in the URL bar' : linkCopied === 'too-long' ? 'Too large for a link' : 'Copy link'}
      </button>
      <button
        class="btn square"
        onclick={cycleTheme}
        title="Theme: {theme} (click for {NEXT_THEME[theme]})"
        aria-label="Theme: {theme}. Activate to switch to {NEXT_THEME[theme]}."
      >
        {#if theme === 'dark'}
          <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true"><path d="M13 9.5A5.5 5.5 0 016.5 3a5.5 5.5 0 106.5 6.5z" fill="currentColor" /></svg>
        {:else if theme === 'light'}
          <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="3" fill="currentColor" /><path d="M8 1v2M8 13v2M1 8h2M13 8h2M3 3l1.4 1.4M11.6 11.6L13 13M3 13l1.4-1.4M11.6 4.4L13 3" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" /></svg>
        {:else}
          <svg width="15" height="15" viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="1.5" fill="none" /><path d="M8 2a6 6 0 010 12z" fill="currentColor" /></svg>
        {/if}
      </button>
      <a class="btn" href="https://github.com/spearsandshields/rheon" target="_blank" rel="noopener">GitHub ↗</a>
    </nav>
  </header>

  {#if unavailable}
    <div class="banner bad" role="alert" data-testid="engine-unavailable">
      <strong>The engine stopped working.</strong>
      <span>Its worker failed to restart ({unavailable}).</span>
      <button class="btn" onclick={() => location.reload()}>Reload the page</button>
    </div>
  {/if}
  {#if linkNotice}
    <div class="banner" data-testid="link-notice">
      <span>{linkNotice}</span>
      <button class="link-btn" onclick={() => (linkNotice = null)}>Dismiss</button>
    </div>
  {/if}
  {#if hashTooLong}
    <div class="banner" data-testid="hash-too-long">
      <span>This state is too large to keep in the URL, so the address bar is not updated while it is this big.</span>
    </div>
  {/if}

  <main class="grid">
    <div class="col">
      <section class="card editor-card o-expr">
        <header class="card-head">
          <h2>{dialect === 'unaryTests' ? 'Unary tests' : 'Expression'}</h2>
          <span class="spacer"></span>
          <div class="segmented" role="radiogroup" aria-label="Dialect">
            <button role="radio" aria-checked={dialect === 'expression'} class:on={dialect === 'expression'} onclick={() => (dialect = 'expression')} data-testid="dialect-expression">Expression</button>
            <button role="radio" aria-checked={dialect === 'unaryTests'} class:on={dialect === 'unaryTests'} onclick={() => (dialect = 'unaryTests')} data-testid="dialect-unary">Unary Tests</button>
          </div>
        </header>
        <div class="editor-box feel-box" data-testid="feel-editor">
          <CodeEditor
            bind:this={feelEditor}
            value={expression}
            language="feel"
            {dialect}
            {contextNames}
            label="FEEL editor"
            describedby="feel-help"
            placeholder={dialect === 'unaryTests' ? '> 10, [1..5], "gold"' : 'e.g. sum([1, 2, 3]) / 3'}
            onchange={(v) => (expression = v)}
            onsubmit={evaluateNow}
            onsyntax={(info) => (syntax = info)}
          />
        </div>
        <footer class="hint" id="feel-help">
          {#if dialect === 'unaryTests'}
            <span class="dialect-help">Each test is checked against <code>?</code>: <code>&gt; 5</code>, <code>[1..10]</code>, <code>"gold"</code>, <code>not(1, 2)</code>, <code>-</code> (anything). Write <code>&gt; 5</code>, not <code>? &gt; 5</code>.</span>
          {/if}
          <span><kbd>⌘</kbd>/<kbd>Ctrl</kbd>+<kbd>Enter</kbd> evaluates now · results update as you type · <kbd>Tab</kbd> indents: press <kbd>Esc</kbd> then <kbd>Tab</kbd> to leave the editor</span>
        </footer>
      </section>

      {#if dialect === 'unaryTests'}
        <section class="card input-card o-input">
          <header class="card-head">
            <h2>Input value <span class="q">?</span></h2>
            <span class="sub">the value the unary tests are checked against (JSON)</span>
          </header>
          <div class="input-row">
            <input
              class="mono"
              class:invalid={inputError}
              aria-invalid={inputError ? 'true' : 'false'}
              aria-describedby={inputError ? 'input-error' : undefined}
              bind:value={input}
              spellcheck="false"
              autocomplete="off"
              aria-label="Input value (?) as JSON"
              placeholder={'42  or  "gold"  or  [1, 2]'}
              data-testid="input-value"
            />
          </div>
          {#if inputError}<p class="field-error" id="input-error">{inputError}</p>{/if}
        </section>
      {/if}

      <section class="card editor-card context-card o-context">
        <header class="card-head">
          <h2>Context</h2>
          <span class="sub">JSON · numbers keep their exact digits</span>
          <span class="spacer"></span>
          {#if contextError}<span class="pill bad">invalid JSON</span>{:else}<span class="pill good">valid</span>{/if}
        </header>
        <div class="editor-box ctx-box" data-testid="context-editor">
          <CodeEditor
            value={context}
            language="json"
            label="Context JSON"
            describedby={contextError ? 'context-error' : undefined}
            placeholder={'{ "x": 1 }'}
            onchange={(v) => (context = v)}
            onsubmit={evaluateNow}
          />
        </div>
        {#if contextError}<p class="field-error" id="context-error" data-testid="context-error">{contextError}</p>{/if}
      </section>
    </div>

    <div class="col">
      <div class="o-result">
      <OutputPanel
        {response}
        busy={pending > 0}
        {blocked}
        {roundTripMs}
        timeoutMs={engine?.timeoutMs ?? 2000}
        {showTyped}
        ontoggletyped={() => (showTyped = !showTyped)}
        onjumptoerror={firstSyntaxError ? jumpToError : null}
      />
      </div>

      <section class="card tree-card o-tree">
        <header class="card-head">
          <button class="collapse" aria-expanded={showTree} onclick={() => (showTree = !showTree)} data-testid="toggle-tree">
            <svg class:open={showTree} width="10" height="10" viewBox="0 0 10 10" aria-hidden="true"><path d="M3 2l3.5 3L3 8" stroke="currentColor" stroke-width="1.6" fill="none" stroke-linecap="round" /></svg>
            <h2>Syntax tree</h2>
          </button>
          {#if showTree}
            <div class="segmented small" role="radiogroup" aria-label="Tree">
              <button role="radio" aria-checked={treeView === 'dsntk'} class:on={treeView === 'dsntk'} onclick={() => (treeView = 'dsntk')} data-testid="tree-dsntk" title="dsntk's AST (what the engine evaluates)">dsntk</button>
              <button role="radio" aria-checked={treeView === 'lezer'} class:on={treeView === 'lezer'} onclick={() => (treeView = 'lezer')} data-testid="tree-lezer" title="lezer-feel parse tree with source positions; click a node to select it">positions</button>
            </div>
          {/if}
          <span class="spacer"></span>
          {#if showTree && treeView === 'dsntk' && treeBusy}<span class="sub">parsing…</span>{/if}
          {#if showTree && treeView === 'dsntk' && !treeBusy && tree?.ok}<span class="sub">{fmtMicros(tree.elapsedMicros)}</span>{/if}
          {#if showTree && ((treeView === 'dsntk' && tree?.ok) || (treeView === 'lezer' && syntax?.nodes.length))}
            <button class="icon-btn" onclick={copyTree} title="Copy the tree as text" data-testid="copy-tree">{treeCopied ? 'Copied' : 'Copy'}</button>
          {/if}
        </header>
        {#if showTree}
          <div class="tree-body">
            {#if treeView === 'dsntk'}
              {#if !expression.trim()}
                <p class="sub">Nothing to parse yet.</p>
              {:else if !tree}
                <p class="sub">parsing…</p>
              {:else if tree.ok}
                <pre class="tree mono" data-testid="tree">{tree.tree}</pre>
              {:else}
                <div class="tree-error" data-testid="tree-error-{tree.error.kind}">
                  <strong>{TREE_ERROR_TITLES[tree.error.kind] ?? tree.error.kind}</strong>
                  {tree.error.message}
                  {#if tree.error.kind === 'syntax' && firstSyntaxError}
                    <button class="link-btn" onclick={jumpToError} data-testid="tree-jump-to-error">Jump to error</button>
                  {/if}
                </div>
                {#if tree.error.kind === 'syntax' && syntax?.nodes.length}
                  <p class="sub partial-note">Partial parse by lezer-feel (dsntk has no partial tree):</p>
                  {@render lezerTree(syntax)}
                {/if}
              {/if}
            {:else if !syntax?.nodes.length}
              <p class="sub">Nothing to parse yet.</p>
            {:else}
              <p class="sub partial-note">lezer-feel parse tree (it can differ from dsntk's); click a node to select it in the editor.</p>
              {@render lezerTree(syntax)}
            {/if}
          </div>
        {/if}
      </section>
    </div>
  </main>

  <footer class="page-foot">
    FEEL by the <a href="https://www.omg.org/dmn/" target="_blank" rel="noopener">OMG DMN</a> spec · evaluated by
    <a href="https://github.com/spearsandshields/rheon" target="_blank" rel="noopener">rheon</a> (a fork of
    <a href="https://github.com/DecisionToolkit/dsntk" target="_blank" rel="noopener">dsntk</a>) compiled to WebAssembly ·
    <a href="third-party-notices.txt" target="_blank" rel="noopener">third-party notices</a>
  </footer>
</div>

<style>
  .app {
    max-width: 1400px;
    margin: 0 auto;
    padding: 16px 20px 24px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px 20px;
    flex-wrap: wrap;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }
  .logo {
    flex: none;
    width: 38px;
    height: 38px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    background: var(--text);
    color: var(--bg);
    font: 600 22px/1 Georgia, 'Times New Roman', serif;
    font-style: italic;
  }
  h1 {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
    letter-spacing: -0.01em;
  }
  .engine-line {
    margin: 1px 0 0;
    font-size: 12.5px;
    color: var(--muted);
  }
  .engine-line b {
    white-space: nowrap;
    font-family: var(--font-mono);
    font-weight: 500;
    color: var(--text);
  }
  .dot {
    margin: 0 6px;
    color: var(--muted-2);
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .btn {
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
    text-decoration: none;
    color: var(--text);
    white-space: nowrap;
  }
  .btn:hover {
    border-color: var(--border-strong);
  }
  .btn.square {
    width: 32px;
    padding: 0;
    justify-content: center;
  }

  .grid {
    flex: 1;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .editor-box {
    min-height: 0;
    overflow: hidden;
  }
  .feel-box {
    height: 260px;
    resize: vertical;
    overflow: auto;
  }
  .ctx-box {
    height: 200px;
    resize: vertical;
    overflow: auto;
  }
  .hint {
    padding: 6px 16px 8px;
    font-size: 12px;
    color: var(--muted-2);
    border-top: 1px solid var(--border);
  }
  kbd {
    font-family: var(--font-mono);
    font-size: 10.5px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-bottom-width: 2px;
    border-radius: 4px;
    color: var(--muted);
  }
  .segmented {
    display: inline-flex;
    padding: 2px;
    border-radius: 9px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .segmented button {
    border: 0;
    background: transparent;
    padding: 4px 10px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--muted);
    border-radius: 7px;
    cursor: pointer;
  }
  .segmented button.on {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.12);
  }
  .sub {
    font-size: 12px;
    color: var(--muted);
  }
  .q {
    font-family: var(--font-mono);
    color: var(--accent);
    text-transform: none;
    font-size: 13px;
  }
  .pill {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 8px;
    border-radius: 999px;
  }
  .pill.good {
    background: var(--ok-soft);
    color: var(--ok);
  }
  .pill.bad {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .input-row {
    padding: 12px 16px;
  }
  .input-row input {
    width: 100%;
    height: 38px;
    padding: 0 12px;
    font-size: 14px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text);
  }
  .input-row input:focus {
    outline: none;
    border-color: var(--accent);
  }
  .input-row input.invalid {
    border-color: var(--danger);
  }
  .field-error {
    margin: 0;
    padding: 8px 16px;
    font-size: 12.5px;
    color: var(--danger);
    background: var(--danger-soft);
    border-top: 1px solid var(--border);
    border-radius: 0 0 var(--radius) var(--radius);
    font-family: var(--font-mono);
  }
  .collapse {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    border: 0;
    background: none;
    padding: 4px 0;
    cursor: pointer;
    color: var(--muted);
  }
  .collapse svg {
    transition: transform 0.15s;
  }
  .collapse svg.open {
    transform: rotate(90deg);
  }
  .tree-card .card-head {
    border-bottom-color: transparent;
  }
  .tree-card:has(.tree-body) .card-head {
    border-bottom-color: var(--border);
  }
  .tree-body {
    padding: 12px 16px 16px;
  }
  .tree {
    margin: 0;
    font-size: 12.5px;
    line-height: 1.3; /* keeps the box-drawing connectors continuous */
    max-height: 50vh;
    overflow: auto;
    color: var(--text);
  }
  .tree-error {
    font-size: 13px;
    color: var(--danger);
    overflow-wrap: anywhere;
  }
  .partial-note {
    margin: 10px 0 4px;
  }
  .ltree {
    list-style: none;
    margin: 0;
    padding: 0;
    font-size: 12.5px;
    max-height: 50vh;
    overflow: auto;
  }
  .lnode {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 14px;
    width: 100%;
    border: 0;
    background: none;
    padding: 1px 6px;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    color: var(--text);
    text-align: left;
  }
  .lnode:hover {
    background: var(--surface-2);
  }
  .lnode.at-cursor {
    background: var(--accent-soft);
  }
  .lnode.err {
    color: var(--danger);
    font-weight: 600;
  }
  .lnode .range {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    font-size: 11.5px;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
    padding: 10px 14px;
    border-radius: 10px;
    border: 1px solid var(--border);
    background: var(--warn-soft);
    color: var(--text);
    font-size: 13.5px;
  }
  .banner.bad {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .engine-failed {
    color: var(--danger);
  }
  .editor-box:focus-within {
    box-shadow: inset 0 0 0 2px var(--accent);
    border-radius: 4px;
  }
  .dialect-help {
    display: block;
    margin-bottom: 2px;
  }
  .hint code {
    font-family: var(--font-mono);
    color: var(--text);
  }
  .segmented.small button {
    padding: 2px 8px;
    font-size: 12px;
  }
  .page-foot {
    font-size: 12px;
    color: var(--muted);
    text-align: center;
  }

  @media (max-width: 900px) {
    /* Single column, result right below the editor(s). */
    .grid {
      display: flex;
      flex-direction: column;
      align-items: stretch;
    }
    .col {
      display: contents;
    }
    .o-expr { order: 1; }
    .o-input { order: 2; }
    .o-result { order: 3; }
    .o-context { order: 4; }
    .o-tree { order: 5; }
    .hint {
      font-size: 12px;
    }
    .feel-box {
      height: 200px;
    }
    .ctx-box {
      height: 160px;
    }
  }
  @media (max-width: 520px) {
    .app {
      padding: 12px 12px 20px;
      gap: 12px;
    }
    .actions {
      width: 100%;
    }
    .engine-line .dot-exact {
      display: none;
    }
    .engine-line .exact {
      display: block;
    }
    .card-head .sub {
      display: none;
    }
    /* touch targets: at least ~40px tall */
    .segmented button,
    .collapse {
      min-height: 40px;
    }
    .btn {
      height: 40px;
    }
    .btn.square {
      width: 40px;
    }
    .page-foot a {
      display: inline-block;
      padding: 10px 2px;
    }
  }
</style>
