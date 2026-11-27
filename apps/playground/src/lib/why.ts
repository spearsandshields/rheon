/**
 * The Why tree: the records of an `explain` answer (docs/CONTRACT.md §3) as flat display rows.
 *
 * A record's parent is the innermost record around it that is evaluated in the same place
 * (`within`), else, for a record inside an iteration or a function body, the record of that loop
 * or invocation. A record that a null is from (`nullFrom`) and that stands elsewhere in the text
 * (the entry that a name reads) is shown under the record whose null it explains, so the way of
 * a null reads from the top down to its origin.
 *
 * Pure: no DOM, no Svelte; `test/why.test.ts` runs it on real engine answers.
 */
import type { ExpandEntry, FeelResult, SummarizedValue, TraceRecord, TraceStep } from '@rheon/feel-runtime';

/** The trail line drawn through a row, in depths of the tree. */
export interface TrailLine {
  /** A vertical line at this depth runs through the row, from the trail row above to one below. */
  through?: number;
  /** The line comes down at this depth and turns into this row, the next row of the trail. */
  elbow?: number;
  /** The line leaves this row downwards, towards the next row of the trail. */
  down?: boolean;
}

/** A part of the text and its value. */
export interface PartRow {
  kind: 'part';
  key: string;
  depth: number;
  /** The text of the part on one line, long brackets shortened to `…`. */
  label: string;
  /** Of a part evaluated for each item of a loop or in a function's invocation: `item 2`, `body`. */
  item: string | null;
  value: string;
  isNull: boolean;
  note: string | null;
  /** On the way of the result's null, from the top down to where it is from. */
  trail: boolean;
  /** Where a null is from: the record says the null is its own. */
  origin: boolean;
  from: number;
  to: number;
  /** The entry of `expand` that asks for its parts. */
  place: ExpandEntry;
  /** Its parts are in the trace. */
  hasParts: boolean;
  /** It may have parts that the trace does not have yet. */
  canExpand: boolean;
  open: boolean;
  /** Index of the parent row; -1 at the top. */
  parent: number;
  line: TrailLine | null;
}

/** The last row of a loop whose iterations are not all in the trace: asks for the next ones. */
export interface MoreRow {
  kind: 'more';
  key: string;
  depth: number;
  label: string;
  place: ExpandEntry;
  parent: number;
  line: TrailLine | null;
}

export type WhyRow = PartRow | MoreRow;

/** How many more iterations one activation of a "more" row asks for. */
export const MORE_STEP = 20;
const LABEL_MAX = 44;

/** Kinds of nodes that have no parts: there is nothing to expand. */
const LEAVES = new Set(['Name', 'Number', 'String', 'Boolean', 'Null', 'Literal', 'Constant', 'Type', 'Comment']);

/** Why a null is a node's own, when the value has no reason of its own. */
const NULL_CODES: Record<NonNullable<TraceRecord['nullCode']>, string> = {
  'missing-entry': 'nothing gives this a value',
  'null-input': 'it is given null',
  operator: 'the operator gives null for these operands',
  function: 'the function gives null for these arguments',
  other: 'null here',
};

interface Node {
  r: TraceRecord;
  i: number;
  key: string;
  within: string;
  parent: Node | null;
  children: Node[];
}

const stepKey = (s: TraceStep) => ('call' in s ? `c${s.at[0]}-${s.at[1]}` : `i${s.at[0]}-${s.at[1]}#${s.iteration}`);
const withinKey = (w: TraceStep[] | undefined) => (w ?? []).map(stepKey).join('/');

/** The identity of a record across answers: its place, its kind and where it is evaluated. */
export const keyOf = (r: TraceRecord) => `${r.kind}@${r.from}-${r.to}/${withinKey(r.within)}`;

const isSummary = (v: FeelResult | SummarizedValue): v is SummarizedValue => 'summarized' in v;
const isNull = (v: FeelResult | SummarizedValue) => v.isNull;
export const isTemporal = (type: string) => /date|time|duration/.test(type) && !type.startsWith('list') && !type.startsWith('context');

/** A value on one line: temporal values unquoted (their FEEL text), a long one by its size. */
export function valueText(v: FeelResult | SummarizedValue): string {
  if (isSummary(v)) {
    if (v.isNull) return 'null';
    if (v.count === undefined) return v.type;
    if (v.type === 'list') return `list of ${v.count} ${v.count === 1 ? 'item' : 'items'}`;
    return `context of ${v.count} ${v.count === 1 ? 'entry' : 'entries'}`;
  }
  // a temporal value and a function are written as quoted strings; they are no strings
  const quoted = v.display.startsWith('"');
  const text = quoted && isTemporal(v.type) ? v.feel : quoted && v.type.startsWith('function') ? (JSON.parse(v.display) as string) : v.display;
  return text
    .replace(/([[{])\n\s*/g, '$1')
    .replace(/\n\s*([\]}])/g, '$1')
    .replace(/\n\s*/g, ' ');
}

