// Runs before `npm pack` / `npm publish` (package.json "prepack"): copies the built wasm module and
// the repository's license files into the package, so that the tarball is self-contained.
// `feel.wasm` must have been built first (`npm run build:wasm` at the repository root).
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const pkg = fileURLToPath(new URL('..', import.meta.url));
const root = fileURLToPath(new URL('../../..', import.meta.url));

const wasm = `${root}dist/feel.wasm`;
if (!existsSync(wasm)) {
  console.error(`prepack: ${wasm} is missing; run \`npm run build:wasm\` at the repository root first`);
  process.exit(1);
}
// The module must be a clean build of the commit being packed: its `info` op reports the commit
// it was built from (see wasm/feel-wasm/build.rs), and a stale or dirty module would carry a
// wrong stamp into the published package. RHEON_ALLOW_STALE_WASM=1 skips the check.
const { instantiateFeel } = await import(`${pkg}dist/index.js`);
const info = instantiateFeel(readFileSync(wasm)).call({ op: 'info' });
const head = execFileSync('git', ['-C', root, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (!process.env.RHEON_ALLOW_STALE_WASM && info.engineCommit !== head) {
  console.error(`prepack: ${wasm} was built from ${info.engineCommit}, but HEAD is ${head}; rebuild it (\`npm run build:wasm\`) or set RHEON_ALLOW_STALE_WASM=1`);
  process.exit(1);
}
copyFileSync(wasm, `${pkg}feel.wasm`);
for (const f of ['LICENSE', 'LICENSE-MIT', 'NOTICE']) copyFileSync(`${root}${f}`, `${pkg}${f}`);
console.log(`prepack: feel.wasm (${(statSync(wasm).size / 1024 / 1024).toFixed(2)} MiB, engine ${info.engineCommit}), LICENSE, LICENSE-MIT, NOTICE copied`);
