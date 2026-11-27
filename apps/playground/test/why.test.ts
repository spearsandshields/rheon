// The Why tree over real answers of the engine (dist/feel.wasm, run in this thread).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { abbreviate, nullOrigin, rows, valueText, type PartRow, type WhyRow } from '../src/lib/why.ts';

const root = new URL('../../../', import.meta.url);
const { instantiateFeel } = await import(new URL('packages/feel-runtime/dist/instance.js', root).href);
const feel = await instantiateFeel(await readFile(new URL('dist/feel.wasm', root)));

function explain(expression: string, context: object, expand: unknown[] = []) {
  const answer = feel.call({ op: 'explain', expression, context, expand });
  assert.equal(answer.ok, true, JSON.stringify(answer.error));
  return answer;
}

const parts = (all: WhyRow[]) => all.filter((r): r is PartRow => r.kind === 'part');
const row = (all: WhyRow[], label: string, item: string | null = null) => {
  const found = parts(all).find((r) => r.label === label && r.item === item);
  assert.ok(found, `no row ${label} ${item ?? ''} in ${parts(all).map((r) => `${r.item ?? ''} ${r.label}`).join(' | ')}`);
  return found;
};

const LOANS = `{
  total: sum(for l in loans return l.amount * l.rate),
  ok: applicant.age >= 18 and total < 500
}.ok`;
const CONTEXT = { applicant: { age: 34, income: 5200 }, loans: [{ amount: 1000, rate: 0.05 }, { amount: 2500 }] };

test('the null of the result is followed from the top down to where it is from', () => {
  const all = rows(explain(LOANS, CONTEXT).trace, LOANS, new Map());
  const trail = parts(all).filter((r) => r.trail);
  assert.deepEqual(
    trail.map((r) => r.label),
    ['{…}.ok', 'ok: applicant.age >= 18 and total < 500', 'total < 500', 'total', 'total: sum(…)'],
  );
  const origin = trail[trail.length - 1];
  assert.equal(origin.origin, true);
  assert.match(origin.note ?? '', /sum/);
  // each row of the trail is inside the one before it
  for (let k = 1; k < trail.length; k++) assert.ok(trail[k].depth > trail[k - 1].depth);
  // the entry that the name reads stands under the name, not beside the other entry
  assert.equal(all[row(all, 'total: sum(…)').parent], row(all, 'total'));
});

test('the trail line runs from each row of the way to the next', () => {
  const all = rows(explain(LOANS, CONTEXT).trace, LOANS, new Map());
  const top = row(all, '{…}.ok');
  const entry = row(all, 'ok: applicant.age >= 18 and total < 500');
  assert.equal(top.line?.down, true);
  assert.equal(entry.line?.elbow, top.depth);
  // the context between them is crossed by the line
  assert.equal(all[all.indexOf(entry) - 1].line?.through, top.depth);
  assert.equal(row(all, 'applicant.age >= 18').line?.through, entry.depth);
});

test('a loop opens into its iterations as items, and an item opens with its parts', () => {
  const first = rows(explain(LOANS, CONTEXT).trace, LOANS, new Map());
  const loop = row(first, 'for l in loans return l.amount * l.rate');
  assert.equal(loop.note, '2 iterations');
  assert.equal(loop.value, '[50, null]');
  assert.deepEqual([loop.hasParts, loop.canExpand, loop.open], [false, true, false]);

  let open = new Map([[loop.key, true]]);
  const second = rows(explain(LOANS, CONTEXT, [loop.place]).trace, LOANS, open);
  assert.equal(row(second, 'loans').value, '[{"amount": 1000, "rate": 0.05}, {"amount": 2500}]');
  const item1 = row(second, 'l.amount * l.rate', 'item 1');
  assert.equal(item1.value, '50');
  assert.deepEqual([item1.hasParts, item1.canExpand], [false, true]);
  assert.equal(row(second, 'l.amount * l.rate', 'item 2').isNull, true);

  open = new Map([...open, [item1.key, true]]);
  const third = rows(explain(LOANS, CONTEXT, [loop.place, item1.place]).trace, LOANS, open);
  const opened = row(third, 'l.amount * l.rate', 'item 1');
  assert.equal(opened.open, true);
  assert.deepEqual(
    parts(third)
      .filter((r) => third[r.parent] === opened)
      .map((r) => [r.label, r.value]),
    [
      ['l.amount', '1000'],
      ['l.rate', '0.05'],
    ],
  );
});

