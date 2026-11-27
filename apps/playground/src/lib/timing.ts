/**
 * The engine times a request with the browser's clock, which moves in steps of 0.1 ms to 1 ms, so
 * one reading of a fast evaluation is the step and not the time. A reading starts at a random
 * point of a step, which makes the mean of many readings the time itself: a fast evaluation is
 * repeated and the mean is shown.
 */

/** A reading at or above this is shown as it is (µs): the clock's step is a small part of it. */
export const EXACT_ENOUGH_MICROS = 5000;
/** How long the repeats of one evaluation may take together (ms), and how many there may be. */
const BUDGET_MS = 150;
const MAX_RUNS = 3000;

export interface Timing {
  /** The time of one evaluation in microseconds: one reading, or the mean of `runs`. */
  micros: number;
  runs: number;
}

/**
 * Repeats an evaluation and gives the mean of the engine's readings. `run` answers one reading in
 * microseconds, or null when the evaluation did not finish (cancelled, failed): then there is no
 * timing.
 */
export async function meanTime(run: () => Promise<number | null>, now: () => number = () => performance.now()): Promise<Timing | null> {
  const start = now();
  let sum = 0;
  let runs = 0;
  while (runs < MAX_RUNS && now() - start < BUDGET_MS) {
    const micros = await run();
    if (micros === null) return null;
    sum += micros;
    runs++;
  }
  return runs ? { micros: sum / runs, runs } : null;
}

const digits = (n: number) => (n < 10 ? n.toFixed(2) : n < 100 ? n.toFixed(1) : String(Math.round(n)));

/** A time in microseconds in the unit that fits it: s, ms, µs or ns, with three figures. */
export function fmtTime(micros: number): string {
  if (micros >= 1_000_000) return `${digits(micros / 1_000_000)} s`;
  if (micros >= 1000) return `${digits(micros / 1000)} ms`;
  if (micros >= 1) return `${digits(micros)} µs`;
  if (micros > 0) return `${digits(micros * 1000)} ns`;
  return '0 µs';
}
