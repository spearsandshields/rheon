import type { Dialect } from '../engine';

/** Everything that is shareable through the URL. */
export interface ShareState {
  dialect: Dialect;
  expression: string;
  context: string;
  input: string;
  showTree: boolean;
  showTyped: boolean;
}

/** Plain hashes longer than this are deflate-compressed (`#z=...`). */
const COMPRESS_ABOVE = 1500;
/** Hashes longer than this (after compression) are not kept in the URL: too long to share. */
export const MAX_HASH_LENGTH = 32 * 1024;

// Readable, stable keys: #d=expression&e=...&c=...&i=...&tree=1&typed=0
function plainHash(s: ShareState): string {
  const p = new URLSearchParams();
  p.set('d', s.dialect);
  p.set('e', s.expression);
  if (s.context.trim() && s.context.trim() !== '{}') p.set('c', s.context);
  if (s.dialect === 'unaryTests') p.set('i', s.input);
  p.set('tree', s.showTree ? '1' : '0');
  p.set('typed', s.showTyped ? '1' : '0');
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
    showTree: p.has('tree') ? p.get('tree') === '1' : fallback.showTree,
    showTyped: p.get('typed') === '1',
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
      return 'The context must be a JSON object, e.g. { "x": 1 }';
    }
    return null;
  } catch (e) {
    return (e as Error).message.replace(/^JSON\.parse: /, '');
  }
}

/** Top-level keys of a JSON object (for FEEL completions); [] when invalid. */
export function topLevelKeys(text: string): string[] {
  try {
    const v: unknown = JSON.parse(text);
    return v && typeof v === 'object' && !Array.isArray(v) ? Object.keys(v) : [];
  } catch {
    return [];
  }
}

/** Formats an engine time in microseconds. */
export function fmtMicros(us: number): string {
  if (us < 1000) return `${us} µs`;
  if (us < 1_000_000) return `${(us / 1000).toFixed(us < 10_000 ? 2 : 1)} ms`;
  return `${(us / 1_000_000).toFixed(2)} s`;
}
