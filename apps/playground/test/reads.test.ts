// The context keys an expression reads, from real `paths` answers of the engine.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { jsonLanguage } from '@codemirror/lang-json';
import { readKeys } from '../src/lib/reads.ts';

const root = new URL('../../../', import.meta.url);
const { instantiateFeel } = await import(new URL('packages/feel-runtime/dist/instance.js', root).href);
const feel = await instantiateFeel(await readFile(new URL('dist/feel.wasm', root)));

/** The texts of the keys that are marked, with the line each stands on. */
function marked(expression: string, context: string, dialect = 'expression'): string[] {
  const answer = feel.call({ op: 'paths', expression, dialect, context: JSON.parse(context) });
  assert.equal(answer.ok, true, JSON.stringify(answer.error));
  const tree = jsonLanguage.parser.parse(context);
  return readKeys(answer.paths, tree.topNode, context).map(({ from, to }) => `${context.slice(0, from).split('\n').length}:${context.slice(from, to)}`);
}

const CONTEXT = `{
  "applicant": { "age": 34, "income": 5200 },
  "loans": [
    { "amount": 1000, "rate": 0.05 },
    { "amount": 2500 }
  ]
}`;

test('the keys read are marked in every item of a list; a key passed through or not read is not', () => {
  const expression = `{
  total: sum(for l in loans return l.amount * l.rate),
  ok: applicant.age >= 18 and total < 500
}.ok`;
  assert.deepEqual(marked(expression, CONTEXT), ['2:"age"', '4:"amount"', '4:"rate"', '5:"amount"']);
});

test('a value read whole marks its key', () => {
  assert.deepEqual(marked('count(loans)', CONTEXT), ['3:"loans"']);
  assert.deepEqual(marked('applicant', CONTEXT), ['2:"applicant"']);
});

test('a filter reads the entries of the items', () => {
  assert.deepEqual(marked('loans[amount > 1500].rate', CONTEXT), ['4:"amount"', '4:"rate"', '5:"amount"']);
});

test('a key is read as the engine reads it: escapes decoded, white space at its ends dropped', () => {
  assert.deepEqual(marked('Monthly Salary * 12', '{ " Monthly Salary ": 10, "Tax\\u0020Rate": 0.2 }'), ['1:" Monthly Salary "']);
  assert.deepEqual(marked('Tax Rate', '{ "Tax\\u0020Rate": 0.2 }'), ['1:"Tax\\u0020Rate"']);
});

test('unary tests read the tested value, which is no key of the context', () => {
  assert.deepEqual(marked('> limit', '{ "limit": 5, "other": 1 }', 'unaryTests'), ['1:"limit"']);
});