/** The groups of brackets at the outer level of a text, strings skipped: `[open, close]`. */
function outerGroups(text: string): [number, number][] {
  const groups: [number, number][] = [];
  let depth = 0;
  let start = -1;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '"') {
      for (i++; i < text.length && text[i] !== '"'; i++) if (text[i] === '\\') i++;
    } else if (c === '(' || c === '[' || c === '{') {
      if (depth++ === 0) start = i;
    } else if ((c === ')' || c === ']' || c === '}') && depth > 0) {
      if (--depth === 0) groups.push([start, i]);
    }
  }
  return groups;
}

/** A part's text on one line; while it is long, the longest bracket at the outer level holds `…`. */
export function abbreviate(text: string, max = LABEL_MAX): string {
  let t = text.replace(/\s+/g, ' ').trim();
  while (t.length > max) {
    const longest = outerGroups(t)
      .filter(([a, b]) => b - a > 2)
      .sort((x, y) => y[1] - y[0] - (x[1] - x[0]))[0];
    if (!longest) break;
    t = `${t.slice(0, longest[0] + 1)}…${t.slice(longest[1])}`;
  }
  return t;
}

const contains = (outer: { from: number; to: number }, inner: { from: number; to: number }) => outer.from <= inner.from && inner.to <= outer.to;
const span = (n: Node) => n.r.to - n.r.from;

/** The innermost of the nodes that contain a place; of nodes with one place the later one. */
function innermost(candidates: Node[], place: { from: number; to: number }, except?: Node): Node | null {
  let best: Node | null = null;
  for (const m of candidates) {
    if (m === except || !contains(m.r, place)) continue;
    if (except && span(m) === span(except) && m.i > except.i) continue;
    if (!best || span(m) < span(best) || (span(m) === span(best) && m.i > best.i)) best = m;
  }
  return best;
}

const isAncestor = (a: Node, b: Node) => {
  for (let n = b.parent; n; n = n.parent) if (n === a) return true;
  return false;
};

interface Tree {
  roots: Node[];
  /** The way of the first record's null (the root of the text) down to where it is from. */
  trail: Node[];
}

/** The records as a tree, with the way of the root's null. */
function build(trace: TraceRecord[]): Tree {
  const nodes: Node[] = trace.map((r, i) => ({ r, i, key: keyOf(r), within: withinKey(r.within), parent: null, children: [] }));
  const byWithin = new Map<string, Node[]>();
  for (const n of nodes) byWithin.set(n.within, [...(byWithin.get(n.within) ?? []), n]);
  const byPlace = new Map(nodes.map((n) => [`${n.r.from}-${n.r.to}/${n.within}`, n]));
  const nullFrom = (n: Node) => n.r.nullFrom && byPlace.get(`${n.r.nullFrom.at[0]}-${n.r.nullFrom.at[1]}/${withinKey(n.r.nullFrom.within)}`);

  for (const n of nodes) {
    let parent = innermost(byWithin.get(n.within) ?? [], n.r, n);
    const steps = n.r.within ?? [];
    if (!parent && steps.length) {
      const step = steps[steps.length - 1];
      parent = innermost(byWithin.get(withinKey(steps.slice(0, -1))) ?? [], { from: step.at[0], to: step.at[1] });
    }
    n.parent = parent;
  }

  const trail: Node[] = [];
  for (let n: Node | undefined = nodes[0]; n && !trail.includes(n) && isNull(n.r.value); n = nullFrom(n)) trail.push(n);

  // A part that a null is from and that stands elsewhere in the text (the entry a name reads) moves
  // under the record whose null it explains; the root's way goes last, so it wins a shared part.
  const onTrail = new Set(trail);
  for (const n of [...nodes.filter((m) => !onTrail.has(m)), ...trail]) {
    const target = nullFrom(n);
    if (!target || target === n || target.parent === n || isAncestor(n, target) || isAncestor(target, n)) continue;
    target.parent = n;
  }
  const roots: Node[] = [];
  for (const n of nodes) (n.parent ? n.parent.children : roots).push(n);
  return { roots, trail };
}

/** The last step of a record's `within`, when it is one step deeper than its parent's: the iteration or the invocation it is in. */
function ownStep(n: Node): TraceStep | null {
  const steps = n.r.within ?? [];
  const parentSteps = n.parent?.r.within?.length ?? 0;
  return steps.length === parentSteps + 1 ? steps[steps.length - 1] : null;
}

const placeOf = (r: TraceRecord): ExpandEntry => (r.within?.length ? { at: [r.from, r.to], within: r.within } : [r.from, r.to]);

function noteOf(n: Node, origin: boolean): string | null {
  const v = n.r.value;
  if (n.r.nullOrigin || origin) return (!isSummary(v) && v.nullReason) || (n.r.nullCode ? NULL_CODES[n.r.nullCode] : null);
  if (n.r.iterations !== undefined) {
    const count = n.r.iterations;
    return n.r.kind === 'Filter' ? `${count} ${count === 1 ? 'item' : 'items'} tested` : `${count} ${count === 1 ? 'iteration' : 'iterations'}`;
  }
  return null;
}

