// The editor extension in a DOM (jsdom) with the engine in this thread: what a user sees after
// typing is what the engine answers for the text.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { JSDOM } from 'jsdom';

const dom = new JSDOM('<!doctype html><html><body></body></html>', { pretendToBeVisual: true });
for (const key of ['window', 'Window', 'document', 'navigator', 'MutationObserver', 'Node', 'Element', 'HTMLElement', 'Text', 'Range', 'DOMRect', 'getComputedStyle', 'requestAnimationFrame', 'cancelAnimationFrame', 'KeyboardEvent', 'MouseEvent']) {
  Object.defineProperty(globalThis, key, { value: dom.window[key], configurable: true, writable: true });
}
// jsdom has no layout: the editor measures nothing
dom.window.Range.prototype.getClientRects = () => [];
dom.window.Range.prototype.getBoundingClientRect = () => new dom.window.DOMRect(0, 0, 0, 0);

const { EditorView } = await import('@codemirror/view');
const { EditorState } = await import('@codemirror/state');
const { startCompletion, currentCompletions, completionStatus } = await import('@codemirror/autocomplete');
const { forEachDiagnostic } = await import('@codemirror/lint');
const { instantiateFeel } = await import('@rheon/feel-runtime');
const { feelEditor, formatDocument, hoverAt, hoverInfo, instanceSource, setFeelScope, signatureHint, syntaxOf } = await import('../dist/index.js');

const bytes = await readFile(new URL('../../../dist/feel.wasm', import.meta.url));
const engine = instanceSource(await instantiateFeel(bytes));

