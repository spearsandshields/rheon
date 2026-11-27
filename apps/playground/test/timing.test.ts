import assert from 'node:assert/strict';
import { test } from 'node:test';
import { fmtTime, meanTime } from '../src/lib/timing.ts';

test('a time is written in the unit that fits it', () => {
  assert.equal(fmtTime(2_340_000), '2.34 s');
  assert.equal(fmtTime(23_400), '23.4 ms');
  assert.equal(fmtTime(1500), '1.50 ms');
  assert.equal(fmtTime(342), '342 µs');
  assert.equal(fmtTime(34.26), '34.3 µs');
  assert.equal(fmtTime(3.4), '3.40 µs');
  assert.equal(fmtTime(0.85), '850 ns');
});

test('the mean of coarse readings is the time', async () => {
  // a clock of 1 ms steps reads an evaluation of 30 µs as 0, and as 1000 once in about 33
  let n = 0;
  let clock = 0;
  const timing = await meanTime(
    async () => (++n % 100 < 3 ? 1000 : 0),
    () => (clock += 0.05),
  );
  assert.ok(timing);
  assert.equal(timing.runs, 3000);
  assert.equal(Math.round(timing.micros), 30);
});

test('the repeats stop at their budget, and a repeat that did not finish gives no timing', async () => {
  let clock = 0;
  const timing = await meanTime(
    async () => 4000,
    () => (clock += 10),
  );
  assert.deepEqual(timing, { micros: 4000, runs: 14 });
  assert.equal(await meanTime(async () => null), null);
});
