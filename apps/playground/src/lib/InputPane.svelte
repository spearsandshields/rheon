<script lang="ts">
  interface Props {
    input: string;
    oninput: (value: string) => void;
    error: string | null;
  }

  let { input, oninput, error }: Props = $props();
</script>

<section class="pane" aria-labelledby="input-title">
  <header class="phead">
    <h2 class="plabel" id="input-title">Input <span class="mono">?</span></h2>
    <span class="hint">the value the tests are checked against, as JSON</span>
  </header>
  <div class="row">
    <input
      class="mono"
      class:invalid={error}
      value={input}
      oninput={(e) => oninput(e.currentTarget.value)}
      aria-invalid={error ? 'true' : 'false'}
      aria-describedby={error ? 'input-error' : undefined}
      spellcheck="false"
      autocomplete="off"
      aria-label="Input value ? as JSON"
      placeholder={'42  or  "gold"  or  [1, 2]'}
      data-testid="input-value"
    />
  </div>
  {#if error}<p class="error" id="input-error">{error}</p>{/if}
</section>

<style>
  .hint {
    color: var(--tx3);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row {
    padding: 0 12px 12px 14px;
  }
  input {
    width: 100%;
    height: 34px;
    padding: 0 10px;
    font-size: var(--fs-code-small);
    border-radius: var(--radius);
    border: 1px solid var(--raised);
    background: var(--well);
  }
  input:focus {
    outline: none;
    border-color: var(--focus);
  }
  input.invalid {
    border-color: var(--err);
  }
  .error {
    padding: 0 14px 12px;
    color: var(--err);
    overflow-wrap: anywhere;
  }
  @media (max-width: 900px) {
    input {
      height: 40px;
    }
  }
</style>