test('a null inside an iteration says which item it is from', () => {
  const text = `{
  cheap: every l in loans satisfies l.rate < 0.1,
  ok: applicant.age >= 18 and cheap
}.ok`;
  const answer = explain(text, CONTEXT);
  const origin = nullOrigin(answer.trace, text);
  assert.deepEqual(origin && { label: origin.label, where: origin.where }, { label: 'l.rate', where: 'in item 2 of loans' });
  assert.match(origin!.reason, /no entry rate/);
  const all = rows(answer.trace, text, new Map());
  const originRow = parts(all).find((r) => r.origin)!;
  assert.equal(originRow.label, 'l.rate');
  assert.equal(all[originRow.parent].item, 'item 2');
});

test('a result that is its own null has no line under it', () => {
  const answer = explain('"total: " + 42', {});
  assert.equal(nullOrigin(answer.trace, '"total: " + 42'), null);
  const [top] = parts(rows(answer.trace, '"total: " + 42', new Map()));
  assert.equal(top.origin, true);
  assert.equal(top.trail, true);
});

test('a value that is not null has no trail', () => {
  const all = rows(explain('1 + 2', {}).trace, '1 + 2', new Map());
  assert.equal(parts(all).some((r) => r.trail || r.origin || r.line), false);
});

test('a long loop shows its first items and a row that asks for the others', () => {
  const text = 'for i in 1..30 return i * i';
  const first = rows(explain(text, {}).trace, text, new Map());
  assert.equal(parts(first).filter((r) => r.item?.startsWith('item')).length, 20);
  const more = first.find((r) => r.kind === 'more')!;
  assert.equal(more.label, 'Show the other 10');
  const second = rows(explain(text, {}, [more.place]).trace, text, new Map());
  assert.equal(parts(second).filter((r) => r.item?.startsWith('item')).length, 30);
  assert.equal(second.some((r) => r.kind === 'more'), false);
});

test('an invocation of a function of the text opens into its body', () => {
  const text = `{
  fact: function(n) if n <= 1 then 1 else n * fact(n - 1),
  result: fact(3)
}.result`;
  let open = new Map<string, boolean>();
  let expand: unknown[] = [];
  const openRow = (label: string, item: string | null = null) => {
    const r = row(rows(explain(text, {}, expand).trace, text, open), label, item);
    open = new Map([...open, [r.key, true]]);
    expand = [...expand, r.place];
  };
  openRow('{…}');
  openRow('result: fact(3)');
  const all = rows(explain(text, {}, expand).trace, text, open);
  const body = row(all, 'if n <= 1 then 1 else n * fact(n - 1)', 'body');
  assert.equal(body.value, '6');
  const call = row(all, 'result: fact(3)');
  assert.equal(all[body.parent], call);
  // the argument comes before the body it is bound in
  assert.ok(all.indexOf(row(all, '3')) < all.indexOf(body));
  assert.equal(row(all, 'fact: function(n) if n <= 1 then 1 else n * fact(…)').value.startsWith('function'), true);
});

test('a closed row hides its parts; a part that came back without parts cannot open', () => {
  const answer = explain(LOANS, CONTEXT);
  const entry = row(rows(answer.trace, LOANS, new Map()), 'ok: applicant.age >= 18 and total < 500');
  const closed = rows(answer.trace, LOANS, new Map([[entry.key, false]]));
  assert.equal(row(closed, 'ok: applicant.age >= 18 and total < 500').open, false);
  assert.equal(parts(closed).some((r) => r.label === 'total < 500'), false);

  assert.equal(row(closed, '{…}.ok').canExpand, true);
  const loop = row(rows(answer.trace, LOANS, new Map()), 'for l in loans return l.amount * l.rate');
  // asked to open, and the answer has no parts for it: there are none
  const asked = row(rows(answer.trace, LOANS, new Map([[loop.key, true]])), 'for l in loans return l.amount * l.rate');
  assert.deepEqual([asked.canExpand, asked.open], [false, false]);
});

test('labels shorten long brackets and values stand on one line', () => {
  assert.equal(abbreviate('{\n  a: 1,\n  b: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]\n}.b'), '{…}.b');
  assert.equal(abbreviate('total: sum(for l in loans return l.amount * l.rate)'), 'total: sum(…)');
  assert.equal(abbreviate('f(")")'), 'f(")")');
  assert.equal(valueText({ display: '[\n  1,\n  2\n]', feel: '[1, 2]', type: 'list<number>', typed: null, isNull: false }), '[1, 2]');
  assert.equal(valueText({ display: '"2026-10-16"', feel: '2026-10-16', type: 'date', typed: null, isNull: false }), '2026-10-16');
  assert.equal(valueText({ summarized: true, type: 'list', isNull: false, count: 300 }), 'list of 300 items');
});
