<script lang="ts">
  import { untrack } from 'svelte';
  import { jsonLanguage } from '@codemirror/lang-json';
  import CodeEditor from './CodeEditor.svelte';
  import TabBar from './TabBar.svelte';
  import { readKeys } from './reads';
  import type { EditorMark } from './marks';
  import type { ReadPath } from '../engine';

  type Tab = 'context' | 'types';

  interface Props {
    context: string;
    oncontext: (value: string) => void;
    /** What is wrong with the context JSON, if anything. */
    contextError: string | null;
    outline: string;
    onoutline: (value: string) => void;
    /** What is wrong with the Types text (its JSON, or the outline it holds), if anything. */
    outlineError: string | null;
    /** What the expression reads of the context; null when the context or the expression is not valid. */
    reads: ReadPath[] | null;
    onsubmit: () => void;
  }

  let { context, oncontext, contextError, outline, onoutline, outlineError, reads, onsubmit }: Props = $props();

  // The keys the expression reads, found in the JSON tree of the text as it is when the answer
  // arrives; the editor moves them with later edits until the next answer.
  const readMarks = $derived.by((): EditorMark[] => {
    if (!reads?.length) return [];
    const text = untrack(() => context);
    return readKeys(reads, jsonLanguage.parser.parse(text).topNode, text).map((key) => ({ ...key, class: 'cm-read' }));
  });
  let tab = $state<Tab>('context');

  const TABS: { id: Tab; label: string }[] = [
    { id: 'context', label: 'Context' },
    { id: 'types', label: 'Types' },
  ];
</script>

<section class="pane" aria-label="Context and types">
  <header class="phead">
    <TabBar tabs={TABS} selected={tab} onselect={(t) => (tab = t)} id="ctx" label="Context and types" />
    <span class="spacer"></span>
    {#if tab === 'context' && readMarks.length}<span class="legend" data-testid="reads-legend"><i aria-hidden="true"></i>read by the expression</span>{/if}
  </header>
  <div class="pbody well" id="ctx-panel-context" role="tabpanel" aria-labelledby="ctx-tab-context" hidden={tab !== 'context'} data-testid="context-editor">
    <CodeEditor
      value={context}
      language="json"
      label="Context JSON"
      describedby={contextError ? 'context-error' : undefined}
      placeholder={'{ "x": 1 }'}
      onchange={oncontext}
      {onsubmit}
      marks={readMarks}
    />
  </div>
  <div class="pbody well" id="ctx-panel-types" role="tabpanel" aria-labelledby="ctx-tab-types" hidden={tab !== 'types'} data-testid="types-editor">
    {#if !outline.trim()}
      <p class="about" id="types-about">Optional. Describe the names of the context for completion, hover and checks: their types, descriptions and allowed values. The evaluation reads the context only.</p>
    {/if}
    <CodeEditor
      value={outline}
      language="json"
      label="Types JSON"
      describedby={outlineError ? 'types-error' : outline.trim() ? undefined : 'types-about'}
      placeholder={'{ "names": { "rate": { "type": "number", "description": "Yearly rate" } } }'}
      onchange={onoutline}
      {onsubmit}
    />
  </div>
  {#if tab === 'context' && contextError}<p class="error" id="context-error" data-testid="context-error">{contextError}</p>{/if}
  {#if tab === 'types' && outlineError}<p class="error" id="types-error" data-testid="types-error">{outlineError}</p>{/if}
</section>

<style>
  .pbody[hidden] {
    display: none;
  }
  .pbody {
    display: flex;
    flex-direction: column;
  }
  .pbody > :global(.editor) {
    flex: 1;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 7px;
    padding-right: 4px;
    color: var(--tx3);
    font-size: var(--fs-small);
    white-space: nowrap;
  }
  .legend i {
    width: 16px;
    height: 12px;
    border-radius: 3px;
    background: var(--read);
  }
  .about {
    padding: 12px 16px 0 46px;
    color: var(--tx2);
    max-width: 62ch;
  }
  .error {
    flex: none;
    padding: 8px 14px;
    color: var(--err);
    background: var(--well);
    overflow-wrap: anywhere;
  }
</style>
