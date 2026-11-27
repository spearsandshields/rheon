import type { Dialect } from '../engine';

/** Everything that is shareable through the URL. */
export interface ShareState {
  dialect: Dialect;
  expression: string;
  context: string;
  input: string;
  /** The Types text: an outline of the names for the editor, as JSON. */
  outline: string;
  showTyped: boolean;
  /** The inspector's tab. */
  inspector: 'why' | 'tree';
}

/** Plain hashes longer than this are deflate-compressed (`#z=...`). */
const COMPRESS_ABOVE = 1500;
/** Hashes longer than this (after compression) are not kept in the URL: too long to share. */
export const MAX_HASH_LENGTH = 32 * 1024;

// Readable, stable keys: #d=expression&e=...&c=...&i=...&o=...&typed=0&tab=tree. Links of earlier
// pages may also have `tree`, which said whether the syntax tree was open: it is not read, the
// inspector opens on Why.
function plainHash(s: ShareState): string {
  const p = new URLSearchParams();
  p.set('d', s.dialect);
  p.set('e', s.expression);
  if (s.context.trim() && s.context.trim() !== '{}') p.set('c', s.context);
  if (s.dialect === 'unaryTests') p.set('i', s.input);
  if (s.outline.trim()) p.set('o', s.outline);
  p.set('typed', s.showTyped ? '1' : '0');
  if (s.inspector === 'tree') p.set('tab', 'tree');
  return '#' + p.toString();
}

const canCompress = () => typeof CompressionStream === 'function' && typeof DecompressionStream === 'function';

async function pipe(bytes: Uint8Array, stream: CompressionStream | DecompressionStream): Promise<Uint8Array> {
  const out = new Response(new Blob([bytes as BlobPart]).stream().pipeThrough(stream));
  return new Uint8Array(await out.arrayBuffer());
}

function toBase64Url(bytes: Uint8Array): string {
  let bin = '';
  for (let i = 0; i < bytes.length; i += 0x8000) bin += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function fromBase64Url(text: string): Uint8Array {
  const bin = atob(text.replace(/-/g, '+').replace(/_/g, '/'));
  return Uint8Array.from(bin, (c) => c.charCodeAt(0));
}

/**
 * Encodes the state as a URL hash: readable `#d=...&e=...` when short, otherwise
 * `#z=<base64url(deflate-raw(readable params))>`.
 */
export async function encodeState(s: ShareState): Promise<string> {
  const plain = plainHash(s);
  if (plain.length <= COMPRESS_ABOVE || !canCompress()) return plain;
  const packed = await pipe(new TextEncoder().encode(plain.slice(1)), new CompressionStream('deflate-raw'));
  const compressed = '#z=' + toBase64Url(packed);
  return compressed.length < plain.length ? compressed : plain;
}

/** Result of reading the URL: the state, plus a notice when the link could not be (fully) used. */
export interface DecodedState {
  state: ShareState;
  notice: string | null;
}

function fromParams(p: URLSearchParams, fallback: ShareState): ShareState {
  const d = p.get('d');
  return {
    dialect: d === 'unaryTests' ? 'unaryTests' : 'expression',
    expression: p.get('e') ?? '',
    context: p.get('c') ?? '{}',
    input: p.get('i') ?? fallback.input,
    outline: p.get('o') ?? '',
    showTyped: p.get('typed') === '1',
    inspector: p.get('tab') === 'tree' ? 'tree' : 'why',
  };
}

/**
 * Reads the state from the URL hash: readable `#d=...&e=...` params or the compressed `#z=...`
 * form written by `encodeState`. Anything else gives the fallback state and a notice.
 */
export async function decodeState(hash: string, fallback: ShareState): Promise<DecodedState> {
  const raw = hash.replace(/^#/, '');
  if (raw) {
    const p = new URLSearchParams(raw);
    if (p.has('z')) {
      if (!canCompress()) return { state: fallback, notice: 'This browser cannot read compressed links.' };
      try {
        const text = new TextDecoder().decode(await pipe(fromBase64Url(p.get('z') ?? ''), new DecompressionStream('deflate-raw')));
        return { state: fromParams(new URLSearchParams(text), fallback), notice: null };
      } catch {
        return { state: fallback, notice: 'The link is damaged (its compressed data could not be read).' };
      }
    }
    if (p.has('e')) return { state: fromParams(p, fallback), notice: null };
  }
  return { state: fallback, notice: raw ? 'The link does not contain an expression; showing the default example.' : null };
}

/** Validates JSON text without ever using the parsed numbers. */
export function checkJson(text: string, mustBeObject: boolean): string | null {
  try {
    const v: unknown = JSON.parse(text);
    if (mustBeObject && (typeof v !== 'object' || v === null || Array.isArray(v))) {
      const found = Array.isArray(v) ? 'a list' : v === null ? 'null' : typeof v === 'string' ? 'a string' : `a ${typeof v}`;
      return `Expected a JSON object, such as { "x": 1 }, found ${found}.`;
    }
    return null;
  } catch (e) {
    return (e as Error).message.replace(/^JSON\.parse: /, '');
  }
}


/** The context text as an object when it is a JSON object, else an empty object. */
export function contextObject(text: string): Record<string, unknown> {
  try {
    const v: unknown = JSON.parse(text);
    return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {};
  } catch {
    return {};
  }
}

/** Formats an engine time in microseconds. */
export function fmtMicros(us: number): string {
  // The engine reads the browser's clock, which moves in steps of 0.1 ms to 1 ms: below 1 ms a
  // figure would be the step, not the time.
  if (us < 1000) return 'under 1 ms';
  if (us < 10_000) return `${(us / 1000).toFixed(1)} ms`;
  if (us < 1_000_000) return `${Math.round(us / 1000)} ms`;
  return `${(us / 1_000_000).toFixed(2)} s`;
}
