// Summarises a TCK run (scripts/tck.sh): totals for the official TCK and for tck-addendum/,
// failures grouped by test folder, and the difference against docs/tck-baseline.txt (the list
// of failing tests at the last recorded run). `--update-baseline` rewrites that file.
//
//   node scripts/tck-summary.mjs .tck/out/tck_results.csv [--update-baseline]
import { readFileSync, writeFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const [csv, flag] = process.argv.slice(2);
const baselinePath = fileURLToPath(new URL('../docs/tck-baseline.txt', import.meta.url));
const parse = (line) => [...line.matchAll(/"((?:[^"]|"")*)"/g)].map((m) => m[1].replaceAll('""', '"'));
const rows = readFileSync(csv, 'utf8').trim().split('\n').map(parse).filter((r) => r.length >= 4);

const key = (r) => `${r[0]} ${r[1]} ${r[2]}`;
const isAddendum = (r) => r[0].split('/')[1]?.startsWith('rheon-');
const stats = { official: { pass: 0, fail: 0 }, addendum: { pass: 0, fail: 0 } };
const failing = new Map();
for (const r of rows) {
  const s = stats[isAddendum(r) ? 'addendum' : 'official'];
  if (r[3] === 'SUCCESS') s.pass++;
  else { s.fail++; failing.set(key(r), r[4] ?? ''); }
}
for (const [name, s] of Object.entries(stats))
  console.log(`${name.padEnd(9)} ${String(s.pass).padStart(5)} passed  ${String(s.fail).padStart(4)} failed  of ${s.pass + s.fail}`);

const byFolder = new Map();
for (const k of failing.keys()) { const f = k.split(' ')[0]; byFolder.set(f, (byFolder.get(f) ?? 0) + 1); }
if (byFolder.size) {
  console.log('\nfailures by folder:');
  for (const [f, n] of [...byFolder].sort()) console.log(`  ${String(n).padStart(3)}  ${f}`);
}

if (existsSync(baselinePath)) {
  const base = new Set(readFileSync(baselinePath, 'utf8').split('\n').filter((l) => l && !l.startsWith('#')));
  const now = new Set(failing.keys());
  const newly = [...now].filter((k) => !base.has(k)).sort();
  const fixed = [...base].filter((k) => !now.has(k)).sort();
  console.log(`\nagainst docs/tck-baseline.txt: ${newly.length} newly failing, ${fixed.length} newly passing`);
  for (const k of newly) console.log(`  NEW FAIL  ${k}  ${failing.get(k).slice(0, 120)}`);
  for (const k of fixed) console.log(`  FIXED     ${k}`);
}
if (flag === '--update-baseline') {
  const header = '# Failing TCK tests at the last recorded run of scripts/tck.sh (official + tck-addendum).\n# Regenerate: node scripts/tck-summary.mjs .tck/out/tck_results.csv --update-baseline\n';
  writeFileSync(baselinePath, header + [...failing.keys()].sort().join('\n') + '\n');
  console.log(`\nwrote ${baselinePath} (${failing.size} failing tests)`);
}
