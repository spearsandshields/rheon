/**
 * The keys of the context that an expression reads: the engine's `paths` answer (docs/CONTRACT.md
 * §3) laid on the syntax tree of the context's JSON text. Pure: `test/reads.test.ts` runs it on
 * real engine answers.
 */
import type { ReadPath } from '@rheon/feel-runtime';

/** A node of the JSON editor's syntax tree (Lezer's JSON grammar): what this module reads of it. */
export interface JsonNode {
  name: string;
  from: number;
  to: number;
  firstChild: JsonNode | null;
  nextSibling: JsonNode | null;
}

const VALUES = new Set(['Object', 'Array', 'String', 'Number', 'True', 'False', 'Null']);

function children(node: JsonNode): JsonNode[] {
  const out: JsonNode[] = [];
  for (let c = node.firstChild; c; c = c.nextSibling) out.push(c);
  return out;
}

/** FEEL reads an entry of a list as the entry of each of its items, also of lists in lists. */
const items = (node: JsonNode): JsonNode[] => (node.name === 'Array' ? children(node).filter((c) => VALUES.has(c.name)).flatMap(items) : [node]);

/** The key a property name stands for: its JSON string without white space at its ends, as the engine reads a key. */
function keyOf(name: JsonNode, text: string): string | null {
  try {
    return (JSON.parse(text.slice(name.from, name.to)) as string).trim();
  } catch {
    return null;
  }
}

/**
 * The places of the keys whose values the expression reads whole: of `loans[].amount` the key
 * `amount` in every item of `loans`. A key the expression only passes through (`applicant` of
 * `applicant.age`) is not marked; one the context does not have is not either.
 */
export function readKeys(paths: ReadPath[], top: JsonNode, text: string): { from: number; to: number }[] {
  const root = children(top).find((c) => VALUES.has(c.name));
  const marked = new Map<number, { from: number; to: number }>();
  for (const { path, whole } of paths) {
    if (!whole || !root) continue;
    let values = [root];
    let keys: JsonNode[] = [];
    for (const segment of path) {
      if (Array.isArray(segment)) {
        values = values.flatMap(items);
        continue;
      }
      const next: JsonNode[] = [];
      keys = [];
      for (const object of values.flatMap(items).filter((v) => v.name === 'Object')) {
        for (const property of children(object).filter((c) => c.name === 'Property')) {
          const name = children(property).find((c) => c.name === 'PropertyName');
          if (!name || keyOf(name, text) !== segment) continue;
          keys.push(name);
          const value = children(property).find((c) => VALUES.has(c.name));
          if (value) next.push(value);
        }
      }
      values = next;
    }
    for (const key of keys) marked.set(key.from, { from: key.from, to: key.to });
  }
  return [...marked.values()].sort((a, b) => a.from - b.from);
}
