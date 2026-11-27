import type { ExpandEntry } from '@rheon/feel-runtime';
import type { MoreRow, PartRow } from './why';

/** The node an entry of `expand` names: its place and where it is evaluated. */
const nodeOf = (e: ExpandEntry) => (Array.isArray(e) ? `${e[0]}-${e[1]}/[]` : `${e.at[0]}-${e.at[1]}/${JSON.stringify(e.within ?? [])}`);

/**
 * What the reader opened in the Why tree. The engine keeps nothing between requests, so every
 * request names all the nodes whose parts are wanted (`expand`).
 */
export class WhyState {
  /** The nodes whose parts the engine is asked to record. */
  expand = $state<ExpandEntry[]>([]);
  /** The rows the reader opened (true) or closed (false). */
  open = $state<ReadonlyMap<string, boolean>>(new Map());
  /** The part under the pointer or the focus, lit in the editor. */
  lit = $state<PartRow | null>(null);

  /** Another text: its places are other places. */
  reset() {
    this.expand = [];
    this.open = new Map();
    this.lit = null;
  }

  /** Opens or closes a part; true when the engine is to be asked for parts it has not recorded. */
  toggle(row: PartRow): boolean {
    this.open = new Map(this.open).set(row.key, !row.open);
    if (row.open || row.hasParts) return false;
    this.expand = [...this.expand, row.place];
    return true;
  }

  /** Asks for the next iterations of a loop, in place of what was asked of it before. */
  more(row: MoreRow) {
    const node = nodeOf(row.place);
    this.expand = [...this.expand.filter((e) => nodeOf(e) !== node), row.place];
  }
}
