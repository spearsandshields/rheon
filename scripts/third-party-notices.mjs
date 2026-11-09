#!/usr/bin/env node
// Writes apps/playground/public/third-party-notices.txt: the license text of every package that
// ends up in the playground's browser bundle (the playground's production dependency tree) and of
// every Rust crate compiled into feel.wasm (the wasm32-wasip1 dependency graph of
// rheon-feel-wasm, workspace crates excluded; their notices are the repository's own).
// Vite strips license comments from the bundle, so the notices are served as a plain file,
// linked from the playground footer. Run by `npm run build:playground`.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const app = join(root, 'apps', 'playground');
const out = join(app, 'public', 'third-party-notices.txt');

const listing = execFileSync('npm', ['ls', '--omit=dev', '--all', '--parseable'], { cwd: app, encoding: 'utf8' });
const dirs = [...new Set(listing.split('\n').filter((d) => d.includes('/node_modules/')))].sort();

const sections = [];
for (const dir of dirs) {
  const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
  if (pkg.name?.startsWith('@rheon/')) continue; // our own workspace packages
  const licenseFile = readdirSync(dir).find((f) => /^(LICENSE|LICENCE|COPYING)(\.|$)/i.test(f));
  if (!licenseFile) throw new Error(`${pkg.name}@${pkg.version}: no license file in ${dir}`);
  const text = readFileSync(join(dir, licenseFile), 'utf8').trim();
  sections.push(`${pkg.name} ${pkg.version} (${pkg.license ?? 'license unspecified'})\n${pkg.homepage ? pkg.homepage + '\n' : ''}\n${text}`);
}

// --- Rust crates compiled into feel.wasm -------------------------------------------------------
const meta = JSON.parse(execFileSync('cargo', ['metadata', '--format-version', '1', '--filter-platform', 'wasm32-wasip1'], { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 }));
const byId = new Map(meta.packages.map((p) => [p.id, p]));
const members = new Set(meta.workspace_members);
const resolveNode = new Map(meta.resolve.nodes.map((n) => [n.id, n]));
const rootId = meta.workspace_members.find((id) => byId.get(id).name === 'rheon-feel-wasm');
if (!rootId) throw new Error('rheon-feel-wasm not found in cargo metadata');
const seen = new Set();
const stack = [rootId];
while (stack.length) {
  const id = stack.pop();
  if (seen.has(id)) continue;
  seen.add(id);
  for (const dep of resolveNode.get(id)?.deps ?? []) {
    // normal + build dependencies for the wasm target (dev-dependencies are not in the module)
    if (dep.dep_kinds.some((k) => k.kind === null || k.kind === 'build')) stack.push(dep.pkg);
  }
}
const crates = [...seen].map((id) => byId.get(id)).filter((p) => !members.has(p.id)).sort((a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version));
const rustSections = [];
for (const p of crates) {
  const dir = dirname(p.manifest_path);
  if (!p.license) throw new Error(`${p.name}@${p.version}: no license declared`);
  // Crate packages often omit their license files (the license is declared in Cargo.toml);
  // then the standard texts apply, which this repository ships as LICENSE and LICENSE-MIT.
  const licenseFiles = readdirSync(dir).filter((f) => /^(LICENSE|LICENCE|COPYING)(\.|-|$)/i.test(f)).sort();
  const text = licenseFiles.length
    ? licenseFiles.map((f) => readFileSync(join(dir, f), 'utf8').trim()).join('\n\n' + '-'.repeat(40) + '\n\n')
    : `The crate package contains no license file. It is distributed under ${p.license}; the standard texts are ` +
      `reproduced in the rheon repository (LICENSE for Apache-2.0, LICENSE-MIT for MIT, with the crate ` +
      `authors' copyright: ${(p.authors ?? []).join(', ') || 'see the repository'}).`;
  rustSections.push(`${p.name} ${p.version} (${p.license})\n${p.repository ? p.repository + '\n' : ''}\n${text}`);
}

const header = `Third-party notices for the rheon FEEL playground

The browser bundle includes the JavaScript packages listed first, and the FEEL engine
(feel.wasm) is compiled from the Rust crates listed after them. Each is distributed under the
license reproduced below. The engine itself is rheon's fork of dsntk
(https://github.com/dsntk/dsntk-rs, Copyright (c) 2015-2026 Dariusz Depta), MIT OR Apache-2.0;
see the LICENSE, LICENSE-MIT and NOTICE files of the rheon repository.
`;
const rule = '\n\n' + '='.repeat(78) + '\n\n';
mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, header + rule + 'JavaScript packages' + rule + sections.join(rule) + rule + 'Rust crates (feel.wasm)' + rule + rustSections.join(rule) + '\n');
console.log(`${out}: ${sections.length} npm packages, ${rustSections.length} Rust crates`);
