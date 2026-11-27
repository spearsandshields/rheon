import { StateEffect, StateField, type Extension } from '@codemirror/state';
import { Decoration, EditorView, hoverTooltip, type DecorationSet } from '@codemirror/view';

/** A span of the text that the page marks: where a null is from, a lit part, a key that is read. */
export interface EditorMark {
  from: number;
  to: number;
  /** The CSS class of the span. */
  class: string;
  /** Shown when the pointer rests on the span. */
  tip?: { title: string; text: string };
}

const setMarks = StateEffect.define<EditorMark[]>();

interface Held {
  marks: EditorMark[];
  decorations: DecorationSet;
}

function decorate(marks: EditorMark[], length: number): DecorationSet {
  const ranges = marks.filter((m) => m.from < m.to && m.to <= length).map((m) => Decoration.mark({ class: m.class }).range(m.from, m.to));
  return Decoration.set(ranges, true);
}

const held = StateField.define<Held>({
  create: () => ({ marks: [], decorations: Decoration.none }),
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setMarks)) return { marks: effect.value, decorations: decorate(effect.value, tr.newDoc.length) };
    }
    if (!tr.docChanged) return value;
    // until the next answer, the marks move with the edits
    const marks = value.marks.map((m) => ({ ...m, from: tr.changes.mapPos(m.from, 1), to: tr.changes.mapPos(m.to, -1) }));
    return { marks, decorations: value.decorations.map(tr.changes) };
  },
  provide: (field) => EditorView.decorations.from(field, (value) => value.decorations),
});

const tips = hoverTooltip((view, pos) => {
  const mark = view.state.field(held).marks.find((m) => m.tip && m.from <= pos && pos <= m.to);
  if (!mark?.tip) return null;
  const { title, text } = mark.tip;
  return {
    pos: mark.from,
    end: mark.to,
    above: true,
    create: () => {
      const dom = document.createElement('div');
      dom.className = 'cm-mark-tip';
      dom.append(Object.assign(document.createElement('div'), { className: 'cm-mark-tip-title', textContent: title }));
      dom.append(Object.assign(document.createElement('div'), { textContent: text }));
      return { dom };
    },
  };
});

/** The extension that holds the marks; give them with `marksEffect`. */
export const marksField: Extension = [held, tips];

export const marksEffect = (marks: EditorMark[]) => setMarks.of(marks);
