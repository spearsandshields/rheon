<script lang="ts">
  import type { EngineInfo } from '../engine';
  import { fmtTime, type Timing } from './timing';

  interface Props {
    info: EngineInfo | null;
    engineError: string | null;
    /** The engine's time of the last evaluation, and the time with the round trip to its worker. */
    timing: Timing | null;
    roundTripMs: number | null;
  }

  let { info, engineError, timing, roundTripMs }: Props = $props();

  const about = $derived(
    [
      timing && timing.runs > 1 ? `The mean of ${timing.runs.toLocaleString('en')} evaluations: the browser's clock is too coarse to time one.` : null,
      roundTripMs === null ? null : `${roundTripMs.toFixed(0)} ms with the round trip to the engine's worker.`,
    ]
      .filter(Boolean)
      .join(' ') || undefined,
  );

  /** The exact build: the repository commit, the upstream base and the number backend. */
  function buildTitle(i: EngineInfo): string {
    const lines = [`commit ${i.engineCommit ?? 'unknown'}`];
    if (i.upstreamCommit) lines.push(`based on upstream commit ${i.upstreamCommit}`);
    lines.push(`number backend ${i.numberBackend}`);
    return lines.join('\n');
  }
</script>

<footer class="status">
  <span data-testid="engine-info">
    {#if info}
      <span title={buildTitle(info)}>{info.engine}{info.version ? ` ${info.version}` : ''}</span>, exact 34-digit decimals
    {:else if engineError}
      <span class="failed">The engine did not load</span>
    {:else}
      Loading the engine
    {/if}
  </span>
  <span class="spacer"></span>
  {#if timing}
    <span class="time" title={about} data-testid="elapsed">Evaluated in {fmtTime(timing.micros)}</span>
  {/if}
  <span class="credits">
    <span>rheon is a fork of <a href="https://github.com/DecisionToolkit/dsntk" target="_blank" rel="noopener">dsntk</a></span>
    <a href="third-party-notices.txt" target="_blank" rel="noopener">Notices</a>
  </span>
</footer>

<style>
  .status {
    flex: none;
    display: flex;
    align-items: center;
    gap: 22px;
    height: 26px;
    padding: 0 16px;
    background: var(--chrome);
    color: var(--tx3);
    font-size: var(--fs-small);
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .time {
    font-variant-numeric: tabular-nums;
  }
  .failed {
    color: var(--err);
  }
  .credits {
    display: flex;
    gap: 16px;
  }
  .credits a {
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, currentColor 45%, transparent);
    text-underline-offset: 3px;
  }
  .credits a:hover {
    color: var(--tx);
    text-decoration-color: currentColor;
  }
  @media (max-width: 900px) {
    .status {
      height: auto;
      flex-wrap: wrap;
      gap: 4px 16px;
      padding: 10px 14px;
      white-space: normal;
    }
    .credits a {
      display: inline-block;
      padding: 8px 0;
    }
  }
</style>