/**
 * The rows of the Why tree. `open` holds the rows the reader opened (true) or closed (false);
 * other rows are open when their parts are in the trace.
 */
export function rows(trace: TraceRecord[], source: string, open: ReadonlyMap<string, boolean>): WhyRow[] {
  const { roots, trail } = build(trace);
  const onTrail = new Set(trail);
  const origin = trail.length && trail[trail.length - 1].r.nullOrigin ? trail[trail.length - 1] : null;
  const out: WhyRow[] = [];
  const indexOf = new Map<Node, number>();

  const visit = (n: Node, depth: number, parent: number) => {
    const hasParts = n.children.length > 0;
    const step = ownStep(n);
    const asked = open.get(n.key);
    const isOpen = hasParts && (asked ?? true);
    indexOf.set(n, out.length);
    out.push({
      kind: 'part',
      key: n.key,
      depth,
      label: abbreviate(source.slice(n.r.from, n.r.to)),
      item: step ? ('call' in step ? 'body' : `item ${step.iteration + 1}`) : null,
      value: valueText(n.r.value),
      isNull: isNull(n.r.value),
      note: noteOf(n, n === origin),
      trail: onTrail.has(n),
      origin: n === origin,
      from: n.r.from,
      to: n.r.to,
      place: placeOf(n.r),
      hasParts,
      // a node the reader opened that came back without parts has none
      canExpand: hasParts || (!LEAVES.has(n.r.kind) && asked !== true),
      open: isOpen,
      parent,
      line: null,
    });
    if (!isOpen) return;
    const me = indexOf.get(n)!;
    // an invocation's arguments, then the body it runs
    const isBody = (c: Node) => !!ownStep(c) && 'call' in ownStep(c)!;
    for (const c of [...n.children.filter((c) => !isBody(c)), ...n.children.filter(isBody)]) visit(c, depth + 1, me);
    // a loop of more iterations than the trace has: a row asks for the next ones
    const shown = n.children.filter((c) => ownStep(c) && !('call' in ownStep(c)!)).length;
    if (n.r.iterations !== undefined && shown > 0 && shown < n.r.iterations) {
      const end = Math.min(n.r.iterations, shown + MORE_STEP);
      const rest = n.r.iterations - shown;
      out.push({
        kind: 'more',
        key: `${n.key}/more`,
        depth: depth + 1,
        label: end - shown < rest ? `Show ${end - shown} more (${rest} not shown)` : rest === 1 ? 'Show the other item' : `Show the other ${rest}`,
        place: { at: [n.r.from, n.r.to], ...(n.r.within?.length ? { within: n.r.within } : {}), iterations: [0, end] },
        parent: me,
        line: null,
      });
    }
  };
  roots.forEach((n) => visit(n, 0, -1));

  // the trail line, between rows of the trail that are both shown
  for (let k = 0; k + 1 < trail.length; k++) {
    const a = indexOf.get(trail[k]);
    const b = indexOf.get(trail[k + 1]);
    if (a === undefined || b === undefined || b < a) break;
    const depth = out[a].depth;
    out[a].line = { ...out[a].line, down: true };
    for (let j = a + 1; j < b; j++) out[j].line = { ...out[j].line, through: depth };
    out[b].line = { ...out[b].line, elbow: depth };
  }
  return out;
}

/** Where a null is from, for the line under the result: the origin's place, its text, where it is evaluated, why. */
export interface NullOrigin {
  from: number;
  to: number;
  label: string;
  where: string | null;
  reason: string;
}

/**
 * The origin of the result's null when the result is not itself where its null is from: the
 * record with `nullOrigin` at the end of the way that `nullFrom` leads from the root.
 */
export function nullOrigin(trace: TraceRecord[], source: string): NullOrigin | null {
  const { trail } = build(trace);
  const last = trail[trail.length - 1];
  if (trail.length < 2 || !last.r.nullOrigin) return null;
  return {
    from: last.r.from,
    to: last.r.to,
    label: abbreviate(source.slice(last.r.from, last.r.to), 32),
    where: whereOf(last.r, trace, source),
    reason: noteOf(last, true) ?? NULL_CODES.other,
  };
}

/** Where a record is evaluated, said by its innermost step: `in item 2 of loans`, `in the call fact(n - 1)`. */
function whereOf(r: TraceRecord, trace: TraceRecord[], source: string): string | null {
  const steps = r.within ?? [];
  const step = steps[steps.length - 1];
  if (!step) return null;
  const at = { from: step.at[0], to: step.at[1] };
  if ('call' in step) return `in the call ${abbreviate(source.slice(at.from, at.to), 28)}`;
  // the loop's domain: its first part evaluated outside the iterations
  const outer = withinKey(steps.slice(0, -1));
  const domain = trace.find((d) => withinKey(d.within) === outer && contains(at, d) && !(d.from === at.from && d.to === at.to));
  const item = `in item ${step.iteration + 1}`;
  return domain ? `${item} of ${abbreviate(source.slice(domain.from, domain.to), 24)}` : item;
}