/** Waits until `condition` gives something. */
async function until(condition, what = 'the condition') {
  for (let waited = 0; waited < 3000; waited += 5) {
    const value = condition();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
  assert.fail(`${what} did not come`);
}

/** Lets the pending answers of the engine arrive. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 30));

const open = [];
function editor(doc, config = {}) {
  const view = new EditorView({
    parent: document.body,
    state: EditorState.create({ doc, selection: { anchor: doc.length }, extensions: [feelEditor({ engine, ...config })] }),
  });
  open.push(view);
  return view;
}
test.after(() => open.forEach((view) => view.destroy()));

/** The texts of the document that have the class. */
const marked = (view, cls) => [...view.contentDOM.querySelectorAll(`.${cls}`)].map((node) => node.textContent);
const type = (view, text) => view.dispatch({ changes: { from: view.state.doc.length, insert: text }, selection: { anchor: view.state.doc.length + text.length } });
const diagnostics = (view) => {
  const found = [];
  forEachDiagnostic(view.state, (d, from, to) => found.push({ from, to, message: d.message, severity: d.severity }));
  return found;
};
/** The completions at the cursor, asked for explicitly. */
async function completions(view) {
  startCompletion(view);
  await until(() => completionStatus(view.state) === 'active' || (completionStatus(view.state) === null && 'none'), 'the completion');
  return currentCompletions(view.state);
}

test('the tokens get the classes of the engine', async () => {
  const view = editor('if total > 10 then "big" else sum([1, 2]) // done', { context: { total: 1 } });
  await until(() => syntaxOf(view.state), 'the answer');
  assert.deepEqual(marked(view, 'cm-feel-keyword'), ['if', 'then', 'else']);
  assert.deepEqual(marked(view, 'cm-feel-string'), ['"big"']);
  assert.deepEqual(marked(view, 'cm-feel-number'), ['10', '1', '2']);
  assert.deepEqual(marked(view, 'cm-feel-comment'), ['// done']);
  assert.deepEqual(marked(view, 'cm-feel-name'), ['total', 'sum']);
  assert.equal(syntaxOf(view.state).nodes, undefined);
});

test('after an edit the colours are those of the new text', async () => {
  const view = editor('1');
  await until(() => syntaxOf(view.state));
  type(view, ' + "a"');
  // until the answer arrives, the answer held is the one for the text before
  await until(() => marked(view, 'cm-feel-string').length === 1, 'the colours of the new text');
  assert.deepEqual(marked(view, 'cm-feel-string'), ['"a"']);
  assert.equal(syntaxOf(view.state).tokens.at(-1)[2], 7);
});

test('an answer for a text that has changed is dropped', async () => {
  const asked = [];
  const slow = { ...engine, syntax: (text, opts) => new Promise((resolve) => asked.push({ text, resolve: () => resolve(engine.syntax(text, opts)) })) };
  const taken = [];
  const view = editor('1', { engine: slow, onSyntax: (syntax) => taken.push(syntax.tokens.length) });
  await until(() => asked.length === 1);
  type(view, ' + 2');
  await until(() => asked.length === 2);
  // the first answer arrives after the edit: not taken
  asked[0].resolve();
  await settle();
  assert.equal(syntaxOf(view.state), null);
  assert.deepEqual(taken, []);
  asked[1].resolve();
  await until(() => syntaxOf(view.state));
  assert.deepEqual(taken, [5]);
});

test('syntax errors are marked with the messages of the engine', async () => {
  const view = editor('if a then');
  const found = await until(() => (diagnostics(view).length ? diagnostics(view) : null), 'the error mark');
  assert.equal(found.length, 1);
  assert.match(found[0].message, /^Expected an expression after `then`/);
  assert.deepEqual([found[0].from, found[0].to], [8, 9]);
  // corrected: the mark goes
  type(view, ' 1 else 2');
  await until(() => diagnostics(view).length === 0, 'the end of the error mark');
});

test('maxDiagnostics limits the marks', async () => {
  const view = editor('1 + + + ', { maxDiagnostics: 0 });
  await until(() => syntaxOf(view.state));
  await settle();
  assert.ok(syntaxOf(view.state).diagnostics.length > 0);
  assert.deepEqual(diagnostics(view), []);
});

test('tree: true holds the nodes of the text', async () => {
  const view = editor('1 + 2', { tree: true });
  const syntax = await until(() => syntaxOf(view.state));
  assert.deepEqual(syntax.nodes.map(([kind]) => kind).slice(0, 2), ['Addition', 'Number']);
});

test('completion offers the names in scope and replaces what was typed', async () => {
  const view = editor('1 + Monthly Sal', { context: { 'Monthly Salary': 1, other: 2 } });
  const options = await completions(view);
  assert.deepEqual(options.map((option) => option.label), ['Monthly Salary']);
  options[0].apply(view, options[0], 4, 15);
  assert.equal(view.state.doc.toString(), '1 + Monthly Salary');
});

test('a function is completed with its parentheses and shows its signature', async () => {
  const view = editor('subs');
  const options = await completions(view);
  const substring = options.find((option) => option.label === 'substring');
  assert.equal(substring.detail, 'substring(string, start position, length?): string');
  assert.equal(substring.info, 'The characters from a position, as many as the length says or all that follow.');
  substring.apply(view, substring, 0, 4);
  assert.equal(view.state.doc.toString(), 'substring()');
  assert.equal(view.state.selection.main.head, 10);
});

test('a parameter is completed with its colon', async () => {
  const view = editor('substring(string: "abc", st');
  const options = await completions(view);
  assert.equal(options[0].label, 'start position');
  options[0].apply(view, options[0], 25, 27);
  assert.equal(view.state.doc.toString(), 'substring(string: "abc", start position: ');
});

test('templates are offered where an expression starts, unless turned off', async () => {
  const withTemplates = await completions(editor('fo'));
  assert.ok(withTemplates.some((option) => option.label === 'for … in … return …'));
  const without = await completions(editor('fo', { templates: false }));
  assert.ok(without.length > 0 && !without.some((option) => option.label === 'for … in … return …'));
});

test('setFeelScope changes the names and the dialect', async () => {
  const view = editor('am', { context: { amount: 1 } });
  assert.deepEqual((await completions(view)).map((option) => option.label), ['amount']);
  view.dispatch({ effects: setFeelScope({ context: { ample: 1 } }) });
  await settle();
  assert.deepEqual((await completions(view)).map((option) => option.label), ['ample']);
  // unary tests: a list of tests is a text without errors
  const tests = editor('1, 2');
  await until(() => syntaxOf(tests.state)?.diagnostics.length > 0, 'the error of an expression');
  tests.dispatch({ effects: setFeelScope({ dialect: 'unaryTests' }) });
  await until(() => syntaxOf(tests.state)?.diagnostics.length === 0, 'the answer for unary tests');
});

test('formatDocument gives the text its layout', async () => {
  const view = editor('{a:1,b:[1,2]}');
  assert.equal(formatDocument(view), true);
  await until(() => view.state.doc.toString() !== '{a:1,b:[1,2]}', 'the formatted text');
  assert.equal(view.state.doc.toString(), '{\n  a: 1,\n  b: [1, 2]\n}');
  // a text with a syntax error stays
  const broken = editor('{a:1,');
  assert.equal(formatDocument(broken), true);
  await settle();
  assert.equal(broken.state.doc.toString(), '{a:1,');
  // the width of the configuration
  const narrow = editor('1111111111 + 2222222222 + 3333333333', { formatWidth: 20 });
  formatDocument(narrow);
  await until(() => narrow.state.doc.lines === 3, 'the broken lines');
  // an engine that does not format
  const plain = editor('1+1', { engine: { syntax: engine.syntax, complete: engine.complete } });
  assert.equal(formatDocument(plain), false);
});

test('the hint of the invoked function follows the cursor', async () => {
  const view = editor('substring("abc", ');
  const hint = await until(() => view.dom.querySelector('.cm-feel-signature'), 'the hint');
  assert.equal(hint.querySelector('.cm-feel-signature-line').textContent, 'substring(string: string, start position: number, length?: number): string');
  assert.equal(hint.querySelector('.cm-feel-signature-active').textContent, 'start position: number');
  assert.equal(hint.querySelector('.cm-feel-signature-summary').textContent, 'The characters from a position, as many as the length says or all that follow.');
  // outside the parentheses the hint goes
  type(view, '1) + ');
  await until(() => !view.dom.querySelector('.cm-feel-signature'), 'the end of the hint');
  // turned off
  const off = editor('substring(', { signatureHelp: false });
  await until(() => syntaxOf(off.state));
  await settle();
  assert.equal(off.dom.querySelector('.cm-feel-signature'), null);
});

test('the hint shows the signatures that fit the argument', () => {
  const call = engine.complete('date(2020, ', 11, {}).call;
  const lines = [...signatureHint(call).querySelectorAll('.cm-feel-signature-line')].map((line) => line.textContent);
  assert.deepEqual(lines, ['date(year: number, month: number, day: number): date']);
  const first = engine.complete('date(', 5, {}).call;
  assert.equal(signatureHint(first).querySelectorAll('.cm-feel-signature-line').length, 2);
  // a rest parameter takes every further argument; named parameters are marked as given
  const rest = signatureHint(engine.complete('append([1], 2, 3, ', 18, {}).call);
  assert.equal(rest.querySelector('.cm-feel-signature-active').textContent, 'item...: Any');
  const named = signatureHint(engine.complete('substring(string: "a", ', 23, {}).call);
  assert.equal(named.querySelector('.cm-feel-signature-given').textContent, 'string: string');
  assert.equal(named.querySelector('.cm-feel-signature-active'), null);
});

test('a name shows its type and what the outline says of it', async () => {
  const outline = {
    names: {
      loan: { names: { amount: { type: 'number', description: 'In dollars' }, old: { type: 'string', deprecated: 'use amount' } } },
      lib: { names: { nz: { function: { parameters: [{ name: 'value', type: 'number' }, { name: 'fallback', type: 'number' }], result: 'number' }, description: 'The value, or the fallback' } } },
    },
  };
  const text = 'loan.amount + lib.nz(loan.old, 1) + floor(2)';
  const view = editor(text, { outline });
  const shown = async (word) => {
    const tooltip = await hoverAt(engine, view, text.indexOf(word) + 1);
    if (!tooltip) return null;
    assert.equal(text.slice(tooltip.pos, tooltip.end), word);
    return [...tooltip.create(view).dom.children].map((line) => `${line.className.replace('cm-feel-hover-', '')}: ${line.textContent}`);
  };
  assert.deepEqual(await shown('amount'), ['line: amount: number', 'documentation: In dollars']);
  assert.deepEqual(await shown('loan'), ['line: loan: context']);
  assert.deepEqual(await shown('old'), ['line: old: string', 'deprecated: Deprecated: use amount']);
  assert.deepEqual(await shown('nz'), ['line: nz(value: number, fallback: number): number', 'documentation: The value, or the fallback']);
  assert.deepEqual(await shown('floor'), ['line: floor(n: number, scale?: number): number', 'documentation: The number rounded towards negative infinity.']);
  // no name at the place, an engine that does not answer hover, a text that has changed since
  assert.equal(await hoverAt(engine, view, text.indexOf('+')), null);
  assert.equal(await hoverAt({ ...engine, hover: undefined }, view, 1), null);
  const slow = { ...engine, hover: async (...args) => (await settle(), engine.hover(...args)) };
  const pending = hoverAt(slow, view, 1);
  type(view, ' ');
  assert.equal(await pending, null);
  // the element alone: a name without a type, deprecated without a reason
  assert.equal(hoverInfo({ from: 0, to: 1, kind: 'name', type: null, deprecated: true }, 'a').textContent, 'aDeprecated.');
});

test('with check, what is certainly wrong with a text that parses is marked as a warning or a hint', async () => {
  const outline = { names: { loan: { names: { amount: { type: 'number' }, old: { type: 'number', deprecated: 'use amount' } } } } };
  const view = editor('loan.amout + loan.old + rate', { outline, check: true });
  const found = await until(() => (diagnostics(view).length === 3 ? diagnostics(view) : null), 'the marks of the check');
  assert.deepEqual(
    found.map((d) => [d.severity, d.from, d.to]),
    [
      ['warning', 0, 10],
      ['hint', 13, 21],
      ['warning', 24, 28],
    ],
  );
  assert.match(found[0].message, /^Expected an entry that `loan` has, found `amout`/);
  assert.match(found[1].message, /^`loan.old` is deprecated: use amount\.$/);
  // an error of the text is an error, and the text is not checked while it has one
  type(view, ' +');
  const broken = await until(() => (diagnostics(view).length === 1 ? diagnostics(view) : null), 'the error mark');
  assert.equal(broken[0].severity, 'error');
  // without check nothing but errors is marked
  const plain = editor('loan.amout + rate', { outline });
  await until(() => syntaxOf(plain.state));
  await settle();
  assert.deepEqual(diagnostics(plain), []);
});
