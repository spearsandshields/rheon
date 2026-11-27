import type { Outline } from '@rheon/feel-runtime';

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
