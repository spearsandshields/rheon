// Runs before `npm pack` / `npm publish` (package.json "prepack"): copies the built wasm module and
// the repository's license files into the package, so that the tarball is self-contained.
// `feel.wasm` must have been built first (`npm run build:wasm` at the repository root).
import { copyFileSync, existsSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const pkg = fileURLToPath(new URL('..', import.meta.url));
const root = fileURLToPath(new URL('../../..', import.meta.url));

const wasm = `${root}dist/feel.wasm`;
if (!existsSync(wasm)) {
  console.error(`prepack: ${wasm} is missing; run \`npm run build:wasm\` at the repository root first`);
  process.exit(1);
}
copyFileSync(wasm, `${pkg}feel.wasm`);
for (const f of ['LICENSE', 'LICENSE-MIT', 'NOTICE']) copyFileSync(`${root}${f}`, `${pkg}${f}`);
console.log(`prepack: feel.wasm (${(statSync(wasm).size / 1024 / 1024).toFixed(2)} MiB), LICENSE, LICENSE-MIT, NOTICE copied`);
