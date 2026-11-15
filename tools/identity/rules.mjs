// Rule entries of the identity ledger: one entry that explains a class of differences, stated as
// relations between the reference's and the candidate's response at named paths (README.md,
// "Rule entries"). Pure functions on parsed responses (json.mjs), so they are checked on literal
// responses, whatever two modules answer.
import { canonical, isObject } from './json.mjs';

const RULE_FIELDS = new Set(['rule', 'why', 'applies', 'allow', 'examples']);
const ALLOW_FIELDS = { gains: ['each'], loses: ['each'], decreases: [], increases: [], rewrite: ['pattern', 'replacement'] };
const EXAMPLE_FIELDS = new Set(['expression', 'request', 'old', 'new']);
const SEGMENT = /^([^.[\]]+)(\[\*\])?$/;

/** The compiled regular expression of a rule field; a pattern that does not compile is an error. */
function regex(text, field, flags = 'u') {
  if (typeof text !== 'string' || text === '') throw new Error(`\`${field}\` must be a regular expression`);
  try {
    return new RegExp(text, flags);
  } catch (e) {
    throw new Error(`\`${field}\`: ${e.message}`);
  }
}

/** The segments of a path such as `diagnostics.count` or `warnings[*].message`: a key, and whether it stands for each element of an array. */
function segments(at) {
  if (typeof at !== 'string' || at === '') throw new Error('`at` must name a path');
  return at.split('.').map((part) => {
    const m = SEGMENT.exec(part);
    if (!m) throw new Error(`\`at\`: ${JSON.stringify(at)} is not a path (keys separated by dots, a key ending in [*] for each element of an array)`);
    return { key: m[1], each: m[2] !== undefined };
  });
}

/**
 * A rule entry of a ledger file, checked: `{ name, why, applies, allow, examples }`, where `applies`
 * is the compiled `expression` pattern or null, and each `allow` item holds its path segments, its
 * relation and the relation's compiled patterns. The examples are returned as written; the caller
 * registers and runs them. Throws an Error naming the first problem.
 */
export function checkRule(entry) {
  const unknown = Object.keys(entry).filter((k) => !RULE_FIELDS.has(k));
  if (unknown.length) throw new Error(`unknown field ${unknown.join(', ')}`);
  if (typeof entry.rule !== 'string' || !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(entry.rule)) throw new Error('`rule` must be a name of lowercase words joined by hyphens');
  if (typeof entry.why !== 'string' || entry.why.trim() === '') throw new Error('`why` must state the basis');
  let applies = null;
  if ('applies' in entry) {
    if (!isObject(entry.applies) || canonical(Object.keys(entry.applies)) !== '["expression"]') throw new Error('`applies` is {"expression": "<regex>"}');
    applies = regex(entry.applies.expression, 'applies.expression');
  }
  if (!Array.isArray(entry.allow) || entry.allow.length === 0) throw new Error('`allow` must list at least one path');
  const allow = entry.allow.map((item) => {
    if (!isObject(item)) throw new Error('an `allow` item is an object');
    const fields = ALLOW_FIELDS[item.change];
    if (!fields) throw new Error(`\`change\` must be one of ${Object.keys(ALLOW_FIELDS).join(', ')}`);
    const extra = Object.keys(item).filter((k) => k !== 'at' && k !== 'change' && !fields.includes(k));
    if (extra.length) throw new Error(`\`${item.change}\` takes no ${extra.join(', ')}`);
    const allowed = { at: item.at, path: segments(item.at), change: item.change };
    if (fields.includes('each')) allowed.each = regex(item.each, 'each');
    if (item.change === 'rewrite') {
      allowed.pattern = regex(item.pattern, 'pattern', 'gu');
      if (typeof item.replacement !== 'string') throw new Error('`replacement` must be a string');
      allowed.replacement = item.replacement;
    }
    return allowed;
  });
  const paths = allow.map((a) => a.at);
  if (new Set(paths).size !== paths.length) throw new Error('a path is allowed at most once');
  if (!Array.isArray(entry.examples) || entry.examples.length === 0) throw new Error('`examples` must hold at least one example');
  const rule = { name: entry.rule, why: entry.why, applies, allow };
  entry.examples.forEach((example, i) => {
    const where = `example #${i}`;
    if (!isObject(example)) throw new Error(`${where}: an example is an object`);
    const extra = Object.keys(example).filter((k) => !EXAMPLE_FIELDS.has(k));
    if (extra.length) throw new Error(`${where}: unknown field ${extra.join(', ')}`);
    if (('expression' in example) === ('request' in example)) throw new Error(`${where}: exactly one of \`expression\` and \`request\``);
    if ('expression' in example && typeof example.expression !== 'string') throw new Error(`${where}: \`expression\` must be a string`);
    if (!isObject(example.old) || !isObject(example.new)) throw new Error(`${where}: \`old\` and \`new\` are whole responses`);
    const request = 'expression' in example ? { op: 'evaluate', expression: example.expression } : example.request;
    if (!applicable(rule, request)) throw new Error(`${where}: the request does not match \`applies\``);
    const verdict = explains(rule, example.old, example.new);
    if (verdict !== true) throw new Error(`${where}: not an instance of the rule (${verdict})`);
  });
  return rule;
}

