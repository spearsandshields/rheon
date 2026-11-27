import type { Dialect, Outline, PlaygroundEngine } from '../engine';
import { rawJson } from '../engine';
import { checkJson } from './share';

type Names = { names?: Record<string, Outline>; types?: Record<string, Outline> };

const isNames = (o: unknown): o is Names =>
  typeof o === 'object' && o !== null && !Array.isArray(o) && !('type' in o) && !('items' in o) && !('function' in o);

/**
 * The names the FEEL editor knows: those of the context, each replaced by its entry of the Types
 * outline when that has one, and the types of the Types outline. So Types describe names without
 * hiding the context's other names, which an evaluation reads.
 */
export function mergeOutline(fromContext: Outline | null, typed: Outline): Outline {
  if (!isNames(fromContext) || !isNames(typed)) return typed;
  return { ...typed, names: { ...fromContext.names, ...typed.names } } as Outline;
}

const DEBOUNCE_MS = 250;

/**
 * The Types text as the engine reads it. While the text is edited the last outline the engine
 * accepted stays, so the editor's marks do not flicker; a text the engine refuses says why.
 */
export class TypesCheck {
  /** The outline the engine last accepted for the Types text, laid over the context's names. */
  outline = $state<Outline | null>(null);
  #refused = $state<{ text: string; message: string } | null>(null);

  constructor(input: () => { text: string; context: string | null; dialect: Dialect; engine: PlaygroundEngine | null }) {
    $effect(() => {
      const { text, context, dialect, engine } = input();
      if (!text.trim()) [this.outline, this.#refused] = [null, null];
      if (!engine || !text.trim() || checkJson(text, true)) return;
      const timer = setTimeout(async () => {
        const fromContext = context === null ? null : await engine.outline({ context: rawJson(context), contextFormat: 'json', types: true });
        const merged = mergeOutline(fromContext?.ok ? fromContext.outline : null, JSON.parse(text) as Outline);
        const res = await engine.syntax('', { dialect, outline: merged });
        const now = input();
        if (now.text !== text || now.context !== context) return;
        if (res.ok) [this.outline, this.#refused] = [merged, null];
        else if (res.error.kind === 'context') [this.outline, this.#refused] = [null, { text, message: res.error.message }];
      }, DEBOUNCE_MS);
      return () => clearTimeout(timer);
    });
  }

  /** What is wrong with a Types text: its JSON, or what the engine said of the outline. */
  errorOf(text: string): string | null {
    if (!text.trim()) return null;
    return checkJson(text, true) ?? (this.#refused?.text === text ? this.#refused.message : null);
  }
}
