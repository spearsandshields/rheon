/**
 * Engine adapter for the playground: the rheon FEEL engine (`dist/feel.wasm`,
 * Rust → wasm32-wasip1, pure-Rust decimal128 numbers) running in a Web Worker via
 * `@rheon/feel-runtime` (docs/CONTRACT.md §4).
 *
 * The UI only talks to the `PlaygroundEngine` interface below.
 */
import { FeelEngine, rawJson, type Dialect, type ParseOptions, type ParseResponse, type RequestOptions } from '@rheon/feel-runtime';
// Built by `npm run build:wasm` (scripts/build-wasm.sh). Vite copies it into the
// build output as a hashed asset; `vite.config.ts` fails early if it is missing.
import feelWasmUrl from '../../../dist/feel.wasm?url';

export type {
  Dialect,
  RawJson,
  EvaluateRequest,
  EvaluateResponse,
  ParseResponse,
  FeelResult,
  ErrorKind,
  Outline,
  ExplainResponse,
  ExpandEntry,
  TraceRecord,
  ReadPath,
} from '@rheon/feel-runtime';
export { rawJson };

/** Per-request time budget; the worker is terminated and restarted after it. */
export const TIMEOUT_MS = 2000;

export interface EngineInfo {
  engine: string;
  engineCommit?: string;
  upstreamCommit?: string;
  numberBackend: string;
  version?: string;
}

/** What the playground needs from an engine (a subset of `FeelEngine`). */
export interface PlaygroundEngine {
  evaluate: FeelEngine['evaluate'];
  /** Evaluates an expression and records how its value came about. */
  explain: FeelEngine['explain'];
  parse(dialect: Dialect, expression: string, opts?: ParseOptions & RequestOptions): Promise<ParseResponse>;
  /** The editor operations: the tokens and errors of a text, what can stand at a cursor, what a name is. */
  syntax: FeelEngine['syntax'];
  complete: FeelEngine['complete'];
  hover: FeelEngine['hover'];
  format: FeelEngine['format'];
  /** The names of a context without its values. */
  outline: FeelEngine['outline'];
  /** What an expression reads of the names in scope, without evaluating it. */
  paths: FeelEngine['paths'];
  info(): Promise<EngineInfo>;
  dispose(): void;
  readonly timeoutMs: number;
  /** Set when the engine gave up restarting its worker; the page must be reloaded. */
  readonly unavailableReason: string | null;
}

export async function createEngine(): Promise<PlaygroundEngine> {
  const engine = await FeelEngine.create({ wasmUrl: feelWasmUrl, timeoutMs: TIMEOUT_MS });
  return {
    get timeoutMs() {
      return engine.timeoutMs;
    },
    get unavailableReason() {
      return engine.unavailableReason;
    },
    evaluate: (req, opts) => engine.evaluate(req, opts),
    explain: (req, opts) => engine.explain(req, opts),
    parse: (dialect, expression, opts) => engine.parse(dialect, expression, opts),
    syntax: (expression, opts) => engine.syntax(expression, opts),
    complete: (expression, cursor, opts) => engine.complete(expression, cursor, opts),
    hover: (expression, at, opts) => engine.hover(expression, at, opts),
    outline: (opts) => engine.outline(opts),
    paths: (expression, opts) => engine.paths(expression, opts),
    format: (expression, opts) => engine.format(expression, opts),
    async info() {
      const r = await engine.info();
      return {
        engine: r.engine ?? 'rheon',
        numberBackend: r.numberBackend ?? 'unknown',
        engineCommit: r.engineCommit,
        upstreamCommit: r.upstreamCommit,
        version: r.version,
      };
    },
    dispose: () => engine.dispose(),
  };
}