/** Whether a rule may explain the difference of a request: it has no `applies`, or the request's expression matches it. */
function applicable(rule, request) {
  if (rule.applies === null) return true;
  return isObject(request) && typeof request.expression === 'string' && rule.applies.test(request.expression);
}

/** The first of the rules, in their order, that may explain the difference of a request and does; undefined when none does. */
export const explainedBy = (rules, request, oldResponse, newResponse) => rules.find((rule) => applicable(rule, request) && explains(rule, oldResponse, newResponse) === true);

const ABSENT = undefined;

/** The value pairs at a path of two values: one pair, or one per element where a segment ends in [*]; a string when the two do not line up. */
function pairs(a, b, path) {
  let current = [[a, b]];
  for (const { key, each } of path) {
    const next = [];
    for (const [x, y] of current) {
      const vx = isObject(x) ? x[key] : ABSENT;
      const vy = isObject(y) ? y[key] : ABSENT;
      if (!each) {
        next.push([vx, vy]);
        continue;
      }
      if (!Array.isArray(vx) || !Array.isArray(vy) || vx.length !== vy.length) return `${key}[*] needs two arrays of equal length`;
      vx.forEach((element, i) => next.push([element, vy[i]]));
    }
    current = next;
  }
  return current;
}

/** The text an element of a list is matched against: a string as it is, any other value as its canonical JSON. */
const elementText = (v) => (typeof v === 'string' ? v : canonical(v));

/** Whether `small` is a subsequence of `large`, with every other element of `large` matching `each`. */
function gainsOnly(small, large, each) {
  let i = 0;
  for (const element of large) {
    if (i < small.length && canonical(element) === canonical(small[i])) i++;
    else if (!each.test(elementText(element))) return `the element ${elementText(element).slice(0, 120)} does not match \`each\``;
  }
  return i === small.length ? true : 'the shorter list is not a subsequence of the longer one';
}

/** A JSON integer as a BigInt, an absent value as 0; null for anything else. */
function integer(v) {
  if (v === ABSENT) return 0n;
  if (!JSON.isRawJSON(v)) return null;
  const text = canonical(v);
  return /^-?(0|[1-9]\d*)$/.test(text) ? BigInt(text) : null;
}

/** A list, an absent value as the empty list; null for anything else. */
const list = (v) => (v === ABSENT ? [] : Array.isArray(v) ? v : null);

/** Whether the old and the new value at an allowed path stand in its relation: true, or the reason they do not. */
function relates(allowed, x, y) {
  switch (allowed.change) {
    case 'gains':
    case 'loses': {
      const [small, large] = allowed.change === 'gains' ? [list(x), list(y)] : [list(y), list(x)];
      if (small === null || large === null) return 'both values must be lists';
      return gainsOnly(small, large, allowed.each);
    }
    case 'decreases':
    case 'increases': {
      const [a, b] = [integer(x), integer(y)];
      if (a === null || b === null) return 'both values must be integers';
      return (allowed.change === 'decreases' ? b < a : b > a) ? true : `${a} to ${b} is not ${allowed.change === 'decreases' ? 'a decrease' : 'an increase'}`;
    }
    case 'rewrite': {
      if (typeof x !== 'string' || typeof y !== 'string') return 'both values must be strings';
      return x.replace(allowed.pattern, allowed.replacement) === y ? true : 'the rewritten old text is not the new text';
    }
  }
}

const REMOVED = Symbol('removed');

/**
 * A copy of a value without the values at a path. An object left empty by the removal is removed
 * too, so that a field whose every part is allowed compares equal to an absent one; an element of
 * an array keeps its place.
 */
function without(value, path) {
  if (path.length === 0) return REMOVED;
  if (!isObject(value)) return value;
  const [{ key, each }, ...rest] = path;
  if (!(key in value)) return value;
  const copy = { ...value };
  let child = value[key];
  if (each && Array.isArray(child)) child = child.map((element) => (rest.length === 0 ? null : without(element, rest))).map((e) => (e === REMOVED ? {} : e));
  else if (each) return value;
  else child = without(child, rest);
  if (child === REMOVED) delete copy[key];
  else copy[key] = child;
  return Object.keys(copy).length === 0 ? REMOVED : copy;
}

/**
 * Whether a rule explains the difference between two answers, each a parsed response: true, or the
 * reason it does not. Both must be response objects, not traps; at each allowed path the two
 * values stand in its relation; and without the allowed paths the two responses are identical.
 */
export function explains(rule, oldResponse, newResponse) {
  if (!isObject(oldResponse) || !isObject(newResponse)) return 'both answers must be responses';
  if ('trap' in oldResponse || 'trap' in newResponse) return 'a trap is never explained by a rule';
  let a = oldResponse;
  let b = newResponse;
  for (const allowed of rule.allow) {
    const found = pairs(oldResponse, newResponse, allowed.path);
    if (typeof found === 'string') return `${allowed.at}: ${found}`;
    for (const [x, y] of found) {
      const verdict = relates(allowed, x, y);
      if (verdict !== true) return `${allowed.at}: ${verdict}`;
    }
    a = without(a, allowed.path);
    b = without(b, allowed.path);
  }
  const rest = (v) => (v === REMOVED ? '{}' : canonical(v));
  return rest(a) === rest(b) ? true : 'the responses differ beyond the allowed paths';
}
